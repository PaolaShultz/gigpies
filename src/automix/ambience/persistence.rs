//! A complete preparation binds its reports, settings and source bytes together.
use super::{Policy, Review, Session};
use crate::{
    automix::identity::{SourceIdentity, bytes_hash, file_hash, identity, source_identities},
    inventory::Result,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::{File, OpenOptions},
    io::Write,
    path::Path,
};

const ARTIFACTS: &[&str] = &[
    "before-settings.json",
    "policy.json",
    "input-identity.json",
    "decisions.json",
    "seed-fx.json",
    "calibration.json",
    "candidate-settings.json",
    "training-checks.json",
    "frozen-before-held-out.json",
    "held-out-checks.json",
    "review.json",
    "REVIEW.md",
    "selection.json",
    "settings.json",
];

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputIdentity {
    pub schema_version: u32,
    pub baseline_id: String,
    pub policy_id: String,
    pub source_files: Vec<SourceIdentity>,
}
impl InputIdentity {
    pub(super) fn capture(s: &Session, p: &Policy, root: &Path) -> Result<Self> {
        Ok(Self {
            schema_version: 1,
            baseline_id: identity(s)?,
            policy_id: identity(p)?,
            source_files: source_identities(s, root)?,
        })
    }
    pub(super) fn verify_sources(&self, s: &Session, root: &Path) -> Result<()> {
        if self.source_files != source_identities(s, root)? {
            return Err("source recordings changed; retain the baseline and prepare again in a new directory".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ready {
    pub schema_version: u32,
    pub candidate_id: String,
    pub input_identity_id: String,
    pub artifacts: BTreeMap<String, String>,
}

/// Publish complete JSON without exposing a partially serialized ready artifact.
/// hard_link preserves no-overwrite semantics; both files are on the same filesystem.
fn publish_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let temporary = path.with_extension("json.part");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    serde_json::to_writer_pretty(&mut file, value)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    std::fs::hard_link(&temporary, path)?;
    std::fs::remove_file(&temporary)?;
    File::open(path.parent().ok_or("missing preparation directory")?)?.sync_all()?;
    Ok(())
}

pub(super) fn finish(out: &Path, candidate: &Session, inputs: &InputIdentity) -> Result<()> {
    publish_json(&out.join("settings.json"), candidate)?;
    let artifacts = ARTIFACTS
        .iter()
        .map(|name| {
            let path = out.join(name);
            File::open(&path)?.sync_all()?;
            Ok(((*name).to_string(), file_hash(&path)?))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;
    let ready = Ready {
        schema_version: 1,
        candidate_id: identity(candidate)?,
        input_identity_id: identity(inputs)?,
        artifacts,
    };
    publish_json(&out.join("ready.json"), &ready)
}

fn read<T: serde::de::DeserializeOwned>(out: &Path, name: &str) -> Result<T> {
    Ok(serde_json::from_reader(File::open(out.join(name))?)?)
}

/// Verify saved evidence and current sources without measuring or writing anything.
/// A legacy or interrupted plan needs its original verification workflow; it cannot
/// be promoted to a newly verified plan by assuming absent hashes or checks.
pub fn verify(out: &Path, root: &Path) -> Result<Review> {
    if out.join("failure.json").exists() {
        return Err(
            "preparation records a failure; keep its evidence and retry in a new directory".into(),
        );
    }
    if !out.join("ready.json").is_file() {
        return Err("plan has no ready.json completion record: incomplete or legacy preparation; retain its evidence and use a new output directory for a new plan".into());
    }
    let ready: Ready = read(out, "ready.json")?;
    if ready.schema_version != 1
        || ready.artifacts.len() != ARTIFACTS.len()
        || ARTIFACTS
            .iter()
            .any(|name| !ready.artifacts.contains_key(*name))
    {
        return Err("unsupported or incomplete preparation manifest".into());
    }
    let mut snapshot = BTreeMap::new();
    for &name in ARTIFACTS {
        let bytes = std::fs::read(out.join(name))?;
        if bytes_hash(&bytes) != ready.artifacts[name] {
            return Err(format!("saved preparation artifact changed: {name}").into());
        }
        snapshot.insert(name, bytes);
    }
    // Deserialize the same bytes that passed the hash checks, rather than reopen
    // files that another process could have changed between those two reads.
    let baseline: Session = serde_json::from_slice(&snapshot["before-settings.json"])?;
    let candidate: Session = serde_json::from_slice(&snapshot["settings.json"])?;
    let measured_candidate: Session = serde_json::from_slice(&snapshot["candidate-settings.json"])?;
    let policy: Policy = serde_json::from_slice(&snapshot["policy.json"])?;
    let inputs: InputIdentity = serde_json::from_slice(&snapshot["input-identity.json"])?;
    policy.validate(&baseline)?;
    candidate.validate()?;
    if inputs.schema_version != 1
        || identity(&inputs)? != ready.input_identity_id
        || identity(&baseline)? != inputs.baseline_id
        || identity(&policy)? != inputs.policy_id
        || identity(&candidate)? != ready.candidate_id
        || identity(&measured_candidate)? != ready.candidate_id
    {
        return Err("saved preparation identity is inconsistent".into());
    }
    let frozen: serde_json::Value =
        serde_json::from_slice(&snapshot["frozen-before-held-out.json"])?;
    if frozen["settings_id"].as_str() != Some(&ready.candidate_id)
        || frozen["input_identity_id"].as_str() != Some(&ready.input_identity_id)
        || frozen["training_passed"] != true
        || frozen["held_out_retries"] != 0
    {
        return Err("held-out evidence does not identify the frozen candidate".into());
    }
    let mut direct = candidate.clone();
    direct.effects = None;
    if identity(&direct)? != inputs.baseline_id {
        return Err("spatial preparation changed the frozen direct settings".into());
    }
    let review: Review = serde_json::from_slice(&snapshot["review.json"])?;
    if review.schema_version != 1
        || !review.technically_eligible
        || review.listener_accepted.is_some()
        || review.full_export_verified.is_some()
        || review.export_gain_db.is_some()
        || review.training_checks.is_empty()
        || review.held_out_checks.is_empty()
        || review
            .training_checks
            .iter()
            .chain(&review.held_out_checks)
            .any(|c| {
                !c.passed
                    || c.max_master_reduction_db != Some(0.)
                    || c.observed_windows.unwrap_or(0) == 0
            })
    {
        return Err("saved preparation is not technically eligible".into());
    }
    inputs.verify_sources(&baseline, root)?;
    Ok(review)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Scratch(std::path::PathBuf);
    impl Scratch {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "gigpies-fx-publish-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn publishing_keeps_existing_destination_and_complete_json() {
        let root = Scratch::new();
        let path = root.0.join("ready.json");
        let original = serde_json::json!({"version":1,"details":[1,2,3]});
        publish_json(&path, &original).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&bytes).unwrap(),
            original
        );
        assert!(!path.with_extension("json.part").exists());
        assert!(publish_json(&path, &serde_json::json!({"replacement":true})).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }

    #[test]
    fn failed_serialization_never_creates_a_ready_file() {
        struct Fails;
        impl Serialize for Fails {
            fn serialize<S: serde::Serializer>(
                &self,
                serializer: S,
            ) -> std::result::Result<S::Ok, S::Error> {
                use serde::ser::{Error, SerializeSeq};
                let mut seq = serializer.serialize_seq(Some(2))?;
                seq.serialize_element(&1)?;
                Err(S::Error::custom("injected serialization failure"))
            }
        }
        let root = Scratch::new();
        let path = root.0.join("ready.json");
        assert!(publish_json(&path, &Fails).is_err());
        assert!(!path.exists());
        assert!(path.with_extension("json.part").exists());
        assert!(publish_json(&path, &1).is_err());
        let fresh = root.0.join("fresh.json");
        publish_json(&fresh, &1).unwrap();
    }
}
