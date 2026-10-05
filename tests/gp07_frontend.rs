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
// Independent direct-form-I evaluation of the RBJ peaking transfer function.
// Production uses transposed direct form II; no production preparation/filter
// routines are used here. Identity is explicit to preserve bypass exactly.
#[derive(Clone, Copy, Default)]
struct ReferenceBell {
    b: [f64; 3],
    a: [f64; 2],
    x: [f64; 2],
    y: [f64; 2],
    identity: bool,
}
impl ReferenceBell {
    fn new(hz: i32, gain: i32, q: i32, bypass: bool) -> Self {
        if bypass || gain == 0 {
            return Self {
                identity: true,
                ..Self::default()
            };
        }
        let amplitude = 10_f64.powf(f64::from(gain) / 40000.);
        let angle = std::f64::consts::TAU * f64::from(hz) / 48000.;
        let bandwidth = angle.sin() * 500. / f64::from(q);
        let denominator = 1. + bandwidth / amplitude;
        Self {
            b: [
                (1. + bandwidth * amplitude) / denominator,
                -2. * angle.cos() / denominator,
                (1. - bandwidth * amplitude) / denominator,
            ],
            a: [
                -2. * angle.cos() / denominator,
                (1. - bandwidth / amplitude) / denominator,
            ],
            ..Self::default()
        }
    }
    fn tick(&mut self, input: f64) -> f64 {
        if self.identity {
            return input;
        }
        let output = self.b[0] * input + self.b[1] * self.x[0] + self.b[2] * self.x[1]
            - self.a[0] * self.y[0]
            - self.a[1] * self.y[1];
        self.x = [input, self.x[0]];
        self.y = [output, self.y[0]];
        output
    }
}
fn bands(c: StripConfig) -> [(i32, i32, i32, bool); 4] {
    [
        (
            c.band1_hz,
            c.band1_gain_mdb,
            c.band1_q_milli,
            c.band1_bypass,
        ),
        (
            c.band2_hz,
            c.band2_gain_mdb,
            c.band2_q_milli,
            c.band2_bypass,
        ),
        (
            c.band3_hz,
            c.band3_gain_mdb,
            c.band3_q_milli,
            c.band3_bypass,
        ),
        (
            c.band4_hz,
            c.band4_gain_mdb,
            c.band4_q_milli,
            c.band4_bypass,
        ),
    ]
}
#[derive(Clone, Copy)]
struct ReferenceEq {
    filters: [ReferenceBell; 4],
    config: StripConfig,
}
impl ReferenceEq {
    fn new(config: StripConfig) -> Self {
        Self {
            filters: bands(config).map(|(hz, gain, q, bypass)| {
                ReferenceBell::new(hz, gain, q, bypass || config.eq_bypass)
            }),
            config,
        }
    }
    fn tick(&mut self, mut input: f64) -> f64 {
        for filter in &mut self.filters {
            input = filter.tick(input);
        }
        input
    }
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct DriverEdit {
    label: String,
    channel: usize,
    config: StripConfig,
    effective_frame: String,
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
    positive_gr: u64,
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
            if state.gain_reduction_mdb.is_some_and(|v| v > 0) {
                self.positive_gr += 1;
            }
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
        positive_gr: 0,
        baseline_ticks: 0,
        deadline: Instant::now() + Duration::from_secs(90),
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
    let deadline = Instant::now() + Duration::from_secs(60);
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
    let edits: Vec<DriverEdit> = serde_json::from_value(driver_evidence["edits"].clone())
        .expect("ordered atomic driver edits with independently observed effective frames");
    assert!(edits.len() >= 17);
    assert_eq!(
        h.edits.len(),
        edits.len(),
        "no omitted, duplicate or replayed transaction"
    );
    for ((channel, boundary, config), edit) in h.edits.iter().zip(&edits) {
        assert!(!edit.label.is_empty());
        assert_eq!(*channel + 1, edit.channel, "{} channel", edit.label);
        assert_eq!(
            *boundary,
            edit.effective_frame.parse::<u64>().unwrap(),
            "{} boundary",
            edit.label
        );
        assert_eq!(
            *config, edit.config,
            "{} complete atomic config",
            edit.label
        );
    }
    // Require the actual action sequence to exercise every independent control,
    // single-band operation, an unsorted cascade and both bypass paths.
    for band in 0..4 {
        let singles: Vec<_> = edits
            .iter()
            .filter(|e| {
                let b = bands(e.config);
                e.channel == 1
                    && !e.config.eq_bypass
                    && e.config.compressor_bypass
                    && b[band].1 != 0
                    && !b[band].3
                    && b.iter()
                        .enumerate()
                        .all(|(i, v)| i == band || v.1 == 0 || v.3)
            })
            .collect();
        assert!(
            singles.len() >= 2,
            "band {} two single-band edits",
            band + 1
        );
        assert!(
            singles.iter().any(|a| singles.iter().any(|b| {
                let a = bands(a.config)[band];
                let b = bands(b.config)[band];
                a.0 != b.0 && a.1 != b.1 && a.2 != b.2
            })),
            "band {} frequency/gain/Q independently exercised",
            band + 1
        );
    }
    assert!(
        edits.iter().any(|e| {
            let b = bands(e.config);
            !e.config.eq_bypass
                && e.config.compressor_bypass
                && b.iter().all(|v| v.1 != 0 && !v.3)
                && b.windows(2).any(|pair| pair[0].0 > pair[1].0)
        }),
        "combined crossed-frequency cascade"
    );
    for band in 0..4 {
        assert!(
            edits.iter().any(|e| {
                let b = bands(e.config);
                !e.config.eq_bypass
                    && e.config.compressor_bypass
                    && b.iter().all(|v| v.1 != 0)
                    && b.iter().enumerate().all(|(i, v)| v.3 == (i == band))
            }),
            "band {} individual bypass with other bands active",
            band + 1
        );
    }
    assert!(
        edits
            .iter()
            .any(|e| e.config.eq_bypass && bands(e.config).iter().any(|b| b.1 != 0)),
        "global bypass"
    );
    assert!(
        edits
            .iter()
            .any(|e| !e.config.eq_bypass && bands(e.config).iter().all(|b| b.1 == 0)),
        "neutral enabled EQ"
    );
    assert!(
        edits
            .iter()
            .any(|e| e.channel == 2 && bands(e.config).iter().any(|b| b.1 != 0 && !b.3)),
        "second-channel active processing"
    );
    assert!(h.positive_gr > 0, "actual provider compressor positive GR");

    // Established default strip level is -6 dB with equal-power centered pan.
    // Preserve the signal-order multiplications for exact identity checks.
    let default_foh_gain = 10_f64.powf(-6. / 20.);
    let mut independent = [ReferenceEq::new(StripConfig::default()); 8];
    let mut previous = independent;
    let mut began = [0_u64; 8];
    let mut independent_samples = 0_u64;
    let mut independent_max_error = 0_f64;
    let mut checked_by_edit = vec![0_u64; edits.len()];
    let mut changed_by_edit = vec![0_u64; edits.len()];
    for (block, output) in h.captured.iter().enumerate() {
        for (offset, actual) in output.iter().enumerate() {
            let frame = block as u64 * 48 + offset as u64;
            for (ch, boundary, config) in &h.edits {
                if frame == *boundary {
                    previous[*ch] = independent[*ch];
                    independent[*ch] = ReferenceEq::new(*config);
                    began[*ch] = frame;
                }
            }
            let mut expected = 0.;
            let mut comparable = true;
            for ch in 0..8 {
                let raw = f64::from(pcm(frame, ch)) / 8388608.;
                let target = independent[ch].tick(raw);
                let age = frame - began[ch];
                let value = if age < 240 {
                    comparable &= previous[ch].config.compressor_bypass;
                    let old = previous[ch].tick(raw);
                    old + (target - old) * age as f64 / 240.
                } else {
                    target
                };
                comparable &= independent[ch].config.compressor_bypass;
                expected += value * default_foh_gain * std::f64::consts::FRAC_1_SQRT_2;
            }
            if comparable {
                for bus in &actual[..2] {
                    let error = (bus - expected).abs();
                    independent_max_error = independent_max_error.max(error);
                    assert!(
                        error < 2e-10,
                        "independent DF-I bell cascade frame {frame}: {bus} != {expected}, error {error}"
                    );
                }
                independent_samples += 1;
                if let Some((index, (_, at, _))) = h
                    .edits
                    .iter()
                    .enumerate()
                    .rev()
                    .find(|(_, (_, at, _))| frame >= *at)
                    && frame >= *at + 240
                {
                    checked_by_edit[index] += 1;
                    let neutral: f64 = (0..8)
                        .map(|ch| {
                            f64::from(pcm(frame, ch)) / 8388608.
                                * default_foh_gain
                                * std::f64::consts::FRAC_1_SQRT_2
                        })
                        .fold(0., |sum, contribution| sum + contribution);
                    if (actual[0] - neutral).abs() > 1e-8 {
                        changed_by_edit[index] += 1;
                    }
                    if independent.iter().all(|r| {
                        r.config.eq_bypass || bands(r.config).iter().all(|b| b.1 == 0 || b.3)
                    }) {
                        for bus in &actual[..2] {
                            assert_eq!(
                                bus.to_bits(),
                                neutral.to_bits(),
                                "neutral/global bypass exact frame {frame}"
                            );
                        }
                    }
                }
            }
        }
    }
    for (i, edit) in edits
        .iter()
        .enumerate()
        .filter(|(_, e)| e.channel == 1 && e.config.compressor_bypass)
    {
        assert!(
            checked_by_edit[i] >= 48,
            "{} requires observed settled independently referenced samples",
            edit.label
        );
        if !edit.config.eq_bypass && bands(edit.config).iter().any(|b| b.1 != 0 && !b.3) {
            assert!(
                changed_by_edit[i] >= 24,
                "{} must produce nontrivial actual changed samples",
                edit.label
            );
        }
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
    let result = serde_json::json!({"driver":driver_evidence,"driver_start_frame":driver_start_frame,"driver_end_frame":driver_end_frame,"processing_boundaries":h.edits.iter().map(|(ch, frame, _)| serde_json::json!({"channel":ch+1,"frame":frame})).collect::<Vec<_>>(),"channel_mapping_comparison":"eight isolated slot-zero mixers, exact full FOH timeline","independent_reference":"RBJ transfer function, direct form I, full transition timeline when compressor bypassed","independent_samples":independent_samples,"independent_max_error":independent_max_error,"independent_samples_by_edit":checked_by_edit,"changed_samples_by_edit":changed_by_edit,"positive_gr_observations":h.positive_gr,"ticks_checked":h.ticks,"neutral_ticks":h.baseline_ticks,"changed_foh_ticks":h.changed,"analysis_windows":h.windows,"raw_stems":8,"raw_frames_per_stem":count,"first_source_frame":first,"recording":recording,"monitor_comparison":"bit exact every tick","module_comparison":"independent actual FX wet+dry -> PA every tick"});
    if let Ok(path) = std::env::var("GP07_ACCEPTANCE_EVIDENCE") {
        std::fs::write(path, serde_json::to_vec_pretty(&result).unwrap()).unwrap();
    }
    drop(c);
    drop(h);
    drop(dir);
}
