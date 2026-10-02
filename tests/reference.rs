use gigpies::automix::reference::{self, Frame};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static ID: AtomicU64 = AtomicU64::new(0);
fn frames(seed: u64) -> Vec<Frame> {
    let mut x = seed;
    (0..2400)
        .map(|i| {
            x = x.wrapping_mul(6364136223846793005).wrapping_add(1);
            let level = -35. + 20. * ((x >> 32) as f64 / u32::MAX as f64);
            Frame {
                seconds: i as f64 / 50.,
                rms_dbfs: level,
                peak_dbfs: level + 6.,
                band_power: [0.001; 5],
                left_power: 0.01,
                right_power: 0.01,
                cross_power: 0.01,
            }
        })
        .collect()
}
fn shifted(a: &[Frame], count: usize) -> Vec<Frame> {
    let mut pad = a[0].clone();
    pad.rms_dbfs = -120.;
    let mut b = vec![pad; count];
    b.extend_from_slice(a);
    for (i, x) in b.iter_mut().enumerate() {
        x.seconds = i as f64 / 50.;
    }
    b
}
#[test]
fn alignment_is_gain_invariant_and_handles_leading_or_removed_audio() {
    let a = frames(42);
    let mut b = shifted(&a, 100);
    for x in &mut b {
        x.rms_dbfs -= 8.;
    }
    let r = reference::align(&a, &b);
    assert_eq!(r.status, "consistent_offset");
    assert_eq!(r.reference_offset_seconds, Some(2.));
    assert!(r.anchors.iter().all(|a| a.confident));
    let r = reference::align(&a, &a[100..]);
    assert_eq!(r.status, "consistent_offset");
    assert_eq!(r.reference_offset_seconds, Some(-2.));
}
#[test]
fn silence_quiet_input_repetition_and_unrelated_audio_do_not_align() {
    let a = frames(5);
    let mut b = frames(99);
    assert_eq!(reference::align(&a, &b).status, "insufficient_confidence");
    for level in [-120., -80.] {
        for x in &mut b {
            x.rms_dbfs = level;
        }
        assert_eq!(reference::align(&b, &b).status, "insufficient_confidence");
    }
    for (i, x) in b.iter_mut().enumerate() {
        x.rms_dbfs = if i % 25 < 3 { -10. } else { -30. };
    }
    assert_eq!(reference::align(&b, &b).status, "insufficient_confidence");
}
#[test]
fn edits_drift_and_search_boundary_are_reported_without_global_offset() {
    let a = frames(37);
    let mut b = shifted(&a, 100);
    b.splice(1300..1300, vec![a[0].clone(); 100]);
    let r = reference::align(&a, &b);
    assert_eq!(r.status, "inconsistent_timeline");
    assert!(r.reference_offset_seconds.is_none());
    let mut b = vec![];
    for chunk in a.chunks(600) {
        b.extend_from_slice(chunk);
        b.extend_from_slice(&chunk[..4]);
    }
    assert_eq!(reference::align(&a, &b).status, "inconsistent_timeline");
    assert_eq!(
        reference::align(&a, &shifted(&a, 1000)).status,
        "insufficient_confidence"
    );
    assert_eq!(
        reference::align(&a[..300], &a[..300]).status,
        "insufficient_confidence"
    );
}
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "gigpies-reference-{}-{}",
            std::process::id(),
            ID.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&p).unwrap();
        Self(p)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn audio(path: &std::path::Path, rate: u32, delay: usize, opposed: bool) {
    let levels = frames(42);
    let mut w = hound::WavWriter::create(
        path,
        hound::WavSpec {
            channels: 2,
            sample_rate: rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        },
    )
    .unwrap();
    for k in 0..(48 + delay) * rate as usize {
        let x = if k < delay * rate as usize {
            0.
        } else {
            let j = k - delay * rate as usize;
            let env = 10_f64.powf(levels[(j * 50 / rate as usize).min(2399)].rms_dbfs / 20.);
            env * (std::f64::consts::TAU * 1600. * j as f64 / rate as f64).sin()
        };
        w.write_sample((x * 32767.) as i16).unwrap();
        w.write_sample((x * if opposed { -32767. } else { 32767. }) as i16)
            .unwrap();
    }
    w.finalize().unwrap();
}
#[test]
fn native_rate_stereo_comparison_preserves_inputs_and_exact_excerpt_samples() {
    let t = Temp::new();
    let a = t.0.join("ours.wav");
    let b = t.0.join("reference.wav");
    audio(&a, 8000, 0, false);
    audio(&b, 16000, 2, true);
    let originals = [std::fs::read(&a).unwrap(), std::fs::read(&b).unwrap()];
    let out = t.0.join("review");
    reference::run(&a, &b, &out, Some(12.)).unwrap();
    let report: serde_json::Value =
        serde_json::from_slice(&std::fs::read(out.join("review.json")).unwrap()).unwrap();
    assert_eq!(report["alignment"]["status"], "consistent_offset");
    assert_eq!(report["excerpts"].as_array().unwrap().len(), 2);
    assert_eq!(report["settings_written"], false);
    assert_eq!(report["gain_changes"], false);
    assert!(
        report["sections"][0]["reference_minus_ours"]["rms_db"]
            .as_f64()
            .unwrap()
            .abs()
            < 0.1
    );
    assert!(report["sections"][0]["processing_recommendation"].is_null());
    assert!(
        report["sections"][0]["reference"]["stereo_correlation"]
            .as_f64()
            .unwrap()
            < -0.99
    );
    for (source, name, start) in [
        (&a, "01-OUR-MIX.wav", 12),
        (&b, "02-SUPPLIED-REFERENCE.wav", 14),
    ] {
        let mut r = hound::WavReader::open(source).unwrap();
        let rate = r.spec().sample_rate;
        r.seek(start * rate).unwrap();
        let expected = r
            .samples::<i16>()
            .take(12 * rate as usize * 2)
            .map(Result::unwrap)
            .collect::<Vec<_>>();
        let mut r = hound::WavReader::open(out.join(name)).unwrap();
        assert_eq!(r.spec().sample_rate, rate);
        assert_eq!(
            r.samples::<i16>().map(Result::unwrap).collect::<Vec<_>>(),
            expected
        );
    }
    assert_eq!(std::fs::read(a.clone()).unwrap(), originals[0]);
    assert_eq!(std::fs::read(b.clone()).unwrap(), originals[1]);
    assert!(reference::run(&a, &b, &out, Some(12.)).is_err());
    assert!(reference::run(&a, &a, &t.0.join("same"), None).is_err());
    assert!(reference::run(&a, &b, &t.0.join("nan"), Some(f64::NAN)).is_err());
}
#[test]
fn invalid_float_and_excessive_duration_fail_without_output() {
    let t = Temp::new();
    let p = t.0.join("invalid.wav");
    let mut w = hound::WavWriter::create(
        &p,
        hound::WavSpec {
            channels: 1,
            sample_rate: 8000,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        },
    )
    .unwrap();
    w.write_sample(f32::NAN).unwrap();
    w.finalize().unwrap();
    assert!(reference::measure(&p, 4000.).is_err());
    assert!(reference::measure(&p, f64::INFINITY).is_err());
    // A sparse payload exercises the duration budget without processing ten minutes.
    use std::io::{Seek, SeekFrom, Write};
    let long = t.0.join("long.wav");
    let mut w = hound::WavWriter::create(
        &long,
        hound::WavSpec {
            channels: 1,
            sample_rate: 8000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        },
    )
    .unwrap();
    w.write_sample(0_i16).unwrap();
    w.finalize().unwrap();
    let header = std::fs::read(&long).unwrap();
    let data = header.windows(4).position(|x| x == b"data").unwrap();
    let bytes = 601_u32 * 8000 * 2;
    let size = data as u64 + 8 + bytes as u64;
    let mut file = std::fs::OpenOptions::new().write(true).open(&long).unwrap();
    file.set_len(size).unwrap();
    file.seek(SeekFrom::Start(4)).unwrap();
    file.write_all(&((size - 8) as u32).to_le_bytes()).unwrap();
    file.seek(SeekFrom::Start(data as u64 + 4)).unwrap();
    file.write_all(&bytes.to_le_bytes()).unwrap();
    drop(file);
    assert!(reference::measure(&long, 4000.).is_err());
}
