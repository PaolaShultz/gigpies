use gigpies::{
    control_model::{Command, Request as Audio, Scope},
    local_audio::LocalAudio,
    show::Counter,
    structural_control::{Command as S, Request},
    topology::EngineTopology,
};
use std::{os::unix::fs::PermissionsExt, path::PathBuf};
const SHOW: &str = "11111111-1111-4111-8111-111111111111";
fn request(command: S) -> Request {
    Request {
        contract: "GP18-master-eq".into(),
        version: 1,
        show_id: SHOW.into(),
        module: "audio".into(),
        epoch: Counter(1),
        writer: None,
        lease: None,
        request_id: None,
        expected_revision: None,
        command,
    }
}
struct Fixture {
    host: LocalAudio,
    dir: PathBuf,
    now: u64,
}
impl Fixture {
    fn new(label: &str, outputs: usize) -> Self {
        let dir = std::env::temp_dir().join(format!("gp18-eq-{label}-{}", std::process::id()));
        std::fs::create_dir(&dir).unwrap();
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
        let host = LocalAudio::bind_configured(
            &dir,
            "audio",
            SHOW,
            Counter(1),
            EngineTopology::software(17, 3, outputs).unwrap(),
        )
        .unwrap();
        Self { host, dir, now: 0 }
    }
    fn block(&mut self) -> Vec<f64> {
        self.now += 1;
        let t = self.host.topology();
        let capture: Vec<_> = (0..48 * t.capture_channels)
            .map(|i| {
                let phase = (self.host.frame() + (i / t.capture_channels) as u64) as f64
                    * 173.
                    * std::f64::consts::TAU
                    / 48000.;
                (0.005 * phase.sin() * 8388608.).round() / 8388608.
            })
            .collect();
        let mut out = vec![0.; 48 * t.playback_channels];
        self.host
            .tick_with_capture(
                self.now,
                self.host.source_epoch(),
                self.host.frame(),
                &capture,
                &mut out,
            )
            .unwrap();
        out
    }
    fn grant(&mut self) -> Counter {
        let revision = self.host.engine_mut().revision();
        self.host
            .engine_mut()
            .handle(
                &Audio {
                    contract: "C-AUDIO".into(),
                    version: 2,
                    show_id: SHOW.into(),
                    module: "audio".into(),
                    epoch: Counter(1),
                    writer: Some("eq-writer".into()),
                    lease: None,
                    request_id: Some(Counter(1)),
                    expected_revision: Some(revision),
                    command: Command::Grant {
                        scope: Scope::PaConfiguration,
                    },
                },
                self.now,
            )
            .unwrap()
            .outcome
            .unwrap()
            .body
            .granted_lease
            .unwrap()
    }
    fn set(&mut self, command: S, lease: Counter, id: u64) -> Request {
        Request {
            writer: Some("eq-writer".into()),
            lease: Some(lease),
            request_id: Some(Counter(id)),
            expected_revision: Some(self.host.engine_mut().revision()),
            command,
            ..request(S::MasterEqSnapshot {})
        }
    }
    fn apply(&mut self, r: Request, fresh: bool) -> gigpies::structural_control::Reply {
        let reply = self
            .host
            .structural_request(r.clone(), self.now, fresh, None)
            .unwrap();
        assert_eq!(reply.state, "pending");
        self.block();
        self.block();
        self.host
            .structural_request(r, self.now, true, None)
            .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.dir).unwrap();
    }
}
#[test]
fn independent_contract_keeps_fractional_owner_json_opaque_and_gp14_frozen() {
    let mut r = request(S::MasterEqSet {
        patch_json: "{\"db\":0.125}".into(),
        program_buses: vec![0, 1],
        owner_instance: Counter(1),
        graph_generation: Counter(1),
        eq_generation: Counter(0),
        map_revision: Counter(1),
    });
    r.writer = Some("eq-writer".into());
    r.lease = Some(Counter(1));
    r.request_id = Some(Counter(2));
    r.expected_revision = Some(Counter(0));
    let bytes = r.encode().unwrap();
    assert_eq!(Request::decode(&bytes).unwrap(), r);
    let mut wrong = r.clone();
    wrong.contract = "GP14-structure".into();
    assert!(wrong.encode().is_err());
    let wrong = request(S::OutputMute {});
    assert!(wrong.encode().is_err());
    let text = String::from_utf8(bytes).unwrap();
    assert!(
        Request::decode(
            text.replace("\"version\":1", "\"version\":1,\"version\":1")
                .as_bytes()
        )
        .is_err()
    );
    assert!(
        Request::decode(
            text.replace("\"owner_instance\":\"1\"", "\"owner_instance\":0.5")
                .as_bytes()
        )
        .is_err()
    );
}
#[test]
fn unavailable_owner_freshness_shared_revision_dedup_and_cancel() {
    let mut f = Fixture::new("unavailable", 0);
    let read = request(S::MasterEqSnapshot {});
    let reply = f.host.structural_request(read, 0, false, None).unwrap();
    assert!(!reply.master_eq.unwrap().live_available);
    let lease = f.grant();
    let command = || S::MasterEqSet {
        patch_json: "{}".into(),
        program_buses: vec![0, 1],
        owner_instance: Counter(1),
        graph_generation: Counter(1),
        eq_generation: Counter(0),
        map_revision: Counter(1),
    };
    let r = f.set(command(), lease, 2);
    let before = f.host.engine_mut().revision();
    let reply = f.apply(r.clone(), false);
    assert_eq!(
        reply.reason.as_deref(),
        Some("fresh_structural_snapshot_required")
    );
    assert_eq!(f.host.engine_mut().revision(), before);
    assert_eq!(
        f.host
            .structural_request(r.clone(), f.now, true, None)
            .unwrap()
            .reason,
        reply.reason
    );
    let r = f.set(command(), lease, 3);
    let reply = f.apply(r, true);
    assert_eq!(reply.reason.as_deref(), Some("live EQ unavailable"));
    let r = f.set(command(), lease, 4);
    assert_eq!(
        f.host
            .structural_request(r, f.now, true, None)
            .unwrap()
            .state,
        "pending"
    );
    f.host.revoke_writer("eq-writer");
    f.block();
    f.block();
    assert!(
        f.host
            .take_remote_completions()
            .iter()
            .all(|r| r["effective_frame"].is_null())
    );
}

#[cfg(feature = "hardware-host")]
#[test]
#[ignore = "bounded actual old/new owner acceptance; requires GP_EQ_MANIFEST, GP_EQ_OLD_MANIFEST, GP_PA_V2_FIXTURES and GP_EQ_CORPUS"]
fn actual_owner_fallback_stereo_identity_transition_metadata_recovery_and_corpus() {
    use serde_json::{Value, json};
    let old = PathBuf::from(std::env::var_os("GP_EQ_OLD_MANIFEST").unwrap());
    let new = PathBuf::from(std::env::var_os("GP_EQ_MANIFEST").unwrap());
    let c = std::fs::read(
        PathBuf::from(std::env::var_os("GP_PA_V2_FIXTURES").unwrap()).join("stereo4way.json"),
    )
    .unwrap();
    let mut oldf = Fixture::new("old", 8);
    oldf.host.enable_modules(&old).unwrap();
    oldf.host.configure_pa(&c).unwrap();
    assert!(!oldf.host.master_eq_snapshot().unwrap().live_supported);
    assert!(oldf.host.structural_snapshot().unwrap().pa_status.is_some());
    let mut f = Fixture::new("actual", 8);
    f.host.enable_modules(&new).unwrap();
    f.host.configure_pa(&c).unwrap();
    let lease = f.grant();
    let mut corpus =
        vec![json!({"label":"unavailable","snapshot":oldf.host.master_eq_snapshot().unwrap()})];
    let mut full: Value = serde_json::from_slice(&c).unwrap();
    let third = full["inputs"][0].clone();
    full["inputs"].as_array_mut().unwrap().push(third);
    for o in full["outputs"].as_array_mut().unwrap() {
        if o["source"] == json!({"input":1}) {
            o["source"] = json!({"input":2});
        }
    }
    let mut fullr = f.set(
        S::PaSet {
            configuration_json: full.to_string(),
            program_buses: vec![1, 2, 0],
        },
        lease,
        2,
    );
    fullr.contract = "GP14-structure".into();
    assert!(f.apply(fullr, true).reason.is_none());
    let mut arm = f.set(S::OutputRearm {}, lease, 3);
    arm.contract = "GP14-structure".into();
    assert!(f.apply(arm, true).reason.is_none());
    let mut armed = vec![];
    for _ in 0..12 {
        armed = f.block();
    }
    assert!(armed.iter().any(|x| x.abs() > 1e-8));
    let baseline = f.host.master_eq_snapshot().unwrap();
    assert_eq!(baseline.master_input_indices, Some([2, 0]));
    corpus.push(json!({"label":"baseline","snapshot":baseline}));
    let settings = |index: usize| {
        let mut e = full["inputs"][index].clone();
        e.as_object_mut()
            .unwrap()
            .retain(|k, _| ["eq_enabled", "eq", "geq_enabled", "geq_db"].contains(&k.as_str()));
        e["input_index"] = json!(index);
        e
    };
    let mut right = settings(0);
    right["eq"][0]["db"] = json!(6.125);
    let left = settings(2);
    let patch = json!({"version":1,"inputs":[right,left]}).to_string();
    let command = |snap: &gigpies::master_eq_wire::Snapshot, patch_json: String| S::MasterEqSet {
        patch_json,
        program_buses: snap.program_buses.clone(),
        owner_instance: snap.owner_instance,
        graph_generation: snap.graph_generation,
        eq_generation: snap.eq_generation,
        map_revision: snap.map_revision,
    };
    let r = f.set(command(&baseline, patch.clone()), lease, 4);
    let pending = f
        .host
        .structural_request(r.clone(), f.now, true, None)
        .unwrap();
    corpus.push(json!({"label":"pending","request":r,"reply":pending}));
    f.block();
    let active_output = f.block();
    assert!(active_output.iter().any(|x| x.abs() > 1e-8));
    assert!(!f.host.structural_snapshot().unwrap().outputs_quiesced);
    assert_eq!(
        f.host.structural_snapshot().unwrap().pa_status.unwrap()["muted"],
        0
    );
    let transition = f.host.master_eq_snapshot().unwrap();
    assert_eq!(transition.transition_remaining_frames, Counter(192));
    assert!(!transition.settled);
    let bank: Value = serde_json::from_str(transition.owner_json.as_ref().unwrap()).unwrap();
    assert_eq!(bank["current"][1]["eq"][0]["db"], 0.);
    assert_eq!(bank["target"][1]["eq"][0]["db"], 6.125);
    let retained: Value = serde_json::from_str(
        f.host
            .structural_snapshot()
            .unwrap()
            .pa_configuration_json
            .as_ref()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(retained["inputs"][0]["eq"][0]["db"], 6.125);
    let mut expected = full.clone();
    expected["inputs"][0]["eq"][0]["db"] = json!(6.125);
    assert_eq!(retained, expected);
    corpus.push(json!({"label":"transition","snapshot":transition}));
    let final_reply = f
        .host
        .structural_request(r.clone(), f.now, true, None)
        .unwrap();
    assert!(final_reply.reason.is_none());
    corpus.push(json!({"label":"final","request":r,"reply":final_reply}));
    let intent = f.host.persisted_intent().unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(intent.pa_configuration_json.as_ref().unwrap()).unwrap(),
        expected
    );
    for _ in 0..4 {
        f.block();
    }
    let settled = f.host.master_eq_snapshot().unwrap();
    assert!(settled.settled);
    assert_eq!(settled.transition_remaining_frames, Counter(0));
    corpus.push(json!({"label":"settled","snapshot":settled}));
    let mut invalid: Value = serde_json::from_str(&patch).unwrap();
    invalid["route"] = json!(0);
    let refused = f.set(command(&settled, invalid.to_string()), lease, 5);
    let old_json = f.host.structural_snapshot().unwrap().pa_configuration_json;
    let refusal = f.apply(refused.clone(), true);
    assert!(refusal.reason.is_some());
    assert_eq!(
        f.host.structural_snapshot().unwrap().pa_configuration_json,
        old_json
    );
    corpus.push(json!({"label":"refusal","request":refused,"reply":refusal}));
    let mut next: Value = serde_json::from_str(&patch).unwrap();
    next["inputs"][0]["eq"][0]["db"] = json!(-4.125);
    let newer = f.host.master_eq_snapshot().unwrap();
    let next_request = f.set(command(&newer, next.to_string()), lease, 6);
    assert!(f.apply(next_request, true).reason.is_none());
    // Recovery during the second fade uses committed target and stays disarmed.
    let committed_before_failure = f.host.structural_snapshot().unwrap().pa_configuration_json;
    f.host
        .quiesce_source("explicit producer source failure")
        .unwrap();
    let failed = f.host.master_eq_snapshot().unwrap();
    assert!(!failed.live_available && failed.source_recovery_required && !failed.settled);
    assert_eq!(
        f.host.structural_snapshot().unwrap().pa_configuration_json,
        committed_before_failure
    );
    corpus.push(json!({"label":"source-failure","snapshot":failed}));
    f.host.recover_source(Counter(2), 0).unwrap();
    let recovered = f.host.master_eq_snapshot().unwrap();
    assert!(recovered.live_available && !recovered.source_recovery_required);
    let rb: Value = serde_json::from_str(recovered.owner_json.as_ref().unwrap()).unwrap();
    assert_eq!(rb["current"], rb["target"]);
    assert_eq!(rb["target"][1]["eq"][0]["db"], -4.125);
    assert!(f.host.structural_snapshot().unwrap().outputs_quiesced);
    corpus.push(json!({"label":"recovery","snapshot":recovered}));
    let path = PathBuf::from(std::env::var_os("GP_EQ_CORPUS").unwrap());
    std::fs::create_dir_all(&path).unwrap();
    std::fs::write(
        path.join("producer.json"),
        serde_json::to_vec_pretty(&corpus).unwrap(),
    )
    .unwrap();
}
