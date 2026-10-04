#![cfg(all(target_os = "linux", feature = "hardware-host"))]
use gigpies::{
    control_model::{Command, Request, Scope},
    local_audio::LocalAudio,
    module_wire::{ModuleCommand, ModuleRequest},
    show::Counter,
};
use std::{
    io::{ErrorKind, Read, Write},
    os::unix::{fs::PermissionsExt, net::UnixStream},
    time::Duration,
};
const SHOW: &str = "11111111-1111-4111-8111-111111111111";
struct Client {
    s: UnixStream,
    bytes: Vec<u8>,
}
impl Client {
    fn new(path: &std::path::Path) -> Self {
        let s = UnixStream::connect(path).unwrap();
        s.set_nonblocking(true).unwrap();
        Self {
            s,
            bytes: Vec::new(),
        }
    }
    fn send(&mut self, b: &[u8]) {
        self.s.write_all(&(b.len() as u32).to_be_bytes()).unwrap();
        self.s.write_all(b).unwrap();
    }
    fn next(&mut self) -> Option<serde_json::Value> {
        let mut b = [0; 8192];
        loop {
            match self.s.read(&mut b) {
                Ok(0) => break,
                Ok(n) => self.bytes.extend_from_slice(&b[..n]),
                Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                Err(e) => panic!("{e}"),
            }
        }
        if self.bytes.len() < 4 {
            return None;
        }
        let n = u32::from_be_bytes(self.bytes[..4].try_into().unwrap()) as usize;
        assert!(n <= 65536);
        if self.bytes.len() < n + 4 {
            return None;
        }
        let v = serde_json::from_slice(&self.bytes[4..n + 4]).unwrap();
        self.bytes.drain(..n + 4);
        Some(v)
    }
    fn wait(
        &mut self,
        server: &mut LocalAudio,
        now: &mut u64,
        predicate: impl Fn(&serde_json::Value) -> bool,
    ) -> serde_json::Value {
        for _ in 0..1000 {
            *now += 1;
            server.tick(*now).unwrap();
            while let Some(v) = self.next() {
                if predicate(&v) {
                    return v;
                }
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        panic!("reply timeout");
    }
}
fn req(command: Command, writer: Option<&str>, lease: Option<Counter>, n: Option<u64>) -> Request {
    Request {
        contract: "C-AUDIO".into(),
        version: 1,
        show_id: SHOW.into(),
        module: "audio".into(),
        epoch: Counter(9),
        writer: writer.map(Into::into),
        lease,
        request_id: n.map(Counter),
        expected_revision: n.map(|_| Counter(0)),
        command,
    }
}
fn attach(server: &mut LocalAudio, c: &mut Client, now: &mut u64, writer: &str) -> Counter {
    c.send(
        &req(Command::Snapshot {}, None, None, None)
            .encode()
            .unwrap(),
    );
    c.wait(server, now, |v| {
        v.get("snapshot").is_some_and(|s| !s.is_null())
    });
    c.send(
        &req(
            Command::Grant { scope: Scope::Foh },
            Some(writer),
            None,
            Some(1),
        )
        .encode()
        .unwrap(),
    );
    let v = c.wait(server, now, |v| {
        v["outcome"]["body"]["granted_lease"].is_string()
    });
    Counter(
        v["outcome"]["body"]["granted_lease"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap(),
    )
}
#[test]
#[ignore = "explicit accepted owner libraries; only private owned UDS and synthetic PCM"]
fn actual_service_recording_survives_client_and_analysis_loss_and_has_shared_identity() {
    let p = std::env::temp_dir().join(format!("gp05-service-{}", std::process::id()));
    std::fs::create_dir(&p).unwrap();
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o700)).unwrap();
    let mut server = LocalAudio::bind(&p, "audio.sock", SHOW, Counter(9)).unwrap();
    server.enable_synthetic_fouraux().unwrap();
    server.enable_analysis(&p).unwrap();
    server
        .enable_modules(std::path::Path::new(
            &std::env::var("GP05_MANIFEST").unwrap(),
        ))
        .unwrap();
    let mut now = 0;
    let mut client = Client::new(&p.join("audio.sock"));
    let lease = attach(&mut server, &mut client, &mut now, "writer-first");
    let start = ModuleRequest {
        contract: "GP05-modules".into(),
        version: 1,
        show_id: SHOW.into(),
        module: "audio".into(),
        epoch: Counter(9),
        writer: Some("writer-first".into()),
        lease: Some(lease),
        request_id: Some(Counter(2)),
        expected_revision: Some(Counter(0)),
        command: ModuleCommand::RecordStart {
            take_id: "take-01".into(),
            operation_id: Counter(17),
        },
    };
    client.send(&start.encode().unwrap());
    let admission = client.wait(&mut server, &mut now, |v| v["request_id"] == "2");
    assert_eq!(admission["state"], "accepted_pending");
    client.wait(&mut server, &mut now, |v| {
        v["request_id"] == "2" && v["state"] == "completed"
    });
    client.send(&start.encode().unwrap());
    let retry = client.wait(&mut server, &mut now, |v| v["request_id"] == "2");
    assert_eq!(retry["state"], "completed");
    let mut changed = start.clone();
    changed.command = ModuleCommand::RecordStart {
        take_id: "take-02".into(),
        operation_id: Counter(18),
    };
    client.send(&changed.encode().unwrap());
    let rejected = client.wait(&mut server, &mut now, |v| v["request_id"] == "2");
    assert_eq!(rejected["reason"], "reused_id");
    assert!(!p.join("takes/take-02").exists());
    client.send(
        &req(
            Command::Renew {},
            Some("writer-first"),
            Some(lease),
            Some(2),
        )
        .encode()
        .unwrap(),
    );
    let cross = client.wait(&mut server, &mut now, |v| {
        v["outcome"]["body"]["reason"] == "reused_id"
    });
    assert_eq!(cross["state"], "final");
    let before = server.module_status(now).unwrap()["recording"]["accepted_frames"]
        .as_str()
        .unwrap()
        .parse::<u64>()
        .unwrap();
    drop(client);
    server.stop_analysis();
    for _ in 0..2200 {
        now += 1;
        server.tick(now).unwrap();
        std::thread::sleep(Duration::from_millis(1));
    }
    let after = server.module_status(now).unwrap();
    assert_eq!(after["recording"]["state"], "recording");
    assert!(
        after["recording"]["accepted_frames"]
            .as_str()
            .unwrap()
            .parse::<u64>()
            .unwrap()
            > before
    );
    assert!(!server.analysis_alive());
    let mut client = Client::new(&p.join("audio.sock"));
    let lease = attach(&mut server, &mut client, &mut now, "writer-second");
    let mut stop = start.clone();
    stop.writer = Some("writer-second".into());
    stop.lease = Some(lease);
    stop.command = ModuleCommand::RecordStop {
        take_id: "take-01".into(),
        operation_id: Counter(17),
    };
    client.send(&stop.encode().unwrap());
    let admit = client.wait(&mut server, &mut now, |v| v["request_id"] == "2");
    assert_eq!(admit["state"], "accepted_pending");
    client.wait(&mut server, &mut now, |v| {
        v["request_id"] == "2" && v["state"] == "completed"
    });
    let final_status = server.module_status(now + 100).unwrap();
    assert_eq!(final_status["recording"]["outcome"], "complete");
    assert!(final_status["recording"]["durable_frames"].is_null());
    let first = final_status["recording"]["first_source_frame"]
        .as_str()
        .unwrap()
        .parse::<u64>()
        .unwrap();
    let count = final_status["recording"]["written_frames"]
        .as_str()
        .unwrap()
        .parse::<u64>()
        .unwrap();
    assert!(count > 48);
    let mut paths: Vec<_> = std::fs::read_dir(p.join("takes/take-01"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "wav"))
        .collect();
    paths.sort();
    assert_eq!(paths.len(), 8);
    for (channel, path) in paths.iter().enumerate() {
        let mut wav = hound::WavReader::open(path).unwrap();
        let samples: Vec<_> = wav.samples::<i32>().map(Result::unwrap).collect();
        assert_eq!(samples.len(), count as usize);
        for (i, block) in samples.chunks_exact(48).enumerate() {
            let raw = gigpies::analysis_stream::synthetic_inputs(first + i as u64 * 48);
            for (j, sample) in block.iter().enumerate() {
                assert_eq!(*sample, raw[j][channel]);
            }
        }
    }
    drop(server);
    drop(client);
    std::fs::remove_dir_all(p).unwrap();
}
