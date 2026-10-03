use super::{
    BusObservation, Check, DecayRange, Decision, ReturnCalibration, ReturnLimit, write_json,
};
use crate::inventory::Result;
use serde::{Deserialize, Serialize};
use std::{fmt::Write as _, fs::OpenOptions, io::Write, path::Path};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Review {
    pub schema_version: u32,
    pub selected: String,
    pub technically_eligible: bool,
    pub reason: String,
    pub decisions: Vec<Decision>,
    pub calibration: Vec<ReturnCalibration>,
    pub bus_observations: Vec<BusObservation>,
    pub training_checks: Vec<Check>,
    pub held_out_checks: Vec<Check>,
    pub listener_accepted: Option<bool>,
    pub full_export_verified: Option<bool>,
    pub export_gain_db: Option<f64>,
    pub limitations: Vec<String>,
}

impl Review {
    pub(super) fn new(
        decisions: Vec<Decision>,
        calibration: Vec<ReturnCalibration>,
        training_checks: Vec<Check>,
    ) -> Self {
        Self {
            schema_version: 1,
            selected: "baseline".into(), technically_eligible: false, reason: String::new(),
            decisions, calibration, bus_observations: vec![], training_checks, held_out_checks: vec![],
            listener_accepted: None, full_export_verified: None, export_gain_db: None,
            limitations: vec![
                "Ensemble protection, individual target attainment, complete export verification and listener acceptance are separate results.".into(),
                "Decay is a six-second filtered engine impulse observation after predelay, not physical room RT60. Six search steps approximate targets within the measured endpoint range; the signed residual remains visible.".into(),
                "Wet/source ratios use active 20 ms windows and each source group's coherent processed signal. Activity gates are frozen from training. Silence or less than 0.5 active seconds leaves the ratio unmeasured.".into(),
                "Positive residual means a longer decay or wetter return. The requested amount changes the wet-level target by 20 log10(amount); control limits can prevent that target.".into(),
                "Held-out bus observations never recalibrate a return or change a recipe. Targets describe artistic intent, not a source defect or preferred sound.".into(),
                "Channel tone, compression, makeup, routing and faders remain frozen. Peak finalization and its common export-gain consequences require a complete render.".into(),
            ],
        }
    }

    pub fn decay_range_limits(&self) -> usize {
        self.decisions
            .iter()
            .filter(|d| {
                d.decay_calibration
                    .as_ref()
                    .is_some_and(|c| c.target_range != DecayRange::WithinMeasuredRange)
            })
            .count()
    }

    pub fn return_limits(&self) -> usize {
        self.calibration
            .iter()
            .filter(|c| c.correction_limit != ReturnLimit::None || c.return_floor_applied)
            .count()
    }

    pub fn summary(&self) -> String {
        format!(
            "{} Decay targets outside measured range: {}. Returns limited by gain bounds: {}. Listener acceptance and full export verification are pending.",
            self.reason,
            self.decay_range_limits(),
            self.return_limits(),
        )
    }

    pub(super) fn write(&self, out: &Path) -> Result<()> {
        write_json(&out.join("review.json"), self)?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(out.join("REVIEW.md"))?;
        file.write_all(self.markdown().as_bytes())?;
        write_json(
            &out.join("selection.json"),
            &serde_json::json!({
                "selected": self.selected, "reason": self.reason,
                "technically_eligible": self.technically_eligible,
                "listener_accepted": null, "full_export_verified": null, "export_gain_db": null,
                "decay_targets_outside_measured_range": self.decay_range_limits(),
                "returns_limited_by_gain_bounds": self.return_limits(),
                "individual_target_review": "review.json",
                "protection_limits": {"wet_ensemble_max_db": -12., "rms_rise_max_db": 2., "peak_rise_max_db": 2., "crest_loss_max_db": 3., "master_reduction_max_db": 0.},
                "limits": "Eligibility does not establish individual target attainment, preferred sound or acoustic safety. Full export/tail verification remains required."
            }),
        )?;
        Ok(())
    }

    pub fn markdown(&self) -> String {
        let mut text = format!(
            "# Artistic FX review\n\n{}\n\nSelected: `{}`. Ensemble eligibility: **{}**.\nListener acceptance: **pending**. Full export verification and export gain: **unmeasured**.\n\n",
            self.reason,
            self.selected,
            if self.technically_eligible {
                "passed"
            } else {
                "failed"
            }
        );
        text.push_str("Source assignments and musical intent are saved in [policy.json](policy.json). The baseline is [before-settings.json](before-settings.json); settings and reports are complete only with a valid [ready.json](ready.json). `ambience-check` verifies that record and the current source files.\n\n## Effect decisions\n\n| Group | Added buses | Reason |\n|---|---|---|\n");
        for d in &self.decisions {
            let buses = if d.bus_names.is_empty() {
                "None".into()
            } else {
                d.bus_names.join(", ")
            };
            let _ = writeln!(text, "| {} | {} | {} |", d.group, buses, d.reason);
        }
        text.push('\n');
        text.push_str("## Decay requests\n\nThese are approximate engine fits. A target inside the measured range can still have a residual.\n\n| Group | Requested s | Measured s | Residual s | Endpoint range s | Status |\n|---|---:|---:|---:|---|---|\n");
        for d in &self.decisions {
            if let Some(c) = &d.decay_calibration {
                let status = match c.target_range {
                    DecayRange::BelowMeasuredMinimum => "Requested decay below engine minimum",
                    DecayRange::AboveMeasuredMaximum => {
                        "Requested decay above measured search range"
                    }
                    DecayRange::WithinMeasuredRange => "Approximate fit within measured range",
                };
                let _ = writeln!(
                    text,
                    "| {} | {:.6} | {:.6} | {:+.6} | {:.6}–{:.6} | {} |",
                    d.group,
                    c.desired_seconds,
                    c.measured_seconds,
                    c.residual_seconds,
                    c.minimum_control_seconds,
                    c.maximum_control_seconds,
                    status
                );
            }
        }
        text.push_str("\n## Return calibration\n\nThe effective target includes the requested amount. Predictions below use the seed measurement; actual outcomes follow.\n\n| Bus | Profile target dB | Amount | Effective target dB | Requested correction dB | Final return dB | Predicted residual dB | Limit |\n|---|---:|---:|---:|---:|---:|---:|---|\n");
        for c in &self.calibration {
            let amount = if c.amount > 0. && c.amount < 0.000001 {
                format!("{:.3e}", c.amount)
            } else {
                format!("{:.6}", c.amount)
            };
            let mut limits = Vec::new();
            match c.correction_limit {
                ReturnLimit::None => {}
                ReturnLimit::MinimumCorrection => limits.push("Minimum −18 dB correction"),
                ReturnLimit::MaximumCorrection => limits.push("Maximum +18 dB correction"),
            }
            if c.return_floor_applied {
                limits.push("−60 dB return floor");
            }
            let limit = if limits.is_empty() {
                "None".into()
            } else {
                limits.join("; ")
            };
            let _ = writeln!(
                text,
                "| {} | {:.3} | {} | {:.3} | {:+.3} | {:+.3} | {:+.3} | {} |",
                c.bus,
                c.artistic_profile_target_db,
                amount,
                c.effective_target_db,
                c.requested_return_db,
                c.bounded_return_db,
                c.predicted_residual_db,
                limit
            );
        }
        text.push_str("\n## Measured bus outcomes\n\nPositive residual means wetter than the amount-adjusted target. A missing observation is not a passed target. Every gate comes from training.\n\n| Bus | Split / passages s | Active s | Wet/source dB | Residual dB | Evidence |\n|---|---|---:|---:|---:|---|\n");
        for b in &self.bus_observations {
            let spans = b
                .spans
                .iter()
                .map(|s| format!("{:.3}–{:.3}", s[0], s[1]))
                .collect::<Vec<_>>()
                .join(", ");
            let value = |v: Option<f64>| {
                v.map(|v| format!("{v:+.3}"))
                    .unwrap_or_else(|| "unmeasured".into())
            };
            let _ = writeln!(
                text,
                "| {} | {} / {} | {:.3} | {} | {} | {} |",
                b.bus,
                b.split,
                spans,
                b.active_seconds,
                value(b.measured_wet_source_db),
                value(b.residual_db),
                b.evidence
            );
        }
        if self.calibration.is_empty() {
            text.push_str("\nNo added buses. Exact baseline settings remain the result.\n");
        }
        text.push_str("\n## Ensemble checks\n\nThese protection checks do not assess musical preference or individual target attainment. The spatial pass requires zero FX master reduction.\n\n| Split / passage s | Wet/dry dB | RMS rise dB | Peak rise dB | Crest loss dB | Master reduction dB | Result |\n|---|---:|---:|---:|---:|---:|---|\n");
        for (split, checks) in [
            ("training", &self.training_checks),
            ("held_out", &self.held_out_checks),
        ] {
            for c in checks {
                let value = |v: Option<f64>| {
                    v.map(|v| format!("{v:.3}"))
                        .unwrap_or_else(|| "unmeasured".into())
                };
                let _ = writeln!(
                    text,
                    "| {} / {:.3}–{:.3} | {} | {} | {} | {} | {} | {} |",
                    split,
                    c.span[0],
                    c.span[1],
                    value(c.wet_to_dry_db),
                    value(c.mixed_rms_change_db),
                    value(c.peak_change_db),
                    value(c.crest_loss_db),
                    value(c.max_master_reduction_db),
                    if c.passed { "passed" } else { "failed" }
                );
            }
        }
        for (split, checks) in [
            ("training", &self.training_checks),
            ("held_out", &self.held_out_checks),
        ] {
            for c in checks.iter().filter(|c| !c.passed) {
                let _ = writeln!(
                    text,
                    "\n{} {:.3}–{:.3} s: {}.\n",
                    split,
                    c.span[0],
                    c.span[1],
                    c.failure_reasons.join("; ")
                );
            }
        }
        text.push_str("\n## Scope and remaining checks\n\n");
        for limit in &self.limitations {
            let _ = writeln!(text, "- {limit}");
        }
        text.push_str("\nUse a fresh output directory for a retry. Existing settings and reports are retained. Playback requires a separate user request.\n");
        text
    }
}
