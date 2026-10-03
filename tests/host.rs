use gigpies::{host::*, transport::*};
#[test]
fn whole_stereo_loss_fades_and_old_epoch_cannot_return() {
    let mut wet = WetRender::new(41).unwrap();
    let mut bytes = [0; MAX_DATAGRAM];
    let s = spec(41, Role::WetReturn);
    let mut values = [0.0; 96];
    for p in values.chunks_exact_mut(2) {
        p[0] = 0.25;
        p[1] = -0.5;
    }
    for n in 0..8 {
        let len = s
            .encode_float(n * 48, n as u32, &values, &mut bytes)
            .unwrap();
        wet.admit(&bytes[..len]).unwrap();
    }
    let mut output = [0.0; 768];
    wet.render(0, &mut output).unwrap();
    assert!(output.iter().all(|x| *x == 0.0));
    wet.render(384, &mut output).unwrap();
    assert_eq!(output[766], 0.25);
    assert_eq!(output[767], -0.5);
    wet.render(768, &mut output).unwrap();
    assert!(output[480..].iter().all(|x| *x == 0.0));
    let old = spec(40, Role::WetReturn);
    let len = old.encode_float(768, 16, &values, &mut bytes).unwrap();
    assert_eq!(wet.admit(&bytes[..len]), Err(Error::Identity));
    let len = s.encode_float(0, 0, &values, &mut bytes).unwrap();
    assert_eq!(wet.admit(&bytes[..len]), Err(Error::Late));
    assert_eq!(wet.missing, 8);
    assert_eq!(wet.present, 8);
}
#[test]
fn block_boundary_admission_and_cursor_are_owned_by_audio() {
    let mut wet = WetRender::new(1).unwrap();
    let mut output = [0.; 768];
    wet.render(0, &mut output).unwrap();
    let mut bytes = [0; MAX_DATAGRAM];
    let len = spec(1, Role::WetReturn)
        .encode_float(0, 0, &[0.2; 96], &mut bytes)
        .unwrap();
    wet.admit(&bytes[..len]).unwrap();
    assert_eq!(wet.render(0, &mut output), Err(Error::Timeline));
    wet.render(384, &mut output).unwrap();
    assert!(output[0] > 0.);
    assert_eq!(wet.present, 1);
}
#[test]
fn stimulus_is_quiet_distinct_and_exact_pcm24() {
    for f in 0..48000 {
        let l = stimulus(f, 0);
        let r = stimulus(f, 1);
        assert!(l.abs() < 0.016 && r.abs() < 0.016);
        assert_eq!(l * 8388608., (l * 8388608.).round());
    }
    assert_ne!(stimulus(1000, 0), stimulus(1000, 1));
}

#[test]
fn usb_precision_requires_capture_and_playback_native_match() {
    let d = "Playback:\n  Format: S32_LE\n  Channels: 2\n  Bits: 24\n\nCapture:\n  Format: S32_LE\n  Channels: 2\n  Bits: 24\n";
    assert!(native_stereo_24(d));
    assert!(!native_stereo_24(&d.replacen(
        "Capture:\n  Format: S32_LE",
        "Capture:\n  Format: S16_LE",
        1
    )));
    assert!(!native_stereo_24(d.split("Capture:").next().unwrap()));
}

#[test]
fn long_fault_delays_do_not_masquerade_as_exact_percentiles() {
    let mut h = Histogram::default();
    assert_eq!(h.percentile(0.99), None);
    for _ in 0..98 {
        h.observe(200.5);
    }
    h.observe(20000.0);
    h.observe(250000.0);
    assert_eq!(h.percentile(0.95), Some(201.0));
    assert_eq!(h.percentile(0.99), None);
    assert_eq!(h.overflow, 2);
    assert_eq!(h.max_us, 250000.0);
}

#[test]
fn unused_s32_bits_cannot_round_into_a_native_24_bit_code() {
    for sample in [-8_388_608, -1_234, -1, 0, 1, 1_234, 8_388_607] {
        for low in [0, 3, 127, 128, 255] {
            let container = (sample << 8) | low;
            assert_eq!(capture_msb24(container) * 8388608., f64::from(sample));
            assert_eq!(
                capture_fullscale(container),
                sample == -8_388_608 || sample == 8_388_607
            );
        }
    }
}
#[test]
fn revised_deadline_is_explicit_and_incompatible_returns_are_refused() {
    let mut early = WetRender::with_delay(15, 768).unwrap();
    let mut wrong = [0; MAX_DATAGRAM];
    let wrong_len = spec(15, Role::WetReturn)
        .encode_float(0, 0, &[0.25; 96], &mut wrong)
        .unwrap();
    assert_eq!(early.admit(&wrong[..wrong_len]), Err(Error::Identity));
    let mut wet = WetRender::with_delay(15, 768).unwrap();
    let mut b = [0; MAX_DATAGRAM];
    let mut output = [0.; 768];
    wet.render(0, &mut output).unwrap();
    wet.render(384, &mut output).unwrap();
    assert_eq!(wet.missing, 0);
    let len = spec(15, Role::WetReturn)
        .encode_float(0, 0, &[0.25; 96], &mut b)
        .unwrap();
    assert_eq!(wet.admit(&b[..len]), Err(Error::Late));
    let len = spec_with_delay(15, Role::WetReturn, 768)
        .encode_float(0, 0, &[0.25; 96], &mut b)
        .unwrap();
    wet.admit(&b[..len]).unwrap();
    wet.render(768, &mut output).unwrap();
    assert!(output[0] > 0.);
    assert_eq!(wet.present, 1);
    assert!(WetRender::with_delay(15, 512).is_err());
}

#[test]
fn device_buffer_budget_is_explicit_and_bounded() {
    assert_eq!(device_buffer_frames(192, 4), Ok(768));
    assert_eq!(device_buffer_frames(384, 8), Ok(3072));
    for (period, periods) in [(0, 4), (383, 8), (384, 0), (384, 7), (usize::MAX, u32::MAX)] {
        assert_eq!(device_buffer_frames(period, periods), Err(Error::Format));
    }
}
