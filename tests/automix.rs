use gigpies::automix::{config::*, dsp::*, run};
use std::{
    f64::consts::PI,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "gigpies-mix-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&p).unwrap();
        Self(p)
    }
    fn wav(&self, name: &str, channels: u16, frames: usize, signal: impl Fn(usize, usize) -> f64) {
        let mut w = hound::WavWriter::create(
            self.0.join(name),
            hound::WavSpec {
                channels,
                sample_rate: 44100,
                bits_per_sample: 24,
                sample_format: hound::SampleFormat::Int,
            },
        )
        .unwrap();
        for i in 0..frames {
            for ch in 0..channels {
                w.write_sample((signal(i, ch as usize) * 8388607.).round() as i32)
                    .unwrap();
            }
        }
        w.finalize().unwrap();
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn small() -> Session {
    let mut s = example();
    s.channels.truncate(2);
    s.groups.truncate(2);
    s.block_frames = 441;
    s
}
fn samples(path: &std::path::Path) -> Vec<i32> {
    hound::WavReader::open(path)
        .unwrap()
        .samples::<i32>()
        .map(|x| x.unwrap())
        .collect()
}
#[test]
fn calibration_silence_quiet_bleed_bounds_jumps_and_settling() {
    let p = example().calibration;
    let mut c = Calibrator::new(&p);
    for _ in 0..1000 {
        c.observe(0., 0., 0.02);
    }
    assert_eq!(c.trim_db, 0.);
    assert_eq!(c.active_seconds, 0.);
    for _ in 0..2000 {
        c.observe(gain(-42.), gain(-36.), 0.02);
    }
    assert!(c.active);
    assert!((c.trim_db - 18.).abs() < 0.01);
    let before = c.trim_db;
    for _ in 0..100 {
        c.observe(gain(-85.), gain(-80.), 0.02);
    }
    assert_eq!(c.trim_db, before);
    assert!(!c.active);
    c.observe(gain(-4.), 1., 0.02);
    assert!(c.trim_db < before - 1.);
    for _ in 0..2000 {
        c.observe(1., 1., 0.02);
        assert!((p.min_trim_db..=p.max_trim_db).contains(&c.trim_db));
    }
    assert!(c.trim_db <= -23.9);
    let mut c = Calibrator::new(&p);
    for _ in 0..100 {
        c.observe(gain(-20.), gain(-10.), 0.02);
    }
    let before = c.trim_db;
    for _ in 0..100 {
        c.observe(gain(-50.), gain(-40.), 0.02);
    }
    assert!(!c.active);
    assert_eq!(before, c.trim_db);
}
#[test]
fn filters_compressor_and_linked_sample_peak_protection() {
    let rate = 44100;
    let measure = |hz: f64, mut f: Biquad| {
        let mut energy = 0.;
        for i in 0..rate {
            let y = f.tick((2. * PI * hz * i as f64 / rate as f64).sin());
            if i > rate / 2 {
                energy += y * y;
            }
        }
        (energy / (rate / 2) as f64).sqrt()
    };
    assert!(db(measure(10., Biquad::highpass(100., 0.707, rate))) < -42.);
    let bell = EqBand {
        kind: Default::default(),
        hz: 1000.,
        q: 1.,
        db: 3.,
    };
    assert!((db(measure(1000., Biquad::bell(&bell, rate))) - (-3.0103 + 3.)).abs() < 0.02);
    let ch = Channel::preset("x", Role::Snare, "x", 0., 0.);
    let mut p = ch.compressor.clone();
    p.knee_db = 0.;
    assert!((compression_db(p.threshold_db + 10., &p) + 8.).abs() < 1e-9);
    let mut strip = Strip::new(&ch, rate);
    let mut limiter = Limiter::new(-2., 100., rate);
    for i in 0..rate {
        let v = (2. * PI * 1000. * i as f64 / rate as f64).sin();
        let y = strip.tick([v, v * 0.25]);
        assert!((y[1] - y[0] * 0.25).abs() < 1e-10);
        let z = limiter.tick([v * 10., v * 2.5]);
        assert!(z[0].abs() <= gain(-2.) + 1e-12);
        assert!((z[1] - z[0] * 0.25).abs() < 1e-10);
    }
    assert!(strip.max_reduction > 10.);
    assert!(limiter.max_reduction > 15.);
}
#[test]
fn loudness_reference_stereo_offset_and_silence_gating() {
    for rate in [44100, 48000] {
        let mut mono = Loudness::new(rate);
        let mut stereo = Loudness::new(rate);
        let mut silence = Loudness::new(rate);
        for i in 0..rate * 3 {
            let x = gain(-20.) * (2. * PI * 997. * i as f64 / rate as f64).sin();
            mono.add([x, 0.]);
            stereo.add([x, x]);
            silence.add([0., 0.]);
        }
        assert!((mono.integrated().unwrap() + 23.01).abs() < 0.08);
        assert!((stereo.integrated().unwrap() - mono.integrated().unwrap() - 3.0103).abs() < 1e-6);
        assert_eq!(silence.integrated(), None);
    }
}
#[test]
fn render_alignment_padding_freeze_persistence_headroom_and_source_integrity() {
    let tmp = Scratch::new();
    tmp.wav(
        "01_Kick.wav",
        1,
        44100,
        |i, _| if i == 4000 { 0.9 } else { 0. },
    );
    tmp.wav("02_Snare.wav", 2, 46000, |i, ch| {
        if i == 45000 {
            if ch == 0 { 0.7 } else { -0.35 }
        } else {
            0.
        }
    });
    let before = std::fs::read(tmp.0.join("01_Kick.wav")).unwrap();
    let s = small();
    run(s, &tmp.0, &tmp.0.join("check"), Some(0.5)).unwrap();
    let prepared: Session =
        serde_json::from_reader(std::fs::File::open(tmp.0.join("check/prepared.json")).unwrap())
            .unwrap();
    assert!(prepared.prepared);
    run(prepared.clone(), &tmp.0, &tmp.0.join("show"), None).unwrap();
    assert!(run(prepared, &tmp.0, &tmp.0.join("show"), None).is_err());
    let bypass = samples(&tmp.0.join("show/bypass.wav"));
    assert_eq!(bypass.len(), 92000);
    assert_eq!(bypass[..8000].iter().filter(|x| **x != 0).count(), 0);
    assert_ne!(bypass[8000], 0);
    assert_ne!(bypass[90000], 0);
    assert!((bypass[90000] as f64 / bypass[90001] as f64 + 2.).abs() < 1e-4);
    for stem in ["bypass", "processed", "bypass-matched", "processed-matched"] {
        let a = samples(&tmp.0.join(format!("show/{stem}.wav")));
        assert_eq!(a.len(), 92000);
        assert!(a.iter().all(|x| (*x as f64 / 8388608.).abs() < gain(-1.99)));
    }
    assert_eq!(std::fs::read(tmp.0.join("01_Kick.wav")).unwrap(), before);
}
#[test]
fn future_audio_cannot_change_earlier_bus_or_gain_history() {
    let tmp = Scratch::new();
    let mut s = small();
    s.channels.truncate(1);
    s.groups.truncate(1);
    for variant in 0..2 {
        let root = tmp.0.join(format!("source{variant}"));
        std::fs::create_dir(&root).unwrap();
        let src = Scratch(root.clone());
        src.wav("01_Kick.wav", 1, 44100, |i, _| {
            if i < 22050 {
                0.005 * (2. * PI * 100. * i as f64 / 44100.).sin()
            } else if variant == 0 {
                0.01
            } else {
                0.8
            }
        });
        run(
            s.clone(),
            &root,
            &tmp.0.join(format!("out{variant}")),
            Some(1.),
        )
        .unwrap();
    }
    // History is strictly causal; exported files may receive whole-program attenuation.
    let h0 = std::fs::read_to_string(tmp.0.join("out0/gain-history.csv")).unwrap();
    let h1 = std::fs::read_to_string(tmp.0.join("out1/gain-history.csv")).unwrap();
    assert_eq!(
        h0.lines().take(51).collect::<Vec<_>>(),
        h1.lines().take(51).collect::<Vec<_>>()
    );
    let a = samples(&tmp.0.join("out0/processed.wav"));
    let b = samples(&tmp.0.join("out1/processed.wav"));
    assert_eq!(&a[..44100], &b[..44100]);
}
#[test]
fn validation_rejects_bad_schema_and_unprepared_render() {
    let mut s = example();
    s.channels[0].eq[0].q = 0.;
    assert!(s.validate().is_err());
    let mut s = example();
    s.groups[0].reference = 999;
    assert!(s.validate().is_err());
    let mut s = example();
    s.calibration.up_db_per_second = f64::NAN;
    assert!(s.validate().is_err());
    let mut s = serde_json::to_value(example()).unwrap();
    s["surprise"] = true.into();
    assert!(serde_json::from_value::<Session>(s).is_err());
    let tmp = Scratch::new();
    assert!(run(small(), &tmp.0, &tmp.0.join("out"), None).is_err());
    assert!(!tmp.0.join("out").exists());
}

fn add_bwf(path: &std::path::Path, reference: u64) {
    let original = std::fs::read(path).unwrap();
    let mut data = original[..12].to_vec();
    data.extend_from_slice(b"bext");
    data.extend_from_slice(&346u32.to_le_bytes());
    let mut metadata = vec![0; 346];
    metadata[338..346].copy_from_slice(&reference.to_le_bytes());
    data.extend(metadata);
    data.extend_from_slice(&original[12..]);
    let size = (data.len() - 8) as u32;
    data[4..8].copy_from_slice(&size.to_le_bytes());
    std::fs::write(path, data).unwrap();
}
#[test]
fn bwf_offsets_are_preserved_and_mixed_metadata_is_rejected() {
    let tmp = Scratch::new();
    let mut s = small();
    s.prepared = true;
    s.channels[0].pan = -1.;
    s.channels[1].pan = 1.;
    tmp.wav("01_Kick.wav", 1, 100, |i, _| if i == 10 { 0.2 } else { 0. });
    tmp.wav(
        "02_Snare.wav",
        1,
        100,
        |i, _| if i == 10 { 0.2 } else { 0. },
    );
    add_bwf(&tmp.0.join("01_Kick.wav"), 1000);
    assert!(run(s.clone(), &tmp.0, &tmp.0.join("bad"), None).is_err());
    assert!(!tmp.0.join("bad").exists());
    add_bwf(&tmp.0.join("02_Snare.wav"), 1200);
    run(s, &tmp.0, &tmp.0.join("good"), None).unwrap();
    let a = samples(&tmp.0.join("good/bypass.wav"));
    assert_eq!(a.len(), 600);
    assert_ne!(a[20], 0);
    assert_eq!(a[21], 0);
    assert_ne!(a[421], 0);
    assert_eq!(a[420], 0);
}
#[test]
fn matched_exports_remeasure_equal_and_frozen_groups_do_not_chase() {
    let tmp = Scratch::new();
    let mut s = small();
    s.prepared = true;
    s.channels[1].group = "kick".into();
    s.groups.truncate(1);
    s.groups[0].trim_db = 6.;
    tmp.wav("01_Kick.wav", 1, 44100, |i, _| {
        0.03 * (2. * PI * 150. * i as f64 / 44100.).sin()
    });
    tmp.wav("02_Snare.wav", 1, 44100, |i, _| {
        0.02 * (2. * PI * 1000. * i as f64 / 44100.).sin()
    });
    run(s, &tmp.0, &tmp.0.join("show"), None).unwrap();
    let mut values = Vec::new();
    for stem in ["bypass", "processed"] {
        let a = samples(&tmp.0.join(format!("show/{stem}-matched.wav")));
        let mut loud = Loudness::new(44100);
        for frame in a.chunks_exact(2) {
            loud.add([frame[0] as f64 / 8388608., frame[1] as f64 / 8388608.]);
        }
        values.push(loud.integrated().unwrap());
    }
    assert!((values[0] - values[1]).abs() < 0.01);
    assert!((values[0] + 23.).abs() < 0.01);
    let history = std::fs::read_to_string(tmp.0.join("show/gain-history.csv")).unwrap();
    for line in history.lines().skip(1) {
        let fields: Vec<_> = line.split(',').collect();
        assert_eq!(fields[7], "6.0000");
        assert_eq!(fields[8], "6.0000");
        assert_eq!(fields[10], "true");
    }
}

#[test]
fn clipped_pcm_and_nonfinite_float_are_handled_explicitly() {
    let tmp = Scratch::new();
    let mut s = small();
    s.channels.truncate(1);
    s.groups.truncate(1);
    let path = tmp.0.join("01_Kick.wav");
    let mut w = hound::WavWriter::create(
        &path,
        hound::WavSpec {
            channels: 1,
            sample_rate: 44100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        },
    )
    .unwrap();
    for i in 0..4410 {
        w.write_sample(if i % 2 == 0 { i16::MAX } else { i16::MIN })
            .unwrap();
    }
    w.finalize().unwrap();
    run(s.clone(), &tmp.0, &tmp.0.join("pcm"), Some(0.1)).unwrap();
    let r: serde_json::Value =
        serde_json::from_reader(std::fs::File::open(tmp.0.join("pcm/measurements.json")).unwrap())
            .unwrap();
    assert_eq!(
        r["channels"][0]["input"]["clipped_or_full_scale_samples"],
        4410
    );
    assert!(run(s.clone(), &tmp.0, &tmp.0.join("tiny"), Some(1e-9)).is_err());
    assert!(!tmp.0.join("tiny").exists());
    let mut w = hound::WavWriter::create(
        &path,
        hound::WavSpec {
            channels: 1,
            sample_rate: 44100,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        },
    )
    .unwrap();
    w.write_sample(f32::NAN).unwrap();
    w.finalize().unwrap();
    assert!(run(s, &tmp.0, &tmp.0.join("nan"), Some(1. / 44100.)).is_err());
    assert!(!tmp.0.join("nan/report.txt").exists());
}
#[test]
fn cli_prepares_renders_and_rejects_overwrite() {
    use std::process::Command;
    let tmp = Scratch::new();
    let binary = env!("CARGO_BIN_EXE_gigpies");
    let config = tmp.0.join("settings.json");
    assert!(
        Command::new(binary)
            .arg("preset")
            .arg(&config)
            .status()
            .unwrap()
            .success()
    );
    let before = std::fs::read(&config).unwrap();
    assert!(
        !Command::new(binary)
            .arg("preset")
            .arg(&config)
            .output()
            .unwrap()
            .status
            .success()
    );
    assert_eq!(before, std::fs::read(&config).unwrap());
    std::fs::write(&config, serde_json::to_vec(&small()).unwrap()).unwrap();
    for file in ["01_Kick.wav", "02_Snare.wav"] {
        tmp.wav(file, 1, 4410, |i, _| 0.01 * (i as f64).sin());
    }
    assert!(
        Command::new(binary)
            .arg("soundcheck")
            .arg(&config)
            .arg(&tmp.0)
            .arg(tmp.0.join("check"))
            .arg("0.1")
            .status()
            .unwrap()
            .success()
    );
    assert!(
        Command::new(binary)
            .arg("render")
            .arg(tmp.0.join("check/prepared.json"))
            .arg(&tmp.0)
            .arg(tmp.0.join("show"))
            .status()
            .unwrap()
            .success()
    );
    assert!(tmp.0.join("show/processed-matched.wav").exists());
}
