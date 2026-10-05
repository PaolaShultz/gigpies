#![cfg(feature = "hardware-host")]
use gigpies::host::{
    adapters::{Dsp, Recorder},
    brain_fx::{BrainFx, FxError},
    pa_v2::Pa,
};
use std::path::Path;

fn manifest() -> gigpies::module_graph::Manifest {
    gigpies::module_graph::Manifest::load(Path::new(
        &std::env::var("GP05_MANIFEST").expect("explicit verified module manifest"),
    ))
    .unwrap()
}
#[test]
#[ignore = "requires explicitly selected actual PA v2 library and owner-produced fixture directory"]
fn actual_loaded_pa_v2_retirement_fault_and_explicit_rearm() {
    let m = manifest();
    let fixtures = std::path::PathBuf::from(std::env::var("GP_PA_V2_FIXTURES").unwrap());
    let three = std::fs::read(fixtures.join("stereo3way.json")).unwrap();
    let four = std::fs::read(fixtures.join("stereo4way.json")).unwrap();
    let mut pa = Pa::load(&m.pa.library, &three, 9, 0).unwrap();
    assert_eq!((pa.input_channels(), pa.output_channels()), (2, 6));
    assert_eq!(pa.status().unwrap().quiesced, 1);
    let input = vec![0.001; 96];
    let mut output = vec![0.; 288];
    assert_eq!(pa.process(&input, &mut output, 9, 0), 0);
    assert!(output.iter().all(|v| *v == 0.));
    assert_eq!(pa.rearm(9, 48), 0);
    for frame in (48..528).step_by(48) {
        assert_eq!(pa.process(&input, &mut output, 9, frame), 0);
    }
    assert!(output.iter().any(|v| v.abs() > 1e-7));
    let mut candidate = pa.prepare(&four).unwrap();
    assert_ne!(pa.commit(&mut candidate, 9, 528), 0);
    assert_eq!(pa.output_channels(), 6);
    assert_eq!(pa.mute(), 0);
    for frame in (528..1056).step_by(48) {
        assert_eq!(pa.process(&input, &mut output, 9, frame), 0);
    }
    assert_eq!(pa.status().unwrap().quiesced, 1);
    assert_eq!(pa.commit(&mut candidate, 9, 1056), 0);
    assert_eq!(pa.output_channels(), 8);
    let mut another = pa.prepare(&three).unwrap();
    assert_eq!(pa.commit(&mut another, 9, 1056), -3);
    pa.retire();
    let mut output = vec![1.; 384];
    assert_eq!(pa.process(&input, &mut output, 9, 1056), 0);
    assert!(output.iter().all(|v| *v == 0.));
    assert_eq!(pa.rearm(9, 1104), 0);
    assert_ne!(pa.process(&input, &mut output, 8, 1104), 0);
    assert!(output.iter().all(|v| *v == 0.));
    assert_eq!(pa.status().unwrap().fault_latched, 1);
}
#[test]
#[ignore = "requires explicitly selected actual SHR FX library"]
fn actual_brain_fx_bank_preserves_each_source_and_resets_stale_tails() {
    let m = manifest();
    for channels in [2, 16, 32, 48, 17] {
        let mut bank = BrainFx::prepare(&m.fx.library, channels, 48, 9).unwrap();
        let mut refs: Vec<Dsp> = (0..channels.div_ceil(2))
            .map(|_| Dsp::load(&m.fx.library, "fx", 48).unwrap())
            .collect();
        let mut input = vec![0.; 48 * channels];
        let mut output = input.clone();
        let mut pair = [0.; 96];
        let mut expected = [0.; 96];
        for block in 0..24 {
            input.fill(0.);
            if block == 0 {
                for (ch, s) in input[..channels].iter_mut().enumerate() {
                    *s = (ch + 1) as f64 / 1024.;
                }
            }
            bank.process(9, block * 48, &input, &mut output).unwrap();
            for (i, reference) in refs.iter_mut().enumerate() {
                for f in 0..48 {
                    pair[f * 2] = input[f * channels + i * 2];
                    pair[f * 2 + 1] = if i * 2 + 1 < channels {
                        input[f * channels + i * 2 + 1]
                    } else {
                        0.
                    };
                }
                assert_eq!(reference.process_result(&pair, &mut expected, 2), 0);
                for f in 0..48 {
                    assert_eq!(output[f * channels + i * 2], expected[f * 2]);
                    if i * 2 + 1 < channels {
                        assert_eq!(output[f * channels + i * 2 + 1], expected[f * 2 + 1]);
                    }
                }
            }
        }
        assert_eq!(
            bank.process(9, 0, &input, &mut output),
            Err(FxError::StaleFrame)
        );
        bank.reset_session(10).unwrap();
        input.fill(0.);
        bank.process(10, 0, &input, &mut output).unwrap();
        assert!(output.iter().all(|v| *v == 0.));
        assert_eq!(bank.intentional_delay_frames(), 960);
    }
}
#[test]
#[ignore = "requires explicit SHR REC library; creates and removes only synthetic task-owned takes"]
fn actual_recorder_preserves_all_configured_raw_tracks() {
    let m = manifest();
    let root = std::env::temp_dir().join(format!("gp0014-raw-{}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    for channels in [16, 32, 48, 17] {
        let path = root.join(format!("take-{channels}"));
        let mut rec = Recorder::create_configured(&m.rec.library, &path, 48, 9, channels).unwrap();
        let observer = rec.observer().unwrap();
        let samples: Vec<f64> = (0..48 * channels)
            .map(|n| (n as i32 - 1000) as f64 / 8388608.)
            .collect();
        assert_eq!(rec.push(96, &samples), 0);
        assert_eq!(rec.finish(), 0);
        let p = observer.snapshot().unwrap();
        assert_eq!(p.written_frames, 48);
        assert_eq!(p.accepted_frames, 48);
        let mut stems: Vec<_> = std::fs::read_dir(&path)
            .unwrap()
            .map(|p| p.unwrap().path())
            .filter(|p| p.extension().is_some_and(|s| s == "wav"))
            .collect();
        stems.sort();
        assert_eq!(stems.len(), channels);
        for (ch, path) in stems.iter().enumerate() {
            let mut wav = hound::WavReader::open(path).unwrap();
            let actual: Vec<i32> = wav.samples::<i32>().map(Result::unwrap).collect();
            assert_eq!(actual.len(), 48);
            for (frame, sample) in actual.iter().enumerate() {
                assert_eq!(*sample, (frame * channels + ch) as i32 - 1000);
            }
        }
    }
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn recorder_admission_refuses_before_loading_or_creating() {
    let absent = Path::new("/definitely-absent-gigpies-test-owner");
    for (channels, block, reason) in [
        (65, 48, "channels"),
        (48, 8193, "block"),
        (48, 8192, "64MiB"),
    ] {
        let error = Recorder::create_configured(absent, absent, block, 1, channels)
            .err()
            .unwrap()
            .to_string();
        assert!(error.contains(reason), "{error}");
    }
}
