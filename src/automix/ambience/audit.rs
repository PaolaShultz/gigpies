//! Read saved scalar evidence without rerunning DSP or upgrading historical checks.
use super::{Check, Decision, Effect, FxConfig, Policy, Session, Style, write_json};
use crate::{
    automix::identity::{bytes_hash, file_hash, identity},
    inventory::Result,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    collections::BTreeMap,
    fmt::Write as _,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Deserialize)]
struct SavedCalibration {
    bus: String,
    measured_seed_wet_source_db: f64,
    artistic_profile_target_db: f64,
    requested_return_db: f64,
    bounded_return_db: f64,
    amount: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuditedBus {
    pub bus: String,
    pub group: String,
    pub effect: Effect,
    pub inputs: Vec<super::Input>,
    pub desired_decay_seconds: Option<f64>,
    pub recorded_decay_seconds: Option<f64>,
    pub decay_residual_seconds: Option<f64>,
    pub decay_above_target_at_minimum_control: bool,
    pub recorded_seed_wet_source_db: f64,
    pub profile_target_db: f64,
    pub amount: f64,
    pub effective_target_db: f64,
    pub requested_correction_db: f64,
    pub applied_return_db: f64,
    pub correction_limited: bool,
    pub return_floor_applied: bool,
    /// Derived from the saved seed and final return, not a fresh measurement.
    pub predicted_calibrated_wet_source_db: f64,
    pub predicted_residual_db: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SavedAudit {
    pub schema_version: u32,
    pub plan_directory: PathBuf,
    pub read_files: BTreeMap<String, String>,
    pub baseline_id: String,
    pub candidate_id: String,
    pub style: Style,
    pub style_basis: String,
    pub recorded_selection: String,
    pub recorded_ensemble_checks_passed: Option<bool>,
    pub training_precedes_held_out: bool,
    pub recorded_master_stage_observations: bool,
    #[serde(default)]
    pub recorded_coverage_observations: bool,
    pub current_source_identity_verified: bool,
    pub recorded_listener_acceptance: Option<bool>,
    pub buses: Vec<AuditedBus>,
    pub limitations: Vec<String>,
}

struct Reader<'a> {
    root: &'a Path,
    hashes: BTreeMap<String, String>,
}
impl Reader<'_> {
    fn read<T: DeserializeOwned>(&mut self, name: &str) -> Result<T> {
        let bytes = std::fs::read(self.root.join(name))?;
        let value = serde_json::from_slice(&bytes)?;
        self.hashes.insert(name.into(), bytes_hash(&bytes));
        Ok(value)
    }
}

fn recorded_checks(checks: &[Check], spans: &[[f64; 2]]) -> Result<()> {
    if checks.len() != spans.len() || checks.iter().zip(spans).any(|(c, span)| c.span != *span) {
        return Err("saved ensemble checks do not match their declared passages".into());
    }
    for c in checks {
        let required = [
            c.dry_rms_dbfs,
            c.mixed_rms_change_db,
            c.crest_loss_db,
            c.peak_change_db,
        ];
        if required.iter().flatten().any(|v| !v.is_finite())
            || c.wet_to_dry_db.is_some_and(|v| !v.is_finite())
            || c.max_master_reduction_db
                .is_some_and(|v| !v.is_finite() || v < 0.)
            || c.observed_seconds.is_some_and(|v| !v.is_finite() || v < 0.)
            || (c.passed
                && (required.iter().any(Option::is_none)
                    || c.mixed_rms_change_db.is_some_and(|v| v > 2.)
                    || c.peak_change_db.is_some_and(|v| v > 2.)
                    || c.crest_loss_db.is_some_and(|v| v > 3.)
                    || c.wet_to_dry_db.is_some_and(|v| v > -12.)
                    || c.max_master_reduction_db.is_some_and(|v| v > 0.)
                    || c.observed_windows == Some(0)
                    || c.observed_seconds
                        .is_some_and(|v| v < c.span[1] - c.span[0] - 0.04 - 1e-9)
                    || !c.failure_reasons.is_empty()))
        {
            return Err("saved ensemble pass disagrees with its recorded measurements".into());
        }
    }
    Ok(())
}

/// Writes only a new audit directory. No source-root argument, audio I/O, fitting,
/// changed settings or readiness record is part of this operation.
pub fn audit_saved(plan: &Path, out: &Path) -> Result<SavedAudit> {
    if out.exists() {
        return Err("audit output must be a new directory".into());
    }
    let mut reader = Reader {
        root: plan,
        hashes: BTreeMap::new(),
    };
    let baseline: Session = reader.read("before-settings.json")?;
    let candidate: Session = reader.read("candidate-settings.json")?;
    let policy: Policy = reader.read("policy.json")?;
    // Apply the original structural contract, and report the later chronology
    // rule separately. Reading a legacy plan never makes it a new eligible plan.
    policy.validate_structure(&baseline)?;
    candidate.validate()?;
    let mut direct = candidate.clone();
    direct.effects = None;
    if identity(&direct)? != identity(&baseline)? {
        return Err("saved spatial candidate changed the baseline's direct settings".into());
    }
    let mut seed: FxConfig = reader.read("seed-fx.json")?;
    seed.validate(&baseline)?;
    if seed.exciter_amount != 0.
        || !seed.master_eq.is_empty()
        || seed.maximizer_drive_db != 0.
        || seed.maximizer_threshold_db != 24.
        || seed.buses.iter().any(|b| b.return_db != 0.)
    {
        return Err("saved seed does not preserve the spatial-only calibration contract".into());
    }
    let decisions: Vec<Decision> = reader.read("decisions.json")?;
    let calibration: Vec<SavedCalibration> = reader.read("calibration.json")?;
    let training: Vec<Check> = reader.read("training-checks.json")?;
    recorded_checks(&training, &policy.training)?;
    let held: Vec<Check> = if plan.join("held-out-checks.json").exists() {
        let checks: Vec<Check> = reader.read("held-out-checks.json")?;
        recorded_checks(&checks, &policy.held_out)?;
        checks
    } else {
        vec![]
    };
    let selection: serde_json::Value = reader.read("selection.json")?;
    let selected = selection["selected"]
        .as_str()
        .ok_or("missing saved selection")?;
    if !["baseline", "expert_fx_preview"].contains(&selected) {
        return Err("unsupported saved selection".into());
    }
    let checks_passed = if training.iter().chain(&held).any(|c| !c.passed) {
        Some(false)
    } else if !held.is_empty() {
        Some(true)
    } else {
        None
    };
    if selected == "expert_fx_preview"
        && (checks_passed != Some(true) || candidate.effects.is_none())
    {
        return Err("saved preview lacks its passed ensemble checks or FX settings".into());
    }
    if plan.join("settings.json").exists() {
        let applied: Session = reader.read("settings.json")?;
        let expected = if selected == "expert_fx_preview" {
            &candidate
        } else {
            &baseline
        };
        if identity(&applied)? != identity(expected)? {
            return Err("saved selected settings disagree with the selection".into());
        }
    } else if selected == "expert_fx_preview" {
        return Err("saved preview is missing its selected settings".into());
    }
    if decisions.len() != policy.groups.len()
        || decisions
            .iter()
            .zip(&policy.groups)
            .any(|(d, g)| d.group != g.name || d.family != g.family)
        || calibration.len() != seed.buses.len()
        || decisions.iter().map(|d| d.bus_names.len()).sum::<usize>() != seed.buses.len()
    {
        return Err("saved FX decisions, groups or calibration are incomplete".into());
    }
    let mut buses = Vec::new();
    for (bus, c) in seed.buses.iter_mut().zip(&calibration) {
        let owners = decisions
            .iter()
            .enumerate()
            .filter(|(_, d)| d.bus_names.contains(&bus.name))
            .collect::<Vec<_>>();
        if owners.len() != 1
            || bus.name != c.bus
            || c.amount != policy.amount
            || c.amount <= 0.
            || c.artistic_profile_target_db != bus.target_wet_db
            || (c.requested_return_db - (bus.target_wet_db - c.measured_seed_wet_source_db)).abs()
                > 1e-9
        {
            return Err(
                "saved return calibration has inconsistent target, amount or ownership".into(),
            );
        }
        let (g, decision) = owners[0];
        if bus.sends.len() != policy.groups[g].inputs.len()
            || bus.sends.iter().any(|send| {
                send.db != 0.
                    || !policy.groups[g]
                        .inputs
                        .iter()
                        .any(|input| input.channel == send.channel)
            })
        {
            return Err("saved FX sends disagree with their source group".into());
        }
        let amount_db = 20. * c.amount.log10();
        let correction = c.requested_return_db.clamp(-18., 18.);
        let expected_return = (correction + amount_db).max(-60.);
        if !c.measured_seed_wet_source_db.is_finite()
            || !c.requested_return_db.is_finite()
            || !c.bounded_return_db.is_finite()
            || (expected_return - c.bounded_return_db).abs() > 1e-9
        {
            return Err("saved return violates its recorded calibration bounds".into());
        }
        bus.return_db = c.bounded_return_db;
        let (desired, actual, at_minimum) = if let Effect::Reverb(reverb) = bus.effect {
            let desired = decision
                .desired_decay_seconds
                .ok_or("missing saved decay target")?;
            let actual = decision
                .measured_decay_seconds
                .ok_or("missing saved decay observation")?;
            if !desired.is_finite() || desired <= 0. || !actual.is_finite() || actual < 0. {
                return Err("invalid saved decay observation".into());
            }
            (
                Some(desired),
                Some(actual),
                reverb.decay == 0. && actual > desired,
            )
        } else {
            (None, None, false)
        };
        buses.push(AuditedBus {
            bus: bus.name.clone(),
            group: decision.group.clone(),
            effect: bus.effect.clone(),
            inputs: policy.groups[g].inputs.clone(),
            desired_decay_seconds: desired,
            recorded_decay_seconds: actual,
            decay_residual_seconds: actual.zip(desired).map(|(a, d)| a - d),
            decay_above_target_at_minimum_control: at_minimum,
            recorded_seed_wet_source_db: c.measured_seed_wet_source_db,
            profile_target_db: bus.target_wet_db,
            amount: c.amount,
            effective_target_db: bus.target_wet_db + amount_db,
            requested_correction_db: c.requested_return_db,
            applied_return_db: bus.return_db,
            correction_limited: c.requested_return_db < -18. || c.requested_return_db > 18.,
            return_floor_applied: correction + amount_db < -60.,
            predicted_calibrated_wet_source_db: c.measured_seed_wet_source_db + bus.return_db,
            predicted_residual_db: c.measured_seed_wet_source_db + bus.return_db
                - bus.target_wet_db
                - amount_db,
        });
    }
    let expected_fx = (!seed.buses.is_empty()).then_some(seed);
    if identity(&expected_fx)? != identity(&candidate.effects)? {
        return Err(
            "saved candidate does not match seed FX plus the recorded return calibration".into(),
        );
    }
    let recorded_listener_acceptance = selection["listener_accepted"].as_bool();
    if !selection["listener_accepted"].is_null() && recorded_listener_acceptance.is_none() {
        return Err("invalid saved listener acceptance field".into());
    }
    for (name, expected) in &reader.hashes {
        if file_hash(&plan.join(name))? != *expected {
            return Err("saved plan changed during the audit; retry from stable evidence".into());
        }
    }
    let audit = SavedAudit {
        schema_version: 1, plan_directory: std::fs::canonicalize(plan)?, read_files: reader.hashes,
        baseline_id: identity(&baseline)?, candidate_id: identity(&candidate)?,
        style: policy.style, style_basis: policy.style_basis.clone(), recorded_selection: selected.into(),
        recorded_ensemble_checks_passed: checks_passed,
        training_precedes_held_out: policy.chronological(),
        recorded_master_stage_observations: !held.is_empty() && training.iter().chain(&held).all(|c| c.max_master_reduction_db.is_some()),
        recorded_coverage_observations: !held.is_empty() && training.iter().chain(&held).all(|c| c.observed_windows.is_some() && c.observed_seconds.is_some()),
        current_source_identity_verified: false, recorded_listener_acceptance, buses,
        limitations: vec![
            "This audit reads saved scalar reports. It does not remeasure audio, verify current recordings or grant new technical eligibility.".into(),
            "Wet residuals are predictions from the recorded seed measurement and applied return. Actual per-passage bus observations, when recorded, remain in the original review.json.".into(),
            "A recorded ensemble pass does not establish individual target attainment or listener preference. Complete-export measurements and export gain are outside this audit.".into(),
            "Missing legacy master-stage observations remain unknown. Interleaved historical passages can carry held-out audio into later training through continuous DSP state; new plans require chronological splits.".into(),
            "Missing legacy window counts or observed durations leave passage coverage unknown. A recorded pass cannot supply those absent observations.".into(),
        ],
    };
    std::fs::create_dir(out)?;
    write_json(&out.join("audit.json"), &audit)?;
    std::fs::write(out.join("AUDIT.md"), audit.markdown())?;
    Ok(audit)
}

fn cell(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('|', "\\|")
        .replace(['\n', '\r'], " ")
}

impl SavedAudit {
    pub fn minimum_decay_limits(&self) -> usize {
        self.buses
            .iter()
            .filter(|b| b.decay_above_target_at_minimum_control)
            .count()
    }
    pub fn return_limits(&self) -> usize {
        self.buses
            .iter()
            .filter(|b| b.correction_limited || b.return_floor_applied)
            .count()
    }
    pub fn markdown(&self) -> String {
        let yes_no = |v| if v { "yes" } else { "no" };
        let recorded = match self.recorded_ensemble_checks_passed {
            Some(true) => "passed",
            Some(false) => "failed",
            None => "incomplete",
        };
        let listener = match self.recorded_listener_acceptance {
            Some(true) => "accepted in the saved record",
            Some(false) => "rejected in the saved record",
            None => "not recorded",
        };
        let mut text = format!(
            "# Saved artistic FX audit\n\nStyle: **{:?}**. Basis: {}\n\nRecorded selection: `{}`. Recorded ensemble checks: **{}**.\nRecorded listener acceptance: **{}**.\n\nTraining entirely before held out: **{}**. Master-stage measurements present in all passages: **{}**. Current recording hashes were **not checked** by this audit.\n\n",
            self.style,
            cell(&self.style_basis),
            cell(&self.recorded_selection),
            recorded,
            listener,
            yes_no(self.training_precedes_held_out),
            yes_no(self.recorded_master_stage_observations)
        );
        let _ = writeln!(
            text,
            "Window counts and observed durations present in all passages: **{}**. This states evidence availability; the recorded ensemble result remains separate.\n",
            yes_no(self.recorded_coverage_observations)
        );
        text.push_str("## Recorded individual targets\n\nPositive residual means a longer tail or wetter return than requested. Wet values are derived from the saved seed and final return; they are not new audio measurements.\n\n| Bus / group | Requested / recorded decay s | Decay residual s | Effective wet target dB | Predicted wet/source dB | Predicted residual dB | Limit |\n|---|---|---:|---:|---:|---:|---|\n");
        for b in &self.buses {
            let mut limits = Vec::new();
            if b.decay_above_target_at_minimum_control {
                limits.push("Decay still above target at minimum control");
            }
            if b.correction_limited {
                limits.push("Return correction bounded to ±18 dB");
            }
            if b.return_floor_applied {
                limits.push("Return floor −60 dB");
            }
            if limits.is_empty() {
                limits.push(if b.recorded_decay_seconds.is_some() {
                    "No bound established here; decay remains approximate"
                } else {
                    "No return bound reached"
                });
            }
            let value = |v: Option<f64>| v.map(|v| format!("{v:.6}")).unwrap_or_else(|| "—".into());
            let _ = writeln!(
                text,
                "| {} / {} | {} / {} | {} | {:.6} | {:.6} | {:+.6} | {} |",
                cell(&b.bus),
                cell(&b.group),
                value(b.desired_decay_seconds),
                value(b.recorded_decay_seconds),
                value(b.decay_residual_seconds),
                b.effective_target_db,
                b.predicted_calibrated_wet_source_db,
                b.predicted_residual_db,
                limits.join("; ")
            );
        }
        if self.buses.is_empty() {
            text.push_str("\nNo added FX buses were recorded.\n");
        }
        text.push_str("\n## Evidence scope\n\n");
        for limit in &self.limitations {
            let _ = writeln!(text, "- {limit}");
        }
        text.push_str("\n`audit.json` retains the exact source-group assignments, effect controls and hashes of every report read. The original plan, settings and media remain in place.\n");
        text
    }
}
