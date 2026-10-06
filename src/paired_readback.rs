//! One correlated envelope for committed raw and Brain state.
use crate::{
    brain_control::Snapshot,
    mixer_control::RenderedSnapshot,
    show::{Counter, Result},
};
use serde::{Deserialize, Serialize};
pub const CONTRACT: &str = "GP15-paired-readback";
pub const MAX_BYTES: usize = crate::snapshot_pages::ASSEMBLY_BYTES;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub contract: String,
    pub version: u32,
    pub kind: String,
    pub show_id: String,
    pub module: String,
    pub epoch: Counter,
    pub authenticated_session: Counter,
    pub writer: String,
    pub capability_generation: Counter,
    pub map_generation: Counter,
    pub query_id: Counter,
}
impl Request {
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let r: Self = crate::show::decode_bounded(bytes, MAX_BYTES)?;
        r.validate()?;
        Ok(r)
    }
    pub fn validate(&self) -> Result<()> {
        if self.contract != CONTRACT
            || self.version != 1
            || self.kind != "readback"
            || self.module != "audio"
            || self.query_id.0 == 0
            || self.authenticated_session.0 == 0
            || self.capability_generation.0 == 0
            || self.map_generation.0 == 0
            || !crate::show::id(&self.writer)
        {
            return Err("paired readback envelope".into());
        }
        crate::control_model::Request {
            contract: "C-AUDIO".into(),
            version: 1,
            show_id: self.show_id.clone(),
            module: self.module.clone(),
            epoch: self.epoch,
            writer: None,
            lease: None,
            request_id: None,
            expected_revision: None,
            command: crate::control_model::Command::Snapshot {},
        }
        .encode()?;
        Ok(())
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    Identity,
    QueryId,
    Capacity,
    Clock,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reply {
    pub contract: String,
    pub version: u32,
    pub state: String,
    pub reason: Option<Reason>,
    pub context: Request,
    pub raw: Option<RenderedSnapshot>,
    pub brain: Option<Snapshot>,
}
impl Reply {
    pub fn new(
        context: Request,
        result: std::result::Result<(RenderedSnapshot, Snapshot), Reason>,
    ) -> Self {
        let (state, reason, raw, brain) = match result {
            Ok((raw, brain)) => ("snapshot", None, Some(raw), Some(brain)),
            Err(e) => ("refused", Some(e), None, None),
        };
        Self {
            contract: CONTRACT.into(),
            version: 1,
            state: state.into(),
            reason,
            context,
            raw,
            brain,
        }
    }
    pub fn validate(&self) -> Result<()> {
        self.context.validate()?;
        if self.contract != CONTRACT || self.version != 1 {
            return Err("paired reply identity".into());
        }
        match (&self.raw, &self.brain, self.reason, self.state.as_str()) {
            (None, None, Some(_), "refused") => Ok(()),
            (Some(raw), Some(brain), None, "snapshot") => {
                raw.validate()?;
                if raw.authority.page != 0
                    || raw.authority.page_count != 1
                    || raw.authority.show_id != self.context.show_id
                    || raw.authority.epoch != self.context.epoch
                    || raw.authority.revision != brain.revision
                    || raw.frame != brain.frame
                    || !raw.frame.0.is_multiple_of(48)
                    || raw
                        .topology
                        .as_ref()
                        .is_some_and(|t| t.map_revision != self.context.map_generation.0)
                {
                    return Err("paired committed identity".into());
                }
                validate_brain(
                    brain,
                    raw.authority.inputs.len(),
                    raw.authority.monitors.len(),
                )
            }
            _ => Err("paired reply shape".into()),
        }
    }
    pub fn encode(&self) -> Result<Vec<u8>> {
        self.validate()?;
        let bytes = serde_json::to_vec(self).map_err(|e| e.to_string())?;
        if bytes.len() > MAX_BYTES {
            return Err("paired capacity".into());
        }
        Ok(bytes)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let value: serde_json::Value = crate::show::decode_bounded(bytes, MAX_BYTES)?;
        crate::lease_maintenance::exact_keys(
            &value,
            &[
                "contract", "version", "state", "reason", "context", "raw", "brain",
            ],
        )?;
        if !value["brain"].is_null() {
            crate::lease_maintenance::exact_keys(
                &value["brain"],
                &[
                    "source",
                    "selection_generation",
                    "monitor_gain_cdb",
                    "monitor_mute",
                    "monitor_dim",
                    "monitor_armed",
                    "talkback_monitors",
                    "talkback_foh",
                    "talkback_gain_cdb",
                    "talkback_mute",
                    "hold_generation_counter",
                    "held_generation",
                    "hold_deadline_ms",
                    "audible_path_ready",
                    "talkback_path_ready",
                    "monitor_path_ready",
                    "microphone_peak_nano",
                    "outgoing_peak_nano",
                    "monitor_peak_nano",
                    "frame",
                    "revision",
                    "heartbeat_ms",
                    "deadman_ms",
                    "fade_frames",
                ],
            )?;
        }
        let r: Self = serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
        // Existing nested schemas include nullable fields and some legacy serde
        // defaults. Require the exact producer field shape, including nulls, so
        // a default or a permissive nested struct cannot hide a malformed half.
        if serde_json::to_value(&r).map_err(|e| e.to_string())? != value {
            return Err("paired exact nested fields".into());
        }
        r.validate()?;
        Ok(r)
    }
}
fn validate_brain(b: &Snapshot, inputs: usize, monitors: usize) -> Result<()> {
    use crate::brain_control::MonitorSource;
    let source_valid = match b.source {
        MonitorSource::None | MonitorSource::Main => true,
        MonitorSource::Monitor { index } => index < monitors,
        MonitorSource::Pfl { input } | MonitorSource::Afl { input } => input < inputs,
    };
    if !source_valid
        || b.selection_generation.0 == 0
        || !(-9000..=0).contains(&b.monitor_gain_cdb)
        || !(-9000..=0).contains(&b.talkback_gain_cdb)
        || b.talkback_monitors.iter().any(|&n| n >= monitors)
        || b.talkback_monitors
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != b.talkback_monitors.len()
        || b.held_generation.is_some() != b.hold_deadline_ms.is_some()
        || b.held_generation
            .is_some_and(|g| g.0 == 0 || g.0 > b.hold_generation_counter.0)
        || b.hold_deadline_ms.is_some_and(|d| d.0 == 0)
        || b.heartbeat_ms != 50
        || b.deadman_ms != 150
        || b.fade_frames != 240
        || b.audible_path_ready != (b.talkback_path_ready || b.monitor_path_ready)
        || (b.talkback_path_ready && (b.held_generation.is_none() || b.talkback_mute))
        || (b.monitor_path_ready
            && (!b.monitor_armed || b.monitor_mute || b.source == MonitorSource::None))
    {
        return Err("paired brain snapshot".into());
    }
    Ok(())
}
