//! Production Brain render acceptance. Owner-library cases are opt-in because
//! trusted independently built artifacts are explicit inputs, never downloaded.
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};
thread_local! {static EVENTS:Cell<Option<usize>>=const{Cell::new(None)};}
struct Guard;
unsafe impl GlobalAlloc for Guard {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        EVENTS.with(|v| {
            if let Some(n) = v.get() {
                v.set(Some(n + 1));
            }
        });
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        EVENTS.with(|v| {
            if let Some(n) = v.get() {
                v.set(Some(n + 1));
            }
        });
        unsafe { System.dealloc(p, l) }
    }
}
#[global_allocator]
static ALLOCATOR: Guard = Guard;
fn guarded<T>(f: impl FnOnce() -> T) -> T {
    EVENTS.with(|v| v.set(Some(0)));
    let result = f();
    let count = EVENTS.with(|v| v.replace(None).unwrap());
    assert_eq!(count, 0, "render allocation or last-owner destruction");
    result
}
#[test]
fn mixer_all_operator_taps_preserve_output_and_allocate_nothing_16_32_48() {
    use gigpies::{brain_control::MonitorSource, mixer::Mixer, topology::EngineTopology};
    for channels in [16, 32, 48] {
        let topology = EngineTopology::software(channels, 5, 0).unwrap();
        let mut reference = Mixer::from_topology(0, topology.clone()).unwrap();
        let mut actual = Mixer::from_topology(0, topology).unwrap();
        let input: Vec<_> = (0..channels * 48)
            .map(|i| (i % channels + 1) as f64 / 1024.)
            .collect();
        let mut a = vec![0.; 48 * 7];
        let mut b = a.clone();
        for source in [
            MonitorSource::None,
            MonitorSource::Main,
            MonitorSource::Monitor { index: 4 },
            MonitorSource::Pfl {
                input: channels - 1,
            },
            MonitorSource::Afl {
                input: channels - 1,
            },
        ] {
            actual.set_operator_tap(source);
            guarded(|| actual.process_interleaved(&input, &mut a)).unwrap();
            guarded(|| reference.process_interleaved(&input, &mut b)).unwrap();
            assert_eq!(a, b);
            if matches!(
                source,
                MonitorSource::Pfl { .. } | MonitorSource::Afl { .. }
            ) {
                let expected = (channels as f64 / 1024.)
                    * std::f64::consts::FRAC_1_SQRT_2
                    * if matches!(source, MonitorSource::Afl { .. }) {
                        10_f64.powf(-6. / 20.)
                    } else {
                        1.
                    };
                assert!(
                    actual
                        .operator_tap()
                        .iter()
                        .all(|v| (*v - expected).abs() < 1e-14)
                );
            }
        }
    }
}
#[cfg(feature = "hardware-host")]
mod owners {
    use super::*;
    use gigpies::{
        host::{adapters::Dsp, pa_v2::Pa},
        module_graph::{Manifest, ModuleGraph},
        show::Counter,
    };
    use std::{
        os::unix::fs::PermissionsExt,
        path::{Path, PathBuf},
        time::{Duration, Instant},
    };
    const SHOW: &str = "11111111-1111-4111-8111-111111111111";
    struct Directory(PathBuf);
    impl Directory {
        fn new(label: &str) -> Self {
            let p =
                std::env::temp_dir().join(format!("gp15-owners-{}-{label}", std::process::id()));
            std::fs::create_dir(&p).unwrap();
            std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o700)).unwrap();
            Self(p)
        }
    }
    impl Drop for Directory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    fn manifest_path() -> PathBuf {
        std::env::var("GP05_MANIFEST")
            .expect("explicit verified owner manifest")
            .into()
    }
    fn manifest() -> Manifest {
        Manifest::load(&manifest_path()).unwrap()
    }
    fn pa_fixture(name: &str) -> Vec<u8> {
        std::fs::read(
            Path::new(&std::env::var("GP14_PA_FIXTURES").expect("explicit PA provider fixtures"))
                .join(format!("{name}.json")),
        )
        .unwrap()
    }
    fn limited_fixture(name: &str) -> Vec<u8> {
        let mut v: serde_json::Value = serde_json::from_slice(&pa_fixture(name)).unwrap();
        for out in v["outputs"].as_array_mut().unwrap() {
            out["processing"]["limiter_db"] = serde_json::json!(-24.);
        }
        serde_json::to_vec(&v).unwrap()
    }
    fn wait_recording(g: &mut ModuleGraph, state: &str) {
        let start = Instant::now();
        loop {
            g.poll_lifecycle().unwrap();
            let status = g.status(SHOW, 0).unwrap().recording.unwrap();
            if status.state == state {
                return;
            }
            assert!(
                start.elapsed() < Duration::from_secs(3),
                "REC expected {state}, observed {status:?}"
            );
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    #[test]
    #[ignore = "explicit hash-verified PA/FX/REC libraries and owner PA fixture directory"]
    fn actual_graph_foh_protection_raw_recording_fx_isolation_and_render_guards() {
        let manifest = manifest();
        let json = limited_fixture("matrix4x8");
        let root = Directory::new("graph");
        for channels in [16, 32, 48] {
            let mut graph = ModuleGraph::load_configured(manifest.clone(), 7, 0, channels).unwrap();
            let map = vec![0, 1, 2, 6];
            let mut prepared = graph.prepare_pa_change(&json, map.clone(), 7).unwrap();
            assert_eq!(guarded(|| graph.commit_pa_change(&mut prepared, 7, 0)), 0);
            graph.retire_pa_changes();
            graph.rearm_pa().unwrap();
            let mut pa = Pa::load(&manifest.pa.library, &json, 7, 0).unwrap();
            assert_eq!(pa.rearm(7, 0), 0);
            let mut fx = Dsp::load(&manifest.fx.library, "fx", 48).unwrap();
            let take = format!("take-{channels}");
            graph.start(&root.0, &take, Counter(1)).unwrap();
            wait_recording(&mut graph, "recording");
            let mut raw = vec![0.; channels * 48];
            let mut mixed = vec![0.; 7 * 48];
            let mut tb = vec![0.; 7 * 48];
            let mut fx_input = [0.; 96];
            let mut wet = [0.; 96];
            let mut pa_input = vec![0.; 4 * 48];
            let mut expected = vec![0.; 8 * 48];
            let mut expected_raw = Vec::new();
            let mut saw_protected = false;
            let mut saw_wet = false;
            for block in 0..64u64 {
                for f in 0..48 {
                    let frame = block * 48 + f as u64;
                    for ch in 0..channels {
                        let pcm = ((frame as usize * 31 + ch * 7) % 4096) as i32 - 2048;
                        raw[f * channels + ch] = pcm as f64 / 8_388_608.;
                        expected_raw.push(pcm);
                    }
                    for bus in 0..7 {
                        mixed[f * 7 + bus] =
                            ((frame as f64 * (bus + 1) as f64 * 0.023).sin()) * 0.02;
                    }
                    fx_input[f * 2] = mixed[f * 7];
                    fx_input[f * 2 + 1] = mixed[f * 7 + 1];
                    tb[f * 7] = if (frame / 24).is_multiple_of(2) {
                        0.8
                    } else {
                        -0.8
                    };
                    tb[f * 7 + 1] = -tb[f * 7];
                    tb[f * 7 + 2] = 0.4;
                    tb[f * 7 + 6] = -0.3;
                }
                assert_eq!(guarded(|| fx.process_result(&fx_input, &mut wet, 2)), 0);
                saw_wet |= wet.iter().any(|v| v.abs() > 1e-9);
                for f in 0..48 {
                    for (ch, &bus) in map.iter().enumerate() {
                        pa_input[f * 4 + ch] = (mixed[f * 7 + bus]
                            + if bus < 2 { wet[f * 2 + bus] } else { 0. })
                            + tb[f * 7 + bus];
                    }
                }
                guarded(|| {
                    graph.process_interleaved_brain(7, block * 48, &raw, &mixed, 7, None, Some(&tb))
                })
                .unwrap();
                assert_eq!(
                    guarded(|| pa.process(&pa_input, &mut expected, 7, block * 48)),
                    0
                );
                assert_eq!(
                    graph.output_interleaved(),
                    expected,
                    "independent input assembly must feed actual protected owner"
                );
                for f in 0..48 {
                    for c in 0..2 {
                        assert!(
                            (graph.program_before_talkback()[f * 2 + c]
                                - (fx_input[f * 2 + c] + wet[f * 2 + c]))
                                .abs()
                                < 1e-14
                        );
                    }
                }
                saw_protected |= expected.iter().any(|v| v.abs() > 0.005);
                // Owner limiter is a bounded peak protector, independently computed threshold.
                let ceiling = 10_f64.powf(-24. / 20.);
                assert!(expected.iter().all(|v| v.abs() <= ceiling + 1e-12));
            }
            assert!(saw_protected);
            assert!(saw_wet, "test must exercise actual wet return");
            graph.stop(&take, Counter(1)).unwrap();
            wait_recording(&mut graph, "finalized");
            let status = graph.status(SHOW, 0).unwrap().recording.unwrap();
            assert_eq!(status.dropped_frames, Counter(0));
            assert_eq!(status.written_frames, Counter(64 * 48));
            let mut stems: Vec<_> = std::fs::read_dir(root.0.join(&take))
                .unwrap()
                .map(|e| e.unwrap().path())
                .filter(|p| p.extension().is_some_and(|e| e == "wav"))
                .collect();
            stems.sort();
            assert_eq!(stems.len(), channels);
            for (ch, path) in stems.iter().enumerate() {
                let mut wav = hound::WavReader::open(path).unwrap();
                let actual: Vec<i32> = wav.samples().map(Result::unwrap).collect();
                assert_eq!(actual.len(), 64 * 48);
                for (f, &sample) in actual.iter().enumerate() {
                    assert_eq!(
                        sample,
                        expected_raw[f * channels + ch],
                        "raw REC contamination at frame{f}/channel{ch}"
                    );
                }
            }
        }
    }
    fn grant(
        host: &mut gigpies::local_audio::LocalAudio,
        writer: &str,
        scope: gigpies::control_model::Scope,
    ) -> Counter {
        use gigpies::control_model::{Command, Request};
        let revision = host.engine_mut().revision();
        let r = Request {
            contract: "C-AUDIO".into(),
            version: 2,
            show_id: SHOW.into(),
            module: "audio".into(),
            epoch: Counter(7),
            writer: Some(writer.into()),
            lease: None,
            request_id: Some(Counter(1)),
            expected_revision: Some(revision),
            command: Command::Grant { scope },
        };
        let now = host.frame() / 48;
        host.engine_mut()
            .handle(&r, now)
            .unwrap()
            .outcome
            .unwrap()
            .body
            .granted_lease
            .unwrap()
    }
    fn pump(host: &mut gigpies::local_audio::LocalAudio, talkback: Option<&[f64]>) {
        let raw = vec![0.; host.topology().capture_channels * 48];
        let mut playback = vec![0.; host.topology().playback_channels * 48];
        let frame = host.frame();
        host.tick_with_capture_and_brain(frame / 48, 7, frame, &raw, &mut playback, None, talkback)
            .unwrap();
    }
    fn command(
        host: &mut gigpies::local_audio::LocalAudio,
        writer: &str,
        lease: Counter,
        id: u64,
        command: gigpies::brain_control::Command,
    ) {
        let r = gigpies::brain_control::Request {
            contract: "GP15-brain".into(),
            version: 1,
            show_id: SHOW.into(),
            module: "audio".into(),
            epoch: Counter(7),
            writer: Some(writer.into()),
            lease: Some(lease),
            request_id: Some(Counter(id)),
            expected_revision: Some(host.engine_mut().revision()),
            command,
        };
        let now = host.frame() / 48;
        let reply = host.brain_request(r.clone(), now, true, None).unwrap();
        assert_eq!(reply.state, "pending", "{reply:?}");
        pump(host, None);
        pump(host, None);
        let now = host.frame() / 48;
        let reply = host.brain_request(r, now, true, None).unwrap();
        assert_eq!(reply.reason, None, "{reply:?}");
    }
    fn read_frame(stream: &mut std::os::unix::net::UnixStream) -> Vec<u8> {
        use std::io::Read;
        let mut n = [0; 4];
        stream.read_exact(&mut n).unwrap();
        let mut bytes = vec![0; u32::from_be_bytes(n) as usize];
        assert!(bytes.len() <= gigpies::analysis_stream::WIRE_BYTES);
        stream.read_exact(&mut bytes).unwrap();
        bytes
    }
    #[test]
    #[ignore = "explicit hash-verified owner libraries; actual provider+PA+raw analysis worker"]
    fn actual_local_audio_every_destination_foh_mute_fx_and_analysis_isolation() {
        use gigpies::{
            brain_control::{Command as B, MonitorSource},
            control_model::Scope,
            local_audio::LocalAudio,
            topology::{EngineTopology, OutputSource},
        };
        use std::io::Write;
        let _ = manifest();
        let json = limited_fixture("stereo3way");
        for channels in [16, 32, 48] {
            let root = Directory::new(&format!("local-{channels}"));
            let mut host = LocalAudio::bind_configured(
                &root.0,
                "audio",
                SHOW,
                Counter(7),
                EngineTopology::software(channels, 5, 6).unwrap(),
            )
            .unwrap();
            host.enable_modules(&manifest_path()).unwrap();
            host.configure_pa(&json).unwrap();
            host.enable_synthetic_fouraux().unwrap();
            host.enable_analysis(&root.0).unwrap();
            host.rearm().unwrap();
            let mut analysis =
                std::os::unix::net::UnixStream::connect(root.0.join("analysis.sock")).unwrap();
            analysis
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let subscribe = br#"{"subscription":"lux.aux.v1","version":1}"#;
            analysis
                .write_all(&(subscribe.len() as u32).to_be_bytes())
                .unwrap();
            analysis.write_all(subscribe).unwrap();
            let descriptor: gigpies::analysis_stream::Descriptor =
                serde_json::from_slice(&read_frame(&mut analysis)).unwrap();
            let mut cursor = gigpies::analysis_stream::WindowCursor::new(descriptor).unwrap();
            let talkback = grant(&mut host, "tb", Scope::TalkbackDestinations);
            let foh = grant(&mut host, "tb-foh", Scope::TalkbackFoh);
            let monitor = grant(&mut host, "listen", Scope::LocalOperatorMonitor);
            command(
                &mut host,
                "listen",
                monitor,
                2,
                B::MonitorSet {
                    source: MonitorSource::Main,
                    gain_cdb: 0,
                    mute: true,
                    dim: false,
                    armed: false,
                },
            );
            command(
                &mut host,
                "tb-foh",
                foh,
                2,
                B::TalkbackFoh { enabled: true },
            );
            let mut request_id = 2;
            for selected in 0..5 {
                command(
                    &mut host,
                    "tb",
                    talkback,
                    request_id,
                    B::TalkbackSet {
                        monitors: vec![selected],
                        gain_cdb: 0,
                        mute: false,
                    },
                );
                request_id += 1;
                command(
                    &mut host,
                    "tb",
                    talkback,
                    request_id,
                    B::Hold {
                        generation: Counter(selected as u64 + 1),
                    },
                );
                request_id += 1;
                for _ in 0..8 {
                    pump(&mut host, Some(&[0.25; 48]));
                }
                for frame in host.last_bus_samples().chunks_exact(7) {
                    assert_eq!(
                        &frame[..2],
                        &[0.; 2],
                        "FOH TB must never enter direct Main bypass"
                    );
                    for index in 0..5 {
                        assert!(
                            (frame[index + 2] - if index == selected { 0.25 } else { 0. }).abs()
                                < 1e-12,
                            "monitor destination isolation"
                        );
                    }
                }
                assert!(host.brain_fx_send().iter().all(|v| *v == 0.));
                assert!(
                    host.brain_monitor_output().iter().all(|v| *v == 0.),
                    "no implicit local TB monitor loop"
                );
                // Physical patch admits actual PA outputs. Copy a further observed block
                // and independently locate each configured owner PA socket.
                let raw = vec![0.; host.topology().capture_channels * 48];
                let mut playback = vec![0.; host.topology().playback_channels * 48];
                let frame = host.frame();
                host.tick_with_capture_and_brain(
                    frame / 48,
                    7,
                    frame,
                    &raw,
                    &mut playback,
                    None,
                    Some(&[0.25; 48]),
                )
                .unwrap();
                let pa_slots: Vec<_> = host
                    .topology()
                    .outputs
                    .iter()
                    .filter(|p| matches!(p.source, Some(OutputSource::Pa { .. })))
                    .map(|p| p.playback_slot)
                    .collect();
                assert_eq!(pa_slots.len(), 6);
                let ceiling = 10_f64.powf(-24. / 20.);
                assert!(
                    playback
                        .chunks_exact(host.topology().playback_channels)
                        .any(|f| pa_slots.iter().any(|&p| f[p].abs() > 1e-5))
                );
                assert!(
                    playback
                        .chunks_exact(host.topology().playback_channels)
                        .all(|f| pa_slots.iter().all(|&p| f[p].abs() <= ceiling + 1e-12))
                );
                if selected == 0 {
                    // The actual published raw analysis window must contain only the original
                    // captured silence, despite nonzero talkback in all selected output profiles.
                    let bytes = read_frame(&mut analysis);
                    let window = cursor
                        .accept(
                            &bytes,
                            gigpies::analysis_stream::local::monotonic_ms().unwrap(),
                        )
                        .unwrap();
                    for packet in window
                        .packets
                        .chunks_exact(gigpies::analysis_stream::PACKET_BYTES)
                    {
                        let p = gigpies::transport::Packet::parse(packet).unwrap();
                        for i in 0..48 * 4 {
                            assert_eq!(p.pcm(i).unwrap(), 0);
                        }
                    }
                }
                command(
                    &mut host,
                    "listen",
                    monitor,
                    selected as u64 + 3,
                    B::MonitorSet {
                        source: MonitorSource::Monitor { index: selected },
                        gain_cdb: 0,
                        mute: true,
                        dim: false,
                        armed: false,
                    },
                );
                for _ in 0..8 {
                    pump(&mut host, Some(&[0.25; 48]));
                }
                assert!(
                    host.brain_monitor_output().iter().all(|v| *v == 0.),
                    "selected performer listen must exclude its injected talkback"
                );
            }
            host.revoke_writer("tb-foh");
            pump(&mut host, Some(&[0.25; 48]));
            assert_eq!(host.brain_snapshot().held_generation, None);
            assert!(!host.brain_snapshot().talkback_foh);
            host.engine_mut().mute_outputs().unwrap();
            for _ in 0..6 {
                pump(&mut host, Some(&[0.25; 48]));
            }
            assert!(host.last_bus_samples().iter().all(|v| *v == 0.));
            host.stop_analysis();
            drop(analysis);
            drop(host);
        }
    }
}

#[test]
fn actual_talkback_sample_renderer_is_bounded_and_does_not_allocate_or_free() {
    use gigpies::brain_control::{TalkbackRender, render_talkback_block};
    let safety = [1.; 48];
    let samples = [0.25; 48];
    let mut output = [0.; 48 * 7];
    let mut envelope = 0.;
    for block in 0..12 {
        let view = TalkbackRender {
            first_frame: block * 48,
            deadline_frame: Some(288),
            monitors: &[0, 4],
            foh: true,
            gain: 0.5,
            safety: &safety,
        };
        guarded(|| render_talkback_block(&view, &mut envelope, Some(&samples), &mut output, 7))
            .unwrap();
        for (f, row) in output.chunks_exact(7).enumerate() {
            let frame = block * 48 + f as u64;
            let expected_envelope = if frame < 240 {
                (frame + 1) as f64 / 240.
            } else if frame < 288 {
                1.
            } else if frame < 528 {
                1. - (frame - 287) as f64 / 240.
            } else {
                0.
            };
            let expected = 0.125 * expected_envelope;
            assert!((row[2] - expected).abs() < 1e-14);
            assert_eq!(row[2], row[6]);
            assert!(row[3..6].iter().all(|v| *v == 0.));
            assert!((row[0] - expected * std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-14);
            assert_eq!(row[0], row[1]);
        }
    }
    let view = TalkbackRender {
        first_frame: 576,
        deadline_frame: None,
        monitors: &[0],
        foh: true,
        gain: 0.5,
        safety: &safety,
    };
    assert!(
        guarded(|| render_talkback_block(
            &view,
            &mut envelope,
            Some(&[f64::NAN; 48]),
            &mut output,
            7
        ))
        .is_err()
    );
    assert!(output.iter().all(|v| *v == 0.));
}
