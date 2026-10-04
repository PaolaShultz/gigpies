//! Explicit-path fixed-owner offline graph. Control preparation/query/lifecycle
//! is separate from bounded processing; no device enumeration or host changes.
use crate::{
    host::adapters::{
        Dsp, FxCapabilities, FxStatus, PaDescriptor, PaStatus, Recorder, RecorderObserver,
        RecorderProgress,
    },
    show::Counter,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
type Result<T> = std::result::Result<T, String>;
pub const FRAMES: usize = 48;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    pub library: PathBuf,
    pub library_sha256: String,
    pub header: PathBuf,
    pub header_sha256: String,
    pub owner_manifest: PathBuf,
    pub owner_manifest_sha256: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub version: u32,
    pub rec: Artifact,
    pub fx: Artifact,
    pub pa: Artifact,
}
fn hash(path: &Path, maximum: u64) -> Result<String> {
    use std::io::Read;
    if !path.is_absolute() {
        return Err("artifact requires absolute path".into());
    }
    let mut file = fs::File::open(path).map_err(|e| e.to_string())?;
    if !file.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("artifact must be a regular file".into());
    }
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];
    let mut total = 0u64;
    loop {
        let n = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        total += n as u64;
        if total > maximum {
            return Err("artifact size bound".into());
        }
        hasher.update(&buffer[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}
impl Artifact {
    fn verify(&self) -> Result<()> {
        for (path, expected, max) in [
            (&self.library, &self.library_sha256, 64 * 1024 * 1024),
            (&self.header, &self.header_sha256, 65536),
            (&self.owner_manifest, &self.owner_manifest_sha256, 65536),
        ] {
            if expected.len() != 64
                || !expected
                    .bytes()
                    .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
                || hash(path, max)? != *expected
            {
                return Err("artifact hash mismatch".into());
            }
        }
        // Exact accepted owner manifest must itself bind both ABI and executable.
        let bytes = fs::read(&self.owner_manifest).map_err(|e| e.to_string())?;
        let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        let files = value
            .get("files")
            .and_then(|v| v.as_object())
            .ok_or("owner manifest files")?;
        for expected in [&self.library_sha256, &self.header_sha256] {
            if !files.values().any(|v| v.as_str() == Some(expected)) {
                return Err("artifact absent from owner manifest".into());
            }
        }
        Ok(())
    }
}
impl Manifest {
    pub fn load(path: &Path) -> Result<Self> {
        let metadata = fs::metadata(path).map_err(|e| e.to_string())?;
        if !metadata.is_file() || metadata.len() > 65536 {
            return Err("manifest capacity".into());
        }
        let m: Self = serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        m.verify()?;
        Ok(m)
    }
    pub fn verify(&self) -> Result<()> {
        if self.version != 1 {
            return Err("module manifest version".into());
        }
        self.rec.verify()?;
        self.fx.verify()?;
        self.pa.verify()
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecordingStatus {
    pub take_id: String,
    pub operation_id: Counter,
    pub source_epoch: Counter,
    pub first_source_frame: Option<Counter>,
    pub mapping: [String; 8],
    pub state: String,
    pub outcome: String,
    pub accepted_frames: Counter,
    pub written_frames: Counter,
    pub durable_frames: Option<Counter>,
    pub dropped_frames: Counter,
    pub overflow_blocks: Counter,
    pub gap_frames: Counter,
    pub invalid_blocks: Counter,
    pub clipped_samples: Counter,
    pub writer_fault: bool,
    pub host_fault: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Configuration {
    pub sample_rate: u32,
    pub block_frames: u32,
    pub raw_inputs: [String; 8],
    pub fx_mix: String,
    pub pa_inputs: u32,
    pub pa_outputs: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FxHealth {
    pub capabilities: FxCapabilities,
    pub status: FxStatus,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaHealth {
    pub descriptor: PaDescriptor,
    pub status: PaStatus,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LibraryIdentity {
    pub sha256: String,
    pub state: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModuleStatus {
    pub contract: String,
    pub version: u32,
    pub show_id: String,
    pub module: String,
    pub epoch: Counter,
    pub frame: Counter,
    pub available: bool,
    pub physical: String,
    pub protection: String,
    pub readiness: String,
    pub activity: String,
    pub source_fault: u32,
    pub configuration: Option<Configuration>,
    pub libraries: BTreeMap<String, LibraryIdentity>,
    pub fx: Option<FxHealth>,
    pub pa: Option<PaHealth>,
    pub recording: Option<RecordingStatus>,
}
impl ModuleStatus {
    pub fn unavailable(show: &str, epoch: u64, frame: u64) -> Self {
        Self {
            contract: "GP05-modules".into(),
            version: 1,
            show_id: show.into(),
            module: "audio".into(),
            epoch: Counter(epoch),
            frame: Counter(frame),
            available: false,
            physical: "unverified".into(),
            protection: "offline-unprotected".into(),
            readiness: "unavailable".into(),
            activity: "idle".into(),
            source_fault: 0,
            configuration: None,
            libraries: BTreeMap::new(),
            fx: None,
            pa: None,
            recording: None,
        }
    }
}
struct Take {
    recorder: Option<Recorder>,
    observer: Option<RecorderObserver>,
    status: RecordingStatus,
}
enum WorkerResult {
    Prepared(std::result::Result<(Recorder, RecorderObserver), String>),
    Finished(i32),
}
struct Lifecycle {
    receiver: std::sync::mpsc::Receiver<WorkerResult>,
    thread: std::thread::JoinHandle<()>,
    cancel_code: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessError {
    Timeline,
    Source,
    Fx(i32),
    Pa(i32),
}
pub struct ModuleGraph {
    manifest: Manifest,
    fx: Dsp,
    pa: Dsp,
    take: Option<Take>,
    lifecycle: Option<Lifecycle>,
    processed: u64,
    source_fault: u32,
    epoch: u64,
    next: u64,
    wet: [f64; FRAMES * 2],
    sum: [f64; FRAMES * 2],
    raw: [f64; FRAMES * 8],
    output: [f64; FRAMES * 6],
    last_poll: Option<u64>,
}
impl ModuleGraph {
    pub fn load(manifest: Manifest, epoch: u64, frame: u64) -> Result<Self> {
        manifest.verify()?;
        if epoch == 0 || !frame.is_multiple_of(FRAMES as u64) {
            return Err("graph timeline".into());
        }
        let fx = Dsp::load(&manifest.fx.library, "fx", FRAMES as u32).map_err(|e| e.to_string())?;
        let pa = Dsp::load(&manifest.pa.library, "pa", FRAMES as u32).map_err(|e| e.to_string())?;
        // Real owner queries establish availability before any capability is exposed.
        let fc = fx.fx_capabilities().map_err(|e| e.to_string())?;
        let pd = pa.pa_descriptor().map_err(|e| e.to_string())?;
        if fc.channels != 2
            || fc.writable_parameters != 0
            || pd.input_channels != 2
            || pd.logical_outputs != 6
        {
            return Err("unsupported owner descriptor".into());
        }
        Ok(Self {
            manifest,
            fx,
            pa,
            take: None,
            lifecycle: None,
            processed: 0,
            source_fault: 0,
            epoch,
            next: frame,
            wet: [0.; FRAMES * 2],
            sum: [0.; FRAMES * 2],
            raw: [0.; FRAMES * 8],
            output: [0.; FRAMES * 6],
            last_poll: None,
        })
    }
    /// Off callback, whole-boundary quiescence. Recreates PA and resets FX. An
    /// active take remains under its original epoch and is marked incomplete.
    pub fn discontinuity(&mut self, epoch: u64, frame: u64) -> Result<()> {
        if epoch == 0 || !frame.is_multiple_of(48) {
            return Err("graph timeline".into());
        }
        let pa = Dsp::load(&self.manifest.pa.library, "pa", 48).map_err(|e| e.to_string())?;
        self.fx.reset();
        self.pa = pa;
        if let Some(t) = &mut self.take
            && (self.lifecycle.is_some() || t.recorder.is_some())
        {
            t.status.state = "finalizing".into();
            if let Some(worker) = &mut self.lifecycle {
                if worker.cancel_code == 0 {
                    worker.cancel_code = 4;
                }
            } else if let Some(mut r) = t.recorder.take() {
                r.fault(4);
                self.finish_worker(r)?;
            }
        }
        self.epoch = epoch;
        self.next = frame;
        self.source_fault = 0;
        self.output.fill(0.);
        Ok(())
    }
    /// No allocation, locks, I/O or lifecycle/query calls. The same unprocessed
    /// eight-input block feeds recording and GP04; FOH alone feeds wet+dry→PA.
    pub fn process(
        &mut self,
        epoch: u64,
        frame: u64,
        raw: &[[f64; 8]; FRAMES],
        mixed: &[[f64; 4]; FRAMES],
    ) -> std::result::Result<(), ProcessError> {
        self.output.fill(0.);
        if epoch != self.epoch || frame != self.next || frame.checked_add(48).is_none() {
            self.mark_source_fault(6);
            return Err(ProcessError::Timeline);
        }
        if raw.iter().flatten().any(|v| {
            !v.is_finite()
                || *v < -1.
                || *v > 8_388_607. / 8_388_608.
                || (v * 8_388_608.).fract() != 0.
        }) {
            self.mark_source_fault(7);
            return Err(ProcessError::Source);
        }
        for i in 0..FRAMES {
            self.raw[i * 8..i * 8 + 8].copy_from_slice(&raw[i]);
            self.sum[i * 2..i * 2 + 2].copy_from_slice(&mixed[i][..2]);
        }
        if let Some(t) = &mut self.take
            && t.status.state == "recording"
            && let Some(r) = &mut t.recorder
        {
            let _ = r.push(frame, &self.raw);
        }
        let fx = self.fx.process_result(&self.sum, &mut self.wet, 2);
        if fx != 0 {
            self.wet.fill(0.);
        }
        for i in 0..FRAMES * 2 {
            self.sum[i] += self.wet[i];
        }
        let pa = self.pa.process_result(&self.sum, &mut self.output, 6);
        if pa != 0 {
            self.output.fill(0.);
        }
        self.next += 48;
        self.processed = self.processed.saturating_add(1);
        if pa != 0 {
            Err(ProcessError::Pa(pa))
        } else if fx != 0 {
            Err(ProcessError::Fx(fx))
        } else {
            Ok(())
        }
    }
    fn mark_source_fault(&mut self, code: u32) {
        if self.source_fault == 0 {
            self.source_fault = code;
        }
        if let Some(t) = &mut self.take
            && let Some(r) = &mut t.recorder
        {
            r.fault(code);
        }
        if let Some(worker) = &mut self.lifecycle
            && worker.cancel_code == 0
        {
            worker.cancel_code = code;
        }
    }
    pub fn output(&self) -> &[f64; FRAMES * 6] {
        &self.output
    }
    /// Accepts preparation only. One bounded worker creates the handle and
    /// observer. No physical endpoints and no client-chosen recording paths.
    pub fn start(&mut self, root: &Path, take_id: &str, operation_id: Counter) -> Result<()> {
        if !crate::show::id(take_id) || operation_id.0 == 0 {
            return Err("record identity".into());
        }
        if self.source_fault != 0 {
            return Err("source discontinuity requires explicit recreation".into());
        }
        if self.lifecycle.is_some() || self.take.as_ref().is_some_and(|t| t.recorder.is_some()) {
            return Err("recorder busy".into());
        }
        let path = root.join(take_id);
        let library = self.manifest.rec.library.clone();
        let epoch = self.epoch;
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        let thread = std::thread::Builder::new()
            .name("gp05-rec-prepare".into())
            .spawn(move || {
                let prepared = (|| {
                    let mut r =
                        Recorder::create(&library, &path, 48, epoch).map_err(|e| e.to_string())?;
                    let observer = r.observer().map_err(|e| e.to_string())?;
                    Ok((r, observer))
                })();
                let _ = sender.send(WorkerResult::Prepared(prepared));
            })
            .map_err(|e| e.to_string())?;
        let status = RecordingStatus {
            take_id: take_id.into(),
            operation_id,
            source_epoch: Counter(self.epoch),
            first_source_frame: None,
            mapping: std::array::from_fn(|i| format!("input-{:02}/raw", i + 1)),
            state: "preparing".into(),
            outcome: "pending".into(),
            accepted_frames: Counter(0),
            written_frames: Counter(0),
            durable_frames: None,
            dropped_frames: Counter(0),
            overflow_blocks: Counter(0),
            gap_frames: Counter(0),
            invalid_blocks: Counter(0),
            clipped_samples: Counter(0),
            writer_fault: false,
            host_fault: 0,
        };
        self.take = Some(Take {
            recorder: None,
            observer: None,
            status,
        });
        self.lifecycle = Some(Lifecycle {
            receiver,
            thread,
            cancel_code: 0,
        });
        self.last_poll = None;
        Ok(())
    }
    /// Quiesces pushes immediately at this control-side whole-frame boundary.
    /// Consuming finish/drain/sync happens exclusively on a bounded worker.
    pub fn stop(&mut self, take_id: &str, operation_id: Counter) -> Result<()> {
        let t = self.take.as_mut().ok_or("no take")?;
        if t.status.take_id != take_id || t.status.operation_id != operation_id {
            return Err("retired record operation".into());
        }
        if let Some(worker) = &mut self.lifecycle {
            if t.status.state != "preparing" {
                return Err("already finalizing".into());
            }
            if worker.cancel_code == 0 {
                worker.cancel_code = 5;
            }
            t.status.state = "finalizing".into();
            return Ok(());
        }
        let r = t.recorder.take().ok_or("already finalized")?;
        t.status.state = "finalizing".into();
        self.finish_worker(r)
    }
    fn finish_worker(&mut self, mut recorder: Recorder) -> Result<()> {
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        let (give, take) = std::sync::mpsc::sync_channel::<Recorder>(1);
        // Spawn before handing over ownership: a spawn refusal must not consume
        // and synchronously finalize the recorder on the control pump.
        let thread = match std::thread::Builder::new()
            .name("gp05-rec-finish".into())
            .spawn(move || {
                if let Ok(r) = take.recv() {
                    let result = r.finish();
                    let _ = sender.send(WorkerResult::Finished(result));
                }
            }) {
            Ok(thread) => thread,
            Err(e) => {
                recorder.fault(10);
                if let Some(t) = &mut self.take {
                    t.recorder = Some(recorder);
                    t.status.state = "finalizing".into();
                    t.status.host_fault = 10;
                }
                return Err(e.to_string());
            }
        };
        // Capacity one, empty queue, worker owns receiver until it obtains value.
        if let Err(e) = give.send(recorder) {
            let mut r = e.0;
            r.fault(10);
            if let Some(t) = &mut self.take {
                t.recorder = Some(r);
                t.status.host_fault = 10;
            }
            let _ = thread.join();
            return Err("finalizer handoff failed".into());
        }
        self.lifecycle = Some(Lifecycle {
            receiver,
            thread,
            cancel_code: 0,
        });
        Ok(())
    }
    /// Nonblocking readiness/completion transfer, outside bounded processing.
    pub fn poll_lifecycle(&mut self) -> Result<()> {
        let Some(worker) = &self.lifecycle else {
            return Ok(());
        };
        if !worker.thread.is_finished() {
            return Ok(());
        }
        let result = worker.receiver.try_recv().map_err(|e| e.to_string());
        let worker = self.lifecycle.take().unwrap();
        let joined = worker.thread.join();
        let t = self.take.as_mut().ok_or("lost take")?;
        if joined.is_err() || result.is_err() {
            t.status.state = "finalized".into();
            t.status.outcome = "error".into();
            return Ok(());
        }
        match result.unwrap() {
            WorkerResult::Prepared(Ok((mut r, observer))) => {
                t.observer = Some(observer);
                if worker.cancel_code != 0 {
                    r.fault(worker.cancel_code);
                    self.finish_worker(r)?;
                } else {
                    t.status.first_source_frame = Some(Counter(self.next));
                    t.status.state = "recording".into();
                    t.recorder = Some(r);
                }
            }
            WorkerResult::Prepared(Err(_)) => {
                t.status.state = "finalized".into();
                t.status.outcome = "error".into();
            }
            WorkerResult::Finished(result) => {
                if let Some(observer) = &t.observer {
                    let progress = observer.snapshot().map_err(|e| e.to_string())?;
                    Self::update(t, progress);
                }
                if result < 0 {
                    t.status.state = "finalized".into();
                    t.status.outcome = "error".into();
                }
            }
        }
        Ok(())
    }
    fn update(t: &mut Take, p: RecorderProgress) {
        t.status.state = match p.state {
            1 => "recording",
            2 => "finalizing",
            _ => "finalized",
        }
        .into();
        t.status.outcome = match p.outcome {
            1 => "complete",
            2 => "incomplete",
            3 => "error",
            _ => "pending",
        }
        .into();
        t.status.accepted_frames = Counter(p.accepted_frames);
        t.status.written_frames = Counter(p.written_frames);
        t.status.durable_frames = if p.durable_frames == u64::MAX {
            None
        } else {
            Some(Counter(p.durable_frames))
        };
        t.status.dropped_frames = Counter(p.dropped_frames);
        t.status.overflow_blocks = Counter(p.overflow_blocks);
        t.status.gap_frames = Counter(p.gap_frames);
        t.status.invalid_blocks = Counter(p.invalid_blocks);
        t.status.clipped_samples = Counter(p.clipped_samples);
        t.status.writer_fault = p.writer_fault != 0;
        t.status.host_fault = p.host_fault;
    }
    /// Off callback <=10 Hz; finalization performs one independent terminal query.
    pub fn status(&mut self, show: &str, now: u64) -> Result<ModuleStatus> {
        if self
            .last_poll
            .is_none_or(|last| now.saturating_sub(last) >= 100)
        {
            if let Some(t) = &mut self.take
                && t.status.state != "finalizing"
                && let Some(observer) = &t.observer
            {
                let p = observer.snapshot().map_err(|e| e.to_string())?;
                Self::update(t, p);
            }
            self.last_poll = Some(now);
        }
        let mut status = ModuleStatus::unavailable(show, self.epoch, self.next);
        status.available = true;
        status.protection = "owner-sample-limiter-only".into();
        status.configuration = Some(Configuration {
            sample_rate: 48000,
            block_frames: 48,
            raw_inputs: std::array::from_fn(|i| format!("input-{:02}/raw", i + 1)),
            fx_mix: "dry-plus-fixed-wet".into(),
            pa_inputs: 2,
            pa_outputs: 6,
        });
        for (owner, a) in [
            ("rec", &self.manifest.rec),
            ("fx", &self.manifest.fx),
            ("pa", &self.manifest.pa),
        ] {
            status.libraries.insert(
                owner.into(),
                LibraryIdentity {
                    sha256: a.library_sha256.clone(),
                    state: if owner == "rec"
                        && !self.take.as_ref().is_some_and(|t| t.observer.is_some())
                    {
                        "verified"
                    } else {
                        "loaded"
                    }
                    .into(),
                },
            );
        }
        status.fx = Some(FxHealth {
            capabilities: self.fx.fx_capabilities().map_err(|e| e.to_string())?,
            status: self.fx.fx_status().map_err(|e| e.to_string())?,
        });
        status.pa = Some(PaHealth {
            descriptor: self.pa.pa_descriptor().map_err(|e| e.to_string())?,
            status: self.pa.pa_status().map_err(|e| e.to_string())?,
        });
        status.source_fault = self.source_fault;
        status.readiness = if status
            .pa
            .as_ref()
            .is_some_and(|p| p.status.fault_latched != 0)
        {
            "faulted"
        } else if self.source_fault != 0
            || status
                .fx
                .as_ref()
                .is_some_and(|f| f.status.last_process_result != 0)
        {
            "degraded"
        } else {
            "ready"
        }
        .into();
        status.activity = if self.processed == 0 {
            "idle"
        } else {
            "processed"
        }
        .into();
        status.recording = self.take.as_ref().map(|t| t.status.clone());
        Ok(status)
    }
}

impl Drop for ModuleGraph {
    fn drop(&mut self) {
        // Explicit control-side shutdown joins only this owned lifecycle worker.
        // A late prepared handle is faulted/finalized by Recorder::drop, never
        // reactivated. All observers/libraries remain retained through cleanup.
        if let Some(worker) = self.lifecycle.take() {
            let _ = worker.thread.join();
            if let Ok(result) = worker.receiver.try_recv() {
                drop(result);
            }
        }
        if let Some(t) = &mut self.take
            && let Some(mut r) = t.recorder.take()
        {
            r.fault(1);
            let _ = r.finish();
        }
    }
}
