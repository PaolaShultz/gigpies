use gigpies::{
    control_model::{Command, Edit, Request, Scope, Target, Value},
    mixer::{Mixer, Prepared},
    mixer_control::OfflineEngine,
    show::Counter,
};
const SHOW: &str = "11111111-1111-4111-8111-111111111111";
fn edit(target: Target, value: Value) -> Edit {
    Edit { target, value }
}
fn fader(n: i32) -> Edit {
    edit(
        Target::Fader {
            input: "input-01".into(),
        },
        Value::Integer(n),
    )
}
fn req(command: Command, id: u64, revision: u64, lease: Option<Counter>) -> Request {
    Request {
        contract: "C-AUDIO".into(),
        version: 1,
        show_id: SHOW.into(),
        module: "audio".into(),
        epoch: Counter(9),
        writer: Some("desk-1".into()),
        lease,
        request_id: Some(Counter(id)),
        expected_revision: Some(Counter(revision)),
        command,
    }
}
fn run(m: &mut Mixer, n: usize) -> Vec<[f64; 4]> {
    let input = vec![[1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]; n];
    let mut output = vec![[0.0; 4]; n];
    m.process(&input, &mut output).unwrap();
    output
}
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 2e-14, "{a} != {b}");
}
#[test]
fn known_samples_monitor_tap_and_exact_endpoints() {
    let mut m = Mixer::new(48000);
    let out = run(&mut m, 1);
    near(out[0][0], 10_f64.powf(-6.0 / 20.0) / 2_f64.sqrt());
    near(out[0][2], 1.0);
    near(out[0][3], 0.001);
    assert_eq!(
        m.schedule(Prepared::edits(&[fader(-3000)]).unwrap(), 1)
            .unwrap(),
        48048
    );
    let out = run(&mut m, 288);
    near(out[46][0], 10_f64.powf(-6.0 / 20.0) / 2_f64.sqrt());
    near(out[287][0], 10_f64.powf(-3.0 / 20.0) / 2_f64.sqrt());
    assert!(out.iter().all(|v| v[2] == 1.0));
    assert_eq!(m.take_completion().unwrap().frame, 48048);
    near(m.coefficients()[0][0], 10_f64.powf(-3.0 / 20.0));
}
#[test]
fn partitions_interruptions_pan_mute_and_all_inputs() {
    let edits = vec![
        fader(-60000),
        edit(
            Target::Pan {
                input: "input-01".into(),
            },
            Value::Integer(100),
        ),
        edit(
            Target::Send {
                input: "input-01".into(),
                monitor: "monitor-2".into(),
            },
            Value::Integer(-3000),
        ),
    ];
    let mut a = Mixer::default();
    let mut b = Mixer::default();
    for m in [&mut a, &mut b] {
        m.schedule(Prepared::edits(&edits).unwrap(), 1).unwrap();
    }
    let expected = run(&mut a, 168);
    let mut actual = Vec::new();
    for n in [1, 47, 1, 39, 48, 32] {
        actual.extend(run(&mut b, n));
    }
    assert_eq!(expected, actual);
    for m in [&mut a, &mut b] {
        m.take_completion();
        m.schedule(Prepared::edits(&[fader(0)]).unwrap(), 2)
            .unwrap();
    }
    let expected = run(&mut a, 265);
    let mut actual = Vec::new();
    for n in [24, 1, 47, 193] {
        actual.extend(run(&mut b, n));
    }
    assert_eq!(expected, actual);
    assert_eq!(a.coefficients(), b.coefficients());
    near(a.coefficients()[0][0], 1.0);
    assert_eq!(a.coefficients()[0][1], 0.0);
    assert_eq!(a.coefficients()[0][2], 1.0);
    a.take_completion();
    a.schedule(
        Prepared::edits(&[edit(
            Target::Mute {
                input: "input-01".into(),
            },
            Value::Boolean(true),
        )])
        .unwrap(),
        3,
    )
    .unwrap();
    let mute = run(&mut a, 300);
    for o in mute {
        near(o[1], o[2]);
        near(o[3], o[2] * 10_f64.powf(-3.0 / 20.0));
    }
    let mut all = Mixer::default();
    let input = [[1.0; 8]; 1];
    let mut out = [[0.0; 4]; 1];
    all.process(&input, &mut out).unwrap();
    near(out[0][2], 8.0);
    near(out[0][0], 8.0 * 10_f64.powf(-6.0 / 20.0) / 2_f64.sqrt());
}
#[test]
fn capacity_invalid_transaction_and_nonfinite_fail_closed() {
    let mut m = Mixer::default();
    let initial = m.coefficients();
    assert!(Prepared::edits(&[fader(-3000), fader(-6000)]).is_err());
    m.schedule(Prepared::edits(&[fader(-3000)]).unwrap(), 1)
        .unwrap();
    assert!(
        m.schedule(Prepared::edits(&[fader(0)]).unwrap(), 2)
            .is_err()
    );
    assert_eq!(m.coefficients(), initial);
    let mut output = [[1.0; 4]; 2];
    m.process(&[[f64::NAN; 8], [1.0; 8]], &mut output).unwrap();
    assert_eq!(output, [[0.0; 4]; 2]);
    assert!(m.faulted());
    let mut m = Mixer::default();
    m.process(&[[f64::MAX; 8]], &mut output[..1]).unwrap();
    assert_eq!(output[0], [0.0; 4]);
    let mut m = Mixer::new(u64::MAX);
    assert!(
        m.schedule(Prepared::edits(&[fader(0)]).unwrap(), 1)
            .is_err()
    );
    assert!(m.process(&[[1.0; 8]], &mut output[..1]).is_err());
    assert_eq!(m.frame(), u64::MAX);
}
#[test]
fn pending_authority_retry_boundary_lost_ack_and_lease_cancellation() {
    let mut e = OfflineEngine::new(SHOW, Counter(9), Counter(12), 48000).unwrap();
    let l = e
        .handle(&req(Command::Grant { scope: Scope::Foh }, 1, 12, None), 0)
        .unwrap()
        .outcome
        .unwrap()
        .body
        .granted_lease
        .unwrap();
    let r = req(
        Command::Set {
            targets: vec![fader(-3000)],
        },
        41,
        12,
        Some(l),
    );
    let pending = e.handle_encoded(&r.encode().unwrap(), 0).unwrap();
    assert!(std::str::from_utf8(&pending).unwrap().contains("pending"));
    assert_eq!(e.revision(), Counter(12));
    assert_eq!(e.handle(&r, 0).unwrap().ticket, Some(Counter(1)));
    let mut out = [[0.0; 4]; 49];
    let replies = e
        .process(&[[1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]; 49], &mut out, 1)
        .unwrap();
    assert_eq!(replies[0].effective_frame, Some(Counter(48048)));
    assert_eq!(e.revision(), Counter(13));
    let retry = e.handle(&r, 1).unwrap();
    assert_eq!(retry.effective_frame, replies[0].effective_frame);
    let r43 = req(
        Command::Set {
            targets: vec![fader(0)],
        },
        43,
        13,
        Some(l),
    );
    assert_eq!(e.handle(&r43, 1).unwrap().state, "pending");
    e.process(&[[0.0; 8]; 48], &mut out[..48], 2).unwrap();
    let late = req(
        Command::Set {
            targets: vec![fader(-6000)],
        },
        42,
        14,
        Some(l),
    );
    assert_eq!(
        e.handle(&late, 2)
            .unwrap()
            .outcome
            .unwrap()
            .body
            .reason
            .as_deref(),
        Some("expired_id")
    );
    assert_eq!(
        e.handle(&r, 2).unwrap().effective_frame,
        Some(Counter(48048))
    );
    let r44 = req(
        Command::Set {
            targets: vec![fader(-9000)],
        },
        44,
        14,
        Some(l),
    );
    e.handle(&r44, 2).unwrap();
    let before = e.mixer().targets();
    let canceled = e.process(&[[0.0; 8]; 48], &mut out[..48], 2000).unwrap();
    assert_eq!(
        canceled[0].outcome.as_ref().unwrap().body.reason.as_deref(),
        Some("lease")
    );
    assert_eq!(e.mixer().targets(), before);
    assert_eq!(e.revision(), Counter(14));
}
#[test]
fn mode_freezes_actual_and_release_commits_proposal_continuously() {
    let mut e = OfflineEngine::new(SHOW, Counter(9), Counter(0), 0).unwrap();
    let l = e
        .handle(&req(Command::Grant { scope: Scope::Foh }, 1, 0, None), 0)
        .unwrap()
        .outcome
        .unwrap()
        .body
        .granted_lease
        .unwrap();
    let mut out = [[0.0; 4]; 100];
    e.handle(
        &req(
            Command::Set {
                targets: vec![fader(0)],
            },
            2,
            0,
            Some(l),
        ),
        0,
    )
    .unwrap();
    e.process(&[[0.0; 8]; 100], &mut out, 0).unwrap();
    e.handle(
        &req(
            Command::SetMode {
                mode: gigpies::control_model::Mode::Assist,
                bounds: vec![],
            },
            3,
            1,
            Some(l),
        ),
        1,
    )
    .unwrap();
    e.process(&[[0.0; 8]; 45], &mut out[..45], 1).unwrap();
    let held = e.mixer().coefficients();
    assert_eq!(held, e.mixer().targets());
    let initial = 10_f64.powf(-6.0 / 20.0);
    near(held[0][0], initial + (1.0 - initial) * 96.0 / 240.0);
    e.handle(
        &req(
            Command::Propose {
                targets: vec![fader(-3000)],
            },
            4,
            2,
            Some(l),
        ),
        2,
    )
    .unwrap();
    let t = Target::Fader {
        input: "input-01".into(),
    };
    let token = e
        .handle(
            &req(Command::PreviewRelease { targets: vec![t] }, 5, 3, Some(l)),
            2,
        )
        .unwrap()
        .outcome
        .unwrap()
        .body
        .preview
        .unwrap()
        .token;
    let release = req(Command::ReleasePreview { token }, 6, 3, Some(l));
    assert_eq!(e.handle(&release, 2).unwrap().state, "pending");
    e.process(
        &[[1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]; 48],
        &mut out[..48],
        3,
    )
    .unwrap();
    for sample in &out[..48] {
        near(sample[0], held[0][0] * held[0][1] * held[0][3]);
        near(sample[2], 1.0);
    }
    near(
        e.mixer().coefficients()[0][0],
        held[0][0] + (10_f64.powf(-3.0 / 20.0) - held[0][0]) / 240.0,
    );
    assert_eq!(e.revision(), Counter(4));
    assert!(e.snapshot().unwrap().authority.parameters[0].hold.is_none());
    assert_eq!(
        e.handle(&release, 3).unwrap().effective_frame,
        Some(Counter(192))
    );
    e.process(&[[0.0; 8]; 100], &mut out, 3).unwrap();
    e.process(&[[0.0; 8]; 100], &mut out, 3).unwrap();
    e.process(&[[0.0; 8]; 100], &mut out, 3).unwrap();
    near(e.mixer().coefficients()[0][0], 10_f64.powf(-3.0 / 20.0));
}

#[test]
fn pending_snapshots_preserve_sequence_clock_and_fence_metadata() {
    let mut e = OfflineEngine::new(SHOW, Counter(9), Counter(0), 0).unwrap();
    let l = e
        .handle(&req(Command::Grant { scope: Scope::Foh }, 1, 0, None), 0)
        .unwrap()
        .outcome
        .unwrap()
        .body
        .granted_lease
        .unwrap();
    e.handle(
        &req(
            Command::Set {
                targets: vec![fader(0)],
            },
            2,
            0,
            Some(l),
        ),
        1,
    )
    .unwrap();
    let mut snapshot = req(Command::Snapshot {}, 0, 0, None);
    snapshot.writer = None;
    snapshot.request_id = None;
    snapshot.expected_revision = None;
    let first = e
        .handle(&snapshot, 100)
        .unwrap()
        .snapshot
        .unwrap()
        .authority
        .sequence;
    assert_eq!(
        e.handle(&req(Command::Renew {}, 3, 0, Some(l)), 100)
            .unwrap()
            .state,
        "backpressure"
    );
    let mut out = [[0.0; 4]; 49];
    e.process(&[[0.0; 8]; 49], &mut out, 101).unwrap();
    let second = e
        .handle(&snapshot, 102)
        .unwrap()
        .snapshot
        .unwrap()
        .authority
        .sequence;
    assert!(second.0 > first.0);
    assert_eq!(
        e.handle(&req(Command::Renew {}, 3, 1, Some(l)), 50)
            .unwrap()
            .outcome
            .unwrap()
            .body
            .reason
            .as_deref(),
        Some("clock")
    );
}
#[test]
fn overlapping_ramp_matches_independent_reference() {
    let mut m = Mixer::default();
    m.schedule(Prepared::edits(&[fader(0)]).unwrap(), 1)
        .unwrap();
    let first = run(&mut m, 168);
    let start = 10_f64.powf(-6.0 / 20.0);
    for (frame, out) in first.iter().enumerate() {
        let amount = if frame < 48 {
            0.0
        } else {
            (frame - 48) as f64 / 240.0
        };
        near(out[0], (start + (1.0 - start) * amount) / 2_f64.sqrt());
    }
    m.take_completion();
    m.schedule(Prepared::edits(&[fader(-12000)]).unwrap(), 2)
        .unwrap();
    let second = run(&mut m, 265);
    let current = start + (1.0 - start) * 144.0 / 240.0;
    let destination = 10_f64.powf(-12.0 / 20.0);
    for (offset, out) in second.iter().enumerate() {
        let frame = 168 + offset;
        let coefficient = if frame < 192 {
            start + (1.0 - start) * (frame - 48) as f64 / 240.0
        } else {
            current + (destination - current) * ((frame - 192).min(240)) as f64 / 240.0
        };
        near(out[0], coefficient / 2_f64.sqrt());
    }
    near(m.coefficients()[0][0], destination);
}
#[test]
fn every_channel_and_send_maps_independently() {
    for i in 1..=8 {
        let mut m = Mixer::default();
        let input = format!("input-{i:02}");
        let edits = [
            edit(
                Target::Send {
                    input: input.clone(),
                    monitor: "monitor-1".into(),
                },
                Value::Integer(-6000),
            ),
            edit(
                Target::Send {
                    input: input.clone(),
                    monitor: "monitor-2".into(),
                },
                Value::Integer(0),
            ),
            edit(Target::Pan { input }, Value::Integer(-100)),
        ];
        m.schedule(Prepared::edits(&edits).unwrap(), 1).unwrap();
        let mut sources = [[0.0; 8]; 289];
        for row in &mut sources {
            row[i - 1] = 1.0;
        }
        let mut out = [[0.0; 4]; 289];
        m.process(&sources, &mut out).unwrap();
        near(out[288][0], 10_f64.powf(-6.0 / 20.0));
        assert_eq!(out[288][1], 0.0);
        near(out[288][2], 10_f64.powf(-6.0 / 20.0));
        near(out[288][3], 1.0);
    }
}
#[test]
fn invalid_pending_mutation_is_atomic_and_session_loss_preserves_mix() {
    let mut e = OfflineEngine::new(SHOW, Counter(9), Counter(0), 0).unwrap();
    let l = e
        .handle(&req(Command::Grant { scope: Scope::Foh }, 1, 0, None), 0)
        .unwrap()
        .outcome
        .unwrap()
        .body
        .granted_lease
        .unwrap();
    let r = req(
        Command::Set {
            targets: vec![fader(0)],
        },
        2,
        0,
        Some(l),
    );
    e.handle(&r, 0).unwrap();
    let mut changed = r.clone();
    changed.command = Command::Set {
        targets: vec![fader(-3000)],
    };
    assert_eq!(
        e.handle(&changed, 0)
            .unwrap()
            .outcome
            .unwrap()
            .body
            .reason
            .as_deref(),
        Some("reused_id")
    );
    assert_eq!(
        e.handle(
            &req(
                Command::Set {
                    targets: vec![fader(-9000)]
                },
                3,
                0,
                Some(l)
            ),
            0
        )
        .unwrap()
        .state,
        "backpressure"
    );
    assert_eq!(e.revision(), Counter(0));
    let mut out = [[0.0; 4]; 300];
    e.process(&[[1.0; 8]; 300], &mut out, 1).unwrap();
    let before = e.mixer().coefficients();
    e.handle(&req(Command::Release {}, 3, 1, Some(l)), 2)
        .unwrap();
    e.process(&[[1.0; 8]; 300], &mut out, 3000).unwrap();
    assert_eq!(e.mixer().coefficients(), before);
    assert_eq!(
        e.handle(&r, 3000)
            .unwrap()
            .outcome
            .unwrap()
            .body
            .reason
            .as_deref(),
        Some("lease")
    );
}
#[test]
fn quiet_writer_exact_rendered_retry_survives_other_scope_activity() {
    let mut e = OfflineEngine::new(SHOW, Counter(9), Counter(0), 0).unwrap();
    let l = e
        .handle(&req(Command::Grant { scope: Scope::Foh }, 1, 0, None), 0)
        .unwrap()
        .outcome
        .unwrap()
        .body
        .granted_lease
        .unwrap();
    let quiet = req(
        Command::Set {
            targets: vec![fader(0)],
        },
        2,
        0,
        Some(l),
    );
    e.handle(&quiet, 0).unwrap();
    let mut output = [[0.0; 4]; 49];
    let original = e
        .process(&[[0.0; 8]; 49], &mut output, 1)
        .unwrap()
        .remove(0)
        .encode()
        .unwrap();
    let mut monitor = req(
        Command::Grant {
            scope: Scope::Monitor1,
        },
        1,
        1,
        None,
    );
    monitor.writer = Some("monitor-writer".into());
    let ml = e
        .handle(&monitor, 1)
        .unwrap()
        .outcome
        .unwrap()
        .body
        .granted_lease
        .unwrap();
    monitor.lease = Some(ml);
    for id in 2..=300 {
        monitor.request_id = Some(Counter(id));
        monitor.expected_revision = Some(e.revision());
        monitor.command = Command::Set {
            targets: vec![edit(
                Target::Send {
                    input: "input-01".into(),
                    monitor: "monitor-1".into(),
                },
                Value::Integer(if id % 2 == 0 { -3000 } else { 0 }),
            )],
        };
        assert_eq!(e.handle(&monitor, 1).unwrap().state, "pending");
        e.process(&[[0.0; 8]; 49], &mut output, 1).unwrap();
    }
    assert_eq!(e.handle(&quiet, 100).unwrap().encode().unwrap(), original);
    assert_eq!(
        e.handle(&req(Command::Renew {}, 3, e.revision().0, Some(l)), 50)
            .unwrap()
            .outcome
            .unwrap()
            .body
            .reason
            .as_deref(),
        Some("clock")
    );
}
#[test]
fn preview_expiry_before_boundary_cancels_with_renewed_live_lease() {
    let mut e = OfflineEngine::new(SHOW, Counter(9), Counter(0), 0).unwrap();
    let l = e
        .handle(&req(Command::Grant { scope: Scope::Foh }, 1, 0, None), 0)
        .unwrap()
        .outcome
        .unwrap()
        .body
        .granted_lease
        .unwrap();
    let mut output = [[0.0; 4]; 49];
    e.handle(
        &req(
            Command::SetMode {
                mode: gigpies::control_model::Mode::Assist,
                bounds: vec![],
            },
            2,
            0,
            Some(l),
        ),
        0,
    )
    .unwrap();
    e.process(&[[0.0; 8]; 49], &mut output, 0).unwrap();
    e.handle(
        &req(
            Command::Propose {
                targets: vec![fader(0)],
            },
            3,
            1,
            Some(l),
        ),
        100,
    )
    .unwrap();
    let token = e
        .handle(
            &req(
                Command::PreviewRelease {
                    targets: vec![Target::Fader {
                        input: "input-01".into(),
                    }],
                },
                4,
                2,
                Some(l),
            ),
            100,
        )
        .unwrap()
        .outcome
        .unwrap()
        .body
        .preview
        .unwrap()
        .token;
    e.handle(&req(Command::Renew {}, 5, 2, Some(l)), 1000)
        .unwrap();
    let release = req(Command::ReleasePreview { token }, 6, 2, Some(l));
    let before = e.mixer().targets();
    assert_eq!(e.handle(&release, 2099).unwrap().state, "pending");
    let failure = e
        .process(&[[0.0; 8]; 49], &mut output, 2101)
        .unwrap()
        .remove(0);
    assert_eq!(failure.outcome.as_ref().unwrap().kind, "conflict");
    assert_eq!(e.revision(), Counter(2));
    assert_eq!(e.mixer().targets(), before);
    assert_eq!(
        e.handle(&release, 2101).unwrap().encode().unwrap(),
        failure.encode().unwrap()
    );
    assert!(e.snapshot().unwrap().authority.parameters[0].hold.is_some());
}
#[test]
fn backpressure_is_explicitly_nonadmitted_and_can_retry_after_drain() {
    let mut e = OfflineEngine::new(SHOW, Counter(9), Counter(0), 0).unwrap();
    let l = e
        .handle(&req(Command::Grant { scope: Scope::Foh }, 1, 0, None), 0)
        .unwrap()
        .outcome
        .unwrap()
        .body
        .granted_lease
        .unwrap();
    let first = req(
        Command::Set {
            targets: vec![fader(0)],
        },
        2,
        0,
        Some(l),
    );
    e.handle(&first, 0).unwrap();
    let mut waiting = req(
        Command::Set {
            targets: vec![fader(-3000)],
        },
        3,
        0,
        Some(l),
    );
    for _ in 0..2 {
        let result = e.handle(&waiting, 0).unwrap();
        assert_eq!(result.state, "backpressure");
        assert!(result.outcome.is_none());
    }
    let mut output = [[0.0; 4]; 49];
    e.process(&[[0.0; 8]; 49], &mut output, 1).unwrap();
    waiting.expected_revision = Some(Counter(1));
    assert_eq!(e.handle(&waiting, 1).unwrap().state, "pending");
    e.process(&[[0.0; 8]; 49], &mut output, 1).unwrap();
    assert_eq!(e.revision(), Counter(2));
}
#[test]
fn not_admitted_stale_revision_can_become_valid_after_pending_commit() {
    let mut e = OfflineEngine::new(SHOW, Counter(9), Counter(0), 0).unwrap();
    let l = e
        .handle(&req(Command::Grant { scope: Scope::Foh }, 1, 0, None), 0)
        .unwrap()
        .outcome
        .unwrap()
        .body
        .granted_lease
        .unwrap();
    e.handle(
        &req(
            Command::Set {
                targets: vec![fader(0)],
            },
            2,
            0,
            Some(l),
        ),
        0,
    )
    .unwrap();
    let next = req(
        Command::Set {
            targets: vec![fader(-3000)],
        },
        3,
        1,
        Some(l),
    );
    let pressure = e.handle(&next, 0).unwrap();
    assert_eq!(pressure.state, "backpressure");
    assert!(pressure.outcome.is_none());
    let mut output = [[0.0; 4]; 49];
    e.process(&[[0.0; 8]; 49], &mut output, 1).unwrap();
    assert_eq!(e.handle(&next, 1).unwrap().state, "pending");
    e.process(&[[0.0; 8]; 49], &mut output, 1).unwrap();
    assert_eq!(e.revision(), Counter(2));
}
#[test]
fn preview_cancel_and_revision_invalidation_preserve_rendered_hold() {
    let mut e = OfflineEngine::new(SHOW, Counter(9), Counter(0), 0).unwrap();
    let l = e
        .handle(&req(Command::Grant { scope: Scope::Foh }, 1, 0, None), 0)
        .unwrap()
        .outcome
        .unwrap()
        .body
        .granted_lease
        .unwrap();
    let mut output = [[0.0; 4]; 49];
    e.handle(
        &req(
            Command::SetMode {
                mode: gigpies::control_model::Mode::Assist,
                bounds: vec![],
            },
            2,
            0,
            Some(l),
        ),
        0,
    )
    .unwrap();
    e.process(&[[0.0; 8]; 49], &mut output, 0).unwrap();
    e.handle(
        &req(
            Command::Propose {
                targets: vec![fader(0)],
            },
            3,
            1,
            Some(l),
        ),
        0,
    )
    .unwrap();
    let target = Target::Fader {
        input: "input-01".into(),
    };
    let token = e
        .handle(
            &req(
                Command::PreviewRelease {
                    targets: vec![target.clone()],
                },
                4,
                2,
                Some(l),
            ),
            0,
        )
        .unwrap()
        .outcome
        .unwrap()
        .body
        .preview
        .unwrap()
        .token;
    let before = e.mixer().coefficients();
    e.handle(
        &req(
            Command::CancelPreview {
                token: token.clone(),
            },
            5,
            2,
            Some(l),
        ),
        0,
    )
    .unwrap();
    assert_eq!(
        e.handle(&req(Command::ReleasePreview { token }, 6, 2, Some(l)), 0)
            .unwrap()
            .outcome
            .unwrap()
            .kind,
        "rejected"
    );
    assert_eq!(e.mixer().coefficients(), before);
    let token = e
        .handle(
            &req(
                Command::PreviewRelease {
                    targets: vec![target],
                },
                7,
                2,
                Some(l),
            ),
            0,
        )
        .unwrap()
        .outcome
        .unwrap()
        .body
        .preview
        .unwrap()
        .token;
    e.handle(
        &req(
            Command::Propose {
                targets: vec![fader(-3000)],
            },
            8,
            2,
            Some(l),
        ),
        0,
    )
    .unwrap();
    assert_eq!(
        e.handle(&req(Command::ReleasePreview { token }, 9, 3, Some(l)), 0)
            .unwrap()
            .outcome
            .unwrap()
            .kind,
        "conflict"
    );
    assert_eq!(e.mixer().coefficients(), before);
    assert!(e.snapshot().unwrap().authority.parameters[0].hold.is_some());
}
#[test]
fn rendered_decoder_refuses_fabricated_capabilities_and_units() {
    let mut e = OfflineEngine::new(SHOW, Counter(9), Counter(0), 0).unwrap();
    let mut r = req(Command::Snapshot {}, 0, 0, None);
    r.writer = None;
    r.request_id = None;
    r.expected_revision = None;
    let reply = e.handle(&r, 0).unwrap();
    let bytes = reply.encode().unwrap();
    assert!(gigpies::control_model::Reply::decode(&bytes).is_err());
    assert!(gigpies::mixer_control::RenderedReply::decode(&bytes).is_ok());
    let mut bad = reply.clone();
    bad.snapshot.as_mut().unwrap().coefficients[0].current_nanogain[1] = 1_000_000_001;
    assert!(bad.encode().is_err());
    let mut bad = reply.clone();
    bad.snapshot.as_mut().unwrap().meters = Some(vec![0]);
    assert!(bad.encode().is_err());
    let mut raw: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    raw["capability_version"] = serde_json::json!(2);
    assert!(
        gigpies::mixer_control::RenderedReply::decode(&serde_json::to_vec(&raw).unwrap()).is_err()
    );
}

#[test]
fn pending_and_final_replies_carry_strict_request_context() {
    let mut e = OfflineEngine::new(SHOW, Counter(9), Counter(0), 0).unwrap();
    let l = e
        .handle(&req(Command::Grant { scope: Scope::Foh }, 1, 0, None), 0)
        .unwrap()
        .outcome
        .unwrap()
        .body
        .granted_lease
        .unwrap();
    let r = req(
        Command::Set {
            targets: vec![fader(0)],
        },
        41,
        0,
        Some(l),
    );
    let pending = e.handle(&r, 0).unwrap();
    assert_eq!(pending.context.request_id, r.request_id);
    assert_eq!(pending.context.writer, r.writer);
    assert_eq!(pending.context.epoch, r.epoch);
    assert_eq!(pending.context.lease, r.lease);
    assert!(gigpies::mixer_control::RenderedReply::decode(&pending.encode().unwrap()).is_ok());
    let mut out = [[0.0; 4]; 49];
    let final_reply = e.process(&[[0.0; 8]; 49], &mut out, 1).unwrap().remove(0);
    let mut wrong = final_reply.clone();
    wrong.context.request_id = Some(Counter(42));
    assert!(wrong.encode().is_err());
    let mut raw: serde_json::Value =
        serde_json::from_slice(&final_reply.encode().unwrap()).unwrap();
    raw["context"]["writer"] = serde_json::json!("other-desk");
    assert!(
        gigpies::mixer_control::RenderedReply::decode(&serde_json::to_vec(&raw).unwrap()).is_err()
    );
}
#[test]
fn primitive_transaction_covers_all_forty_targets_atomically() {
    let mut edits = Vec::new();
    for i in 1..=8 {
        let input = format!("input-{i:02}");
        edits.push(edit(
            Target::Fader {
                input: input.clone(),
            },
            Value::Integer(0),
        ));
        edits.push(edit(
            Target::Pan {
                input: input.clone(),
            },
            Value::Integer(100),
        ));
        edits.push(edit(
            Target::Mute {
                input: input.clone(),
            },
            Value::Boolean(false),
        ));
        for monitor in ["monitor-1", "monitor-2"] {
            edits.push(edit(
                Target::Send {
                    input: input.clone(),
                    monitor: monitor.into(),
                },
                Value::Integer(-3000),
            ));
        }
    }
    assert_eq!(edits.len(), 40);
    let mut mixer = Mixer::default();
    mixer.schedule(Prepared::edits(&edits).unwrap(), 1).unwrap();
    let mut output = [[0.0; 4]; 289];
    mixer.process(&[[1.0; 8]; 289], &mut output).unwrap();
    assert_eq!(output[288][0], 0.0);
    near(output[288][1], 8.0);
    near(output[288][2], 8.0 * 10_f64.powf(-3.0 / 20.0));
    near(output[288][3], 8.0 * 10_f64.powf(-3.0 / 20.0));
}
#[test]
fn expired_pending_retry_checks_lease_before_payload_or_ticket() {
    let mut e = OfflineEngine::new(SHOW, Counter(9), Counter(0), 0).unwrap();
    let l = e
        .handle(&req(Command::Grant { scope: Scope::Foh }, 1, 0, None), 0)
        .unwrap()
        .outcome
        .unwrap()
        .body
        .granted_lease
        .unwrap();
    let request = req(
        Command::Set {
            targets: vec![fader(0)],
        },
        2,
        0,
        Some(l),
    );
    e.handle(&request, 0).unwrap();
    let retry = e.handle(&request, 2000).unwrap();
    assert!(retry.ticket.is_none());
    assert_eq!(retry.outcome.unwrap().body.reason.as_deref(), Some("lease"));
    let mut changed = request.clone();
    changed.command = Command::Set {
        targets: vec![fader(-3000)],
    };
    assert_eq!(
        e.handle(&changed, 2000)
            .unwrap()
            .outcome
            .unwrap()
            .body
            .reason
            .as_deref(),
        Some("lease")
    );
    let mut output = [[0.0; 4]; 49];
    let before = e.mixer().targets();
    e.process(&[[0.0; 8]; 49], &mut output, 2000).unwrap();
    assert_eq!(e.mixer().targets(), before);
    assert_eq!(e.revision(), Counter(0));
}
