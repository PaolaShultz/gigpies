//! GP-METER:1 independent read-only telemetry; no control admission or lease state.
use crate::show::{Counter, Result};
use serde::{Deserialize, Serialize};
fn uuid(s: &str) -> bool {
    s.len() == 36
        && s.bytes().enumerate().all(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                b == b'-'
            } else {
                b.is_ascii_digit() || (b'a'..=b'f').contains(&b)
            }
        })
}
pub const CONTRACT: &str = "GP-METER";
pub const MAX_TAPS: usize = 2048;
pub const MAX_BYTES: usize = 16 * 8192;
fn required<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> std::result::Result<Option<T>, D::Error> {
    Option::deserialize(d)
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub contract: String,
    pub version: u32,
    pub kind: String,
    pub show_id: String,
    pub module: String,
    pub source_epoch: Counter,
    pub query_id: Counter,
    pub expected_map: Counter,
    #[serde(deserialize_with = "required")]
    pub writer: Option<String>,
    #[serde(deserialize_with = "required")]
    pub lease: Option<Counter>,
    #[serde(deserialize_with = "required")]
    pub request_id: Option<Counter>,
    #[serde(deserialize_with = "required")]
    pub expected_revision: Option<Counter>,
}
impl Request {
    pub fn new(show: &str, epoch: u64, map: u64, query: u64) -> Self {
        Self {
            contract: CONTRACT.into(),
            version: 1,
            kind: "meter_snapshot".into(),
            show_id: show.into(),
            module: "audio".into(),
            source_epoch: Counter(epoch),
            query_id: Counter(query),
            expected_map: Counter(map),
            writer: None,
            lease: None,
            request_id: None,
            expected_revision: None,
        }
    }
    pub fn validate(&self) -> Result<()> {
        if self.contract != CONTRACT
            || self.kind != "meter_snapshot"
            || self.module != "audio"
            || !uuid(&self.show_id)
            || self.source_epoch.0 == 0
            || self.query_id.0 == 0
            || self.expected_map.0 == 0
            || self.writer.is_some()
            || self.lease.is_some()
            || self.request_id.is_some()
            || self.expected_revision.is_some()
        {
            return Err("meter read envelope".into());
        }
        Ok(())
    }
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        if bytes.len() > 65536 {
            return Err("meter request capacity".into());
        }
        let r: Self = crate::show::decode(bytes)?;
        r.validate()?;
        Ok(r)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Tap {
    pub id: String,
    pub valid: bool,
    #[serde(deserialize_with = "required")]
    pub reason: Option<String>,
    #[serde(deserialize_with = "required")]
    pub peak_millidbfs: Option<i32>,
    #[serde(deserialize_with = "required")]
    pub rms_millidbfs: Option<i32>,
    pub silent: bool,
    pub below_floor: bool,
    pub over_range: bool,
    pub clip_count: Counter,
    pub invalid_count: Counter,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub contract: String,
    pub version: u32,
    pub kind: String,
    pub show_id: String,
    pub module: String,
    pub source_epoch: Counter,
    pub query_id: Counter,
    pub capability_generation: Counter,
    pub map_generation: Counter,
    pub topology: String,
    pub sample_rate: u32,
    pub inputs: usize,
    pub monitors: usize,
    pub sequence: Counter,
    #[serde(deserialize_with = "required")]
    pub first_frame: Option<Counter>,
    #[serde(deserialize_with = "required")]
    pub end_frame: Option<Counter>,
    #[serde(deserialize_with = "required")]
    pub acquisition_age_ms: Option<Counter>,
    pub publication_loss: Counter,
    pub valid: bool,
    #[serde(deserialize_with = "required")]
    pub reason: Option<String>,
    pub taps: Vec<Tap>,
}
impl Snapshot {
    pub fn validate(&self) -> Result<()> {
        let count = self
            .inputs
            .checked_mul(2)
            .and_then(|n| n.checked_add(self.monitors))
            .and_then(|n| n.checked_add(2))
            .ok_or("meter inventory overflow")?;
        if self.contract != CONTRACT
            || self.version != 1
            || self.kind != "meter_snapshot"
            || self.module != "audio"
            || !uuid(&self.show_id)
            || self.source_epoch.0 == 0
            || self.query_id.0 == 0
            || self.map_generation.0 == 0
            || self.capability_generation.0 == 0
            || self.topology.is_empty()
            || self.topology.len() > 128
        {
            return Err("meter identity".into());
        }
        if !self.valid {
            if !matches!(
                self.reason.as_deref(),
                Some("missing_window" | "unsupported" | "identity" | "capacity" | "quiesced")
            ) || !self.taps.is_empty()
                || self.first_frame.is_some()
                || self.end_frame.is_some()
                || self.acquisition_age_ms.is_some()
            {
                return Err("meter unavailable shape".into());
            }
            return Ok(());
        }
        if count > MAX_TAPS
            || self.inputs == 0
            || self.sample_rate == 0
            || !self.sample_rate.is_multiple_of(50)
            || self.reason.is_some()
            || self.taps.len() != count
            || self.sequence.0 == 0
            || self.acquisition_age_ms.is_none()
        {
            return Err("meter window shape".into());
        }
        let first = self.first_frame.ok_or("first frame")?.0;
        let end = self.end_frame.ok_or("end frame")?.0;
        let frames = u64::from(self.sample_rate / 50);
        if first.checked_add(frames) != Some(end) {
            return Err("meter window frames".into());
        }
        for (i, t) in self.taps.iter().enumerate() {
            let id = if i < self.inputs * 2 {
                format!(
                    "input-{:02}:{}",
                    i / 2 + 1,
                    if i % 2 == 0 { "raw" } else { "processed" }
                )
            } else if i == self.inputs * 2 {
                "main-l".into()
            } else if i == self.inputs * 2 + 1 {
                "main-r".into()
            } else {
                format!("monitor-{}", i - self.inputs * 2 - 1)
            };
            if t.id != id
                || t.clip_count.0 > frames
                || t.invalid_count.0 > frames
                || t.clip_count
                    .0
                    .checked_add(t.invalid_count.0)
                    .is_none_or(|n| n > frames)
            {
                return Err("meter tap identity/count".into());
            }
            if t.valid {
                let p = t.peak_millidbfs.ok_or("peak")?;
                let r = t.rms_millidbfs.ok_or("rms")?;
                if t.reason.is_some()
                    || t.invalid_count.0 != 0
                    || !(-120000..=6165095).contains(&p)
                    || !(-120000..=6165095).contains(&r)
                    || r > p
                    || (t.silent
                        && (p != -120000
                            || r != -120000
                            || t.clip_count.0 != 0
                            || t.over_range
                            || !t.below_floor))
                    || (t.below_floor && p != -120000)
                    || (t.over_range && (p < 0 || t.clip_count.0 == 0))
                    || (t.clip_count.0 > 0 && p < 0)
                    || (p > 0 && !t.over_range)
                {
                    return Err("meter numeric relationships".into());
                }
            } else if (t.reason.as_deref() == Some("nonfinite")) != (t.invalid_count.0 > 0)
                || !matches!(
                    t.reason.as_deref(),
                    Some("nonfinite" | "graph_fault" | "quiesced")
                )
                || t.peak_millidbfs.is_some()
                || t.rms_millidbfs.is_some()
                || t.silent
                || t.below_floor
                || t.over_range
            {
                return Err("meter invalid shape".into());
            }
        }
        Ok(())
    }
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        if bytes.len() > MAX_BYTES {
            return Err("meter document capacity".into());
        }
        let s: Self = crate::show::decode_bounded(bytes, MAX_BYTES)?;
        s.validate()?;
        Ok(s)
    }
}
