use gigpies::transport::*;

fn spec() -> StreamSpec {
    StreamSpec {
        session: 41,
        stream: 7,
        first_channel: 0,
        channels: 8,
        frames: 48,
        encoding: Encoding::Pcm24,
        role: Role::Analysis,
        delay_frames: 0,
    }
}
fn bytes(frame: u64, seq: u32) -> [u8; MAX_DATAGRAM] {
    let mut b = [0; MAX_DATAGRAM];
    spec().encode_pcm(frame, seq, &[17; 384], &mut b).unwrap();
    b
}
fn packet(b: &[u8; MAX_DATAGRAM]) -> Packet<'_> {
    Packet::parse(&b[..spec().packet_bytes()]).unwrap()
}

#[test]
fn integer_wire_endpoints_channel_order_and_no_conversion_loss() {
    let mut data = [0; 384];
    for (i, x) in data.iter_mut().enumerate() {
        *x = match i % 5 {
            0 => -8_388_608,
            1 => 8_388_607,
            2 => -1,
            _ => i as i32,
        };
    }
    let mut b = [0; MAX_DATAGRAM];
    let n = spec().encode_pcm(48, 9, &data, &mut b).unwrap();
    assert_eq!(&b[48..54], &[128, 0, 0, 127, 255, 255]);
    let p = Packet::parse(&b[..n]).unwrap();
    for (i, &x) in data.iter().enumerate() {
        assert_eq!(p.pcm(i).unwrap(), x);
        assert_eq!(p.sample(i).unwrap() * 8_388_608.0, f64::from(x));
    }
    data[23] = 8_388_608;
    assert_eq!(spec().encode_pcm(48, 9, &data, &mut b), Err(Error::Sample));
}
#[test]
fn float_headroom_and_error_bound_are_explicit() {
    let s = StreamSpec {
        channels: 4,
        encoding: Encoding::Float32,
        role: Role::FxSend,
        ..spec()
    };
    let source: Vec<f64> = (0..192).map(|i| ((i as f64) * 0.121).sin() * 4.0).collect();
    let data: Vec<f32> = source.iter().map(|&x| x as f32).collect();
    let mut b = [0; MAX_DATAGRAM];
    let n = s.encode_float(0, 0, &data, &mut b).unwrap();
    let p = Packet::parse(&b[..n]).unwrap();
    for (i, &x) in source.iter().enumerate() {
        assert_eq!(p.sample(i).unwrap(), f64::from(data[i]));
        assert!((p.sample(i).unwrap() - x).abs() <= 2.0_f64.powi(-22));
    }
    b[48..52].copy_from_slice(&f32::NAN.to_be_bytes());
    assert_eq!(Packet::parse(&b[..n]).unwrap_err(), Error::Sample);
}
#[test]
fn bounded_parser_rejects_bad_headers_and_all_truncations() {
    let b = bytes(0, 0);
    let n = spec().packet_bytes();
    for len in 0..n {
        assert!(Packet::parse(&b[..len]).is_err());
    }
    for index in [0, 4, 5, 6, 8, 10, 36, 44] {
        let mut bad = b;
        bad[index] = 255;
        assert!(Packet::parse(&bad[..n]).is_err(), "index {index}");
    }
    assert!(Packet::parse(&[0; MAX_DATAGRAM + 1]).is_err());
    // Arbitrary input stays bounded and cannot panic, including malformed lengths.
    let mut state = 1u64;
    for len in 0..=MAX_DATAGRAM {
        let mut fuzz = [0; MAX_DATAGRAM];
        for x in &mut fuzz[..len] {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            *x = (state >> 32) as u8;
        }
        let _ = Packet::parse(&fuzz[..len]);
    }
    assert_eq!(
        StreamSpec {
            channels: 9,
            ..spec()
        }
        .validate(),
        Err(Error::Size)
    );
}
#[test]
fn jitter_reorders_deduplicates_and_never_rewinds_after_loss() {
    let mut q = JitterBuffer::new(spec(), 0, 0, 8).unwrap();
    let mut out = [0; MAX_DATAGRAM];
    let a = bytes(0, 0);
    let b = bytes(48, 1);
    let c = bytes(144, 3);
    q.insert(packet(&b)).unwrap();
    q.insert(packet(&a)).unwrap();
    assert_eq!(q.insert(packet(&a)), Err(Error::Duplicate));
    assert_eq!(q.stats().reordered, 1);
    assert_eq!(q.occupancy(), 2);
    q.pop(&mut out).unwrap();
    assert_eq!(
        Packet::parse(&out[..spec().packet_bytes()])
            .unwrap()
            .source_frame(),
        0
    );
    q.pop(&mut out).unwrap();
    assert!(q.pop(&mut out).unwrap().is_none());
    assert!(out.iter().all(|&x| x == 0));
    assert_eq!(q.insert(packet(&b)), Err(Error::Late));
    q.insert(packet(&c)).unwrap();
    q.pop(&mut out).unwrap();
    assert_eq!(q.stats().missing, 1);
    assert_eq!(q.occupancy(), 0);
}
#[test]
fn admission_refuses_future_wrong_session_and_wrong_sequence() {
    let mut q = JitterBuffer::new(spec(), 0, 0, 4).unwrap();
    let future = bytes(192, 4);
    assert_eq!(q.insert(packet(&future)), Err(Error::Future));
    let mut bad = bytes(0, 0);
    bad[23] = 42;
    assert_eq!(q.insert(packet(&bad)), Err(Error::Identity));
    let bad = bytes(0, 1);
    assert_eq!(q.insert(packet(&bad)), Err(Error::Timeline));
    assert_eq!(q.occupancy(), 0);
}
#[test]
fn sequence_wrap_and_32_bit_frame_crossing_keep_alignment() {
    let start = (u64::from(u32::MAX) / 48) * 48;
    let mut q = JitterBuffer::new(spec(), start, u32::MAX, 4).unwrap();
    let mut out = [0; MAX_DATAGRAM];
    for i in 0..4 {
        let b = bytes(start + i * 48, u32::MAX.wrapping_add(i as u32));
        q.insert(packet(&b)).unwrap();
        q.pop(&mut out).unwrap();
        assert_eq!(packet(&out).source_frame(), start + i * 48);
    }
    let end = u64::MAX / 48 * 48;
    assert_eq!(spec().header(end, 0, &mut out), Err(Error::Timeline));
}
#[test]
fn lost_bursts_have_bounded_storage_and_new_epoch_cannot_replay() {
    let mut q = JitterBuffer::new(spec(), 0, 0, 8).unwrap();
    let mut out = [0; MAX_DATAGRAM];
    for i in 0..1000u64 {
        if !(50..120).contains(&i) && i % 17 != 0 {
            let b = bytes(i * 48, i as u32);
            q.insert(packet(&b)).unwrap();
        }
        q.pop(&mut out).unwrap();
        assert!(q.occupancy() <= 8);
    }
    assert!(q.stats().missing >= 70);
    let new = StreamSpec {
        session: 42,
        ..spec()
    };
    let mut q = JitterBuffer::new(new, 0, 0, 8).unwrap();
    assert_eq!(q.insert(packet(&bytes(0, 0))), Err(Error::Identity));
}
#[test]
fn wet_loss_and_recovery_fade_and_reset() {
    let mut g = WetGate::default();
    for _ in 0..241 {
        g.step(Some(0.5));
    }
    let mut prev = 1.0;
    for _ in 0..241 {
        let x = g.step(None);
        assert!(x <= prev);
        prev = x;
    }
    assert_eq!(prev, 0.0);
    let x = g.step(Some(0.5));
    assert!((x - 0.5 / 240.0).abs() < 1e-12);
    g.reset();
    assert_eq!(g.step(None), 0.0);
    assert_eq!(g.step(Some(f64::INFINITY)), 0.0);
}
#[test]
fn drift_diagnostics_do_not_create_a_brain_clock() {
    for ppm in [-1000.0, -100.0, -10.0, 10.0, 100.0, 1000.0] {
        let mut d = DriftObserver::default();
        d.observe(0, 0).unwrap();
        let frame = 28_800_000u64;
        let nanos = (frame as f64 / (48_000.0 * (1.0 + ppm / 1e6)) * 1e9).round() as u64;
        assert!((d.observe(frame, nanos).unwrap().unwrap() - ppm).abs() < 0.001);
        assert_eq!(d.observe(frame - 1, nanos + 1), Err(Error::Timeline));
        // A free-running Brain would drift by 2880 frames at 100 ppm/600 s.
        // A frame-following identity return always schedules source+384.
        let independent_error = frame as f64 * ppm / 1e6;
        assert!(independent_error.abs() >= 288.0);
        let mut b = [0; MAX_DATAGRAM];
        let s = StreamSpec {
            role: Role::WetReturn,
            delay_frames: 384,
            ..spec()
        };
        let n = s.header(frame, 0, &mut b).unwrap();
        assert_eq!(Packet::parse(&b[..n]).unwrap().output_frame(), frame + 384);
    }
}
fn command() -> Command {
    Command {
        session: 41,
        writer: 9,
        lease: 1,
        id: 1,
        expected_revision: 0,
        value_milli: -3000,
    }
}
#[test]
fn control_retry_identity_stale_and_reconnect_are_safe() {
    let mut a = Authority::new(41, 9).unwrap();
    let c = command();
    assert_eq!(a.apply(c).disposition, Disposition::Disconnected);
    assert_eq!(a.resync(1).unwrap(), (0, 0));
    assert_eq!(a.apply(c).disposition, Disposition::Applied);
    assert_eq!(a.apply(c).disposition, Disposition::Duplicate);
    assert_eq!(a.snapshot(), (1, -3000));
    assert_eq!(
        a.apply(Command {
            value_milli: 2000,
            ..c
        })
        .disposition,
        Disposition::Stale
    );
    assert_eq!(
        a.apply(Command { id: 2, ..c }).disposition,
        Disposition::Stale
    );
    a.disconnect();
    assert_eq!(a.apply(c).disposition, Disposition::Disconnected);
    assert_eq!(a.resync(2).unwrap(), (1, -3000));
    assert!(a.resync(1).is_err());
    assert_eq!(a.apply(c).disposition, Disposition::Identity);
    assert_eq!(
        a.apply(Command {
            lease: 2,
            expected_revision: 1,
            ..c
        })
        .disposition,
        Disposition::Applied
    );
}
#[test]
fn control_wire_is_fixed_and_validated() {
    let b = command().encode();
    assert_eq!(Command::parse(&b).unwrap(), command());
    for n in 0..64 {
        assert!(Command::parse(&b[..n]).is_err());
    }
    let mut bad = b;
    bad[63] = 1;
    assert!(Command::parse(&bad).is_err());
    let mut a = Authority::new(41, 9).unwrap();
    a.resync(1).unwrap();
    let ack = a.apply(command());
    assert_eq!(Ack::parse(&ack.encode()).unwrap(), ack);
    assert_eq!(
        a.apply(Command {
            id: 2,
            expected_revision: 1,
            value_milli: i32::MAX,
            ..command()
        })
        .disposition,
        Disposition::Invalid
    );
}

#[test]
fn stalled_network_cannot_block_local_processing_or_recorder_handoff() {
    let (mut producer, mut network, mut recorder) = CaptureFanout::<u64>::new(8, 16).unwrap();
    let mut local_frames = 0;
    for frame in 0..10_000 {
        local_frames += 48;
        producer.offer(frame);
        assert_eq!(recorder.pop().unwrap(), frame);
    }
    assert_eq!(local_frames, 480_000);
    assert_eq!(producer.network_drops, 9992);
    assert_eq!(producer.recorder_drops, 0);
    for frame in 0..8 {
        assert_eq!(network.pop().unwrap(), frame);
    }
    assert!(network.pop().is_err());
    producer.offer(10_000);
    assert_eq!(network.pop().unwrap(), 10_000);
    // The network host must discard old queued epochs on reconnect. Dry/recording
    // continuity here proves a handoff contract, not physical devices or NVMe.
}

#[test]
fn snapshot_retry_timeout_and_old_ack_refusal() {
    let mut a = Authority::new(41, 9).unwrap();
    let h = Hello {
        session: 41,
        writer: 9,
        lease: 1,
    };
    assert_eq!(Hello::parse(&h.encode()).unwrap(), h);
    let snap = a.hello(h).unwrap();
    assert_eq!(a.hello(h).unwrap(), snap);
    let ack = a.apply(command());
    assert!(ack.matches(command()));
    assert!(!ack.matches(Command {
        lease: 2,
        ..command()
    }));
    a.disconnect();
    assert!(a.hello(h).is_err());
    let snap = a.hello(Hello { lease: 2, ..h }).unwrap();
    assert_eq!(snap.revision, 1);
    assert_eq!(snap.value_milli, -3000);
}

#[test]
fn session_negotiation_refuses_overlap_gaps_and_mismatched_returns() {
    let a = spec();
    let send = StreamSpec {
        stream: 100,
        channels: 4,
        encoding: Encoding::Float32,
        role: Role::FxSend,
        ..a
    };
    let ret = StreamSpec {
        stream: 1100,
        role: Role::WetReturn,
        delay_frames: 384,
        ..send
    };
    assert!(validate_session(&[a, send, ret]).is_ok());
    assert!(validate_session(&[a, send]).is_err());
    assert!(validate_session(&[a, a]).is_err());
    assert!(
        validate_session(&[
            a,
            StreamSpec {
                stream: 2,
                first_channel: 4,
                ..a
            }
        ])
        .is_err()
    );
    assert!(
        validate_session(&[StreamSpec {
            first_channel: 1,
            ..a
        }])
        .is_err()
    );
    assert!(
        validate_session(&[
            a,
            StreamSpec {
                session: 42,
                ..send
            },
            ret
        ])
        .is_err()
    );
}

#[test]
fn rejected_old_identity_cannot_acknowledge_new_lease_command() {
    let mut a = Authority::new(41, 9).unwrap();
    a.resync(1).unwrap();
    let old = command();
    a.apply(old);
    a.disconnect();
    a.resync(2).unwrap();
    let new = Command {
        lease: 2,
        expected_revision: 1,
        ..old
    };
    let reply = a.apply(old);
    assert!(!reply.matches(new));
    assert!(reply.matches(old));
    assert_eq!(a.apply(new).disposition, Disposition::Applied);
}
#[test]
fn overdue_receive_is_late_even_before_playout_cursor_catches_up() {
    let s = StreamSpec {
        role: Role::WetReturn,
        delay_frames: 384,
        ..spec()
    };
    let mut b = [0; MAX_DATAGRAM];
    let n = s.encode_pcm(0, 0, &[17; 384], &mut b).unwrap();
    let mut q = JitterBuffer::new(s, 0, 0, 8).unwrap();
    assert_eq!(q.next_frame(), 0);
    assert_eq!(
        q.insert_wet_at(Packet::parse(&b[..n]).unwrap(), 384),
        Err(Error::Late)
    );
    assert!(q.pop(&mut b).unwrap().is_none());
}
fn request(w: &mut ControlWriter, t: u64) -> ControlRequest {
    w.poll(t, -3000).unwrap().unwrap()
}
fn answer(a: &mut Authority, r: ControlRequest) -> Ack {
    match r {
        ControlRequest::Hello(h) => a.hello(h).unwrap(),
        ControlRequest::Command(c) => a.apply(c),
    }
}
#[test]
fn writer_recovers_when_every_applied_command_ack_was_lost() {
    let mut a = Authority::new(41, 9).unwrap();
    let mut w = ControlWriter::new(41, 9, 1).unwrap();
    let ack = answer(&mut a, request(&mut w, 0));
    w.accept(ack, 1).unwrap();
    for t in [20_000_000, 120_000_000, 220_000_000] {
        answer(&mut a, request(&mut w, t));
    }
    assert_eq!(a.snapshot().0, 1); // All three ACKs discarded.
    let hello = request(&mut w, 320_000_000);
    assert!(matches!(hello, ControlRequest::Hello(_)));
    w.accept(answer(&mut a, hello), 320_000_001).unwrap();
    assert_eq!(w.revision(), 1);
    let c = request(&mut w, 340_000_000);
    assert_eq!(
        w.accept(answer(&mut a, c), 341_000_000).unwrap(),
        WriterEvent::Applied {
            roundtrip_ns: 1_000_000
        }
    );
    assert_eq!(a.snapshot().0, 2);
}
#[test]
fn writer_rotates_unanswered_hello_and_counts_latency_from_first_attempt() {
    let mut a = Authority::new(41, 9).unwrap();
    let mut w = ControlWriter::new(41, 9, 1).unwrap();
    let h = request(&mut w, 0);
    answer(&mut a, h); // lost snapshot, followed by timeout
    a.disconnect();
    request(&mut w, 100_000_000);
    request(&mut w, 200_000_000);
    let h = request(&mut w, 300_000_000);
    w.accept(answer(&mut a, h), 300_000_001).unwrap();
    let c = request(&mut w, 320_000_000);
    let first = answer(&mut a, c);
    assert_eq!(request(&mut w, 420_000_000), c);
    assert_eq!(
        w.accept(first, 430_000_000).unwrap(),
        WriterEvent::Applied {
            roundtrip_ns: 110_000_000
        }
    );
    assert_eq!(w.poll(400_000_000, 0), Err(Error::Timeline));
}

#[test]
fn parsed_metadata_is_a_copy_and_cannot_change_sample_bounds() {
    let s = StreamSpec {
        channels: 1,
        ..spec()
    };
    let mut b = [0; MAX_DATAGRAM];
    let n = s.encode_pcm(0, 0, &[17; 48], &mut b).unwrap();
    let p = Packet::parse(&b[..n]).unwrap();
    let mut copy = p.spec();
    copy.channels = 8;
    assert_ne!(copy, p.spec());
    assert_eq!(p.pcm(383), Err(Error::Size));
    assert_eq!(p.sample(47).unwrap(), 17.0 / 8_388_608.0);
}
