//! Explicit opt-in private Unix stream binding for the offline synthetic graph.
//! One control owner; no device, TCP, service installation or realtime deadline.
use crate::{
    control_model::{Command, Request},
    mixer_control::{OfflineEngine, RenderedReply},
    show::{Counter, Result},
};
use std::{
    collections::{BTreeMap, VecDeque},
    fs,
    io::{ErrorKind, Read, Write},
    os::{
        fd::AsRawFd,
        unix::{
            fs::{FileTypeExt, MetadataExt, PermissionsExt},
            net::{UnixListener, UnixStream},
        },
    },
    path::{Path, PathBuf},
};
pub const MAX_CLIENTS: usize = 4;
pub const MAX_FRAME: usize = 65536;
pub const MAX_REPLIES: usize = 32;
const IO_BUDGET: usize = 8192;
const TIMEOUT_MS: u64 = 2000;
struct Endpoint {
    path: PathBuf,
    dev: u64,
    ino: u64,
}
impl Drop for Endpoint {
    fn drop(&mut self) {
        if let Ok(m) = fs::symlink_metadata(&self.path)
            && m.file_type().is_socket()
            && m.dev() == self.dev
            && m.ino() == self.ino
        {
            let _ = fs::remove_file(&self.path);
        }
    }
}
struct Packet {
    bytes: Vec<u8>,
    offset: usize,
}
impl Packet {
    fn paged(bytes: Vec<u8>) -> Result<Self> {
        let pages = crate::snapshot_pages::encode(bytes)?;
        let size = pages
            .iter()
            .try_fold(0usize, |sum, p| sum.checked_add(p.len() + 4))
            .ok_or("paged reply overflow")?;
        let mut bytes = Vec::with_capacity(size);
        for page in pages {
            bytes.extend_from_slice(&(page.len() as u32).to_be_bytes());
            bytes.extend_from_slice(&page);
        }
        Ok(Self { bytes, offset: 0 })
    }

    fn new(bytes: Vec<u8>) -> Result<Self> {
        if bytes.is_empty() || bytes.len() > MAX_FRAME {
            return Err("frame capacity".into());
        }
        let mut framed = Vec::with_capacity(bytes.len() + 4);
        framed.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
        framed.extend_from_slice(&bytes);
        Ok(Self {
            bytes: framed,
            offset: 0,
        })
    }
}
enum Incoming {
    Audio(Request),
    Structural(crate::structural_control::Request),
    Modules(crate::module_wire::ModuleRequest),
    Processing(crate::processing_wire::ProcessingRequest),
    UnsupportedProcessing(crate::processing_wire::UnsupportedRequest),
}
struct Client {
    id: u64,
    stream: UnixStream,
    buffer: Box<[u8; MAX_FRAME + 4]>,
    used: usize,
    started: Option<u64>,
    replies: VecDeque<Packet>,
    telemetry: Option<Packet>,
    last_write: u64,
    snapshot: bool,
    processing_snapshot_ms: Option<u64>,
    structural_snapshot_ms: Option<u64>,
    writer: Option<String>,
    lease: Option<Counter>,
}
impl Client {
    fn queue(&mut self, reply: &RenderedReply, now: u64) -> Result<()> {
        if self.replies.len() >= MAX_REPLIES {
            return Err("reply capacity".into());
        }
        if self.replies.is_empty() && self.telemetry.is_none() {
            self.last_write = now;
        }
        reply.validate()?;
        self.replies.push_back(Packet::paged(
            serde_json::to_vec(reply).map_err(|e| e.to_string())?,
        )?);
        Ok(())
    }
    fn receive(&mut self, now: u64, processing_version: u32) -> Result<Option<Incoming>> {
        if self
            .started
            .is_some_and(|t| now.saturating_sub(t) >= TIMEOUT_MS)
        {
            return Err("frame timeout".into());
        }
        let count = (self.buffer.len() - self.used).min(IO_BUDGET);
        if count > 0 {
            match self
                .stream
                .read(&mut self.buffer[self.used..self.used + count])
            {
                Ok(0) => return Err("disconnected".into()),
                Ok(n) => {
                    if self.used == 0 {
                        self.started = Some(now);
                    }
                    self.used += n;
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => (),
                Err(e) if e.kind() == ErrorKind::Interrupted => (),
                Err(e) => return Err(e.to_string()),
            }
        }
        if self.used < 4 {
            return Ok(None);
        }
        let length = u32::from_be_bytes(self.buffer[..4].try_into().unwrap()) as usize;
        if length == 0 || length > MAX_FRAME {
            return Err("frame length".into());
        }
        if self.used < length + 4 {
            return Ok(None);
        }
        let bytes = &self.buffer[4..length + 4];
        let value: serde_json::Value = crate::show::decode(bytes)?;
        let request = if value.get("contract").and_then(|v| v.as_str()) == Some("GP14-structure") {
            Incoming::Structural(crate::structural_control::Request::decode(bytes)?)
        } else if value.get("contract").and_then(|v| v.as_str()) == Some("GP05-modules") {
            Incoming::Modules(crate::module_wire::ModuleRequest::decode(bytes)?)
        } else if value.get("contract").and_then(|v| v.as_str()) == Some("GP07-processing") {
            if let Some(request) =
                crate::processing_wire::UnsupportedRequest::decode_for(bytes, processing_version)?
            {
                Incoming::UnsupportedProcessing(request)
            } else {
                Incoming::Processing(crate::processing_wire::ProcessingRequest::decode(bytes)?)
            }
        } else {
            Incoming::Audio(Request::decode(bytes)?)
        };
        self.buffer.copy_within(length + 4..self.used, 0);
        self.used -= length + 4;
        if self.used == 0 {
            self.started = None;
        }
        Ok(Some(request))
    }
    fn queue_module(&mut self, value: &impl serde::Serialize, now: u64) -> Result<()> {
        if self.replies.len() >= MAX_REPLIES {
            return Err("reply capacity".into());
        }
        if self.replies.is_empty() && self.telemetry.is_none() {
            self.last_write = now;
        }
        self.replies.push_back(Packet::paged(
            serde_json::to_vec(value).map_err(|e| e.to_string())?,
        )?);
        Ok(())
    }
    fn flush(&mut self, now: u64) -> Result<()> {
        // A partly sent telemetry frame cannot be interleaved with a control frame.
        let partial = self.telemetry.as_ref().is_some_and(|p| p.offset > 0);
        let packet = if partial {
            self.telemetry.as_mut()
        } else {
            self.replies.front_mut().or(self.telemetry.as_mut())
        };
        let Some(packet) = packet else {
            return Ok(());
        };
        if now.saturating_sub(self.last_write) >= TIMEOUT_MS {
            return Err("stalled reader".into());
        }
        let end = (packet.offset + IO_BUDGET).min(packet.bytes.len());
        match self.stream.write(&packet.bytes[packet.offset..end]) {
            Ok(0) => return Err("disconnected".into()),
            Ok(n) => {
                packet.offset += n;
                self.last_write = now;
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::Interrupted => (),
            Err(e) => return Err(e.to_string()),
        }
        let done = packet.offset == packet.bytes.len();
        if done {
            if partial || self.replies.is_empty() {
                self.telemetry = None;
            } else {
                self.replies.pop_front();
            }
        }
        Ok(())
    }
}
/// Same effective UID only; no hardware-host feature or device opens.
fn same_uid(stream: &UnixStream) -> Result<bool> {
    let mut credential = libc::ucred {
        pid: 0,
        uid: 0,
        gid: 0,
    };
    let mut length = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    // SAFETY: valid connected socket and correctly sized writable ucred/length.
    let rc = unsafe {
        libc::getsockopt(
            stream.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            (&mut credential as *mut libc::ucred).cast(),
            &mut length,
        )
    };
    if rc != 0 || length as usize != std::mem::size_of::<libc::ucred>() {
        return Err("peer credential".into());
    }
    Ok(credential.uid == unsafe { libc::geteuid() })
}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct EpochRecord {
    version: u32,
    show: String,
    high_epoch: Counter,
}
struct EpochOwner {
    _file: fs::File,
    directory: PathBuf,
    show: String,
    epoch: Counter,
}
impl EpochOwner {
    fn advance(&mut self, epoch: Counter) -> Result<()> {
        use std::os::unix::fs::OpenOptionsExt;
        if epoch.0 <= self.epoch.0 {
            return Err("fresh durable epoch required".into());
        }
        let bytes = serde_json::to_vec(&EpochRecord {
            version: 1,
            show: self.show.clone(),
            high_epoch: epoch,
        })
        .map_err(|e| e.to_string())?;
        let path = self.directory.join("audio.identity");
        let temp = self
            .directory
            .join(format!("audio.identity.recovery-{}", std::process::id()));
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&temp)
            .map_err(|e| e.to_string())?;
        let result = (|| {
            file.write_all(&bytes).map_err(|e| e.to_string())?;
            file.sync_all().map_err(|e| e.to_string())?;
            fs::rename(&temp, &path).map_err(|e| e.to_string())?;
            fs::File::open(&self.directory)
                .and_then(|d| d.sync_all())
                .map_err(|e| e.to_string())?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temp);
        } else {
            self.epoch = epoch;
        }
        result
    }
    fn reserve(directory: &Path, show: &str, epoch: Counter) -> Result<Self> {
        use std::os::unix::fs::OpenOptionsExt;
        let lockpath = directory.join("audio.owner");
        // A persistent owner without its registry means reservation was interrupted or
        // identity metadata was lost. Never reinterpret it as a new authority.
        let options = || {
            let mut o = fs::OpenOptions::new();
            o.read(true)
                .write(true)
                .mode(0o600)
                .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
            o
        };
        let (lock, fresh_owner) = match options().create_new(true).open(&lockpath) {
            Ok(file) => (file, true),
            Err(e) if e.kind() == ErrorKind::AlreadyExists => {
                (options().open(&lockpath).map_err(|e| e.to_string())?, false)
            }
            Err(e) => return Err(e.to_string()),
        };
        let metadata = lock.metadata().map_err(|e| e.to_string())?;
        if !metadata.is_file()
            || metadata.uid() != unsafe { libc::geteuid() }
            || metadata.mode() & 0o777 != 0o600
            || metadata.nlink() != 1
            || metadata.len() != 0
        {
            return Err("private epoch owner".into());
        }
        // SAFETY: flock applies only to this owned regular file descriptor.
        if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err("authority already owned".into());
        }
        if fresh_owner {
            // Make the fail-closed marker durable before the first identity reservation.
            lock.sync_all().map_err(|e| e.to_string())?;
            fs::File::open(directory)
                .and_then(|d| d.sync_all())
                .map_err(|e| e.to_string())?;
        }
        let path = directory.join("audio.identity");
        match fs::symlink_metadata(&path) {
            Ok(m) => {
                if !m.is_file()
                    || m.file_type().is_symlink()
                    || m.uid() != unsafe { libc::geteuid() }
                    || m.mode() & 0o777 != 0o600
                    || m.nlink() != 1
                    || m.len() > 4096
                {
                    return Err("private epoch record".into());
                }
                let file = fs::OpenOptions::new()
                    .read(true)
                    .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
                    .open(&path)
                    .map_err(|e| e.to_string())?;
                let opened = file.metadata().map_err(|e| e.to_string())?;
                if opened.dev() != m.dev() || opened.ino() != m.ino() || opened.len() > 4096 {
                    return Err("changed epoch record".into());
                }
                let mut bytes = Vec::new();
                file.take(4097)
                    .read_to_end(&mut bytes)
                    .map_err(|e| e.to_string())?;
                let record: EpochRecord = crate::show::decode(&bytes)?;
                if record.version != 1
                    || record.high_epoch.0 == 0
                    || record.show != show
                    || epoch.0 <= record.high_epoch.0
                {
                    return Err("reused or regressed authority epoch".into());
                }
            }
            Err(e) if e.kind() == ErrorKind::NotFound && fresh_owner => (),
            Err(e) if e.kind() == ErrorKind::NotFound => {
                return Err("lost authority identity".into());
            }
            Err(e) => return Err(e.to_string()),
        }
        let bytes = serde_json::to_vec(&EpochRecord {
            version: 1,
            show: show.into(),
            high_epoch: epoch,
        })
        .map_err(|e| e.to_string())?;
        let temp = directory.join(format!("audio.identity.pending-{}", std::process::id()));
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&temp)
            .map_err(|e| e.to_string())?;
        let saved = (|| {
            file.write_all(&bytes).map_err(|e| e.to_string())?;
            file.sync_all().map_err(|e| e.to_string())?;
            fs::rename(&temp, &path).map_err(|e| e.to_string())?;
            fs::File::open(directory)
                .and_then(|d| d.sync_all())
                .map_err(|e| e.to_string())?;
            Ok::<(), String>(())
        })();
        if saved.is_err() {
            let _ = fs::remove_file(&temp);
        }
        saved?;
        Ok(Self {
            _file: lock,
            directory: directory.into(),
            show: show.into(),
            epoch,
        })
    }
}
// One bounded pending slot retains prepared state through the render boundary.
// Keep it inline so commit does not introduce an additional heap owner to retire.
#[allow(clippy::large_enum_variant)]
enum StructuralAction {
    #[cfg(feature = "hardware-host")]
    Pa(crate::module_graph::GraphPaPrepared),
    Patch(crate::mixer::PreparedOutputPatch),
    Mute,
    Rearm,
    Failed(String),
}
pub struct LocalAudio {
    listener: UnixListener,
    _epoch_owner: EpochOwner,
    _endpoint: Endpoint,
    engine: OfflineEngine,
    clients: Vec<Client>,
    next_client: u64,
    ticks: u64,
    last_now: u64,
    pending_owner: Option<(u64, u64)>,
    processing_owner: Option<(u64, u64)>,
    writer_connections: BTreeMap<String, u64>,
    show: String,
    epoch: Counter,
    synthetic_fouraux: bool,
    raw_scratch: Vec<f64>,
    bus_scratch: Vec<f64>,
    pa_silence: Vec<f64>,
    capture_scratch: Vec<f64>,
    playback_scratch: Vec<f64>,
    pcm_scratch: Vec<i32>,
    remote_completions: VecDeque<serde_json::Value>,
    structural_pending: Option<(
        crate::structural_control::Request,
        StructuralAction,
        Option<u64>,
    )>,
    structural_cache: VecDeque<(
        crate::structural_control::Request,
        crate::structural_control::Reply,
    )>,
    #[cfg(feature = "hardware-host")]
    private_directory: PathBuf,
    module_cache: VecDeque<(
        crate::module_wire::ModuleRequest,
        crate::module_wire::ModuleReply,
    )>,
    #[cfg(feature = "hardware-host")]
    modules: Option<crate::module_graph::ModuleGraph>,
    #[cfg(feature = "hardware-host")]
    module_pending: Vec<(crate::module_wire::ModuleRequest, u64)>,
    analysis: Option<(
        crate::analysis_stream::Tap,
        crate::analysis_stream::local::Worker,
    )>,
}
impl LocalAudio {
    pub fn bind(directory: &Path, name: &str, show: &str, epoch: Counter) -> Result<Self> {
        Self::bind_configured(
            directory,
            name,
            show,
            epoch,
            crate::topology::EngineTopology::legacy(),
        )
    }
    pub fn bind_configured(
        directory: &Path,
        name: &str,
        show: &str,
        epoch: Counter,
        topology: crate::topology::EngineTopology,
    ) -> Result<Self> {
        if name.is_empty()
            || name.len() > 40
            || !name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'.')
        {
            return Err("endpoint name".into());
        }
        let metadata = fs::symlink_metadata(directory).map_err(|e| e.to_string())?;
        if !directory.is_absolute()
            || !metadata.is_dir()
            || metadata.file_type().is_symlink()
            || metadata.uid() != unsafe { libc::geteuid() }
            || metadata.mode() & 0o777 != 0o700
            || fs::canonicalize(directory).map_err(|e| e.to_string())? != directory
        {
            return Err("private owned directory required".into());
        }
        let path = directory.join(name);
        match fs::symlink_metadata(&path) {
            Err(e) if e.kind() == ErrorKind::NotFound => (),
            _ => return Err("preexisting endpoint".into()),
        }
        let ni = topology.inputs.len();
        let nb = topology.monitors + 2;
        let np = topology.pa_outputs;
        let nc = topology.capture_channels;
        let no = topology.playback_channels;
        let engine = OfflineEngine::with_topology(show, epoch, Counter(0), 0, topology)?;
        let epoch_owner = EpochOwner::reserve(directory, show, epoch)?;
        let listener = UnixListener::bind(&path).map_err(|e| e.to_string())?;
        let m = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
        let endpoint = Endpoint {
            path,
            dev: m.dev(),
            ino: m.ino(),
        };
        fs::set_permissions(&endpoint.path, fs::Permissions::from_mode(0o600))
            .map_err(|e| e.to_string())?;
        listener.set_nonblocking(true).map_err(|e| e.to_string())?;
        Ok(Self {
            listener,
            _epoch_owner: epoch_owner,
            _endpoint: endpoint,
            engine,
            clients: Vec::with_capacity(MAX_CLIENTS),
            next_client: 1,
            ticks: 0,
            last_now: 0,
            pending_owner: None,
            processing_owner: None,
            writer_connections: BTreeMap::new(),
            show: show.into(),
            epoch,
            analysis: None,
            synthetic_fouraux: false,
            remote_completions: VecDeque::with_capacity(64),
            structural_pending: None,
            structural_cache: VecDeque::with_capacity(256),
            raw_scratch: vec![0.; 48 * ni],
            bus_scratch: vec![0.; 48 * nb],
            pa_silence: vec![0.; 48 * np],
            capture_scratch: vec![0.; 48 * nc],
            playback_scratch: vec![0.; 48 * no],
            pcm_scratch: vec![0; 48 * ni],
            #[cfg(feature = "hardware-host")]
            private_directory: directory.into(),
            module_cache: VecDeque::with_capacity(256),
            #[cfg(feature = "hardware-host")]
            modules: None,
            #[cfg(feature = "hardware-host")]
            module_pending: Vec::with_capacity(2),
        })
    }
    pub fn persisted_intent(&mut self) -> Result<crate::structural_control::Intent> {
        let snapshot = self.structural_snapshot()?;
        Ok(crate::structural_control::Intent {
            version: 1,
            engine: self.engine.persisted_intent()?,
            pa_configuration_json: snapshot.pa_configuration_json,
            pa_program_buses: snapshot.pa_program_buses,
        })
    }
    /// Restore only on a newly bound, unprocessed authority. Libraries have been
    /// explicitly loaded by the caller; all owner validation precedes replacement.
    pub fn restore_composed_intent(
        &mut self,
        intent: &crate::structural_control::Intent,
    ) -> Result<()> {
        intent.validate()?;
        if self.frame() != 0
            || !self.clients.is_empty()
            || self.engine.revision() != Counter(0)
            || self.show != intent.engine.show_id
        {
            return Err("fresh unattached provider required".into());
        }
        let restored =
            OfflineEngine::restore_intent_for(&intent.engine, self.topology(), self.epoch, 0)?;
        #[cfg(feature = "hardware-host")]
        if let Some(json) = &intent.pa_configuration_json {
            let capacity = self.topology().pa_outputs;
            let total = self.topology().monitors + 2;
            let g = self
                .modules
                .as_mut()
                .ok_or("persisted PA requires explicit owner library")?;
            let mut prepared =
                g.prepare_pa_change(json.as_bytes(), intent.pa_program_buses.clone(), total)?;
            if prepared.output_channels() != capacity {
                return Err("persisted PA capability mismatch".into());
            }
            let result = g.commit_pa_change(&mut prepared, self.epoch.0, 0);
            if result != 0 {
                return Err(format!("persisted PA commit {result}"));
            }
            g.retire_pa_changes();
        }
        #[cfg(not(feature = "hardware-host"))]
        if intent.pa_configuration_json.is_some() {
            return Err("persisted PA unavailable".into());
        }
        self.engine = restored;
        Ok(())
    }
    pub fn structural_snapshot(&self) -> Result<crate::structural_control::Snapshot> {
        #[cfg(feature = "hardware-host")]
        let (configuration, buses, status, capabilities) = if let Some(g) = &self.modules {
            (
                g.pa_configuration_json().map(str::to_owned),
                g.pa_program_buses().to_vec(),
                g.pa_v2_status()
                    .transpose()
                    .map_err(|e| format!("PA status {e}"))?
                    .map(serde_json::to_value)
                    .transpose()
                    .map_err(|e| e.to_string())?,
                g.pa_v2_capabilities()
                    .map(serde_json::to_value)
                    .transpose()
                    .map_err(|e| e.to_string())?,
            )
        } else {
            (None, Vec::new(), None, None)
        };
        #[cfg(not(feature = "hardware-host"))]
        let (configuration, buses, status, capabilities) = (None, Vec::new(), None, None);
        Ok(crate::structural_control::Snapshot {
            show_id: self.show.clone(),
            epoch: self.epoch,
            revision: self.engine.revision(),
            frame: Counter(self.frame()),
            topology: self.topology().clone(),
            clock: self.engine.clock_status().clone(),
            outputs_quiesced: self.engine.outputs_quiesced(),
            pa_configuration_json: configuration,
            pa_program_buses: buses,
            pa_status: status,
            pa_capabilities: capabilities,
        })
    }
    /// Control-side preparation; exact identity, scopes, retry history and final
    /// revision remain owned by the same engine authority as channel controls.
    pub fn structural_request(
        &mut self,
        r: crate::structural_control::Request,
        now: u64,
        fresh: bool,
        owner: Option<u64>,
    ) -> Result<crate::structural_control::Reply> {
        use crate::structural_control::{Command as S, Reply};
        r.validate()?;
        if r.show_id != self.show || r.epoch != self.epoch {
            return Err("structural attachment identity".into());
        }
        if matches!(r.command, S::StructuralSnapshot {}) {
            return Ok(Reply::new(
                &r,
                "snapshot",
                None,
                None,
                self.engine.revision(),
                Some(self.structural_snapshot()?),
            ));
        }
        if self.engine.external_boundary().is_none() {
            self.structural_pending = None;
        }
        let auth = r.authority_request();
        let (frame, cached) =
            match self
                .engine
                .begin_external(&auth, &r.fingerprint()?, r.scope().unwrap(), now)
            {
                Ok(v) => v,
                Err(reason) => {
                    return Ok(Reply::new(
                        &r,
                        "final",
                        Some(reason),
                        None,
                        self.engine.revision(),
                        None,
                    ));
                }
            };
        if let Some(cached) = cached {
            return Ok(self
                .structural_cache
                .iter()
                .find(|(old, _)| old == &r)
                .map(|(_, v)| v.clone())
                .unwrap_or_else(|| {
                    Reply::new(
                        &r,
                        "final",
                        cached
                            .body
                            .reason
                            .clone()
                            .or_else(|| Some("expired_outcome".into())),
                        cached.body.effective_frame.map(|f| f.0),
                        cached.body.revision,
                        None,
                    )
                }));
        }
        if let Some((old, _, _)) = &self.structural_pending {
            if old == &r {
                return Ok(Reply::new(
                    &r,
                    "pending",
                    None,
                    Some(frame),
                    self.engine.revision(),
                    None,
                ));
            }
            return Err("structural pending ownership".into());
        }
        let prepared = (|| -> Result<StructuralAction> {
            if !fresh {
                return Err("fresh_structural_snapshot_required".into());
            }
            match &r.command {
                S::OutputMute {} => Ok(StructuralAction::Mute),
                S::OutputRearm {} => Ok(StructuralAction::Rearm),
                S::OutputPatch { outputs } => {
                    if !self.engine.outputs_quiesced() {
                        return Err("outputs_must_be_quiesced".into());
                    }
                    if self.topology().mapping_evidence == "operator-verified"
                        && self.topology().pa_outputs > 0
                        && outputs.iter().any(|p| {
                            matches!(p.source, Some(crate::topology::OutputSource::Main { .. }))
                        })
                    {
                        return Err("physical main route must pass through PA protection".into());
                    }
                    let current = &self.topology().outputs;
                    if outputs.len() != current.len()
                        || outputs.iter().zip(current).any(|(a, b)| {
                            a.id != b.id
                                || a.playback_slot != b.playback_slot
                                || a.physical_port != b.physical_port
                        })
                    {
                        return Err("physical remap requires reviewed topology reopen".into());
                    }
                    Ok(StructuralAction::Patch(
                        self.engine.prepare_output_patch(outputs.clone())?,
                    ))
                }
                S::PaSet {
                    configuration_json,
                    program_buses,
                } => {
                    if !self.engine.outputs_quiesced() {
                        return Err("outputs_must_be_quiesced".into());
                    }
                    #[cfg(feature = "hardware-host")]
                    {
                        let total = self.topology().monitors + 2;
                        let capacity = self.topology().pa_outputs;
                        let g = self.modules.as_mut().ok_or("modules unavailable")?;
                        let prepared = g.prepare_pa_change(
                            configuration_json.as_bytes(),
                            program_buses.clone(),
                            total,
                        )?;
                        if prepared.output_channels() != capacity {
                            return Err("PA outputs must match admitted module ports".into());
                        }
                        Ok(StructuralAction::Pa(prepared))
                    }
                    #[cfg(not(feature = "hardware-host"))]
                    {
                        let _ = (configuration_json, program_buses);
                        Err("modules unavailable".into())
                    }
                }
                S::StructuralSnapshot {} => unreachable!(),
            }
        })();
        self.structural_pending = Some((
            r.clone(),
            prepared.unwrap_or_else(StructuralAction::Failed),
            owner,
        ));
        Ok(Reply::new(
            &r,
            "pending",
            None,
            Some(frame),
            self.engine.revision(),
            None,
        ))
    }
    fn commit_structure(&mut self, now: u64) -> Result<()> {
        if self.engine.external_boundary() != Some(self.frame()) {
            return Ok(());
        }
        let (r, mut action, owner) = self
            .structural_pending
            .take()
            .ok_or("missing structural prepared state")?;
        if !self
            .engine
            .external_matches(&r.authority_request(), &r.fingerprint()?)
        {
            self.quiesce_source("structural_identity_mismatch")?;
            return Err("structural prepared identity mismatch".into());
        }
        let frame = self.frame();
        let epoch = self.epoch.0;
        #[cfg(feature = "hardware-host")]
        let modules = &mut self.modules;
        let mut failure = None;
        let result = self.engine.commit_external_with_engine(now, |engine| {
            let applied = match &mut action {
                StructuralAction::Failed(reason) => Err(reason.clone()),
                StructuralAction::Patch(prepared) => engine.apply_output_patch(prepared),
                StructuralAction::Mute => {
                    engine.mute_outputs()?;
                    #[cfg(feature = "hardware-host")]
                    if let Some(g) = modules {
                        g.mute_pa()?;
                    }
                    Ok(())
                }
                StructuralAction::Rearm => {
                    if !matches!(
                        engine.clock_status().state,
                        crate::clock_domain::ClockState::Running
                            | crate::clock_domain::ClockState::Disarmed
                    ) {
                        return Err("clock recovery required".into());
                    }
                    #[cfg(feature = "hardware-host")]
                    if let Some(g) = modules {
                        g.rearm_pa()?;
                    }
                    engine.rearm_outputs()
                }
                #[cfg(feature = "hardware-host")]
                StructuralAction::Pa(prepared) => {
                    let g = modules.as_mut().ok_or("modules unavailable")?;
                    let code = g.commit_pa_change(prepared, epoch, frame);
                    if code == 0 {
                        Ok(())
                    } else {
                        Err(format!("PA commit {code}"))
                    }
                }
            };
            if let Err(e) = &applied {
                failure = Some(e.clone());
            }
            applied
        })?;
        #[cfg(not(feature = "hardware-host"))]
        let _ = epoch;
        #[cfg(feature = "hardware-host")]
        if let Some(g) = &mut self.modules {
            g.retire_pa_changes();
        }
        let applied_frame = if result.kind == "applied" {
            Some(frame)
        } else {
            None
        };
        let reply = crate::structural_control::Reply::new(
            &r,
            "final",
            failure.or(result.body.reason),
            applied_frame,
            result.body.revision,
            None,
        );
        let writer = r.writer.clone();
        self.structural_cache.push_back((r, reply.clone()));
        if self
            .structural_cache
            .iter()
            .filter(|(r, _)| r.writer == writer)
            .count()
            > 64
            && let Some(index) = self
                .structural_cache
                .iter()
                .position(|(r, _)| r.writer == writer)
        {
            self.structural_cache.remove(index);
        }
        if self.structural_cache.len() > 256 {
            self.structural_cache.pop_front();
        }
        if let Some(id) = owner {
            if let Some(c) = self.clients.iter_mut().find(|c| c.id == id) {
                let _ = c.queue_module(&reply, now);
            }
        } else if self.remote_completions.len() < 64 {
            self.remote_completions
                .push_back(serde_json::to_value(reply).map_err(|e| e.to_string())?);
        }
        Ok(())
    }
    pub fn take_remote_completions(&mut self) -> Vec<serde_json::Value> {
        self.remote_completions.drain(..).collect()
    }
    pub fn source_epoch(&self) -> u64 {
        self.epoch.0
    }
    pub fn quiesce_source(&mut self, reason: &str) -> Result<()> {
        self.engine.quiesce(reason);
        #[cfg(feature = "hardware-host")]
        if let Some(g) = &mut self.modules {
            g.quiesce_source()?;
        }
        self.structural_pending = None;
        self.structural_cache.clear();
        self.clients.clear();
        self.writer_connections.clear();
        self.pending_owner = None;
        self.processing_owner = None;
        Ok(())
    }
    pub fn recover_source(&mut self, epoch: Counter, frame: u64) -> Result<()> {
        if epoch.0 <= self.epoch.0 {
            return Err("fresh epoch required".into());
        }
        self.quiesce_source("source_reopen")?;
        self._epoch_owner.advance(epoch)?;
        self.engine.recover(epoch, frame)?;
        #[cfg(feature = "hardware-host")]
        if let Some(g) = &mut self.modules {
            g.discontinuity(epoch.0, frame)?;
        }
        self.epoch = epoch;
        self.analysis = None;
        self.remote_completions.clear();
        Ok(())
    }
    pub fn rearm(&mut self) -> Result<()> {
        #[cfg(feature = "hardware-host")]
        if let Some(g) = &mut self.modules {
            g.rearm_pa()?;
        }
        self.engine.rearm()
    }
    #[cfg(feature = "hardware-host")]
    pub fn configure_pa(&mut self, json: &[u8]) -> Result<()> {
        if self.frame() != 0 {
            return Err("initial PA setup requires unprocessed provider".into());
        }
        let capacity = self.topology().pa_outputs;
        let total = self.topology().monitors + 2;
        let g = self.modules.as_mut().ok_or("modules unavailable")?;
        if g.pa_configuration_json().is_some() {
            return Err("PA already configured; use structural transaction".into());
        }
        let mut prepared = g.prepare_pa_change(json, vec![0, 1], total)?;
        if prepared.output_channels() != capacity {
            return Err("initial PA output capability mismatch".into());
        }
        let code = g.commit_pa_change(&mut prepared, self.epoch.0, 0);
        if code != 0 {
            return Err(format!("initial PA commit {code}"));
        }
        g.retire_pa_changes();
        Ok(())
    }
    pub fn last_bus_samples(&self) -> &[f64] {
        &self.bus_scratch
    }
    pub fn revoke_writer(&mut self, writer: &str) {
        self.engine.revoke_writer(writer);
        if self
            .structural_pending
            .as_ref()
            .is_some_and(|(r, _, _)| r.writer.as_deref() == Some(writer))
        {
            self.structural_pending = None;
        }
        self.structural_cache
            .retain(|(r, _)| r.writer.as_deref() != Some(writer));
        // Keep the connection tombstone for this epoch: reconnect must use a
        // fresh writer identity, even after its old lease is revoked.
        self.remote_completions.retain(|r| {
            r.get("context")
                .and_then(|c| c.get("writer"))
                .and_then(|w| w.as_str())
                != Some(writer)
        });
    }
    pub fn engine_mut(&mut self) -> &mut OfflineEngine {
        &mut self.engine
    }
    pub fn topology(&self) -> &crate::topology::EngineTopology {
        self.engine.topology()
    }
    pub fn enable_synthetic_fouraux(&mut self) -> Result<()> {
        if self.frame() != 0 || self.analysis.is_some() {
            return Err("source must be selected before processing".into());
        }
        self.synthetic_fouraux = true;
        Ok(())
    }
    pub fn analysis_alive(&self) -> bool {
        self.analysis
            .as_ref()
            .is_some_and(|(_, worker)| worker.alive())
    }
    pub fn stop_analysis(&self) {
        if let Some((_, worker)) = &self.analysis {
            worker.stop();
        }
    }
    pub fn enable_analysis(&mut self, directory: &Path) -> Result<()> {
        if !self.synthetic_fouraux {
            return Err(
                "four named inputs unavailable in default source; select explicit fouraux".into(),
            );
        }
        if self.analysis.is_some() {
            return Err("analysis already enabled".into());
        }
        let descriptor = crate::analysis_stream::Descriptor::new(self.epoch.0, self.frame());
        let (tap, input) = crate::analysis_stream::Tap::new(&descriptor).map_err(String::from)?;
        let service =
            crate::analysis_stream::local::LocalAnalysis::bind(directory, descriptor, input)?;
        self.analysis = Some((tap, crate::analysis_stream::local::Worker::start(service)?));
        Ok(())
    }
    #[cfg(feature = "hardware-host")]
    pub fn enable_modules(&mut self, manifest: &Path) -> Result<()> {
        if self.frame() != 0 || self.modules.is_some() {
            return Err("module preparation before processing required".into());
        }
        let root = self.private_directory.join("takes");
        match fs::create_dir(&root) {
            Ok(()) => fs::set_permissions(&root, fs::Permissions::from_mode(0o700))
                .map_err(|e| e.to_string())?,
            Err(e) if e.kind() == ErrorKind::AlreadyExists => (),
            Err(e) => return Err(e.to_string()),
        }
        let m = fs::symlink_metadata(&root).map_err(|e| e.to_string())?;
        if !m.is_dir()
            || m.file_type().is_symlink()
            || m.uid() != unsafe { libc::geteuid() }
            || m.mode() & 0o777 != 0o700
            || fs::canonicalize(&root).map_err(|e| e.to_string())? != root
        {
            return Err("owned recording root".into());
        }
        let manifest = crate::module_graph::Manifest::load(manifest)?;
        self.modules = Some(crate::module_graph::ModuleGraph::load_configured(
            manifest,
            self.epoch.0,
            self.frame(),
            self.topology().inputs.len(),
        )?);
        Ok(())
    }
    pub fn module_status(&mut self, now: u64) -> Result<serde_json::Value> {
        #[cfg(feature = "hardware-host")]
        if let Some(graph) = &mut self.modules {
            graph.poll_lifecycle()?;
            return serde_json::to_value(graph.status(&self.show, now)?).map_err(|e| e.to_string());
        }
        let _ = now;
        Ok(crate::module_wire::unavailable_status(
            &self.show,
            self.epoch.0,
            self.frame(),
        ))
    }
    fn module_request(
        &mut self,
        index: usize,
        r: crate::module_wire::ModuleRequest,
        now: u64,
    ) -> Result<()> {
        if !matches!(
            r.command,
            crate::module_wire::ModuleCommand::ModuleStatus {}
        ) {
            let c = &self.clients[index];
            if !c.snapshot || c.writer != r.writer || c.lease != r.lease {
                return Err("module connection authority".into());
            }
        }
        let result = self.dispatch_module(r, now, Some(self.clients[index].id))?;
        self.clients[index].queue_module(&result, now)
    }
    pub fn dispatch_module(
        &mut self,
        r: crate::module_wire::ModuleRequest,
        now: u64,
        owner: Option<u64>,
    ) -> Result<serde_json::Value> {
        use crate::module_wire::{ModuleCommand, ModuleReply};
        #[cfg(not(feature = "hardware-host"))]
        let _ = owner;
        if r.version != self.engine.wire_version() {
            return Err("module wire version mismatch".into());
        }
        if r.show_id != self.show || r.epoch != self.epoch {
            return Err("module attachment identity".into());
        }
        if matches!(r.command, ModuleCommand::ModuleStatus {}) {
            let status = self.module_status(now)?;
            return Ok(status);
        }
        let auth = self
            .engine
            .authorize_module(&r.authority_request(), &r.fingerprint()?, now);
        let reply = match auth {
            Err(reason) => ModuleReply::new(&r, "rejected", Some(reason)),
            Ok((true, _)) => self
                .module_cache
                .iter()
                .find(|(old, _)| old == &r)
                .map(|(_, p)| p.clone())
                .unwrap_or_else(|| {
                    ModuleReply::new(&r, "rejected", Some("expired_outcome".into()))
                }),
            Ok((false, Some(reason))) => {
                let reply = ModuleReply::new(&r, "rejected", Some(reason));
                self.module_cache.push_back((r.clone(), reply.clone()));
                if self.module_cache.len() > 256 {
                    self.module_cache.pop_front();
                }
                reply
            }
            Ok((false, None)) => {
                #[cfg(feature = "hardware-host")]
                let result = if let Some(graph) = &mut self.modules {
                    if self.module_pending.len() >= 2 {
                        Err("lifecycle backpressure".into())
                    } else {
                        match &r.command {
                            ModuleCommand::RecordStart {
                                take_id,
                                operation_id,
                            } => graph.start(
                                &self.private_directory.join("takes"),
                                take_id,
                                *operation_id,
                            ),
                            ModuleCommand::RecordStop {
                                take_id,
                                operation_id,
                            } => graph.stop(take_id, *operation_id),
                            _ => unreachable!(),
                        }
                    }
                } else {
                    Err("modules unavailable".into())
                };
                #[cfg(not(feature = "hardware-host"))]
                let result: Result<()> = Err("modules unavailable".into());
                let reply = match result {
                    Ok(()) => {
                        #[cfg(feature = "hardware-host")]
                        self.module_pending.push((r.clone(), owner.unwrap_or(0)));
                        ModuleReply::new(&r, "accepted_pending", None)
                    }
                    Err(reason) => ModuleReply::new(&r, "rejected", Some(reason)),
                };
                self.module_cache.push_back((r.clone(), reply.clone()));
                if self.module_cache.len() > 256 {
                    self.module_cache.pop_front();
                }
                reply
            }
        };
        serde_json::to_value(reply).map_err(|e| e.to_string())
    }
    fn processing_request(
        &mut self,
        index: usize,
        r: crate::processing_wire::ProcessingRequest,
        now: u64,
    ) -> Result<()> {
        use crate::processing_wire::{ProcessingCommand, ProcessingReply};
        let read = matches!(r.command, ProcessingCommand::ProcessingSnapshot {});
        let client = &self.clients[index];
        let reply = if !read
            && (!client.snapshot || client.writer != r.writer || client.lease != r.lease)
        {
            ProcessingReply::new(&r, "final", Some("lease".into()), self.engine.revision())
        } else {
            self.engine.handle_processing_with_freshness(
                &r,
                now,
                client
                    .processing_snapshot_ms
                    .is_some_and(|t| now.saturating_sub(t) <= 250),
            )?
        };
        if read && reply.snapshot.is_some() {
            self.clients[index].processing_snapshot_ms = Some(now);
        }
        if reply.state == "pending" {
            self.processing_owner = Some((
                reply.ticket.expect("pending ticket").0,
                self.clients[index].id,
            ));
        }
        reply.validate()?;
        self.clients[index].queue_module(&reply, now)
    }
    /// Captured final PA output from the actual owner graph, when enabled.
    #[cfg(feature = "hardware-host")]
    pub fn module_output(&self) -> Option<&[f64; 48 * 6]> {
        self.modules.as_ref().map(|g| g.output())
    }
    #[cfg(feature = "hardware-host")]
    fn module_completions(&mut self, now: u64) -> Result<()> {
        use crate::module_wire::{ModuleCommand, ModuleReply};
        let Some(graph) = &mut self.modules else {
            return Ok(());
        };
        graph.poll_lifecycle()?;
        if self.module_pending.is_empty() {
            return Ok(());
        }
        let status = graph.status(&self.show, now)?;
        let Some(recording) = status.recording else {
            return Ok(());
        };
        let mut i = 0;
        while i < self.module_pending.len() {
            let (r, owner) = &self.module_pending[i];
            let start = matches!(r.command, ModuleCommand::RecordStart { .. });
            let done = if start {
                recording.state == "recording" || recording.state == "finalized"
            } else {
                recording.state == "finalized"
            };
            if !done {
                i += 1;
                continue;
            }
            let state = if start && recording.state != "recording" {
                "cancelled"
            } else {
                "completed"
            };
            let reply = ModuleReply::new(r, state, Some(recording.outcome.clone()));
            if let Some((_, cached)) = self.module_cache.iter_mut().find(|(old, _)| old == r) {
                *cached = reply.clone();
            }
            if *owner == 0 && self.remote_completions.len() < 64 {
                self.remote_completions
                    .push_back(serde_json::to_value(&reply).map_err(|e| e.to_string())?);
            }
            if let Some(c) = self.clients.iter_mut().find(|c| c.id == *owner) {
                let _ = c.queue_module(&reply, now);
            }
            self.module_pending.remove(i);
        }
        Ok(())
    }
    pub fn frame(&self) -> u64 {
        self.engine.frame()
    }
    pub fn client_count(&self) -> usize {
        self.clients.len()
    }
    pub fn snapshot(&mut self) -> Result<crate::mixer_control::RenderedSnapshot> {
        self.engine.snapshot()
    }
    /// Control-side observation, outside the bounded mixer callback.
    pub fn processing_snapshot(&mut self) -> Result<crate::processing_wire::ProcessingSnapshot> {
        self.engine.processing_snapshot()
    }
    /// Bounded synthetic work: <=4 accepts, one frame/client,8KiB read/write/client,
    ///48 render frames. All parsing/IPC/heap work is outside Mixer::process.
    pub fn tick(&mut self, now: u64) -> Result<()> {
        let mut capture = std::mem::take(&mut self.capture_scratch);
        capture.fill(0.);
        let topology = self.topology();
        for f in 0..48 {
            for (i, p) in topology.inputs.iter().enumerate() {
                capture[f * topology.capture_channels + p.capture_slot] = if self.synthetic_fouraux
                {
                    synthetic_sample(self.frame() + f as u64, i)
                } else if i == 0 {
                    0.125
                } else {
                    0.
                };
            }
        }
        let mut playback = std::mem::take(&mut self.playback_scratch);
        let result =
            self.tick_with_capture(now, self.epoch.0, self.frame(), &capture, &mut playback);
        self.capture_scratch = capture;
        self.playback_scratch = playback;
        result
    }
    /// Same provider pump, with caller-owned observation of the FOH/monitor block.
    /// This is software evidence; it does not open a hardware endpoint.
    pub fn tick_with_output(&mut self, now: u64, output: &mut [[f64; 4]; 48]) -> Result<()> {
        if !self.topology().is_legacy() {
            return Err("legacy output shape".into());
        }
        let mut inputs = [[0.; 8]; 48];
        for (f, row) in inputs.iter_mut().enumerate() {
            for (i, v) in row.iter_mut().enumerate() {
                *v = if self.synthetic_fouraux {
                    synthetic_sample(self.frame() + f as u64, i)
                } else if i == 0 {
                    0.125
                } else {
                    0.
                };
            }
        }
        self.tick_raw(now, inputs.as_flattened(), output.as_flattened_mut())
    }
    /// Capture and playback use configured transport slots. Logical strip ordering
    /// is independent of that permutation. This control pump is outside render.
    pub fn tick_with_capture(
        &mut self,
        now: u64,
        epoch: u64,
        first_frame: u64,
        capture: &[f64],
        playback: &mut [f64],
    ) -> Result<()> {
        self.tick_with_capture_and_wet(now, epoch, first_frame, capture, playback, None)
    }
    pub fn tick_with_capture_and_wet(
        &mut self,
        now: u64,
        epoch: u64,
        first_frame: u64,
        capture: &[f64],
        playback: &mut [f64],
        wet: Option<&[f64]>,
    ) -> Result<()> {
        let topology = self.topology();
        if capture.len() != 48 * topology.capture_channels
            || playback.len() != 48 * topology.playback_channels
        {
            return Err("device block shape".into());
        }
        playback.fill(0.);
        if epoch != self.epoch.0 || first_frame != self.frame() {
            self.quiesce_source("source_discontinuity")?;
            return Err("source timeline".into());
        }
        if let Err(e) = self.engine.admit_source(epoch, first_frame, 48) {
            self.quiesce_source("source_discontinuity")?;
            return Err(e);
        }
        let mut raw = std::mem::take(&mut self.raw_scratch);
        let topology = self.topology();
        for f in 0..48 {
            for (i, p) in topology.inputs.iter().enumerate() {
                raw[f * topology.inputs.len() + i] =
                    capture[f * topology.capture_channels + p.capture_slot];
            }
        }
        let mut buses = std::mem::take(&mut self.bus_scratch);
        let result = self.tick_raw_with_wet(now, &raw, &mut buses, wet);
        if result.is_ok() {
            let topology = self.topology();
            #[cfg(feature = "hardware-host")]
            let pa = self
                .modules
                .as_ref()
                .map(|m| m.output_interleaved())
                .filter(|p| p.len() == self.pa_silence.len())
                .unwrap_or(&self.pa_silence);
            #[cfg(not(feature = "hardware-host"))]
            let pa = &self.pa_silence;
            for f in 0..48 {
                topology.patch_outputs(
                    &buses[f * (topology.monitors + 2)..(f + 1) * (topology.monitors + 2)],
                    &pa[f * topology.pa_outputs..(f + 1) * topology.pa_outputs],
                    &mut playback
                        [f * topology.playback_channels..(f + 1) * topology.playback_channels],
                )?;
            }
        }
        self.raw_scratch = raw;
        self.bus_scratch = buses;
        if result.is_err() {
            self.quiesce_source("processing_failure")?;
        }
        result
    }

    fn tick_raw(&mut self, now: u64, inputs: &[f64], output: &mut [f64]) -> Result<()> {
        self.tick_raw_with_wet(now, inputs, output, None)
    }
    fn tick_raw_with_wet(
        &mut self,
        now: u64,
        inputs: &[f64],
        output: &mut [f64],
        wet: Option<&[f64]>,
    ) -> Result<()> {
        if now < self.last_now {
            return Err("clock".into());
        }
        self.last_now = now;
        self.ticks = self.ticks.checked_add(1).ok_or("tick exhausted")?;
        for _ in 0..MAX_CLIENTS {
            match self.listener.accept() {
                Ok((stream, _)) => {
                    if self.clients.len() >= MAX_CLIENTS || !same_uid(&stream).unwrap_or(false) {
                        continue;
                    }
                    stream.set_nonblocking(true).map_err(|e| e.to_string())?;
                    let id = self.next_client;
                    self.next_client = self.next_client.checked_add(1).ok_or("client exhausted")?;
                    self.clients.push(Client {
                        id,
                        stream,
                        buffer: Box::new([0; MAX_FRAME + 4]),
                        used: 0,
                        started: None,
                        replies: VecDeque::with_capacity(MAX_REPLIES),
                        telemetry: None,
                        last_write: now,
                        snapshot: false,
                        processing_snapshot_ms: None,
                        structural_snapshot_ms: None,
                        writer: None,
                        lease: None,
                    });
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                Err(e) if e.kind() == ErrorKind::Interrupted => break,
                Err(e) => return Err(e.to_string()),
            }
        }
        let mut index = 0;
        while index < self.clients.len() {
            let result = self.clients[index].receive(now, self.engine.wire_version() + 1);
            let mut keep = true;
            match result {
                Err(_) => keep = false,
                Ok(None) => (),
                Ok(Some(Incoming::UnsupportedProcessing(request))) => {
                    let reply = request.refusal(self.engine.revision());
                    if self.clients[index].queue_module(&reply, now).is_err() {
                        keep = false;
                    }
                }
                Ok(Some(Incoming::Structural(request))) => {
                    let read = matches!(
                        request.command,
                        crate::structural_control::Command::StructuralSnapshot {}
                    );
                    let c = &self.clients[index];
                    let authorized = read
                        || (c.snapshot && c.writer == request.writer && c.lease == request.lease);
                    let fresh = c
                        .structural_snapshot_ms
                        .is_some_and(|t| now.saturating_sub(t) <= 250);
                    if !authorized {
                        keep = false;
                    } else {
                        let owner = Some(c.id);
                        let reply = self.structural_request(request, now, fresh, owner)?;
                        if read && reply.snapshot.is_some() {
                            self.clients[index].structural_snapshot_ms = Some(now);
                        }
                        if self.clients[index].queue_module(&reply, now).is_err() {
                            keep = false;
                        }
                    }
                }
                Ok(Some(Incoming::Processing(request))) => {
                    if self.processing_request(index, request, now).is_err() {
                        keep = false;
                    }
                }
                Ok(Some(Incoming::Modules(request))) => {
                    if self.module_request(index, request, now).is_err() {
                        keep = false;
                    }
                }
                Ok(Some(Incoming::Audio(request))) => {
                    let client = &mut self.clients[index];
                    let read = matches!(request.command, Command::Snapshot {});
                    let grant = matches!(request.command, Command::Grant { .. });
                    if !read
                        && (!client.snapshot
                            || (!grant
                                && (client.writer != request.writer
                                    || client.lease != request.lease))
                            || (grant
                                && request
                                    .writer
                                    .as_ref()
                                    .and_then(|w| self.writer_connections.get(w))
                                    .is_some_and(|owner| *owner != client.id)))
                    {
                        keep = false;
                    } else {
                        let reply = self.engine.handle(&request, now)?;
                        if read && reply.outcome.as_ref().is_some_and(|p| p.kind == "applied") {
                            client.snapshot = true;
                        }
                        if grant
                            && let Some(lease) =
                                reply.outcome.as_ref().and_then(|p| p.body.granted_lease)
                        {
                            client.writer = request.writer.clone();
                            client.lease = Some(lease);
                            self.writer_connections
                                .insert(request.writer.clone().unwrap(), client.id);
                        }
                        if reply.state == "pending" {
                            self.pending_owner = Some((reply.ticket.unwrap().0, client.id));
                        }
                        if client.queue(&reply, now).is_err() {
                            keep = false;
                        }
                    }
                }
            }
            if keep {
                index += 1;
            } else {
                let removed = self.clients.remove(index);
                if let Some(writer) = removed.writer {
                    self.revoke_writer(&writer);
                }
            }
        }
        #[cfg(feature = "hardware-host")]
        self.module_completions(now)?;
        self.commit_structure(now)?;
        output.fill(0.);
        let source_frame = self.frame();
        let ni = self.topology().inputs.len();
        let buses = self.topology().monitors + 2;
        if let Some((tap, worker)) = &mut self.analysis
            && let Ok(mono) = crate::analysis_stream::local::monotonic_ms()
        {
            for (out, v) in self.pcm_scratch.iter_mut().zip(inputs) {
                *out = (v * 8_388_608.).round() as i32;
            }
            let _ = tap.offer_interleaved(source_frame, &self.pcm_scratch, ni, mono);
            worker.update_losses(tap.dropped_windows);
        }
        let completions = self.engine.process_interleaved(inputs, output, now)?;
        #[cfg(feature = "hardware-host")]
        if let Some(graph) = &mut self.modules {
            match graph.process_interleaved(self.epoch.0, source_frame, inputs, output, buses, wet)
            {
                Ok(()) | Err(crate::module_graph::ProcessError::Fx(_)) => (),
                Err(error) => {
                    output.fill(0.);
                    self.quiesce_source("module_processing_failure")?;
                    return Err(format!("module processing {error:?}"));
                }
            }
        }
        #[cfg(not(feature = "hardware-host"))]
        let _ = (buses, wet);
        for reply in completions {
            if self.pending_owner.is_none() && self.remote_completions.len() < 64 {
                self.remote_completions
                    .push_back(serde_json::to_value(&reply).map_err(|e| e.to_string())?);
            }
            if let Some((ticket, owner)) = self.pending_owner.take()
                && reply.ticket.is_none_or(|t| t.0 == ticket)
                && let Some(index) = self.clients.iter().position(|c| c.id == owner)
                && self.clients[index].queue(&reply, now).is_err()
            {
                self.clients.remove(index);
            }
        }
        for reply in self.engine.take_processing_completions() {
            if self.processing_owner.is_none() && self.remote_completions.len() < 64 {
                self.remote_completions
                    .push_back(serde_json::to_value(&reply).map_err(|e| e.to_string())?);
            }
            if let Some((ticket, owner)) = self.processing_owner.take()
                && reply.ticket.is_none_or(|t| t.0 == ticket)
                && let Some(index) = self.clients.iter().position(|c| c.id == owner)
                && self.clients[index].queue_module(&reply, now).is_err()
            {
                self.clients.remove(index);
            }
        }
        // Latest-only telemetry never replaces a partially transmitted frame.
        if self.ticks.is_multiple_of(16) {
            for client in &mut self.clients {
                if client.snapshot && !client.telemetry.as_ref().is_some_and(|p| p.offset > 0) {
                    let request = Request {
                        contract: "C-AUDIO".into(),
                        version: self.engine.wire_version(),
                        show_id: self.show.clone(),
                        module: "audio".into(),
                        epoch: self.epoch,
                        writer: None,
                        lease: None,
                        request_id: None,
                        expected_revision: None,
                        command: Command::Snapshot {},
                    };
                    let reply = self.engine.handle(&request, now)?;
                    if client.replies.is_empty() && client.telemetry.is_none() {
                        client.last_write = now;
                    }
                    let bytes = serde_json::to_vec(&reply).map_err(|e| e.to_string())?;
                    if bytes.len() <= MAX_FRAME {
                        client.telemetry = Some(Packet::new(bytes)?);
                    } // expanded clients request coherent pages; never partial periodic snapshots
                }
            }
        }
        let mut index = 0;
        while index < self.clients.len() {
            if self.clients[index].flush(now).is_ok() {
                index += 1;
            } else {
                let c = self.clients.remove(index);
                if let Some(writer) = c.writer {
                    self.revoke_writer(&writer);
                }
            }
        }
        Ok(())
    }
}

fn synthetic_sample(frame: u64, input: usize) -> f64 {
    // Legacy fouraux samples are exact; additional channels have distinct PCM24
    // identity and all pass through the same raw tap and channel processor.
    let block = crate::analysis_stream::synthetic_inputs(frame - frame % 48);
    if input < 8 {
        f64::from(block[(frame % 48) as usize][input]) / 8_388_608.
    } else {
        (((frame as i64 * 17 + input as i64 * 7919) % 65537) - 32768) as f64 / 8_388_608.
    }
}
