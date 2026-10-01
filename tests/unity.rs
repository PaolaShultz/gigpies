use gigpies::automix::{
    self,
    config::{OutputMode, Role, example},
    effects, unity,
};
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "gigpies-unity-{}-{}",
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
fn wav(path: &Path, channels: u16, n: usize, signal: impl Fn(usize) -> f64) {
    let mut w = hound::WavWriter::create(
        path,
        hound::WavSpec {
            channels,
            sample_rate: 44100,
            bits_per_sample: 24,
            sample_format: hound::SampleFormat::Int,
        },
    )
    .unwrap();
    for i in 0..n {
        for _ in 0..channels {
            w.write_sample((signal(i) * 8388608.).round() as i32)
                .unwrap();
        }
    }
    w.finalize().unwrap();
}
#[test]
fn prepare_enforces_unity_di_only_and_exact_low_cuts() {
    let mut s = example();
    effects::add_pass(&mut s).unwrap();
    unity::prepare(&mut s).unwrap();
    assert_eq!(s.channels.len(), 12);
    assert_eq!(s.output_mode, OutputMode::Unmatched);
    assert_eq!(s.master_db, 0.);
    assert_eq!(s.master_hpf_hz, 40.);
    assert!(s.groups.iter().all(|g| g.trim_db == 0.));
    for c in &s.channels {
        assert!(!matches!(c.role, Role::BassAmp));
        assert_eq!(c.fader_db, 0.);
        assert_eq!(
            c.hpf_hz,
            if matches!(c.role, Role::Kick | Role::BassDi) {
                0.
            } else {
                90.
            }
        );
    }
    let fx = s.effects.unwrap();
    assert!(fx.master_eq.is_empty());
    assert!(fx.buses.iter().all(|b| b.hpf_hz == 90.));
    assert!(
        fx.buses
            .iter()
            .filter(|b| b.name.starts_with("vocal"))
            .all(|b| b.sends[0].channel == 10)
    );
}
#[test]
fn unmatched_retains_exact_float_sum_and_only_shared_measured_peak_attenuation() {
    let t = Scratch::new();
    for name in ["01_Kick.wav", "02_Snare.wav"] {
        wav(&t.0.join(name), 2, 4410, |_| 0.8);
    }
    let mut s = example();
    s.channels.truncate(2);
    s.groups.truncate(2);
    s.prepared = true;
    s.master_db = 0.;
    s.output_mode = OutputMode::Unmatched;
    s.ceiling_db = -0.5;
    for c in &mut s.channels {
        c.fader_db = 0.;
        c.hpf_hz = 0.;
        c.eq.clear();
        c.compressor.ratio = 1.;
        c.compressor.makeup_db = 0.;
    }
    automix::run(s, &t.0, &t.0.join("out"), None).unwrap();
    let samples: Vec<_> = hound::WavReader::open(t.0.join("out/bypass-unity-float.wav"))
        .unwrap()
        .samples::<f32>()
        .map(Result::unwrap)
        .collect();
    assert!(samples.iter().all(|v| (*v - 1.6).abs() < 1e-6));
    let m: serde_json::Value =
        serde_json::from_reader(std::fs::File::open(t.0.join("out/measurements.json")).unwrap())
            .unwrap();
    assert_eq!(m["output_mode"], "unmatched");
    assert!(m["matched_target_lufs"].is_null());
    assert!(m["bypass_matching_gain_db"].is_null());
    assert!(
        (m["common_export_gain_db"].as_f64().unwrap() - (-0.5 - 20. * 1.6_f64.log10())).abs()
            < 1e-5
    );
    assert!(!t.0.join("out/bypass-matched.wav").exists());
    assert_eq!(
        std::fs::read(t.0.join("out/bypass.wav")).unwrap(),
        std::fs::read(t.0.join("out/processed.wav")).unwrap()
    );
}
#[test]
fn measured_compression_preserves_input_level_difference() {
    let t = Scratch::new();
    for (name, a) in [("01_Kick.wav", 0.05), ("02_Snare.wav", 0.5)] {
        wav(&t.0.join(name), 1, 44100, |i| {
            a * (std::f64::consts::TAU * 1000. * i as f64 / 44100.).sin()
        });
    }
    let mut s = example();
    s.channels.truncate(2);
    s.groups.truncate(2);
    s.channels[1].role = Role::Kick;
    s.channels[1].compressor = s.channels[0].compressor.clone();
    for c in &mut s.channels {
        c.eq.clear();
    }
    unity::prepare(&mut s).unwrap();
    let report = unity::calibrate_compressors(&mut s, &t.0).unwrap();
    assert!(
        (s.channels[1].compressor.threshold_db - s.channels[0].compressor.threshold_db - 20.).abs()
            < 0.001
    );
    assert!(
        (s.channels[1].compressor.makeup_db - s.channels[0].compressor.makeup_db).abs() < 0.001
    );
    assert!(s.groups.iter().all(|g| g.trim_db == 0.));
    assert!(s.channels.iter().all(|c| c.fader_db == 0.));
    assert!(
        report
            .iter()
            .all(|r| r["measured_max_reduction_db"].as_f64().unwrap() <= 4.01)
    );
}
#[test]
fn envelope_audit_measures_crest_and_includes_final_partial_window() {
    let t = Scratch::new();
    let path = t.0.join("signal.wav");
    wav(&path, 2, 44101, |i| {
        if i == 44100 {
            0.9
        } else {
            0.1 * (std::f64::consts::TAU * 1000. * i as f64 / 44100.).sin()
        }
    });
    let e = unity::audit(&path, &t.0.join("envelope.csv")).unwrap();
    assert_eq!(e.windows, 11);
    assert!((e.crest_p50_db - 3.01).abs() < 0.01);
    assert!((e.peak_dbfs - 20. * 0.9_f64.log10()).abs() < 0.01);
}
#[test]
fn unity_workflow_writes_measurement_reasons_and_envelopes_without_matching() {
    let t = Scratch::new();
    wav(&t.0.join("01_Kick.wav"), 1, 44100, |i| {
        0.2 * (std::f64::consts::TAU * 150. * i as f64 / 44100.).sin()
    });
    let mut s = example();
    s.channels.truncate(1);
    s.groups.truncate(1);
    unity::run(s, &t.0, &t.0.join("pass")).unwrap();
    for name in [
        "raw-envelope.csv",
        "processed-envelope.csv",
        "envelope-review.json",
        "measured-channel-decisions.json",
        "raw-spectrum.json",
        "processed-spectrum.json",
        "settings.json",
    ] {
        assert!(t.0.join("pass").join(name).exists());
    }
    assert!(!t.0.join("pass/final/processed-matched.wav").exists());
    let mut saved: gigpies::automix::config::Session =
        serde_json::from_reader(std::fs::File::open(t.0.join("pass/settings.json")).unwrap())
            .unwrap();
    saved.validate().unwrap();
    saved.master_hpf_hz = f64::NAN;
    assert!(saved.validate().is_err());
}
