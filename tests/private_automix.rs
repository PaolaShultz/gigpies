//! Explicit local audition only; original copyrighted media is never used by CI.
use gigpies::automix::{config::example, run};
use std::path::PathBuf;
#[test]
#[ignore = "requires local private media; writes ignored experiment artifacts"]
fn deterministic_source_variations() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out = std::env::var_os("GIGPIES_VARIATIONS_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("artifacts/automix/robustness"));
    std::fs::create_dir(&out).expect("use a new GIGPIES_VARIATIONS_OUT directory");
    let mut session = example();
    session.channels.truncate(2);
    session.groups.truncate(2);
    let rate = 44100;
    let n = rate * 8;
    let material: Vec<Vec<f64>> = session
        .channels
        .iter()
        .map(|ch| {
            hound::WavReader::open(
                root.join("recordings/sessions/complainiacs-etc")
                    .join(&ch.file),
            )
            .unwrap()
            .samples::<i32>()
            .skip(rate * 30)
            .take(n)
            .map(|x| x.unwrap() as f64 / 8388608.)
            .collect()
        })
        .collect();
    assert!(material.iter().all(|m| m.len() == n));
    for case in ["low", "high", "silence", "pauses", "jump", "clipped"] {
        let src = out.join(format!("{case}-sources"));
        std::fs::create_dir(&src).unwrap();
        for (ch, data) in session.channels.iter().zip(&material) {
            let mut w = hound::WavWriter::create(
                src.join(&ch.file),
                hound::WavSpec {
                    channels: 1,
                    sample_rate: rate as u32,
                    bits_per_sample: 24,
                    sample_format: hound::SampleFormat::Int,
                },
            )
            .unwrap();
            for (i, x) in data.iter().enumerate() {
                let value = match case {
                    "low" => x * 0.01,
                    "high" => x * 3.,
                    "silence" => 0.,
                    "pauses" => {
                        if (rate * 2..rate * 5).contains(&i) {
                            0.
                        } else {
                            *x
                        }
                    }
                    "jump" => x * if i < rate * 4 { 0.03 } else { 4. },
                    "clipped" => x * 30.,
                    _ => unreachable!(),
                };
                w.write_sample((value.clamp(-1., 1. - 1. / 8388608.) * 8388608.).round() as i32)
                    .unwrap();
            }
            w.finalize().unwrap();
        }
        let dest = out.join(case);
        run(session.clone(), &src, &dest, Some(8.)).unwrap();
        let saved: gigpies::automix::config::Session =
            serde_json::from_reader(std::fs::File::open(dest.join("prepared.json")).unwrap())
                .unwrap();
        saved.validate().unwrap();
        assert!(
            saved
                .groups
                .iter()
                .all(|g| (-30. ..=18.).contains(&g.trim_db))
        );
        let report: serde_json::Value =
            serde_json::from_reader(std::fs::File::open(dest.join("measurements.json")).unwrap())
                .unwrap();
        assert!(report["processed_export_peak_dbfs"].as_f64().unwrap() <= -1.99);
        if case == "silence" {
            assert!(saved.groups.iter().all(|g| g.trim_db == 0.));
            assert!(report["matched_target_lufs"].is_null());
        }
        if case == "clipped" {
            assert!(
                report["channels"][0]["input"]["clipped_or_full_scale_samples"]
                    .as_u64()
                    .unwrap()
                    > 0
            );
        }
    }
}

#[test]
#[ignore = "remeasures full-song local listening files after the documented render"]
fn full_song_listening_files() {
    use gigpies::automix::dsp::{Loudness, Meter};
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("artifacts/automix/show");
    let mut loudness = Vec::new();
    for stem in ["bypass", "processed", "bypass-matched", "processed-matched"] {
        let mut r = hound::WavReader::open(out.join(format!("{stem}.wav"))).unwrap();
        assert_eq!(r.duration(), 5038080);
        assert_eq!(r.spec().sample_rate, 44100);
        let mut loud = Loudness::new(44100);
        let mut meter = Meter::default();
        let mut frame = [0.; 2];
        for (i, s) in r.samples::<i32>().enumerate() {
            let v = s.unwrap() as f64 / 8388608.;
            meter.add(v);
            frame[i % 2] = v;
            if i % 2 == 1 {
                loud.add(frame);
            }
        }
        assert_eq!(meter.clipped_samples, 0);
        let l = loud.integrated().unwrap();
        println!(
            "{stem}: {} dBFS, {l} LUFS",
            gigpies::automix::dsp::db(meter.peak)
        );
        loudness.push(l);
    }
    assert!((loudness[2] - loudness[3]).abs() < 0.05);
    assert!((loudness[2] + 23.).abs() < 0.05);
}

#[test]
#[ignore = "requires completed local automatic FX pass; remeasures private exports"]
fn fx_pass_listening_files() {
    use gigpies::automix::dsp::{Loudness, Meter, db};
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("artifacts/automix/fx-pass");
    let mut levels = Vec::new();
    for path in [
        "compare-with-first/previous-matched.wav",
        "compare-with-first/new-matched.wav",
        "automatic-final/final/processed-matched.wav",
    ] {
        let mut reader = hound::WavReader::open(root.join(path)).unwrap();
        assert_eq!(reader.duration(), 5214480);
        assert_eq!(reader.spec().sample_rate, 44100);
        assert_eq!(reader.spec().channels, 2);
        let mut loud = Loudness::new(44100);
        let mut meter = Meter::default();
        let mut frame = [0.; 2];
        for (i, sample) in reader.samples::<i32>().enumerate() {
            let v = sample.unwrap() as f64 / 8388608.;
            meter.add(v);
            frame[i % 2] = v;
            if i % 2 == 1 {
                loud.add(frame);
            }
        }
        assert_eq!(meter.clipped_samples, 0);
        assert!(db(meter.peak) < -1.99);
        let level = loud.integrated().unwrap();
        println!("{path}: {:.3} LUFS, {:.3} dBFS", level, db(meter.peak));
        levels.push(level);
    }
    assert!((levels[0] + 23.).abs() < 0.05);
    assert!((levels[1] - levels[0]).abs() < 0.05);
    assert!((levels[2] + 20.5).abs() < 0.05);
    let report: serde_json::Value = serde_json::from_reader(
        std::fs::File::open(root.join("automatic-final/review-result.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(report["within_reduction_budget"], true);
    assert_eq!(report["algorithm"], "bounded_review_v1");
}
