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
}
impl Client {
    fn new(path: &std::path::Path) -> Self {
        let stream = UnixStream::connect(path).unwrap();
        stream.set_nonblocking(true).unwrap();
        Self {
            stream,
            bytes: vec![],
            assembly: Default::default(),
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
                    let page = serde_json::from_value(v).unwrap();
                    let Some(bytes) = self.assembly.offer(page, *now).unwrap() else {
                        continue;
                    };
                    v = serde_json::from_slice(&bytes).unwrap();
                }
                if predicate(&v) {
                    return v;
                }
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
    server.recover_source(Counter(10), 0).unwrap();
    assert!(server.engine_mut().outputs_quiesced());
    assert!(server.engine_mut().mixer().sends_ready());
    assert_eq!(
        server.engine_mut().mixer().send_observations()[16][2].1,
        Tap::ProcessedPostFader
    );
    drop(server);
    std::fs::remove_dir_all(dir).unwrap();
}
