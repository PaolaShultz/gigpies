//! Offline intent-conditioned guitar tone. Identity is supplied by setup, never inferred.
//! Spectral policy is a musical hypothesis; held-out checks are not listening approval.
use super::{
    analysis::fft,
    balance::validate_baseline,
    config::{EqBand, EqKind, Role, Session},
    dsp::{Biquad, Strip, db, gain},
    render::{Source, route},
    write_json,
};
use crate::inventory::Result;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
const SIZE: usize = 8192;

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Intent {
    #[default]
    Balanced,
    Thin,
    Dark,
    Full,
}
impl Intent {
    pub fn range(self) -> [f64; 2] {
        match self {
            Self::Balanced => [-2., 4.],
            Self::Thin => [-12., -4.],
            Self::Dark => [1., 7.],
            Self::Full => [2., 8.],
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Instrument {
    pub name: String,
    /// File and channel identity are both checked against the supplied session.
    pub primary: usize,
    pub primary_file: std::path::PathBuf,
    pub secondary: Vec<usize>,
    pub secondary_files: Vec<std::path::PathBuf>,
    #[serde(default)]
    pub intent: Intent,
    #[serde(default)]
    pub capture: Option<super::expert::Capture>,
    #[serde(default)]
    pub profile: Option<super::expert::Profile>,
}
impl Instrument {
    pub fn body_range(&self) -> [f64; 2] {
        self.profile
            .as_ref()
            .and_then(|p| p.body_presence_db)
            .unwrap_or_else(|| self.intent.range())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Policy {
    pub instruments: Vec<Instrument>,
    pub section_seconds: f64,
    pub minimum_active_seconds: f64,
    pub activity_floor_dbfs: f64,
    pub relative_activity_db: f64,
    pub minimum_consistency: f64,
    pub max_secondary_relative_db: f64,
    pub max_added_reduction_db: f64,
    pub max_crest_loss_db: f64,
    pub max_section_regression_db: f64,
}
impl Default for Policy {
    fn default() -> Self {
        Self {
            instruments: vec![],
            section_seconds: 12.,
            minimum_active_seconds: 3.,
            activity_floor_dbfs: -65.,
            relative_activity_db: 24.,
            minimum_consistency: 0.7,
            max_secondary_relative_db: -3.,
            max_added_reduction_db: 1.,
            max_crest_loss_db: 1.5,
            max_section_regression_db: 1.,
        }
    }
}
impl Policy {
    pub fn validate(&self, s: &Session) -> Result<()> {
        validate_baseline(s)?;
        let bounded = |v: f64, a: f64, b: f64| v.is_finite() && (a..=b).contains(&v);
        if self.instruments.is_empty()
            || self.instruments.len() > 8
            || !bounded(self.section_seconds, 2., 60.)
            || !bounded(self.minimum_active_seconds, 1., 60.)
            || !bounded(self.activity_floor_dbfs, -90., -30.)
            || !bounded(self.relative_activity_db, 6., 36.)
            || !bounded(self.minimum_consistency, 0.6, 1.)
            || !bounded(self.max_secondary_relative_db, -24., 0.)
            || !bounded(self.max_added_reduction_db, 0., 3.)
            || !bounded(self.max_crest_loss_db, 0., 3.)
            || !bounded(self.max_section_regression_db, 0., 2.)
        {
            return Err("invalid tone policy".into());
        }
        // The first pass deliberately isolates the dry channel chain. FX require a later guard.
        if s.effects.is_some() {
            return Err("tone preparation currently requires effects disabled".into());
        }
        let mut used = BTreeSet::new();
        for g in &self.instruments {
            if let Some(profile) = &g.profile {
                profile.validate(s, g)?;
            }
            if g.name.is_empty()
                || g.primary >= s.channels.len()
                || s.channels[g.primary].file != g.primary_file
                || !matches!(
                    s.channels[g.primary].role,
                    Role::RhythmGuitar | Role::LeadGuitar | Role::AcousticGuitar
                )
                || g.secondary.len() > 4
                || g.secondary.len() != g.secondary_files.len()
                || g.secondary
                    .iter()
                    .zip(&g.secondary_files)
                    .any(|(&i, file)| i >= s.channels.len() || &s.channels[i].file != file)
            {
                return Err("invalid tone instrument or insufficient EQ slots".into());
            }
            for i in std::iter::once(&g.primary).chain(g.secondary.iter()) {
                if *i >= s.channels.len() || !used.insert(*i) {
                    return Err("overlapping/invalid tone source identity".into());
                }
            }
        }
        Ok(())
    }
}
#[derive(Clone, Serialize)]
pub struct Frame {
    pub seconds: f64,
    pub raw_dbfs: f64,
    pub raw_secondary_relative_db: f64,
    pub input_full_scale_fraction: f64,
    pub input_is_float: bool,
    pub primary_dbfs: f64,
    pub secondary_relative_db: f64,
    pub crest_db: f64,
    pub mean_reduction_db: f64,
    pub max_reduction_db: f64,
    pub body_presence_db: f64,
    pub group_body_presence_db: f64,
    pub body_power_fraction: f64,
    pub presence_power_fraction: f64,
    #[serde(skip)]
    pub spectrum: Vec<f64>,
}
fn power_db(p: f64) -> f64 {
    db(p.max(0.).sqrt())
}
pub(super) fn percentile(values: impl Iterator<Item = f64>, p: f64) -> f64 {
    let mut v = values.collect::<Vec<_>>();
    if v.is_empty() {
        return 0.;
    }
    v.sort_by(f64::total_cmp);
    v[((v.len() - 1) as f64 * p).round() as usize]
}
fn band_power(spectrum: &[f64], rate: u32, lo: f64, hi: f64) -> f64 {
    spectrum
        .iter()
        .enumerate()
        .filter(|(k, _)| {
            let hz = *k as f64 * rate as f64 / SIZE as f64;
            hz >= lo && hz < hi
        })
        .map(|(_, v)| v)
        .sum()
}
fn ratio(spectrum: &[f64], rate: u32) -> f64 {
    power_db(band_power(spectrum, rate, 100., 400.))
        - power_db(band_power(spectrum, rate, 800., 3200.))
}

fn spectrum(block: &[[f64; 2]]) -> Vec<f64> {
    let mut p = vec![0.; SIZE / 2 + 1];
    let mut x = vec![[0.; 2]; SIZE];
    for channel in 0..2 {
        for (i, v) in block.iter().enumerate() {
            x[i] = [
                v[channel] * (0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / SIZE as f64).cos()),
                0.,
            ];
        }
        fft(&mut x);
        for (k, v) in p.iter_mut().enumerate() {
            *v += (x[k][0] * x[k][0] + x[k][1] * x[k][1]) / 2.;
        }
    }
    p
}
/// Native timing, linked production compression, coherent stereo instrument sum.
pub fn measure(s: &Session, root: &Path, g: &Instrument) -> Result<Vec<Frame>> {
    Policy {
        instruments: vec![g.clone()],
        ..Default::default()
    }
    .validate(s)?;
    let mut sources = s
        .channels
        .iter()
        .map(|c| Source::open(&root.join(&c.file), s.sample_rate))
        .collect::<Result<Vec<_>>>()?;
    let refs = sources
        .iter()
        .filter_map(|x| x.reference)
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
        .map(|src| {
            src.frames
                .checked_add(src.offset)
                .ok_or("timeline overflow")
        })
        .collect::<std::result::Result<Vec<_>, _>>()?
        .into_iter()
        .max()
        .unwrap_or(0);
    // Bounded cache, at most 64 MiB of spectra per instrument; ignore incomplete final FFT block.
    if total / SIZE as u64 > 2000 {
        return Err("tone evidence exceeds 64 MiB; use a shorter soundcheck".into());
    }
    let indices = std::iter::once(g.primary)
        .chain(g.secondary.iter().copied())
        .collect::<Vec<_>>();
    let mut strips = indices
        .iter()
        .map(|&i| Strip::new(&s.channels[i], s.sample_rate))
        .collect::<Vec<_>>();
    let mut frames = vec![];
    for start in (0..total.saturating_sub(SIZE as u64 - 1)).step_by(SIZE) {
        let full_scale_before = sources[g.primary].full_scale_samples();
        let mut main = vec![[0.; 2]; SIZE];
        let mut group = main.clone();
        let (mut raw, mut power, mut secondary, mut peak, mut gr, mut max_gr) =
            (0., 0., 0., 0_f64, 0., 0_f64);
        let mut raw_secondary = 0.;
        for j in 0..SIZE {
            let mut other = [0.; 2];
            for (slot, &i) in indices.iter().enumerate() {
                let x = sources[i].next(start + j as u64)?;
                let y = route(
                    strips[slot].tick(x),
                    sources[i].channels,
                    s.channels[i].pan,
                    gain(s.channels[i].fader_db),
                );
                if slot == 0 {
                    raw += (x[0] * x[0] + x[1] * x[1]) / 2.;
                    main[j] = y;
                    power += (y[0] * y[0] + y[1] * y[1]) / 2.;
                    peak = peak.max(y[0].abs().max(y[1].abs()));
                    gr += strips[slot].reduction_db();
                    max_gr = max_gr.max(strips[slot].reduction_db());
                } else {
                    raw_secondary += (x[0] * x[0] + x[1] * x[1]) / 2.;
                    for c in 0..2 {
                        other[c] += y[c];
                    }
                }
            }
            secondary += (other[0] * other[0] + other[1] * other[1]) / 2.;
            group[j] = [main[j][0] + other[0], main[j][1] + other[1]];
        }
        let ps = spectrum(&main);
        let gs = spectrum(&group);
        frames.push(Frame {
            seconds: start as f64 / s.sample_rate as f64,
            raw_dbfs: power_db(raw / SIZE as f64),
            raw_secondary_relative_db: power_db(raw_secondary / SIZE as f64)
                - power_db(raw / SIZE as f64),
            input_is_float: sources[g.primary].is_float(),
            input_full_scale_fraction: (sources[g.primary].full_scale_samples() - full_scale_before)
                as f64
                / (SIZE * sources[g.primary].channels) as f64,
            primary_dbfs: power_db(power / SIZE as f64),
            secondary_relative_db: power_db(secondary / SIZE as f64)
                - power_db(power / SIZE as f64),
            crest_db: db(peak) - power_db(power / SIZE as f64),
            mean_reduction_db: gr / SIZE as f64,
            max_reduction_db: max_gr,
            body_presence_db: ratio(&ps, s.sample_rate),
            group_body_presence_db: ratio(&gs, s.sample_rate),
            body_power_fraction: band_power(&ps, s.sample_rate, 100., 400.)
                / ps.iter().sum::<f64>().max(1e-30),
            presence_power_fraction: band_power(&ps, s.sample_rate, 800., 3200.)
                / ps.iter().sum::<f64>().max(1e-30),
            spectrum: ps,
        });
    }
    Ok(frames)
}
pub(super) fn within_section(f: &Frame, rate: u32, p: &Policy) -> bool {
    (f.seconds / p.section_seconds) as u64
        == ((f.seconds + (SIZE - 1) as f64 / rate as f64) / p.section_seconds) as u64
}
pub(super) fn held(f: &Frame, p: &Policy) -> bool {
    (f.seconds / p.section_seconds) as u64 % 2 == 1
}
pub(super) fn violation(x: f64, range: [f64; 2]) -> f64 {
    (range[0] - x).max(0.) + (x - range[1]).max(0.)
}
fn band(hz: f64, db: f64) -> EqBand {
    EqBand {
        kind: EqKind::Bell,
        hz,
        q: 0.7,
        db,
    }
}
#[derive(Clone, Serialize)]
pub struct Summary {
    pub windows: usize,
    pub median_body_presence_db: f64,
    pub median_group_body_presence_db: f64,
    pub p95_reduction_db: f64,
    pub median_crest_db: f64,
    pub p10_crest_db: f64,
}
pub(super) fn summary(frames: &[Frame], mask: &[bool], p: &Policy, hold: bool) -> Summary {
    let v = frames
        .iter()
        .zip(mask)
        .filter(|(f, m)| **m && held(f, p) == hold)
        .map(|(f, _)| f)
        .collect::<Vec<_>>();
    Summary {
        windows: v.len(),
        median_body_presence_db: percentile(v.iter().map(|f| f.body_presence_db), 0.5),
        median_group_body_presence_db: percentile(v.iter().map(|f| f.group_body_presence_db), 0.5),
        p95_reduction_db: percentile(v.iter().map(|f| f.max_reduction_db), 0.95),
        median_crest_db: percentile(v.iter().map(|f| f.crest_db), 0.5),
        p10_crest_db: percentile(v.iter().map(|f| f.crest_db), 0.1),
    }
}
#[derive(Serialize)]
pub struct Proposal {
    pub instrument: String,
    pub range_db: [f64; 2],
    pub reason: String,
    pub consistency: f64,
    pub evaluated: usize,
    pub training_sections_guarded: usize,
    pub proposed_eq: Vec<EqBand>,
    pub predicted_training_ratio_db: Option<f64>,
    pub before: [Summary; 2],
    #[serde(skip)]
    pub eligible: Vec<bool>,
}
/// Fixed 546-candidate budget, training only; no supplied mix/reference or manual curve.
pub fn propose(frames: &[Frame], rate: u32, g: &Instrument, p: &Policy) -> Proposal {
    let high = percentile(
        frames
            .iter()
            .filter(|f| !held(f, p) && within_section(f, rate, p))
            .map(|f| f.raw_dbfs),
        0.95,
    );
    let mask = frames
        .iter()
        .map(|f| {
            within_section(f, rate, p)
                && f.body_power_fraction >= 0.01
                && f.presence_power_fraction >= 0.01
                && f.raw_dbfs > p.activity_floor_dbfs
                && f.raw_dbfs > high - p.relative_activity_db
                && f.secondary_relative_db <= p.max_secondary_relative_db
                && f.body_presence_db > -30.
        })
        .collect::<Vec<_>>();
    let before = [
        summary(frames, &mask, p, false),
        summary(frames, &mask, p, true),
    ];
    let range = g.body_range();
    let mut out = Proposal {
        instrument: g.name.clone(),
        range_db: range,
        reason: "insufficient direct-source evidence".into(),
        consistency: 0.,
        evaluated: 0,
        training_sections_guarded: 0,
        proposed_eq: vec![],
        predicted_training_ratio_db: None,
        before,
        eligible: mask,
    };
    if g.profile
        .as_ref()
        .is_some_and(|p| p.body_presence_db.is_none())
    {
        out.reason = "body rule disabled by profile".into();
        return out;
    }
    let needed = ((p.minimum_active_seconds * rate as f64 / SIZE as f64).ceil() as usize).max(3);
    // Holdout values are never used in selection, only checked after the single proposal.
    if out.before[0].windows < needed {
        return out;
    }
    let fit = frames
        .iter()
        .zip(&out.eligible)
        .filter(|(f, m)| **m && !held(f, p))
        .map(|(f, _)| f)
        .collect::<Vec<_>>();
    let low = out.before[0].median_body_presence_db < range[0];
    out.consistency = fit
        .iter()
        .filter(|f| {
            if low {
                f.body_presence_db < range[0]
            } else {
                f.body_presence_db > range[1]
            }
        })
        .count() as f64
        / fit.len() as f64;
    if violation(out.before[0].median_body_presence_db, range) < 0.5 {
        out.reason = "within chosen intent".into();
        return out;
    }
    if out.consistency < p.minimum_consistency {
        out.reason = "inconsistent tone; abstained".into();
        return out;
    }
    let mut sections: BTreeMap<u64, Vec<&Frame>> = BTreeMap::new();
    for f in &fit {
        sections
            .entry((f.seconds / p.section_seconds) as u64)
            .or_default()
            .push(f);
    }
    sections.retain(|_, v| v.len() >= 3);
    out.training_sections_guarded = sections.len();
    let mut ps = vec![0.; SIZE / 2 + 1];
    for f in &fit {
        for (v, x) in ps.iter_mut().zip(&f.spectrum) {
            *v += x;
        }
    }
    let initial = ratio(&ps, rate);
    let mut cost = violation(initial, range).powi(2);
    for hz in [160., 220., 300.] {
        for body in -6..=6 {
            for presence_hz in [1600., 2400.] {
                for presence in -3..=3 {
                    out.evaluated += 1;
                    // Opposing equalizers add complexity without serving the diagnosed direction.
                    if (low && (body < 0 || presence > 0)) || (!low && (body > 0 || presence < 0)) {
                        continue;
                    }
                    let eq = [band(hz, body as f64), band(presence_hz, presence as f64)];
                    let filters = eq
                        .iter()
                        .map(|e| Biquad::equalizer(e, rate))
                        .collect::<Vec<_>>();
                    let response = (0..ps.len())
                        .map(|k| {
                            filters
                                .iter()
                                .map(|f| {
                                    f.power_response(k as f64 * rate as f64 / SIZE as f64, rate)
                                })
                                .product::<f64>()
                        })
                        .collect::<Vec<_>>();
                    let predict = |spectrum: &[f64]| {
                        let energy = |lo: f64, hi: f64| {
                            let a = (lo * SIZE as f64 / rate as f64).ceil() as usize;
                            let b = (hi * SIZE as f64 / rate as f64).ceil() as usize;
                            spectrum[a..b]
                                .iter()
                                .zip(&response[a..b])
                                .map(|(v, r)| v * r)
                                .sum::<f64>()
                        };
                        power_db(energy(100., 400.)) - power_db(energy(800., 3200.))
                    };
                    if sections.values().any(|frames| {
                        let old = percentile(frames.iter().map(|f| f.body_presence_db), 0.5);
                        let new = percentile(frames.iter().map(|f| predict(&f.spectrum)), 0.5);
                        violation(new, range) > violation(old, range) + p.max_section_regression_db
                    }) {
                        continue;
                    }
                    let r = predict(&ps);
                    let c = violation(r, range).powi(2)
                        + 0.025 * (body * body + presence * presence) as f64;
                    if c < cost - 1e-9 {
                        cost = c;
                        out.proposed_eq = eq.into_iter().filter(|e| e.db != 0.).collect();
                        out.predicted_training_ratio_db = Some(r);
                    }
                }
            }
        }
    }
    out.reason = if out.proposed_eq.is_empty() {
        "no bounded improvement"
    } else if low {
        "body deficit relative to chosen intent"
    } else {
        "body excess relative to chosen intent"
    }
    .into();
    out
}
#[derive(Serialize)]
pub struct Validation {
    pub accepted: bool,
    pub reason: String,
    pub after: [Summary; 2],
}
pub fn validate_candidate(
    before: &[Frame],
    after: &[Frame],
    q: &Proposal,
    rate: u32,
    p: &Policy,
) -> Validation {
    if q.proposed_eq.is_empty() {
        return Validation {
            accepted: false,
            reason: "no proposal".into(),
            after: [
                summary(after, &q.eligible, p, false),
                summary(after, &q.eligible, p, true),
            ],
        };
    }
    validate_changes(before, after, q, rate, p, true)
}
pub(super) fn validate_changes(
    before: &[Frame],
    after: &[Frame],
    q: &Proposal,
    rate: u32,
    p: &Policy,
    require_tone_improvement: bool,
) -> Validation {
    let summaries = [
        summary(after, &q.eligible, p, false),
        summary(after, &q.eligible, p, true),
    ];
    let mut v = Validation {
        accepted: false,
        reason: "no proposal".into(),
        after: summaries,
    };
    if before.len() != after.len()
        || before
            .iter()
            .zip(after)
            .any(|(a, b)| a.seconds != b.seconds)
    {
        v.reason = "timeline mismatch".into();
        return v;
    }
    for (old, new) in q.before.iter().zip(&v.after) {
        if new.windows < 3
            || (new.windows as f64 * SIZE as f64 / rate as f64) < p.minimum_active_seconds
        {
            v.reason = "insufficient held-out evidence".into();
            return v;
        }
        if require_tone_improvement
            && violation(new.median_body_presence_db, q.range_db)
                > violation(old.median_body_presence_db, q.range_db) - 0.25
        {
            v.reason = "no actual improvement in both splits".into();
            return v;
        }
        if !require_tone_improvement
            && violation(new.median_body_presence_db, q.range_db)
                > violation(old.median_body_presence_db, q.range_db) + p.max_section_regression_db
        {
            v.reason = "primary tone regressed during another repair".into();
            return v;
        }
        if violation(new.median_group_body_presence_db, q.range_db)
            > violation(old.median_group_body_presence_db, q.range_db) + p.max_section_regression_db
        {
            v.reason = "coherent instrument sum regressed".into();
            return v;
        }
        if new.p95_reduction_db > old.p95_reduction_db + p.max_added_reduction_db
            || new.p10_crest_db < old.p10_crest_db - p.max_crest_loss_db
            || new.median_crest_db < old.median_crest_db - p.max_crest_loss_db
        {
            v.reason = "compression or transient guard rejected correction".into();
            return v;
        }
    }
    let sections = before
        .last()
        .map(|f| (f.seconds / p.section_seconds) as usize + 1)
        .unwrap_or(0);
    for section in 0..sections {
        let ids = before
            .iter()
            .enumerate()
            .filter(|(i, f)| q.eligible[*i] && (f.seconds / p.section_seconds) as usize == section)
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        if ids.len() < 3 {
            continue;
        }
        for group in [false, true] {
            let measure = |f: &Frame| {
                if group {
                    f.group_body_presence_db
                } else {
                    f.body_presence_db
                }
            };
            let a = percentile(ids.iter().map(|&i| measure(&before[i])), 0.5);
            let b = percentile(ids.iter().map(|&i| measure(&after[i])), 0.5);
            if violation(b, q.range_db) > violation(a, q.range_db) + p.max_section_regression_db {
                v.reason = "individual section regressed".into();
                return v;
            }
        }
    }
    v.accepted = true;
    v.reason = "actual DSP improved fit and held-out tone within guards".into();
    v
}
/// Write a single frozen proposal, validate actual DSP, then render only accepted changes.
pub fn run(s: Session, root: &Path, out: &Path, p: Policy, render: bool) -> Result<()> {
    p.validate(&s)?;
    if p.instruments
        .iter()
        .any(|g| s.channels[g.primary].eq.len() > 6)
    {
        return Err("tone preparation needs two free EQ slots".into());
    }
    std::fs::create_dir(out)?;
    write_json(&out.join("policy.json"), &p)?;
    write_json(&out.join("before-settings.json"), &s)?;
    let mut accepted = s.clone();
    let mut reports = vec![];
    for (n, g) in p.instruments.iter().enumerate() {
        let before = measure(&s, root, g)?;
        let proposal = propose(&before, s.sample_rate, g, &p);
        let mut candidate = s.clone();
        candidate.channels[g.primary]
            .eq
            .extend(proposal.proposed_eq.clone());
        candidate.validate()?;
        let after = if proposal.proposed_eq.is_empty() {
            before.clone()
        } else {
            measure(&candidate, root, g)?
        };
        let validation = validate_candidate(&before, &after, &proposal, s.sample_rate, &p);
        if validation.accepted {
            accepted.channels[g.primary] = candidate.channels[g.primary].clone();
        }
        write_json(&out.join(format!("instrument-{n}-before.json")), &before)?;
        write_json(&out.join(format!("instrument-{n}-candidate.json")), &after)?;
        write_json(
            &out.join(format!("instrument-{n}-eligible.json")),
            &proposal.eligible,
        )?;
        reports.push(serde_json::json!({"identity":g,"proposal":proposal,"validation":validation,"original_eq":s.channels[g.primary].eq,"candidate_eq":candidate.channels[g.primary].eq,"applied_eq":accepted.channels[g.primary].eq,"tradeoff":"More body can mask bass or vocals. Faders and other source paths are unchanged. No room/microphone-position inference or console-emulation claim."}));
    }
    accepted.validate()?;
    write_json(&out.join("decisions.json"), &reports)?;
    write_json(&out.join("settings.json"), &accepted)?;
    if render {
        super::run(accepted, root, &out.join("final"), None)?;
    }
    Ok(())
}
