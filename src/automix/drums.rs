//! Offline kick/snare diagnosis. All signal processing uses the production strips.
use super::{
    balance, bass,
    config::{Role, Session},
    dsp::{Biquad, Strip, db, gain},
    render::{Source, route, write_json},
};
use crate::inventory::Result;
use serde::{Deserialize, Serialize};
use std::{
    f64::consts::FRAC_1_SQRT_2,
    path::{Path, PathBuf},
};
pub const STAGES: [&str; 15] = [
    "kick_raw",
    "kick_eq",
    "kick_compressed_makeup",
    "kick_routed",
    "kick_master_hpf",
    "snare_raw",
    "snare_eq",
    "snare_compressed_makeup",
    "snare_routed",
    "snare_master_hpf",
    "bass_master_hpf",
    "guitars_master_hpf",
    "vocals_master_hpf",
    "mix_pre_hpf",
    "mix_master_hpf",
];
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub kick: usize,
    pub kick_file: PathBuf,
    pub snare: usize,
    pub snare_file: PathBuf,
    pub bass: usize,
    pub bass_file: PathBuf,
    /// Explicit balance reference, never inferred from absolute fader numbers.
    pub neutral_basis: String,
    pub current_emphasis_db: Option<[f64; 2]>,
    pub desired_emphasis_db: [f64; 2],
}
impl Policy {
    pub fn validate(&self, s: &Session) -> Result<()> {
        balance::validate_baseline(s)?;
        if s.effects.is_some()
            || self.neutral_basis.trim().is_empty()
            || [self.kick, self.snare, self.bass]
                .iter()
                .enumerate()
                .any(|(a, i)| [self.kick, self.snare, self.bass][..a].contains(i))
        {
            return Err(
                "drum policy requires distinct verified inputs, a balance basis and no FX".into(),
            );
        }
        for (i, file, role) in [
            (self.kick, &self.kick_file, Role::Kick),
            (self.snare, &self.snare_file, Role::Snare),
            (self.bass, &self.bass_file, Role::BassDi),
        ] {
            if !s.channels.get(i).is_some_and(|c| {
                std::mem::discriminant(&c.role) == std::mem::discriminant(&role) && &c.file == file
            }) {
                return Err("drum input identity mismatch".into());
            }
        }
        if !self
            .desired_emphasis_db
            .iter()
            .chain(self.current_emphasis_db.iter().flatten())
            .all(|x| x.is_finite() && (-3. ..=3.).contains(x))
        {
            return Err("rhythmic intent outside bounded +/-3 dB".into());
        }
        balance::rhythmic_emphasis_delta(self.current_emphasis_db, self.desired_emphasis_db)?;
        Ok(())
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Frame {
    pub seconds: f64,
    pub duration: f64,
    pub rms: [f64; 15],
    pub peak: [f64; 15],
    pub mean_gr: [f64; 2],
    pub max_gr: [f64; 2],
    pub role_band: [f64; 2],
}
#[derive(Serialize)]
pub struct Spectral {
    pub seconds: f64,
    pub bands: Vec<[f64; 9]>,
    pub bass_note: Option<bass::Note>,
    pub pcm_contact_fraction: [f64; 2],
}
#[derive(Serialize)]
pub struct Measurement {
    pub stages: [&'static str; 15],
    pub frames: Vec<Frame>,
    pub spectra: Vec<Spectral>,
    pub pcm_contacts: [u64; 2],
    pub rate: u32,
}
fn power(x: f64) -> f64 {
    db(x.max(0.).sqrt())
}
pub fn measure(s: &Session, root: &Path, p: &Policy, start: f64, end: f64) -> Result<Measurement> {
    p.validate(s)?;
    if !start.is_finite() || !end.is_finite() || start < 0. || end <= start || end > 600. {
        return Err("drum span must be within 0..600 seconds".into());
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
    let mut strips = s
        .channels
        .iter()
        .map(|c| Strip::new(c, s.sample_rate))
        .collect::<Vec<_>>();
    let mut eq = [p.kick, p.snare].map(|i| {
        let mut c = s.channels[i].clone();
        c.compressor.ratio = 1.;
        c.compressor.makeup_db = 0.;
        Strip::new(&c, s.sample_rate)
    });
    let mut hp = vec![[Biquad::highpass(s.master_hpf_hz, FRAC_1_SQRT_2, s.sample_rate); 2]; 6];
    let mut role_filters =
        [40., 120., 120., 500.].map(|hz| [Biquad::highpass(hz, FRAC_1_SQRT_2, s.sample_rate); 2]);
    let mut frames = vec![];
    let mut spectra = vec![];
    let mut sums = [0.; 15];
    let mut peaks = [0_f64; 15];
    let mut gr = [0.; 2];
    let mut grmax = [0_f64; 2];
    let mut bands = [0.; 2];
    let mut pcm_contacts = [0; 2];
    let mut block_contacts = [0_u64; 2];
    let mut blocks = vec![vec![[0.; 2]; bass::SIZE]; 15];
    let mut bass_raw = vec![[0.; 2]; bass::SIZE];
    let mut count = 0_u64;
    let mut begin = 0;
    for t in 0..end {
        let mut x = [[0.; 2]; 15];
        for (i, (src, strip)) in sources.iter_mut().zip(&mut strips).enumerate() {
            let contacts = src.full_scale_samples();
            let raw = src.next(t)?;
            let post = strip.tick(raw);
            let routed = route(
                post,
                src.channels,
                s.channels[i].pan,
                gain(s.channels[i].fader_db),
            );
            for (d, index) in [p.kick, p.snare].iter().enumerate() {
                if i == *index {
                    let k = d * 5;
                    x[k] = raw;
                    x[k + 1] = eq[d].tick(raw);
                    x[k + 2] = post;
                    x[k + 3] = routed;
                    x[k + 4] = routed;
                    gr[d] += strip.reduction_db();
                    grmax[d] = grmax[d].max(strip.reduction_db());
                    if t as f64 / s.sample_rate as f64 >= start && !src.is_float() {
                        pcm_contacts[d] += src.full_scale_samples() - contacts;
                        block_contacts[d] += src.full_scale_samples() - contacts;
                    }
                    for (ch, v) in raw.iter().enumerate() {
                        let a = role_filters[2 * d][ch].tick(*v);
                        let b = role_filters[2 * d + 1][ch].tick(*v);
                        bands[d] += (a - b).powi(2) / 2.;
                    }
                }
            }
            if i == p.bass {
                x[10] = routed;
                bass_raw[t as usize % bass::SIZE] = raw;
            }
            for ch in 0..2 {
                if matches!(
                    s.channels[i].role,
                    Role::RhythmGuitar | Role::LeadGuitar | Role::AcousticGuitar
                ) {
                    x[11][ch] += routed[ch];
                }
                if matches!(
                    s.channels[i].role,
                    Role::LeadVocal | Role::BackingVocal | Role::VocalRoom
                ) {
                    x[12][ch] += routed[ch];
                }
                x[13][ch] += routed[ch] * gain(s.master_db);
            }
        }
        x[14] = x[13];
        for (h, k) in [4, 9, 10, 11, 12, 14].iter().enumerate() {
            for ch in 0..2 {
                x[*k][ch] = hp[h][ch].tick(x[*k][ch]);
            }
        }
        for k in 0..15 {
            blocks[k][t as usize % bass::SIZE] = x[k];
            sums[k] += (x[k][0].powi(2) + x[k][1].powi(2)) / 2.;
            peaks[k] = peaks[k].max(x[k][0].abs().max(x[k][1].abs()));
        }
        count += 1;
        // Rational boundaries retain all samples, including a partial final frame.
        if (t + 1) * 100 / s.sample_rate as u64 != t * 100 / s.sample_rate as u64 || t + 1 == end {
            let seconds = begin as f64 / s.sample_rate as f64;
            if seconds >= start {
                frames.push(Frame {
                    seconds,
                    duration: count as f64 / s.sample_rate as f64,
                    rms: sums.map(|x| power(x / count as f64)),
                    peak: peaks.map(db),
                    mean_gr: gr.map(|x| x / count as f64),
                    max_gr: grmax,
                    role_band: bands.map(|x| power(x / count as f64)),
                });
            }
            sums = [0.; 15];
            peaks = [0.; 15];
            gr = [0.; 2];
            grmax = [0.; 2];
            bands = [0.; 2];
            count = 0;
            begin = t + 1;
        }
        if ((t + 1) as usize).is_multiple_of(bass::SIZE) {
            let seconds = (t + 1 - bass::SIZE as u64) as f64 / s.sample_rate as f64;
            if seconds >= start {
                spectra.push(Spectral {
                    seconds,
                    bands: blocks
                        .iter()
                        .map(|x| {
                            let sp = bass::spectrum(x);
                            bass::BANDS
                                .map(|(lo, hi)| power(bass::band(&sp, s.sample_rate, lo, hi)))
                        })
                        .collect(),
                    bass_note: bass::estimate_note(&bass_raw, s.sample_rate),
                    pcm_contact_fraction: std::array::from_fn(|d| {
                        block_contacts[d] as f64
                            / (bass::SIZE * sources[[p.kick, p.snare][d]].channels) as f64
                    }),
                });
            }
            block_contacts = [0; 2];
        }
    }
    Ok(Measurement {
        stages: STAGES,
        frames,
        spectra,
        pcm_contacts,
        rate: s.sample_rate,
    })
}
pub fn quantile(mut v: Vec<f64>, q: f64) -> Option<f64> {
    v.retain(|x| x.is_finite());
    v.sort_by(f64::total_cmp);
    if v.is_empty() {
        None
    } else {
        Some(v[((v.len() - 1) as f64 * q).round() as usize])
    }
}
#[derive(Clone, Serialize)]
pub struct Event {
    pub frame: usize,
    pub seconds: f64,
    pub held_out: bool,
    pub next_seconds: Option<f64>,
    pub raw_attack_db: f64,
    pub rise_candidates_seconds: Vec<f64>,
    pub ambiguous_onset: bool,
    pub attack_gr_db: f64,
    pub body_gr_db: f64,
    pub recovery_gr_db: Option<f64>,
    pub pre_hit_gr_db: f64,
    pub attack_body_db: f64,
    pub pre_compressor_attack_body_db: f64,
    pub tail_attack_db: Option<f64>,
}
fn average(frames: &[Frame], stage: usize) -> f64 {
    power(
        frames
            .iter()
            .map(|x| gain(x.rms[stage]).powi(2) * x.duration)
            .sum::<f64>()
            / frames.iter().map(|x| x.duration).sum::<f64>().max(1e-12),
    )
}
fn mean_gr(frames: &[Frame], d: usize) -> f64 {
    frames.iter().map(|x| x.mean_gr[d]).sum::<f64>() / frames.len().max(1) as f64
}
/// Events are energy rises, not verified isolated drum hits. Masks use raw input only.
pub fn events(frames: &[Frame], d: usize) -> Vec<Event> {
    let floor = quantile(
        frames
            .iter()
            .filter(|f| ((f.seconds / 12.).floor() as u64).is_multiple_of(2))
            .map(|f| f.role_band[d])
            .collect(),
        0.95,
    )
    .unwrap_or(-120.)
        - 35.;
    // A weak precursor must not lock out a stronger attack. Offline clustering
    // considers the complete bounded 80 ms neighborhood; retain competing rises
    // so a possible flam/ghost note is not relabelled as an isolated hit.
    let mut clusters: Vec<(usize, usize, Vec<usize>)> = vec![];
    for i in 4..frames.len().saturating_sub(25) {
        let before = frames[i - 4..i].iter().map(|x| x.role_band[d]).sum::<f64>() / 4.;
        if frames[i].role_band[d] <= floor.max(-65.) || frames[i].role_band[d] - before < 5. {
            continue;
        }
        let a = (i..i + 3)
            .max_by(|a, b| frames[*a].role_band[d].total_cmp(&frames[*b].role_band[d]))
            .unwrap();
        if let Some((first, strongest, candidates)) = clusters.last_mut()
            && frames[a].seconds - frames[*first].seconds < 0.08
            && (frames[a].seconds / 12.).floor() == (frames[*first].seconds / 12.).floor()
        {
            if !candidates.contains(&a) {
                candidates.push(a);
            }
            if frames[a].role_band[d] > frames[*strongest].role_band[d] {
                *strongest = a;
            }
        } else {
            clusters.push((a, a, vec![a]));
        }
    }
    let anchors = clusters.iter().map(|(_, a, _)| *a).collect::<Vec<_>>();
    anchors
        .iter()
        .enumerate()
        .filter_map(|(n, &a)| {
            let f = &frames[a];
            let last = &frames[a + 23];
            if (f.seconds / 12.).floor() != (last.seconds / 12.).floor() {
                return None;
            }
            // The next cluster's quiet precursor already interrupts this decay,
            // even when that cluster's strongest peak arrives after 240 ms.
            let next = clusters
                .get(n + 1)
                .map(|(first, _, _)| frames[*first].seconds - f.seconds);
            let rises = clusters[n]
                .2
                .iter()
                .map(|a| frames[*a].seconds)
                .collect::<Vec<_>>();
            let ambiguous = rises.last().unwrap() - rises[0] >= 0.03 - 1e-8;
            let isolated = next.is_none_or(|x| x >= 0.24) && !ambiguous;
            let post = d * 5 + 2;
            Some(Event {
                frame: a,
                seconds: f.seconds,
                held_out: (f.seconds / 12.).floor() as u64 % 2 == 1,
                next_seconds: next,
                raw_attack_db: average(&frames[a..a + 3], d * 5),
                rise_candidates_seconds: rises,
                ambiguous_onset: ambiguous,
                attack_gr_db: mean_gr(&frames[a..a + 3], d),
                body_gr_db: mean_gr(&frames[a + 3..a + 8], d),
                recovery_gr_db: isolated.then(|| mean_gr(&frames[a + 15..a + 24], d)),
                pre_hit_gr_db: frames[a.saturating_sub(2)].mean_gr[d],
                attack_body_db: average(&frames[a..a + 3], post)
                    - average(&frames[a + 3..a + 8], post),
                pre_compressor_attack_body_db: average(&frames[a..a + 3], d * 5 + 1)
                    - average(&frames[a + 3..a + 8], d * 5 + 1),
                tail_attack_db: isolated.then(|| {
                    average(&frames[a + 8..a + 24], post) - average(&frames[a..a + 3], post)
                }),
            })
        })
        .collect()
}
#[derive(Serialize)]
pub struct Diagnosis {
    pub events: [Vec<Event>; 2],
    pub summary: serde_json::Value,
    pub confidence: String,
    pub unresolved: Vec<String>,
}
pub fn diagnose(m: &Measurement) -> Diagnosis {
    let ev = [events(&m.frames, 0), events(&m.frames, 1)];
    let summary=ev.each_ref().map(|e|{
        [false,true].map(|held|{let e=e.iter().filter(|e|e.held_out==held).collect::<Vec<_>>();serde_json::json!({"events":e.len(),"attack_gr_median":quantile(e.iter().map(|e|e.attack_gr_db).collect(),0.5),"body_gr_median":quantile(e.iter().map(|e|e.body_gr_db).collect(),0.5),"recovery_gr_median":quantile(e.iter().filter_map(|e|e.recovery_gr_db).collect(),0.5),"pre_hit_gr_p95":quantile(e.iter().map(|e|e.pre_hit_gr_db).collect(),0.95),"compression_attack_body_change_median":quantile(e.iter().map(|e|e.attack_body_db-e.pre_compressor_attack_body_db).collect(),0.5),"tail_attack_median":quantile(e.iter().filter_map(|e|e.tail_attack_db).collect(),0.5)})})
    });
    let enough = ev.iter().all(|e| {
        [false, true]
            .iter()
            .all(|h| e.iter().filter(|e| e.held_out == *h).count() >= 8)
    });
    Diagnosis{events:ev,summary:serde_json::json!(summary),confidence:if enough{"sufficient repeated energy events; source isolation unverified"}else{"insufficient events in one or both splits"}.into(),unresolved:vec!["Energy rises may include bleed or unison playing; controlled isolated hits are needed to establish capture causes.".into(),"Tone ratios and band competition do not establish preferred timbre or perceptual masking.".into(),"A provisional neutral fit is not a listener-approved neutral balance.".into()]}
}
pub fn analyze(s: Session, root: &Path, out: &Path, p: Policy, start: f64, end: f64) -> Result<()> {
    if out.exists() {
        return Err("output already exists".into());
    }
    let m = measure(&s, root, &p, start, end)?;
    let d = diagnose(&m);
    std::fs::create_dir(out)?;
    write_json(&out.join("settings.json"), &s)?;
    write_json(&out.join("policy.json"), &p)?;
    write_json(&out.join("diagnosis.json"), &d)?;
    write_json(&out.join("measurement.json"), &m)
}

#[derive(Serialize)]
pub struct ProcessingPlan {
    pub threshold_relief_db: [f64; 2],
    pub reasons: [String; 2],
    pub tone_abstention: String,
}
/// A reduction maximum alone cannot trigger relief. Require repeated loss of
/// attack/body contrast AND persistent recovery attenuation in training events.
pub fn processing_plan(d: &Diagnosis) -> ProcessingPlan {
    let mut delta = [0.; 2];
    let reasons = std::array::from_fn(|i| {
        let e = d.events[i]
            .iter()
            .filter(|e| {
                !e.held_out && !e.ambiguous_onset && e.next_seconds.is_none_or(|t| t >= 0.08)
            })
            .collect::<Vec<_>>();
        let recover = e
            .iter()
            .filter_map(|e| e.recovery_gr_db)
            .collect::<Vec<_>>();
        if e.len() < 8 || recover.len() < 8 {
            return "Abstain: insufficient isolated training events for compression relief.".into();
        }
        let damaged = e
            .iter()
            .filter(|e| e.attack_body_db - e.pre_compressor_attack_body_db < -1.5)
            .count();
        if damaged as f64 / e.len() as f64 >= 0.7 && quantile(recover, 0.5).unwrap_or(0.) > 3. {
            delta[i] = 2.;
            "Repeated attack/body loss and incomplete recovery: test +2 dB threshold relief once; measure makeup independently.".into()
        } else {
            "Retain compression: repeated attack/body loss with incomplete recovery is not established.".into()
        }
    });
    ProcessingPlan{threshold_relief_db:delta,reasons,tone_abstention:"No numerical drum timbre intent or isolated bleed evidence supplied. Report static-EQ sensitivity; do not infer a capture defect or select EQ solely from overlap.".into()}
}
#[derive(Serialize)]
pub struct ToneProbe {
    pub name: String,
    pub hypothesis: String,
    pub settings: Session,
}
/// Two bounded sensitivity probes of existing positive EQ. Neither presumes
/// that overlap or a bright spectrum is a fault, and neither adds an EQ band.
pub fn tone_probes(s: &Session, p: &Policy) -> Vec<ToneProbe> {
    let mut probes = Vec::new();
    for (d, i) in [p.kick, p.snare].iter().enumerate() {
        let Some((band, _)) = s.channels[*i].eq.iter().enumerate().find(|(_, b)| {
            if d == 0 {
                b.kind == super::config::EqKind::Bell
                    && (70. ..=130.).contains(&b.hz)
                    && b.db >= 1.5
            } else {
                b.kind == super::config::EqKind::HighShelf
                    && (4000. ..=8000.).contains(&b.hz)
                    && b.db >= 2.
            }
        }) else {
            continue;
        };
        let mut settings = s.clone();
        settings.channels[*i].eq[band].db -= if d == 0 { 1.5 } else { 2. };
        probes.push(ToneProbe{name:if d==0{"kick-low-boost-relief"}else{"snare-high-shelf-relief"}.into(),
            hypothesis:if d==0{"Measure low-band support and compressor sensitivity; overlap alone cannot select this cut."}else{"Measure body, brightness and residual-bleed tradeoff; a shelf boost alone cannot establish excessive brightness."}.into(),settings});
    }
    probes
}

/// Relief must reduce measured event compression in both splits, with measured
/// makeup retaining the prior hit level within 0.5 dB. No held-out retuning.
pub fn relief_failures(
    before: &Measurement,
    after: &Measurement,
    d: &Diagnosis,
    changed: [bool; 2],
) -> Vec<String> {
    let mut failures = Vec::new();
    for (i, changed) in changed.iter().enumerate() {
        if !changed {
            continue;
        }
        for held in [false, true] {
            let events = d.events[i]
                .iter()
                .filter(|e| e.held_out == held)
                .collect::<Vec<_>>();
            let gr = quantile(
                events
                    .iter()
                    .map(|e| {
                        mean_gr(&before.frames[e.frame..e.frame + 8], i)
                            - mean_gr(&after.frames[e.frame..e.frame + 8], i)
                    })
                    .collect(),
                0.5,
            );
            let level = quantile(
                events
                    .iter()
                    .map(|e| {
                        average(&after.frames[e.frame..e.frame + 8], i * 5 + 2)
                            - average(&before.frames[e.frame..e.frame + 8], i * 5 + 2)
                    })
                    .collect(),
                0.5,
            );
            if events.len() < 8
                || gr.is_none_or(|x| x < 0.25)
                || level.is_none_or(|x| x.abs() > 0.5)
            {
                failures.push(format!("drum {i} held_out={held}: insufficient relief or residual event-level change exceeds 0.5 dB"));
            }
        }
    }
    failures
}
#[derive(Serialize)]
pub struct Outcome {
    pub accepted: bool,
    pub failures: Vec<String>,
    pub max_added_gr_db: [f64; 2],
    pub max_processing_output_rise_db: [f64; 2],
    pub max_attack_body_loss_db: [f64; 2],
    pub max_mix_peak_rise_db: f64,
    pub event_output_change_db: [[Option<f64>; 2]; 2],
    pub intended_emphasis_db: [f64; 2],
    pub listener_preference: Option<String>,
}
/// Fixed raw-event masks and all-frame guards. Artistic gain is accounted for
/// explicitly, rather than misclassified as extra compressor makeup or masking.
pub fn validate_change(
    before: &Measurement,
    after: &Measurement,
    d: &Diagnosis,
    emphasis: [f64; 2],
) -> Outcome {
    let mut failures = vec![];
    let mut gr = [0_f64; 2];
    let mut rise = [0_f64; 2];
    let mut fall = [0_f64; 2];
    let mut loss = [0_f64; 2];
    let mut quiet = [0_f64; 2];
    let mut event_delta = [[None; 2]; 2];
    if before.rate != after.rate
        || before.frames.len() != after.frames.len()
        || before
            .frames
            .iter()
            .zip(&after.frames)
            .any(|(a, b)| a.seconds != b.seconds || a.duration != b.duration)
    {
        failures.push("timeline mismatch".into());
    }
    for i in 0..2 {
        if before
            .spectra
            .iter()
            .filter(|w| w.pcm_contact_fraction[i] >= super::expert::PCM_CONTACT_FRACTION)
            .count()
            >= super::expert::PCM_CONTACT_WINDOWS
        {
            failures.push(format!(
                "drum {i}: repeated PCM contact; inspect recorded signal path"
            ));
        }
    }
    for i in 0..2 {
        for (a, b) in before.frames.iter().zip(&after.frames) {
            gr[i] = gr[i].max(b.max_gr[i] - a.max_gr[i]);
            if a.rms[i * 5] > -65. {
                let stage = i * 5 + 2;
                let crest_loss = (a.peak[stage] - a.rms[stage]) - (b.peak[stage] - b.rms[stage]);
                loss[i] = loss[i].max(crest_loss);
            }
            if a.rms[i * 5] > -90. {
                let r = b.rms[i * 5 + 4] - a.rms[i * 5 + 4] - emphasis[i];
                rise[i] = rise[i].max(r);
                if a.rms[i * 5] > -65. {
                    fall[i] = fall[i].max(-r);
                }
                if a.rms[i * 5] < -65. {
                    quiet[i] = quiet[i].max(r);
                }
            }
        }
        for (h, held) in [false, true].iter().enumerate() {
            let e = d.events[i]
                .iter()
                .filter(|e| e.held_out == *held)
                .collect::<Vec<_>>();
            if e.len() < 8 {
                failures.push(format!("drum {i} split {h}: insufficient events"));
                continue;
            }
            let mut changes = vec![];
            for e in e {
                let a = e.frame;
                if a + 8 > after.frames.len() {
                    failures.push("event outside candidate timeline".into());
                    continue;
                }
                let contrast = average(&after.frames[a..a + 3], i * 5 + 2)
                    - average(&after.frames[a + 3..a + 8], i * 5 + 2);
                if !e.ambiguous_onset && e.next_seconds.is_none_or(|t| t >= 0.08) {
                    loss[i] = loss[i].max(e.attack_body_db - contrast);
                }
                changes.push(
                    average(&after.frames[a..a + 3], i * 5 + 4)
                        - average(&before.frames[a..a + 3], i * 5 + 4),
                );
            }
            event_delta[i][h] = quantile(changes, 0.5);
        }
        if fall[i] > 3.0001 {
            failures.push(format!(
                "drum {i}: active output loss >3 dB beyond artistic offset"
            ));
        }
        if gr[i] > 1.0001 {
            failures.push(format!("drum {i}: added reduction >1 dB"));
        }
        if rise[i] > 3.0001 {
            failures.push(format!("drum {i}: processing output rise >3 dB"));
        }
        if quiet[i] > 1.0001 {
            failures.push(format!(
                "drum {i}: quiet output rise >1 dB beyond artistic offset"
            ));
        }
        if loss[i] > 1.5001 {
            failures.push(format!("drum {i}: event attack/body loss >1.5 dB"));
        }
    }
    let peak = |m: &Measurement| m.frames.iter().map(|f| f.peak[14]).fold(-240_f64, f64::max);
    let peak_rise = peak(after) - peak(before);
    if peak_rise > 3.0001 {
        failures.push("full mix peak rise >3 dB before independent export".into());
    }
    Outcome {
        accepted: failures.is_empty(),
        failures,
        max_added_gr_db: gr,
        max_processing_output_rise_db: rise,
        max_attack_body_loss_db: loss,
        max_mix_peak_rise_db: peak_rise,
        event_output_change_db: event_delta,
        intended_emphasis_db: emphasis,
        listener_preference: None,
    }
}
/// One proposal from training, then fixed held-out validation. Never searches
/// again following a veto. Rendering is an explicit separate command.
pub fn correct(s: Session, root: &Path, out: &Path, p: Policy, start: f64, end: f64) -> Result<()> {
    p.validate(&s)?;
    if out.exists() {
        return Err("output already exists".into());
    }
    let baseline = measure(&s, root, &p, start, end)?;
    let diagnosis = diagnose(&baseline);
    let plan = processing_plan(&diagnosis);
    std::fs::create_dir(out)?;
    write_json(&out.join("before-settings.json"), &s)?;
    write_json(&out.join("policy.json"), &p)?;
    write_json(&out.join("baseline.json"), &baseline)?;
    write_json(&out.join("diagnosis.json"), &diagnosis)?;
    write_json(&out.join("processing-plan.json"), &plan)?;
    for probe in tone_probes(&s, &p) {
        let dir = out.join(&probe.name);
        std::fs::create_dir(&dir)?;
        let measured = measure(&probe.settings, root, &p, start, end)?;
        write_json(&dir.join("proposal.json"), &probe)?;
        write_json(&dir.join("measurement.json"), &measured)?;
        write_json(&dir.join("diagnosis.json"), &diagnose(&measured))?;
        write_json(
            &dir.join("technical-outcome.json"),
            &validate_change(&baseline, &measured, &diagnosis, [0.; 2]),
        )?;
        // A sensitivity experiment is retained for review, never automatically
        // chosen as a preferred tone when the requested tone is unspecified.
    }
    let mut processed = s.clone();
    for (d, i) in [p.kick, p.snare].iter().enumerate() {
        processed.channels[*i].compressor.threshold_db += plan.threshold_relief_db[d];
    }
    let mut compensation = [0.; 2];
    let processing_changed = plan.threshold_relief_db.iter().any(|x| *x != 0.);
    if processing_changed {
        let trial = measure(&processed, root, &p, start, end)?;
        // Total compensation is the measured change from the existing output,
        // not the existing makeup plus a second estimate of all reduction.
        for (d, i) in [p.kick, p.snare].iter().enumerate() {
            if plan.threshold_relief_db[d] == 0. {
                continue;
            }
            let shifts = diagnosis.events[d]
                .iter()
                .filter(|e| !e.held_out)
                .map(|e| {
                    let a = e.frame;
                    average(&baseline.frames[a..a + 8], d * 5 + 2)
                        - average(&trial.frames[a..a + 8], d * 5 + 2)
                })
                .collect();
            compensation[d] = quantile(shifts, 0.5).unwrap_or(0.).clamp(-2., 2.);
            processed.channels[*i].compressor.makeup_db += compensation[d];
        }
        write_json(&out.join("uncompensated-processing.json"), &trial)?;
    }
    let processing_measurement = if processing_changed {
        Some(measure(&processed, root, &p, start, end)?)
    } else {
        None
    };
    let processing = processing_measurement.as_ref().unwrap_or(&baseline);
    let mut processing_outcome = validate_change(&baseline, processing, &diagnosis, [0.; 2]);
    processing_outcome.failures.extend(relief_failures(
        &baseline,
        processing,
        &diagnosis,
        plan.threshold_relief_db.map(|x| x != 0.),
    ));
    processing_outcome.accepted = processing_outcome.failures.is_empty();
    write_json(&out.join("processing-settings.json"), &processed)?;
    write_json(&out.join("processing.json"), &processing)?;
    write_json(&out.join("processing-outcome.json"), &processing_outcome)?;
    let delta = balance::rhythmic_emphasis_delta(p.current_emphasis_db, p.desired_emphasis_db)?;
    let mut combined = processed.clone();
    let mut fader = s.clone();
    for (d, i) in [p.kick, p.snare].iter().enumerate() {
        combined.channels[*i].fader_db += delta.unwrap_or([0.; 2])[d];
        fader.channels[*i].fader_db += delta.unwrap_or([0.; 2])[d];
    }
    let fader_measurement = measure(&fader, root, &p, start, end)?;
    let fader_outcome = validate_change(
        &baseline,
        &fader_measurement,
        &diagnosis,
        delta.unwrap_or([0.; 2]),
    );
    write_json(&out.join("fader-only-settings.json"), &fader)?;
    write_json(&out.join("fader-only.json"), &fader_measurement)?;
    write_json(&out.join("fader-outcome.json"), &fader_outcome)?;
    let combined_measurement = if processing_changed {
        Some(measure(&combined, root, &p, start, end)?)
    } else {
        None
    };
    let candidate = combined_measurement.as_ref().unwrap_or(&fader_measurement);
    let outcome = validate_change(&baseline, candidate, &diagnosis, delta.unwrap_or([0.; 2]));
    // Unknown perceptual balance withholds fader movement, not independent
    // processing. All three technical checks still apply to the unchanged faders.
    let accepted = processing_outcome.accepted && fader_outcome.accepted && outcome.accepted;
    write_json(&out.join("candidate-settings.json"), &combined)?;
    write_json(&out.join("candidate.json"), &candidate)?;
    write_json(&out.join("outcome.json"), &outcome)?;
    write_json(
        &out.join("decision.json"),
        &serde_json::json!({"accepted":accepted,"makeup_delta_db":compensation,"emphasis_delta_db":delta,"neutral_basis":p.neutral_basis,"unknown_neutral_basis_abstention":delta.is_none(),"processing_changed":processing_changed,"tone_outcome":"No established tone target; no automatic EQ selected","listener_preference":null}),
    )?;
    write_json(
        &out.join("settings.json"),
        if accepted { &combined } else { &s },
    )
}

/// Reanalyze a frozen pilot candidate without running proposal selection again.
pub fn verify(
    s: Session,
    candidate: Session,
    root: &Path,
    out: &Path,
    p: Policy,
    start: f64,
    end: f64,
) -> Result<()> {
    p.validate(&s)?;
    p.validate(&candidate)?;
    if out.exists() {
        return Err("output already exists".into());
    }
    let mut invariant = candidate.clone();
    for i in [p.kick, p.snare] {
        invariant.channels[i] = s.channels[i].clone();
    }
    if serde_json::to_value(&invariant)? != serde_json::to_value(&s)? {
        return Err("drum candidate changed protected settings".into());
    }
    for i in [p.kick, p.snare] {
        let (a, b) = (&s.channels[i], &candidate.channels[i]);
        if a.file != b.file
            || a.pan != b.pan
            || a.hpf_hz != b.hpf_hz
            || a.group != b.group
            || std::mem::discriminant(&a.role) != std::mem::discriminant(&b.role)
        {
            return Err("drum candidate changed routing or required HPF".into());
        }
    }
    let requested = balance::rhythmic_emphasis_delta(p.current_emphasis_db, p.desired_emphasis_db)?;
    for (d, i) in [p.kick, p.snare].iter().enumerate() {
        if (candidate.channels[*i].fader_db
            - s.channels[*i].fader_db
            - requested.unwrap_or([0.; 2])[d])
            .abs()
            > 1e-8
        {
            return Err("candidate fader change does not match explicit rhythmic intent".into());
        }
    }
    let before = measure(&s, root, &p, start, end)?;
    let after = measure(&candidate, root, &p, start, end)?;
    let diagnosis = diagnose(&before);
    let delta = balance::rhythmic_emphasis_delta(p.current_emphasis_db, p.desired_emphasis_db)?;
    let mut outcome = validate_change(&before, &after, &diagnosis, delta.unwrap_or([0.; 2]));
    let changed = [p.kick, p.snare].map(|i| {
        s.channels[i].compressor.threshold_db != candidate.channels[i].compressor.threshold_db
    });
    outcome
        .failures
        .extend(relief_failures(&before, &after, &diagnosis, changed));
    outcome.accepted = outcome.failures.is_empty();
    // The fader contract above already requires zero movement for unknown intent.
    // A processing-only check does not require a preferred musical balance.
    std::fs::create_dir(out)?;
    write_json(&out.join("before-settings.json"), &s)?;
    write_json(&out.join("candidate-settings.json"), &candidate)?;
    write_json(&out.join("policy.json"), &p)?;
    write_json(&out.join("baseline.json"), &before)?;
    write_json(&out.join("baseline-diagnosis.json"), &diagnosis)?;
    write_json(&out.join("measurement.json"), &after)?;
    write_json(&out.join("diagnosis.json"), &diagnose(&after))?;
    write_json(&out.join("outcome.json"), &outcome)?;
    write_json(
        &out.join("balance-intent.json"),
        &serde_json::json!({"emphasis_delta_db":delta,"unknown_neutral_basis_abstention":delta.is_none(),"faders_unchanged":delta.is_none_or(|d| d == [0.;2]),"listener_preference":null}),
    )?;
    write_json(
        &out.join("settings.json"),
        if outcome.accepted { &candidate } else { &s },
    )
}

/// Re-run event diagnosis from saved synchronized frames after detector fixes.
/// Source/DSP measurements remain immutable; this does not propose new settings.
pub fn redetect(input: &Path, out: &Path) -> Result<()> {
    let value: serde_json::Value = serde_json::from_reader(std::fs::File::open(input)?)?;
    if value.get("stages") != Some(&serde_json::to_value(STAGES)?) {
        return Err("incompatible drum measurement stages".into());
    }
    let frames: Vec<Frame> =
        serde_json::from_value(value.get("frames").ok_or("missing frames")?.clone())?;
    if frames.len() > 60001
        || frames.iter().any(|f| {
            !f.seconds.is_finite()
                || !f.duration.is_finite()
                || f.duration <= 0.
                || f.duration > 0.011
        })
        || frames
            .windows(2)
            .any(|f| (f[0].seconds + f[0].duration - f[1].seconds).abs() > 1e-8)
    {
        return Err("invalid synchronized frame timeline".into());
    }
    let m = Measurement {
        stages: STAGES,
        frames,
        spectra: vec![],
        pcm_contacts: [0; 2],
        rate: 0,
    };
    write_json(out, &diagnose(&m))
}
