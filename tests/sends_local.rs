#![cfg(target_os = "linux")]
use gigpies::{
    control_model::{Command, Request, Scope},
    local_audio::LocalAudio,
    sends_wire::{SendsCommand, SendsReply, SendsRequest, Tap},
    show::Counter,
    topology::EngineTopology,
};
use std::{
    io::{ErrorKind, Read, Write},
    os::unix::{fs::PermissionsExt, net::UnixStream},
};
const SHOW: &str = "11111111-1111-4111-8111-111111111111";
struct Client {
    stream: UnixStream,
    bytes: Vec<u8>,
    assembly: gigpies::snapshot_pages::Assembly,
    inbox: Vec<serde_json::Value>,
    pages: usize,
    assembled_bytes: usize,
}
impl Client {
    fn new(path: &std::path::Path) -> Self {
        let stream = UnixStream::connect(path).unwrap();
        stream.set_nonblocking(true).unwrap();
        Self {
            stream,
            bytes: vec![],
            assembly: Default::default(),
            inbox: vec![],
            pages: 0,
            assembled_bytes: 0,
        }
    }
    fn send(&mut self, value: &impl serde::Serialize) {
        let b = serde_json::to_vec(value).unwrap();
        self.stream
            .write_all(&(b.len() as u32).to_be_bytes())
            .unwrap();
        self.stream.write_all(&b).unwrap();
    }
    fn wait(
        &mut self,
        server: &mut LocalAudio,
        now: &mut u64,
        predicate: impl Fn(&serde_json::Value) -> bool,
    ) -> serde_json::Value {
        for _ in 0..200 {
            if let Some(i) = self.inbox.iter().position(&predicate) {
                return self.inbox.remove(i);
            }
            *now += 1;
            server.tick(*now).unwrap();
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
                let mut v: serde_json::Value =
                    serde_json::from_slice(&self.bytes[4..n + 4]).unwrap();
                self.bytes.drain(..n + 4);
                if v["contract"] == "GP14-snapshot-pages" {
                    self.pages += 1;
                    let page = serde_json::from_value(v).unwrap();
                    let Some(bytes) = self.assembly.offer(page, *now).unwrap() else {
                        continue;
                    };
                    self.assembled_bytes = bytes.len();
                    v = serde_json::from_slice(&bytes).unwrap();
                }
                if predicate(&v) {
                    return v;
                }
                self.inbox.push(v);
            }
        }
        panic!("bounded reply timeout")
    }
}
fn read() -> SendsRequest {
    SendsRequest {
        contract: "GP18-sends".into(),
        version: 1,
        show_id: SHOW.into(),
        module: "audio".into(),
        epoch: Counter(9),
        writer: None,
        lease: None,
        request_id: None,
        expected_revision: None,
        command: SendsCommand::SendsSnapshot {},
    }
}
fn set(id: u64, rev: u64) -> SendsRequest {
    SendsRequest {
        writer: Some("uds-sends".into()),
        lease: Some(Counter(1)),
        request_id: Some(Counter(id)),
        expected_revision: Some(Counter(rev)),
        command: SendsCommand::SendTapSet {
            input: "input-17".into(),
            monitor: "monitor-3".into(),
            tap: Tap::ProcessedPostFader,
        },
        ..read()
    }
}
fn attach(c: &mut Client, s: &mut LocalAudio, now: &mut u64, id: u64) {
    let mut r = Request {
        contract: "C-AUDIO".into(),
        version: 2,
        show_id: SHOW.into(),
        module: "audio".into(),
        epoch: Counter(9),
        writer: None,
        lease: None,
        request_id: None,
        expected_revision: None,
        command: Command::Snapshot {},
    };
    c.send(&r);
    let v = c.wait(s, now, |v| {
        v["capability"] == "GP03-rendered" && v["snapshot"].is_object()
    });
    r.writer = Some("uds-sends".into());
    r.request_id = Some(Counter(id));
    r.expected_revision = Some(Counter(
        v["outcome"]["body"]["revision"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap(),
    ));
    r.command = Command::Grant {
        scope: Scope::Monitor(3),
    };
    c.send(&r);
    c.wait(s, now, |v| {
        v["outcome"]["body"]["granted_lease"].is_string()
    });
}
#[test]
fn actual_unix_dispatch_freshness_completions_retry_disconnect_and_recovery() {
    let dir = std::env::temp_dir().join(format!("gp18-uds-{}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
    let mut server = LocalAudio::bind_configured(
        &dir,
        "send.sock",
        SHOW,
        Counter(9),
        EngineTopology::software(17, 3, 0).unwrap(),
    )
    .unwrap();
    let mut c = Client::new(&dir.join("send.sock"));
    let mut now = 0;
    attach(&mut c, &mut server, &mut now, 1);
    c.send(&set(2, 0));
    let v = c.wait(&mut server, &mut now, |v| v["contract"] == "GP18-sends");
    assert_eq!(v["reason"], "stale_snapshot");
    c.send(&read());
    let v = c.wait(&mut server, &mut now, |v| {
        v["contract"] == "GP18-sends" && v["snapshot"].is_object()
    });
    SendsReply::decode(&serde_json::to_vec(&v).unwrap()).unwrap();
    c.send(&set(3, 0));
    let pending = c.wait(&mut server, &mut now, |v| {
        v["contract"] == "GP18-sends" && v["state"] == "pending"
    });
    let final_reply = c.wait(&mut server, &mut now, |v| {
        v["contract"] == "GP18-sends" && v["state"] == "final"
    });
    assert_eq!(pending["ticket"], final_reply["ticket"]);
    assert_eq!(final_reply["revision"], "1");
    c.send(&set(3, 0));
    let cached = c.wait(&mut server, &mut now, |v| {
        v["contract"] == "GP18-sends" && v["state"] == "final"
    });
    assert_eq!(cached, final_reply);
    for _ in 0..8 {
        now += 1;
        server.tick(now).unwrap();
    }
    now += 251;
    c.send(&set(4, 1));
    let stale = c.wait(&mut server, &mut now, |v| v["contract"] == "GP18-sends");
    assert_eq!(stale["reason"], "stale_snapshot");
    c.send(&read());
    c.wait(&mut server, &mut now, |v| {
        v["contract"] == "GP18-sends" && v["snapshot"].is_object()
    });
    let mut change = set(5, 1);
    change.command = SendsCommand::SendTapSet {
        input: "input-17".into(),
        monitor: "monitor-3".into(),
        tap: Tap::RawPostMute,
    };
    c.send(&change);
    c.wait(&mut server, &mut now, |v| {
        v["contract"] == "GP18-sends" && v["state"] == "pending"
    });
    drop(c);
    now += 1;
    server.tick(now).unwrap();
    assert_eq!(server.engine_mut().revision(), Counter(1));
    assert_eq!(
        server.engine_mut().mixer().send_observations()[16][2].1,
        Tap::ProcessedPostFader
    );
    // A canceled Unix owner must not steal the next remote producer completion.
    let grant = Request {
        contract: "C-AUDIO".into(),
        version: 2,
        show_id: SHOW.into(),
        module: "audio".into(),
        epoch: Counter(9),
        writer: Some("remote-after-uds".into()),
        lease: None,
        request_id: Some(Counter(1)),
        expected_revision: Some(Counter(1)),
        command: Command::Grant {
            scope: Scope::Monitor2,
        },
    };
    let granted = server.engine_mut().handle(&grant, now).unwrap();
    assert!(
        granted
            .outcome
            .as_ref()
            .unwrap()
            .body
            .granted_lease
            .is_some(),
        "remote grant: {granted:?}"
    );
    let lease = granted.outcome.unwrap().body.granted_lease.unwrap();
    let mut remote = change.clone();
    remote.writer = Some("remote-after-uds".into());
    remote.lease = Some(lease);
    remote.request_id = Some(Counter(2));
    remote.command = SendsCommand::SendTapSet {
        input: "input-01".into(),
        monitor: "monitor-2".into(),
        tap: Tap::ProcessedPreFader,
    };
    assert_eq!(
        server
            .engine_mut()
            .handle_sends(&remote, now)
            .unwrap()
            .state,
        "pending"
    );
    server.revoke_writer("unrelated-writer");
    for _ in 0..2 {
        now += 1;
        server.tick(now).unwrap();
    }
    let completions = server.take_remote_completions();
    assert!(
        completions.iter().any(|v| v["contract"] == "GP18-sends"
            && v["state"] == "final"
            && v["context"]["writer"] == "remote-after-uds"),
        "{completions:?}"
    );
    assert_eq!(
        server
            .engine_mut()
            .handle_sends(&remote, now)
            .unwrap()
            .revision,
        Counter(2)
    );
    // Actual composed persistence restores settled committed taps and stays disarmed.
    let intent = server.persisted_intent().unwrap();
    let restored_dir = dir.join("restored");
    std::fs::create_dir(&restored_dir).unwrap();
    std::fs::set_permissions(&restored_dir, std::fs::Permissions::from_mode(0o700)).unwrap();
    let mut restored = LocalAudio::bind_configured(
        &restored_dir,
        "send.sock",
        SHOW,
        Counter(10),
        EngineTopology::software(17, 3, 0).unwrap(),
    )
    .unwrap();
    restored.restore_composed_intent(&intent).unwrap();
    assert!(restored.engine_mut().outputs_quiesced());
    assert_eq!(
        restored.engine_mut().mixer().send_observations()[16][2].1,
        Tap::ProcessedPostFader
    );
    assert_eq!(
        restored.engine_mut().mixer().send_observations()[0][1].1,
        Tap::ProcessedPreFader
    );
    assert!(restored.take_remote_completions().is_empty());
    assert!(restored.engine_mut().mixer().sends_ready());
    let mut replay = remote.clone();
    replay.epoch = Counter(10);
    assert_eq!(
        restored
            .engine_mut()
            .handle_sends(&replay, now)
            .unwrap()
            .reason
            .as_deref(),
        Some("lease")
    );
    for _ in 0..8 {
        now += 1;
        restored.tick(now).unwrap();
    }
    assert_eq!(restored.engine_mut().revision(), Counter(0));
    assert!(restored.take_remote_completions().is_empty());
    drop(restored);
    std::fs::remove_dir_all(restored_dir).unwrap();
    server.recover_source(Counter(10), 0).unwrap();
    assert!(server.engine_mut().outputs_quiesced());
    assert!(server.engine_mut().mixer().sends_ready());
    assert_eq!(
        server.engine_mut().mixer().send_observations()[16][2].1,
        Tap::ProcessedPostFader
    );
    assert_eq!(
        server.engine_mut().mixer().send_observations()[0][1].1,
        Tap::ProcessedPreFader
    );
    drop(server);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn actual_large_gp18_unix_read_uses_immutable_pages_and_client_assembly() {
    let dir = std::env::temp_dir().join(format!("gp18-pages-{}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
    let topology = EngineTopology::software(16, 40, 0).unwrap();
    let mut server =
        LocalAudio::bind_configured(&dir, "send.sock", SHOW, Counter(9), topology).unwrap();
    let mut c = Client::new(&dir.join("send.sock"));
    let mut now = 0;
    c.send(&read());
    let value = c.wait(&mut server, &mut now, |v| v["contract"] == "GP18-sends");
    assert!(
        c.assembled_bytes > 65536,
        "actual size {}",
        c.assembled_bytes
    );
    assert!(c.pages > 1);
    let bytes = serde_json::to_vec(&value).unwrap();
    assert_eq!(bytes.len(), c.assembled_bytes);
    let reply = SendsReply::decode_assembled(&bytes).unwrap();
    let snapshot = reply.snapshot.unwrap();
    assert_eq!(snapshot.channels.len(), 16);
    assert_eq!(snapshot.monitors.len(), 40);
    for channel in &snapshot.channels {
        assert_eq!(channel.sends.len(), 40);
        assert!(
            channel
                .sends
                .iter()
                .all(|s| s.current == Tap::RawPostMute && s.target == Tap::RawPostMute && s.ready)
        );
    }
    // Paging spans multiple provider ticks but preserves the original observation.
    assert!(snapshot.frame.0 < server.frame());
    assert_eq!(snapshot.revision, Counter(0));
    eprintln!(
        "GP18 actual immutable reply: {} bytes, {} pages, observed frame {}, delivered frame {}",
        c.assembled_bytes,
        c.pages,
        snapshot.frame.0,
        server.frame()
    );
    drop(c);
    drop(server);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn remote_cancel_then_unix_final_survives_unrelated_revoke_and_cached_final_retry() {
    let dir = std::env::temp_dir().join(format!("gp18-mixed-{}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
    let mut server = LocalAudio::bind_configured(
        &dir,
        "send.sock",
        SHOW,
        Counter(9),
        EngineTopology::software(17, 3, 0).unwrap(),
    )
    .unwrap();
    let mut c = Client::new(&dir.join("send.sock"));
    let mut now = 0;
    attach(&mut c, &mut server, &mut now, 1);
    c.send(&read());
    c.wait(&mut server, &mut now, |v| {
        v["snapshot"].is_object() && v["contract"] == "GP18-sends"
    });
    let mut wrong = set(2, 0);
    wrong.command = SendsCommand::SendTapSet {
        input: "input-01".into(),
        monitor: "monitor-2".into(),
        tap: Tap::ProcessedPreFader,
    };
    c.send(&wrong);
    let refused = c.wait(&mut server, &mut now, |v| {
        v["contract"] == "GP18-sends" && v["state"] == "final"
    });
    assert_eq!(refused["reason"], "scope");
    let original = set(3, 0);
    c.send(&original);
    c.wait(&mut server, &mut now, |v| v["state"] == "pending");
    let first = c.wait(&mut server, &mut now, |v| {
        v["state"] == "final" && v["ticket"].is_string()
    });
    for _ in 0..8 {
        now += 1;
        server.tick(now).unwrap();
    }
    let grant = Request {
        contract: "C-AUDIO".into(),
        version: 2,
        show_id: SHOW.into(),
        module: "audio".into(),
        epoch: Counter(9),
        writer: Some("remote-before-uds".into()),
        lease: None,
        request_id: Some(Counter(1)),
        expected_revision: Some(Counter(1)),
        command: Command::Grant {
            scope: Scope::Monitor2,
        },
    };
    let granted = server.engine_mut().handle(&grant, now).unwrap();
    assert!(
        granted
            .outcome
            .as_ref()
            .unwrap()
            .body
            .granted_lease
            .is_some(),
        "remote grant: {granted:?}"
    );
    let lease = granted.outcome.unwrap().body.granted_lease.unwrap();
    let remote = SendsRequest {
        writer: grant.writer.clone(),
        lease: Some(lease),
        request_id: Some(Counter(2)),
        expected_revision: Some(Counter(1)),
        command: SendsCommand::SendTapSet {
            input: "input-01".into(),
            monitor: "monitor-2".into(),
            tap: Tap::ProcessedPreFader,
        },
        ..read()
    };
    assert_eq!(
        server
            .engine_mut()
            .handle_sends(&remote, now)
            .unwrap()
            .state,
        "pending"
    );
    server.revoke_writer("remote-before-uds");
    assert!(server.take_remote_completions().is_empty());
    let next = SendsRequest {
        command: SendsCommand::SendTapSet {
            input: "input-02".into(),
            monitor: "monitor-3".into(),
            tap: Tap::ProcessedPreFader,
        },
        ..set(4, 1)
    };
    c.send(&next);
    let pending = c.wait(&mut server, &mut now, |v| v["state"] == "pending");
    server.revoke_writer("unrelated-writer");
    c.send(&original);
    let cached = c.wait(&mut server, &mut now, |v| {
        v["ticket"] == first["ticket"] && v["state"] == "final"
    });
    assert_eq!(cached, first);
    let final_reply = c.wait(&mut server, &mut now, |v| {
        v["ticket"] == pending["ticket"] && v["state"] == "final"
    });
    assert_eq!(final_reply["revision"], "2");
    assert_eq!(final_reply["context"]["request_id"], "4");
    assert!(server.take_remote_completions().is_empty());
    assert_eq!(
        server.engine_mut().mixer().send_observations()[0][1].1,
        Tap::RawPostMute
    );
    assert_eq!(
        server.engine_mut().mixer().send_observations()[1][2].1,
        Tap::ProcessedPreFader
    );
    drop(c);
    drop(server);
    std::fs::remove_dir_all(dir).unwrap();
}
