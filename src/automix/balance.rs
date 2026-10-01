//! Whole-recording preparation, separate from causal soundcheck.
//! Role-conditioned energy ratios are engineering proxies, not auditory masking.
use super::{
    config::{OutputMode, Role, Session},
    dsp::{Biquad, Strip, db, gain},
    render::{Source, route},
    write_json,
};
use crate::inventory::Result;
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};

const BANDS: usize = 5; // broadband, 40–120, 120–500, 500–1500, 1500–5000 Hz
const EDGES: [f64; 5] = [40., 120., 500., 1500., 5000.];
fn percentile(x: &[f64], p: f64) -> Option<f64> {
    if x.is_empty() {
        return None;
    }
    let mut x = x.to_vec();
    x.sort_by(f64::total_cmp);
    Some(x[((x.len() - 1) as f64 * p).round() as usize])
}
fn power_db(x: f64) -> f64 {
    db(x.max(0.).sqrt())
}
fn percussion(r: Role) -> bool {
    matches!(r, Role::Kick | Role::Snare | Role::Tom)
}
fn guitar(r: Role) -> bool {
    matches!(r, Role::RhythmGuitar | Role::LeadGuitar)
}
fn kit(r: Role) -> bool {
    percussion(r) || matches!(r, Role::Overheads | Role::DrumRoom)
}
fn event_band(r: Role) -> usize {
    if matches!(r, Role::Kick) { 1 } else { 2 }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Relationship {
    pub name: String,
    pub numerator: Vec<usize>,
    pub denominator: Vec<usize>,
    pub activity_source: usize,
    pub band: usize,
    pub event_only: bool,
    pub kit_stage: bool,
    pub range_db: [f64; 2],
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub description: String,
    pub relationships: Vec<Relationship>,
    pub bound_db: f64,
    pub step_db: f64,
    pub evaluations_per_stage: usize,
    pub change_penalty: f64,
    pub max_regression_db: f64,
    pub minimum_windows: usize,
    pub minimum_confidence: f64,
    pub activity_floor_dbfs: f64,
    pub relative_activity_db: f64,
    pub onset_rise_db: f64,
    pub section_seconds: f64,
}
impl Policy {
    pub fn for_session(s: &Session) -> Self {
        let select = |f: fn(Role) -> bool| {
            s.channels
                .iter()
                .enumerate()
                .filter(|(_, c)| f(c.role))
                .map(|(i, _)| i)
                .collect::<Vec<_>>()
        };
        let close = select(percussion);
        let drums = select(kit);
        let guitars = select(guitar);
        let bass = select(|r| matches!(r, Role::BassDi));
        let vocal = select(|r| matches!(r, Role::LeadVocal));
        let oh = select(|r| matches!(r, Role::Overheads));
        let room = select(|r| matches!(r, Role::DrumRoom));
        let mut relationships = Vec::new();
        let mut add = |name: String, n: Vec<usize>, d: Vec<usize>, a: usize, b, e, k, range| {
            if !n.is_empty() && !d.is_empty() {
                relationships.push(Relationship {
                    name,
                    numerator: n,
                    denominator: d,
                    activity_source: a,
                    band: b,
                    event_only: e,
                    kit_stage: k,
                    range_db: range,
                });
            }
        };
        for &i in &close {
            let c = &s.channels[i];
            add(
                format!("{} impact / overhead", c.file.display()),
                vec![i],
                oh.clone(),
                i,
                event_band(c.role),
                true,
                true,
                [-3., 12.],
            );
            let others = bass.iter().chain(&guitars).copied().collect();
            add(
                format!("{} impact / band", c.file.display()),
                vec![i],
                others,
                i,
                event_band(c.role),
                true,
                false,
                [-6., 12.],
            );
        }
        if let Some(&i) = close.first() {
            add(
                "drum room / close hits".into(),
                room,
                close.clone(),
                i,
                0,
                true,
                true,
                [-24., -9.],
            );
        }
        if let Some(&i) = bass.first() {
            add(
                "bass foundation / guitars".into(),
                bass.clone(),
                guitars.clone(),
                i,
                1,
                false,
                false,
                [-3., 15.],
            );
        }
        for &i in &guitars {
            add(
                format!("{} body / drums", s.channels[i].file.display()),
                vec![i],
                drums.clone(),
                i,
                2,
                false,
                false,
                [-15., 0.],
            );
        }
        if let Some(&i) = vocal.first() {
            add(
                "vocal presence / competing guitars".into(),
                vocal.clone(),
                guitars,
                i,
                4,
                false,
                false,
                [0., 12.],
            );
            add(
                "vocal room / direct phrases".into(),
                select(|r| matches!(r, Role::VocalRoom)),
                vocal,
                i,
                0,
                false,
                true,
                [-24., -12.],
            );
        }
        Self{description:"Initial engineering hypotheses, not universal correct-mix values. Ratios use simultaneous routed energy; no auditory-model claim.".into(),relationships,bound_db:6.,step_db:0.5,evaluations_per_stage:512,change_penalty:0.04,max_regression_db:1.,minimum_windows:8,minimum_confidence:0.65,activity_floor_dbfs:-75.,relative_activity_db:30.,onset_rise_db:5.,section_seconds:12.}
    }
    pub fn validate(&self, n: usize) -> Result<()> {
        if !self.bound_db.is_finite()
            || !(0.0..=9.).contains(&self.bound_db)
            || !self.step_db.is_finite()
            || !(0.25..=2.).contains(&self.step_db)
            || !(1..=4096).contains(&self.evaluations_per_stage)
            || !self.change_penalty.is_finite()
            || !(0.001..=1.).contains(&self.change_penalty)
            || !self.max_regression_db.is_finite()
            || !(0.0..=2.).contains(&self.max_regression_db)
            || !(2..=1000).contains(&self.minimum_windows)
            || !self.minimum_confidence.is_finite()
            || !(0.5..=1.).contains(&self.minimum_confidence)
            || !self.activity_floor_dbfs.is_finite()
            || !(-100.0..=-40.).contains(&self.activity_floor_dbfs)
            || !self.relative_activity_db.is_finite()
            || !(12.0..=48.).contains(&self.relative_activity_db)
            || !self.onset_rise_db.is_finite()
            || !(3.0..=15.).contains(&self.onset_rise_db)
            || !self.section_seconds.is_finite()
            || !(2.0..=30.).contains(&self.section_seconds)
        {
            return Err("invalid balance policy bounds".into());
        }
        for r in &self.relationships {
            if r.numerator.is_empty()
                || r.denominator.is_empty()
                || r.band >= BANDS
                || r.activity_source >= n
                || r.numerator.iter().chain(&r.denominator).any(|&i| i >= n)
                || r.numerator.iter().any(|i| r.denominator.contains(i))
                || r.range_db.iter().any(|x| !x.is_finite() || x.abs() > 60.)
                || r.range_db[0] > r.range_db[1]
            {
                return Err("invalid balance relationship".into());
            }
        }
        Ok(())
    }
}
#[derive(Clone, Default, Serialize)]
pub struct ChannelWindow {
    pub pre_compressor_dbfs: f64,
    pub post_compressor_dbfs: f64,
    pub input_dbfs: f64,
    pub peak_dbfs: f64,
    pub mean_reduction_db: f64,
    pub max_reduction_db: f64,
    pub stereo_correlation: f64,
    pub active: bool,
    pub confidence: f64,
    pub onset: bool,
    pub bleed_ambiguous: bool,
}
#[derive(Clone)]
pub struct Window {
    pub start_frame: u64,
    pub frames: u64,
    pub channels: Vec<ChannelWindow>,
    // [band][source][source], mean stereo product; last source is summed generated FX.
    pub covariance: Vec<f64>,
    pub master_mean_reduction_db: f64,
    pub master_max_reduction_db: f64,
    pub master_dbfs: f64,
}
#[derive(Clone)]
pub struct Measurement {
    pub windows: Vec<Window>,
    pub signals: usize,
    pub rate: u32,
}
impl Measurement {
    pub fn energy(&self, w: &Window, indices: &[usize], band: usize, gains: &[f64]) -> f64 {
        let n = self.signals;
        indices
            .iter()
            .flat_map(|&i| {
                indices
                    .iter()
                    .map(move |&j| w.covariance[(band * n + i) * n + j] * gains[i] * gains[j])
            })
            .sum::<f64>()
            .max(0.)
    }
}
/// Refuse incompatible baselines rather than silently rewriting their processing.
pub fn validate_baseline(s: &Session) -> Result<()> {
    s.validate()?;
    if !s.prepared
        || s.output_mode != OutputMode::Unmatched
        || s.master_hpf_hz != 40.
        || s.ceiling_db != -0.01
        || s.calibration.initial_trim_db != 0.
        || s.groups.iter().any(|g| g.trim_db != 0.)
        || s.channels.iter().any(|c| {
            matches!(c.role, Role::BassAmp)
                || c.hpf_hz
                    != if matches!(c.role, Role::Kick | Role::BassDi) {
                        0.
                    } else {
                        90.
                    }
        })
    {
        return Err("balance requires prepared DI-only unity trims, 90/40 Hz HPFs and unmatched -0.01 dBFS export".into());
    }
    Ok(())
}
/// Native-rate linked DSP, unchanged metadata offsets and pan. Only features are buffered.
pub fn measure(s: &Session, root: &Path, p: &Policy) -> Result<Measurement> {
    validate_baseline(s)?;
    p.validate(s.channels.len())?;
    let mut sources = s
        .channels
        .iter()
        .map(|c| Source::open(&root.join(&c.file), s.sample_rate))
        .collect::<Result<Vec<_>>>()?;
    let refs = sources
        .iter()
        .filter_map(|s| s.reference)
        .collect::<Vec<_>>();
    if !refs.is_empty() && refs.len() != sources.len() {
        return Err("mixed BWF references".into());
    }
    let origin = refs.iter().min().copied().unwrap_or(0);
    for src in &mut sources {
        src.offset = src.reference.unwrap_or(0) - origin;
    }
    let total = sources
        .iter()
        .map(|x| x.offset.checked_add(x.frames).ok_or("timeline overflow"))
        .collect::<std::result::Result<Vec<_>, _>>()?
        .into_iter()
        .max()
        .unwrap_or(0);
    if total == 0 || total > u64::from(s.sample_rate) * 86400 {
        return Err("empty or >24h timeline".into());
    }
    let count = s.channels.len();
    let n = count + 1;
    let mut strips = s
        .channels
        .iter()
        .map(|c| Strip::new(c, s.sample_rate))
        .collect::<Vec<_>>();
    let mut flat = s
        .channels
        .iter()
        .map(|c| {
            let mut c = c.clone();
            c.compressor.ratio = 1.;
            c.compressor.makeup_db = 0.;
            Strip::new(&c, s.sample_rate)
        })
        .collect::<Vec<_>>();
    let mut rack = s
        .effects
        .as_ref()
        .map(|fx| super::effects::Rack::new(fx, count, s.sample_rate));
    let mut filters = vec![
        EDGES.map(|hz| [Biquad::highpass(
            hz.min(s.sample_rate as f64 * 0.44),
            std::f64::consts::FRAC_1_SQRT_2,
            s.sample_rate
        ); 2]);
        n
    ];
    let mut master_hp = [Biquad::highpass(
        s.master_hpf_hz,
        std::f64::consts::FRAC_1_SQRT_2,
        s.sample_rate,
    ); 2];
    let hop = (s.sample_rate / 50) as u64;
    if total.div_ceil(hop).saturating_mul((BANDS * n * n) as u64) > 64_000_000 {
        return Err(
            "balance covariance budget exceeds 512 MB; use a shorter preparation session".into(),
        );
    }
    let mut windows = Vec::new();
    let mut t = 0;
    while t < total {
        let frames = hop.min(total - t);
        let mut master_mean_reduction_db = 0.;
        let mut master_max_reduction_db = 0_f64;
        let mut master_power = 0.;
        let mut covariance = vec![0.; BANDS * n * n];
        let mut cw = vec![ChannelWindow::default(); count];
        let mut in_power = vec![0.; count];
        let mut pre = vec![0.; count];
        let mut post = vec![0.; count];
        let mut lr = vec![[0.; 3]; count];
        let mut values = vec![[[0.; 2]; BANDS]; n];
        for frame in t..t + frames {
            for i in 0..count {
                let x = sources[i].next(frame)?;
                let before = flat[i].tick(x);
                let after = strips[i].tick(x);
                in_power[i] += (x[0] * x[0] + x[1] * x[1]) / 2.;
                pre[i] += (before[0] * before[0] + before[1] * before[1]) / 2.;
                post[i] += (after[0] * after[0] + after[1] * after[1]) / 2.;
                cw[i].peak_dbfs = cw[i].peak_dbfs.max(after[0].abs().max(after[1].abs()));
                cw[i].mean_reduction_db += strips[i].reduction_db();
                cw[i].max_reduction_db = cw[i].max_reduction_db.max(strips[i].reduction_db());
                lr[i][0] += after[0] * after[0];
                lr[i][1] += after[1] * after[1];
                lr[i][2] += after[0] * after[1];
                let excited = if let Some(r) = &mut rack {
                    r.excite(i, after)
                } else {
                    after
                };
                let y = route(
                    excited,
                    sources[i].channels,
                    s.channels[i].pan,
                    gain(s.channels[i].fader_db),
                );
                if let Some(r) = &mut rack {
                    r.send(i, y);
                }
                values[i][0] = y;
            }
            values[count][0] = rack.as_mut().map(|r| r.returns()).unwrap_or([0.; 2]);
            let sum = std::array::from_fn(|c| {
                master_hp[c].tick(values.iter().map(|v| v[0][c]).sum::<f64>() * gain(s.master_db))
            });
            let processed = if let Some(r) = &mut rack {
                let y = r.master(sum);
                master_mean_reduction_db += r.reduction_db();
                master_max_reduction_db = master_max_reduction_db.max(r.reduction_db());
                y
            } else {
                sum
            };
            master_power += (processed[0] * processed[0] + processed[1] * processed[1]) / 2.;
            for i in 0..n {
                let x = values[i][0];
                let mut hp = [[0.; 2]; 5];
                for (b, f) in filters[i].iter_mut().enumerate() {
                    for c in 0..2 {
                        hp[b][c] = f[c].tick(x[c]);
                    }
                }
                for b in 1..BANDS {
                    for c in 0..2 {
                        values[i][b][c] = hp[b - 1][c] - hp[b][c];
                    }
                }
            }
            for b in 0..BANDS {
                for i in 0..n {
                    for j in 0..=i {
                        let v = (values[i][b][0] * values[j][b][0]
                            + values[i][b][1] * values[j][b][1])
                            / 2.;
                        covariance[(b * n + i) * n + j] += v;
                    }
                }
            }
        }
        for b in 0..BANDS {
            for i in 0..n {
                for j in 0..=i {
                    let v = covariance[(b * n + i) * n + j] / frames as f64;
                    covariance[(b * n + i) * n + j] = v;
                    covariance[(b * n + j) * n + i] = v;
                }
            }
        }
        for i in 0..count {
            cw[i].input_dbfs = power_db(in_power[i] / frames as f64);
            cw[i].pre_compressor_dbfs = power_db(pre[i] / frames as f64);
            cw[i].post_compressor_dbfs = power_db(post[i] / frames as f64);
            cw[i].peak_dbfs = db(cw[i].peak_dbfs);
            cw[i].mean_reduction_db /= frames as f64;
            cw[i].stereo_correlation = lr[i][2] / (lr[i][0] * lr[i][1]).sqrt().max(1e-24);
        }
        windows.push(Window {
            start_frame: t,
            frames,
            channels: cw,
            covariance,
            master_mean_reduction_db: master_mean_reduction_db / frames as f64,
            master_max_reduction_db,
            master_dbfs: power_db(master_power / frames as f64),
        });
        t += frames;
    }
    let mut m = Measurement {
        windows,
        signals: n,
        rate: s.sample_rate,
    };
    classify(&mut m, s, p);
    Ok(m)
}
/// Explicit tom-to-kick/snare relationships: coincident weaker events are uncertain.
/// This rejects likely bleed but cannot prove source identity or remove bleed audio.
pub fn classify(m: &mut Measurement, s: &Session, p: &Policy) {
    let n = s.channels.len();
    let gains = vec![1.; m.signals];
    let levels = (0..n)
        .map(|i| {
            m.windows
                .iter()
                .map(|w| power_db(m.energy(w, &[i], event_band(s.channels[i].role), &gains)))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let mut onset = vec![vec![false; m.windows.len()]; n];
    for i in 0..n {
        let all = m
            .windows
            .iter()
            .map(|w| w.channels[i].pre_compressor_dbfs)
            .collect::<Vec<_>>();
        let high = percentile(&all, 0.95).unwrap_or(-240.);
        let low = percentile(&all, 0.1).unwrap_or(-240.);
        let floor = p.activity_floor_dbfs.max(high - p.relative_activity_db);
        let mut last = None;
        for k in 0..m.windows.len() {
            let level = all[k];
            let cw = &mut m.windows[k].channels[i];
            cw.active = level > floor;
            // Separated quiet floor raises confidence; sustained playing remains eligible.
            cw.confidence = if !cw.active {
                0.
            } else if level > low + 6. || low > p.activity_floor_dbfs + 3. {
                0.85
            } else {
                0.55
            };
            if percussion(s.channels[i].role) && k >= 3 {
                let previous = levels[i][k - 3..k].iter().sum::<f64>() / 3.;
                if cw.active
                    && levels[i][k] - previous >= p.onset_rise_db
                    && last.is_none_or(|j| k - j >= 6)
                {
                    let peak = (k..(k + 3).min(m.windows.len()))
                        .max_by(|&a, &b| levels[i][a].total_cmp(&levels[i][b]))
                        .unwrap();
                    onset[i][peak] = true;
                    last = Some(k);
                }
            }
        }
    }
    for i in 0..n {
        // Learn only an ambiguity reference from coincident kick/snare events.
        // Repeated tom+snare unisons remain uncertain rather than becoming a boost.
        let transfer = (0..n)
            .map(|j| {
                if !matches!(s.channels[j].role, Role::Kick | Role::Snare) {
                    return None;
                }
                let ratios = (0..m.windows.len())
                    .filter(|&k| onset[j][k])
                    .map(|k| levels[i][k] - levels[j][k])
                    .collect::<Vec<_>>();
                if ratios.len() < 3 {
                    None
                } else {
                    percentile(&ratios, 0.5)
                }
            })
            .collect::<Vec<_>>();
        for k in 0..m.windows.len() {
            let ambiguous = matches!(s.channels[i].role, Role::Tom)
                && onset[i][k]
                && (0..n).any(|j| {
                    transfer[j].is_some_and(|reference| {
                        (k.saturating_sub(1)..=(k + 1).min(m.windows.len() - 1))
                            .any(|a| onset[j][a] && levels[i][k] - levels[j][a] <= reference + 6.)
                    })
                });
            let cw = &mut m.windows[k].channels[i];
            cw.bleed_ambiguous = ambiguous;
            cw.onset = onset[i][k] && !ambiguous;
            if ambiguous {
                cw.confidence = 0.35;
                cw.active = false;
            }
        }
        if matches!(s.channels[i].role, Role::Tom) {
            let mut last_hit = None;
            for (k, w) in m.windows.iter_mut().enumerate() {
                let c = &mut w.channels[i];
                if c.onset {
                    last_hit = Some(k);
                }
                if last_hit.is_none_or(|hit| k - hit >= 12) {
                    c.active = false;
                    c.confidence = c.confidence.min(0.35);
                }
            }
        }
    }
}
#[derive(Clone, Serialize)]
pub struct TargetResult {
    pub name: String,
    pub windows: usize,
    pub confidence: f64,
    pub median_db: Option<f64>,
    pub p10_db: Option<f64>,
    pub p90_db: Option<f64>,
    pub range_db: [f64; 2],
    pub violation_db: Option<f64>,
}
fn violation(x: f64, r: [f64; 2]) -> f64 {
    (r[0] - x).max(x - r[1]).max(0.)
}
fn chosen(w: &Window, r: &Relationship, p: &Policy) -> bool {
    let c = &w.channels[r.activity_source];
    c.active && c.confidence >= p.minimum_confidence && (!r.event_only || c.onset)
}
pub fn evaluate(m: &Measurement, p: &Policy, deltas: &[f64], holdout: bool) -> Vec<TargetResult> {
    evaluate_subset(m, p, deltas, holdout, None)
}
fn evaluate_subset(
    m: &Measurement,
    p: &Policy,
    deltas: &[f64],
    holdout: bool,
    only_section: Option<usize>,
) -> Vec<TargetResult> {
    let mut g = deltas.iter().map(|&d| gain(d)).collect::<Vec<_>>();
    g.resize(m.signals, 1.);
    p.relationships
        .iter()
        .map(|r| {
            let mut values = Vec::new();
            let mut conf = 0.;
            for w in &m.windows {
                let section =
                    (w.start_frame as f64 / m.rate as f64 / p.section_seconds).floor() as usize;
                if only_section.is_some_and(|s| s != section)
                    || (section % 2 == 1) != holdout
                    || !chosen(w, r, p)
                {
                    continue;
                }
                // The same frame window is used for both terms. Denominator must be active.
                if !r.denominator.iter().any(|&i| w.channels[i].active) {
                    continue;
                }
                let a = m.energy(w, &r.numerator, r.band, &g);
                let b = m.energy(w, &r.denominator, r.band, &g);
                if a > 1e-20 && b > 1e-20 {
                    values.push(power_db(a) - power_db(b));
                    conf += w.channels[r.activity_source].confidence;
                }
            }
            let median = percentile(&values, 0.5);
            let enough = values.len() >= p.minimum_windows;
            TargetResult {
                name: r.name.clone(),
                windows: values.len(),
                confidence: if values.is_empty() {
                    0.
                } else {
                    conf / values.len() as f64
                },
                median_db: median,
                p10_db: percentile(&values, 0.1),
                p90_db: percentile(&values, 0.9),
                range_db: r.range_db,
                violation_db: if enough {
                    median.map(|x| violation(x, r.range_db))
                } else {
                    None
                },
            }
        })
        .collect()
}
fn cost(results: &[TargetResult], g: &[f64], p: &Policy, kit_stage: bool) -> f64 {
    results
        .iter()
        .zip(&p.relationships)
        .filter(|(_, r)| !kit_stage || r.kit_stage)
        .filter_map(|(r, _)| r.violation_db)
        .map(|v| v * v)
        .sum::<f64>()
        + p.change_penalty * g.iter().map(|x| x * x).sum::<f64>()
}
fn guarded(before: &[TargetResult], after: &[TargetResult], p: &Policy) -> bool {
    before
        .iter()
        .zip(after)
        .all(|(a, b)| match (a.violation_db, b.violation_db) {
            (Some(a), Some(b)) => b <= a + p.max_regression_db + 1e-8,
            (None, _) => true,
            _ => false,
        })
}
#[derive(Serialize)]
pub struct SectionResult {
    pub section: usize,
    pub held_out: bool,
    pub before: Vec<TargetResult>,
    pub after: Vec<TargetResult>,
    pub no_unacceptable_regression: bool,
}
fn section_results(m: &Measurement, p: &Policy, deltas: &[f64]) -> Vec<SectionResult> {
    let count = m
        .windows
        .last()
        .map(|w| (w.start_frame as f64 / m.rate as f64 / p.section_seconds).floor() as usize + 1)
        .unwrap_or(0);
    (0..count)
        .map(|section| {
            let before = evaluate_subset(
                m,
                p,
                &vec![0.; deltas.len()],
                section % 2 == 1,
                Some(section),
            );
            let after = evaluate_subset(m, p, deltas, section % 2 == 1, Some(section));
            let no_unacceptable_regression = guarded(&before, &after, p);
            SectionResult {
                section,
                held_out: section % 2 == 1,
                before,
                after,
                no_unacceptable_regression,
            }
        })
        .collect()
}
#[derive(Serialize)]
pub struct Optimization {
    pub deltas_db: Vec<f64>,
    pub evaluations: usize,
    pub accepted: bool,
    pub sections: Vec<SectionResult>,
    pub fit_before: Vec<TargetResult>,
    pub fit_after: Vec<TargetResult>,
    pub held_out_before: Vec<TargetResult>,
    pub held_out_after: Vec<TargetResult>,
    pub reason: String,
}
/// Deterministic coordinate search: kit/direct-room first, then linked musical groups.
/// Held-out windows never choose a coordinate; they can veto the frozen result.
pub fn optimize(m: &Measurement, s: &Session, p: &Policy) -> Result<Optimization> {
    p.validate(s.channels.len())?;
    let zero = vec![0.; s.channels.len()];
    let mut g = zero.clone();
    let mut evaluations = 0;
    let fit_before = evaluate(m, p, &g, false);
    let held_out_before = evaluate(m, p, &g, true);
    for stage in [true, false] {
        let moves = if stage {
            s.channels
                .iter()
                .enumerate()
                .filter(|(_, c)| kit(c.role) || matches!(c.role, Role::VocalRoom))
                .map(|(i, _)| vec![i])
                .collect::<Vec<_>>()
        } else {
            [
                kit as fn(Role) -> bool,
                |r| matches!(r, Role::BassDi),
                |r| matches!(r, Role::RhythmGuitar),
                |r| matches!(r, Role::LeadGuitar),
                |r| matches!(r, Role::LeadVocal),
            ]
            .iter()
            .map(|f| {
                s.channels
                    .iter()
                    .enumerate()
                    .filter(|(_, c)| f(c.role))
                    .map(|(i, _)| i)
                    .collect::<Vec<_>>()
            })
            .filter(|x| !x.is_empty())
            .collect()
        };
        let mut used = 0;
        while used < p.evaluations_per_stage {
            let before = evaluate(m, p, &g, false);
            let current = cost(&before, &g, p, stage);
            let mut best = current;
            let mut next = None;
            for indices in &moves {
                for direction in [-1., 1.] {
                    if used >= p.evaluations_per_stage {
                        break;
                    }
                    let mut trial = g.clone();
                    for &i in indices {
                        trial[i] += direction * p.step_db;
                    }
                    if trial.iter().enumerate().any(|(i, v)| {
                        v.abs() > p.bound_db + 1e-9
                            || !(-80.0..=12.).contains(&(s.channels[i].fader_db + v))
                    }) {
                        continue;
                    }
                    let after = evaluate(m, p, &trial, false);
                    used += 1;
                    let score = cost(&after, &trial, p, stage);
                    if score < best - 1e-8 && guarded(&fit_before, &after, p) {
                        best = score;
                        next = Some(trial);
                    }
                }
            }
            if let Some(next) = next {
                g = next;
            } else {
                break;
            }
        }
        evaluations += used;
    }
    let fit_after = evaluate(m, p, &g, false);
    let held_out_after = evaluate(m, p, &g, true);
    let improves = cost(&fit_after, &g, p, false) < cost(&fit_before, &zero, p, false) - 1e-8;
    let held_out_cost = |r: &[TargetResult]| {
        r.iter()
            .filter_map(|r| r.violation_db)
            .map(|v| v * v)
            .sum::<f64>()
    };
    let evidence = p
        .relationships
        .iter()
        .zip(&fit_before)
        .zip(&held_out_before)
        .any(|((_, a), b)| a.violation_db.is_some() && b.violation_db.is_some());
    // An unrelated well-measured instrument cannot stand in for missing validation
    // of the relationship whose improvement justified a move.
    let improved_targets_have_holdout =
        fit_before
            .iter()
            .zip(&fit_after)
            .enumerate()
            .all(|(i, (a, b))| match (a.violation_db, b.violation_db) {
                (Some(a), Some(b)) if b < a - 1e-8 => {
                    held_out_before[i].violation_db.is_some()
                        && held_out_after[i].violation_db.is_some()
                }
                _ => true,
            });
    let sections = section_results(m, p, &g);
    let accepted = sections.iter().all(|s| s.no_unacceptable_regression)
        && improves
        && evidence
        && improved_targets_have_holdout
        && guarded(&held_out_before, &held_out_after, p)
        && held_out_cost(&held_out_after) <= held_out_cost(&held_out_before) + 1e-8;
    Ok(Optimization{sections,deltas_db:if accepted{g}else{zero},evaluations,accepted,fit_before,fit_after,held_out_before,held_out_after,reason:if accepted{"Fit improves; held-out aggregate does not regress and each measured relationship stays within the regression budget."}else{"No accepted change: insufficient evidence, no necessary improvement, or held-out regression. Proposed results retained for diagnosis."}.into()})
}

pub fn save_measurement(m: &Measurement, s: &Session, p: &Policy, out: &Path) -> Result<()> {
    std::fs::create_dir(out)?;
    let g = vec![1.; m.signals];
    let mut file = BufWriter::new(File::create(out.join("windows.csv"))?);
    writeln!(
        file,
        "start_frame,frames,channel,input_dbfs,pre_compressor_dbfs,post_compressor_dbfs,peak_dbfs,mean_reduction_db,max_reduction_db,stereo_correlation,active,confidence,onset,bleed_ambiguous,broadband_dbfs,40_120_dbfs,120_500_dbfs,500_1500_dbfs,1500_5000_dbfs"
    )?;
    let mut master = BufWriter::new(File::create(out.join("master.csv"))?);
    writeln!(
        master,
        "start_frame,mean_reduction_db,max_reduction_db,pre_export_rms_dbfs"
    )?;
    for w in &m.windows {
        writeln!(
            master,
            "{},{:.5},{:.5},{:.5}",
            w.start_frame, w.master_mean_reduction_db, w.master_max_reduction_db, w.master_dbfs
        )?;
    }
    master.flush()?;
    let mut summaries = Vec::new();
    let mut events = Vec::new();
    for (i, ch) in s.channels.iter().enumerate() {
        let active = m
            .windows
            .iter()
            .filter(|w| w.channels[i].active && w.channels[i].confidence >= p.minimum_confidence)
            .collect::<Vec<_>>();
        let levels = active
            .iter()
            .map(|w| w.channels[i].post_compressor_dbfs)
            .collect::<Vec<_>>();
        let reductions = active
            .iter()
            .map(|w| w.channels[i].mean_reduction_db)
            .collect::<Vec<_>>();
        let mut phrase_active = false;
        for (k, w) in m.windows.iter().enumerate() {
            let c = &w.channels[i];
            let bands = (0..BANDS)
                .map(|b| power_db(m.energy(w, &[i], b, &g)))
                .collect::<Vec<_>>();
            writeln!(
                file,
                "{},{},{},{:.5},{:.5},{:.5},{:.5},{:.5},{:.5},{:.6},{},{:.2},{},{},{:.5},{:.5},{:.5},{:.5},{:.5}",
                w.start_frame,
                w.frames,
                i,
                c.input_dbfs,
                c.pre_compressor_dbfs,
                c.post_compressor_dbfs,
                c.peak_dbfs,
                c.mean_reduction_db,
                c.max_reduction_db,
                c.stereo_correlation,
                c.active,
                c.confidence,
                c.onset,
                c.bleed_ambiguous,
                bands[0],
                bands[1],
                bands[2],
                bands[3],
                bands[4]
            )?;
            let phrase = matches!(ch.role, Role::LeadVocal) && c.active && !phrase_active;
            phrase_active = c.active;
            if c.onset || phrase {
                let spans = if phrase {
                    [(0, 5), (5, 15), (15, 30)]
                } else {
                    [(0, 1), (1, 4), (4, 12)]
                };
                let envelope=spans.iter().map(|&(a,b)|{
                    let rows=&m.windows[(k+a).min(m.windows.len())..(k+b).min(m.windows.len())];
                    let mean=|f:fn(&ChannelWindow)->f64|if rows.is_empty(){None}else{Some(rows.iter().map(|w|f(&w.channels[i])).sum::<f64>()/rows.len() as f64)};
                    let power=|f:fn(&ChannelWindow)->f64|if rows.is_empty(){None}else{Some(power_db(rows.iter().map(|w|gain(f(&w.channels[i])).powi(2)).sum::<f64>()/rows.len() as f64))};
                    serde_json::json!({"start_ms":a*20,"end_ms":b*20,"pre_compressor_dbfs":power(|c|c.pre_compressor_dbfs),"post_compressor_dbfs":power(|c|c.post_compressor_dbfs),"mean_reduction_db":mean(|c|c.mean_reduction_db),"max_reduction_db":rows.iter().map(|w|w.channels[i].max_reduction_db).fold(0.,f64::max)})
                }).collect::<Vec<_>>();
                events.push(serde_json::json!({"channel":i,"frame":w.start_frame,"seconds":w.start_frame as f64/m.rate as f64,"kind":if phrase{"phrase_activity_start"}else{"hit_onset_proxy"},"confidence":c.confidence,"attack_body_decay":envelope,"note":"20 ms resolution; hit anchor is strongest band window within 60 ms after rise. Adjacent events may overlap decay. Activity is not transcription."}));
            }
        }
        let band_stats = (0..BANDS)
            .map(|b| {
                percentile(
                    &active
                        .iter()
                        .map(|w| power_db(m.energy(w, &[i], b, &g)))
                        .collect::<Vec<_>>(),
                    0.5,
                )
            })
            .collect::<Vec<_>>();
        summaries.push(serde_json::json!({"channel":i,"file":ch.file,"role":ch.role,"active_windows":active.len(),"active_rms_p10_dbfs":percentile(&levels,0.1),"active_rms_p50_dbfs":percentile(&levels,0.5),"active_rms_p90_dbfs":percentile(&levels,0.9),"mean_reduction_p50_db":percentile(&reductions,0.5),"mean_reduction_p90_db":percentile(&reductions,0.9),"active_band_medians_dbfs":band_stats,"onsets":m.windows.iter().filter(|w|w.channels[i].onset).count(),"ambiguous_bleed_events":m.windows.iter().filter(|w|w.channels[i].bleed_ambiguous).count()}));
    }
    file.flush()?;
    type GroupSelector = (&'static str, fn(Role) -> bool);
    let group_specs: [GroupSelector; 8] = [
        ("drum_close", percussion),
        ("overheads", |r| matches!(r, Role::Overheads)),
        ("drum_room", |r| matches!(r, Role::DrumRoom)),
        ("bass", |r| matches!(r, Role::BassDi)),
        ("guitars", guitar),
        ("lead_vocal", |r| matches!(r, Role::LeadVocal)),
        ("vocal_room", |r| matches!(r, Role::VocalRoom)),
        ("generated_fx", |_| false),
    ];
    let mut groups = BufWriter::new(File::create(out.join("groups.csv"))?);
    writeln!(
        groups,
        "start_frame,group,broadband_dbfs,40_120_dbfs,120_500_dbfs,500_1500_dbfs,1500_5000_dbfs"
    )?;
    for (name, select) in group_specs {
        let indices = if name == "generated_fx" {
            vec![s.channels.len()]
        } else {
            s.channels
                .iter()
                .enumerate()
                .filter(|(_, c)| select(c.role))
                .map(|(i, _)| i)
                .collect()
        };
        for w in &m.windows {
            let b = (0..BANDS)
                .map(|b| power_db(m.energy(w, &indices, b, &g)))
                .collect::<Vec<_>>();
            writeln!(
                groups,
                "{},{},{:.5},{:.5},{:.5},{:.5},{:.5}",
                w.start_frame, name, b[0], b[1], b[2], b[3], b[4]
            )?;
        }
    }
    groups.flush()?;
    let mut pairs = Vec::new();
    for i in 0..s.channels.len() {
        for j in i + 1..s.channels.len() {
            let related = (kit(s.channels[i].role) && kit(s.channels[j].role))
                || matches!(
                    (s.channels[i].role, s.channels[j].role),
                    (Role::LeadVocal, Role::VocalRoom) | (Role::VocalRoom, Role::LeadVocal)
                );
            if !related {
                continue;
            }
            let mut corr = Vec::new();
            let mut cancellation = Vec::new();
            let mut ratios = Vec::new();
            let mut coincident = 0;
            for w in &m.windows {
                if !w.channels[i].active || !w.channels[j].active {
                    continue;
                }
                let a = m.energy(w, &[i], 0, &g);
                let b = m.energy(w, &[j], 0, &g);
                if a < 1e-20 || b < 1e-20 {
                    continue;
                }
                corr.push(w.covariance[i * m.signals + j] / (a * b).sqrt());
                cancellation.push(power_db(m.energy(w, &[i, j], 0, &g)) - power_db(a + b));
                if w.channels[i].onset || w.channels[j].onset {
                    coincident += 1;
                    ratios.push(power_db(b) - power_db(a));
                }
            }
            pairs.push(serde_json::json!({"channels":[i,j],"simultaneous_windows":corr.len(),"routed_correlation_p50":percentile(&corr,0.5),"sum_vs_incoherent_power_p10_db":percentile(&cancellation,0.1),"sum_vs_incoherent_power_p50_db":percentile(&cancellation,0.5),"coincident_event_windows":coincident,"event_j_over_i_p50_db":percentile(&ratios,0.5),"interpretation":"Explicit related microphones, zero-lag routed covariance. Negative sum differences flag possible cancellation; neither correlation nor coincidence identifies bleed or proves polarity error. No alignment/polarity changes."}));
        }
    }
    write_json(&out.join("sources.json"), &summaries)?;
    write_json(&out.join("events.json"), &events)?;
    write_json(&out.join("microphone-relationships.json"), &pairs)?;
    write_json(
        &out.join("targets.json"),
        &serde_json::json!({"fit":evaluate(m,p,&vec![0.;s.channels.len()],false),"held_out":evaluate(m,p,&vec![0.;s.channels.len()],true),"bands_hz":["broadband","40-120","120-500","500-1500","1500-5000"],"filter":"Difference of two second-order Butterworth highpasses; overlapping responses, not rectangular FFT bands.","scope":"post-channel/pre-master routed energy, with excitation; generated FX measured separately. Whole-recording offline preparation. No auditory model or listener preference."}),
    )?;
    Ok(())
}
/// Analyze or prepare the fader-only candidate. Processing revision is a separate,
/// reviewable settings edit after inspecting A's evidence.
pub fn run(
    s: Session,
    root: &Path,
    out: &Path,
    policy: Option<Policy>,
    render_candidate: bool,
) -> Result<()> {
    validate_baseline(&s)?;
    let p = policy.unwrap_or_else(|| Policy::for_session(&s));
    p.validate(s.channels.len())?;
    std::fs::create_dir(out)?;
    write_json(&out.join("policy.json"), &p)?;
    let m = measure(&s, root, &p)?;
    save_measurement(&m, &s, &p, &out.join("before"))?;
    if !render_candidate {
        return Ok(());
    }
    let result = optimize(&m, &s, &p)?;
    write_json(&out.join("optimization.json"), &result)?;
    let mut a = s.clone();
    let mut decisions = Vec::new();
    for (i, c) in a.channels.iter_mut().enumerate() {
        let old = c.fader_db;
        c.fader_db += result.deltas_db[i];
        decisions.push(serde_json::json!({"channel":i,"file":c.file,"parameter":"fader_db","old":old,"new":c.fader_db,"evidence":"optimization.json: fit/held-out simultaneous role ratios and bounds","tradeoff":"Static whole-song compromise; source/FX balance and low-confidence events require listening."}));
    }
    a.validate()?;
    write_json(&out.join("A-settings.json"), &a)?;
    write_json(&out.join("decisions.json"), &decisions)?;
    super::run(a.clone(), root, &out.join("A"), None)?;
    let after = measure(&a, root, &p)?;
    save_measurement(&after, &a, &p, &out.join("after-A"))?;
    let actual_fit = evaluate(&after, &p, &vec![0.; a.channels.len()], false);
    let actual_hold = evaluate(&after, &p, &vec![0.; a.channels.len()], true);
    let targets_met = actual_fit
        .iter()
        .chain(&actual_hold)
        .all(|r| r.violation_db.is_some_and(|v| v < 1e-6));
    write_json(
        &out.join("status.json"),
        &serde_json::json!({"technical_checks_passed":true,"policy_targets_met":targets_met,"listener_preferred":null,"actual_fit":actual_fit,"actual_held_out":actual_hold,"accepted_fader_change":result.accepted,"no_playback":true}),
    )?;
    Ok(())
}
