use gigpies::brain_audio::{bridge::*, device::*, host::*};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
thread_local! {static WATCH:Cell<bool>=const{Cell::new(false)};static ALLOC:Cell<usize>=const{Cell::new(0)};static FREE:Cell<usize>=const{Cell::new(0)};}
struct Guard;
unsafe impl GlobalAlloc for Guard {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        WATCH.with(|w| {
            if w.get() {
                ALLOC.with(|v| v.set(v.get() + 1));
            }
        });
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        WATCH.with(|w| {
            if w.get() {
                FREE.with(|v| v.set(v.get() + 1));
            }
        });
        unsafe { System.dealloc(p, l) }
    }
}
#[global_allocator]
static GLOBAL: Guard = Guard;
fn epochs() -> BridgeEpochs {
    BridgeEpochs {
        source: 11,
        destination: 29,
        route: 3,
    }
}
fn prepared(channels: usize) -> Bridge {
    Bridge::prepare(BridgeConfig::voice(channels), epochs()).unwrap()
}
fn fill(b: &mut Bridge, mut first: u64, frames: usize, channels: usize, value: f64) -> u64 {
    let data = vec![value; 48 * channels];
    for _ in 0..frames / 48 {
        b.push(epochs(), first, &data).unwrap();
        first += 48;
    }
    first
}
#[test]
fn bridge_prefill_epoch_fault_and_five_ms_silence() {
    let mut b = prepared(1);
    assert_eq!(b.arm(epochs()), Err(BridgeError::NotReady));
    let mut source = fill(&mut b, 0, 1152, 1, 0.25);
    b.arm(epochs()).unwrap();
    let mut out = [0.; 48];
    for block in 0..100 {
        b.push(epochs(), source, &[0.25; 48]).unwrap();
        source += 48;
        b.render(epochs(), block * 48, &mut out).unwrap();
    }
    assert!((out[47] - 0.25).abs() < 1e-6);
    b.invalidate();
    let mut fade = [0.; 288];
    assert_eq!(
        b.render(epochs(), 4800, &mut fade),
        Err(BridgeError::Identity)
    );
    assert!(fade.windows(2).all(|p| p[1] <= p[0]));
    assert_eq!(&fade[239..], &[0.; 49]);
    assert_eq!(b.reset(epochs()), Err(BridgeError::Identity));
    let next = BridgeEpochs {
        route: 4,
        ..epochs()
    };
    b.reset(next).unwrap();
    assert_eq!(
        b.push(epochs(), source, &[1.; 48]),
        Err(BridgeError::Identity)
    );
    assert!(!b.status().armed);
    assert_eq!(b.arm(next), Err(BridgeError::NotReady));
}
#[test]
fn bridge_rejects_invalid_and_stale_data_without_unbounded_recovery() {
    let mut b = prepared(2);
    b.push(epochs(), 10, &[0.; 96]).unwrap();
    assert_eq!(b.push(epochs(), 10, &[1.; 96]), Err(BridgeError::Timeline));
    assert_eq!(b.status().occupancy_frames, 48);
    assert_eq!(b.push(epochs(), 59, &[1.; 96]), Err(BridgeError::Timeline));
    assert_eq!(b.status().occupancy_frames, 0);
    assert!(!b.status().ready);
    let mut b = prepared(1);
    assert_eq!(
        b.push(epochs(), 0, &[f64::NAN; 48]),
        Err(BridgeError::Nonfinite)
    );
    let mut b = prepared(1);
    fill(&mut b, 0, 4080, 1, 0.);
    assert_eq!(
        b.push(epochs(), 4080, &[0.; 48]),
        Err(BridgeError::Overflow)
    );
    assert_eq!(b.status().overflows, 1);
}
#[test]
fn arbitrary_output_partition_is_exact_and_render_push_reset_do_not_allocate_or_free() {
    let mut a = prepared(2);
    let mut b = prepared(2);
    let mut source = fill(&mut a, 0, 1152, 2, 0.1);
    fill(&mut b, 0, 1152, 2, 0.1);
    a.arm(epochs()).unwrap();
    b.arm(epochs()).unwrap();
    let mut whole = [0.; 96];
    let mut split = [0.; 96];
    WATCH.with(|v| v.set(true));
    for block in 0..100 {
        a.push(epochs(), source, &[0.1; 96]).unwrap();
        b.push(epochs(), source, &[0.1; 96]).unwrap();
        source += 48;
        a.render(epochs(), block * 48, &mut whole).unwrap();
        b.render(epochs(), block * 48, &mut split[..34]).unwrap();
        b.render(epochs(), block * 48 + 17, &mut split[34..])
            .unwrap();
        assert_eq!(whole, split);
    }
    a.invalidate();
    let _ = a.render(epochs(), 4800, &mut whole);
    a.reset(BridgeEpochs {
        route: 4,
        ..epochs()
    })
    .unwrap();
    WATCH.with(|v| v.set(false));
    assert_eq!(ALLOC.with(Cell::get), 0);
    assert_eq!(FREE.with(Cell::get), 0);
}
// The reference is analytic RMS of a sinusoid, independent of Rubato. A silent
// second input checks leakage. Startup/attack and edge transients are excluded.
fn tone_response(hz: f64) -> (f64, f64) {
    let mut b = prepared(2);
    let mut first = 0u64;
    let mut input = [0.; 96];
    let mut output = [0.; 96];
    let put = |b: &mut Bridge, first: u64, input: &mut [f64; 96]| {
        for n in 0..48 {
            input[n * 2] =
                0.5 * (std::f64::consts::TAU * hz * (first + n as u64) as f64 / 48_000.).sin();
        }
        b.push(epochs(), first, input).unwrap();
    };
    for _ in 0..24 {
        put(&mut b, first, &mut input);
        first += 48;
    }
    b.arm(epochs()).unwrap();
    let mut power = 0.;
    let mut leakage: f64 = 0.;
    let mut samples = 0;
    for block in 0..240 {
        put(&mut b, first, &mut input);
        first += 48;
        b.render(epochs(), block * 48, &mut output).unwrap();
        if block >= 40 {
            for frame in output.chunks_exact(2) {
                power += frame[0] * frame[0];
                leakage = leakage.max(frame[1].abs());
                samples += 1;
            }
        }
    }
    (
        20. * ((power / samples as f64).sqrt() / (0.5 / 2_f64.sqrt())).log10(),
        leakage,
    )
}
#[test]
fn numerical_independent_passband_stopband_and_channel_isolation() {
    for hz in [20., 997., 8000., 18000.] {
        let (db, leak) = tone_response(hz);
        assert!(db.abs() <= 0.1, "{hz}: {db} dB");
        assert!(leak < 1e-5);
    }
    for hz in [23000., 23700.] {
        let (db, leak) = tone_response(hz);
        assert!(db <= -80., "{hz}: {db} dB");
        assert!(leak < 1e-5);
    }
}
fn drift(ppm: f64, seconds: usize, jitter: bool) -> BridgeStatus {
    drift_config(ppm, seconds, jitter, BridgeConfig::voice(1), 1056)
}
fn drift_config(
    ppm: f64,
    seconds: usize,
    jitter: bool,
    config: BridgeConfig,
    prefill: usize,
) -> BridgeStatus {
    let mut b = Bridge::prepare(config, epochs()).unwrap();
    let mut source = fill(&mut b, 0, prefill, 1, 0.01);
    b.arm(epochs()).unwrap();
    let mut produced = 0.;
    let mut delivered = 0u64;
    let mut out = [0.; 48];
    let started = std::time::Instant::now();
    let mut low = config.capacity_frames;
    let mut high = 0;
    for block in 0..seconds * 1000 {
        produced += 48. * (1. + ppm * 1e-6);
        let available = (produced as u64).saturating_sub(delivered);
        // A bounded three-ms scheduling burst is separate from oscillator skew.
        if !jitter || block % 3 == 2 {
            let n = available as usize;
            let input = [0.01; 192];
            assert!(n <= input.len());
            b.push(epochs(), source, &input[..n]).unwrap();
            source += n as u64;
            delivered += n as u64;
        }
        let result = b.render(epochs(), block as u64 * 48, &mut out);
        assert!(
            result.is_ok(),
            "ppm{ppm} block{block}: {result:?} {:?}",
            b.status()
        );
        let occupancy = b.status().occupancy_frames;
        low = low.min(occupancy);
        high = high.max(occupancy);
        assert!(occupancy < 2000);
    }
    eprintln!(
        "virtual_clock ppm={ppm} seconds={seconds} jitter={jitter} occupancy={low}..{high} elapsed_s={:.3} final={:?}",
        started.elapsed().as_secs_f64(),
        b.status()
    );
    b.status()
}
#[test]
#[ignore = "one-time 45-second virtual-clock stress; run explicitly after bridge/controller changes"]
fn production_bridge_tracks_both_1000ppm_signs_and_separate_jitter() {
    for ppm in [-1000., 1000.] {
        let status = drift(ppm, 45, true);
        assert!(
            (status.estimated_source_skew_ppm - ppm).abs() < 300.,
            "{status:?}"
        );
        assert_eq!(status.underruns, 0);
        assert_eq!(status.overflows, 0);
    }
    let status = drift(0., 5, true);
    assert!(status.estimated_source_skew_ppm.abs() < 300., "{status:?}");
}
#[test]
#[ignore = "one-time long virtual-clock acceptance; run explicitly after bridge/controller changes"]
fn long_virtual_clocks_exceed_uncompensated_buffer_exhaustion() {
    for (ppm, seconds) in [
        (-10., 1250),
        (10., 1250),
        (-100., 125),
        (100., 125),
        (-1000., 120),
        (1000., 120),
    ] {
        let config = BridgeConfig {
            capacity_frames: 1024,
            target_frames: 480,
            ..BridgeConfig::voice(1)
        };
        let status = drift_config(ppm, seconds, true, config, 576);
        assert!(
            (status.estimated_source_skew_ppm - ppm).abs() < 30.,
            "{status:?}"
        );
    }
}
#[test]
#[ignore = "one-time out-of-envelope saturation evidence; run explicitly after bridge/controller changes"]
fn outside_admitted_clock_skew_fails_closed() {
    let mut b = prepared(1);
    let mut source = fill(&mut b, 0, 1152, 1, 0.01);
    b.arm(epochs()).unwrap();
    let mut phase = 0.;
    let mut delivered = 0u64;
    let mut out = [0.; 48];
    let mut failed = false;
    for block in 0..60_000 {
        phase += 48. * 1.002;
        let n = phase as u64 - delivered;
        delivered += n;
        if b.push(epochs(), source, &[0.01; 49][..n as usize]).is_err() {
            failed = true;
            break;
        }
        source += n;
        if b.render(epochs(), block * 48, &mut out).is_err() {
            failed = true;
            break;
        }
    }
    assert!(failed);
    assert!(!b.status().armed);
    assert!(matches!(
        b.status().fault,
        Some(BridgeError::Skew | BridgeError::Overflow)
    ));
}
fn device_config() -> DeviceConfig {
    DeviceConfig {
        device_id: "synthetic-boundary".into(),
        endpoint: "fake:duplex".into(),
        sample_rate: 48_000,
        capture_channels: 3,
        playback_channels: 4,
        format: SampleFormat::S32Le,
        period_frames: 48,
        buffer_frames: 384,
        microphone: PortMap {
            id: "mic".into(),
            socket: "fake-c3".into(),
            slot: 2,
        },
        monitor: [
            PortMap {
                id: "left".into(),
                socket: "fake-p4".into(),
                slot: 3,
            },
            PortMap {
                id: "right".into(),
                socket: "fake-p2".into(),
                slot: 1,
            },
        ],
        socket_mapping_record: None,
        shared_clock_record: None,
        hardware_monitoring_record: None,
    }
}
#[derive(Default)]
struct Renderer {
    capture: f64,
    epoch: u64,
    calls: usize,
    fault: Option<DeviceError>,
}
impl BrainRender for Renderer {
    fn capture(&mut self, epoch: u64, _: u64, mono: &[f64]) {
        self.epoch = epoch;
        self.capture = mono[47];
        self.calls += 1;
    }
    fn playback(&mut self, _: u64, _: u64, stereo: &mut [f64]) {
        for f in stereo.chunks_exact_mut(2) {
            f[0] = 0.2;
            f[1] = -0.3;
        }
    }
    fn fault(&mut self, _: u64, error: DeviceError) {
        self.fault = Some(error);
    }
}
#[test]
fn fake_and_raw_boundary_mapping_partial_transfers_readback_arm_and_fault() {
    let config = device_config();
    let mut fake = FakeDuplex::prepare(&config).unwrap();
    fake.set_max_transfer(17);
    let mut host = BrainHost::prepare(fake, config, 42, 3).unwrap();
    let mut renderer = Renderer::default();
    assert_eq!(host.service(0, &mut renderer), Err(DeviceError::NotArmed));
    let wrong = DeviceReadback {
        epoch: 41,
        configuration_generation: 3,
    };
    assert_eq!(host.arm(wrong, 0), Err(DeviceError::StaleReadback));
    host.arm(host.readback(), 0).unwrap();
    host.set_levels(LocalLevels {
        microphone_muted: false,
        monitor_muted: false,
        monitor_gain_db: 0.,
        ..LocalLevels::default()
    })
    .unwrap();
    let mut capture = [0.; 144];
    for f in capture.chunks_exact_mut(3) {
        f[2] = 0.1;
    }
    let mut output = [0.; 192];
    for block in 0..10 {
        host.device_mut().advance(&capture, 48).unwrap();
        for _ in 0..3 {
            host.service(block, &mut renderer).unwrap();
        }
        assert_eq!(host.device_mut().drain_playback(&mut output), 48);
    }
    assert_eq!(renderer.epoch, 42);
    assert_eq!(renderer.calls, 10);
    assert_eq!(renderer.capture, 0.1);
    for f in output.chunks_exact(4) {
        assert_eq!(f, [0., -0.3, 0., 0.2]);
    }
    assert!(!host.status().physical_mapping_verified);
    host.device_mut().inject_fault(DeviceError::Disconnected);
    assert_eq!(
        host.service(11, &mut renderer),
        Err(DeviceError::Disconnected)
    );
    assert_eq!(renderer.fault, Some(DeviceError::Disconnected));
    assert_eq!(host.arm(host.readback(), 12), Err(DeviceError::NotArmed));
}
#[test]
fn persisted_device_intent_cannot_restore_permissions_or_accept_bad_maps() {
    let config = device_config();
    let saved = serde_json::to_value(&config).unwrap();
    assert!(saved.get("armed").is_none());
    assert!(saved.get("epoch").is_none());
    let mut bad = config.clone();
    bad.monitor[1].slot = bad.monitor[0].slot;
    assert_eq!(bad.validate(), Err(DeviceError::Mapping));
    bad = config.clone();
    bad.sample_rate = 44100;
    assert_eq!(bad.validate(), Err(DeviceError::Configuration));
    let fake = FakeDuplex::prepare(&config).unwrap();
    let mut caps = fake.capabilities().clone();
    caps.physical = true;
    assert_eq!(
        config.validate_capabilities(&caps),
        Err(DeviceError::PhysicalEvidence)
    );
    let mut host = BrainHost::prepare(fake, config, 44, 4).unwrap();
    host.arm(host.readback(), 0).unwrap();
    assert_eq!(
        host.service(101, &mut Renderer::default()),
        Err(DeviceError::Deadline)
    );
}

#[test]
fn impulse_delay_dc_silence_and_chirp_are_finite_with_declared_bounds() {
    let mut bridge = prepared(1);
    let mut first = 0u64;
    let mut input = [0.; 48];
    let mut output = [0.; 48];
    let mut observed = Vec::with_capacity(14_400);
    for _ in 0..24 {
        bridge.push(epochs(), first, &input).unwrap();
        first += 48;
    }
    bridge.arm(epochs()).unwrap();
    let delay = bridge.status().filter_delay_output_frames;
    for block in 0..300 {
        for (n, sample) in input.iter_mut().enumerate() {
            let frame = first + n as u64;
            *sample = if frame == 2400 {
                0.5
            } else if frame >= 7200 {
                // Independent continuous chirp: phase integral of f(t)=100+4000t.
                let t = (frame - 7200) as f64 / 48_000.;
                0.25 * (std::f64::consts::TAU * (100. * t + 2000. * t * t)).sin()
            } else {
                0.
            };
        }
        bridge.push(epochs(), first, &input).unwrap();
        first += 48;
        bridge.render(epochs(), block * 48, &mut output).unwrap();
        observed.extend_from_slice(&output);
    }
    assert!(observed.iter().all(|x| x.is_finite() && x.abs() <= 0.51));
    let peak = observed[..4800]
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
        .unwrap()
        .0;
    assert!(peak.abs_diff(2400 + delay) <= 2, "peak{peak}, delay{delay}");
    assert!(observed[..1800].iter().all(|x| x.abs() < 1e-8));
    let sum: f64 = observed[1800..3600].iter().sum();
    assert!((sum - 0.5).abs() < 0.001, "impulse area{sum}");
    let mut dc = prepared(1);
    let mut source = fill(&mut dc, 0, 1152, 1, 0.2);
    dc.arm(epochs()).unwrap();
    for block in 0..100 {
        dc.push(epochs(), source, &[0.2; 48]).unwrap();
        source += 48;
        dc.render(epochs(), block * 48, &mut output).unwrap();
    }
    assert!(output.iter().all(|x| (*x - 0.2).abs() < 1e-6));
}

#[test]
fn startup_with_both_drift_signs_and_bursty_delivery_remains_bounded() {
    for ppm in [-1000., 1000.] {
        let status = drift(ppm, 1, true);
        assert!(status.ratio_output_per_input.is_finite());
        assert!(status.estimated_source_skew_ppm.abs() <= 1500.);
        assert_eq!(status.underruns, 0);
    }
}

#[test]
fn prepared_host_render_and_failure_neither_allocate_nor_deallocate() {
    let config = device_config();
    let fake = FakeDuplex::prepare(&config).unwrap();
    let mut host = BrainHost::prepare(fake, config, 52, 7).unwrap();
    let mut renderer = Renderer::default();
    host.arm(host.readback(), 0).unwrap();
    let mut output = [0.; 192];
    WATCH.with(|v| v.set(true));
    for block in 0..10 {
        host.device_mut().advance(&[0.; 144], 48).unwrap();
        host.service(block, &mut renderer).unwrap();
        host.device_mut().drain_playback(&mut output);
    }
    host.device_mut().inject_fault(DeviceError::Xrun);
    let _ = host.service(11, &mut renderer);
    WATCH.with(|v| v.set(false));
    assert_eq!(ALLOC.with(Cell::get), 0);
    assert_eq!(FREE.with(Cell::get), 0);
}

#[test]
#[ignore = "one-time virtual-clock rate-ramp evidence; run after controller changes"]
fn slow_rate_ramp_crosses_both_signs_without_sample_corrections() {
    let config = BridgeConfig {
        capacity_frames: 1024,
        target_frames: 480,
        ..BridgeConfig::voice(1)
    };
    let mut bridge = Bridge::prepare(config, epochs()).unwrap();
    let mut source = fill(&mut bridge, 0, 576, 1, 0.01);
    bridge.arm(epochs()).unwrap();
    let mut produced = 0.;
    let mut delivered = 0u64;
    let mut out = [0.; 48];
    for block in 0..240_000 {
        let ppm = -1000. + 2000. * (block as f64 / 200_000.).min(1.);
        produced += 48. * (1. + ppm * 1e-6);
        if block % 3 == 2 {
            let n = produced as u64 - delivered;
            bridge
                .push(epochs(), source, &[0.01; 192][..n as usize])
                .unwrap();
            source += n;
            delivered += n;
        }
        bridge.render(epochs(), block * 48, &mut out).unwrap();
        assert!(bridge.status().occupancy_frames < config.capacity_frames);
    }
    let status = bridge.status();
    assert_eq!(status.underruns, 0);
    assert_eq!(status.overflows, 0);
    assert!(
        (status.estimated_source_skew_ppm - 1000.).abs() < 100.,
        "{status:?}"
    );
}

#[test]
fn analytic_tone_phase_matches_declared_delay_at_unity_clock_ratio() {
    let mut bridge = prepared(1);
    let hz = 997.;
    let radians = std::f64::consts::TAU * hz / 48_000.;
    let mut initial = [0.; 1009];
    for (i, x) in initial.iter_mut().enumerate() {
        *x = 0.5 * (radians * i as f64).sin();
    }
    bridge.push(epochs(), 0, &initial).unwrap();
    bridge.arm(epochs()).unwrap();
    // The reviewed phase-grid convention starts at -(256-1), advances before
    // output, and phase zero centers at 128-127/128. This yields this analytic
    // fractional delay; no output fitting or second resampler supplies it.
    let delay = 127. - 1. / 128.;
    assert!((bridge.status().filter_delay_output_frames as f64 - delay).abs() <= 2.);
    let mut source = 1009u64;
    let mut input = [0.; 48];
    let mut output = [0.; 48];
    let mut max_error: f64 = 0.;
    for block in 0..100 {
        if block > 0 {
            for (i, x) in input.iter_mut().enumerate() {
                *x = 0.5 * (radians * (source + i as u64) as f64).sin();
            }
            bridge.push(epochs(), source, &input).unwrap();
            source += 48;
        }
        bridge.render(epochs(), block * 48, &mut output).unwrap();
        if block >= 20 {
            for (n, actual) in output.iter().enumerate() {
                let expected = 0.5 * (radians * ((block * 48 + n as u64) as f64 - delay)).sin();
                max_error = max_error.max((actual - expected).abs());
            }
        }
    }
    // Maximum absolute waveform error0.001 corresponds to <0.002rad phase error
    // at this amplitude when measured near zero crossings; delay is not refitted.
    assert!(
        max_error < 0.001,
        "analytic phase/waveform error{max_error}, delay{delay}, {:?}",
        bridge.status()
    );
}

#[test]
fn inactive_playback_can_advance_silently_then_arm_only_fresh_prefill() {
    let mut bridge = prepared(2);
    let mut out = [1.; 96];
    for block in 0..1000 {
        bridge.render(epochs(), block * 48, &mut out).unwrap();
        assert_eq!(out, [0.; 96]);
    }
    // The owner discards incoming media until explicit arm intent. First source
    // frame need not equal playback position across independent domains.
    for block in 0..22 {
        bridge
            .push(epochs(), 90_000 + block * 48, &[0.2; 96])
            .unwrap();
    }
    bridge.arm(epochs()).unwrap();
    bridge.render(epochs(), 48_000, &mut out).unwrap();
    assert!(bridge.status().armed);
    bridge.disarm();
    for block in 0..6 {
        let _ = bridge.render(epochs(), 48_048 + block * 48, &mut out);
    }
    assert_eq!(out, [0.; 96]);
    assert_eq!(bridge.arm(epochs()), Err(BridgeError::NotReady));
    let next = BridgeEpochs {
        route: 4,
        ..epochs()
    };
    bridge.reset(next).unwrap();
    assert_eq!(
        bridge.push(epochs(), 91_056, &[0.2; 96]),
        Err(BridgeError::Identity)
    );
    for block in 0..22 {
        bridge.push(next, 100_000 + block * 48, &[0.; 96]).unwrap();
    }
    bridge.arm(next).unwrap();
    bridge.render(next, 49_000, &mut out).unwrap();
    assert_eq!(out, [0.; 96]);
}

#[test]
fn failed_device_start_invalidates_epoch_instead_of_retrying_audibly() {
    let config = device_config();
    let mut device = FakeDuplex::prepare(&config).unwrap();
    device.inject_fault(DeviceError::Xrun);
    let mut host = BrainHost::prepare(device, config, 63, 7).unwrap();
    assert_eq!(host.arm(host.readback(), 0), Err(DeviceError::Xrun));
    assert_eq!(host.status().fault, Some(DeviceError::Xrun));
    assert_eq!(host.status().faults, 1);
    assert_eq!(host.arm(host.readback(), 1), Err(DeviceError::NotArmed));
}

#[test]
fn selection_barrier_silences_unsubmitted_partial_period_without_resetting_clock() {
    let config = device_config();
    let mut device = FakeDuplex::prepare(&config).unwrap();
    device.set_max_transfer(17);
    let mut host = BrainHost::prepare(device, config, 73, 9).unwrap();
    let mut renderer = Renderer::default();
    host.set_levels(LocalLevels {
        monitor_muted: false,
        monitor_gain_db: 0.,
        ..LocalLevels::default()
    })
    .unwrap();
    host.arm(host.readback(), 0).unwrap();
    host.device_mut().advance(&[0.; 144], 48).unwrap();
    host.service(0, &mut renderer).unwrap();
    assert_eq!(host.invalidate_monitor_output(), Some(17));
    host.service(1, &mut renderer).unwrap();
    host.service(2, &mut renderer).unwrap();
    let mut submitted = [0.; 192];
    assert_eq!(host.device_mut().drain_playback(&mut submitted), 48);
    assert!(submitted[..17 * 4].iter().any(|v| *v != 0.));
    assert!(submitted[17 * 4..].iter().all(|v| *v == 0.));
    assert_eq!(host.status().capture_frame, 48);
    assert_eq!(host.status().playback_frame, 48);
    assert_eq!(host.readback().epoch, 73);
    assert!(host.is_armed());
}

#[derive(Debug, Clone, Copy)]
enum ClockFault {
    Device,
    Bridge,
    Identity,
}
struct DuplexClockRenderer {
    monitor: Bridge,
    talkback: Bridge,
    fault: Option<ClockFault>,
}
impl BrainRender for DuplexClockRenderer {
    fn capture(&mut self, epoch: u64, first: u64, mono: &[f64]) {
        if epoch != epochs().source {
            self.fault = Some(ClockFault::Identity);
            return;
        }
        if self.talkback.push(epochs(), first, mono).is_err() {
            self.fault = Some(ClockFault::Bridge);
        }
    }
    fn playback(&mut self, epoch: u64, first: u64, stereo: &mut [f64]) {
        let monitor_epochs = BridgeEpochs {
            source: epochs().destination,
            destination: epochs().source,
            route: 3,
        };
        if epoch != monitor_epochs.destination {
            stereo.fill(0.);
            self.fault = Some(ClockFault::Identity);
            return;
        }
        // Explicit test arm intent was issued before either clock started.
        if self.monitor.status().ready
            && !self.monitor.status().armed
            && self.monitor.arm(monitor_epochs).is_err()
        {
            self.fault = Some(ClockFault::Bridge);
        }
        if self.monitor.render(monitor_epochs, first, stereo).is_err() {
            self.fault = Some(ClockFault::Bridge);
        }
    }
    fn fault(&mut self, _: u64, _: DeviceError) {
        self.fault = Some(ClockFault::Device);
    }
}

#[test]
#[ignore = "one-time combined fake duplex and both bridge clock acceptance; run after host/bridge changes"]
fn actual_fake_duplex_and_both_bridges_follow_independent_integer_clocks() {
    for (ppm, seconds) in [
        (-10_i64, 1250_u64),
        (10, 1250),
        (-100, 125),
        (100, 125),
        (-1000, 120),
        (1000, 120),
    ] {
        let bridge_config = BridgeConfig {
            capacity_frames: 1024,
            target_frames: 480,
            ..BridgeConfig::voice(1)
        };
        let monitor_epochs = BridgeEpochs {
            source: epochs().destination,
            destination: epochs().source,
            route: 3,
        };
        let mut renderer = DuplexClockRenderer {
            talkback: Bridge::prepare(bridge_config, epochs()).unwrap(),
            monitor: Bridge::prepare(
                BridgeConfig {
                    channels: 2,
                    ..bridge_config
                },
                monitor_epochs,
            )
            .unwrap(),
            fault: None,
        };
        let config = device_config();
        let fake = FakeDuplex::prepare(&config).unwrap();
        let mut host = BrainHost::prepare(fake, config, epochs().source, 3).unwrap();
        host.set_levels(LocalLevels {
            microphone_muted: false,
            monitor_muted: false,
            monitor_gain_db: 0.,
            ..LocalLevels::default()
        })
        .unwrap();
        host.arm(host.readback(), 0).unwrap();
        let mut capture = [0.; 144];
        for frame in capture.chunks_exact_mut(3) {
            frame[2] = 0.125;
        }
        let mut program = [0.; 96];
        for frame in program.chunks_exact_mut(2) {
            frame[0] = 0.2;
            frame[1] = -0.1;
        }
        let mut playback = [0.; 192];
        let mut talkback = [0.; 48];
        let mut brain_phase = 0i64;
        let mut tb_low = 1024;
        let mut tb_high = 0;
        let mut mon_low = 1024;
        let mut mon_high = 0;
        let started = std::time::Instant::now();
        let mut tb_ppm_sum = 0.;
        let mut monitor_ppm_sum = 0.;
        let mut ratio_observations = 0u64;
        for block in 0..seconds * 1000 {
            // Stagebox advances exactly48frames per virtual millisecond. Brain's
            // independent oscillator has its own integer millionth-period phase.
            // Every device transfer and media grouping remains48frames.
            renderer
                .monitor
                .push(monitor_epochs, block * 48, &program)
                .unwrap();
            brain_phase += 1_000_000 + ppm;
            for _ in 0..2 {
                if brain_phase < 1_000_000 {
                    break;
                }
                brain_phase -= 1_000_000;
                host.device_mut().advance(&capture, 48).unwrap();
                host.service(block, &mut renderer).unwrap();
                assert_eq!(host.device_mut().drain_playback(&mut playback), 48);
                if block > 1000 {
                    for frame in playback.chunks_exact(4) {
                        assert_eq!(frame[0], 0.);
                        assert_eq!(frame[2], 0.);
                        assert!((frame[3] - 0.2).abs() < 1e-5);
                        assert!((frame[1] + 0.1).abs() < 1e-5);
                    }
                }
            }
            assert!(
                renderer.fault.is_none(),
                "{ppm} block{block}: {:?}",
                renderer.fault
            );
            if renderer.talkback.status().ready && !renderer.talkback.status().armed {
                renderer.talkback.arm(epochs()).unwrap();
            }
            renderer
                .talkback
                .render(epochs(), block * 48, &mut talkback)
                .unwrap();
            if block > 1000 {
                assert!(talkback.iter().all(|v| (*v - 0.125).abs() < 1e-5));
            }
            let tb = renderer.talkback.status();
            let monitor = renderer.monitor.status();
            // Fixed packet cadence quantizes instantaneous occupancy by48frames.
            // Assess oscillator tracking over the settled second half, separating
            // that bounded scheduling phase from the slow mean clock skew.
            if block >= seconds * 500 {
                tb_ppm_sum += tb.estimated_source_skew_ppm;
                monitor_ppm_sum += monitor.estimated_source_skew_ppm;
                ratio_observations += 1;
            }
            if block > 1000 {
                tb_low = tb_low.min(tb.occupancy_frames);
                tb_high = tb_high.max(tb.occupancy_frames);
                mon_low = mon_low.min(monitor.occupancy_frames);
                mon_high = mon_high.max(monitor.occupancy_frames);
            }
        }
        let tb = renderer.talkback.status();
        let monitor = renderer.monitor.status();
        let mean_tb_ppm = tb_ppm_sum / ratio_observations as f64;
        let mean_monitor_ppm = monitor_ppm_sum / ratio_observations as f64;
        assert!(
            (mean_tb_ppm - ppm as f64).abs() < 30.,
            "mean{mean_tb_ppm} {tb:?}"
        );
        assert!(
            (mean_monitor_ppm + ppm as f64).abs() < 30.,
            "mean{mean_monitor_ppm} {monitor:?}"
        );
        assert_eq!(
            (
                tb.underruns,
                tb.overflows,
                monitor.underruns,
                monitor.overflows
            ),
            (0, 0, 0, 0)
        );
        eprintln!(
            "combined_duplex ppm={ppm} seconds={seconds} elapsed_s={:.3} TB_occupancy={tb_low}..{tb_high} monitor_occupancy={mon_low}..{mon_high} TB_mean_ppm={} monitor_mean_ppm={} device_frame={}",
            started.elapsed().as_secs_f64(),
            mean_tb_ppm,
            mean_monitor_ppm,
            host.status().capture_frame
        );
    }
}

#[test]
fn independent_direction_stalls_xrun_clock_fault_and_fresh_reopen_close_safely() {
    for capture_live in [false, true] {
        let config = device_config();
        let fake = FakeDuplex::prepare(&config).unwrap();
        let mut host = BrainHost::prepare(fake, config, 83, 11).unwrap();
        let mut renderer = Renderer::default();
        host.arm(host.readback(), 0).unwrap();
        let mut out = [0.; 192];
        for now in 0..=101 {
            host.device_mut()
                .advance(
                    if capture_live { &[0.; 144] } else { &[] },
                    if capture_live { 0 } else { 48 },
                )
                .unwrap();
            let result = host.service(now, &mut renderer);
            host.device_mut().drain_playback(&mut out);
            if now <= 100 {
                assert!(result.is_ok());
            } else {
                assert_eq!(result, Err(DeviceError::Deadline));
            }
        }
        assert_eq!(renderer.fault, Some(DeviceError::Deadline));
        assert!(!host.is_armed());
        let old = host.readback();
        let config = host.config().clone();
        drop(host);
        let fake = FakeDuplex::prepare(&config).unwrap();
        let mut reopened = BrainHost::prepare(fake, config, 84, 12).unwrap();
        assert_eq!(reopened.arm(old, 102), Err(DeviceError::StaleReadback));
        reopened.arm(reopened.readback(), 102).unwrap();
        assert_eq!(reopened.status().capture_frame, 0);
        assert_eq!(reopened.status().playback_frame, 0);
    }
    for error in [
        DeviceError::Xrun,
        DeviceError::ClockDiscontinuity,
        DeviceError::Disconnected,
    ] {
        let config = device_config();
        let fake = FakeDuplex::prepare(&config).unwrap();
        let mut host = BrainHost::prepare(fake, config, 91, 15).unwrap();
        let mut renderer = Renderer::default();
        host.arm(host.readback(), 10).unwrap();
        host.device_mut().inject_fault(error);
        assert_eq!(host.service(11, &mut renderer), Err(error));
        assert_eq!(renderer.fault, Some(error));
        assert!(!host.is_armed());
        assert_eq!(host.status().faults, 1);
    }
    let config = device_config();
    let fake = FakeDuplex::prepare(&config).unwrap();
    let mut host = BrainHost::prepare(fake, config, 101, 17).unwrap();
    host.arm(host.readback(), 10).unwrap();
    assert_eq!(
        host.service(9, &mut Renderer::default()),
        Err(DeviceError::ClockDiscontinuity)
    );
}

#[test]
fn local_monitor_gain_dim_and_mute_change_only_mapped_playback() {
    struct LevelRenderer {
        microphone: [f64; 48],
    }
    impl BrainRender for LevelRenderer {
        fn capture(&mut self, _: u64, _: u64, mono: &[f64]) {
            self.microphone.copy_from_slice(mono);
        }
        fn playback(&mut self, _: u64, _: u64, stereo: &mut [f64]) {
            for frame in stereo.chunks_exact_mut(2) {
                frame[0] = 0.2;
                frame[1] = -0.3;
            }
        }
        fn fault(&mut self, _: u64, error: DeviceError) {
            panic!("unexpected device fault: {error}");
        }
    }
    let config = device_config();
    let fake = FakeDuplex::prepare(&config).unwrap();
    let mut host = BrainHost::prepare(fake, config, 111, 19).unwrap();
    let mut renderer = LevelRenderer {
        microphone: [0.; 48],
    };
    let mut capture = [0.; 144];
    let mut expected_microphone = [0.; 48];
    for (n, frame) in capture.chunks_exact_mut(3).enumerate() {
        frame[0] = 0.75;
        frame[1] = -0.5;
        frame[2] = 0.125 + n as f64 / 1024.;
        expected_microphone[n] = frame[2];
    }
    let mut levels = LocalLevels {
        microphone_muted: false,
        monitor_muted: false,
        monitor_gain_db: 0.,
        ..LocalLevels::default()
    };
    host.set_levels(levels).unwrap();
    host.arm(host.readback(), 0).unwrap();
    let mut output = [0.; 192];
    let mut now = 0;
    // Settle both startup ramps before testing isolated operator-level changes.
    for _ in 0..5 {
        host.device_mut().advance(&capture, 48).unwrap();
        host.service(now, &mut renderer).unwrap();
        now += 1;
        assert_eq!(host.device_mut().drain_playback(&mut output), 48);
    }
    assert_eq!(renderer.microphone, expected_microphone);
    let mut previous_gain = 1.;
    for (dim, muted, expected_gain) in [(false, false, 0.1), (true, false, 0.01), (true, true, 0.)]
    {
        levels.monitor_gain_db = -20.;
        levels.monitor_dim = dim;
        levels.monitor_muted = muted;
        host.set_levels(levels).unwrap();
        for block in 0..6 {
            host.device_mut().advance(&capture, 48).unwrap();
            host.service(now, &mut renderer).unwrap();
            now += 1;
            assert_eq!(host.device_mut().drain_playback(&mut output), 48);
            assert_eq!(
                renderer.microphone, expected_microphone,
                "operator output controls changed capture"
            );
            for (n, frame) in output.chunks_exact(4).enumerate() {
                assert!(frame.iter().all(|sample| sample.is_finite()));
                assert_eq!(frame[0], 0.);
                assert_eq!(frame[2], 0.);
                let gain = frame[3] / 0.2;
                assert!(gain >= expected_gain - 1e-14 && gain <= previous_gain + 1e-14);
                assert!(
                    (gain - previous_gain).abs() <= 1. / 240. + 1e-14,
                    "unbounded gain step"
                );
                assert!((frame[1] + 0.3 * gain).abs() < 1e-14, "stereo gains differ");
                if block * 48 + n >= 239 {
                    assert!((frame[3] - 0.2 * expected_gain).abs() < 1e-14);
                    assert!((frame[1] + 0.3 * expected_gain).abs() < 1e-14);
                    if muted {
                        assert_eq!(frame, [0.; 4]);
                    }
                }
                previous_gain = gain;
            }
        }
    }
    assert_eq!(host.status().captured_peak, expected_microphone[47]);
    assert_eq!(host.status().outgoing_peak, expected_microphone[47]);
}
