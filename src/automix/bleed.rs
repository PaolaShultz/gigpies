//! Conditional snare-spill analysis. Correlation supports a hypothesis; it does
//! not separate sources or identify microphone position. Offline only.
use super::{
    config::{EqKind, Role, Session},
    drums::{self, Event},
    dsp::{Biquad, Strip, db, gain},
    render::{Source, write_json},
};
use crate::inventory::Result;
use serde::{Deserialize, Serialize};
use std::{
    f64::consts::FRAC_1_SQRT_2,
    path::{Path, PathBuf},
};
pub const BANDS: [(f64, f64); 5] = [
    (40., 120.),
    (120., 400.),
    (400., 1600.),
    (1600., 4800.),
    (4800., 12000.),
];
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub drums: drums::Policy,
    pub overhead: usize,
    pub overhead_file: PathBuf,
    pub listener_confirmed_snare_bleed: bool,
}
impl Policy {
    pub fn validate(&self, s: &Session) -> Result<()> {
        self.drums.validate(s)?;
        if [self.drums.kick, self.drums.snare, self.drums.bass].contains(&self.overhead)
            || !s
                .channels
                .get(self.overhead)
                .is_some_and(|c| matches!(c.role, Role::Overheads) && c.file == self.overhead_file)
        {
            return Err("snare spill analysis requires verified distinct overhead identity".into());
        }
        // Full five-band analysis must be below Nyquist. No silent band remapping.
        if s.sample_rate < 32000 {
            return Err("snare spill analysis requires at least 32 kHz".into());
        }
        Ok(())
    }
}
#[derive(Clone, Serialize)]
pub struct Frame {
    pub seconds: f64,
    pub bands: [[f64; 5]; 5],
    pub broad: [f64; 5],
}
#[derive(Serialize)]
pub struct Measurement {
    pub stages: [&'static str; 5],
    pub bands_hz: [(f64, f64); 5],
    pub frames: Vec<Frame>,
    pub waveform_analysis_rate: f64,
    #[serde(skip)]
    pub low_wave: Vec<[[f64; 2]; 3]>,
}
fn power(x: f64) -> f64 {
    db(x.max(0.).sqrt())
}
/// Full-rate filters and linked production snare processing. Only the low-band
/// correlation representation is block-averaged to approximately 6 kHz.
pub fn measure(s: &Session, root: &Path, p: &Policy, start: f64, end: f64) -> Result<Measurement> {
    p.validate(s)?;
    if !start.is_finite() || !end.is_finite() || start < 0. || end <= start || end > 600. {
        return Err("invalid snare spill span".into());
    }
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
        .map(|x| x.offset.checked_add(x.frames).ok_or("timeline overflow"))
        .collect::<std::result::Result<Vec<_>, _>>()?
        .into_iter()
        .max()
        .unwrap_or(0);
    let end = total.min((end * s.sample_rate as f64) as u64);
    let mut strip = Strip::new(&s.channels[p.drums.snare], s.sample_rate);
    let mut eq_channel = s.channels[p.drums.snare].clone();
    eq_channel.compressor.ratio = 1.;
    eq_channel.compressor.makeup_db = 0.;
    let mut eq = Strip::new(&eq_channel, s.sample_rate);
    let mut filters = (0..5)
        .map(|_| {
            BANDS.map(|(lo, hi)| {
                [
                    [Biquad::highpass(lo, FRAC_1_SQRT_2, s.sample_rate); 2],
                    [Biquad::highpass(hi, FRAC_1_SQRT_2, s.sample_rate); 2],
                ]
            })
        })
        .collect::<Vec<_>>();
    let mut energy = [[0.; 5]; 5];
    let mut broad = [0.; 5];
    let mut count = 0;
    let mut begin = 0;
    let step = (s.sample_rate / 6000).max(1) as u64;
    let mut wave_sum = [[0.; 2]; 3];
    let mut low_wave = vec![];
    let mut frames = vec![];
    for t in 0..end {
        let mut x = [[0.; 2]; 5];
        for (k, i) in [p.drums.kick, p.drums.snare, p.overhead].iter().enumerate() {
            x[k] = sources[*i].next(t)?;
        }
        x[3] = eq.tick(x[1]);
        x[4] = strip.tick(x[1]);
        for k in 0..5 {
            broad[k] += (x[k][0].powi(2) + x[k][1].powi(2)) / 2.;
            for band in 0..5 {
                for ch in 0..2 {
                    let a = filters[k][band][0][ch].tick(x[k][ch]);
                    let b = filters[k][band][1][ch].tick(x[k][ch]);
                    let v = a - b;
                    energy[k][band] += v * v / 2.;
                    if k < 3 && band < 2 {
                        wave_sum[k][ch] += v;
                    }
                }
            }
        }
        // Keep origin-relative indices even when the requested report starts later.
        if (t + 1).is_multiple_of(step) {
            low_wave.push(wave_sum.map(|v| v.map(|x| x / step as f64)));
            wave_sum = [[0.; 2]; 3];
        }
        count += 1;
        if (t + 1) * 100 / s.sample_rate as u64 != t * 100 / s.sample_rate as u64 || t + 1 == end {
            let seconds = begin as f64 / s.sample_rate as f64;
            if seconds >= start {
                frames.push(Frame {
                    seconds,
                    bands: energy.map(|v| v.map(|x| power(x / count as f64))),
                    broad: broad.map(|x| power(x / count as f64)),
                });
            }
            energy = [[0.; 5]; 5];
            broad = [0.; 5];
            count = 0;
            begin = t + 1;
        }
    }
    Ok(Measurement {
        stages: [
            "kick_raw",
            "snare_raw",
            "overheads_raw",
            "snare_eq",
            "snare_compressed_makeup",
        ],
        bands_hz: BANDS,
        frames,
        waveform_analysis_rate: s.sample_rate as f64 / step as f64,
        low_wave,
    })
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Correlation {
    pub absolute: f64,
    pub signed: f64,
    pub lag_ms: f64,
    pub channel_pair: [usize; 2],
}
/// Lagged normalized waveform similarity. Return the actual sign/channel pair;
/// stereo is never cancelled by summing L/R. Delay sign: snare follows reference.
pub fn correlate(a: &[[f64; 2]], b: &[[f64; 2]], rate: f64) -> Correlation {
    let limit = (rate * 0.015).round() as isize;
    let mut best = Correlation {
        absolute: 0.,
        signed: 0.,
        lag_ms: 0.,
        channel_pair: [0, 0],
    };
    for ca in 0..2 {
        for cb in 0..2 {
            for lag in -limit..=limit {
                let (mut aa, mut bb, mut ab, mut sa, mut sb, mut n) = (0., 0., 0., 0., 0., 0.);
                for (i, x) in a.iter().enumerate() {
                    let j = i as isize + lag;
                    if j < 0 || j >= b.len() as isize {
                        continue;
                    }
                    let x = x[ca];
                    let y = b[j as usize][cb];
                    aa += x * x;
                    bb += y * y;
                    ab += x * y;
                    sa += x;
                    sb += y;
                    n += 1.;
                }
                if n < 16. {
                    continue;
                }
                let den = ((aa - sa * sa / n).max(0.) * (bb - sb * sb / n).max(0.)).sqrt();
                if den < 1e-16 {
                    continue;
                }
                let r = ((ab - sa * sb / n) / den).clamp(-1., 1.);
                if r.abs() > best.absolute {
                    best = Correlation {
                        absolute: r.abs(),
                        signed: r,
                        lag_ms: lag as f64 / rate * 1000.,
                        channel_pair: [ca, cb],
                    };
                }
            }
        }
    }
    best
}
fn average(v: impl Iterator<Item = f64>) -> f64 {
    let v = v.collect::<Vec<_>>();
    power(v.iter().map(|x| gain(*x).powi(2)).sum::<f64>() / v.len().max(1) as f64)
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Feature {
    pub seconds: f64,
    pub held_out: bool,
    pub ambiguous_onset: bool,
    pub raw_attack_db: f64,
    pub kick_distance_seconds: f64,
    pub phase_bands: [[[f64; 5]; 5]; 4],
    pub phase_broad: [[f64; 5]; 4],
    pub low_band_kick_similarity: Correlation,
    pub low_band_overhead_similarity: Correlation,
    pub high_band_overhead_envelope_correlation: Option<f64>,
    pub spectrum_distance_db: Option<f64>,
    pub classification: String,
}
pub fn features(m: &Measurement, snare: &[Event], kick: &[Event]) -> Vec<Feature> {
    let mut result = vec![];
    for e in snare {
        let spans = [(0., 0.03), (0.03, 0.08), (0.08, 0.16), (0.16, 0.24)];
        if e.seconds < m.frames.first().map_or(0., |f| f.seconds)
            || e.seconds + 0.24 > m.frames.last().map_or(0., |f| f.seconds + 0.01)
        {
            continue;
        }
        let mut phase_bands = [[[0.; 5]; 5]; 4];
        let mut phase_broad = [[0.; 5]; 4];
        for (phase, (lo, hi)) in spans.iter().enumerate() {
            let fs = m
                .frames
                .iter()
                .filter(|f| f.seconds >= e.seconds + lo - 1e-8 && f.seconds < e.seconds + hi - 1e-8)
                .collect::<Vec<_>>();
            for k in 0..5 {
                phase_broad[phase][k] = average(fs.iter().map(|f| f.broad[k]));
                for (band, value) in phase_bands[phase][k].iter_mut().enumerate() {
                    *value = average(fs.iter().map(|f| f.bands[k][band]));
                }
            }
        }
        let a = ((e.seconds - 0.02).max(0.) * m.waveform_analysis_rate).round() as usize;
        let b = ((e.seconds + 0.08) * m.waveform_analysis_rate).round() as usize;
        if b > m.low_wave.len() {
            continue;
        }
        let wave = (0..3)
            .map(|k| m.low_wave[a..b].iter().map(|x| x[k]).collect::<Vec<_>>())
            .collect::<Vec<_>>();
        let env = m
            .frames
            .iter()
            .filter(|f| f.seconds >= e.seconds - 0.04 && f.seconds < e.seconds + 0.2)
            .map(|f| (f.bands[1][4], f.bands[2][4]))
            .collect::<Vec<_>>();
        let n = env.len() as f64;
        let ma = env.iter().map(|x| x.0).sum::<f64>() / n.max(1.);
        let mb = env.iter().map(|x| x.1).sum::<f64>() / n.max(1.);
        let va = env.iter().map(|x| (x.0 - ma).powi(2)).sum::<f64>();
        let vb = env.iter().map(|x| (x.1 - mb).powi(2)).sum::<f64>();
        let envelope_correlation = (n >= 12. && va / n > 1. && vb / n > 1.)
            .then(|| env.iter().map(|x| (x.0 - ma) * (x.1 - mb)).sum::<f64>() / (va * vb).sqrt());
        result.push(Feature {
            seconds: e.seconds,
            held_out: e.held_out,
            ambiguous_onset: e.ambiguous_onset,
            raw_attack_db: e.raw_attack_db,
            kick_distance_seconds: kick
                .iter()
                .map(|k| (k.seconds - e.seconds).abs())
                .fold(600_f64, f64::min),
            phase_bands,
            phase_broad,
            low_band_kick_similarity: correlate(&wave[0], &wave[1], m.waveform_analysis_rate),
            low_band_overhead_similarity: correlate(&wave[2], &wave[1], m.waveform_analysis_rate),
            high_band_overhead_envelope_correlation: envelope_correlation,
            spectrum_distance_db: None,
            classification: "unclassified_protected".into(),
        });
    }
    result
}
#[derive(Serialize, Deserialize)]
pub struct Diagnosis {
    pub direct_reference_level: Option<f64>,
    pub direct_reference_shape: Option<[f64; 5]>,
    pub events: Vec<Feature>,
    pub counts: serde_json::Value,
    pub interpretation: Vec<String>,
    pub listener_confirmed_bleed: bool,
}
fn shape(f: &Feature) -> [f64; 5] {
    let p = f.phase_bands[0][1];
    let total = 10.
        * p.iter()
            .map(|x| 10_f64.powf(x / 10.))
            .sum::<f64>()
            .max(1e-24)
            .log10();
    p.map(|v| v - total)
}
pub fn classify(f: Vec<Feature>, confirmed: bool) -> Diagnosis {
    let reference = drums::quantile(
        f.iter()
            .filter(|e| !e.held_out && !e.ambiguous_onset)
            .map(|e| e.raw_attack_db)
            .collect(),
        0.75,
    );
    let anchors = f
        .iter()
        .filter(|e| {
            !e.held_out
                && !e.ambiguous_onset
                && reference.is_some_and(|x| e.raw_attack_db >= x - 3.)
        })
        .collect::<Vec<_>>();
    let template = (anchors.len() >= 8).then(|| {
        std::array::from_fn(|i| {
            drums::quantile(anchors.iter().map(|e| shape(e)[i]).collect(), 0.5).unwrap()
        })
    });
    classify_with_reference(f, confirmed, reference, template)
}
pub fn classify_with_reference(
    mut f: Vec<Feature>,
    confirmed: bool,
    reference: Option<f64>,
    template: Option<[f64; 5]>,
) -> Diagnosis {
    if let (Some(level), Some(template)) = (reference, template) {
        let strong = f
            .iter()
            .filter(|e| e.raw_attack_db >= level - 6.)
            .map(|e| e.seconds)
            .collect::<Vec<_>>();
        for e in &mut f {
            let distance = drums::quantile(
                shape(e)
                    .iter()
                    .zip(template)
                    .map(|(a, b)| (a - b).abs())
                    .collect(),
                0.5,
            )
            .unwrap();
            e.spectrum_distance_db = Some(distance);
            if e.ambiguous_onset {
                e.classification = "compound_event_protected".into();
            } else if e.raw_attack_db >= level - 6. || distance <= 4. {
                e.classification = "snare_like_protected".into();
            } else if confirmed
                && e.raw_attack_db < level - 8.
                && e.kick_distance_seconds <= 0.04
                && e.low_band_kick_similarity.absolute >= 0.55
                && strong
                    .iter()
                    .all(|t| e.seconds - t >= 0.12 || t - e.seconds >= 0.08)
            {
                e.classification = "kick_correlated_spill_candidate".into();
            }
        }
    }
    let counts=[false,true].map(|held|{
        let rows=f.iter().filter(|e|e.held_out==held).collect::<Vec<_>>();
        serde_json::json!({"held_out":held,"events":rows.len(),"snare_like":rows.iter().filter(|e|e.classification=="snare_like_protected").count(),"compound":rows.iter().filter(|e|e.ambiguous_onset).count(),"spill_candidates":rows.iter().filter(|e|e.classification=="kick_correlated_spill_candidate").count(),"unclassified":rows.iter().filter(|e|e.classification=="unclassified_protected").count()})
    });
    Diagnosis{direct_reference_level:reference,direct_reference_shape:template,events:f,counts:serde_json::json!(counts),listener_confirmed_bleed:confirmed,interpretation:vec![if confirmed {"Listener confirms bleed; event assignments remain hypotheses. Correlation can also arise from unison playing or common resonances."}else{"No listener confirmation supplied; waveform relationships remain hypotheses."}.into(),"Strong snare training events define a relative spectral-shape reference. Quiet template-like and compound events are protected, not relabelled as bleed.".into(),"Band powers use full-rate HPF differences. Low-band lagged waveform similarity uses a block-averaged analysis representation; no audio is resampled or subtracted.".into()]}
}
#[derive(Serialize)]
pub struct StaticResult {
    pub name: String,
    pub split: [serde_json::Value; 2],
    pub training_eligible: bool,
    pub accepted: bool,
    pub reasons: Vec<String>,
}
pub fn evaluate(name: &str, b: &Diagnosis, a: &[Feature]) -> StaticResult {
    let mut reasons = vec![];
    let mut allowed = [true; 2];
    if b.events.len() != a.len() || b.events.iter().zip(a).any(|(x, y)| x.seconds != y.seconds) {
        return StaticResult {
            name: name.into(),
            split: [serde_json::Value::Null, serde_json::Value::Null],
            training_eligible: false,
            accepted: false,
            reasons: vec!["feature timeline mismatch".into()],
        };
    }
    let split=[false,true].map(|held|{
        let mut spill=vec![];let mut direct=0;let mut body_loss=0_f64;let mut high_loss=0_f64;let mut quiet_loss=0_f64;
        let mut worst_body=None;let mut worst_high=None;let mut worst_quiet=None;
        for (x,y) in b.events.iter().zip(a).filter(|(x,_)|x.held_out==held){
            let change=y.phase_broad[0][4]-x.phase_broad[0][4];
            if x.classification=="kick_correlated_spill_candidate" {spill.push(-change);}
            else if -change>quiet_loss {quiet_loss = -change;worst_quiet=Some(x.seconds);}
            if x.classification=="snare_like_protected" {
                direct+=1;
                for phase in 0..2 {let loss=x.phase_bands[phase][4][1]-y.phase_bands[phase][4][1];if loss>body_loss {body_loss=loss;worst_body=Some(x.seconds);}}
                let loss=x.phase_bands[0][4][4]-y.phase_bands[0][4][4];if loss>high_loss {high_loss=loss;worst_high=Some(x.seconds);}
            }
        }
        let improvement=drums::quantile(spill.clone(),0.5);
        let pass=spill.len()>=8&&direct>=8&&improvement.is_some_and(|x|x>=1.)&&body_loss<=0.75&&high_loss<=1.5&&quiet_loss<=0.75;
        allowed[held as usize]=pass;
        for (failed,reason) in [(spill.len()<8,"insufficient_spill_events"),(direct<8,"insufficient_snare_reference_events"),(improvement.is_none_or(|x|x<1.),"spill_reduction_below_1_db"),(body_loss>0.75,"body_loss_over_0_75_db"),(high_loss>1.5,"high_attack_loss_over_1_5_db"),(quiet_loss>0.75,"protected_event_loss_over_0_75_db")] {
            if failed {reasons.push(format!("held_out={held}: {reason}"));}
        }
        serde_json::json!({"spill_events":spill.len(),"protected_snare_events":direct,"median_spill_reduction_db":improvement,"max_body_loss_db":body_loss,"max_high_attack_loss_db":high_loss,"max_protected_broadband_loss_db":quiet_loss,"worst_body_seconds":worst_body,"worst_high_seconds":worst_high,"worst_protected_seconds":worst_quiet,"pass":pass})
    });
    StaticResult {
        name: name.into(),
        split,
        training_eligible: allowed[0],
        accepted: allowed.iter().all(|x| *x),
        reasons,
    }
}
#[derive(Serialize)]
pub struct StaticProposal {
    pub name: String,
    pub settings: Option<Session>,
    pub abstention: Option<String>,
}
/// Fixed budget; missing bands or session bounds become explicit abstentions.
pub fn static_proposals(s: &Session, p: &Policy) -> Result<Vec<StaticProposal>> {
    p.validate(s)?;
    let ch = &s.channels[p.drums.snare];
    let low = ch
        .eq
        .iter()
        .position(|b| b.kind == EqKind::Bell && (100. ..=180.).contains(&b.hz));
    let high = ch
        .eq
        .iter()
        .position(|b| b.kind == EqKind::HighShelf && (4000. ..=8000.).contains(&b.hz));
    Ok([
        ("high-relief", false, true),
        ("body-relief", true, false),
        ("combined-relief", true, true),
    ]
    .into_iter()
    .map(|(name, l, h)| {
        if (l && low.is_none()) || (h && high.is_none()) {
            return StaticProposal {
                name: name.into(),
                settings: None,
                abstention: Some(
                    "Compatible existing EQ band is absent; no substitute is invented.".into(),
                ),
            };
        }
        let mut c = s.clone();
        if l {
            c.channels[p.drums.snare].eq[low.unwrap()].db -= 2.;
        }
        if h {
            c.channels[p.drums.snare].eq[high.unwrap()].db -= 2.;
        }
        match c.validate() {
            Ok(()) => StaticProposal {
                name: name.into(),
                settings: Some(c),
                abstention: None,
            },
            Err(e) => StaticProposal {
                name: name.into(),
                settings: None,
                abstention: Some(format!("Proposed correction violates session bounds: {e}")),
            },
        }
    })
    .collect())
}

pub fn run(s: Session, root: &Path, out: &Path, p: Policy, start: f64, end: f64) -> Result<()> {
    p.validate(&s)?;
    if out.exists() {
        return Err("output already exists".into());
    }
    let drums = drums::measure(&s, root, &p.drums, start, end)?;
    let event_diagnosis = drums::diagnose(&drums);
    let baseline = measure(&s, root, &p, start, end)?;
    let diagnosis = classify(
        features(
            &baseline,
            &event_diagnosis.events[1],
            &event_diagnosis.events[0],
        ),
        p.listener_confirmed_snare_bleed,
    );
    std::fs::create_dir(out)?;
    write_json(&out.join("before-settings.json"), &s)?;
    write_json(&out.join("policy.json"), &p)?;
    write_json(&out.join("events.json"), &event_diagnosis)?;
    write_json(&out.join("baseline.json"), &baseline)?;
    write_json(&out.join("diagnosis.json"), &diagnosis)?;
    write_json(
        &out.join("prediction-probes.json"),
        &[
            prediction_probe(&baseline, &diagnosis, 0),
            prediction_probe(&baseline, &diagnosis, 2),
        ],
    )?;
    let mut candidates = vec![];
    let mut results = vec![];
    for proposal in static_proposals(&s, &p)? {
        let name = &proposal.name;
        let dir = out.join(name);
        std::fs::create_dir(&dir)?;
        write_json(&dir.join("proposal.json"), &proposal)?;
        let Some(c) = proposal.settings.clone() else {
            let result = StaticResult {
                name: name.clone(),
                split: [serde_json::Value::Null, serde_json::Value::Null],
                training_eligible: false,
                accepted: false,
                reasons: vec![
                    proposal
                        .abstention
                        .unwrap_or_else(|| "No supported proposal".into()),
                ],
            };
            write_json(&dir.join("outcome.json"), &result)?;
            candidates.push(s.clone());
            results.push(result);
            continue;
        };
        let measured = measure(&c, root, &p, start, end)?;
        let f = features(
            &measured,
            &event_diagnosis.events[1],
            &event_diagnosis.events[0],
        );
        let result = evaluate(name, &diagnosis, &f);
        write_json(&dir.join("settings.json"), &c)?;
        write_json(&dir.join("measurement.json"), &measured)?;
        write_json(&dir.join("features.json"), &f)?;
        write_json(&dir.join("outcome.json"), &result)?;
        candidates.push(c);
        results.push(result);
    }
    // Select once from training; never use held-out scores to choose a fallback.
    let chosen = results.iter().position(|r| r.training_eligible);
    let accepted = chosen.filter(|i| results[*i].accepted);
    write_json(
        &out.join("decision.json"),
        &serde_json::json!({"chosen_on_training":chosen,"accepted":accepted.is_some(),"results":results,"listener_preference":null,"no_gate_or_expander":true,"remaining_issue":"Listener-confirmed bleed remains unresolved; a static proxy cannot establish source separation or preferred tone."}),
    )?;
    write_json(
        &out.join("settings.json"),
        accepted.map(|i| &candidates[i]).unwrap_or(&s),
    )
}

#[derive(Serialize, Deserialize)]
pub struct PredictionProbe {
    pub reference_stage: usize,
    pub coefficient: Option<f64>,
    pub lag_ms: Option<f64>,
    pub channel_pair: Option<[usize; 2]>,
    pub training_windows: usize,
    pub evaluation: Vec<serde_json::Value>,
    pub interpretation: String,
}
/// Fit a single delayed low-band predictor to weak kick-coincident events.
/// These events are a hypothesis set, NOT ground-truth bleed labels. Evaluate
/// the frozen model on strong/quiet and held-out events; never subtract audio.
pub fn prediction_probe(m: &Measurement, d: &Diagnosis, reference: usize) -> PredictionProbe {
    let level = d.direct_reference_level.unwrap_or(-240.);
    let weak = |e: &Feature| {
        !e.ambiguous_onset && e.raw_attack_db < level - 8. && e.kick_distance_seconds <= 0.04
    };
    let train = d
        .events
        .iter()
        .filter(|e| !e.held_out && weak(e))
        .collect::<Vec<_>>();
    let mut result=PredictionProbe{reference_stage:reference,coefficient:None,lag_ms:None,channel_pair:None,training_windows:train.len(),evaluation:vec![],interpretation:"Predictability is not source identity or a safe subtraction recipe. Weak kick-coincident hits may include genuine ghost notes; overheads also contain genuine snare. This probe changes no audio.".into()};
    if train.len() < 8 || ![0, 2].contains(&reference) {
        return result;
    }
    let bounds = |e: &Feature| {
        let a = ((e.seconds - 0.02).max(0.) * m.waveform_analysis_rate).round() as usize;
        let b = ((e.seconds + 0.08) * m.waveform_analysis_rate).round() as usize;
        (a, b.min(m.low_wave.len()))
    };
    let limit = (m.waveform_analysis_rate * 0.015).round() as isize;
    let mut best = (f64::NEG_INFINITY, 0., 0_isize, 0, 0);
    for ca in 0..2 {
        for cb in 0..2 {
            for lag in -limit..=limit {
                let (mut xx, mut xy) = (0., 0.);
                for e in &train {
                    let (a, b) = bounds(e);
                    for j in a..b {
                        let k = j as isize - lag;
                        if k < 0 || k >= m.low_wave.len() as isize {
                            continue;
                        }
                        let x = m.low_wave[k as usize][reference][ca];
                        let y = m.low_wave[j][1][cb];
                        xx += x * x;
                        xy += x * y;
                    }
                }
                if xx < 1e-16 {
                    continue;
                }
                let coefficient = xy / xx;
                // Reject an implausibly large cross-channel predictor rather than
                // letting a weak reference explain anything through arbitrary gain.
                if coefficient.abs() > 4. {
                    continue;
                }
                let explained = xy * xy / xx;
                if explained > best.0 {
                    best = (explained, coefficient, lag, ca, cb);
                }
            }
        }
    }
    if !best.0.is_finite() {
        return result;
    }
    let (_, coefficient, lag, ca, cb) = best;
    result.coefficient = Some(coefficient);
    result.lag_ms = Some(lag as f64 / m.waveform_analysis_rate * 1000.);
    result.channel_pair = Some([ca, cb]);
    result.evaluation = evaluate_prediction(m, d, &result);
    result
}

/// Apply saved predictor parameters to a new diagnostic span without refitting.
pub fn evaluate_prediction(
    m: &Measurement,
    d: &Diagnosis,
    model: &PredictionProbe,
) -> Vec<serde_json::Value> {
    let (Some(coefficient), Some(lag_ms), Some([ca, cb])) =
        (model.coefficient, model.lag_ms, model.channel_pair)
    else {
        return vec![];
    };
    let reference = model.reference_stage;
    if ![0, 2].contains(&reference)
        || ca > 1
        || cb > 1
        || !coefficient.is_finite()
        || coefficient.abs() > 4.
        || !lag_ms.is_finite()
        || lag_ms.abs() > 15.
    {
        return vec![serde_json::json!({"error":"invalid frozen predictor bounds"})];
    }
    let lag = (lag_ms * m.waveform_analysis_rate / 1000.).round() as isize;
    let level = d.direct_reference_level.unwrap_or(-240.);
    let weak = |e: &Feature| {
        !e.ambiguous_onset && e.raw_attack_db < level - 8. && e.kick_distance_seconds <= 0.04
    };
    let bounds = |e: &Feature| {
        let a = ((e.seconds - 0.02).max(0.) * m.waveform_analysis_rate).round() as usize;
        let b = ((e.seconds + 0.08) * m.waveform_analysis_rate).round() as usize;
        (a, b.min(m.low_wave.len()))
    };
    let mut evaluation = Vec::new();
    for held in [false, true] {
        for weak_group in [false, true] {
            let mut ratios = vec![];
            let mut events = vec![];
            for e in d.events.iter().filter(|e| {
                e.held_out == held
                    && if weak_group {
                        weak(e)
                    } else {
                        e.raw_attack_db >= level - 6. && !e.ambiguous_onset
                    }
            }) {
                let (a, b) = bounds(e);
                let (mut raw, mut residual) = (0., 0.);
                for j in a..b {
                    let k = j as isize - lag;
                    if k < 0 || k >= m.low_wave.len() as isize {
                        continue;
                    }
                    let x = m.low_wave[k as usize][reference][ca];
                    let y = m.low_wave[j][1][cb];
                    raw += y * y;
                    residual += (y - coefficient * x).powi(2);
                }
                if raw > 1e-16 {
                    let explained = 1. - residual / raw;
                    ratios.push(explained);
                    events.push(serde_json::json!({"seconds":e.seconds,"explained_energy_fraction":explained}));
                }
            }
            evaluation.push(serde_json::json!({"held_out":held,"weak_kick_coincident_hypothesis":weak_group,"events":events,"median_explained_energy_fraction":drums::quantile(ratios.clone(),0.5),"p10_explained_energy_fraction":drums::quantile(ratios,0.1)}));
        }
    }
    evaluation
}

pub fn predict_saved(root: &Path, pilot: &Path, out: &Path, start: f64, end: f64) -> Result<()> {
    if out.exists() {
        return Err("output already exists".into());
    }
    let s: Session =
        serde_json::from_reader(std::fs::File::open(pilot.join("before-settings.json"))?)?;
    let p: Policy = serde_json::from_reader(std::fs::File::open(pilot.join("policy.json"))?)?;
    let d: Diagnosis = serde_json::from_reader(std::fs::File::open(pilot.join("diagnosis.json"))?)?;
    let m = measure(&s, root, &p, start, end)?;
    write_json(
        out,
        &[prediction_probe(&m, &d, 0), prediction_probe(&m, &d, 2)],
    )
}
/// Whole-source diagnostic check with the pilot's frozen reference, no new
/// profile estimation, candidate search, rendering or audio subtraction.
pub fn inspect_frozen(root: &Path, pilot: &Path, out: &Path, start: f64, end: f64) -> Result<()> {
    if out.exists() {
        return Err("output already exists".into());
    }
    let s: Session =
        serde_json::from_reader(std::fs::File::open(pilot.join("before-settings.json"))?)?;
    let p: Policy = serde_json::from_reader(std::fs::File::open(pilot.join("policy.json"))?)?;
    let d: Diagnosis = serde_json::from_reader(std::fs::File::open(pilot.join("diagnosis.json"))?)?;
    let dm = drums::measure(&s, root, &p.drums, start, end)?;
    let events = drums::diagnose(&dm);
    let m = measure(&s, root, &p, start, end)?;
    let diagnosis = classify_with_reference(
        features(&m, &events.events[1], &events.events[0]),
        p.listener_confirmed_snare_bleed,
        d.direct_reference_level,
        d.direct_reference_shape,
    );
    std::fs::create_dir(out)?;
    write_json(&out.join("before-settings.json"), &s)?;
    write_json(&out.join("policy.json"), &p)?;
    write_json(
        &out.join("frozen-reference.json"),
        &serde_json::json!({"pilot":pilot,"reference_level":d.direct_reference_level,"reference_shape":d.direct_reference_shape,"selection_rerun":false}),
    )?;
    write_json(&out.join("measurement.json"), &m)?;
    write_json(&out.join("diagnosis.json"), &diagnosis)?;
    let model_path = pilot.join("prediction-probes.json");
    if model_path.exists() {
        let models: Vec<PredictionProbe> =
            serde_json::from_reader(std::fs::File::open(&model_path)?)?;
        write_json(&out.join("frozen-prediction-evaluation.json"),&models.iter().map(|model|serde_json::json!({"frozen_model":model,"full_evaluation":evaluate_prediction(&m,&diagnosis,model),"refitted":false})).collect::<Vec<_>>())?;
    }
    write_json(&out.join("events.json"), &events)
}
