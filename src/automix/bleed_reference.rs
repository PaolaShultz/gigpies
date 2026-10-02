//! Offline reference-identifiability probes. No processor or settings writer.
//! Predictable energy can include wanted playing; every model remains diagnostic.
use super::{
    bleed::{self, Diagnosis, Measurement, Policy, PredictionProbe},
    config::Session,
    drums::quantile,
    render::write_json,
};
use crate::inventory::Result;
use serde::{Deserialize, Serialize};
use std::{ops::Range, path::Path};

const RIDGE: f64 = 0.001;
const MAX_CONDITION: f64 = 100.;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reference {
    pub stage: usize,
    pub channel: usize,
    /// Positive means the snare follows this reference; negative requires future data.
    pub lag_ms: f64,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Model {
    pub name: String,
    pub analysis_rate: f64,
    pub target_channel: usize,
    pub references: Vec<Reference>,
    pub coefficients: Option<Vec<f64>>,
    pub reference_level: Option<f64>,
    pub training_events: Vec<f64>,
    pub training_samples: usize,
    pub normalized_condition: Option<f64>,
    pub abstention: Option<String>,
}
impl Model {
    fn validate(&self, m: &Measurement) -> Result<()> {
        if !self.analysis_rate.is_finite()
            || self.analysis_rate != m.waveform_analysis_rate
            || self.target_channel > 1
            || self.references.is_empty()
            || self.references.len() > 2
            || self.references.iter().any(|r| {
                ![0, 2].contains(&r.stage)
                    || r.channel > 1
                    || !r.lag_ms.is_finite()
                    || r.lag_ms.abs() > 15.
            })
            || self.reference_level.is_some_and(|x| !x.is_finite())
            || self.coefficients.as_ref().is_some_and(|cs| {
                cs.len() != self.references.len()
                    || cs.iter().any(|x| !x.is_finite() || x.abs() > 4.)
            })
        {
            return Err("invalid frozen reference model or analysis-rate mismatch".into());
        }
        Ok(())
    }
    fn lags(&self) -> Vec<isize> {
        self.references
            .iter()
            .map(|r| (r.lag_ms * self.analysis_rate / 1000.).round() as isize)
            .collect()
    }
}
fn validate_measurement(m: &Measurement, d: &Diagnosis) -> Result<()> {
    if !m.waveform_analysis_rate.is_finite()
        || !(100. ..=192000.).contains(&m.waveform_analysis_rate)
        || m.low_wave.len() as f64 > m.waveform_analysis_rate * 600. + 1.
        || m.low_wave
            .iter()
            .flatten()
            .flatten()
            .any(|x| !x.is_finite())
        || d.events.iter().any(|e| {
            !e.seconds.is_finite()
                || !(0. ..=600.).contains(&e.seconds)
                || !e.raw_attack_db.is_finite()
                || !e.kick_distance_seconds.is_finite()
                || e.held_out != ((e.seconds / 12.).floor() as u64 % 2 == 1)
        })
        || d.events.windows(2).any(|e| e[0].seconds >= e[1].seconds)
    {
        return Err("invalid reference measurement or event timeline".into());
    }
    Ok(())
}
/// Complete target AND shifted reference support must stay in one split and span.
/// Never silently clip a window or borrow held-out samples to fit a training event.
fn support(
    m: &Measurement,
    model: &Model,
    lo: f64,
    hi: f64,
    span: [f64; 2],
) -> Option<Range<usize>> {
    let rate = m.waveform_analysis_rate;
    let a = (lo * rate).round() as isize;
    let b = (hi * rate).round() as isize;
    let lags = model.lags();
    let first = lags.iter().fold(a, |x, lag| x.min(a - lag));
    let last = lags.iter().fold(b, |x, lag| x.max(b - lag));
    if a >= b || first < 0 || last > m.low_wave.len() as isize {
        return None;
    }
    let start = first as f64 / rate;
    let end = last as f64 / rate;
    if start + 1e-8 < span[0]
        || end > span[1] + 1e-8
        || (start / 12.).floor() != ((end - 0.5 / rate) / 12.).floor()
    {
        return None;
    }
    Some(a as usize..b as usize)
}

pub fn from_probe(probe: &PredictionProbe, rate: f64, level: Option<f64>) -> Result<Model> {
    let (Some(c), Some(lag), Some([reference, target])) =
        (probe.coefficient, probe.lag_ms, probe.channel_pair)
    else {
        return Err("missing frozen single-reference parameters".into());
    };
    Ok(Model {
        name: format!("frozen_reference_{}", probe.reference_stage),
        analysis_rate: rate,
        target_channel: target,
        references: vec![Reference {
            stage: probe.reference_stage,
            channel: reference,
            lag_ms: lag,
        }],
        coefficients: Some(vec![c]),
        reference_level: level,
        training_events: vec![],
        training_samples: 0,
        normalized_condition: None,
        abstention: None,
    })
}

/// One joint fit at prior frozen lags/channels. The normalized ridge and condition
/// budget are fixed; numerical fit quality cannot authorize source removal.
pub fn fit_joint(m: &Measurement, d: &Diagnosis, seeds: &[Model], span: [f64; 2]) -> Result<Model> {
    validate_measurement(m, d)?;
    if seeds.len() != 2
        || !span.iter().all(|x| x.is_finite())
        || span[0] < 0.
        || span[1] <= span[0]
        || span[1] > 600.
    {
        return Err("joint fit requires two frozen references and a bounded span".into());
    }
    for s in seeds {
        s.validate(m)?;
    }
    if seeds.iter().any(|s| s.references.len() != 1)
        || seeds[0].target_channel != seeds[1].target_channel
        || seeds[0].references[0].stage == seeds[1].references[0].stage
        || seeds[0].reference_level != seeds[1].reference_level
    {
        return Err(
            "joint references require distinct paths and the same target/reference level".into(),
        );
    }
    let mut model = Model {
        name: "joint_fixed_lags_ridge_0_001".into(),
        analysis_rate: m.waveform_analysis_rate,
        target_channel: seeds[0].target_channel,
        references: seeds.iter().map(|s| s.references[0].clone()).collect(),
        coefficients: None,
        reference_level: seeds[0].reference_level,
        training_events: vec![],
        training_samples: 0,
        normalized_condition: None,
        abstention: Some("insufficient_training_events".into()),
    };
    let Some(level) = model.reference_level else {
        return Ok(model);
    };
    let mut mask = vec![false; m.low_wave.len()];
    for e in d.events.iter().filter(|e| {
        !e.held_out
            && !e.ambiguous_onset
            && e.raw_attack_db < level - 8.
            && e.kick_distance_seconds <= 0.04
    }) {
        if let Some(r) = support(m, &model, e.seconds - 0.02, e.seconds + 0.08, span) {
            model.training_events.push(e.seconds);
            mask[r].fill(true);
        }
    }
    model.training_samples = mask.iter().filter(|x| **x).count();
    if model.training_events.len() < 8 {
        return Ok(model);
    }
    let lags = model.lags();
    let (mut xx, mut xy, mut cross) = ([0.; 2], [0.; 2], 0.);
    for (j, _) in mask.iter().enumerate().filter(|(_, include)| **include) {
        let x: [f64; 2] = std::array::from_fn(|i| {
            let r = &model.references[i];
            m.low_wave[(j as isize - lags[i]) as usize][r.stage][r.channel]
        });
        let y = m.low_wave[j][1][model.target_channel];
        for i in 0..2 {
            xx[i] += x[i] * x[i];
            xy[i] += x[i] * y;
        }
        cross += x[0] * x[1];
    }
    if xx.iter().any(|x| *x < 1e-16) {
        model.abstention = Some("silent_reference".into());
        return Ok(model);
    }
    let rho = (cross / (xx[0] * xx[1]).sqrt()).clamp(-1., 1.);
    let condition = (1. + rho.abs()) / (1. - rho.abs()).max(1e-15);
    model.normalized_condition = Some(condition);
    if condition > MAX_CONDITION {
        model.abstention = Some("collinear_references".into());
        return Ok(model);
    }
    let z = [xy[0] / xx[0].sqrt(), xy[1] / xx[1].sqrt()];
    let a = 1. + RIDGE;
    let determinant = a * a - rho * rho;
    let cs = vec![
        (a * z[0] - rho * z[1]) / determinant / xx[0].sqrt(),
        (a * z[1] - rho * z[0]) / determinant / xx[1].sqrt(),
    ];
    if cs.iter().any(|x| !x.is_finite() || x.abs() > 4.) {
        model.abstention = Some("coefficient_bounds".into());
        return Ok(model);
    }
    model.coefficients = Some(cs);
    model.abstention = None;
    Ok(model)
}

#[derive(Serialize)]
pub struct Audit {
    pub seconds: f64,
    pub held_out: bool,
    pub group: String,
    pub samples: usize,
    pub explained_energy_fraction: Option<f64>,
    pub prediction_energy_fraction: Option<f64>,
    /// Hypothetical low-band residual level, not wanted-signal loss or production DSP.
    pub residual_change_db: Option<f64>,
    pub exclusion: Option<String>,
}
fn audit(m: &Measurement, model: &Model, lo: f64, hi: f64, seconds: f64, group: &str) -> Audit {
    let mut a = Audit {
        seconds,
        held_out: (seconds / 12.).floor() as u64 % 2 == 1,
        group: group.into(),
        samples: 0,
        explained_energy_fraction: None,
        prediction_energy_fraction: None,
        residual_change_db: None,
        exclusion: None,
    };
    let Some(cs) = &model.coefficients else {
        a.exclusion = Some("model_abstained".into());
        return a;
    };
    let Some(range) = support(m, model, lo, hi, [0., 600.]) else {
        a.exclusion = Some("incomplete_or_split_crossing_support".into());
        return a;
    };
    let lags = model.lags();
    let (mut raw, mut residual, mut predicted) = (0., 0., 0.);
    a.samples = range.len();
    for j in range {
        let y = m.low_wave[j][1][model.target_channel];
        let prediction: f64 = model
            .references
            .iter()
            .enumerate()
            .map(|(i, r)| cs[i] * m.low_wave[(j as isize - lags[i]) as usize][r.stage][r.channel])
            .sum();
        raw += y * y;
        predicted += prediction * prediction;
        residual += (y - prediction).powi(2);
    }
    if raw <= 1e-16 {
        a.exclusion = Some(
            if predicted > 1e-16 {
                "silent_target_reference_injects_energy"
            } else {
                "silent_target"
            }
            .into(),
        );
    } else {
        a.explained_energy_fraction = Some(1. - residual / raw);
        a.prediction_energy_fraction = Some(predicted / raw);
        a.residual_change_db = Some(10. * (residual / raw).max(1e-24).log10());
    }
    a
}
#[derive(Serialize)]
pub struct Evaluation {
    pub model: Model,
    pub events: Vec<Audit>,
    pub intervals: Vec<Audit>,
    pub summary: Vec<serde_json::Value>,
    pub decision: &'static str,
    pub production_dsp_validated: bool,
}
pub fn evaluate(m: &Measurement, d: &Diagnosis, model: &Model) -> Result<Evaluation> {
    validate_measurement(m, d)?;
    model.validate(m)?;
    let level = model.reference_level;
    let events = d
        .events
        .iter()
        .map(|e| {
            let group = if e.ambiguous_onset {
                "compound"
            } else if level.is_some_and(|l| e.raw_attack_db >= l - 6.) {
                "strong"
            } else if level.is_some_and(|l| e.raw_attack_db < l - 8.)
                && e.kick_distance_seconds <= 0.04
            {
                "weak_kick_coincident"
            } else {
                "other_quiet"
            };
            audit(
                m,
                model,
                e.seconds - 0.02,
                e.seconds + 0.08,
                e.seconds,
                group,
            )
        })
        .collect::<Vec<_>>();
    // Origin-aligned fixed windows retain silence, long tails and endings with no detected events.
    let intervals = (0..(m.low_wave.len() as f64 / m.waveform_analysis_rate * 10.).floor()
        as usize)
        .map(|i| {
            audit(
                m,
                model,
                i as f64 / 10.,
                (i + 1) as f64 / 10.,
                i as f64 / 10.,
                "fixed_100ms",
            )
        })
        .collect::<Vec<_>>();
    let mut summary = vec![];
    for held in [false, true] {
        for group in [
            "strong",
            "weak_kick_coincident",
            "other_quiet",
            "compound",
            "fixed_100ms",
        ] {
            let rows = events
                .iter()
                .chain(&intervals)
                .filter(|a| a.held_out == held && a.group == group)
                .collect::<Vec<_>>();
            let explained = rows
                .iter()
                .filter_map(|a| a.explained_energy_fraction)
                .collect::<Vec<_>>();
            summary.push(serde_json::json!({"held_out":held,"group":group,"rows":rows.len(),"measured":explained.len(),
                "median_explained_energy_fraction":quantile(explained.clone(),0.5),
                "p10_explained_energy_fraction":quantile(explained,0.1),
                "energy_increase_rows":rows.iter().filter(|a|a.residual_change_db.is_some_and(|x|x>0.)).count(),
                "attenuation_over_0_75_db_rows":rows.iter().filter(|a|a.residual_change_db.is_some_and(|x|x< -0.75)).count(),
                "worst_energy_increase_seconds":rows.iter().filter(|a|a.residual_change_db.is_some()).max_by(|a,b|a.residual_change_db.unwrap().total_cmp(&b.residual_change_db.unwrap())).map(|a|a.seconds)}));
        }
    }
    Ok(Evaluation {
        model: model.clone(),
        events,
        intervals,
        summary,
        decision: "abstain_missing_independent_source_labels",
        production_dsp_validated: false,
    })
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Frozen {
    pub version: u32,
    pub settings: Session,
    pub policy: Policy,
    pub training_span: [f64; 2],
    pub models: Vec<Model>,
}
/// Write models from an explicit training passage before full-song evaluation.
pub fn fit_saved(root: &Path, pilot: &Path, out: &Path, span: [f64; 2]) -> Result<()> {
    if out.exists() {
        return Err("output already exists".into());
    }
    let read = |name: &str| std::fs::File::open(pilot.join(name));
    let settings: Session = serde_json::from_reader(read("before-settings.json")?)?;
    let policy: Policy = serde_json::from_reader(read("policy.json")?)?;
    let d: Diagnosis = serde_json::from_reader(read("diagnosis.json")?)?;
    let probes: Vec<PredictionProbe> = serde_json::from_reader(read("prediction-probes.json")?)?;
    let m = bleed::measure(&settings, root, &policy, span[0], span[1])?;
    let mut models = probes
        .iter()
        .map(|p| from_probe(p, m.waveform_analysis_rate, d.direct_reference_level))
        .collect::<Result<Vec<_>>>()?;
    let joint = fit_joint(&m, &d, &models, span)?;
    models.push(joint);
    write_json(
        out,
        &Frozen {
            version: 1,
            settings,
            policy,
            training_span: span,
            models,
        },
    )
}
pub fn evaluate_saved(root: &Path, review: &Path, frozen: &Path, out: &Path) -> Result<()> {
    if out.exists() {
        return Err("output already exists".into());
    }
    let f: Frozen = serde_json::from_reader(std::fs::File::open(frozen)?)?;
    if f.version != 1 || f.models.len() != 3 {
        return Err("invalid frozen experiment".into());
    }
    let s: Session =
        serde_json::from_reader(std::fs::File::open(review.join("before-settings.json"))?)?;
    let p: Policy = serde_json::from_reader(std::fs::File::open(review.join("policy.json"))?)?;
    if serde_json::to_value(&s)? != serde_json::to_value(&f.settings)?
        || serde_json::to_value(&p)? != serde_json::to_value(&f.policy)?
    {
        return Err("frozen experiment settings or identities differ from review".into());
    }
    let d: Diagnosis =
        serde_json::from_reader(std::fs::File::open(review.join("diagnosis.json"))?)?;
    let m = bleed::measure(&f.settings, root, &f.policy, 0., 600.)?;
    let evaluation = f
        .models
        .iter()
        .map(|model| evaluate(&m, &d, model))
        .collect::<Result<Vec<_>>>()?;
    write_json(out, &evaluation)
}
