//! Individual artistic targets are independent of ensemble protection.
use super::{Decision, Frame, FxConfig, Policy, db, inside};
use crate::inventory::Result;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecayRange {
    BelowMeasuredMinimum,
    WithinMeasuredRange,
    AboveMeasuredMaximum,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DecayCalibration {
    pub desired_seconds: f64,
    pub measured_seconds: f64,
    /// Actual minus requested; positive means a longer tail.
    pub residual_seconds: f64,
    pub selected_control: f32,
    pub control_range: [f32; 2],
    pub minimum_control_seconds: f64,
    pub maximum_control_seconds: f64,
    pub target_range: DecayRange,
    pub search_steps: usize,
    pub impulse_capture_seconds: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReturnLimit {
    None,
    MinimumCorrection,
    MaximumCorrection,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReturnCalibration {
    // Preserve the original report keys for existing evidence readers.
    pub bus: String,
    pub measured_seed_wet_source_db: f64,
    pub artistic_profile_target_db: f64,
    pub requested_return_db: f64,
    pub bounded_return_db: f64,
    pub amount: f64,
    pub group: String,
    pub activity_threshold_dbfs: f64,
    pub active_windows: usize,
    pub active_seconds: f64,
    pub correction_limits_db: [f64; 2],
    pub correction_limit: ReturnLimit,
    /// Correction before applying the requested amplitude amount.
    pub correction_db: f64,
    pub amount_db: f64,
    pub return_floor_applied: bool,
    pub effective_target_db: f64,
    pub predicted_wet_source_db: f64,
    /// Prediction minus the amount-adjusted target; positive means wetter.
    pub predicted_residual_db: f64,
    pub note: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BusObservation {
    pub bus: String,
    pub group: String,
    pub split: String,
    pub spans: Vec<[f64; 2]>,
    pub activity_threshold_dbfs: f64,
    pub active_windows: usize,
    pub active_seconds: f64,
    pub effective_target_db: f64,
    pub measured_wet_source_db: Option<f64>,
    /// Measured minus the amount-adjusted target; never selects a retry.
    pub residual_db: Option<f64>,
    pub evidence: String,
}

struct ActiveReturn {
    windows: usize,
    seconds: f64,
    dry: f64,
    wet: f64,
}

fn active_return<'a>(
    frames: impl IntoIterator<Item = &'a Frame>,
    group: usize,
    bus: usize,
    gate: f64,
) -> Result<ActiveReturn> {
    let mut result = ActiveReturn {
        windows: 0,
        seconds: 0.,
        dry: 0.,
        wet: 0.,
    };
    for frame in frames {
        let dry = *frame
            .group_power
            .get(group)
            .ok_or("missing FX source observation")?;
        let wet = *frame
            .return_power
            .get(bus)
            .ok_or("missing FX return observation")?;
        if !dry.is_finite()
            || !wet.is_finite()
            || dry < 0.
            || wet < 0.
            || !frame.start.is_finite()
            || !frame.end.is_finite()
            || frame.end <= frame.start
        {
            return Err("invalid FX calibration observation".into());
        }
        if db(dry.sqrt()) >= gate {
            result.windows += 1;
            result.seconds += frame.end - frame.start;
            // Match the existing equal-window calibration statistic. The report
            // names its support; this is not a full-song or export-level ratio.
            result.dry += dry;
            result.wet += wet;
        }
    }
    Ok(result)
}

pub(super) fn calibrate(
    fx: &mut FxConfig,
    owners: &[usize],
    decisions: &[Decision],
    p: &Policy,
    seed: &[Frame],
) -> Result<Vec<ReturnCalibration>> {
    let mut reports = Vec::new();
    for (i, bus) in fx.buses.iter_mut().enumerate() {
        let owner = owners[i];
        let gate = (decisions[owner].activity.p95_dbfs - 24.).max(-65.);
        let active = active_return(seed, owner, i, gate)?;
        if active.dry <= 1e-12 || active.wet <= 1e-18 {
            return Err(
                "selected FX bus has insufficient measured response; preserve baseline".into(),
            );
        }
        let measured = db((active.wet / active.dry).sqrt());
        let ideal = bus.target_wet_db - measured;
        let correction = ideal.clamp(-18., 18.);
        // Amount is intent, not a level meter: retain its full finite logarithm
        // below the metering floor. The existing final return floor still applies.
        let amount_db = 20. * p.amount.log10();
        let requested = correction + amount_db;
        bus.return_db = requested.max(-60.);
        let effective_target = bus.target_wet_db + amount_db;
        reports.push(ReturnCalibration {
            bus: bus.name.clone(),
            group: p.groups[owner].name.clone(),
            activity_threshold_dbfs: gate,
            active_windows: active.windows,
            active_seconds: active.seconds,
            measured_seed_wet_source_db: measured,
            artistic_profile_target_db: bus.target_wet_db,
            requested_return_db: ideal,
            bounded_return_db: bus.return_db,
            amount: p.amount,
            correction_limits_db: [-18., 18.],
            correction_limit: if ideal < -18. {
                ReturnLimit::MinimumCorrection
            } else if ideal > 18. {
                ReturnLimit::MaximumCorrection
            } else {
                ReturnLimit::None
            },
            correction_db: correction,
            amount_db,
            return_floor_applied: requested < -60.,
            effective_target_db: effective_target,
            predicted_wet_source_db: measured + bus.return_db,
            predicted_residual_db: measured + bus.return_db - effective_target,
            note: "Training-only return calibration at fixed source gain, makeup and faders. Prediction is separate from measured outcomes; targets express artistic intent.".into(),
        });
    }
    Ok(reports)
}

pub(super) fn observe(
    frames: &[Frame],
    owners: &[usize],
    calibration: &[ReturnCalibration],
    spans: &[[f64; 2]],
    split: &str,
) -> Result<Vec<BusObservation>> {
    calibration
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let active = active_return(
                frames.iter().filter(|f| inside(f, spans)),
                owners[i],
                i,
                c.activity_threshold_dbfs,
            )?;
            let enough = active.seconds + 1e-9 >= 0.5 && active.dry > 1e-12;
            let measured =
                (enough && active.wet > 1e-18).then(|| db((active.wet / active.dry).sqrt()));
            Ok(BusObservation {
                bus: c.bus.clone(),
                group: c.group.clone(),
                split: split.into(),
                spans: spans.to_vec(),
                activity_threshold_dbfs: c.activity_threshold_dbfs,
                active_windows: active.windows,
                active_seconds: active.seconds,
                effective_target_db: c.effective_target_db,
                measured_wet_source_db: measured,
                residual_db: measured.map(|v| v - c.effective_target_db),
                evidence: if !enough {
                    "insufficient_active_source"
                } else if measured.is_none() {
                    "insufficient_return_energy"
                } else {
                    "measured"
                }
                .into(),
            })
        })
        .collect()
}
