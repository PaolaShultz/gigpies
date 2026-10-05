#![cfg(all(target_os = "linux", feature = "hardware-host"))]
//! Explicit software acceptance with independently built Desk and owner libraries.
use gigpies::{
    analysis_stream::{Descriptor, WindowCursor, local::monotonic_ms},
    channel_processing::{Config as StripConfig, Prepared as StripPrepared},
    control_model::{Command, Request, Scope},
    host::adapters::Dsp,
    local_audio::LocalAudio,
    mixer::Mixer,
    mixer::Prepared as MixerPrepared,
    module_graph::Manifest,
    module_wire::{ModuleCommand, ModuleRequest},
    show::Counter,
};
use std::{
    io::{ErrorKind, Read, Write},
    os::unix::{fs::PermissionsExt, net::UnixStream},
    path::{Path, PathBuf},
    process::{Child, Command as ProcessCommand, Stdio},
    time::{Duration, Instant},
};
const SHOW: &str = "11111111-1111-4111-8111-111111111111";
fn pcm(frame: u64, channel: usize) -> i32 {
    ((frame % 4096) as i32 - 2048) * (channel as i32 + 1) * 32
}
struct Directory(PathBuf);
impl Drop for Directory {
    fn drop(&mut self) {
        if std::thread::panicking() {
            eprintln!("GP07 failure evidence retained at {}", self.0.display());
            if let Ok(destination) = std::env::var("GP07_ACCEPTANCE_EVIDENCE") {
                let destination = PathBuf::from(destination);
                if let Some(parent) = destination.parent() {
                    for name in ["driver.log", "driver.json"] {
                        let source = self.0.join(name);
                        if source.is_file() {
                            let _ = std::fs::copy(source, parent.join(format!("failed-{name}")));
                        }
                    }
                }
            }
        } else {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}
struct Driver(Child);
impl Drop for Driver {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            let _ = self.0.kill();
        }
        let _ = self.0.wait();
    }
}
struct Client {
    socket: UnixStream,
    bytes: Vec<u8>,
}
impl Client {
    fn new(path: &Path) -> Self {
        let socket = UnixStream::connect(path).unwrap();
        socket.set_nonblocking(true).unwrap();
        Self {
            socket,
            bytes: Vec::new(),
        }
    }
    fn send(&mut self, bytes: &[u8]) {
        self.socket.set_nonblocking(false).unwrap();
        self.socket
            .set_write_timeout(Some(Duration::from_millis(100)))
            .unwrap();
        self.socket
            .write_all(&(bytes.len() as u32).to_be_bytes())
            .unwrap();
        self.socket.write_all(bytes).unwrap();
        self.socket.set_nonblocking(true).unwrap();
    }
    fn next(&mut self) -> Option<Vec<u8>> {
        let mut buf = [0; 8192];
        loop {
            match self.socket.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => self.bytes.extend_from_slice(&buf[..n]),
                Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                Err(e) => panic!("socket read: {e}"),
            }
        }
        assert!(self.bytes.len() < 1_048_576);
        if self.bytes.len() < 4 {
            return None;
        }
        let n = u32::from_be_bytes(self.bytes[..4].try_into().unwrap()) as usize;
        assert!(n <= 65536);
        if self.bytes.len() < n + 4 {
            return None;
        }
        let result = self.bytes[4..n + 4].to_vec();
        self.bytes.drain(..n + 4);
        Some(result)
    }
}
struct Harness {
    server: LocalAudio,
    baseline: Mixer,
    fx: Dsp,
    pa: Dsp,
    analysis: Client,
    cursor: Option<WindowCursor>,
    windows: [u64; 3],
    phase: usize,
    ticks: u64,
    changed: u64,
    baseline_ticks: u64,
    deadline: Instant,
    captured: Vec<[[f64; 4]; 48]>,
    edits: Vec<(usize, u64, StripConfig)>,
}
impl Harness {
    fn tick(&mut self) {
        assert!(
            Instant::now() < self.deadline,
            "acceptance overall deadline"
        );
        let frame = self.server.frame();
        let now = monotonic_ms().unwrap();
        let mut actual = [[0.; 4]; 48];
        self.server.tick_with_output(now, &mut actual).unwrap();
        self.captured.push(actual);
        let observation = self.server.processing_snapshot().unwrap();
        assert_eq!(observation.frame.0, frame + 48);
        for (channel, state) in observation.channels.iter().enumerate() {
            if state.transition_remaining_frames > 0 {
                let boundary =
                    observation.frame.0 + u64::from(state.transition_remaining_frames) - 240;
                if !self
                    .edits
                    .iter()
                    .any(|(ch, at, _)| *ch == channel && *at == boundary)
                {
                    self.edits.push((channel, boundary, state.target));
                }
            }
        }
        let raw = std::array::from_fn::<_, 48, _>(|f| {
            std::array::from_fn::<_, 8, _>(|ch| f64::from(pcm(frame + f as u64, ch)) / 8388608.)
        });
        let mut reference = [[0.; 4]; 48];
        self.baseline.process(&raw, &mut reference).unwrap();
        for (a, b) in actual.iter().zip(reference) {
            for ch in 2..4 {
                assert_eq!(
                    a[ch].to_bits(),
                    b[ch].to_bits(),
                    "monitor frame {frame} channel {ch}"
                );
            }
        }
        let differs = actual.iter().zip(reference).any(|(a, b)| a[..2] != b[..2]);
        if self.phase == 0 {
            assert!(!differs, "neutral provider must match established mixer");
            self.baseline_ticks += 1;
        } else if differs {
            self.changed += 1;
        }
        let dry: Vec<_> = actual.iter().flat_map(|f| f[..2].iter().copied()).collect();
        let mut wet = [0.; 96];
        assert_eq!(self.fx.process_result(&dry, &mut wet, 2), 0);
        let sum: Vec<_> = dry.iter().zip(wet).map(|(d, w)| d + w).collect();
        let mut wanted = [0.; 288];
        assert_eq!(self.pa.process_result(&sum, &mut wanted, 6), 0);
        assert_eq!(
            self.server.module_output().unwrap(),
            wanted.as_slice(),
            "actual FX wet plus dry -> actual PA frame {frame}"
        );
        while let Some(bytes) = self.analysis.next() {
            if let Some(cursor) = &mut self.cursor {
                let window = cursor.accept(&bytes, monotonic_ms().unwrap()).unwrap();
                for bytes in window.packets.chunks_exact(624) {
                    let packet = gigpies::transport::Packet::parse(bytes).unwrap();
                    for i in 0..192 {
                        assert_eq!(
                            packet.pcm(i).unwrap(),
                            pcm(packet.source_frame() + (i / 4) as u64, i % 4)
                        );
                    }
                }
                self.windows[self.phase] += 1;
            } else {
                let d: Descriptor = serde_json::from_slice(&bytes).unwrap();
                d.validate().unwrap();
                assert_eq!(d.inputs, ["input-01", "input-02", "input-03", "input-04"]);
                self.cursor = Some(WindowCursor::new(d).unwrap());
            }
        }
        self.ticks += 1;
        std::thread::sleep(Duration::from_millis(1));
    }
    fn wait(
        &mut self,
        c: &mut Client,
        predicate: impl Fn(&serde_json::Value) -> bool,
    ) -> serde_json::Value {
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            assert!(Instant::now() < deadline, "reply deadline");
            self.tick();
            while let Some(bytes) = c.next() {
                let value = serde_json::from_slice(&bytes).unwrap();
                if predicate(&value) {
                    return value;
                }
            }
        }
    }
    fn request(
        &self,
        command: Command,
        writer: Option<&str>,
        lease: Option<Counter>,
        id: Option<u64>,
        revision: u64,
    ) -> Request {
        Request {
            contract: "C-AUDIO".into(),
            version: 1,
            show_id: SHOW.into(),
            module: "audio".into(),
            epoch: Counter(100),
            writer: writer.map(Into::into),
            lease,
            request_id: id.map(Counter),
            expected_revision: id.map(|_| Counter(revision)),
            command,
        }
    }
    fn attach(&mut self, c: &mut Client, writer: &str) -> (Counter, u64) {
        c.send(
            &self
                .request(Command::Snapshot {}, None, None, None, 0)
                .encode()
                .unwrap(),
        );
        let v = self.wait(c, |v| v["snapshot"].is_object());
        let revision = v["snapshot"]["authority"]["revision"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        c.send(
            &self
                .request(
                    Command::Grant { scope: Scope::Foh },
                    Some(writer),
                    None,
                    Some(1),
                    revision,
                )
                .encode()
                .unwrap(),
        );
        let v = self.wait(c, |v| v["outcome"]["body"]["granted_lease"].is_string());
        (
            Counter(
                v["outcome"]["body"]["granted_lease"]
                    .as_str()
                    .unwrap()
                    .parse()
                    .unwrap(),
            ),
            revision,
        )
    }
    fn recording(
        &mut self,
        c: &mut Client,
        writer: &str,
        lease: Counter,
        revision: u64,
        start: bool,
    ) {
        let command = if start {
            ModuleCommand::RecordStart {
                take_id: "gp07".into(),
                operation_id: Counter(17),
            }
        } else {
            ModuleCommand::RecordStop {
                take_id: "gp07".into(),
                operation_id: Counter(17),
            }
        };
        let r = ModuleRequest {
            contract: "GP05-modules".into(),
            version: 1,
            show_id: SHOW.into(),
            module: "audio".into(),
            epoch: Counter(100),
            writer: Some(writer.into()),
            lease: Some(lease),
            request_id: Some(Counter(2)),
            expected_revision: Some(Counter(revision)),
            command,
        };
        c.send(&r.encode().unwrap());
        self.wait(c, |v| v["request_id"] == "2" && v["state"] == "completed");
    }
}
#[test]
#[ignore = "explicit independently verified GP05_MANIFEST and GP07_DESK_DRIVER; synthetic software acceptance only"]
fn actual_frontend_processing_preserves_raw_and_monitors_and_module_order() {
    let executable = PathBuf::from(
        std::env::var("GP07_DESK_DRIVER").expect("exact independently built Desk test binary"),
    );
    assert!(executable.is_absolute() && executable.is_file());
    let path = std::env::temp_dir().join(format!("gp07-frontend-{}", std::process::id()));
    std::fs::create_dir(&path).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    let dir = Directory(path);
    let manifest_path = PathBuf::from(std::env::var("GP05_MANIFEST").unwrap());
    let manifest = Manifest::load(&manifest_path).unwrap();
    let mut server = LocalAudio::bind(&dir.0, "audio.sock", SHOW, Counter(100)).unwrap();
    server.enable_synthetic_fouraux().unwrap();
    server.enable_analysis(&dir.0).unwrap();
    server.enable_modules(&manifest_path).unwrap();
    let mut analysis = Client::new(&dir.0.join("analysis.sock"));
    analysis.send(br#"{"subscription":"lux.aux.v1","version":1}"#);
    let mut h = Harness {
        server,
        baseline: Mixer::default(),
        fx: Dsp::load(&manifest.fx.library, "fx", 48).unwrap(),
        pa: Dsp::load(&manifest.pa.library, "pa", 48).unwrap(),
        analysis,
        cursor: None,
        windows: [0; 3],
        phase: 0,
        ticks: 0,
        changed: 0,
        baseline_ticks: 0,
        deadline: Instant::now() + Duration::from_secs(35),
        captured: Vec::new(),
        edits: Vec::new(),
    };
    let mut c = Client::new(&dir.0.join("audio.sock"));
    let (lease, revision) = h.attach(&mut c, "recorder-before");
    h.recording(&mut c, "recorder-before", lease, revision, true);
    c.send(
        &h.request(
            Command::Release {},
            Some("recorder-before"),
            Some(lease),
            Some(3),
            revision,
        )
        .encode()
        .unwrap(),
    );
    let released = h.wait(&mut c, |v| {
        v["state"] == "final" && v["context"]["request_id"] == "3"
    });
    assert_eq!(released["outcome"]["kind"], "applied");
    drop(c);
    while h.windows[0] < 3 {
        h.tick();
    }
    let driver_start_frame = h.server.frame();
    h.phase = 1;
    let evidence = dir.0.join("driver.json");
    let stdout = std::fs::File::create(dir.0.join("driver.log")).unwrap();
    let stderr = stdout.try_clone().unwrap();
    let mut driver = Driver(
        ProcessCommand::new(executable)
            .args([
                "--ignored",
                "--exact",
                "gp07_external_driver",
                "--nocapture",
            ])
            .env("GP07_EXTERNAL_ENDPOINT", dir.0.join("audio.sock"))
            .env("GP07_DRIVER_EVIDENCE", &evidence)
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr))
            .spawn()
            .unwrap(),
    );
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        h.tick();
        if let Some(status) = driver.0.try_wait().unwrap() {
            assert!(
                status.success(),
                "Desk driver failed: {}",
                std::fs::read_to_string(dir.0.join("driver.log"))
                    .unwrap()
                    .chars()
                    .take(8192)
                    .collect::<String>()
            );
            break;
        }
        assert!(Instant::now() < deadline, "Desk driver deadline");
    }
    drop(driver);
    let driver_end_frame = h.server.frame();
    let driver_evidence: serde_json::Value = serde_json::from_slice(
        &std::fs::read(evidence).expect("driver must emit acceptance evidence"),
    )
    .unwrap();
    h.phase = 2;
    let expiry = Instant::now() + Duration::from_millis(2200);
    while Instant::now() < expiry {
        h.tick();
    }
    let confirmed = h.server.processing_snapshot().unwrap();
    let confirmed_revision = driver_evidence["reconnect_revision"]
        .as_str()
        .unwrap()
        .parse::<u64>()
        .unwrap();
    assert_eq!(
        confirmed.revision.0, confirmed_revision,
        "no delayed reconnect replay"
    );
    assert_eq!(
        h.server.snapshot().unwrap().authority.revision.0,
        confirmed_revision
    );
    let configs: [StripConfig; 2] = ["channel1", "channel2"]
        .map(|key| serde_json::from_value(driver_evidence[key].clone()).unwrap());
    for (ch, observed) in confirmed.channels.iter().enumerate() {
        let expected = configs.get(ch).copied().unwrap_or_default();
        assert!(observed.ready);
        assert_eq!(observed.current, expected);
        assert_eq!(observed.target, expected);
    }
    let mut c = Client::new(&dir.0.join("audio.sock"));
    let (lease, revision) = h.attach(&mut c, "recorder-after");
    h.recording(&mut c, "recorder-after", lease, revision, false);
    let status = h.server.module_status(monotonic_ms().unwrap()).unwrap();
    let recording = &status["recording"];
    assert_eq!(recording["state"], "finalized");
    assert_eq!(recording["outcome"], "complete");
    let first = recording["first_source_frame"]
        .as_str()
        .unwrap()
        .parse::<u64>()
        .unwrap();
    let count = recording["written_frames"]
        .as_str()
        .unwrap()
        .parse::<u64>()
        .unwrap();
    assert!(count > 480);
    assert!(
        first <= driver_start_frame,
        "REC must start before frontend edits"
    );
    assert!(
        first + count >= driver_end_frame,
        "REC must cover complete frontend run"
    );
    assert_eq!(recording["accepted_frames"], recording["written_frames"]);
    let mut paths: Vec<_> = std::fs::read_dir(dir.0.join("takes/gp07"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "wav"))
        .collect();
    paths.sort();
    assert_eq!(paths.len(), 8);
    for (ch, path) in paths.iter().enumerate() {
        let mut wav = hound::WavReader::open(path).unwrap();
        assert_eq!(wav.spec().bits_per_sample, 24);
        assert_eq!(wav.spec().channels, 1);
        let mut n = 0;
        for (i, sample) in wav.samples::<i32>().enumerate() {
            assert_eq!(sample.unwrap(), pcm(first + i as u64, ch));
            n += 1;
        }
        assert_eq!(n, count);
    }
    assert!(
        h.changed > 20,
        "frontend edits must change actual FOH samples"
    );
    assert!(
        h.windows.iter().all(|n| *n > 0),
        "analysis before/during/after driver"
    );
    // Each physical input is independently rendered in slot zero. This deliberately
    // avoids repeating the provider's channel-index routing: input-02's config is
    // applied to slot zero of its own mixer, not slot one of a shared mixer.
    assert_eq!(
        h.edits.len(),
        2,
        "exactly the two confirmed frontend transactions"
    );
    for (ch, config) in configs.iter().enumerate() {
        assert_eq!(
            h.edits
                .iter()
                .filter(|(index, _, value)| *index == ch && value == config)
                .count(),
            1
        );
    }
    let mut isolated: [Mixer; 8] = std::array::from_fn(|_| Mixer::default());
    for (block, actual) in h.captured.iter().enumerate() {
        let frame = block as u64 * 48;
        let mut expected = [[0.; 2]; 48];
        for (ch, mixer) in isolated.iter_mut().enumerate() {
            for (edited, boundary, config) in &h.edits {
                if *edited == ch && *boundary == frame + 48 {
                    let prepared =
                        MixerPrepared::processing(0, StripPrepared::new(*config).unwrap()).unwrap();
                    assert_eq!(mixer.schedule(prepared, ch as u64 + 1).unwrap(), *boundary);
                }
            }
            let raw = std::array::from_fn::<_, 48, _>(|i| {
                let mut input = [0.; 8];
                input[0] = f64::from(pcm(frame + i as u64, ch)) / 8388608.;
                input
            });
            let mut output = [[0.; 4]; 48];
            mixer.process(&raw, &mut output).unwrap();
            let _ = mixer.take_completion();
            for (sum, contribution) in expected.iter_mut().zip(output) {
                for bus in 0..2 {
                    sum[bus] += contribution[bus];
                }
            }
        }
        for (i, (sample, wanted)) in actual.iter().zip(expected).enumerate() {
            for bus in 0..2 {
                assert_eq!(
                    sample[bus].to_bits(),
                    wanted[bus].to_bits(),
                    "isolated per-input FOH frame {} bus {bus}",
                    frame + i as u64
                );
            }
        }
    }
    let result = serde_json::json!({"driver":driver_evidence,"driver_start_frame":driver_start_frame,"driver_end_frame":driver_end_frame,"processing_boundaries":h.edits.iter().map(|(ch, frame, _)| serde_json::json!({"channel":ch+1,"frame":frame})).collect::<Vec<_>>(),"channel_mapping_comparison":"eight isolated slot-zero mixers, exact full FOH timeline","ticks_checked":h.ticks,"neutral_ticks":h.baseline_ticks,"changed_foh_ticks":h.changed,"analysis_windows":h.windows,"raw_stems":8,"raw_frames_per_stem":count,"first_source_frame":first,"recording":recording,"monitor_comparison":"bit exact every tick","module_comparison":"independent actual FX wet+dry -> PA every tick"});
    if let Ok(path) = std::env::var("GP07_ACCEPTANCE_EVIDENCE") {
        std::fs::write(path, serde_json::to_vec_pretty(&result).unwrap()).unwrap();
    }
    drop(c);
    drop(h);
    drop(dir);
}
