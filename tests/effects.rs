use gigpies::automix::{
    analysis::{self, Policy},
    config::{Session, example},
    dsp::gain,
    effects::{self, Effect, Rack, ReverbKind},
};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "gigpies-fx-{}-{}",
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
fn settings() -> Session {
    let mut s = example();
    s.prepared = true;
    effects::add_pass(&mut s).unwrap();
    s
}
#[test]
fn vocal_predelay_validation_and_legacy_defaults() {
    let mut s = settings();
    let fx = s.effects.as_mut().unwrap();
    for bus in &mut fx.buses {
        if bus.name == "vocal_plate"
            && let Effect::Reverb(r) = &mut bus.effect
        {
            r.predelay_ms = 29.;
        }
    }
    assert!(s.validate().is_err());
    let mut v = serde_json::to_value(example()).unwrap();
    v.as_object_mut().unwrap().remove("effects");
    let old: Session = serde_json::from_value(v).unwrap();
    assert!(old.effects.is_none());
    old.validate().unwrap();
    let mut s = settings();
    s.effects.as_mut().unwrap().maximizer_drive_db = 20.;
    assert!(s.validate().is_err());
    let mut s = settings();
    s.effects.as_mut().unwrap().buses[0].sends[0].channel = 999;
    assert!(s.validate().is_err());
}
#[test]
fn reverb_predelay_decay_and_distinct_engines() {
    let s = settings();
    let mut energies = Vec::new();
    for name in ["snare_plate", "tom_chamber", "vocal_hall", "vocal_plate"] {
        let mut p = s.effects.clone().unwrap();
        let bus = p.buses.iter().find(|b| b.name == name).unwrap().clone();
        let send_channel = bus.sends[0].channel;
        let predelay = match bus.effect {
            Effect::Reverb(r) => r.predelay_ms,
            _ => unreachable!(),
        };
        p.buses = vec![bus];
        let mut rack = Rack::new(&p, s.channels.len(), 44100);
        let mut early = 0.;
        let mut late = 0.;
        let mut total = 0.;
        for i in 0..44100 * 5 {
            if i == 0 {
                rack.send(send_channel, [0.5, 0.5]);
            }
            let y = rack.returns();
            for v in y {
                assert!(v.is_finite());
                if i < (predelay as f64 * 44.1).floor() as usize {
                    assert_eq!(v, 0.);
                }
                total += v * v;
                if i > 44100 * 4 {
                    late += v * v;
                } else if i < 44100 {
                    early += v * v;
                }
            }
        }
        assert!(early > 0.);
        assert!(late < early * 0.001, "{name}: {late} {early}");
        energies.push(total);
    }
    assert!(energies.windows(2).all(|e| (e[0] - e[1]).abs() > 1e-10));
}
#[test]
fn silence_stays_silent_and_maximizer_is_linked_and_bounded() {
    let s = settings();
    let mut rack = Rack::new(s.effects.as_ref().unwrap(), s.channels.len(), 44100);
    for _ in 0..10000 {
        for i in 0..s.channels.len() {
            assert_eq!(rack.excite(i, [0., 0.]), [0., 0.]);
            rack.send(i, [0., 0.]);
        }
        assert_eq!(rack.returns(), [0., 0.]);
        assert_eq!(rack.master([0., 0.]), [0., 0.]);
    }
    let ceiling = gain(s.effects.as_ref().unwrap().maximizer_threshold_db);
    for i in 0..10000 {
        let x = (i as f64 * 0.1).sin();
        let y = rack.master([x, x * 0.2]);
        assert!(y[0].abs() <= ceiling + 1e-12);
        assert!((y[0] * 0.2 - y[1]).abs() < 1e-12);
    }
    assert!(rack.max_reduction() > 10.);
}
fn wav(path: &std::path::Path, kind: &str) {
    let mut w = hound::WavWriter::create(
        path,
        hound::WavSpec {
            channels: 2,
            sample_rate: 44100,
            bits_per_sample: 24,
            sample_format: hound::SampleFormat::Int,
        },
    )
    .unwrap();
    let mut rng = 1u64;
    for i in 0..44100 * 10 {
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        let noise = ((rng >> 32) as u32 as f64 / u32::MAX as f64 - 0.5) * 0.01;
        let hz = if kind == "bass" { 63. } else { 315. };
        let tone = 0.12 * (std::f64::consts::TAU * hz * i as f64 / 44100.).sin();
        let x = match kind {
            "silence" => 0.,
            "noise" => noise,
            "transient" => {
                noise
                    + if i > 44100 * 4 && i < 44100 * 4 + 4410 {
                        tone
                    } else {
                        0.
                    }
            }
            _ => noise + tone,
        };
        for v in [x, x * 0.25] {
            w.write_sample((v * 8388608.).round() as i32).unwrap();
        }
    }
    w.finalize().unwrap();
}
#[test]
fn spectral_code_corrects_persistent_buildup_not_silence_noise_or_transients() {
    let t = Scratch::new();
    let p = Policy::default();
    for kind in ["silence", "noise", "transient", "bass", "persistent"] {
        let file = t.0.join(format!("{kind}.wav"));
        wav(&file, kind);
        let spectrum = analysis::analyze(&file, &p).unwrap();
        let mut s = settings();
        let changes = analysis::decisions(&mut s, &spectrum, &serde_json::json!({}), &p).unwrap();
        if kind == "persistent" {
            assert!(!changes.is_empty());
            assert!(
                s.effects
                    .as_ref()
                    .unwrap()
                    .master_eq
                    .iter()
                    .any(|b| (b.hz - 315.).abs() < 1. && b.db >= -1.5 && b.db < 0.)
            );
            let repeated =
                analysis::decisions(&mut s, &spectrum, &serde_json::json!({}), &p).unwrap();
            assert!(repeated.is_empty());
        } else {
            assert!(changes.is_empty(), "{kind}: {changes:?}");
        }
    }
}
#[test]
fn review_reduces_excess_dynamics_and_caps_return_correction() {
    let mut s = settings();
    let spec = analysis::Spectrum {
        sample_rate: 44100,
        frames: 1,
        active_windows: 0,
        active_seconds: 0.,
        peak_dbfs: 0.,
        rms_dbfs: -240.,
        bands: vec![],
    };
    let m = serde_json::json!({"effects":{"maximizer_max_reduction_db":4.5,"send_reference_meters":[{"rms_dbfs":-30.}],"return_meters_before_master":[{"rms_dbfs":-90.}]}});
    let changes = analysis::decisions(&mut s, &spec, &m, &Policy::default()).unwrap();
    assert_eq!(changes.len(), 2);
    let p = s.effects.unwrap();
    assert_eq!(p.maximizer_drive_db, 1.);
    assert_eq!(p.buses[0].return_db, 12.);
}
#[test]
fn different_wet_routes_are_deterministic_and_do_not_include_direct_signal() {
    let s = settings();
    let p = s.effects.as_ref().unwrap();
    let mut a = Rack::new(p, 13, 44100);
    let mut b = Rack::new(p, 13, 44100);
    for i in 0..44100 {
        let x = if i == 0 { [1., 0.5] } else { [0., 0.] };
        a.send(11, x);
        b.send(11, x);
        let ay = a.returns();
        assert_eq!(ay, b.returns());
        if i < 441 {
            assert_eq!(ay, [0., 0.]);
        }
    }
    assert!(matches!(p.buses[0].effect,Effect::Reverb(r) if r.kind==ReverbKind::Plate));
}

#[test]
fn finish_workflow_persists_decisions_and_preserves_timeline_with_fx_tail() {
    let t = Scratch::new();
    let mut s = settings();
    s.channels.truncate(1);
    s.groups.truncate(1);
    let p = s.effects.as_mut().unwrap();
    p.buses.truncate(1);
    p.buses[0].sends = vec![effects::Send {
        channel: 0,
        db: -13.,
    }];
    p.tail_seconds = 0.5;
    let path = t.0.join("01_Kick.wav");
    let mut w = hound::WavWriter::create(
        &path,
        hound::WavSpec {
            channels: 1,
            sample_rate: 44100,
            bits_per_sample: 24,
            sample_format: hound::SampleFormat::Int,
        },
    )
    .unwrap();
    for i in 0..44100 {
        w.write_sample(if i % 4410 == 0 { 4000000 } else { 0 })
            .unwrap();
    }
    w.finalize().unwrap();
    let original = std::fs::read(&path).unwrap();
    analysis::finish(s.clone(), &t.0, &t.0.join("finish"), Policy::default()).unwrap();
    let report: serde_json::Value = serde_json::from_reader(
        std::fs::File::open(t.0.join("finish/review-result.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(report["algorithm"], "bounded_review_v1");
    assert!(t.0.join("finish/decisions.json").exists());
    assert!(t.0.join("finish/reviewed-settings.json").exists());
    for stem in ["previous-matched", "new-matched"] {
        let mut r =
            hound::WavReader::open(t.0.join(format!("finish/review-comparison/{stem}.wav")))
                .unwrap();
        assert_eq!(r.duration(), 66150);
        assert!(
            r.samples::<i32>()
                .all(|v| v.unwrap().unsigned_abs() < 8388608)
        );
    }
    assert_eq!(original, std::fs::read(path).unwrap());
    assert!(analysis::finish(s, &t.0, &t.0.join("finish"), Policy::default()).is_err());
}
