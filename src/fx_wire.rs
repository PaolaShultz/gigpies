//! GP21 names the actual remote stereo FOH owner. GP05 remains the fixed local fallback.
use crate::{control_model, show::Counter};
use serde::{Deserialize, Serialize};
pub const CONTRACT: &str = "GP21-fx";
pub const PREPARE_MS: u64 = 2_000;
pub const READ_MS: u64 = 250;
pub const DEFAULT_LEAD: u64 = 4_800;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub session: Counter,
    pub source_epoch: Counter,
    pub capability_generation: Counter,
    pub map_generation: Counter,
    pub owner_instance: Counter,
    pub library_sha256: String,
    pub abi_version: u32,
    pub config_size: u32,
    pub status_size: u32,
    pub capabilities_size: u32,
    pub channels: [String; 2],
}
impl Binding {
    pub fn validate(&self) -> Result<(), String> {
        if [
            self.session.0,
            self.source_epoch.0,
            self.capability_generation.0,
            self.map_generation.0,
            self.owner_instance.0,
        ]
        .contains(&0)
            || self.abi_version != 2
            || self.config_size != 104
            || self.status_size != 168
            || self.capabilities_size != 128
            || self.channels != ["foh-left", "foh-right"]
            || self.library_sha256.len() != 64
            || !self
                .library_sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err("FX binding".into());
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Channel {
    pub delay_ms: f64,
    pub feedback: f64,
    pub damping: f64,
    pub wet_gain: f64,
    pub bypass: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Configuration {
    pub channels: [Channel; 2],
}
impl Configuration {
    pub fn decode(text: &str) -> Result<Self, String> {
        if text.len() > 4_096 {
            return Err("FX configuration capacity".into());
        }
        // JSON numbers are allowed only inside this bounded opaque owner document.
        let config: Self = serde_json::from_str(text).map_err(|e| e.to_string())?;
        for c in &config.channels {
            if !c.delay_ms.is_finite()
                || !(1.0..=500.0).contains(&c.delay_ms)
                || !c.feedback.is_finite()
                || !(0.0..=0.85).contains(&c.feedback)
                || !c.damping.is_finite()
                || !(0.0..=0.99).contains(&c.damping)
                || !c.wet_gain.is_finite()
                || !(0.0..=1.0).contains(&c.wet_gain)
            {
                return Err("FX channel range".into());
            }
        }
        Ok(config)
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub binding: Binding,
    pub generation: Counter,
    pub settled_generation: Counter,
    pub applied_source_frame: Counter,
    pub settled_source_frame: Counter,
    pub next_source_frame: Counter,
    pub reset_count: Counter,
    pub remaining_frames: [u32; 2],
    pub owner_json: String,
}
impl Observation {
    pub fn validate(&self) -> Result<(), String> {
        self.binding.validate()?;
        Configuration::decode(&self.owner_json)?;
        if self.settled_generation.0 > self.generation.0
            || self.remaining_frames.iter().any(|n| *n > 960)
        {
            return Err("FX observation".into());
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mutation {
    pub binding: Binding,
    pub expected_generation: Counter,
    pub expected_reset_count: Counter,
    pub lead_frames: Counter,
    pub configuration_json: Option<String>,
    pub panic_mask: Option<u32>,
}
impl Mutation {
    pub fn validate(&self) -> Result<(), String> {
        self.binding.validate()?;
        if !(48..=48_000).contains(&self.lead_frames.0) || !self.lead_frames.0.is_multiple_of(48) {
            return Err("FX generation/lead".into());
        }
        if self.configuration_json.is_some() && self.expected_generation.0 == u64::MAX
            || self.panic_mask.is_some() && self.expected_reset_count.0 == u64::MAX
        {
            return Err("FX counter exhausted".into());
        }
        match (&self.configuration_json, self.panic_mask) {
            (Some(text), None) => {
                Configuration::decode(text)?;
            }
            (None, Some(1..=3)) => (),
            _ => return Err("FX operation".into()),
        }
        Ok(())
    }
}
/// The canonical authority envelope is checked before substituting a scoped
/// external transaction. No extra key may disappear during translation.
pub fn request(
    value: &serde_json::Value,
) -> Result<(control_model::Request, Option<Mutation>), String> {
    let mut value = value.clone();
    let obj = value.as_object_mut().ok_or("FX envelope")?;
    if obj.get("contract").and_then(|v| v.as_str()) != Some(CONTRACT)
        || obj.get("version").and_then(|v| v.as_u64()) != Some(1)
    {
        return Err("FX contract".into());
    }
    let snapshot = obj.get("kind").and_then(|v| v.as_str()) == Some("fx_snapshot");
    let mutation = if snapshot {
        if obj.get("body") != Some(&serde_json::json!({})) {
            return Err("FX snapshot body".into());
        }
        None
    } else {
        if obj.get("kind").and_then(|v| v.as_str()) != Some("fx_configure") {
            return Err("FX kind".into());
        }
        let m: Mutation = serde_json::from_value(obj.get("body").cloned().ok_or("FX body")?)
            .map_err(|e| e.to_string())?;
        m.validate()?;
        Some(m)
    };
    obj.insert("contract".into(), serde_json::json!("C-AUDIO"));
    obj.insert("version".into(), serde_json::json!(2));
    obj.insert(
        "kind".into(),
        serde_json::json!(if snapshot { "snapshot" } else { "renew" }),
    );
    obj.insert("body".into(), serde_json::json!({}));
    Ok((
        control_model::Request::decode(&serde_json::to_vec(&value).map_err(|e| e.to_string())?)?,
        mutation,
    ))
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum OwnerMessage {
    Observe {
        observation: Observation,
    },
    Prepared {
        ticket: Counter,
        binding: Binding,
        generation: Counter,
        configuration_json: Option<String>,
        panic_mask: Option<u32>,
    },
    Completed {
        ticket: Counter,
        effective_source_frame: Counter,
        panic_mask: Option<u32>,
        observation: Observation,
    },
    Refused {
        ticket: Counter,
        reason: String,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum OwnerCommand {
    Prepare {
        ticket: Counter,
        mutation: Mutation,
        apply_frame: Counter,
    },
    Permit {
        ticket: Counter,
        binding: Binding,
        apply_frame: Counter,
    },
    Cancel {
        ticket: Counter,
    },
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gp21_ranges_are_bounded() {
        let good = r#"{"channels":[{"delay_ms":20.0,"feedback":0.2,"damping":0.2,"wet_gain":0.1,"bypass":false},{"delay_ms":30.0,"feedback":0.3,"damping":0.1,"wet_gain":0.2,"bypass":true}]}"#;
        assert!(Configuration::decode(good).is_ok());
        assert!(Configuration::decode(&good.replace("20.0", "2001.0")).is_err());
        assert!(Configuration::decode(&good.replace("0.2", "-0.2")).is_err());
    }
    #[test]
    fn gp21_frozen_actual_owner_corpus_decodes() {
        let corpus: serde_json::Value =
            crate::show::decode(include_bytes!("../tests/fixtures/gp21/v1/owner-relay.json"))
                .unwrap();
        let (_, mutation) = request(&corpus["request"]).unwrap();
        assert!(mutation.is_some());
        for name in [
            "snapshot",
            "preparing",
            "permitted",
            "applied",
            "settled",
            "final_snapshot",
        ] {
            let observation: Observation =
                serde_json::from_value(corpus[name]["observation"].clone()).unwrap();
            observation.validate().unwrap();
        }
        assert_eq!(
            corpus["settled"]["observation"]["settled_source_frame"],
            "5760"
        );
    }
}
