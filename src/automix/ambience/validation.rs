//! Passage protection requires complete, finite observations, independently of taste.
use super::{Frame, db, inside};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Check {
    pub span: [f64; 2],
    pub dry_rms_dbfs: Option<f64>,
    pub wet_to_dry_db: Option<f64>,
    pub mixed_rms_change_db: Option<f64>,
    pub crest_loss_db: Option<f64>,
    pub peak_change_db: Option<f64>,
    /// Missing in legacy reports; never infer that unmeasured master action was zero.
    #[serde(default)]
    pub max_master_reduction_db: Option<f64>,
    #[serde(default)]
    pub observed_windows: Option<usize>,
    #[serde(default)]
    pub observed_seconds: Option<f64>,
    pub passed: bool,
    #[serde(default)]
    pub failure_reasons: Vec<String>,
}

pub fn checks(frames: &[Frame], spans: &[[f64; 2]]) -> Vec<Check> {
    spans.iter().map(|&span| check(frames, span)).collect()
}

fn check(frames: &[Frame], span: [f64; 2]) -> Check {
    let chosen = frames
        .iter()
        .filter(|f| inside(f, &[span]))
        .collect::<Vec<_>>();
    let mut result = Check {
        span,
        dry_rms_dbfs: None,
        wet_to_dry_db: None,
        mixed_rms_change_db: None,
        crest_loss_db: None,
        peak_change_db: None,
        max_master_reduction_db: None,
        observed_windows: Some(chosen.len()),
        observed_seconds: None,
        passed: false,
        failure_reasons: vec![],
    };
    if chosen.is_empty() {
        result.observed_seconds = Some(0.);
        result
            .failure_reasons
            .push("No measured windows in this passage".into());
        return result;
    }
    if chosen.iter().any(|f| {
        !f.start.is_finite()
            || !f.end.is_finite()
            || f.end <= f.start
            || [
                f.dry_power,
                f.wet_power,
                f.mixed_power,
                f.dry_peak,
                f.mixed_peak,
                f.master_reduction_db,
            ]
            .iter()
            .any(|v| !v.is_finite() || *v < 0.)
    }) {
        result
            .failure_reasons
            .push("Nonfinite or invalid energy, peak or time observation".into());
        return result;
    }
    result.observed_seconds = Some(chosen.iter().map(|f| f.end - f.start).sum());
    // Complete nonoverlapping production windows may leave less than one 20 ms
    // window at each requested boundary. Missing internal windows are not evidence.
    if !span.iter().all(|x| x.is_finite())
        || span[1] <= span[0]
        || chosen[0].start > span[0] + 0.02 + 1e-9
        || chosen.last().unwrap().end < span[1] - 0.02 - 1e-9
        || chosen
            .windows(2)
            .any(|w| (w[1].start - w[0].end).abs() > 1e-9)
        || chosen.iter().any(|f| f.end - f.start > 0.02 + 1e-9)
    {
        result
            .failure_reasons
            .push("Incomplete or overlapping passage coverage".into());
        return result;
    }
    let n = chosen.len() as f64;
    let d = chosen.iter().map(|f| f.dry_power).sum::<f64>() / n;
    let w = chosen.iter().map(|f| f.wet_power).sum::<f64>() / n;
    let m = chosen.iter().map(|f| f.mixed_power).sum::<f64>() / n;
    let dp = chosen.iter().map(|f| f.dry_peak).fold(0., f64::max);
    let mp = chosen.iter().map(|f| f.mixed_peak).fold(0., f64::max);
    let master = chosen
        .iter()
        .map(|f| f.master_reduction_db)
        .fold(0., f64::max);
    if [d, w, m].iter().any(|v| !v.is_finite()) {
        result
            .failure_reasons
            .push("Nonfinite aggregate energy".into());
        return result;
    }
    let rms = db(m.sqrt()) - db(d.sqrt());
    let peak = db(mp) - db(dp);
    let crest = rms - peak;
    let ratio = if d > 1e-13 && w > 0. {
        Some(db((w / d).sqrt()))
    } else {
        None
    };
    result.dry_rms_dbfs = Some(db(d.sqrt()));
    result.wet_to_dry_db = ratio;
    result.mixed_rms_change_db = Some(rms);
    result.peak_change_db = Some(peak);
    result.crest_loss_db = Some(crest);
    result.max_master_reduction_db = Some(master);
    for (failed, reason) in [
        (rms > 2., "Ensemble RMS rise exceeds 2 dB"),
        (peak > 2., "Ensemble peak rise exceeds 2 dB"),
        (crest > 3., "Ensemble crest loss exceeds 3 dB"),
        (
            ratio.is_some_and(|v| v > -12.),
            "Wet ensemble exceeds -12 dB relative to direct ensemble",
        ),
        (master > 0., "FX master reduction must remain zero"),
    ] {
        if failed {
            result.failure_reasons.push(reason.into());
        }
    }
    result.passed = result.failure_reasons.is_empty();
    result
}
