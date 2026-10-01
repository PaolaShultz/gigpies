use gigpies::automix::{
    balance::{self, ChannelWindow, Measurement, Policy, Relationship, Window},
    config::{self, Role},
    unity,
};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
fn session(roles: &[Role]) -> config::Session {
    let mut s = config::example();
    let base = s.channels[0].clone();
    s.channels = roles
        .iter()
        .enumerate()
        .map(|(i, &r)| {
            let mut c = base.clone();
            c.role = r;
            c.group = format!("g{i}");
            c.file = format!("{i}.wav").into();
            c.eq.clear();
            c.compressor.ratio = 1.;
            c.compressor.makeup_db = 0.;
            c
        })
        .collect();
    s.sample_rate = 8000;
    unity::prepare(&mut s).unwrap();
    s
}
fn policy(
    s: &config::Session,
    a: usize,
    b: usize,
    event: bool,
    kit: bool,
    range: [f64; 2],
) -> Policy {
    let mut p = Policy::for_session(s);
    p.section_seconds = 2.;
    p.minimum_windows = 2;
    p.relationships = vec![Relationship {
        name: "known relationship".into(),
        numerator: vec![a],
        denominator: vec![b],
        activity_source: if matches!(s.channels[a].role, Role::DrumRoom) {
            b
        } else {
            a
        },
        band: 0,
        event_only: event,
        kit_stage: kit,
        range_db: range,
    }];
    p
}
fn features(levels: &[f64], active: bool) -> Measurement {
    let n = levels.len() + 1;
    let windows = (0..400)
        .map(|k| {
            let mut cov = vec![0.; 5 * n * n];
            for band in 0..5 {
                for (i, &db) in levels.iter().enumerate() {
                    cov[(band * n + i) * n + i] = 10_f64.powf(db / 10.);
                }
            }
            Window {
                start_frame: k * 20,
                frames: 20,
                channels: levels
                    .iter()
                    .map(|&x| ChannelWindow {
                        pre_compressor_dbfs: x,
                        post_compressor_dbfs: x,
                        input_dbfs: x,
                        active,
                        confidence: if active { 0.9 } else { 0. },
                        onset: active && k % 10 == 0,
                        ..Default::default()
                    })
                    .collect(),
                covariance: cov,
                master_mean_reduction_db: 0.,
                master_max_reduction_db: 0.,
                master_dbfs: -240.,
            }
        })
        .collect();
    Measurement {
        windows,
        signals: n,
        rate: 1000,
    }
}
#[test]
fn buried_hits_move_up_relative_to_competitor_with_fixed_budget_and_bounds() {
    let s = session(&[Role::Kick, Role::RhythmGuitar]);
    let p = policy(&s, 0, 1, true, false, [-6., 8.]);
    let m = features(&[-32., -20.], true);
    let r = balance::optimize(&m, &s, &p).unwrap();
    assert!(r.accepted);
    assert!(r.deltas_db[0] - r.deltas_db[1] > 4.);
    assert!(r.deltas_db.iter().any(|&x| x > 0.));
    assert!(r.deltas_db.iter().any(|&x| x < 0.));
    assert!(r.deltas_db.iter().all(|x| x.abs() <= p.bound_db));
    assert!(r.evaluations <= 2 * p.evaluations_per_stage);
    assert_eq!(
        r.deltas_db,
        balance::optimize(&m, &s, &p).unwrap().deltas_db
    );
}
#[test]
fn excessive_room_is_cut_and_vocal_competition_moves_towards_vocal() {
    let s = session(&[Role::Kick, Role::DrumRoom]);
    let p = policy(&s, 1, 0, true, true, [-24., -9.]);
    let r = balance::optimize(&features(&[-20., -20.], true), &s, &p).unwrap();
    assert!(r.accepted);
    assert!(r.deltas_db[1] < 0.);
    assert!(r.deltas_db[1] - r.deltas_db[0] < -6.);
    let s = session(&[Role::LeadVocal, Role::RhythmGuitar]);
    let p = policy(&s, 0, 1, false, false, [0., 12.]);
    let r = balance::optimize(&features(&[-28., -20.], true), &s, &p).unwrap();
    assert!(r.accepted);
    assert!(r.deltas_db[0] > r.deltas_db[1]);
}
#[test]
fn no_change_silence_and_insufficient_confidence_do_not_move_faders() {
    let s = session(&[Role::LeadVocal, Role::RhythmGuitar]);
    let p = policy(&s, 0, 1, false, false, [0., 12.]);
    for m in [
        features(&[-20., -24.], true),
        features(&[-240., -240.], false),
    ] {
        let r = balance::optimize(&m, &s, &p).unwrap();
        assert!(!r.accepted);
        assert_eq!(r.deltas_db, vec![0., 0.]);
    }
    let mut m = features(&[-40., -20.], true);
    for w in &mut m.windows {
        w.channels[0].confidence = 0.3;
    }
    assert_eq!(
        balance::optimize(&m, &s, &p).unwrap().deltas_db,
        vec![0., 0.]
    );
}
#[test]
fn common_gain_cannot_improve_relationships_and_holdout_can_veto() {
    let s = session(&[Role::LeadVocal, Role::RhythmGuitar]);
    let p = policy(&s, 0, 1, false, false, [0., 4.]);
    let mut m = features(&[-30., -20.], true);
    let a = balance::evaluate(&m, &p, &[0., 0.], false);
    let b = balance::evaluate(&m, &p, &[-9., -9.], false);
    assert!((a[0].median_db.unwrap() - b[0].median_db.unwrap()).abs() < 1e-9);
    for w in &mut m.windows {
        if (w.start_frame / 2000) % 2 == 1 {
            w.covariance[0] = 0.1;
        }
    }
    let r = balance::optimize(&m, &s, &p).unwrap();
    assert!(!r.accepted);
    assert_eq!(r.deltas_db, vec![0., 0.]);
}
#[test]
fn correlated_microphone_group_energy_keeps_cross_terms() {
    let mut m = features(&[-20., -20.], true);
    let n = m.signals;
    for w in &mut m.windows {
        w.covariance[1] = -0.01;
        w.covariance[n] = -0.01;
    }
    assert!(m.energy(&m.windows[0], &[0, 1], 0, &[1., 1., 1.]) < 1e-20);
    assert!((m.energy(&m.windows[0], &[0, 1], 0, &[1., 0.5, 1.]) - 0.0025).abs() < 1e-12);
}
#[test]
fn rejects_invalid_policy_and_nonunity_baselines() {
    let mut s = session(&[Role::Kick, Role::DrumRoom]);
    let mut p = policy(&s, 0, 1, true, true, [-3., 9.]);
    p.step_db = f64::NAN;
    assert!(p.validate(2).is_err());
    p.step_db = 0.5;
    p.relationships[0].denominator = vec![0];
    assert!(p.validate(2).is_err());
    s.groups[0].trim_db = 1.;
    assert!(balance::validate_baseline(&s).is_err());
}
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "gigpies-balance-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&p).unwrap();
        Self(p)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
#[test]
fn wav_analysis_rejects_coincident_bleed_keeps_quiet_playing_and_stereo_timing() {
    let t = Scratch::new();
    let s = session(&[Role::Snare, Role::Tom, Role::RhythmGuitar]);
    let p = Policy::for_session(&s);
    for channel in 0..3 {
        let mut w = hound::WavWriter::create(
            t.0.join(format!("{channel}.wav")),
            hound::WavSpec {
                channels: if channel == 2 { 2 } else { 1 },
                sample_rate: 8000,
                bits_per_sample: 32,
                sample_format: hound::SampleFormat::Float,
            },
        )
        .unwrap();
        for frame in 0..32000 {
            let phase = (frame % 4000) as f64 / 8000.;
            let hit = if (0.1..0.25).contains(&phase) {
                0.5 * (-(phase - 0.1) * 30.).exp()
                    * (std::f64::consts::TAU * 220. * frame as f64 / 8000.).sin()
            } else {
                0.
            };
            let x = match channel {
                0 => hit,
                1 => hit * 0.05,
                _ => 0.0005 * (std::f64::consts::TAU * 400. * frame as f64 / 8000.).sin(),
            };
            w.write_sample(x as f32).unwrap();
            if channel == 2 {
                w.write_sample((-x * 0.5) as f32).unwrap();
            }
        }
        w.finalize().unwrap();
    }
    let m = balance::measure(&s, &t.0, &p).unwrap();
    balance::save_measurement(&m, &s, &p, &t.0.join("evidence")).unwrap();
    for file in [
        "sources.json",
        "events.json",
        "microphone-relationships.json",
        "targets.json",
    ] {
        let _: serde_json::Value =
            serde_json::from_reader(std::fs::File::open(t.0.join("evidence").join(file)).unwrap())
                .unwrap();
    }
    assert!(balance::save_measurement(&m, &s, &p, &t.0.join("evidence")).is_err());

    assert_eq!(m.windows.len(), 200);
    assert_eq!(m.windows[1].start_frame, 160);
    assert!(m.windows.iter().filter(|w| w.channels[0].onset).count() >= 6);
    assert_eq!(m.windows.iter().filter(|w| w.channels[1].onset).count(), 0);
    assert!(m.windows.iter().any(|w| w.channels[1].bleed_ambiguous));
    assert!(
        m.windows
            .iter()
            .filter(|w| w.channels[2].active && w.channels[2].confidence >= p.minimum_confidence)
            .count()
            > 190
    );
    assert!(m.windows[100].channels[2].stereo_correlation < -0.999);
    let n = m.signals;
    assert!(m.windows[100].covariance[(2 * n + 2) * n + 2] > 0.);
    let mut shifted = s.clone();
    shifted.channels[0].fader_db = 3.;
    let next = balance::measure(&shifted, &t.0, &p).unwrap();
    for (a, b) in m.windows.iter().zip(&next.windows) {
        assert_eq!(a.start_frame, b.start_frame);
        assert_eq!(a.channels[0].onset, b.channels[0].onset);
        assert_eq!(
            a.channels[0].pre_compressor_dbfs,
            b.channels[0].pre_compressor_dbfs
        );
    }
}

#[test]
fn improvement_needs_its_own_held_out_evidence() {
    let s = session(&[Role::LeadVocal, Role::RhythmGuitar, Role::BassDi]);
    let mut p = policy(&s, 0, 1, false, false, [0., 12.]);
    p.relationships.push(Relationship {
        name: "unrelated measured ratio".into(),
        numerator: vec![2],
        denominator: vec![1],
        activity_source: 2,
        band: 0,
        event_only: false,
        kit_stage: false,
        range_db: [-40., 40.],
    });
    let mut m = features(&[-30., -20., -20.], true);
    for w in &mut m.windows {
        if (w.start_frame / 2000) % 2 == 1 {
            w.channels[0].confidence = 0.3;
        }
    }
    let r = balance::optimize(&m, &s, &p).unwrap();
    assert!(!r.accepted);
    assert_eq!(r.deltas_db, vec![0.; 3]);
}

#[test]
fn acoustic_roles_enter_policy_but_unsupported_ensembles_are_not_success() {
    let s = session(&[Role::LeadVocal, Role::AcousticGuitar]);
    assert!(
        Policy::for_session(&s)
            .relationships
            .iter()
            .any(|r| r.denominator == vec![1])
    );
    let t = Scratch::new();
    let s = session(&[Role::Other]);
    let mut wav = hound::WavWriter::create(
        t.0.join("0.wav"),
        hound::WavSpec {
            channels: 1,
            sample_rate: 8000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        },
    )
    .unwrap();
    for _ in 0..1600 {
        wav.write_sample(0_i16).unwrap();
    }
    wav.finalize().unwrap();
    balance::run(s, &t.0, &t.0.join("pass"), None, true).unwrap();
    let status: serde_json::Value =
        serde_json::from_reader(std::fs::File::open(t.0.join("pass/status.json")).unwrap())
            .unwrap();
    assert_eq!(status["policy_targets_met"], false);
    assert_eq!(status["accepted_fader_change"], false);
}
