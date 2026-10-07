use gigpies::{
    control_model::{Edit, Target, Value},
    mixer::{Mixer, Prepared},
    mixer_control::OfflineEngine,
    show::Counter,
    topology::{EngineTopology, OutputSource, ResourceBudget},
};
const SHOW: &str = "11111111-1111-4111-8111-111111111111";
#[test]
fn admission_rejects_overflow_and_duplicate_physical_writers() {
    assert!(EngineTopology::software(usize::MAX, 4, 6).is_err());
    let mut t = EngineTopology::software(17, 3, 0).unwrap();
    t.outputs[1].playback_slot = t.outputs[0].playback_slot;
    assert!(t.validate(ResourceBudget::default()).is_err());
    t.outputs[1].playback_slot = 1;
    t.measurement_slots = vec![0];
    assert!(t.validate(ResourceBudget::default()).is_err());
}
#[test]
fn reference_logical_bus_inventory_is_independent_of_socket_count() {
    let topology = EngineTopology::reference_16_18(8, 12).unwrap();
    assert_eq!(topology.pa_outputs + topology.monitors, 20);
    assert_eq!(topology.outputs.len(), 18);
    assert!(topology.outputs.iter().all(|p| p.source.is_none()));
    topology.validate(ResourceBudget::default()).unwrap();
}
#[test]
fn every_admitted_strip_and_monitor_render_and_four_band_processing() {
    for n in [16, 17, 32, 48] {
        let topology = EngineTopology::software(n, 5, 0).unwrap();
        for input in 0..n {
            let mut mixer = Mixer::from_topology(0, topology.clone()).unwrap();
            let edit = Edit {
                target: Target::Send {
                    input: format!("input-{:02}", input + 1),
                    monitor: "monitor-5".into(),
                },
                value: Value::Integer(0),
            };
            mixer
                .schedule(Prepared::edits_for(n, 5, &[edit]).unwrap(), 1)
                .unwrap();
            let mut raw = vec![0.; 336 * n];
            for row in raw.chunks_exact_mut(n) {
                row[input] = 0.125;
            }
            let mut output = vec![0.; 336 * 7];
            mixer.process_interleaved(&raw, &mut output).unwrap();
            assert_eq!(output[335 * 7 + 6], 0.125);
            assert_eq!(output[335 * 7 + 2], 0.125);
            mixer.take_completion();
            let config = gigpies::channel_processing::Config {
                eq_bypass: false,
                band4_gain_mdb: 6000,
                ..Default::default()
            };
            mixer
                .schedule(
                    Prepared::processing_for(
                        n,
                        5,
                        input,
                        gigpies::channel_processing::Prepared::new(config).unwrap(),
                    )
                    .unwrap(),
                    2,
                )
                .unwrap();
            mixer.process_interleaved(&raw, &mut output).unwrap();
            assert_eq!(mixer.processing_observations()[input].1, config);
            assert_eq!(output[335 * 7 + 6], 0.125);
        }
    }
}
#[test]
fn noncontiguous_reference_and_arbitrary_patch_have_no_implicit_destinations() {
    let mut t = EngineTopology::reference_16_18(8, 7).unwrap();
    assert_eq!(t.outputs[0].physical_port, "umc1820-main-out-left");
    assert_eq!(t.outputs[1].physical_port, "umc1820-main-out-right");
    assert_eq!(t.outputs[2].physical_port, "umc1820-line-out-3");
    assert_eq!(t.outputs[9].physical_port, "umc1820-line-out-10");
    assert_eq!(
        t.inputs.iter().map(|p| p.capture_slot).collect::<Vec<_>>(),
        (0..8).chain(10..18).collect::<Vec<_>>()
    );
    assert!(t.outputs.iter().all(|p| p.source.is_none()));
    t.outputs[17].source = Some(OutputSource::Monitor { index: 6 });
    t.outputs[2].source = Some(OutputSource::Pa { index: 7 });
    t.outputs.swap(0, 15);
    t.inputs[0].capture_slot = 17;
    t.inputs[15].capture_slot = 0;
    t.validate(ResourceBudget::default()).unwrap();
    let mut output = [99.; 20];
    let mut buses = [0.; 9];
    buses[8] = 0.3;
    let mut pa = [0.; 8];
    pa[7] = 0.4;
    t.patch_outputs(&buses, &pa, &mut output).unwrap();
    assert_eq!(output[19], 0.3);
    assert_eq!(output[2], 0.4);
    assert_eq!(output.iter().filter(|v| **v != 0.).count(), 2);
}
#[test]
fn expanded_authority_refuses_legacy_and_reports_all_strips() {
    let mut e = OfflineEngine::with_topology(
        SHOW,
        Counter(7),
        Counter(0),
        0,
        EngineTopology::software(48, 5, 0).unwrap(),
    )
    .unwrap();
    let r = gigpies::control_model::Request {
        contract: "C-AUDIO".into(),
        version: 1,
        show_id: SHOW.into(),
        module: "audio".into(),
        epoch: Counter(7),
        writer: None,
        lease: None,
        request_id: None,
        expected_revision: None,
        command: gigpies::control_model::Command::Snapshot {},
    };
    assert_eq!(
        e.handle(&r, 0)
            .unwrap()
            .outcome
            .unwrap()
            .body
            .reason
            .as_deref(),
        Some("version")
    );
    let s = e.snapshot().unwrap();
    s.validate().unwrap();
    assert_eq!(s.coefficients.len(), 48);
    assert_eq!(s.coefficients[47].current_nanogain.len(), 9);
    assert_eq!(e.processing_snapshot().unwrap().channels.len(), 48);
    let mut output = vec![1.; 48 * 7];
    e.process_interleaved(&vec![0.1; 48 * 48], &mut output, 0)
        .unwrap();
    assert!(output.iter().all(|v| *v == 0.));
}
#[test]
fn safe_restore_and_clock_fault_never_restore_a_grant_or_unmute() {
    let mut e = OfflineEngine::with_topology(
        SHOW,
        Counter(7),
        Counter(0),
        0,
        EngineTopology::software(17, 3, 0).unwrap(),
    )
    .unwrap();
    let mut intent = e.persisted_intent().unwrap();
    intent.processing[16].eq_bypass = false;
    intent.processing[16].band3_gain_mdb = 6000;
    let mut restored = OfflineEngine::restore_intent(&intent, Counter(8), 0).unwrap();
    assert_eq!(
        restored.processing_snapshot().unwrap().channels[16].target,
        intent.processing[16]
    );
    assert_eq!(
        restored.clock_status().state,
        gigpies::clock_domain::ClockState::Disarmed
    );
    restored.rearm().unwrap();
    restored.admit_source(8, 0, 48).unwrap();
    let mut out = vec![0.; 48 * 5];
    restored
        .process_interleaved(&vec![0.1; 48 * 17], &mut out, 0)
        .unwrap();
    assert!(restored.admit_source(8, 96, 48).is_err());
    assert_eq!(
        restored.clock_status().state,
        gigpies::clock_domain::ClockState::Quiesced
    );
    assert!(restored.rearm().is_err());
    restored.recover(Counter(9), 0).unwrap();
    assert_eq!(
        restored.processing_snapshot().unwrap().channels[16].target,
        intent.processing[16]
    );
    assert_eq!(
        restored.clock_status().state,
        gigpies::clock_domain::ClockState::Disarmed
    );
}
#[test]
fn coherent_snapshot_pages_reject_mixed_and_stale_assemblies() {
    let data = vec![b'a'; 200_000];
    let encoded = gigpies::snapshot_pages::encode(data.clone()).unwrap();
    let mut assembly = gigpies::snapshot_pages::Assembly::default();
    let mut result = None;
    for page in &encoded {
        assert!(page.len() < 65536);
        result = assembly
            .offer(serde_json::from_slice(page).unwrap(), 100)
            .unwrap();
    }
    assert_eq!(result, Some(data));
    let mut assembly = gigpies::snapshot_pages::Assembly::default();
    assembly
        .offer(serde_json::from_slice(&encoded[0]).unwrap(), 0)
        .unwrap();
    assert!(
        assembly
            .offer(serde_json::from_slice(&encoded[1]).unwrap(), 2001)
            .is_err()
    );
}
#[test]
#[ignore = "producer fixture generation; explicit task-private output required"]
fn write_producer_fixtures() {
    let root = std::env::var("GP14_FIXTURES").unwrap();
    std::fs::create_dir_all(&root).unwrap();
    for pa_outputs in [6, 8] {
        let topology = EngineTopology::reference_16_18(pa_outputs, 18 - pa_outputs).unwrap();
        std::fs::write(
            format!("{root}/reference-unpatched-{pa_outputs}.json"),
            serde_json::to_vec_pretty(&topology).unwrap(),
        )
        .unwrap();
    }
    for n in [16, 17, 32, 48] {
        let mut e = OfflineEngine::with_topology(
            SHOW,
            Counter(14),
            Counter(0),
            0,
            EngineTopology::software(n, 5, 0).unwrap(),
        )
        .unwrap();
        let mut r = gigpies::control_model::Request {
            contract: "C-AUDIO".into(),
            version: 2,
            show_id: SHOW.into(),
            module: "audio".into(),
            epoch: Counter(14),
            writer: None,
            lease: None,
            request_id: None,
            expected_revision: None,
            command: gigpies::control_model::Command::Snapshot {},
        };
        let snapshot = e.handle(&r, 0).unwrap();
        snapshot.validate().unwrap();
        r.writer = Some("fixture-writer".into());
        r.request_id = Some(Counter(1));
        r.expected_revision = Some(Counter(0));
        r.command = gigpies::control_model::Command::Grant {
            scope: gigpies::control_model::Scope::Foh,
        };
        let grant = e.handle(&r, 0).unwrap();
        r.lease = grant.outcome.unwrap().body.granted_lease;
        let pr = gigpies::processing_wire::ProcessingRequest {
            contract: "GP07-processing".into(),
            version: 4,
            show_id: SHOW.into(),
            module: "audio".into(),
            epoch: Counter(14),
            writer: r.writer.clone(),
            lease: r.lease,
            request_id: Some(Counter(2)),
            expected_revision: Some(Counter(0)),
            command: gigpies::processing_wire::ProcessingCommand::ProcessingSet {
                input: format!("input-{n:02}"),
                config: gigpies::channel_processing::Config {
                    eq_bypass: false,
                    band4_gain_mdb: 3000,
                    ..Default::default()
                },
            },
        };
        let pending = e.handle_processing(&pr, 0).unwrap();
        assert_eq!(pending.state, "pending");
        e.rearm().unwrap();
        let mut out = vec![0.; 96 * 7];
        e.process_interleaved(&vec![0.1; 96 * n], &mut out, 1)
            .unwrap();
        let final_reply = e.take_processing_completions().pop().unwrap();
        final_reply.validate().unwrap();
        let fixture = serde_json::json!({"inputs":n,"snapshot":snapshot,"processing_request":pr,"pending":pending,"final":final_reply,"rendered_after":e.snapshot().unwrap(),"sample_output":&out[48*7..49*7]});
        std::fs::write(
            format!("{root}/profile-{n}.json"),
            serde_json::to_vec_pretty(&fixture).unwrap(),
        )
        .unwrap();
    }
}
#[test]
fn external_transaction_shares_revision_scope_retry_and_boundary() {
    use gigpies::control_model::{Command, Request, Scope};
    let mut e = OfflineEngine::with_topology(
        SHOW,
        Counter(7),
        Counter(0),
        0,
        EngineTopology::software(16, 3, 0).unwrap(),
    )
    .unwrap();
    let mut r = Request {
        contract: "C-AUDIO".into(),
        version: 2,
        show_id: SHOW.into(),
        module: "audio".into(),
        epoch: Counter(7),
        writer: Some("patcher".into()),
        lease: None,
        request_id: Some(Counter(1)),
        expected_revision: Some(Counter(0)),
        command: Command::Grant {
            scope: Scope::OutputRoutes,
        },
    };
    r.lease = e.handle(&r, 0).unwrap().outcome.unwrap().body.granted_lease;
    r.request_id = Some(Counter(2));
    r.command = Command::Renew {};
    let (frame, cached) = e
        .begin_external(&r, "route-hash", Scope::OutputRoutes, 0)
        .unwrap();
    assert_eq!(frame, 48);
    assert!(cached.is_none());
    let mut output = vec![0.; 49 * 5];
    assert!(
        e.process_interleaved(&vec![0.; 49 * 16], &mut output, 0)
            .is_err()
    );
    e.process_interleaved(&vec![0.; 48 * 16], &mut output[..48 * 5], 0)
        .unwrap();
    let reply = e.commit_external(1, || Ok(())).unwrap();
    assert_eq!(reply.body.revision, Counter(1));
    assert!(
        e.begin_external(&r, "route-hash", Scope::OutputRoutes, 2)
            .unwrap()
            .1
            .is_some()
    );
    assert_eq!(
        e.begin_external(&r, "changed-hash", Scope::OutputRoutes, 2)
            .unwrap()
            .1
            .unwrap()
            .body
            .reason
            .as_deref(),
        Some("reused_id")
    );
}
#[test]
fn raw_analysis_groups_preserve_all_channels_and_drop_whole_blocks() {
    use gigpies::analysis_stream::{RawDescriptor, RawTap};
    for n in [16, 17, 32, 48] {
        let descriptor =
            RawDescriptor::new(14, 0, 7, (1..=n).map(|i| format!("input-{i:02}")).collect())
                .unwrap();
        let (mut tap, mut consumer) = RawTap::new(&descriptor, n).unwrap();
        let raw: Vec<i32> = (0..48)
            .flat_map(|f| (0..n).map(move |i| (f * n + i) as i32 - 1000))
            .collect();
        tap.offer(14, 0, &raw, n).unwrap();
        let groups = n.div_ceil(4);
        let mut got = vec![0; 48 * n];
        for _ in 0..groups {
            let packet = consumer.pop().unwrap();
            assert_eq!(packet.map_revision, 7);
            let bytes = &packet.bytes[..usize::from(packet.length)];
            let p = gigpies::transport::Packet::parse(bytes).unwrap();
            let first = usize::from(p.spec().first_channel);
            let count = usize::from(p.spec().channels);
            for f in 0..48 {
                for c in 0..count {
                    let offset = 48 + (f * count + c) * 3;
                    let v = i32::from_be_bytes([
                        if bytes[offset] & 0x80 != 0 { 255 } else { 0 },
                        bytes[offset],
                        bytes[offset + 1],
                        bytes[offset + 2],
                    ]);
                    got[f * n + first + c] = v;
                }
            }
        }
        assert_eq!(raw, got);
        tap.offer(14, 48, &raw, n).unwrap();
        tap.offer(14, 96, &raw, n).unwrap();
        tap.offer(14, 144, &raw, n).unwrap();
        assert_eq!(tap.dropped_blocks, 1);
        assert_eq!(consumer.slots(), groups * 2);
    }
}

#[test]
fn configurable_partition_equivalence_and_output_rearm_envelope() {
    for n in [16, 32, 48] {
        let topology = EngineTopology::software(n, 3, 0).unwrap();
        let mut one = Mixer::from_topology(0, topology.clone()).unwrap();
        let mut split = Mixer::from_topology(0, topology).unwrap();
        let config = gigpies::channel_processing::Config {
            eq_bypass: false,
            band2_gain_mdb: 6000,
            compressor_bypass: false,
            ratio_milli: 4000,
            ..Default::default()
        };
        for mixer in [&mut one, &mut split] {
            mixer
                .schedule(
                    Prepared::processing_for(
                        n,
                        3,
                        n - 1,
                        gigpies::channel_processing::Prepared::new(config).unwrap(),
                    )
                    .unwrap(),
                    1,
                )
                .unwrap();
        }
        let input: Vec<_> = (0..1000 * n)
            .map(|i| ((i * 17 % 503) as f64 - 251.) / 512.)
            .collect();
        let mut a = vec![0.; 5000];
        let mut b = vec![0.; 5000];
        one.process_interleaved(&input, &mut a).unwrap();
        let mut start = 0;
        for count in [1, 47, 1, 37, 154, 321, 439] {
            split
                .process_interleaved(
                    &input[start * n..(start + count) * n],
                    &mut b[start * 5..(start + count) * 5],
                )
                .unwrap();
            start += count;
        }
        assert_eq!(a, b);
        one.take_completion();
        one.mute_outputs().unwrap();
        one.process_interleaved(&input[..240 * n], &mut a[..1200])
            .unwrap();
        assert!(one.outputs_quiesced());
        one.rearm().unwrap();
        one.process_interleaved(&input[..n], &mut a[..5]).unwrap();
        assert!(a[..5].iter().all(|v| *v == 0.));
    }
}

#[test]
fn versioned_authority_decoder_never_labels_expanded_inventory_legacy() {
    use gigpies::control_model::{Authority, Command, Reply, Request};
    let mut a = Authority::with_dimensions(SHOW, Counter(14), Counter(0), 16, 4).unwrap();
    let r = Request {
        contract: "C-AUDIO".into(),
        version: 2,
        show_id: SHOW.into(),
        module: "audio".into(),
        epoch: Counter(14),
        writer: None,
        lease: None,
        request_id: None,
        expected_revision: None,
        command: Command::Snapshot {},
    };
    let mut reply = a.handle(&r, 0);
    let bytes = serde_json::to_vec(&reply).unwrap();
    assert!(Reply::decode_versioned(&bytes).is_ok());
    assert!(Reply::decode(&bytes).is_err());
    reply.version = 1;
    assert!(reply.validate().is_err());
}

#[test]
fn output_ramp_clock_exhaustion_refuses_without_state_mutation() {
    let mut mixer = Mixer::new(u64::MAX - 100);
    mixer.quiesce();
    assert!(mixer.rearm().is_err());
    assert!(mixer.outputs_quiesced());
    assert!(mixer.mute_outputs().is_err());
    assert_eq!(mixer.frame(), u64::MAX - 100);
}

#[test]
fn prepared_patch_is_bound_to_complete_base_not_only_user_identity() {
    let a = EngineTopology::software(16, 3, 0).unwrap();
    let mut b = EngineTopology::software(17, 3, 0).unwrap();
    b.identity = a.identity.clone();
    let source = Mixer::from_topology(0, a.clone()).unwrap();
    let mut target = Mixer::from_topology(0, b.clone()).unwrap();
    target.quiesce();
    let mut prepared = source.prepare_output_patch(a.outputs.clone()).unwrap();
    assert!(target.apply_output_patch(&mut prepared).is_err());
    assert_eq!(target.topology(), &b);
    let mut b = a.clone();
    b.inputs[0].capture_slot = 1;
    b.inputs[1].capture_slot = 0;
    let mut target = Mixer::from_topology(0, b.clone()).unwrap();
    target.quiesce();
    assert!(target.apply_output_patch(&mut prepared).is_err());
    assert_eq!(target.topology(), &b);
}

#[test]
fn high_strip_four_band_and_dynamics_samples_match_slot_zero_reference() {
    for n in [16, 17, 32, 48] {
        let mut high = Mixer::from_topology(0, EngineTopology::software(n, 5, 0).unwrap()).unwrap();
        let mut reference = Mixer::new(0);
        let config = gigpies::channel_processing::Config {
            eq_bypass: false,
            band1_gain_mdb: 3000,
            band2_gain_mdb: -4000,
            band3_gain_mdb: 6000,
            band4_gain_mdb: -3000,
            compressor_bypass: false,
            threshold_mdb: -30000,
            ratio_milli: 4000,
            attack_us: 100,
            ..Default::default()
        };
        high.schedule(
            Prepared::processing_for(
                n,
                5,
                n - 1,
                gigpies::channel_processing::Prepared::new(config).unwrap(),
            )
            .unwrap(),
            1,
        )
        .unwrap();
        reference
            .schedule(
                Prepared::processing(
                    0,
                    gigpies::channel_processing::Prepared::new(config).unwrap(),
                )
                .unwrap(),
                1,
            )
            .unwrap();
        let signal: Vec<_> = (0..1024)
            .map(|f| 0.6 * (std::f64::consts::TAU * 3100. * f as f64 / 48000.).sin())
            .collect();
        let mut raw = vec![0.; n * 1024];
        let mut legacy = vec![[0.; 8]; 1024];
        for (f, &v) in signal.iter().enumerate() {
            raw[f * n + n - 1] = v;
            legacy[f][0] = v;
        }
        let mut actual = vec![0.; 7 * 1024];
        let mut expected = vec![[0.; 4]; 1024];
        high.process_interleaved(&raw, &mut actual).unwrap();
        reference.process(&legacy, &mut expected).unwrap();
        for f in 0..1024 {
            assert_eq!(&actual[f * 7..f * 7 + 2], &expected[f][..2]);
            assert_eq!(actual[f * 7 + 2], signal[f]);
        }
        let neutral_gain = 10_f64.powf(-6. / 20.) * std::f64::consts::FRAC_1_SQRT_2;
        assert!((500..1024).any(|f| (actual[f * 7] - signal[f] * neutral_gain).abs() > 0.01));
        let (_, target, remaining, gr) = high.processing_observations()[n - 1];
        assert_eq!(target, config);
        assert_eq!(remaining, 0);
        assert!(gr.is_some_and(|v| v > 0));
    }
}

#[test]
fn explicit_six_and_eight_pa_examples_patch_all_analog_sockets_flexibly() {
    for pa_count in [6, 8] {
        let monitors = 18 - pa_count;
        let mut topology = EngineTopology::reference_16_18(pa_count, monitors).unwrap();
        assert!(topology.outputs.iter().all(|p| p.source.is_none()));
        let analog_slots: Vec<_> = (0..10).chain(12..20).collect();
        let buses: Vec<_> = (0..monitors + 2).map(|i| 100. + i as f64).collect();
        let pa: Vec<_> = (0..pa_count).map(|i| 200. + i as f64).collect();
        let mut expected = [0.; 20];
        for source in 0..18 {
            // A full permutation deliberately interleaves PA and monitor sockets.
            let socket = (source * 5 + 7) % 18;
            let port = &mut topology.outputs[socket];
            // Transport ordering is independent of both logical socket and source.
            port.playback_slot = analog_slots[(socket + 11) % 18];
            port.source = Some(if source < pa_count {
                OutputSource::Pa { index: source }
            } else {
                OutputSource::Monitor {
                    index: source - pa_count,
                }
            });
            expected[port.playback_slot] = if source < pa_count {
                pa[source]
            } else {
                buses[2 + source - pa_count]
            };
        }
        topology.validate(ResourceBudget::default()).unwrap();
        let mut playback = [999.; 20];
        topology.patch_outputs(&buses, &pa, &mut playback).unwrap();
        assert_eq!(playback, expected);
        assert_eq!(&playback[10..12], &[0., 0.]);
        assert_eq!(playback.iter().filter(|&&sample| sample != 0.).count(), 18);
        assert_eq!(
            topology.mapping_evidence,
            "manufacturer-reference-unverified"
        );
    }
}

#[test]
fn processing_version_refusal_precedes_body_interpretation_for_each_provider() {
    let mut legacy: serde_json::Value =
        serde_json::from_slice(include_bytes!("fixtures/gp07/v1/set-request.json")).unwrap();
    for (provider, request) in [(2, 3), (3, 2)] {
        legacy["version"] = request.into();
        let bytes = serde_json::to_vec(&legacy).unwrap();
        let refusal = gigpies::processing_wire::UnsupportedRequest::decode_for(&bytes, provider)
            .unwrap()
            .unwrap()
            .refusal(Counter(7));
        assert_eq!(refusal["reason"], "unsupported_version");
        assert_eq!(refusal["version"], request);
        assert_eq!(refusal["revision"], "7");
        assert!(refusal["snapshot"].is_null());
    }
}
