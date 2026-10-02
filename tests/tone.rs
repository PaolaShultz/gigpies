use gigpies::automix::{
    config::{self, Role},
    dsp::{Biquad, gain},
    tone::{self, Instrument, Intent, Policy},
    unity,
};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static ID: AtomicU64 = AtomicU64::new(0);
struct Fixture {
    root: PathBuf,
    session: config::Session,
    policy: Policy,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
fn fixture(stereo: bool, scale: f64, room: f64, pauses: bool) -> Fixture {
    let root = std::env::temp_dir().join(format!(
        "gigpies-tone-{}-{}",
        std::process::id(),
        ID.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&root).unwrap();
    let mut s = config::example();
    let base = s.channels[0].clone();
    s.channels = (0..2)
        .map(|i| {
            let mut c = base.clone();
            c.file = format!("{i}.wav").into();
            c.group = format!("g{i}");
            c.role = Role::RhythmGuitar;
            c.eq.clear();
            c.compressor.ratio = 1.;
            c.compressor.makeup_db = 0.;
            c
        })
        .collect();
    s.sample_rate = 8000;
    unity::prepare(&mut s).unwrap();
    let p = Policy {
        instruments: vec![Instrument {
            name: "one known guitar".into(),
            primary: 0,
            primary_file: "0.wav".into(),
            secondary: vec![1],
            secondary_files: vec!["1.wav".into()],
            intent: Intent::Balanced,
            capture: None,
            profile: None,
        }],
        section_seconds: 2.,
        minimum_active_seconds: 1.,
        ..Default::default()
    };
    for i in 0..2 {
        let mut w = hound::WavWriter::create(
            root.join(format!("{i}.wav")),
            hound::WavSpec {
                channels: if stereo { 2 } else { 1 },
                sample_rate: 8000,
                bits_per_sample: 24,
                sample_format: hound::SampleFormat::Int,
            },
        )
        .unwrap();
        for k in 0..8000 * 14 {
            let t = k as f64 / 8000.;
            let env = if pauses && ((t / 2.) as u32 % 2 == 1) {
                0.0001
            } else {
                1.
            };
            let x = scale
                * (if i == 0 { env } else { room })
                * (0.04 * (std::f64::consts::TAU * 220. * t).sin()
                    + 0.1 * (std::f64::consts::TAU * 1800. * t).sin());
            w.write_sample((x * 8388607.) as i32).unwrap();
            if stereo {
                w.write_sample((-x * 0.5 * 8388607.) as i32).unwrap();
            }
        }
        w.finalize().unwrap();
    }
    Fixture {
        root,
        session: s,
        policy: p,
    }
}
fn propose(f: &Fixture) -> (Vec<tone::Frame>, tone::Proposal) {
    f.policy.validate(&f.session).unwrap();
    let m = tone::measure(&f.session, &f.root, &f.policy.instruments[0]).unwrap();
    let q = tone::propose(
        &m,
        f.session.sample_rate,
        &f.policy.instruments[0],
        &f.policy,
    );
    (m, q)
}
#[test]
fn measured_thin_source_is_corrected_but_explicit_thin_intent_is_respected() {
    let mut f = fixture(false, 1., 0.1, false);
    let (m, q) = propose(&f);
    assert!(!q.proposed_eq.is_empty());
    assert_eq!(q.evaluated, 546);
    let mut s = f.session.clone();
    s.channels[0].eq.extend(q.proposed_eq.clone());
    let after = tone::measure(&s, &f.root, &f.policy.instruments[0]).unwrap();
    let v = tone::validate_candidate(&m, &after, &q, s.sample_rate, &f.policy);
    assert!(v.accepted, "{}", v.reason);
    assert!(v.after[1].median_body_presence_db > q.before[1].median_body_presence_db + 1.);
    f.policy.instruments[0].intent = Intent::Thin;
    assert!(propose(&f).1.proposed_eq.is_empty());
}
#[test]
fn silence_insufficient_evidence_and_dominant_secondary_abstain() {
    for (scale, room) in [(0., 0.), (1., 2.)] {
        let f = fixture(false, scale, room, false);
        assert!(propose(&f).1.proposed_eq.is_empty());
    }
    let f = fixture(false, 1., 0.1, false);
    let (m, _) = propose(&f);
    assert!(
        tone::propose(&m[..1], 8000, &f.policy.instruments[0], &f.policy)
            .proposed_eq
            .is_empty()
    );
}
#[test]
fn quiet_playing_is_eligible_and_bleed_only_pauses_do_not_drive_eq() {
    let f = fixture(false, 0.02, 0.1, false);
    assert!(!propose(&f).1.proposed_eq.is_empty());
    let f = fixture(false, 1., 0.1, true);
    let (m, q) = propose(&f);
    assert!(
        m.iter()
            .zip(&q.eligible)
            .filter(|(f, _)| f.seconds > 2.1 && f.seconds < 3.)
            .all(|(_, eligible)| !*eligible)
    );
}
#[test]
fn actual_stereo_linking_and_secondary_routing_are_preserved() {
    let f = fixture(true, 1., 0.1, false);
    let (m, q) = propose(&f);
    assert!(!q.proposed_eq.is_empty());
    let mut s = f.session.clone();
    s.channels[0].eq.extend(q.proposed_eq.clone());
    let a = tone::measure(&s, &f.root, &f.policy.instruments[0]).unwrap();
    assert!(tone::validate_candidate(&m, &a, &q, 8000, &f.policy).accepted);
    tone::run(
        f.session.clone(),
        &f.root,
        &f.root.join("result"),
        f.policy.clone(),
        false,
    )
    .unwrap();
    let frozen: config::Session =
        serde_json::from_reader(std::fs::File::open(f.root.join("result/settings.json")).unwrap())
            .unwrap();
    for i in 0..2 {
        assert_eq!(frozen.channels[i].pan, f.session.channels[i].pan);
        assert_eq!(frozen.channels[i].fader_db, f.session.channels[i].fader_db);
    }
    assert_eq!(
        serde_json::to_value(&frozen.channels[1]).unwrap(),
        serde_json::to_value(&f.session.channels[1]).unwrap()
    );
    assert!(
        tone::run(
            f.session.clone(),
            &f.root,
            &f.root.join("result"),
            f.policy.clone(),
            false
        )
        .is_err()
    );
}
#[test]
fn transient_compression_and_coherent_sum_regressions_are_vetoed() {
    let f = fixture(false, 1., 0.1, false);
    let (m, q) = propose(&f);
    let mut s = f.session.clone();
    s.channels[0].eq.extend(q.proposed_eq.clone());
    let a = tone::measure(&s, &f.root, &f.policy.instruments[0]).unwrap();
    for case in 0..3 {
        let mut bad = a.clone();
        for v in &mut bad {
            match case {
                0 => {
                    v.crest_db -= 6.;
                }
                1 => {
                    v.max_reduction_db += 6.;
                }
                _ => {
                    v.group_body_presence_db -= 20.;
                }
            }
        }
        assert!(!tone::validate_candidate(&m, &bad, &q, 8000, &f.policy).accepted);
    }
}
#[test]
fn held_out_failure_vetoes_without_retuning() {
    let f = fixture(false, 1., 0.1, false);
    let (m, q) = propose(&f);
    let mut s = f.session.clone();
    s.channels[0].eq.extend(q.proposed_eq.clone());
    let mut a = tone::measure(&s, &f.root, &f.policy.instruments[0]).unwrap();
    for (x, y) in a.iter_mut().zip(&m) {
        if (x.seconds / 2.) as u32 % 2 == 1 {
            x.body_presence_db = y.body_presence_db;
        }
    }
    assert!(!tone::validate_candidate(&m, &a, &q, 8000, &f.policy).accepted);
    let again = tone::propose(&m, 8000, &f.policy.instruments[0], &f.policy);
    assert_eq!(
        serde_json::to_value(&q.proposed_eq).unwrap(),
        serde_json::to_value(&again.proposed_eq).unwrap()
    );
}
#[test]
fn identity_policy_and_nonfinite_validation() {
    let mut f = fixture(false, 1., 0.1, false);
    f.policy.instruments[0].secondary.push(0);
    assert!(f.policy.validate(&f.session).is_err());
    f.policy.instruments[0].secondary = vec![1];
    f.policy.instruments[0].primary_file = "wrong.wav".into();
    assert!(f.policy.validate(&f.session).is_err());
    f.policy.instruments[0].primary_file = "0.wav".into();
    f.policy.minimum_consistency = f64::NAN;
    assert!(f.policy.validate(&f.session).is_err());
}
#[test]
fn filter_response_predicts_measured_sine_gain() {
    let e = config::EqBand {
        kind: config::EqKind::Bell,
        hz: 220.,
        q: 0.7,
        db: 6.,
    };
    let mut f = Biquad::equalizer(&e, 8000);
    let mut before = 0.;
    let mut after = 0.;
    for k in 0..16000 {
        let x = (std::f64::consts::TAU * 220. * k as f64 / 8000.).sin();
        let y = f.tick(x);
        if k > 8000 {
            before += x * x;
            after += y * y;
        }
    }
    assert!((after / before - f.power_response(220., 8000)).abs() < 0.002);
    assert!((f.power_response(220., 8000) - gain(6.).powi(2)).abs() < 1e-9);
}
#[test]
fn correlated_microphones_use_actual_sum_not_independent_power() {
    let mut f = fixture(false, 1., 0.4, false);
    f.session.channels[0].pan = 0.;
    f.session.channels[1].pan = 0.;
    let m = tone::measure(&f.session, &f.root, &f.policy.instruments[0]).unwrap();
    // Identical waveforms at different gains retain the same tone in the coherent sum.
    for w in m {
        assert!((w.body_presence_db - w.group_body_presence_db).abs() < 0.01);
    }
}

#[test]
fn sparse_musical_decay_does_not_define_broad_tone_and_boundaries_do_not_leak() {
    let f = fixture(false, 1., 0.1, false);
    // Replace the primary with an ordinary broad guitar followed by a decaying 165 Hz note.
    let path = f.root.join("0.wav");
    let mut reader = hound::WavReader::open(&path).unwrap();
    let spec = reader.spec();
    let mut samples = reader
        .samples::<i32>()
        .map(Result::unwrap)
        .collect::<Vec<_>>();
    drop(reader);
    for (k, x) in samples.iter_mut().enumerate().skip(8000 * 8) {
        let t = k as f64 / 8000. - 8.;
        *x = (0.04 * (-t / 4.).exp() * (std::f64::consts::TAU * 165. * t).sin() * 8388607.) as i32;
    }
    let mut writer = hound::WavWriter::create(path, spec).unwrap();
    for x in samples {
        writer.write_sample(x).unwrap();
    }
    writer.finalize().unwrap();
    let (frames, q) = propose(&f);
    assert!(
        frames
            .iter()
            .zip(&q.eligible)
            .filter(|(f, _)| f.seconds > 9.)
            .all(|(_, yes)| !*yes)
    );
    assert!(
        frames
            .iter()
            .zip(&q.eligible)
            .filter(|(f, _)| (f.seconds / 2.) as u64 != ((f.seconds + 8191. / 8000.) / 2.) as u64)
            .all(|(_, yes)| !*yes)
    );
}

#[test]
fn last_two_eq_slots_can_be_measured_and_filled() {
    let mut f = fixture(false, 1., 0.1, false);
    f.session.channels[0].eq = vec![
        config::EqBand {
            kind: config::EqKind::Bell,
            hz: 400.,
            q: 1.,
            db: 0.
        };
        6
    ];
    tone::run(
        f.session.clone(),
        &f.root,
        &f.root.join("last-slots"),
        f.policy.clone(),
        false,
    )
    .unwrap();
    let s: config::Session = serde_json::from_reader(
        std::fs::File::open(f.root.join("last-slots/settings.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(s.channels[0].eq.len(), 8);
}

#[test]
fn training_sections_constrain_a_misleading_average() {
    let mut f = fixture(false, 1., 0.1, false);
    f.policy.section_seconds = 4.;
    let (mut frames, unconstrained) = propose(&f);
    let more = frames
        .iter()
        .cloned()
        .map(|mut w| {
            w.seconds += 16.;
            w
        })
        .collect::<Vec<_>>();
    frames.extend(more);
    // One training section has strong body, still with valid energy in both bands.
    // Transform measured spectral features to isolate search behavior from the DSP guard.
    for frame in frames
        .iter_mut()
        .filter(|w| w.seconds >= 8. && w.seconds < 12.)
    {
        for (k, x) in frame.spectrum.iter_mut().enumerate() {
            if (100.0..400.0).contains(&(k as f64 * 8000. / 8192.)) {
                *x *= 100.;
            }
        }
        frame.body_presence_db += 20.;
        frame.body_power_fraction = 0.9;
        frame.presence_power_fraction = 0.09;
    }
    f.policy.minimum_consistency = 0.6;
    let bounded = tone::propose(&frames, 8000, &f.policy.instruments[0], &f.policy);
    assert!(bounded.training_sections_guarded >= 2);
    let effort = |q: &tone::Proposal| q.proposed_eq.iter().map(|e| e.db.abs()).sum::<f64>();
    assert!(effort(&bounded) < effort(&unconstrained));
}
