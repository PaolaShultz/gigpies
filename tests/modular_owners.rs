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

#[test]
#[ignore = "explicit hash-verified PA/FX/REC libraries and owner PA fixtures; software only"]
fn actual_monitor_taps_feed_owner_matrix_after_send_gain_without_reprocessing_foh() {
    use gigpies::{
        channel_processing::Config, mixer_control::OfflineEngine, module_graph::ModuleGraph,
        sends_wire::Tap, show::Counter, topology::EngineTopology,
    };
    let manifest = manifest();
    let root = std::path::PathBuf::from(std::env::var("GP_PA_V2_FIXTURES").unwrap());
    let json = std::fs::read(root.join("matrix4x8.json")).unwrap();
    let mut engine = OfflineEngine::with_topology(
        "11111111-1111-4111-8111-111111111111",
        Counter(7),
        Counter(0),
        0,
        EngineTopology::software(17, 3, 8).unwrap(),
    )
    .unwrap();
    let mut intent = engine.persisted_intent().unwrap();
    intent.processing[0] = Config {
        eq_bypass: false,
        band3_gain_mdb: 6000,
        compressor_bypass: false,
        threshold_mdb: -24000,
        ratio_milli: 4000,
        attack_us: 100,
        ..Default::default()
    };
    intent.send_taps.as_mut().unwrap()[0][0] = Tap::ProcessedPreFader;
    intent.send_taps.as_mut().unwrap()[0][1] = Tap::ProcessedPostFader;
    let mut actual = gigpies::mixer::Mixer::from_topology(0, intent.topology.clone()).unwrap();
    actual
        .restore_intent(&intent.parameters, &intent.processing)
        .unwrap();
    actual
        .restore_sends(intent.send_taps.as_ref().unwrap())
        .unwrap();
    intent.send_taps.as_mut().unwrap()[0].fill(Tap::RawPostMute);
    let mut legacy = gigpies::mixer::Mixer::from_topology(0, intent.topology.clone()).unwrap();
    legacy
        .restore_intent(&intent.parameters, &intent.processing)
        .unwrap();
    legacy
        .restore_sends(intent.send_taps.as_ref().unwrap())
        .unwrap();
    actual.rearm().unwrap();
    legacy.rearm().unwrap();
    let mut graph = ModuleGraph::load_configured(manifest.clone(), 7, 0, 17).unwrap();
    let mut prepared = graph.prepare_pa_change(&json, vec![0, 1, 2, 3], 5).unwrap();
    assert_eq!(graph.commit_pa_change(&mut prepared, 7, 0), 0);
    graph.retire_pa_changes();
    graph.rearm_pa().unwrap();
    let mut pa = Pa::load(&manifest.pa.library, &json, 7, 0).unwrap();
    assert_eq!(pa.rearm(7, 0), 0);
    let mut fx = Dsp::load(&manifest.fx.library, "fx", 48).unwrap();
    let mut raw = vec![0.; 17 * 48];
    let mut buses = vec![0.; 5 * 48];
    let mut original = vec![0.; 5 * 48];
    let mut pa_input = vec![0.; 4 * 48];
    let mut expected = vec![0.; 8 * 48];
    let mut foh = [0.; 96];
    let mut wet = [0.; 96];
    let mut difference = false;
    for block in 0..32u64 {
        for f in 0..48 {
            let frame = block * 48 + f as u64;
            raw[f * 17] = 0.25 * (std::f64::consts::TAU * 2000. * frame as f64 / 48000.).sin();
        }
        actual.process_interleaved(&raw, &mut buses).unwrap();
        legacy.process_interleaved(&raw, &mut original).unwrap();
        for f in 0..48 {
            assert_eq!(&buses[f * 5..f * 5 + 2], &original[f * 5..f * 5 + 2]);
            foh[f * 2] = buses[f * 5];
            foh[f * 2 + 1] = buses[f * 5 + 1];
        }
        assert_eq!(fx.process_result(&foh, &mut wet, 2), 0);
        for f in 0..48 {
            pa_input[f * 4] = buses[f * 5] + wet[f * 2];
            pa_input[f * 4 + 1] = buses[f * 5 + 1] + wet[f * 2 + 1];
            pa_input[f * 4 + 2] = buses[f * 5 + 2];
            pa_input[f * 4 + 3] = buses[f * 5 + 3];
        }
        graph
            .process_interleaved_brain(7, block * 48, &raw, &buses, 5, None, None)
            .unwrap();
        assert_eq!(pa.process(&pa_input, &mut expected, 7, block * 48), 0);
        assert_eq!(graph.output_interleaved(), expected);
        if block > 12 {
            difference |= buses
                .iter()
                .zip(&original)
                .any(|(a, b)| (a - b).abs() > 1e-6);
        }
    }
    assert!(
        difference,
        "actual PA matrix must receive changed monitor signals"
    );
}
