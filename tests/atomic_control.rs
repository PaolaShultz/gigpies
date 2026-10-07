use gigpies::{
    control_model::{Command, Edit, Request, Scope, Target, Value},
    lease_maintenance::{self as maintenance, Reason},
    mixer_control::OfflineEngine,
    processing_wire::{ProcessingCommand, ProcessingRequest},
    show::Counter,
    topology::EngineTopology,
};
const SHOW: &str = "11111111-1111-4111-8111-111111111111";
fn request(
    engine: &OfflineEngine,
    writer: &str,
    lease: Option<Counter>,
    id: u64,
    command: Command,
) -> Request {
    Request {
        contract: "C-AUDIO".into(),
        version: 2,
        show_id: SHOW.into(),
        module: "audio".into(),
        epoch: Counter(1),
        writer: Some(writer.into()),
        lease,
        request_id: Some(Counter(id)),
        expected_revision: Some(engine.revision()),
        command,
    }
}
fn grant(engine: &mut OfflineEngine, writer: &str, scope: Scope) -> Counter {
    let r = request(engine, writer, None, 1, Command::Grant { scope });
    engine
        .handle(&r, 0)
        .unwrap()
        .outcome
        .unwrap()
        .body
        .granted_lease
        .unwrap()
}
fn engine() -> OfflineEngine {
    let mut e = OfflineEngine::with_topology(
        SHOW,
        Counter(1),
        Counter(0),
        0,
        EngineTopology::software(16, 5, 0).unwrap(),
    )
    .unwrap();
    e.rearm_outputs().unwrap();
    e
}
fn maintenance(lease: Counter) -> maintenance::Request {
    maintenance::Request {
        contract: maintenance::CONTRACT.into(),
        version: 1,
        kind: "maintain".into(),
        show_id: SHOW.into(),
        module: "audio".into(),
        epoch: Counter(1),
        authenticated_session: Counter(1),
        writer: "maintainer".into(),
        capability_generation: Counter(1),
        map_generation: Counter(1),
        maintenance_id: Counter(1),
        scope: Scope::TalkbackDestinations,
        lease,
    }
}
fn render(e: &mut OfflineEngine, frames: usize, now: u64) {
    e.process_interleaved(&vec![0.; frames * 16], &mut vec![0.; frames * 7], now)
        .unwrap();
}
#[test]
fn atomic_all_staged_authority_classes_preserve_maintenance_history_through_commit_and_cancel() {
    for scope in [
        Scope::Foh,
        Scope::Monitor1,
        Scope::Monitor2,
        Scope::Monitor(3),
        Scope::Monitor(5),
        Scope::PaConfiguration,
        Scope::OutputRoutes,
        Scope::LocalOperatorMonitor,
        Scope::TalkbackDestinations,
        Scope::TalkbackFoh,
    ] {
        for class in ["ordinary", "processing", "sends", "external"] {
            for cancel in [false, true] {
                let mut e = engine();
                let lease = grant(&mut e, "maintainer", scope);
                let other_scope = match class {
                    "sends" => Scope::Monitor(3),
                    "external" if scope == Scope::PaConfiguration => Scope::OutputRoutes,
                    "external" => Scope::PaConfiguration,
                    "ordinary" if scope == Scope::Foh => Scope::Monitor1,
                    _ => Scope::Foh,
                };
                // Processing requires FOH. When FOH itself is being maintained,
                // cancellation revokes that same lease and must invalidate its cache.
                let editor = if other_scope == scope {
                    "maintainer"
                } else {
                    "editor"
                };
                let other = if editor == "maintainer" {
                    lease
                } else {
                    grant(&mut e, editor, other_scope)
                };
                let mut m = maintenance(lease);
                m.scope = scope;
                let first = e.maintain_lease(&m, 100).unwrap();
                let stale_renew = request(&e, "maintainer", Some(lease), 3, Command::Renew {});
                let r = request(&e, editor, Some(other), 2, Command::Renew {});
                match class {
                    "ordinary" => {
                        let mut r = r.clone();
                        r.command = Command::Set {
                            targets: vec![Edit {
                                target: if other_scope == Scope::Monitor1 {
                                    Target::Send {
                                        input: "input-01".into(),
                                        monitor: "monitor-1".into(),
                                    }
                                } else {
                                    Target::Fader {
                                        input: "input-01".into(),
                                    }
                                },
                                value: Value::Integer(-12000),
                            }],
                        };
                        assert_eq!(e.handle(&r, 101).unwrap().state, "pending");
                    }
                    "processing" => {
                        let p = ProcessingRequest {
                            contract: "GP07-processing".into(),
                            version: 4,
                            show_id: r.show_id.clone(),
                            module: r.module.clone(),
                            epoch: r.epoch,
                            writer: r.writer.clone(),
                            lease: r.lease,
                            request_id: r.request_id,
                            expected_revision: r.expected_revision,
                            command: ProcessingCommand::ProcessingSet {
                                input: "input-01".into(),
                                config: Default::default(),
                            },
                        };
                        assert_eq!(e.handle_processing(&p, 101).unwrap().state, "pending");
                    }
                    "sends" => {
                        let p = gigpies::sends_wire::SendsRequest {
                            contract: "GP18-sends".into(),
                            version: 1,
                            show_id: r.show_id.clone(),
                            module: r.module.clone(),
                            epoch: r.epoch,
                            writer: r.writer.clone(),
                            lease: r.lease,
                            request_id: r.request_id,
                            expected_revision: r.expected_revision,
                            command: gigpies::sends_wire::SendsCommand::SendTapSet {
                                input: "input-16".into(),
                                monitor: "monitor-3".into(),
                                tap: gigpies::sends_wire::Tap::ProcessedPreFader,
                            },
                        };
                        assert_eq!(e.handle_sends(&p, 101).unwrap().state, "pending");
                    }
                    "external" => {
                        assert!(
                            e.begin_external(&r, "atomic-test", other_scope, 101)
                                .unwrap()
                                .1
                                .is_none()
                        );
                    }
                    _ => unreachable!(),
                }
                assert_eq!(
                    e.maintain_lease(&m, 102).unwrap(),
                    first,
                    "replay precedes busy: {class}"
                );
                m.maintenance_id = Counter(2);
                assert_eq!(
                    e.maintain_lease(&m, 103),
                    Err(Reason::Unavailable),
                    "{class}"
                );
                if cancel {
                    e.revoke_writer(editor);
                    if editor == "maintainer" {
                        assert_eq!(e.maintain_lease(&m, 105), Err(Reason::Lease));
                        m.maintenance_id = Counter(1);
                        assert_eq!(e.maintain_lease(&m, 106), Err(Reason::Lease));
                        continue;
                    }
                } else if class == "external" {
                    render(&mut e, 48, 104);
                    assert_eq!(e.commit_external(104, || Ok(())).unwrap().kind, "applied");
                } else {
                    render(&mut e, 96, 104);
                }
                assert_eq!(
                    e.maintain_lease(&m, 105),
                    Err(Reason::Unavailable),
                    "cached busy survives {class} cancel={cancel}"
                );
                m.maintenance_id = Counter(1);
                assert_eq!(
                    e.maintain_lease(&m, 106).unwrap(),
                    first,
                    "success survives {class} cancel={cancel}"
                );
                // Neither duplicate nor busy retries extended the original expiry.
                let mut witness = stale_renew.clone();
                witness.command = Command::Renew {};
                assert_eq!(e.live_lease_witness(&witness, 2099).unwrap().1, 1);
                if !cancel {
                    let legacy = e.handle(&stale_renew, 2099).unwrap();
                    assert_eq!(
                        legacy.outcome.unwrap().body.reason.as_deref(),
                        Some("stale_revision")
                    );
                }
                m.maintenance_id = Counter(3);
                assert!(
                    e.maintain_lease(&m, 2099).is_ok(),
                    "atomic operation ignores unrelated revision churn"
                );
                e.revoke_writer("maintainer");
                assert_eq!(e.maintain_lease(&m, 2099), Err(Reason::Lease));
            }
        }
    }
}
#[test]
fn strict_atomic_codecs_and_exact_byte_boundaries() {
    let r = maintenance(Counter(1));
    let bytes = serde_json::to_vec(&r).unwrap();
    assert_eq!(maintenance::Request::decode(&bytes).unwrap(), r);
    let mut at_limit = bytes.clone();
    at_limit.resize(maintenance::MAX_BYTES, b' ');
    assert!(maintenance::Request::decode(&at_limit).is_ok());
    at_limit.push(b' ');
    assert!(maintenance::Request::decode(&at_limit).is_err());
    let mut value = serde_json::to_value(&r).unwrap();
    for (key, bad) in [
        ("version", serde_json::json!(2)),
        ("maintenance_id", serde_json::json!("01")),
        ("epoch", serde_json::json!("0")),
        ("lease", serde_json::json!(1)),
        ("unexpected", serde_json::json!(true)),
    ] {
        let mut v = value.clone();
        v[key] = bad;
        assert!(
            maintenance::Request::decode(&serde_json::to_vec(&v).unwrap()).is_err(),
            "{key}"
        );
    }
    value.as_object_mut().unwrap().remove("scope");
    assert!(maintenance::Request::decode(&serde_json::to_vec(&value).unwrap()).is_err());
    let reply = maintenance::Reply::new(r, Err(Reason::Unavailable));
    let mut v = serde_json::to_value(reply).unwrap();
    v.as_object_mut().unwrap().remove("result");
    assert!(maintenance::Reply::decode(&serde_json::to_vec(&v).unwrap()).is_err());
}
#[test]
fn atomic_scope_codecs_preserve_canonical_wire_and_existing_brain_bytes() {
    for scope in [
        Scope::Foh,
        Scope::Monitor1,
        Scope::Monitor2,
        Scope::Monitor(3),
        Scope::Monitor(u16::MAX),
        Scope::PaConfiguration,
        Scope::OutputRoutes,
        Scope::LocalOperatorMonitor,
        Scope::TalkbackDestinations,
        Scope::TalkbackFoh,
    ] {
        let mut r = maintenance(Counter(1));
        r.scope = scope;
        assert!(r.supported_scope());
        let bytes = serde_json::to_vec(&r).unwrap();
        assert_eq!(maintenance::Request::decode(&bytes).unwrap(), r);
        let success = maintenance::Reply::new(
            r,
            Ok(maintenance::Maintained {
                revision: Counter(0),
                source_frame: Counter(48),
                lease_remaining_ms: 2000,
            }),
        );
        assert_eq!(
            maintenance::Reply::decode(&success.encode().unwrap()).unwrap(),
            success
        );
        let mut value = serde_json::to_value(&success).unwrap();
        if let Some(name) = value["context"]["scope"].as_str().map(str::to_owned) {
            value["context"]["scope"] = serde_json::json!({name: null});
            assert!(
                maintenance::Request::decode(&serde_json::to_vec(&value["context"]).unwrap())
                    .is_err()
            );
            assert!(maintenance::Reply::decode(&serde_json::to_vec(&value).unwrap()).is_err());
        }
    }
    for n in 0..=2 {
        let mut r = maintenance(Counter(1));
        r.scope = Scope::Monitor(n);
        assert!(!r.supported_scope());
        assert!(
            maintenance::Reply::new(
                r,
                Ok(maintenance::Maintained {
                    revision: Counter(0),
                    source_frame: Counter(48),
                    lease_remaining_ms: 2000,
                })
            )
            .encode()
            .is_err()
        );
    }
    for inputs in [16, 32, 48] {
        let bytes = std::fs::read(format!(
            "tests/fixtures/atomic-control-v1/maintain-request-{inputs}.json"
        ))
        .unwrap();
        let r = maintenance::Request::decode(&bytes).unwrap();
        assert_eq!(
            serde_json::to_value(r).unwrap(),
            serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()
        );
        let bytes = std::fs::read(format!(
            "tests/fixtures/atomic-control-v1/maintained-{inputs}.json"
        ))
        .unwrap();
        maintenance::Reply::decode(&bytes).unwrap();
    }
}

#[test]
fn atomic_paired_fixture_schema_identity_dimensions_and_whole_envelope_boundaries() {
    use gigpies::paired_readback as p;
    for inputs in [16, 32, 48] {
        let bytes = std::fs::read(format!(
            "tests/fixtures/atomic-control-v1/read-snapshot-{inputs}.json"
        ))
        .unwrap();
        let decoded = p::Reply::decode(&bytes).unwrap();
        assert_eq!(decoded.raw.as_ref().unwrap().authority.inputs.len(), inputs);
        let mut at_limit = decoded.encode().unwrap();
        at_limit.resize(p::MAX_BYTES, b' ');
        assert!(p::Reply::decode(&at_limit).is_ok());
        at_limit.push(b' ');
        assert!(p::Reply::decode(&at_limit).is_err());
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        for path in ["frame", "revision"] {
            let mut v = value.clone();
            v["brain"][path] = serde_json::json!("48");
            assert!(p::Reply::decode(&serde_json::to_vec(&v).unwrap()).is_err());
        }
        for half in ["raw", "brain"] {
            let mut v = value.clone();
            v[half] = serde_json::Value::Null;
            assert!(p::Reply::decode(&serde_json::to_vec(&v).unwrap()).is_err());
        }
        let mut v = value.clone();
        v["brain"]
            .as_object_mut()
            .unwrap()
            .remove("held_generation");
        assert!(p::Reply::decode(&serde_json::to_vec(&v).unwrap()).is_err());
        let mut v = value.clone();
        v["raw"]["coefficients"][0]["extra"] = serde_json::json!(0);
        assert!(p::Reply::decode(&serde_json::to_vec(&v).unwrap()).is_err());
        let mut v = value.clone();
        v["brain"]["source"] = serde_json::json!({"kind":"pfl","input":inputs});
        assert!(p::Reply::decode(&serde_json::to_vec(&v).unwrap()).is_err());
        let mut v = value.clone();
        v["raw"]["authority"]["page_count"] = serde_json::json!(2);
        assert!(p::Reply::decode(&serde_json::to_vec(&v).unwrap()).is_err());
        let mut v = value.clone();
        v["raw"].as_object_mut().unwrap().remove("meters");
        assert!(p::Reply::decode(&serde_json::to_vec(&v).unwrap()).is_err());
        let mut v = value.clone();
        v["raw"]["resources"]["render_budget"]["unexpected"] = serde_json::json!(0);
        assert!(p::Reply::decode(&serde_json::to_vec(&v).unwrap()).is_err());
        let mut v = value.clone();
        v["version"] = serde_json::json!(2);
        assert!(p::Reply::decode(&serde_json::to_vec(&v).unwrap()).is_err());
    }
}
