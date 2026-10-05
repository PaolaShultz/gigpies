use gigpies::{
    channel_processing::{Config, Prepared as DspPrepared},
    control_model::{Command, Request, Scope},
    mixer::{Mixer, Prepared},
    mixer_control::OfflineEngine,
    processing_wire::{ProcessingCommand, ProcessingReply, ProcessingRequest},
    show::Counter,
};
const SHOW: &str = "11111111-1111-4111-8111-111111111111";
fn audio(command: Command, id: u64, rev: u64, lease: Option<Counter>) -> Request {
    Request {
        contract: "C-AUDIO".into(),
        version: 1,
        show_id: SHOW.into(),
        module: "audio".into(),
        epoch: Counter(9),
        writer: Some("desk-1".into()),
        lease,
        request_id: Some(Counter(id)),
        expected_revision: Some(Counter(rev)),
        command,
    }
}
fn read() -> ProcessingRequest {
    ProcessingRequest {
        contract: "GP07-processing".into(),
        version: 1,
        show_id: SHOW.into(),
        module: "audio".into(),
        epoch: Counter(9),
        writer: None,
        lease: None,
        request_id: None,
        expected_revision: None,
        command: ProcessingCommand::ProcessingSnapshot {},
    }
}
fn set(config: Config, id: u64, rev: u64) -> ProcessingRequest {
    ProcessingRequest {
        writer: Some("desk-1".into()),
        lease: Some(Counter(1)),
        request_id: Some(Counter(id)),
        expected_revision: Some(Counter(rev)),
        command: ProcessingCommand::ProcessingSet {
            input: "input-01".into(),
            config,
        },
        ..read()
    }
}
fn engine() -> OfflineEngine {
    let mut e = OfflineEngine::new(SHOW, Counter(9), Counter(0), 0).unwrap();
    assert_eq!(
        e.handle(&audio(Command::Grant { scope: Scope::Foh }, 1, 0, None), 0)
            .unwrap()
            .outcome
            .unwrap()
            .body
            .granted_lease,
        Some(Counter(1))
    );
    e
}
fn pump(e: &mut OfflineEngine, n: usize, now: u64) -> Vec<[f64; 4]> {
    let input = vec![[0.5, 0., 0., 0., 0., 0., 0., 0.]; n];
    let mut out = vec![[0.; 4]; n];
    e.process(&input, &mut out, now).unwrap();
    out
}
fn config() -> Config {
    Config {
        eq_bypass: false,
        mid_gain_mdb: 6000,
        compressor_bypass: false,
        threshold_mdb: -12000,
        ratio_milli: 4000,
        makeup_mdb: 3000,
        ..Config::default()
    }
}
fn prep(c: Config) -> Prepared {
    Prepared::processing(0, DspPrepared::new(c).unwrap()).unwrap()
}
fn render(m: &mut Mixer, source: &[[f64; 8]]) -> Vec<[f64; 4]> {
    let mut out = vec![[0.; 4]; source.len()];
    m.process(source, &mut out).unwrap();
    out
}
fn near(a: f64, b: f64, t: f64) {
    assert!((a - b).abs() < t, "{a} != {b}, tolerance {t}");
}
fn foh_gain() -> f64 {
    10_f64.powf(-6. / 20.) / 2_f64.sqrt()
}

#[test]
fn authority_boundary_cache_cross_contract_and_no_implicit_renew() {
    let mut e = engine();
    let r = set(config(), 2, 0);
    let pending = e.handle_processing(&r, 1000).unwrap();
    assert_eq!(pending.state, "pending");
    assert_eq!(pending.effective_frame, Some(Counter(48)));
    assert_eq!(e.handle_processing(&r, 1001).unwrap(), pending);
    let mut changed = r.clone();
    if let ProcessingCommand::ProcessingSet { config, .. } = &mut changed.command {
        config.makeup_mdb = 4000;
    }
    assert_eq!(
        e.handle_processing(&changed, 1002)
            .unwrap()
            .reason
            .as_deref(),
        Some("reused_id")
    );
    assert_eq!(
        e.handle(&audio(Command::Renew {}, 2, 0, Some(Counter(1))), 1003)
            .unwrap()
            .outcome
            .unwrap()
            .body
            .reason
            .as_deref(),
        Some("reused_id")
    );
    pump(&mut e, 48, 1004);
    assert_eq!(e.revision(), Counter(0));
    assert!(e.take_processing_completions().is_empty());
    pump(&mut e, 1, 1004);
    assert_eq!(e.revision(), Counter(1));
    let final_reply = e.take_processing_completions().pop().unwrap();
    assert_eq!(final_reply.effective_frame, pending.effective_frame);
    assert_eq!(
        final_reply.snapshot.as_ref().unwrap().channels[0].transition_remaining_frames,
        239
    );
    assert_eq!(e.handle_processing(&r, 1005).unwrap(), final_reply);
    assert_eq!(
        e.handle(&audio(Command::Renew {}, 2, 0, Some(Counter(1))), 1006)
            .unwrap()
            .outcome
            .unwrap()
            .body
            .reason
            .as_deref(),
        Some("reused_id")
    );
    assert_eq!(
        e.handle_processing(&r, 2000).unwrap().reason.as_deref(),
        Some("lease")
    ); // grant at zero, never renewed
}
#[test]
fn expiry_boundary_cancels_without_audio_or_revision_change() {
    let mut e = engine();
    let before = e.processing_snapshot().unwrap();
    e.handle_processing(&set(config(), 2, 0), 1999).unwrap();
    let out = pump(&mut e, 100, 2000);
    assert_eq!(e.revision(), Counter(0));
    assert_eq!(e.processing_snapshot().unwrap().channels, before.channels);
    let p = e.take_processing_completions().pop().unwrap();
    assert_eq!(p.reason.as_deref(), Some("lease"));
    assert!(p.ticket.is_none());
    p.encode().unwrap();
    assert_eq!(out, pump(&mut engine(), 100, 0));
}
#[test]
fn pressure_is_not_admission_and_modes_do_not_replace_processing() {
    let mut e = engine();
    let r = set(config(), 2, 0);
    e.handle_processing(&r, 1).unwrap();
    assert_eq!(
        e.handle_processing(&set(Config::default(), 3, 1), 2)
            .unwrap()
            .state,
        "backpressure"
    );
    let mode = audio(
        Command::SetMode {
            mode: gigpies::control_model::Mode::Assist,
            bounds: vec![],
        },
        3,
        1,
        Some(Counter(1)),
    );
    assert_eq!(e.handle(&mode, 3).unwrap().state, "backpressure");
    pump(&mut e, 100, 4);
    e.take_processing_completions();
    assert_eq!(
        e.handle_processing(&set(Config::default(), 3, 1), 5)
            .unwrap()
            .state,
        "backpressure"
    );
    pump(&mut e, 188, 6);
    assert!(e.mixer().processing_ready());
    assert_eq!(e.handle(&mode, 7).unwrap().state, "pending");
    pump(&mut e, 100, 8);
    assert_eq!(
        e.processing_snapshot().unwrap().channels[0].target,
        config()
    );
    let p = e
        .handle_processing(&set(Config::default(), 4, 2), 9)
        .unwrap();
    assert_eq!(p.state, "pending");
    pump(&mut e, 300, 10);
    assert_eq!(e.revision(), Counter(3));
}
#[test]
fn shared_highwater_stale_refusal_scope_and_module_collisions() {
    let mut e = engine();
    assert_eq!(
        e.handle_processing(&set(config(), 2, 99), 1)
            .unwrap()
            .reason
            .as_deref(),
        Some("stale_revision")
    );
    assert_eq!(
        e.handle_processing(&set(config(), 2, 0), 2)
            .unwrap()
            .reason
            .as_deref(),
        Some("reused_id")
    );
    let module = audio(Command::Renew {}, 3, 0, Some(Counter(1)));
    assert_eq!(
        e.authorize_module(&module, "gp05-payload", 3).unwrap(),
        (false, None)
    );
    assert_eq!(
        e.handle_processing(&set(config(), 3, 0), 4)
            .unwrap()
            .reason
            .as_deref(),
        Some("reused_id")
    );
    e.handle_processing(&set(config(), 4, 0), 5).unwrap();
    pump(&mut e, 300, 6);
    e.take_processing_completions();
    assert!(
        e.authorize_module(
            &audio(Command::Renew {}, 4, 0, Some(Counter(1))),
            "gp05-payload",
            7
        )
        .is_err()
    );
    for id in 5..72 {
        let r = audio(Command::Renew {}, id, 1, Some(Counter(1)));
        e.handle(&r, 8).unwrap();
    }
    assert_eq!(
        e.handle_processing(&set(config(), 4, 0), 9)
            .unwrap()
            .reason
            .as_deref(),
        Some("expired_id")
    );
    let mut e = OfflineEngine::new(SHOW, Counter(9), Counter(0), 0).unwrap();
    e.handle(
        &audio(
            Command::Grant {
                scope: Scope::Monitor1,
            },
            1,
            0,
            None,
        ),
        0,
    )
    .unwrap();
    assert_eq!(
        e.handle_processing(&set(config(), 2, 0), 1)
            .unwrap()
            .reason
            .as_deref(),
        Some("scope")
    );
}
#[test]
fn invalid_complete_request_does_not_mutate_or_consume_id() {
    let mut e = engine();
    let mut bad = set(config(), 2, 0);
    if let ProcessingCommand::ProcessingSet { config, .. } = &mut bad.command {
        config.ratio_milli = 1050;
    }
    assert!(e.handle_processing(&bad, 1).is_err());
    assert_eq!(e.revision(), Counter(0));
    assert_eq!(
        e.handle_processing(&set(config(), 2, 0), 2).unwrap().state,
        "pending"
    );
    let mut wrong = set(config(), 2, 0);
    wrong.show_id = "22222222-2222-4222-8222-222222222222".into();
    let p = e.handle_processing(&wrong, 3).unwrap();
    assert_eq!(p.reason.as_deref(), Some("wrong_show"));
    assert_eq!(p.context.show_id, wrong.show_id);
    p.encode().unwrap();
}
#[test]
fn neutral_bypass_monitor_and_channel_isolation_are_exact() {
    let source: Vec<_> = (0..2048)
        .map(|n| [(n as f64 * 0.071).sin(), 0.25, 0., 0., 0., 0., 0., 0.])
        .collect();
    let mut a = Mixer::default();
    let expected = render(&mut a, &source);
    for c in [
        Config {
            eq_bypass: false,
            compressor_bypass: false,
            ..Config::default()
        },
        Config {
            low_gain_mdb: 12000,
            ratio_milli: 20000,
            makeup_mdb: 12000,
            ..Config::default()
        },
    ] {
        let mut b = Mixer::default();
        b.schedule(prep(c), 1).unwrap();
        assert_eq!(render(&mut b, &source), expected);
    }
    let mut b = Mixer::default();
    b.schedule(prep(config()), 1).unwrap();
    let changed = render(&mut b, &source);
    assert!(
        changed
            .iter()
            .zip(&expected)
            .any(|(a, b)| (a[0] - b[0]).abs() > 0.01)
    );
    for (a, b) in changed.iter().zip(&expected) {
        assert_eq!(a[2..], b[2..]);
    }
    let only_second = vec![[0., 0.25, 0., 0., 0., 0., 0., 0.]; 2048];
    let mut a = Mixer::default();
    let mut b = Mixer::default();
    b.schedule(prep(config()), 1).unwrap();
    assert_eq!(render(&mut a, &only_second), render(&mut b, &only_second));
}
#[test]
fn transition_partition_busy_retarget_noop_and_reset() {
    let c = Config {
        compressor_bypass: false,
        makeup_mdb: 6000,
        ..Config::default()
    };
    let source = vec![[1., 0., 0., 0., 0., 0., 0., 0.]; 600];
    let mut a = Mixer::default();
    let mut b = Mixer::default();
    for m in [&mut a, &mut b] {
        m.schedule(prep(c), 1).unwrap();
    }
    let expected = render(&mut a, &source);
    let mut actual = Vec::new();
    let mut offset = 0;
    for n in [1, 47, 1, 17, 100, 122, 1, 311] {
        actual.extend(render(&mut b, &source[offset..offset + n]));
        offset += n;
    }
    assert_eq!(actual, expected);
    near(expected[48][0], foh_gain(), 1e-15);
    near(
        expected[168][0],
        foh_gain() * (1. + 10_f64.powf(6. / 20.)) / 2.,
        1e-15,
    );
    near(expected[288][0], foh_gain() * 10_f64.powf(6. / 20.), 1e-15);
    let mut m = Mixer::default();
    m.schedule(prep(c), 1).unwrap();
    render(&mut m, &source[..49]);
    m.take_completion();
    assert!(m.schedule(prep(Config::default()), 2).is_err());
    render(&mut m, &source[..239]);
    m.schedule(prep(c), 2).unwrap();
    render(&mut m, &source[..49]);
    assert!(m.processing_ready());
    m.take_completion();
    let boundary = m.schedule(prep(Config::default()), 3).unwrap();
    let start = m.frame();
    let out = render(&mut m, &source[..300]);
    near(
        out[(boundary - start) as usize][0],
        foh_gain() * 10_f64.powf(6. / 20.),
        1e-15,
    );
    near(out[(boundary - start + 240) as usize][0], foh_gain(), 1e-15);
    assert_eq!(
        Mixer::default().processing_observations()[0].0,
        Config::default()
    );
}
#[test]
fn independent_eq_frequency_and_impulse_reference() {
    // Bell has exactly the requested gain at its center, independent of Q.
    for q in [100, 1000, 10000] {
        let c = Config {
            eq_bypass: false,
            mid_gain_mdb: 6000,
            mid_q_milli: q,
            ..Config::default()
        };
        let mut m = Mixer::default();
        m.schedule(prep(c), 1).unwrap();
        let source: Vec<_> = (0..24000)
            .map(|n| {
                [
                    (std::f64::consts::TAU * n as f64 / 48.).sin(),
                    0.,
                    0.,
                    0.,
                    0.,
                    0.,
                    0.,
                    0.,
                ]
            })
            .collect();
        let out = render(&mut m, &source);
        let rms = (out[12000..].iter().map(|v| v[0] * v[0]).sum::<f64>() / 12000.).sqrt();
        near(
            rms,
            foh_gain() * 10_f64.powf(6. / 20.) / 2_f64.sqrt(),
            1e-10,
        );
    }
    // Independent direct-form I recurrence for the RBJ bell impulse, using
    // separate x/y history rather than the production transposed-form state.
    let c = Config {
        eq_bypass: false,
        mid_gain_mdb: -9000,
        mid_hz: 1700,
        mid_q_milli: 2300,
        ..Config::default()
    };
    let a = 10_f64.powf(-9. / 40.);
    let w = std::f64::consts::TAU * 1700. / 48000.;
    let alpha = w.sin() / (2. * 2.3);
    let a0 = 1. + alpha / a;
    let b = [
        (1. + alpha * a) / a0,
        -2. * w.cos() / a0,
        (1. - alpha * a) / a0,
    ];
    let aa = [-2. * w.cos() / a0, (1. - alpha / a) / a0];
    let mut m = Mixer::default();
    m.schedule(prep(c), 1).unwrap();
    render(&mut m, &vec![[0.; 8]; 300]);
    let mut impulse = vec![[0.; 8]; 256];
    impulse[0][0] = 1.;
    let out = render(&mut m, &impulse);
    let mut ys = [0.; 2];
    let mut xs = [0.; 2];
    for (n, v) in out.iter().enumerate() {
        let x = if n == 0 { 1. } else { 0. };
        let y = b[0] * x + b[1] * xs[0] + b[2] * xs[1] - aa[0] * ys[0] - aa[1] * ys[1];
        near(v[0] / foh_gain(), y, 2e-14);
        xs = [x, xs[0]];
        ys = [y, ys[0]];
    }
    // Shelf asymptotic gain at DC, both + and - extremes, unity high shelf DC.
    for gain in [-12000, 12000] {
        let c = Config {
            eq_bypass: false,
            low_gain_mdb: gain,
            high_gain_mdb: 12000,
            ..Config::default()
        };
        let mut m = Mixer::default();
        m.schedule(prep(c), 1).unwrap();
        let out = render(&mut m, &vec![[1., 0., 0., 0., 0., 0., 0., 0.]; 10000]);
        near(
            out[9999][0] / foh_gain(),
            10_f64.powf(gain as f64 / 20000.),
            1e-10,
        );
    }
}
#[test]
fn compressor_static_soft_knee_attack_release_and_feedback() {
    let c = Config {
        compressor_bypass: false,
        threshold_mdb: -12000,
        ratio_milli: 4000,
        knee_mdb: 0,
        attack_us: 10000,
        release_ms: 100,
        ..Config::default()
    };
    let mut m = Mixer::default();
    m.schedule(prep(c), 1).unwrap();
    render(&mut m, &vec![[0.; 8]; 300]);
    let out = render(&mut m, &vec![[1., 0., 0., 0., 0., 0., 0., 0.]; 480]);
    // At 10 ms, attenuation reaches 1-exp(-1) of 9 dB.
    let reduction = -9. * (1. - (-1_f64).exp());
    near(
        out[479][0] / foh_gain(),
        10_f64.powf(reduction / 20.),
        1e-13,
    );
    let observed = m.processing_observations()[0].3.unwrap();
    assert_eq!(observed, (-reduction * 1000.).round() as i32);
    let low = 10_f64.powf(-60. / 20.);
    let out = render(&mut m, &vec![[low, 0., 0., 0., 0., 0., 0., 0.]; 4800]);
    near(
        out[4799][0] / (foh_gain() * low),
        10_f64.powf(reduction * (-1_f64).exp() / 20.),
        1e-12,
    );
    for (input_db, expected_reduction) in [(-24., 0.), (-12., 0.5625), (0., 9.)] {
        let c = Config {
            knee_mdb: 6000,
            attack_us: 100,
            ..c
        };
        let mut m = Mixer::default();
        m.schedule(prep(c), 1).unwrap();
        let x = 10_f64.powf(input_db / 20.);
        let out = render(&mut m, &vec![[x, 0., 0., 0., 0., 0., 0., 0.]; 10000]);
        near(
            out[9999][0] / (foh_gain() * x),
            10_f64.powf(-expected_reduction / 20.),
            1e-12,
        );
    }
}
#[test]
fn extremes_finite_fault_latch_and_range_steps() {
    for c in [
        Config {
            eq_bypass: false,
            low_hz: 20,
            mid_hz: 20000,
            high_hz: 20000,
            low_gain_mdb: 12000,
            mid_gain_mdb: -12000,
            high_gain_mdb: 12000,
            mid_q_milli: 100,
            compressor_bypass: false,
            threshold_mdb: -60000,
            ratio_milli: 20000,
            knee_mdb: 18000,
            attack_us: 100,
            release_ms: 2000,
            makeup_mdb: 12000,
        },
        Config {
            mid_q_milli: 10000,
            attack_us: 200000,
            ..config()
        },
    ] {
        let mut m = Mixer::default();
        m.schedule(prep(c), 1).unwrap();
        let source: Vec<_> = (0..10000)
            .map(|n| {
                [
                    if n % 2 == 0 { 1. } else { -1. },
                    0.,
                    0.,
                    0.,
                    0.,
                    0.,
                    0.,
                    0.,
                ]
            })
            .collect();
        assert!(
            render(&mut m, &source)
                .iter()
                .flatten()
                .all(|v| v.is_finite())
        );
    }
    for x in [f64::NAN, f64::INFINITY, f64::MAX] {
        let mut m = Mixer::default();
        m.schedule(prep(config()), 1).unwrap();
        let out = render(&mut m, &vec![[x; 8]; 400]);
        assert!(m.faulted());
        assert_eq!(out[399], [0.; 4]);
        assert!(m.processing_observations().iter().all(|v| v.3.is_none()));
    }
    let mut v = serde_json::to_value(Config::default()).unwrap();
    for (field, value) in [
        ("low_hz", 19),
        ("mid_q_milli", 150),
        ("makeup_mdb", 12001),
        ("ratio_milli", 999),
        ("attack_us", 101),
        ("release_ms", 2001),
    ] {
        let old = v[field].clone();
        v[field] = value.into();
        assert!(
            serde_json::from_value::<Config>(v.clone())
                .unwrap()
                .validate()
                .is_err()
        );
        v[field] = old;
    }
}
#[test]
fn strict_schema_and_reply_coherence() {
    let r = set(config(), 2, 0);
    let good = r.encode().unwrap();
    assert_eq!(ProcessingRequest::decode(&good).unwrap(), r);
    let value: serde_json::Value = serde_json::from_slice(&good).unwrap();
    for key in ["writer", "lease", "request_id", "expected_revision", "body"] {
        let mut v = value.clone();
        v.as_object_mut().unwrap().remove(key);
        assert!(ProcessingRequest::decode(&serde_json::to_vec(&v).unwrap()).is_err());
    }
    let mut v = value.clone();
    v["body"]["config"]
        .as_object_mut()
        .unwrap()
        .remove("eq_bypass");
    assert!(ProcessingRequest::decode(&serde_json::to_vec(&v).unwrap()).is_err());
    for bad in [
        String::from_utf8(good.clone())
            .unwrap()
            .replace("\"low_hz\":120", "\"low_hz\":120.0"),
        String::from_utf8(good.clone())
            .unwrap()
            .replace("\"low_hz\":120", "\"low_hz\":120,\"low_hz\":120"),
        format!("{} null", String::from_utf8(good).unwrap()),
        "[".repeat(65537),
    ] {
        assert!(ProcessingRequest::decode(bad.as_bytes()).is_err());
    }
    let mut e = engine();
    let pending = e.handle_processing(&r, 1).unwrap();
    pending.encode().unwrap();
    pump(&mut e, 300, 2);
    let final_reply = e.take_processing_completions().pop().unwrap();
    final_reply.encode().unwrap();
    let mut p = final_reply.clone();
    p.revision = Counter(9);
    assert!(p.encode().is_err());
    let mut p = final_reply.clone();
    p.snapshot.as_mut().unwrap().channels[0].gain_reduction_mdb = Some(1);
    assert!(p.encode().is_err());
    let mut p = e.handle_processing(&set(config(), 3, 99), 3).unwrap();
    p.context.lease = None;
    assert!(p.encode().is_err());
    let mut v = serde_json::to_value(pending).unwrap();
    v.as_object_mut().unwrap().remove("snapshot");
    assert!(ProcessingReply::decode(&serde_json::to_vec(&v).unwrap()).is_err());
}

fn producer_fixtures() -> std::collections::BTreeMap<&'static str, Vec<u8>> {
    let mut e = engine();
    let request = set(config(), 2, 0);
    let mut records = std::collections::BTreeMap::new();
    records.insert("snapshot-request.json", read().encode().unwrap());
    records.insert(
        "snapshot-reply.json",
        e.handle_processing(&read(), 1).unwrap().encode().unwrap(),
    );
    records.insert("set-request.json", request.encode().unwrap());
    records.insert(
        "set-pending.json",
        e.handle_processing(&request, 2).unwrap().encode().unwrap(),
    );
    records.insert(
        "backpressure.json",
        e.handle_processing(&set(Config::default(), 3, 0), 3)
            .unwrap()
            .encode()
            .unwrap(),
    );
    pump(&mut e, 49, 4);
    records.insert(
        "set-final.json",
        e.take_processing_completions()
            .pop()
            .unwrap()
            .encode()
            .unwrap(),
    );
    pump(&mut e, 239, 5);
    records.insert(
        "ready-reply.json",
        e.handle_processing(&read(), 6).unwrap().encode().unwrap(),
    );
    records.insert(
        "stale-revision.json",
        e.handle_processing(&set(Config::default(), 3, 0), 7)
            .unwrap()
            .encode()
            .unwrap(),
    );
    records
}
#[test]
fn producer_fixture_v1() {
    for (name, bytes) in producer_fixtures() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/gp07/v1")
            .join(name);
        assert_eq!(
            std::fs::read(&path).unwrap(),
            bytes,
            "producer fixture {name}"
        );
    }
}
#[test]
#[ignore = "opt-in fixture generation; review bytes and source provenance before consumer transfer"]
fn write_producer_fixture_v1() {
    for (name, bytes) in producer_fixtures() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/gp07/v1")
            .join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }
}

#[test]
fn faulted_final_consumes_shared_id_without_revision() {
    for at_boundary in [false, true] {
        let mut e = engine();
        let r = set(config(), 2, 0);
        if at_boundary {
            e.handle_processing(&r, 1).unwrap();
        }
        e.process(&[[f64::NAN; 8]], &mut [[0.; 4]], 2).unwrap();
        let p = if at_boundary {
            pump(&mut e, 100, 3);
            e.take_processing_completions().pop().unwrap()
        } else {
            e.handle_processing(&r, 3).unwrap()
        };
        assert_eq!(p.reason.as_deref(), Some("faulted"));
        assert_eq!(e.revision(), Counter(0));
        assert_eq!(e.handle_processing(&r, 4).unwrap(), p);
        assert_eq!(
            e.handle(&audio(Command::Renew {}, 2, 0, Some(Counter(1))), 5)
                .unwrap()
                .outcome
                .unwrap()
                .body
                .reason
                .as_deref(),
            Some("reused_id")
        );
    }
}

#[test]
fn all_eight_inputs_process_independently_and_high_shelf_nyquist_gain() {
    for i in 0..8 {
        let mut e = engine();
        let mut r = set(
            Config {
                compressor_bypass: false,
                makeup_mdb: 6000,
                ..Config::default()
            },
            2,
            0,
        );
        if let ProcessingCommand::ProcessingSet { input, .. } = &mut r.command {
            *input = format!("input-{:02}", i + 1);
        }
        assert_eq!(e.handle_processing(&r, 1).unwrap().state, "pending");
        e.process(&[[0.; 8]; 300], &mut [[0.; 4]; 300], 2).unwrap();
        e.take_processing_completions();
        for j in 0..8 {
            let mut source = [0.; 8];
            source[j] = 0.25;
            let mut output = [[0.; 4]];
            e.process(&[source], &mut output, 3).unwrap();
            let gain = if i == j { 10_f64.powf(6. / 20.) } else { 1. };
            near(output[0][0], 0.25 * foh_gain() * gain, 1e-15);
            assert_eq!(output[0][2], 0.25);
        }
        let snap = e.processing_snapshot().unwrap();
        for (j, c) in snap.channels.iter().enumerate() {
            assert_eq!(c.target.makeup_mdb, if i == j { 6000 } else { 0 });
        }
    }
    for gain in [-12000, 12000] {
        let c = Config {
            eq_bypass: false,
            low_gain_mdb: 12000,
            high_gain_mdb: gain,
            ..Config::default()
        };
        let mut m = Mixer::default();
        m.schedule(prep(c), 1).unwrap();
        let source: Vec<_> = (0..10000)
            .map(|i| {
                [
                    if i % 2 == 0 { 1. } else { -1. },
                    0.,
                    0.,
                    0.,
                    0.,
                    0.,
                    0.,
                    0.,
                ]
            })
            .collect();
        let out = render(&mut m, &source);
        near(
            out[9999][0] / foh_gain(),
            -10_f64.powf(gain as f64 / 20000.),
            1e-11,
        );
    }
}
#[test]
fn reduction_reporting_saturates_without_clipping_dsp_at_240_db() {
    let c = Config {
        compressor_bypass: false,
        threshold_mdb: -60000,
        ratio_milli: 20000,
        knee_mdb: 0,
        attack_us: 100,
        ..Config::default()
    };
    let mut m = Mixer::default();
    m.schedule(prep(c), 1).unwrap();
    let out = render(&mut m, &vec![[1e20, 0., 0., 0., 0., 0., 0., 0.]; 1000]);
    assert!(!m.faulted());
    assert_eq!(m.processing_observations()[0].3, Some(240000));
    let expected = 1e20 * 10_f64.powf((-460. * 0.95) / 20.);
    near(out[999][0] / foh_gain(), expected, 1e-13);
}

#[test]
fn boundary_lease_refusal_precedes_fault_refusal() {
    let mut e = engine();
    e.handle_processing(&set(config(), 2, 0), 1).unwrap();
    e.process(&[[f64::NAN; 8]], &mut [[0.; 4]], 2).unwrap();
    pump(&mut e, 100, 2000);
    assert_eq!(
        e.take_processing_completions()[0].reason.as_deref(),
        Some("lease")
    );
    assert_eq!(e.revision(), Counter(0));
}
