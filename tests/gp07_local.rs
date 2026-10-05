#![cfg(target_os = "linux")]
use gigpies::{
    channel_processing::Config,
    control_model::{Command, Request, Scope},
    local_audio::LocalAudio,
    processing_wire::{ProcessingCommand, ProcessingReply, ProcessingRequest},
    show::Counter,
};
use std::{
    io::{ErrorKind, Read, Write},
    os::unix::{fs::PermissionsExt, net::UnixStream},
};
const SHOW: &str = "11111111-1111-4111-8111-111111111111";
struct Client {
    stream: UnixStream,
    bytes: Vec<u8>,
}
impl Client {
    fn new(path: &std::path::Path) -> Self {
        let stream = UnixStream::connect(path).unwrap();
        stream.set_nonblocking(true).unwrap();
        Self {
            stream,
            bytes: Vec::new(),
        }
    }
    fn send(&mut self, b: &[u8]) {
        self.stream
            .write_all(&(b.len() as u32).to_be_bytes())
            .unwrap();
        self.stream.write_all(b).unwrap();
    }
    fn wait(
        &mut self,
        s: &mut LocalAudio,
        now: &mut u64,
        predicate: impl Fn(&serde_json::Value) -> bool,
    ) -> serde_json::Value {
        for _ in 0..100 {
            *now += 1;
            s.tick(*now).unwrap();
            let mut b = [0; 8192];
            loop {
                match self.stream.read(&mut b) {
                    Ok(0) => break,
                    Ok(n) => self.bytes.extend_from_slice(&b[..n]),
                    Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                    Err(e) => panic!("{e}"),
                }
            }
            while self.bytes.len() >= 4 {
                let n = u32::from_be_bytes(self.bytes[..4].try_into().unwrap()) as usize;
                if self.bytes.len() < n + 4 {
                    break;
                }
                let v = serde_json::from_slice(&self.bytes[4..n + 4]).unwrap();
                self.bytes.drain(..n + 4);
                if predicate(&v) {
                    return v;
                }
            }
        }
        panic!("reply timeout")
    }
    fn processing(
        &mut self,
        s: &mut LocalAudio,
        now: &mut u64,
        r: &ProcessingRequest,
        state: &str,
    ) -> ProcessingReply {
        self.send(&r.encode().unwrap());
        let v = self.wait(s, now, |v| {
            v["contract"] == "GP07-processing"
                && v["state"] == state
                && v["context"]["request_id"] == serde_json::to_value(r.request_id).unwrap()
        });
        ProcessingReply::decode(&serde_json::to_vec(&v).unwrap()).unwrap()
    }
}
fn read() -> ProcessingRequest {
    ProcessingRequest {
        contract: "GP07-processing".into(),
        version: 2,
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
fn set(id: u64, rev: u64) -> ProcessingRequest {
    ProcessingRequest {
        writer: Some("desk-1".into()),
        lease: Some(Counter(1)),
        request_id: Some(Counter(id)),
        expected_revision: Some(Counter(rev)),
        command: ProcessingCommand::ProcessingSet {
            input: "input-01".into(),
            config: Config {
                compressor_bypass: false,
                makeup_mdb: 6000,
                ..Config::default()
            },
        },
        ..read()
    }
}
fn attach(c: &mut Client, s: &mut LocalAudio, now: &mut u64) {
    let mut r = Request {
        contract: "C-AUDIO".into(),
        version: 1,
        show_id: SHOW.into(),
        module: "audio".into(),
        epoch: Counter(9),
        writer: None,
        lease: None,
        request_id: None,
        expected_revision: None,
        command: Command::Snapshot {},
    };
    c.send(&r.encode().unwrap());
    c.wait(s, now, |v| v["capability"] == "GP03-rendered");
    r.writer = Some("desk-1".into());
    r.request_id = Some(Counter(1));
    r.expected_revision = Some(Counter(0));
    r.command = Command::Grant { scope: Scope::Foh };
    c.send(&r.encode().unwrap());
    c.wait(s, now, |v| v["outcome"]["body"]["granted_lease"] == "1");
}
#[test]
fn actual_endpoint_freshness_cached_retry_output_and_disconnect() {
    let dir = std::env::temp_dir().join(format!("gp07-local-{}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
    let mut server = LocalAudio::bind(&dir, "audio.sock", SHOW, Counter(9)).unwrap();
    let mut c = Client::new(&dir.join("audio.sock"));
    let mut now = 0;
    attach(&mut c, &mut server, &mut now);
    let missing = c.processing(&mut server, &mut now, &set(2, 0), "final");
    assert_eq!(missing.reason.as_deref(), Some("stale_snapshot"));
    c.processing(&mut server, &mut now, &read(), "final");
    let request = set(3, 0);
    let pending = c.processing(&mut server, &mut now, &request, "pending");
    let v = c.wait(&mut server, &mut now, |v| {
        v["contract"] == "GP07-processing"
            && v["state"] == "final"
            && v["context"]["request_id"] == "3"
    });
    let applied = ProcessingReply::decode(&serde_json::to_vec(&v).unwrap()).unwrap();
    assert_eq!(applied.ticket, pending.ticket);
    for _ in 0..6 {
        now += 1;
        server.tick(now).unwrap();
    }
    let mut output = [[0.; 4]; 48];
    now += 1;
    server.tick_with_output(now, &mut output).unwrap();
    let expected =
        0.125 * 10_f64.powf(-6. / 20.) * std::f64::consts::FRAC_1_SQRT_2 * 10_f64.powf(6. / 20.);
    assert!((output[0][0] - expected).abs() < 1e-15);
    assert_eq!(output[0][2], 0.125);
    now += 300;
    let retry = c.processing(&mut server, &mut now, &request, "final");
    assert_eq!(retry, applied);
    let stale = c.processing(&mut server, &mut now, &set(4, 1), "final");
    assert_eq!(stale.reason.as_deref(), Some("stale_snapshot"));
    c.processing(&mut server, &mut now, &read(), "final");
    assert_eq!(
        c.processing(&mut server, &mut now, &set(4, 1), "final"),
        stale
    );
    // Admitted work survives connection loss; reconnect cannot replay its authority.
    let p = c.processing(&mut server, &mut now, &set(5, 1), "pending");
    assert!(p.ticket.is_some());
    drop(c);
    now += 1;
    server.tick(now).unwrap();
    now += 1;
    server.tick(now).unwrap();
    assert_eq!(server.snapshot().unwrap().authority.revision, Counter(2));
    let mut c = Client::new(&dir.join("audio.sock"));
    c.processing(&mut server, &mut now, &read(), "final");
    assert_eq!(
        c.processing(&mut server, &mut now, &set(5, 1), "final")
            .reason
            .as_deref(),
        Some("lease")
    );
    drop(c);
    drop(server);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn unsupported_versions_preserve_connection_shared_history_revision_and_lease() {
    let dir = std::env::temp_dir().join(format!("gp07-versions-{}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
    let mut server = LocalAudio::bind(&dir, "audio.sock", SHOW, Counter(9)).unwrap();
    let mut c = Client::new(&dir.join("audio.sock"));
    let mut now = 0;
    attach(&mut c, &mut server, &mut now);
    let legacy: serde_json::Value =
        serde_json::from_slice(include_bytes!("fixtures/gp07/v1/set-request.json")).unwrap();
    fn refuse(
        c: &mut Client,
        s: &mut LocalAudio,
        now: &mut u64,
        mut value: serde_json::Value,
        version: u32,
        id: u64,
        revision: u64,
    ) {
        value["version"] = version.into();
        value["request_id"] = id.to_string().into();
        c.send(&serde_json::to_vec(&value).unwrap());
        let reply = c.wait(s, now, |v| {
            v["contract"] == "GP07-processing" && v["version"] == version
        });
        assert_eq!(reply["state"], "final");
        assert_eq!(reply["reason"], "unsupported_version");
        assert_eq!(reply["revision"], revision.to_string());
        assert_eq!(reply["context"]["request_id"], id.to_string());
        for field in ["snapshot", "ticket", "effective_frame", "ramp_frames"] {
            assert!(reply[field].is_null());
        }
    }
    // Legacy body has shelves. It is refused before Config parsing and ID admission.
    for version in [0, 1, 3, u32::MAX] {
        refuse(&mut c, &mut server, &mut now, legacy.clone(), version, 2, 0);
    }
    c.processing(&mut server, &mut now, &read(), "final");
    let request = set(2, 0);
    c.processing(&mut server, &mut now, &request, "pending");
    let final_v = c.wait(&mut server, &mut now, |v| {
        v["contract"] == "GP07-processing" && v["state"] == "final"
    });
    let original = ProcessingReply::decode(&serde_json::to_vec(&final_v).unwrap()).unwrap();
    // A legacy same-ID request must neither retrieve cached v2 success nor poison retries.
    refuse(&mut c, &mut server, &mut now, legacy.clone(), 1, 2, 1);
    assert_eq!(
        c.processing(&mut server, &mut now, &request, "final"),
        original
    );
    refuse(&mut c, &mut server, &mut now, legacy.clone(), 1, 10000, 1);
    for _ in 0..6 {
        now += 1;
        server.tick(now).unwrap();
    }
    c.processing(&mut server, &mut now, &read(), "final");
    assert_eq!(
        c.processing(&mut server, &mut now, &set(3, 1), "pending")
            .state,
        "pending"
    );
    c.wait(&mut server, &mut now, |v| {
        v["contract"] == "GP07-processing" && v["state"] == "final"
    });
    // GP03 continues on exactly the same stream after complete unsupported frames.
    let audio = Request {
        contract: "C-AUDIO".into(),
        version: 1,
        show_id: SHOW.into(),
        module: "audio".into(),
        epoch: Counter(9),
        writer: None,
        lease: None,
        request_id: None,
        expected_revision: None,
        command: Command::Snapshot {},
    };
    c.send(&audio.encode().unwrap());
    let snapshot = c.wait(&mut server, &mut now, |v| {
        v["capability"] == "GP03-rendered"
    });
    assert_eq!(snapshot["snapshot"]["authority"]["revision"], "2");
    now = 1999;
    refuse(&mut c, &mut server, &mut now, legacy, 1, 10001, 2);
    now = 2100;
    let expired = c.processing(&mut server, &mut now, &set(4, 2), "final");
    assert_eq!(expired.reason.as_deref(), Some("lease"));
    drop(c);
    drop(server);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn gp05_and_v2_processing_reuse_refuses_both_directions_without_lifecycle() {
    use gigpies::module_wire::{ModuleCommand, ModuleRequest};
    let dir = std::env::temp_dir().join(format!("gp07-module-history-{}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
    let mut server = LocalAudio::bind(&dir, "audio.sock", SHOW, Counter(9)).unwrap();
    let mut client = Client::new(&dir.join("audio.sock"));
    let mut now = 0;
    attach(&mut client, &mut server, &mut now);
    let mut module = ModuleRequest {
        contract: "GP05-modules".into(),
        version: 1,
        show_id: SHOW.into(),
        module: "audio".into(),
        epoch: Counter(9),
        writer: Some("desk-1".into()),
        lease: Some(Counter(1)),
        request_id: Some(Counter(2)),
        expected_revision: Some(Counter(0)),
        command: ModuleCommand::RecordStart {
            take_id: "no-take".into(),
            operation_id: Counter(1),
        },
    };
    client.send(&module.encode().unwrap());
    let rejected = client.wait(&mut server, &mut now, |v| v["contract"] == "GP05-modules");
    assert_eq!(rejected["reason"], "modules unavailable");
    client.processing(&mut server, &mut now, &read(), "final");
    assert_eq!(
        client
            .processing(&mut server, &mut now, &set(2, 0), "final")
            .reason
            .as_deref(),
        Some("reused_id")
    );
    assert_eq!(server.snapshot().unwrap().authority.revision, Counter(0));
    let request = set(3, 0);
    client.processing(&mut server, &mut now, &request, "pending");
    client.wait(&mut server, &mut now, |v| {
        v["contract"] == "GP07-processing" && v["state"] == "final"
    });
    module.request_id = Some(Counter(3));
    client.send(&module.encode().unwrap());
    let rejected = client.wait(&mut server, &mut now, |v| v["contract"] == "GP05-modules");
    assert_eq!(rejected["reason"], "reused_id");
    assert_eq!(server.snapshot().unwrap().authority.revision, Counter(1));
    assert!(!dir.join("takes").exists());
    assert_eq!(
        client
            .processing(&mut server, &mut now, &request, "final")
            .revision,
        Counter(1)
    );
    drop(client);
    drop(server);
    std::fs::remove_dir_all(dir).unwrap();
}
