use gigpies::{control_model::*, show::Counter};
const SHOW: &str = "11111111-1111-4111-8111-111111111111";
fn request(command: Command, n: u64, rev: u64, lease: Option<Counter>) -> Request {
    Request {
        contract: "C-AUDIO".into(),
        version: 1,
        show_id: SHOW.into(),
        module: "audio".into(),
        epoch: Counter(9),
        writer: Some("desk-a".into()),
        lease,
        request_id: Some(Counter(n)),
        expected_revision: Some(Counter(rev)),
        command,
    }
}
fn setup() -> (Authority, Counter) {
    let mut a = Authority::new(SHOW, Counter(9), Counter(12)).unwrap();
    let p = encoded(
        &mut a,
        &request(Command::Grant { scope: Scope::Foh }, 1, 12, None),
        0,
    );
    (a, p.body.granted_lease.unwrap())
}
fn encoded(a: &mut Authority, r: &Request, t: u64) -> Reply {
    Reply::decode(&a.handle_encoded(&r.encode().unwrap(), t).unwrap()).unwrap()
}
fn edit(v: i32) -> Edit {
    Edit {
        target: Target::Fader {
            input: "input-01".into(),
        },
        value: Value::Integer(v),
    }
}
fn set(n: u64, rev: u64, lease: Counter, v: i32) -> Request {
    request(
        Command::Set {
            targets: vec![edit(v)],
        },
        n,
        rev,
        Some(lease),
    )
}
#[test]
fn e03_and_e03r_real_encoded_authority_retry_ordering() {
    let (mut a, l) = setup();
    let r41 = set(41, 12, l, -3000);
    let p41 = encoded(&mut a, &r41, 1);
    assert_eq!(p41.body.revision, Counter(13));
    assert_eq!(p41.body.effective_frame, None);
    assert_eq!(p41.body.ramp_frames, Some(240));
    assert_eq!(
        encoded(&mut a, &set(43, 13, l, -2000), 2).body.revision,
        Counter(14)
    );
    let state = a.state().0.to_vec();
    assert_eq!(
        encoded(&mut a, &set(42, 14, l, -1000), 3)
            .body
            .reason
            .as_deref(),
        Some("expired_id")
    );
    assert_eq!(encoded(&mut a, &r41, 4), p41);
    assert_eq!(a.state().0, state);
    assert_eq!(
        encoded(&mut a, &set(41, 14, l, -1000), 5)
            .body
            .reason
            .as_deref(),
        Some("reused_id")
    );
    let mut bad = r41.clone();
    bad.lease = Some(Counter(l.0 + 1));
    assert_eq!(
        encoded(&mut a, &bad, 6).body.reason.as_deref(),
        Some("lease")
    );
    bad = r41.clone();
    bad.epoch = Counter(8);
    assert_eq!(
        encoded(&mut a, &bad, 6).body.reason.as_deref(),
        Some("epoch")
    );
    bad = r41.clone();
    bad.show_id = "22222222-2222-4222-8222-222222222222".into();
    assert_eq!(
        encoded(&mut a, &bad, 6).body.reason.as_deref(),
        Some("wrong_show")
    );
    assert_eq!(
        encoded(&mut a, &r41, 2000).body.reason.as_deref(),
        Some("lease")
    );
    assert_eq!(a.revision(), Counter(14));
    assert_eq!(a.state().0, state);
}
#[test]
fn strict_codec_and_atomic_transactions() {
    let (mut a, l) = setup();
    let good = set(2, 12, l, -3000).encode().unwrap();
    let s = String::from_utf8(good.clone()).unwrap();
    for b in [
        s.replace("\"version\":1", "\"version\":1,\"version\":1"),
        s.replace("\"version\":1", "\"version\":1,\"unknown\":0"),
        s.replace("\"value\":-3000", "\"value\":-3000,\"unknown\":0"),
        s.replace("\"request_id\":\"2\"", "\"request_id\":\"02\""),
        s.replace("\"request_id\":\"2\"", "\"request_id\":2"),
        s.replace(
            "\"request_id\":\"2\"",
            "\"request_id\":\"18446744073709551616\"",
        ),
        s.replace("\"value\":-3000", "\"value\":-3000.0"),
        format!("{s} null"),
    ] {
        assert!(a.handle_encoded(b.as_bytes(), 1).is_err(), "{b}");
    }
    assert!(Request::decode(&[255]).is_err());
    assert!(Request::decode(&vec![b' '; 65537]).is_err());
    assert!(Request::decode(format!("{}0{}", "[".repeat(13), "]".repeat(13)).as_bytes()).is_err());
    let state = a.state().0.to_vec();
    let mut r = set(2, 12, l, -3000);
    r.command = Command::Set {
        targets: vec![
            edit(-3000),
            Edit {
                target: Target::Pan {
                    input: "input-02".into(),
                },
                value: Value::Integer(101),
            },
        ],
    };
    assert_eq!(encoded(&mut a, &r, 1).body.reason.as_deref(), Some("range"));
    assert_eq!(a.state().0, state);
    assert_eq!(a.revision(), Counter(12));
    for (n, targets, reason) in [
        (3, vec![edit(-3001)], "range"),
        (4, vec![edit(-3000); 65], "capacity"),
        (5, vec![edit(-3000); 2], "target"),
        (
            6,
            vec![Edit {
                target: Target::Send {
                    input: "input-01".into(),
                    monitor: "monitor-1".into(),
                },
                value: Value::Integer(0),
            }],
            "scope",
        ),
    ] {
        r = request(Command::Set { targets }, n, 12, Some(l));
        assert_eq!(encoded(&mut a, &r, 2).body.reason.as_deref(), Some(reason));
        assert_eq!(a.state().0, state);
        assert_eq!(a.revision(), Counter(12));
    }
    assert_eq!(encoded(&mut a, &set(7, 11, l, -3000), 3).kind, "conflict");
    assert_eq!(a.state().0, state);
}
#[test]
fn cached_refusal_eviction_high_water_and_overflow() {
    let (mut a, l) = setup();
    let bad = set(2, 11, l, -3000);
    let old = encoded(&mut a, &bad, 1);
    let p = encoded(&mut a, &set(3, 12, l, -3000), 2);
    assert_eq!(p.body.revision, Counter(13));
    assert_eq!(encoded(&mut a, &bad, 3), old);
    for n in 4..=68 {
        let rev = a.revision().0;
        assert_eq!(encoded(&mut a, &set(n, rev, l, -3000), 4).kind, "applied");
    }
    assert_eq!(
        encoded(&mut a, &bad, 5).body.reason.as_deref(),
        Some("expired_id")
    );
    let mut a = Authority::new(SHOW, Counter(9), Counter(u64::MAX)).unwrap();
    let l = encoded(
        &mut a,
        &request(Command::Grant { scope: Scope::Foh }, 1, u64::MAX, None),
        0,
    )
    .body
    .granted_lease
    .unwrap();
    let state = a.state().0.to_vec();
    assert_eq!(
        encoded(&mut a, &set(2, u64::MAX, l, -3000), 1)
            .body
            .reason
            .as_deref(),
        Some("capacity")
    );
    assert_eq!(a.state().0, state);
    assert_eq!(a.revision(), Counter(u64::MAX));
    assert_eq!(
        encoded(
            &mut a,
            &request(Command::Renew {}, 3, u64::MAX, Some(l)),
            u64::MAX
        )
        .body
        .reason
        .as_deref(),
        Some("lease")
    );
}
#[test]
fn lease_scope_expiry_renew_and_retired_writer() {
    let (mut a, l) = setup();
    let mut other = request(Command::Grant { scope: Scope::Foh }, 1, 12, None);
    other.writer = Some("desk-b".into());
    assert_eq!(encoded(&mut a, &other, 1).kind, "busy");
    let renew = request(Command::Renew {}, 2, 12, Some(l));
    assert_eq!(
        encoded(&mut a, &renew, 1500).body.lease_remaining_ms,
        Some(2000)
    );
    assert_eq!(encoded(&mut a, &set(3, 12, l, -3000), 2500).kind, "applied");
    assert_eq!(
        encoded(&mut a, &set(4, 13, l, -4000), 3500)
            .body
            .reason
            .as_deref(),
        Some("lease")
    );
    other.expected_revision = Some(Counter(13));
    assert_eq!(encoded(&mut a, &other, 3500).kind, "applied");
    assert_eq!(
        encoded(
            &mut a,
            &request(
                Command::Grant {
                    scope: Scope::Monitor1
                },
                1,
                13,
                None
            ),
            3501
        )
        .body
        .reason
        .as_deref(),
        Some("lease")
    );
    assert!(a.state().0.iter().any(|p| p.hold.is_some()));
    let mut release = other.clone();
    release.command = Command::Release {};
    release.lease = Some(Counter(2));
    release.request_id = Some(Counter(2));
    assert_eq!(encoded(&mut a, &release, 3502).kind, "applied");
    assert_eq!(
        encoded(&mut a, &release, 3503).body.reason.as_deref(),
        Some("lease")
    );
    assert_eq!(
        encoded(&mut a, &other, 3504).body.reason.as_deref(),
        Some("lease")
    );
    let mut late = other;
    late.writer = Some("desk-c".into());
    assert_eq!(
        encoded(&mut a, &late, u64::MAX).body.reason.as_deref(),
        Some("capacity")
    );
}
#[test]
fn holds_modes_preview_truthful_dsp_limitation_and_monitor_independence() {
    let (mut a, l) = setup();
    encoded(&mut a, &set(2, 12, l, -3000), 1);
    let monitors: Vec<_> = a
        .state()
        .0
        .iter()
        .filter(|p| matches!(p.target, Target::Send { .. }))
        .cloned()
        .collect();
    let mode = request(
        Command::SetMode {
            mode: Mode::Assist,
            bounds: vec![],
        },
        3,
        13,
        Some(l),
    );
    assert_eq!(encoded(&mut a, &mode, 2).body.revision, Counter(14));
    let prop = request(
        Command::Propose {
            targets: vec![edit(-2000)],
        },
        4,
        14,
        Some(l),
    );
    assert_eq!(encoded(&mut a, &prop, 3).body.revision, Counter(15));
    let preview = request(
        Command::PreviewRelease {
            targets: vec![edit(0).target],
        },
        5,
        15,
        Some(l),
    );
    let p = encoded(&mut a, &preview, 4).body.preview.unwrap();
    assert_eq!(p.ramp_frames, 240);
    let state = a.state().0.to_vec();
    let commit = request(
        Command::ReleasePreview {
            token: p.token.clone(),
        },
        6,
        15,
        Some(l),
    );
    assert_eq!(
        encoded(&mut a, &commit, 5).body.reason.as_deref(),
        Some("unavailable")
    );
    assert_eq!(a.state().0, state);
    assert_eq!(a.revision(), Counter(15));
    let cancel = request(Command::CancelPreview { token: p.token }, 7, 15, Some(l));
    assert_eq!(encoded(&mut a, &cancel, 6).kind, "applied");
    assert_eq!(a.state().0, state);
    let r = request(
        Command::Set {
            targets: vec![Edit {
                target: Target::Mute {
                    input: "input-01".into(),
                },
                value: Value::Boolean(true),
            }],
        },
        8,
        15,
        Some(l),
    );
    encoded(&mut a, &r, 7);
    assert_eq!(
        a.state()
            .0
            .iter()
            .filter(|p| matches!(p.target, Target::Send { .. }))
            .cloned()
            .collect::<Vec<_>>(),
        monitors
    );
    assert!(a.state().0.iter().all(|p| p.actual.is_none()));
    let snap = a.snapshot().unwrap();
    assert_eq!(snap.inputs.len(), 8);
    assert_eq!(snap.monitors.len(), 2);
    assert!(!snap.rendered_application);
    assert!(!snap.release_commit);
    assert_eq!(snap.acquisition_frame, None);
    assert_eq!(snap.validity, "unavailable");
}
#[test]
fn snapshot_page_collection_and_freshness() {
    let (mut a, _) = setup();
    let mut p = a.snapshot().unwrap();
    p.page_count = 2;
    let mut q = p.clone();
    q.page = 1;
    q.parameters = p.parameters.split_off(20);
    let mut pages = SnapshotPages::default();
    assert!(pages.push(p.clone(), 0).unwrap().is_none());
    assert_eq!(pages.push(q.clone(), 2000).unwrap().unwrap().len(), 2);
    assert!(pages.push(p.clone(), 0).unwrap().is_none());
    q.revision = Counter(99);
    assert!(pages.push(q, 1).is_err());
    assert!(pages.push(p.clone(), 0).unwrap().is_none());
    assert!(pages.push(p.clone(), 1).is_err());
    assert!(pages.push(p.clone(), 0).unwrap().is_none());
    p.page = 1;
    assert!(pages.push(p, 2001).is_err());
}
// One-time evidence generation is explicitly opt-in; normal tests replay the
// retained encoded corpus against real authority logic.
fn corpus_scenario(name: &str) -> Vec<(Request, u64)> {
    let l = Counter(1);
    let grant = request(Command::Grant { scope: Scope::Foh }, 1, 12, None);
    match name {
        "e03" => vec![
            (grant, 0),
            (set(41, 12, l, -3000), 1),
            (set(41, 12, l, -3000), 2),
            (set(42, 13, l, -3001), 3),
            (set(43, 11, l, -3000), 4),
        ],
        "e03r" => vec![
            (grant, 0),
            (set(41, 12, l, -3000), 1),
            (set(43, 13, l, -2000), 2),
            (set(42, 14, l, -1000), 3),
            (set(41, 12, l, -3000), 4),
        ],
        "e03m" => vec![
            (grant, 0),
            (set(41, 12, l, -3000), 1),
            (
                request(
                    Command::Set {
                        targets: vec![Edit {
                            target: Target::Mute {
                                input: "input-01".into(),
                            },
                            value: Value::Boolean(true),
                        }],
                    },
                    42,
                    13,
                    Some(l),
                ),
                2,
            ),
        ],
        _ => unreachable!(),
    }
}
#[test]
#[ignore = "one-time versioned corpus regeneration; see fixture README"]
fn regenerate_gp02_corpus() {
    let destination =
        std::env::var("GP02_CORPUS_OUTPUT").expect("explicit output directory required");
    let dir = std::path::Path::new(&destination);
    assert!(dir.is_dir());
    for name in ["e03", "e03r", "e03m"] {
        let mut a = Authority::new(SHOW, Counter(9), Counter(12)).unwrap();
        std::fs::write(
            dir.join(format!("{name}-initial-snapshot.json")),
            serde_json::to_vec_pretty(&a.snapshot().unwrap()).unwrap(),
        )
        .unwrap();
        for (index, (r, now)) in corpus_scenario(name).iter().enumerate() {
            let bytes = r.encode().unwrap();
            let response = a.handle_encoded(&bytes, *now).unwrap();
            std::fs::write(
                dir.join(format!("{name}-{:02}-request.json", index + 1)),
                bytes,
            )
            .unwrap();
            std::fs::write(
                dir.join(format!("{name}-{:02}-response.json", index + 1)),
                response,
            )
            .unwrap();
        }
        std::fs::write(
            dir.join(format!("{name}-final-snapshot.json")),
            serde_json::to_vec_pretty(&a.snapshot().unwrap()).unwrap(),
        )
        .unwrap();
    }
}
#[test]
fn versioned_encoded_corpus_replays_real_authority() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/gp02/v1");
    for name in ["e03", "e03r", "e03m"] {
        let mut a = Authority::new(SHOW, Counter(9), Counter(12)).unwrap();
        let initial: Snapshot = serde_json::from_slice(
            &std::fs::read(dir.join(format!("{name}-initial-snapshot.json"))).unwrap(),
        )
        .unwrap();
        assert_eq!(initial, a.snapshot().unwrap());
        for (i, (r, now)) in corpus_scenario(name).iter().enumerate() {
            let bytes =
                std::fs::read(dir.join(format!("{name}-{:02}-request.json", i + 1))).unwrap();
            assert_eq!(Request::decode(&bytes).unwrap(), *r);
            assert_eq!(
                a.handle_encoded(&bytes, *now).unwrap(),
                std::fs::read(dir.join(format!("{name}-{:02}-response.json", i + 1))).unwrap()
            );
        }
        let final_state: Snapshot = serde_json::from_slice(
            &std::fs::read(dir.join(format!("{name}-final-snapshot.json"))).unwrap(),
        )
        .unwrap();
        assert_eq!(final_state, a.snapshot().unwrap());
    }
}
#[test]
fn malformed_provider_snapshots_and_replies_are_never_trusted() {
    let (mut a, l) = setup();
    let snapshot = a.snapshot().unwrap();
    for mutation in 0..10 {
        let mut s = snapshot.clone();
        match mutation {
            0 => s.inputs[0] = "input-02".into(),
            1 => s.monitors.pop().map(|_| ()).unwrap(),
            2 => s.parameters[0].target_value = Value::Integer(-3001),
            3 => s.parameters[0].actual = Some(Value::Integer(0)),
            4 => s.parameters[1] = s.parameters[0].clone(),
            5 => {
                s.parameters.pop();
            }
            6 => s.modes[0] = s.modes[1],
            7 => s.rendered_application = true,
            8 => s.sequence = Counter(0),
            _ => s.session_history_capacity = 0,
        };
        assert!(Snapshot::decode(&serde_json::to_vec(&s).unwrap()).is_err());
    }
    let reply = encoded(&mut a, &set(2, 12, l, -3000), 1);
    for mutation in 0..6 {
        let mut p = reply.clone();
        match mutation {
            0 => p.version = 2,
            1 => p.kind = "accepted_pending".into(),
            2 => p.body.effective_frame = Some(Counter(48048)),
            3 => p.body.reason = Some("range".into()),
            4 => p.writer = Some("Bad Writer".into()),
            _ => p.body.ramp_frames = Some(0),
        }
        assert!(Reply::decode(&serde_json::to_vec(&p).unwrap()).is_err());
    }
    let mut p = snapshot.clone();
    p.page_count = 2;
    p.parameters.truncate(20);
    let mut q = p.clone();
    q.page = 1;
    let mut pages = SnapshotPages::default();
    assert!(pages.push(p, 0).unwrap().is_none());
    assert!(pages.push(q, 1).is_err());
    let mut p = snapshot.clone();
    p.page_count = 2;
    p.parameters.truncate(20);
    let mut q = snapshot;
    q.page_count = 2;
    q.page = 1;
    q.parameters.drain(..21);
    assert!(pages.push(p, 0).unwrap().is_none());
    assert!(pages.push(q, 1).is_err());
}
#[test]
fn encoded_command_body_and_token_id_rejections() {
    let (mut a, l) = setup();
    let valid = request(Command::Renew {}, 2, 12, Some(l)).encode().unwrap();
    let text = String::from_utf8(valid).unwrap();
    assert!(
        Request::decode(
            text.replace("\"body\":{}", "\"body\":{\"unexpected\":1}")
                .as_bytes()
        )
        .is_err()
    );
    let mut r = request(
        Command::CancelPreview {
            token: "Bad Token".into(),
        },
        2,
        12,
        Some(l),
    );
    assert!(r.encode().is_err());
    r.command = Command::Set {
        targets: vec![Edit {
            target: Target::Fader {
                input: "../input".into(),
            },
            value: Value::Integer(0),
        }],
    };
    assert!(r.encode().is_err());
    let mut grant = request(
        Command::Grant {
            scope: Scope::Monitor1,
        },
        1,
        12,
        None,
    );
    grant.expected_revision = None;
    assert!(grant.encode().is_err());
    let before = a.state().0.to_vec();
    assert!(
        a.handle_encoded(
            text.replace("\"body\":{}", "\"body\":{\"unexpected\":1}")
                .as_bytes(),
            1
        )
        .is_err()
    );
    assert_eq!(a.state().0, before);
}
#[test]
fn auto_requires_explicit_valid_bounds_and_preserves_human_holds() {
    let (mut a, l) = setup();
    encoded(&mut a, &set(2, 12, l, -3000), 1);
    let state = a.state().0.to_vec();
    let req = request(
        Command::SetMode {
            mode: Mode::Auto,
            bounds: vec![],
        },
        3,
        13,
        Some(l),
    );
    assert_eq!(
        encoded(&mut a, &req, 2).body.reason.as_deref(),
        Some("scope")
    );
    assert_eq!(a.state().0, state);
    let bound = AutoBound {
        target: edit(0).target,
        min: Value::Integer(-6000),
        max: Value::Integer(0),
    };
    let req = request(
        Command::SetMode {
            mode: Mode::Auto,
            bounds: vec![bound.clone()],
        },
        4,
        13,
        Some(l),
    );
    assert_eq!(encoded(&mut a, &req, 3).body.revision, Counter(14));
    let snapshot = a.snapshot().unwrap();
    snapshot.validate().unwrap();
    assert_eq!(snapshot.automation_bounds, vec![bound]);
    assert!(
        snapshot
            .parameters
            .iter()
            .filter(|p| p.target == edit(0).target)
            .all(|p| p.hold == Some(Value::Integer(-3000)) && p.actual.is_none())
    );
    let mut bad = AutoBound {
        target: Target::Mute {
            input: "input-01".into(),
        },
        min: Value::Boolean(false),
        max: Value::Boolean(true),
    };
    let req = request(
        Command::SetMode {
            mode: Mode::Auto,
            bounds: vec![bad.clone()],
        },
        5,
        14,
        Some(l),
    );
    assert_eq!(
        encoded(&mut a, &req, 4).body.reason.as_deref(),
        Some("target")
    );
    bad.target = edit(0).target;
    bad.min = Value::Integer(100);
    bad.max = Value::Integer(0);
    let req = request(
        Command::SetMode {
            mode: Mode::Auto,
            bounds: vec![bad],
        },
        6,
        14,
        Some(l),
    );
    assert_eq!(
        encoded(&mut a, &req, 5).body.reason.as_deref(),
        Some("range")
    );
    assert_eq!(a.revision(), Counter(14));
}
#[test]
fn fresh_writer_starts_at_one_and_request_high_water_never_wraps() {
    let mut a = Authority::new(SHOW, Counter(9), Counter(12)).unwrap();
    let before = a.state().0.to_vec();
    let bad = request(Command::Grant { scope: Scope::Foh }, 2, 12, None);
    assert_eq!(
        encoded(&mut a, &bad, 0).body.reason.as_deref(),
        Some("range")
    );
    assert_eq!(a.state().0, before);
    assert_eq!(a.revision(), Counter(12));
    let l = encoded(
        &mut a,
        &request(Command::Grant { scope: Scope::Foh }, 1, 12, None),
        0,
    )
    .body
    .granted_lease
    .unwrap();
    assert_eq!(
        encoded(&mut a, &set(u64::MAX, 12, l, -3000), 1)
            .body
            .revision,
        Counter(13)
    );
    let before = a.state().0.to_vec();
    assert_eq!(
        encoded(&mut a, &set(2, 13, l, -1000), 2)
            .body
            .reason
            .as_deref(),
        Some("expired_id")
    );
    assert_eq!(a.state().0, before);
    assert_eq!(a.revision(), Counter(13));
}
