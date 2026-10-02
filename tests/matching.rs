use gigpies::automix::{
    self,
    config::{self, EqBand, EqKind, Role},
    dsp::{Biquad, Strip},
    effects::*,
    matching::*,
    preservation, unity,
};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
const RATE: u32 = 16000;
struct Fixture {
    root: PathBuf,
    s: config::Session,
    r: Request,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
fn band(hz: f64, db: f64) -> EqBand {
    EqBand {
        kind: EqKind::Bell,
        hz,
        q: 0.7,
        db,
    }
}
fn fixture(noise: bool) -> Fixture {
    let root = std::env::temp_dir().join(format!(
        "gigpies-matching-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&root).unwrap();
    let mut s = config::example();
    s.channels.truncate(2);
    s.channels[0].role = Role::RhythmGuitar;
    s.channels[1].role = Role::LeadGuitar;
    s.sample_rate = RATE;
    for (i, c) in s.channels.iter_mut().enumerate() {
        c.file = format!("{i}.wav").into();
        c.group = "guitar".into();
        c.pan = 0.;
    }
    s.groups = vec![config::Group {
        name: "guitar".into(),
        reference: 0,
        trim_db: 0.,
    }];
    unity::prepare(&mut s).unwrap();
    s = preservation::source_settings(&s).unwrap();
    s.channels[1].fader_db = -9.;
    let mut writers = (0..2)
        .map(|i| {
            hound::WavWriter::create(
                root.join(format!("{i}.wav")),
                hound::WavSpec {
                    channels: 2,
                    sample_rate: RATE,
                    bits_per_sample: 24,
                    sample_format: hound::SampleFormat::Int,
                },
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
    let mut seed = 1u64;
    // Repeated multi-pitch chord phrases, harmonic yet broadband, with fixed sample timing.
    let phrase = (0..RATE * 4)
        .map(|k| {
            let t = k as f64 / RATE as f64;
            let f = [98., 123.47, 146.83, 196.][(t / 0.5) as usize % 4];
            let env = 0.25 + 0.75 * (-(t % 0.5) * 5.).exp();
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            if noise {
                ((seed >> 32) as u32 as f64 / u32::MAX as f64 - 0.5) * 0.15
            } else {
                (1..48)
                    .map(|h| {
                        let hz = f * h as f64;
                        if hz > RATE as f64 * 0.44 {
                            0.
                        } else {
                            (std::f64::consts::TAU * hz * t + 0.27 * h as f64).sin()
                                / (h as f64).sqrt()
                        }
                    })
                    .sum::<f64>()
                    * 0.028
                    * env
            }
        })
        .collect::<Vec<_>>();
    for k in 0..RATE as usize * 16 {
        let x = phrase[k % phrase.len()];
        for (i, w) in writers.iter_mut().enumerate() {
            for v in [x, x * if i == 0 { 0.7 } else { -0.4 }] {
                w.write_sample((v * 8388608.) as i32).unwrap();
            }
        }
    }
    for w in writers {
        w.finalize().unwrap();
    }
    let context = Context {
        instrument: Instrument::ElectricGuitar,
        capture: Capture::RecordedTrack,
        tuning: "synthetic changing chords".into(),
        register_hz: Some([98., 196.]),
        technique: "repeated picked chord phrases".into(),
    };
    let r = Request {
        group: InputGroup {
            name: "guitar group".into(),
            inputs: vec![
                Input {
                    channel: 0,
                    file: "0.wav".into(),
                },
                Input {
                    channel: 1,
                    file: "1.wav".into(),
                },
            ],
            identity_basis: "Generated coherent two-path fixture".into(),
            context,
            representative_phrases: "Four pitches, repeated attack/body/sustain".into(),
        },
        training: vec![[0., 8.]],
        held_out: vec![[8., 16.]],
        maps: vec!["keep-current".into(), "dark-metal-guitar".into()],
        routing_basis: "Synthetic sources only, no imported returns".into(),
        excluded_fx_returns: vec![],
    };
    Fixture { root, s, r }
}
fn fx(s: &mut config::Session) {
    s.channels[0].compressor.threshold_db = -24.;
    s.channels[0].compressor.ratio = 2.;
    s.channels[0].compressor.makeup_db = 0.5;
    s.channels[0].eq.push(band(500., 1.));
    s.effects = Some(FxConfig {
        buses: vec![Bus {
            name: "guitar_space".into(),
            effect: Effect::Reverb(ReverbConfig {
                kind: ReverbKind::SmallRoom,
                predelay_ms: 10.,
                decay: 0.3,
                damping: 0.4,
            }),
            sends: vec![
                Send {
                    channel: 0,
                    db: -18.,
                },
                Send {
                    channel: 1,
                    db: -18.,
                },
            ],
            hpf_hz: 160.,
            lowpass_hz: 5000.,
            return_db: -12.,
            target_wet_db: -24.,
        }],
        exciter: ExciterConfig {
            tune_hz: 2000.,
            drive: 0.1,
            tone: 0.4,
            bright: false,
        },
        exciter_amount: 0.05,
        master_eq: vec![band(400., 0.5)],
        maximizer_drive_db: 0.,
        maximizer_threshold_db: 24.,
        maximizer_release_ms: 100.,
        tail_seconds: 0.5,
        listening_target_lufs: -20.,
    });
}
fn value(x: &impl serde::Serialize) -> serde_json::Value {
    serde_json::to_value(x).unwrap()
}
fn reference(f: &Fixture) -> ToneMap {
    import(
        f.s.clone(),
        &f.root,
        &f.root.join("map.json"),
        ImportSpec {
            id: "fixture-phrase".into(),
            version: 1,
            label: "Synthetic calibration phrase; no instrument standard".into(),
            basis: "Known generated chord fixture".into(),
            group: f.r.group.clone(),
            passages: f.r.training.clone(),
            sources: vec![Reference {
                citation: "GigPies synthetic fixture".into(),
                locator: "tests/matching.rs".into(),
                license: "MIT".into(),
                distribution: Distribution::OriginalAuthored,
                capture_notes: "Generated pitched chords".into(),
            }],
            supported_hz: [125., 6300.],
            limitations: vec!["Synthetic test; no musical acceptance".into()],
        },
    )
    .unwrap()
}
#[test]
fn production_zero_reset_and_frozen_amounts_preserve_complete_baseline() {
    let mut f = fixture(false);
    fx(&mut f.s);
    let bands = vec![band(2000., -3.), band(315., 1.)];
    let baseline = value(&f.s);
    for amount in [0., 100., 50., 25., 100., 0., 75., 0.] {
        let s = apply(&f.s, &f.r.group, &bands, amount).unwrap();
        if amount == 0. {
            assert_eq!(value(&s), baseline);
        } else {
            for i in &f.r.group.inputs {
                assert_eq!(s.channels[i.channel].eq.last().unwrap().db, amount / 100.);
                assert_eq!(
                    &value(&s.channels[i.channel].compressor),
                    &value(&f.s.channels[i.channel].compressor)
                );
            }
            assert_eq!(value(&s.effects), value(&f.s.effects));
        }
    }
    let a = f.root.join("baseline");
    let b = f.root.join("zero");
    automix::run(f.s.clone(), &f.root, &a, None).unwrap();
    automix::run(
        apply(&f.s, &f.r.group, &bands, 0.).unwrap(),
        &f.root,
        &b,
        None,
    )
    .unwrap();
    for file in ["processed.wav", "processed-unity-float.wav"] {
        assert_eq!(
            std::fs::read(a.join(file)).unwrap(),
            std::fs::read(b.join(file)).unwrap()
        );
    }
    let state = plan(f.s.clone(), &f.root, &f.root.join("plan"), f.r.clone()).unwrap();
    let saved: State = read_json(&f.root.join("plan/state.json")).unwrap();
    saved.validate().unwrap();
    assert_eq!(state.frozen_id, saved.frozen_id);
    let mut changed = saved.clone();
    changed.baseline.channels[0].fader_db += 1.;
    assert!(changed.validate().is_err());
    reset(saved, &f.root.join("reset")).unwrap();
    let restored: config::Session = read_json(&f.root.join("reset/settings.json")).unwrap();
    assert_eq!(value(&restored), baseline);
    assert!(plan(f.s.clone(), &f.root, &f.root.join("plan"), f.r.clone()).is_err());
}
#[test]
fn measured_self_match_abstains_and_injected_colour_is_corrected_by_actual_dsp() {
    let f = fixture(false);
    let map = reference(&f);
    let before = measure(&f.s, &f.root, &f.r.group, &[[0., 16.]]).unwrap();
    let e = envelope(&before, &f.r.training, None);
    assert!(e.active_seconds > 3., "{e:?}");
    assert!(fit(&e, &map, RATE, 8).unwrap().bands.is_empty());
    let mut colored = f.s.clone();
    for i in &f.r.group.inputs {
        colored.channels[i.channel].eq.push(band(2000., 4.));
    }
    let observed = measure(&colored, &f.root, &f.r.group, &[[0., 16.]]).unwrap();
    let e = envelope(&observed, &f.r.training, None);
    let proposal = fit(&e, &map, RATE, 7).unwrap();
    assert!(!proposal.bands.is_empty(), "{proposal:?}");
    assert!(proposal.predicted_error_db < proposal.before_error_db * 0.8);
    let corrected = apply(&colored, &f.r.group, &proposal.bands, 100.).unwrap();
    let after = measure(&corrected, &f.root, &f.r.group, &[[0., 16.]]).unwrap();
    let checks = checks(&observed, &after, &f.r, &map, &proposal, 100.);
    assert!(checks.iter().all(|c| c.passed), "{checks:?}");
    println!(
        "Synthetic recovery evidence: {}",
        serde_json::json!({"injected_db":4., "proposal":proposal, "actual_checks":checks})
    );
    for i in &f.r.group.inputs {
        assert_eq!(corrected.channels[i.channel].eq[0].db, 4.);
    }
    let mut offset = e.clone();
    for v in &mut offset.values_db {
        *v += 20.;
    }
    let again = fit(&offset, &map, RATE, 7).unwrap();
    assert_eq!(value(&again.bands), value(&proposal.bands));
    let full = fit(&e, &map, RATE, 0).unwrap();
    assert!(full.bands.is_empty());
    assert!(full.reason.contains("capacity"));
    let mut exact = e.clone();
    exact.values_db = map.values_db.clone();
    assert!(fit(&exact, &map, RATE, 0).unwrap().bands.is_empty());
}
#[test]
fn silence_noise_unsupported_bands_and_cancellation_do_not_acquire_boosts() {
    let mut f = fixture(true);
    let map = builtin_maps().remove(1);
    let m = measure(&f.s, &f.root, &f.r.group, &[[0., 16.]]).unwrap();
    let e = envelope(&m, &f.r.training, None);
    assert!(
        fit(&e, &map, RATE, 8).unwrap().bands.is_empty(),
        "noise envelope {e:?}"
    );
    for c in &mut f.s.channels {
        c.fader_db = -80.;
    }
    let silent = measure(&f.s, &f.root, &f.r.group, &[[0., 16.]]).unwrap();
    assert!(
        fit(&envelope(&silent, &f.r.training, None), &map, RATE, 8)
            .unwrap()
            .bands
            .is_empty()
    );
    let mut e = Envelope {
        values_db: vec![0.; BANDS],
        uncertainty_db: vec![0.; BANDS],
        supported: vec![true; BANDS],
        boost_supported: vec![false; BANDS],
        active_seconds: 10.,
        eligible_windows: 20,
        phrase_variation_db: 1.,
        activity_threshold_dbfs: -65.,
    };
    let mut relative = map;
    for v in &mut relative.values_db {
        *v = -*v;
    }
    let p = fit(&e, &relative, RATE, 8).unwrap();
    for hz in frequencies() {
        assert!(response(&p.bands, RATE, hz, 100.) <= 0.250001);
    }
    let mut invalid = e.clone();
    invalid.values_db.pop();
    assert!(fit(&invalid, &relative, RATE, 8).is_err());
    e.supported.fill(false);
    assert!(fit(&e, &relative, RATE, 8).unwrap().bands.is_empty());
}
#[test]
fn shared_coherent_curve_preserves_phase_and_observer_matches_renderer_with_fx() {
    let mut f = fixture(false);
    fx(&mut f.s);
    f.s.master_hpf_hz = 45.;
    f.s.master_db = -2.;
    let added = vec![band(2000., -2.)];
    let s = apply(&f.s, &f.r.group, &added, 50.).unwrap();
    let m = measure(&s, &f.root, &f.r.group, &[[0., 16.]]).unwrap();
    let out = f.root.join("render");
    automix::run(s.clone(), &f.root, &out, None).unwrap();
    let mut r = hound::WavReader::open(out.join("processed-unity-float.wav")).unwrap();
    let samples = r.samples::<f32>().map(Result::unwrap).collect::<Vec<_>>();
    for w in &m.windows {
        let start = (w.start * RATE as f64).round() as usize * 2;
        let end = (w.end * RATE as f64).round() as usize * 2;
        let rms = (samples[start..end]
            .iter()
            .map(|v| (*v as f64).powi(2))
            .sum::<f64>()
            / (end - start) as f64)
            .sqrt();
        assert!((gigpies::automix::dsp::db(rms) - w.mix_rms_dbfs).abs() < 1e-6);
    }
    assert!(m.windows.iter().any(|w| w.compressor_max_db[0] > 0.));
    assert!(m.windows.iter().any(|w| w.return_rms_dbfs[0] > -90.));
    let baseline = measure(&f.s, &f.root, &f.r.group, &[[0., 16.]]).unwrap();
    assert!(
        baseline
            .windows
            .iter()
            .zip(&m.windows)
            .any(|(a, b)| (a.compressor_max_db[0] - b.compressor_max_db[0]).abs() > 0.001)
    );
    assert!(
        baseline
            .windows
            .iter()
            .zip(&m.windows)
            .any(|(a, b)| (a.return_rms_dbfs[0] - b.return_rms_dbfs[0]).abs() > 0.01)
    );
    // Identical appended phase response on each coherent path and both sides.
    let mut l = Biquad::equalizer(&band(2000., -1.), RATE);
    let mut r = l;
    for k in 0..8000 {
        let x = (k as f64 * 0.33).sin();
        assert!((r.tick(-x) + l.tick(x)).abs() < 1e-12);
    }
    let mut strip = Strip::new(&s.channels[0], RATE);
    let mut tap = Strip::new(&s.channels[0], RATE);
    for k in 0..8000 {
        let x = [(k as f64 * 0.2).sin() * 0.1, -0.01];
        assert_eq!(strip.tick(x), tap.tick_with_eq_tap(x).1);
    }
}
#[test]
fn invalid_maps_identity_amount_capacity_and_routing_fail_explicitly() {
    for mut m in builtin_maps() {
        m.validate().unwrap();
        m.analysis.smoothing_octaves = 0.;
        assert!(m.validate().is_err());
    }
    let mut f = fixture(false);
    let mut r = f.r.clone();
    r.held_out = vec![[1., 9.]];
    assert!(r.validate(&f.s).is_err());
    r = f.r.clone();
    r.group.context.instrument = Instrument::BassDi;
    assert!(r.validate(&f.s).is_err());
    r = f.r.clone();
    r.excluded_fx_returns.push("0.wav".into());
    assert!(r.validate(&f.s).is_err());
    for a in [-0.1, 100.1, f64::NAN] {
        assert!(apply(&f.s, &f.r.group, &[band(1000., 1.)], a).is_err());
    }
    f.s.channels[1].eq = vec![band(300., 0.5); 8];
    assert!(apply(&f.s, &f.r.group, &[band(2000., -1.)], 100.).is_err());
    assert_eq!(
        value(&apply(&f.s, &f.r.group, &[band(2000., -1.)], 0.).unwrap()),
        value(&f.s)
    );
    let mut map = builtin_maps().remove(1);
    map.values_db.pop();
    assert!(map.validate().is_err());
}
#[test]
fn pitch_changes_are_smoothed_and_filters_stay_broad_deterministic() {
    let f = fixture(false);
    let m = measure(&f.s, &f.root, &f.r.group, &[[0., 16.]]).unwrap();
    let e = envelope(&m, &f.r.training, None);
    let map = builtin_maps().remove(1);
    let p = fit(&e, &map, RATE, 8).unwrap();
    assert!(!p.bands.is_empty(), "{p:?}");
    assert!(p.bands.iter().all(|b| b.q == 0.7 && b.db.abs() <= 3.));
    assert!(p.bands.len() <= 3);
    assert_eq!(value(&p), value(&fit(&e, &map, RATE, 8).unwrap()));
    let state = plan(f.s.clone(), &f.root, &f.root.join("plan"), f.r.clone()).unwrap();
    let c = state.choice("dark-metal-guitar").unwrap();
    assert!(c.eligible, "{:?}", c.checks);
    let full_selection = Selection {
        frozen_id: state.frozen_id.clone(),
        map_id: c.map.id.clone(),
        map_version: c.map.version,
        amount_percent: 100.,
    };
    let full = select(state.clone(), &f.root, &f.root.join("full"), full_selection).unwrap();
    assert_eq!(
        value(&full.last_attempt.as_ref().unwrap().checks),
        value(&c.checks)
    );
    assert_eq!(full.amount_percent, 100.);
    reset(full, &f.root.join("full-reset")).unwrap();
    let reset_settings: config::Session =
        read_json(&f.root.join("full-reset/settings.json")).unwrap();
    assert_eq!(value(&reset_settings), value(&f.s));
    let selection = Selection {
        frozen_id: state.frozen_id.clone(),
        map_id: c.map.id.clone(),
        map_version: c.map.version,
        amount_percent: 50.,
    };
    let selected = select(
        state.clone(),
        &f.root,
        &f.root.join("half"),
        selection.clone(),
    )
    .unwrap();
    assert_eq!(selected.amount_percent, 50.);
    let reloaded: State = read_json(&f.root.join("half/state.json")).unwrap();
    let again = select(reloaded, &f.root, &f.root.join("half-again"), selection).unwrap();
    assert_eq!(
        value(&again.settings().unwrap()),
        value(&selected.settings().unwrap())
    );
    let mut stale = Selection {
        frozen_id: "stale".into(),
        map_id: c.map.id.clone(),
        map_version: 1,
        amount_percent: 50.,
    };
    assert!(select(state.clone(), &f.root, &f.root.join("stale"), stale.clone()).is_err());
    stale.frozen_id = state.frozen_id.clone();
    stale.map_version = 2;
    assert!(select(state.clone(), &f.root, &f.root.join("stale"), stale).is_err());
    let html = std::fs::read_to_string(f.root.join("plan/review.html")).unwrap();
    assert!(html.contains("aria-live=\"polite\""));
    assert!(!html.contains("autoplay"));
}

#[test]
fn held_out_failure_vetoes_without_rewriting_the_training_proposal() {
    let f = fixture(false);
    let before = measure(&f.s, &f.root, &f.r.group, &[[0., 16.]]).unwrap();
    let map = builtin_maps().remove(1);
    let training = envelope(&before, &f.r.training, None);
    let p = fit(&training, &map, RATE, 8).unwrap();
    let s = apply(&f.s, &f.r.group, &p.bands, 100.).unwrap();
    let after = measure(&s, &f.root, &f.r.group, &[[0., 16.]]).unwrap();
    let frozen = value(&p);
    let mut bad = after.clone();
    for w in &mut bad.windows {
        if w.start >= 8. {
            w.compressor_max_db[0] += 4.;
        }
    }
    let result = checks(&before, &bad, &f.r, &map, &p, 100.);
    assert!(result[0].passed);
    assert!(!result[1].passed);
    let mut other = before.clone();
    for w in &mut other.windows {
        if w.start >= 8. {
            w.envelope_db.fill(-100.);
            w.raw_rms_dbfs = -120.;
        }
    }
    assert_eq!(
        value(&fit(&envelope(&other, &f.r.training, None), &map, RATE, 8).unwrap()),
        frozen
    );
    assert_eq!(value(&p), frozen);
}

#[test]
fn single_notes_actual_cancellation_and_changed_recordings_abstain_or_refuse() {
    let mut f = fixture(false);
    let state = plan(f.s.clone(), &f.root, &f.root.join("plan"), f.r.clone()).unwrap();
    // Replace only this fixture's private generated samples with one sustained rich note.
    for i in 0..2 {
        let mut w = hound::WavWriter::create(
            f.root.join(format!("{i}.wav")),
            hound::WavSpec {
                channels: 2,
                sample_rate: RATE,
                bits_per_sample: 24,
                sample_format: hound::SampleFormat::Int,
            },
        )
        .unwrap();
        for k in 0..RATE * 16 {
            let t = k as f64 / RATE as f64;
            let x = (1..25)
                .map(|h| (std::f64::consts::TAU * 125. * h as f64 * t).sin() / h as f64)
                .sum::<f64>()
                * 0.02;
            for v in [x, x * 0.7] {
                w.write_sample((v * 8388608.) as i32).unwrap();
            }
        }
        w.finalize().unwrap();
    }
    let selection = Selection {
        frozen_id: state.frozen_id.clone(),
        map_id: "keep-current".into(),
        map_version: 1,
        amount_percent: 0.,
    };
    assert!(select(state, &f.root, &f.root.join("changed"), selection).is_err());
    assert!(!f.root.join("changed").exists());
    let obs = measure(&f.s, &f.root, &f.r.group, &[[0., 16.]]).unwrap();
    let e = envelope(&obs, &f.r.training, None);
    assert!(
        fit(&e, &builtin_maps().remove(1), RATE, 8)
            .unwrap()
            .bands
            .is_empty(),
        "{e:?}"
    );
    // Two coherent paths nearly cancel. Their individual spectral powers are retained.
    let mut reader = hound::WavReader::open(f.root.join("0.wav")).unwrap();
    let spec = reader.spec();
    let samples = reader
        .samples::<i32>()
        .map(Result::unwrap)
        .collect::<Vec<_>>();
    drop(reader);
    let mut w = hound::WavWriter::create(f.root.join("1.wav"), spec).unwrap();
    for x in samples {
        w.write_sample((-x as f64 * 0.8) as i32).unwrap();
    }
    w.finalize().unwrap();
    f.s.channels[1].fader_db = f.s.channels[0].fader_db;
    let obs = measure(&f.s, &f.root, &f.r.group, &[[0., 16.]]).unwrap();
    let e = envelope(&obs, &f.r.training, None);
    assert!(obs.windows.iter().all(|w| w.cancellation_db[15] < -9.));
    assert!(e.boost_supported.iter().all(|v| !*v));
}

#[test]
fn different_registers_cannot_create_narrow_or_out_of_range_filters() {
    let f = fixture(false);
    let mut m = measure(&f.s, &f.root, &f.r.group, &[[0., 16.]]).unwrap();
    let map = builtin_maps().remove(1);
    // A one-octave shifted envelope models a changed register. No fitter frequency
    // is chosen from a spectral peak and Q remains fixed under either register.
    let original = envelope(&m, &f.r.training, None);
    for w in &mut m.windows {
        w.envelope_db.rotate_right(3);
        w.band_supported.rotate_right(3);
    }
    let shifted = envelope(&m, &f.r.training, None);
    for e in [&original, &shifted] {
        let p = fit(e, &map, RATE, 8).unwrap();
        for b in &p.bands {
            assert_eq!(b.q, 0.7);
            assert!(b.hz >= 125. && b.hz <= 6300.);
        }
    }
    let mut context = f.r.group.context.clone();
    context.register_hz = Some([800., 1200.]);
    assert!(!context.compatible(&f.r.group.context));
}
