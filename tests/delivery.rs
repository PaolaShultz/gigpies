use gigpies::automix::{self, config::*, delivery::*, effects};
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "gigpies-delivery-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&p).unwrap();
        Self(p)
    }
    fn fixture(&self, silent: bool) -> Session {
        let mut s = example();
        s.channels.truncate(1);
        s.groups.truncate(1);
        s.prepared = true;
        s.sample_rate = 8000;
        s.block_frames = 127;
        s.output_mode = OutputMode::Unmatched;
        s.master_db = 0.;
        s.master_hpf_hz = 40.;
        s.calibration.initial_trim_db = 0.;
        s.ceiling_db = -0.01;
        let c = &mut s.channels[0];
        c.eq.clear();
        c.hpf_hz = 0.;
        c.compressor.ratio = 1.;
        c.compressor.makeup_db = 0.;
        c.fader_db = 12.;
        c.pan = 0.;
        let mut w = hound::WavWriter::create(
            self.0.join(&c.file),
            hound::WavSpec {
                channels: 2,
                sample_rate: 8000,
                bits_per_sample: 32,
                sample_format: hound::SampleFormat::Float,
            },
        )
        .unwrap();
        for t in 0..4000 {
            let a = if silent {
                0.
            } else {
                0.7 * ((t as f64 * std::f64::consts::PI / 2.) + std::f64::consts::PI / 4.).sin()
            };
            w.write_sample(a as f32).unwrap();
            w.write_sample((a * 0.3) as f32).unwrap();
        }
        w.finalize().unwrap();
        s
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn json(p: &Path) -> serde_json::Value {
    serde_json::from_reader(std::fs::File::open(p).unwrap()).unwrap()
}
#[test]
fn comparison_and_ceiling_do_not_toggle_either_master_processor() {
    let tmp = Scratch::new();
    let base = tmp.fixture(false);
    for fx_active in [false, true] {
        for final_active in [false, true] {
            let mut s = base.clone();
            let mut full = example();
            effects::add_pass(&mut full).unwrap();
            let mut fx = full.effects.unwrap();
            fx.buses.clear();
            fx.exciter_amount = 0.;
            fx.exciter.tune_hz = 1000.;
            fx.master_eq.clear();
            fx.tail_seconds = 0.;
            fx.maximizer_drive_db = 0.;
            fx.maximizer_threshold_db = if fx_active { -3. } else { 24. };
            s.effects = Some(fx);
            let p = Policy {
                final_sample_limiter: if final_active {
                    FinalLimiter::Enabled {
                        threshold_dbfs: -9.,
                        release_ms: 100.,
                    }
                } else {
                    FinalLimiter::Disabled
                },
                ..Policy::default()
            };
            let a = tmp.0.join(format!("a-{fx_active}-{final_active}"));
            let b = tmp.0.join(format!("b-{fx_active}-{final_active}"));
            let c = tmp.0.join(format!("c-{fx_active}-{final_active}"));
            automix::run_policy(s.clone(), &tmp.0, &a, p.clone()).unwrap();
            let mut matched = p.clone();
            matched.comparison = Comparison::Loudness { target_lufs: -23. };
            automix::run_policy(s.clone(), &tmp.0, &b, matched).unwrap();
            let mut lower = p;
            lower.ceiling_db = -6.;
            automix::run_policy(s, &tmp.0, &c, lower).unwrap();
            for name in ["processed-unity-float.wav", "prepared.json"] {
                assert_eq!(
                    std::fs::read(a.join(name)).unwrap(),
                    std::fs::read(b.join(name)).unwrap()
                );
                assert_eq!(
                    std::fs::read(a.join(name)).unwrap(),
                    std::fs::read(c.join(name)).unwrap()
                );
            }
            assert_eq!(
                std::fs::read(a.join("processed.wav")).unwrap(),
                std::fs::read(b.join("processed.wav")).unwrap()
            );
            let report = json(&a.join("measurements.json"));
            assert_eq!(
                report["master_max_reduction_db"].as_f64().unwrap() > 0.,
                final_active
            );
            assert_eq!(
                report["effects"]["maximizer_max_reduction_db"]
                    .as_f64()
                    .unwrap()
                    > 0.,
                fx_active
            );
            assert!(
                json(&a.join("stages.json"))["peak_contributions"]["residual"]
                    .as_f64()
                    .unwrap()
                    .abs()
                    < 1e-10
            );
            check(&a, &tmp.0).unwrap();
            check(&b, &tmp.0).unwrap();
        }
    }
}
#[test]
fn legacy_bus_reuse_identity_silence_and_failure_recovery() {
    let tmp = Scratch::new();
    let s = tmp.fixture(false);
    let legacy = tmp.0.join("legacy");
    automix::run(s.clone(), &tmp.0, &legacy, None).unwrap();
    let new = tmp.0.join("delivery");
    finalize(&legacy, &tmp.0, &new, Policy::default()).unwrap();
    check(&new, &tmp.0).unwrap();
    let engine = json(&new.join("measurements.json"));
    assert!(engine["processed_export_gain_db"].is_null());
    assert!(engine["historical_export_observations"]["processed_export_gain_db"].is_number());
    assert_eq!(engine["render_contract"], "delivery-policy-v1");
    let report = json(&new.join("delivery.json"));
    assert!(
        std::fs::read_to_string(new.join("report.txt"))
            .unwrap()
            .contains(&format!(
                "Static export gain: {:.6}",
                report["primary"]["gain_db"].as_f64().unwrap()
            ))
    );
    assert!(finalize(&legacy, &tmp.0, &new, Policy::default()).is_err());
    let bad = Policy {
        delivery_gain: DeliveryGain::Fixed { gain_db: 0. },
        ..Policy::default()
    };
    let failed = tmp.0.join("failed");
    assert!(automix::run_policy(s.clone(), &tmp.0, &failed, bad).is_err());
    assert!(!failed.join("delivery-ready.json").exists());
    std::fs::write(new.join("processed.wav"), b"changed").unwrap();
    assert!(check(&new, &tmp.0).is_err());
    let tmp = Scratch::new();
    let s = tmp.fixture(true);
    let out = tmp.0.join("silent");
    automix::run_policy(s, &tmp.0, &out, Policy::default()).unwrap();
    let r = json(&out.join("delivery.json"));
    assert_eq!(r["primary"]["gain_db"], 0.);
    assert_eq!(r["primary"]["output"]["true_peak"]["amplitude"], 0.);
    check(&out, &tmp.0).unwrap();
}
#[test]
fn policy_validation_and_observation_do_not_write_audio() {
    let tmp = Scratch::new();
    let s = tmp.fixture(false);
    let out = tmp.0.join("observation");
    automix::observe(s.clone(), &tmp.0, &out, 0.25).unwrap();
    assert_eq!(json(&out.join("measurements.json"))["frames"], 2000);
    assert!(!out.join("processed.wav").exists());
    assert!(!out.join("processed-bus.tmp.wav").exists());
    let p = Policy {
        version: 2,
        ..Policy::default()
    };
    assert!(automix::run_policy(s, &tmp.0, &tmp.0.join("invalid"), p).is_err());
    assert!(!tmp.0.join("invalid").exists());
    let p = Policy {
        ceiling_db: f64::NAN,
        ..Policy::default()
    };
    assert!(p.validate().is_err());
    let mut v = serde_json::to_value(Policy::default()).unwrap();
    v["unknown"] = true.into();
    assert!(serde_json::from_value::<Policy>(v).is_err());
}
