//! Revision-independent maintenance of an existing, authenticated scoped lease.
use crate::{
    control_model::Scope,
    show::{Counter, Result},
};
use serde::{Deserialize, Serialize};
pub const CONTRACT: &str = "GP15-lease-maintenance";
pub const MAX_BYTES: usize = 8192;
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
    pub maintenance_id: Counter,
    #[serde(deserialize_with = "canonical_scope")]
    pub scope: Scope,
    pub lease: Counter,
}
// Keep C-AUDIO's typed Scope and wire spelling without broadening the global
// codec. Serde otherwise also accepts object spellings for unit variants.
fn canonical_scope<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Scope, D::Error> {
    use serde::de::Error;
    let value = serde_json::Value::deserialize(deserializer)?;
    let scope: Scope = serde_json::from_value(value.clone()).map_err(D::Error::custom)?;
    if serde_json::to_value(scope).map_err(D::Error::custom)? != value {
        return Err(D::Error::custom("noncanonical maintenance scope"));
    }
    Ok(scope)
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
            || self.kind != "maintain"
            || self.module != "audio"
            || self.maintenance_id.0 == 0
            || self.lease.0 == 0
            || self.authenticated_session.0 == 0
            || self.capability_generation.0 == 0
            || self.map_generation.0 == 0
            || !crate::show::id(&self.writer)
        {
            return Err("lease maintenance envelope".into());
        }
        self.authority_request().encode()?;
        Ok(())
    }
    pub fn authority_request(&self) -> crate::control_model::Request {
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
    }
    pub fn supported_scope(&self) -> bool {
        // Admission is canonical syntax only. The owner still requires the same
        // live lease, whose grant was checked against the configured topology.
        match self.scope {
            Scope::Monitor(n) => n >= 3,
            Scope::Foh
            | Scope::Monitor1
            | Scope::Monitor2
            | Scope::FxConfiguration
            | Scope::PaConfiguration
            | Scope::OutputRoutes
            | Scope::LocalOperatorMonitor
            | Scope::TalkbackDestinations
            | Scope::TalkbackFoh => true,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    Identity,
    Permission,
    Scope,
    Lease,
    Clock,
    Unavailable,
    ReusedId,
    ExpiredId,
    Capacity,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Maintained {
    pub revision: Counter,
    pub source_frame: Counter,
    pub lease_remaining_ms: u32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reply {
    pub contract: String,
    pub version: u32,
    pub state: String,
    pub reason: Option<Reason>,
    pub context: Request,
    pub result: Option<Maintained>,
}
impl Reply {
    pub fn new(context: Request, result: std::result::Result<Maintained, Reason>) -> Self {
        let (state, reason, result) = match result {
            Ok(v) => ("maintained", None, Some(v)),
            Err(e) => ("refused", Some(e), None),
        };
        Self {
            contract: CONTRACT.into(),
            version: 1,
            state: state.into(),
            reason,
            context,
            result,
        }
    }
    pub fn validate(&self) -> Result<()> {
        self.context.validate()?;
        if self.contract != CONTRACT || self.version != 1 {
            return Err("maintenance reply identity".into());
        }
        match (&self.result, self.reason, self.state.as_str()) {
            (None, Some(_), "refused") => Ok(()),
            (Some(r), None, "maintained")
                if self.context.supported_scope()
                    && r.lease_remaining_ms == crate::control_model::LEASE_MS as u32
                    && r.source_frame.0.is_multiple_of(48) =>
            {
                Ok(())
            }
            _ => Err("maintenance reply shape".into()),
        }
    }
    pub fn encode(&self) -> Result<Vec<u8>> {
        self.validate()?;
        let bytes = serde_json::to_vec(self).map_err(|e| e.to_string())?;
        if bytes.len() > MAX_BYTES {
            return Err("maintenance capacity".into());
        }
        Ok(bytes)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let value: serde_json::Value = crate::show::decode_bounded(bytes, MAX_BYTES)?;
        exact_keys(
            &value,
            &[
                "contract", "version", "state", "reason", "context", "result",
            ],
        )?;
        let r: Self = serde_json::from_value(value).map_err(|e| e.to_string())?;
        r.validate()?;
        Ok(r)
    }
}
pub(crate) fn exact_keys(value: &serde_json::Value, keys: &[&str]) -> Result<()> {
    let object = value.as_object().ok_or("atomic object")?;
    if object.len() != keys.len() || object.keys().any(|k| !keys.contains(&k.as_str())) {
        return Err("atomic exact fields".into());
    }
    Ok(())
}
