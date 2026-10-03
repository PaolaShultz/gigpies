//! Explicit artistic FX preparation. Source tone and artistic faders are frozen.
//! Profiles propose a space; measured fit is not a verdict on musical quality.
use super::{
    balance::validate_baseline,
    config::Session,
    dsp::{Biquad, Strip, db, gain},
    effects::{
        Bus, ChorusConfig, DelayConfig, Effect, ExciterConfig, FxConfig, Rack, ReverbConfig,
        ReverbKind, Send,
    },
    render::{Source, route, write_json},
};
use crate::inventory::Result;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    f64::consts::FRAC_1_SQRT_2,
    path::{Path, PathBuf},
};

mod audit;
mod calibration;
mod persistence;
mod review;
mod validation;
pub use audit::{SavedAudit, audit_saved};
pub use calibration::{
    BusObservation, DecayCalibration, DecayRange, ReturnCalibration, ReturnLimit,
};
pub use persistence::{InputIdentity, Ready, verify};
pub use review::Review;
pub use validation::{Check, checks};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Style {
    Metal,
    Rock,
    Punk,
    Ska,
    Pop,
    Acoustic,
    Folk,
    Jazz,
    Electronic,
    Ambient,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Family {
    Kick,
    Bass,
    Snare,
    Toms,
    Percussion,
    RhythmGuitar,
    LeadGuitar,
    Acoustic,
    LeadVocal,
    BackingVocal,
    Keys,
    Winds,
    Strings,
    RoomCapture,
    Unknown,
    SuppliedFx,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    pub channel: usize,
    pub file: PathBuf,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Group {
    pub name: String,
    pub family: Family,
    pub inputs: Vec<Input>,
    /// Provenance supplied by setup/operator, never inferred from a filename.
    pub identity_basis: String,
    pub existing_space_reported: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub style: Style,
    pub style_basis: String,
    pub tempo_bpm: Option<f64>,
    /// Amplitude of the generated wet signal. Zero preserves the entire baseline.
    pub amount: f64,
    pub groups: Vec<Group>,
    pub training: Vec<[f64; 2]>,
    pub held_out: Vec<[f64; 2]>,
}
impl Policy {
    pub fn validate(&self, s: &Session) -> Result<()> {
        self.validate_structure(s)?;
        if !self.chronological() {
            return Err("all ambience training must precede held-out passages: continuous DSP history would otherwise let held-out audio change calibration".into());
        }
        Ok(())
    }

    // Reading historical evidence must not pretend it used newer admission rules.
    fn validate_structure(&self, s: &Session) -> Result<()> {
        validate_baseline(s)?;
        if s.effects.is_some()
            || s.master_db != 0.
            || self.style_basis.trim().is_empty()
            || !self.amount.is_finite()
            || !(0. ..=1.).contains(&self.amount)
            || self
                .tempo_bpm
                .is_some_and(|v| !v.is_finite() || !(40. ..=300.).contains(&v))
            || self.groups.is_empty()
            || self.groups.len() > 64
        {
            return Err("ambience requires a frozen unity-trim baseline without FX, explicit style and bounded amount/tempo".into());
        }
        let mut seen = BTreeSet::new();
        let mut names = BTreeSet::new();
        for g in &self.groups {
            if g.name.is_empty()
                || !g
                    .name
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"_-".contains(&c))
                || !names.insert(&g.name)
                || g.identity_basis.trim().is_empty()
                || g.inputs.is_empty()
                || g.family == Family::SuppliedFx
            {
                return Err("name and identify input groups; supplied FX returns must be excluded from the source session".into());
            }
            for i in &g.inputs {
                if !seen.insert(i.channel)
                    || !s.channels.get(i.channel).is_some_and(|c| c.file == i.file)
                {
                    return Err(
                        "ambience input identity differs, overlaps or is out of bounds".into(),
                    );
                }
            }
        }
        if seen.len() != s.channels.len() {
            return Err("every source needs an explicit FX routing disposition".into());
        }
        let mut spans = Vec::new();
        for set in [&self.training, &self.held_out] {
            if set.is_empty() || set.len() > 8 {
                return Err("supply one to eight training and held-out spans".into());
            }
            for &[a, b] in set {
                if !a.is_finite()
                    || !b.is_finite()
                    || a < 0.
                    || b - a < 1.
                    || b - a > 60.
                    || b > 86400.
                {
                    return Err("invalid ambience evaluation span".into());
                }
                if spans.iter().any(|&(x, y)| a < y && b > x) {
                    return Err("training and held-out spans must not overlap".into());
                }
                spans.push((a, b));
            }
        }
        Ok(())
    }

    fn chronological(&self) -> bool {
        super::training_precedes_held_out(&self.training, &self.held_out)
    }
}

fn empty_fx(rate: u32) -> FxConfig {
    FxConfig {
        buses: vec![],
        exciter: ExciterConfig {
            tune_hz: 2500_f64.min(rate as f64 * 0.2) as f32,
            drive: 0.,
            tone: 0.,
            bright: false,
        },
        exciter_amount: 0.,
        master_eq: vec![],
        maximizer_drive_db: 0.,
        maximizer_threshold_db: 24.,
        maximizer_release_ms: 120.,
        tail_seconds: 6.,
        listening_target_lufs: -20.5,
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct Frame {
    pub start: f64,
    pub end: f64,
    pub group_power: Vec<f64>,
    pub return_power: Vec<f64>,
    pub dry_power: f64,
    pub wet_power: f64,
    pub mixed_power: f64,
    pub dry_peak: f64,
    pub mixed_peak: f64,
    pub master_reduction_db: f64,
}
fn inside(f: &Frame, spans: &[[f64; 2]]) -> bool {
    spans
        .iter()
        .any(|s| f.start >= s[0] - 1e-9 && f.end <= s[1] + 1e-9)
}

/// Continuous production strip/routing/FX state from sample zero. Only requested
/// spans are retained; no audio is written and held-out frames never tune settings.
pub fn measure(
    s: &Session,
    root: &Path,
    p: &Policy,
    fx: Option<&FxConfig>,
    spans: &[[f64; 2]],
) -> Result<Vec<Frame>> {
    p.validate(s)?;
    let mut proposed = s.clone();
    proposed.effects = fx.cloned();
    proposed.validate()?;
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
    let end = (spans.iter().map(|x| x[1]).fold(0., f64::max) * s.sample_rate as f64).round() as u64;
    if end == 0 || end > total {
        return Err("evaluation extends beyond source timeline".into());
    }
    let mut strips = s
        .channels
        .iter()
        .map(|c| Strip::new(c, s.sample_rate))
        .collect::<Vec<_>>();
    let mut rack = fx.map(|f| Rack::new(f, s.channels.len(), s.sample_rate));
    let mut which = vec![0; s.channels.len()];
    for (g, group) in p.groups.iter().enumerate() {
        for input in &group.inputs {
            which[input.channel] = g;
        }
    }
    let mut hp = [[Biquad::highpass(s.master_hpf_hz, FRAC_1_SQRT_2, s.sample_rate); 2]; 2];
    let nb = fx.map_or(0, |f| f.buses.len());
    let mut bus_hp = vec![[Biquad::highpass(s.master_hpf_hz, FRAC_1_SQRT_2, s.sample_rate); 2]; nb];
    let mut group_hp =
        vec![[Biquad::highpass(s.master_hpf_hz, FRAC_1_SQRT_2, s.sample_rate); 2]; p.groups.len()];
    let size = (s.sample_rate / 50) as u64;
    let mut frames = Vec::new();
    let mut groups = vec![[0.; 2]; p.groups.len()];
    let blank = |start, end| Frame {
        start,
        end,
        group_power: vec![0.; p.groups.len()],
        return_power: vec![0.; nb],
        dry_power: 0.,
        wet_power: 0.,
        mixed_power: 0.,
        dry_peak: 0.,
        mixed_peak: 0.,
        master_reduction_db: 0.,
    };
    let mut f = blank(0., 0.);
    for t in 0..end {
        groups.fill([0.; 2]);
        let mut dry = [0.; 2];
        for (i, src) in sources.iter_mut().enumerate() {
            let y = strips[i].tick(src.next(t)?);
            let y = if let Some(r) = &mut rack {
                r.excite(i, y)
            } else {
                y
            };
            let y = route(
                y,
                src.channels,
                s.channels[i].pan,
                gain(s.channels[i].fader_db),
            );
            for c in 0..2 {
                dry[c] += y[c];
                groups[which[i]][c] += y[c];
            }
            if let Some(r) = &mut rack {
                r.send(i, y);
            }
        }
        let wet = rack.as_mut().map_or([0.; 2], Rack::returns);
        let direct = std::array::from_fn::<_, 2, _>(|c| hp[0][c].tick(dry[c]));
        let filtered_wet = std::array::from_fn::<_, 2, _>(|c| hp[1][c].tick(wet[c]));
        let mut mixed = std::array::from_fn(|c| direct[c] + filtered_wet[c]);
        if let Some(r) = &mut rack {
            mixed = r.master(mixed);
            f.master_reduction_db = f.master_reduction_db.max(r.reduction_db());
        }
        for c in 0..2 {
            let d = direct[c];
            let w = filtered_wet[c];
            let mix = mixed[c];
            if !mix.is_finite() {
                return Err("nonfinite FX output".into());
            }
            f.dry_power += d * d;
            f.wet_power += w * w;
            f.mixed_power += mix * mix;
            f.dry_peak = f.dry_peak.max(d.abs());
            f.mixed_peak = f.mixed_peak.max(mix.abs());
        }
        for (g, x) in groups.iter().enumerate() {
            for (c, &v) in x.iter().enumerate() {
                f.group_power[g] += group_hp[g][c].tick(v).powi(2);
            }
        }
        if let Some(r) = &rack {
            for (i, x) in r.return_outputs.iter().enumerate() {
                for (c, &v) in x.iter().enumerate() {
                    f.return_power[i] += bus_hp[i][c].tick(v).powi(2);
                }
            }
        }
        if (t + 1) % size == 0 {
            f.end = (t + 1) as f64 / s.sample_rate as f64;
            let count = ((f.end - f.start) * s.sample_rate as f64).round() * 2.;
            f.dry_power /= count;
            f.wet_power /= count;
            f.mixed_power /= count;
            for v in f.group_power.iter_mut().chain(f.return_power.iter_mut()) {
                *v /= count;
            }
            if inside(&f, spans) {
                frames.push(f);
            } else {
                drop(f);
            }
            if frames.len() > 24000 {
                return Err("FX evidence exceeds bounded window count".into());
            }
            f = blank((t + 1) as f64 / s.sample_rate as f64, 0.);
        }
    }
    Ok(frames)
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Activity {
    pub p95_dbfs: f64,
    pub active_fraction: f64,
    pub onsets_per_second: f64,
    pub active_seconds: f64,
}
fn activity(frames: &[Frame], g: usize) -> Activity {
    let mut levels = frames
        .iter()
        .map(|f| db(f.group_power[g].sqrt()))
        .collect::<Vec<_>>();
    levels.sort_by(f64::total_cmp);
    let p95 = levels
        .get((levels.len().saturating_sub(1) as f64 * 0.95).round() as usize)
        .copied()
        .unwrap_or(-240.);
    let gate = (p95 - 24.).max(-65.);
    let mut active = 0.;
    let mut duration = 0.;
    let mut onsets = 0.;
    let mut last = -1.;
    let mut prev = -240.;
    let mut previous_end = -1.;
    for f in frames {
        let level = db(f.group_power[g].sqrt());
        let dt = f.end - f.start;
        duration += dt;
        if (f.start - previous_end).abs() > 0.001 {
            prev = level;
        }
        if level >= gate {
            active += dt;
            if level - prev >= 5. && f.start - last >= 0.1 {
                onsets += 1.;
                last = f.start;
            }
        }
        prev = level;
        previous_end = f.end;
    }
    Activity {
        p95_dbfs: p95,
        active_fraction: active / duration.max(1e-9),
        onsets_per_second: onsets / duration.max(1e-9),
        active_seconds: active,
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Decision {
    pub group: String,
    pub family: Family,
    pub activity: Activity,
    pub reason: String,
    pub expected_benefit: String,
    pub musical_risk: String,
    pub bus_names: Vec<String>,
    pub desired_decay_seconds: Option<f64>,
    pub measured_decay_seconds: Option<f64>,
    #[serde(default)]
    pub decay_calibration: Option<DecayCalibration>,
}
fn reverb_recipe(style: Style, family: Family) -> Option<(ReverbKind, f64, f64, f64, f64)> {
    use Family::*;
    let tight = matches!(style, Style::Metal | Style::Punk);
    let natural = matches!(style, Style::Acoustic | Style::Folk | Style::Jazz);
    let spacious = matches!(style, Style::Ambient);
    // (type, desired energy-decay seconds, predelay ms, wet/source dB, wet LPF Hz)
    Some(match family {
        Kick | Bass | RoomCapture | Unknown | SuppliedFx => return None,
        Snare => (
            if natural {
                ReverbKind::Chamber
            } else {
                ReverbKind::Plate
            },
            if tight { 0.5 } else { 0.8 },
            12.,
            -20.,
            6500.,
        ),
        Toms | Percussion => (
            ReverbKind::Chamber,
            if tight { 0.45 } else { 0.7 },
            18.,
            -22.,
            5500.,
        ),
        LeadVocal => (
            if natural || spacious {
                ReverbKind::Hall
            } else {
                ReverbKind::Plate
            },
            if spacious {
                1.8
            } else if tight {
                0.8
            } else {
                1.2
            },
            45.,
            if tight { -19. } else { -17. },
            6500.,
        ),
        BackingVocal => (
            ReverbKind::Chamber,
            if tight { 0.7 } else { 1.1 },
            35.,
            -20.,
            5500.,
        ),
        RhythmGuitar => (
            if style == Style::Ska {
                ReverbKind::Plate
            } else {
                ReverbKind::SmallRoom
            },
            if style == Style::Ska { 0.65 } else { 0.35 },
            18.,
            if tight { -28. } else { -24. },
            5000.,
        ),
        LeadGuitar => (
            ReverbKind::Plate,
            if tight { 0.7 } else { 1.0 },
            28.,
            -20.,
            5500.,
        ),
        Acoustic => (
            ReverbKind::Chamber,
            if spacious { 1.5 } else { 0.75 },
            22.,
            -22.,
            6500.,
        ),
        Keys => (
            if spacious {
                ReverbKind::Hall
            } else {
                ReverbKind::Chamber
            },
            if spacious { 1.8 } else { 0.85 },
            25.,
            -24.,
            6000.,
        ),
        Winds | Strings => (
            if natural {
                ReverbKind::Chamber
            } else {
                ReverbKind::Hall
            },
            if spacious { 1.8 } else { 0.85 },
            22.,
            -20.,
            6500.,
        ),
    })
}
/// Calibrate this engine's unitless decay against its filtered impulse response.
/// Returns measured -60 dB remaining-energy time after the requested predelay.
fn measured_decay(
    kind: ReverbKind,
    decay: f32,
    predelay: f32,
    rate: u32,
    highpass: f64,
    lowpass: f64,
) -> f64 {
    let mut fx = empty_fx(rate);
    fx.buses.push(Bus {
        name: "impulse".into(),
        effect: Effect::Reverb(ReverbConfig {
            kind,
            predelay_ms: predelay,
            decay,
            damping: 0.55,
        }),
        sends: vec![Send { channel: 0, db: 0. }],
        hpf_hz: highpass,
        lowpass_hz: lowpass,
        return_db: 0.,
        target_wet_db: -20.,
    });
    let mut rack = Rack::new(&fx, 1, rate);
    let mut energy = Vec::with_capacity((rate * 6) as usize);
    for i in 0..rate * 6 {
        if i == 0 {
            rack.send(0, [1.; 2]);
        }
        let y = rack.returns();
        energy.push(y[0] * y[0] + y[1] * y[1]);
    }
    let total = energy.iter().sum::<f64>();
    let mut remaining = 0.;
    let mut crossing = energy.len() - 1;
    for (i, &e) in energy.iter().enumerate().rev() {
        remaining += e;
        if remaining >= total * 1e-6 {
            crossing = i;
            break;
        }
    }
    (crossing as f64 / rate as f64 - predelay as f64 / 1000.).max(0.)
}
fn fit_decay(
    kind: ReverbKind,
    wanted: f64,
    predelay: f32,
    rate: u32,
    highpass: f64,
    lowpass: f64,
) -> DecayCalibration {
    let mut lo = 0.;
    let mut hi = 0.9;
    let minimum = measured_decay(kind, 0., predelay, rate, highpass, lowpass);
    let mut best = (0., minimum);
    for _ in 0..6 {
        let mid = (lo + hi) * 0.5;
        let actual = measured_decay(kind, mid, predelay, rate, highpass, lowpass);
        if (actual - wanted).abs() < (best.1 - wanted).abs() {
            best = (mid, actual);
        }
        if actual < wanted {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    // Measure the upper endpoint for reporting only. Keep the established six-step
    // selection, including its original tie handling, unchanged.
    let maximum = measured_decay(kind, 0.9, predelay, rate, highpass, lowpass);
    DecayCalibration {
        desired_seconds: wanted,
        measured_seconds: best.1,
        residual_seconds: best.1 - wanted,
        selected_control: best.0,
        control_range: [0., 0.9],
        minimum_control_seconds: minimum,
        maximum_control_seconds: maximum,
        target_range: if wanted < minimum {
            DecayRange::BelowMeasuredMinimum
        } else if wanted > maximum {
            DecayRange::AboveMeasuredMaximum
        } else {
            DecayRange::WithinMeasuredRange
        },
        search_steps: 6,
        impulse_capture_seconds: 6.,
    }
}

pub fn propose(
    s: &Session,
    p: &Policy,
    observations: &[Frame],
) -> Result<(FxConfig, Vec<usize>, Vec<Decision>)> {
    p.validate(s)?;
    let mut fx = empty_fx(s.sample_rate);
    let mut owners = Vec::new();
    let mut decisions = Vec::new();
    for (g, group) in p.groups.iter().enumerate() {
        let a = activity(observations, g);
        let mut d=Decision{group:group.name.clone(),family:group.family,activity:a.clone(),reason:String::new(),expected_benefit:"Requested spatial finish while retaining the accepted direct sound".into(),musical_risk:"Added tails can mask articulation, change coherent stereo and consume export headroom; listening remains required".into(),bus_names:vec![],desired_decay_seconds:None,measured_decay_seconds:None,decay_calibration:None};
        if p.amount == 0.
            || a.active_seconds < 0.5
            || reverb_recipe(p.style, group.family).is_none()
        {
            d.reason=if p.amount==0.{"Explicit zero FX amount; preserve baseline"}else if a.active_seconds<0.5{"Insufficient active source in training; no invented FX level"}else{"Keep low-end/recorded ambience or unclassified source intact; no additional FX prescription"}.into();
            decisions.push(d);
            continue;
        }
        let (kind, mut duration, predelay, mut target, lp) =
            reverb_recipe(p.style, group.family).unwrap();
        let busy = a.active_fraction > 0.75 || a.onsets_per_second > 3.;
        if busy {
            duration *= 0.75;
            target -= 2.;
        }
        if group.existing_space_reported {
            duration *= 0.8;
            target -= 4.;
        }
        let lp = lp.min(s.sample_rate as f64 * 0.4);
        let hp = if matches!(group.family, Family::LeadVocal | Family::BackingVocal) {
            220.
        } else {
            180.
        };
        let fitted = fit_decay(kind, duration, predelay as f32, s.sample_rate, hp, lp);
        let decay = fitted.selected_control;
        let name = format!("{}_space", group.name);
        d.bus_names.push(name.clone());
        d.desired_decay_seconds = Some(duration);
        d.measured_decay_seconds = Some(fitted.measured_seconds);
        d.decay_calibration = Some(fitted);
        d.reason = format!(
            "Explicit {:?}/{:?} spatial profile; busy training phrase={busy}; reported existing space={}; engine decay calibrated by impulse, no source-quality diagnosis",
            p.style, group.family, group.existing_space_reported
        );
        let sends = group
            .inputs
            .iter()
            .map(|x| Send {
                channel: x.channel,
                db: 0.,
            })
            .collect::<Vec<_>>();
        fx.buses.push(Bus {
            name,
            effect: Effect::Reverb(ReverbConfig {
                kind,
                predelay_ms: predelay as f32,
                decay,
                damping: 0.55,
            }),
            sends: sends.clone(),
            hpf_hz: hp,
            lowpass_hz: lp,
            return_db: 0.,
            target_wet_db: target.max(-40.),
        });
        owners.push(g);
        if let Some(bpm) = p.tempo_bpm
            && matches!(group.family, Family::LeadVocal | Family::LeadGuitar)
            && matches!(
                p.style,
                Style::Rock
                    | Style::Metal
                    | Style::Pop
                    | Style::Ska
                    | Style::Electronic
                    | Style::Ambient
            )
        {
            let quarter = 60000. / bpm;
            let name = format!("{}_echo", group.name);
            d.bus_names.push(name.clone());
            fx.buses.push(Bus {
                name,
                effect: Effect::Delay(DelayConfig {
                    left_ms: (quarter * 0.5).clamp(20., 1000.) as f32,
                    right_ms: (quarter * 0.75).clamp(20., 1000.) as f32,
                    feedback: if busy { 0.12 } else { 0.2 },
                    damping: 0.6,
                }),
                sends: sends.clone(),
                hpf_hz: 250.,
                lowpass_hz: 4500_f64.min(s.sample_rate as f64 * 0.4),
                return_db: 0.,
                target_wet_db: if busy { -27. } else { -24. },
            });
            owners.push(g);
        }
        if group.family == Family::Keys && matches!(p.style, Style::Pop | Style::Electronic) {
            let name = format!("{}_chorus", group.name);
            d.bus_names.push(name.clone());
            fx.buses.push(Bus {
                name,
                effect: Effect::Chorus(ChorusConfig {
                    rate_hz: 0.3,
                    depth_ms: 1.,
                    base_ms: 17.,
                    ensemble: false,
                }),
                sends,
                hpf_hz: 200.,
                lowpass_hz: 6500_f64.min(s.sample_rate as f64 * 0.4),
                return_db: 0.,
                target_wet_db: -28.,
            });
            owners.push(g);
        }
        decisions.push(d);
    }
    if fx.buses.len() > 12 {
        return Err("FX plan exceeds 12 buses; explicitly group compatible sends rather than silently drop sources".into());
    }
    fx.validate(s)?;
    Ok((fx, owners, decisions))
}

pub fn run(s: Session, root: &Path, out: &Path, p: Policy) -> Result<Review> {
    p.validate(&s)?;
    if out.exists() {
        return Err("output exists".into());
    }
    let inputs = InputIdentity::capture(&s, &p, root)?;
    std::fs::create_dir(out)?;
    let result = prepare(&s, root, out, &p, &inputs);
    if let Err(error) = &result {
        // Preserve partial evidence and the original error even when an I/O
        // failure also prevents writing this recovery note.
        let _ = write_json(
            &out.join("failure.json"),
            &serde_json::json!({
                "ready": false, "reason": error.to_string(), "baseline_retained": true,
                "recovery": "Keep these inputs and reports. Retry with a new output directory; no partial preparation is ready without ready.json."
            }),
        );
    }
    result
}

fn prepare(
    s: &Session,
    root: &Path,
    out: &Path,
    p: &Policy,
    inputs: &InputIdentity,
) -> Result<Review> {
    write_json(&out.join("before-settings.json"), &s)?;
    write_json(&out.join("policy.json"), &p)?;
    write_json(&out.join("input-identity.json"), inputs)?;
    let observed = measure(s, root, p, None, &p.training)?;
    let (mut fx, owners, decisions) = propose(s, p, &observed)?;
    write_json(&out.join("decisions.json"), &decisions)?;
    write_json(&out.join("seed-fx.json"), &fx)?;
    let seed = measure(s, root, p, Some(&fx), &p.training)?;
    let calibration = calibration::calibrate(&mut fx, &owners, &decisions, p, &seed)?;
    // One training-only adjustment of return levels; no held-out feedback.
    let mut candidate = s.clone();
    if !fx.buses.is_empty() {
        candidate.effects = Some(fx);
    }
    candidate.validate()?;
    write_json(&out.join("calibration.json"), &calibration)?;
    write_json(&out.join("candidate-settings.json"), &candidate)?;
    let training = measure(s, root, p, candidate.effects.as_ref(), &p.training)?;
    let train_checks = checks(&training, &p.training);
    let mut review = Review::new(decisions, calibration, train_checks);
    review.bus_observations.extend(calibration::observe(
        &training,
        &owners,
        &review.calibration,
        &p.training,
        "training_pooled",
    )?);
    for span in &p.training {
        review.bus_observations.extend(calibration::observe(
            &training,
            &owners,
            &review.calibration,
            &[*span],
            "training",
        )?);
    }
    write_json(&out.join("training-checks.json"), &review.training_checks)?;
    if review.training_checks.iter().any(|x| !x.passed) {
        review.reason = "Training protection failure; baseline retained".into();
        review.write(out)?;
        return Err(
            "FX training protection failed; baseline retained and no ready mix published".into(),
        );
    }
    write_json(
        &out.join("frozen-before-held-out.json"),
        &serde_json::json!({"settings":"candidate-settings.json","settings_id":super::identity::identity(&candidate)?,"input_identity_id":super::identity::identity(inputs)?,"training_passed":true,"held_out_retries":0,"musical_acceptance":null}),
    )?;
    let held = measure(s, root, p, candidate.effects.as_ref(), &p.held_out)?;
    let held_checks = checks(&held, &p.held_out);
    write_json(&out.join("held-out-checks.json"), &held_checks)?;
    review.held_out_checks = held_checks;
    for span in &p.held_out {
        review.bus_observations.extend(calibration::observe(
            &held,
            &owners,
            &review.calibration,
            &[*span],
            "held_out",
        )?);
    }
    if review.held_out_checks.iter().any(|x| !x.passed) {
        review.reason = "Held-out protection failure; baseline retained, no retry".into();
        review.write(out)?;
        return Err(
            "FX held-out protection failed; baseline retained and no ready mix published".into(),
        );
    }
    inputs.verify_sources(s, root)?;
    review.technically_eligible = true;
    review.selected = if candidate.effects.is_some() {
        "expert_fx_preview"
    } else {
        "baseline"
    }
    .into();
    review.reason = if candidate.effects.is_some() {
        "Ensemble protection checks passed. Individual artistic target results remain separate."
    } else {
        "No added FX; the complete baseline is preserved."
    }
    .into();
    review.write(out)?;
    persistence::finish(out, &candidate, inputs)?;
    Ok(review)
}
