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
    Modules(crate::module_wire::ModuleRequest),
    Processing(crate::processing_wire::ProcessingRequest),
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
        self.replies.push_back(Packet::new(reply.encode()?)?);
        Ok(())
    }
    fn receive(&mut self, now: u64) -> Result<Option<Incoming>> {
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
        let request = if value.get("contract").and_then(|v| v.as_str()) == Some("GP05-modules") {
            Incoming::Modules(crate::module_wire::ModuleRequest::decode(bytes)?)
        } else if value.get("contract").and_then(|v| v.as_str()) == Some("GP07-processing") {
            Incoming::Processing(crate::processing_wire::ProcessingRequest::decode(bytes)?)
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
        self.replies.push_back(Packet::new(
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
}
impl EpochOwner {
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
        Ok(Self { _file: lock })
    }
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
        let engine = OfflineEngine::new(show, epoch, Counter(0), 0)?;
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
            #[cfg(feature = "hardware-host")]
            private_directory: directory.into(),
            module_cache: VecDeque::with_capacity(256),
            #[cfg(feature = "hardware-host")]
            modules: None,
            #[cfg(feature = "hardware-host")]
            module_pending: Vec::with_capacity(2),
        })
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
        self.modules = Some(crate::module_graph::ModuleGraph::load(
            manifest,
            self.epoch.0,
            self.frame(),
        )?);
        Ok(())
    }
    pub fn module_status(&mut self, now: u64) -> Result<serde_json::Value> {
        #[cfg(feature = "hardware-host")]
        if let Some(graph) = &mut self.modules {
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
        use crate::module_wire::{ModuleCommand, ModuleReply};
        if r.show_id != self.show || r.epoch != self.epoch {
            return Err("module attachment identity".into());
        }
        if matches!(r.command, ModuleCommand::ModuleStatus {}) {
            let status = self.module_status(now)?;
            return self.clients[index].queue_module(&status, now);
        }
        let client = &self.clients[index];
        if !client.snapshot || client.writer != r.writer || client.lease != r.lease {
            return Err("module connection authority".into());
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
                        self.module_pending
                            .push((r.clone(), self.clients[index].id));
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
        self.clients[index].queue_module(&reply, now)
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
        self.tick_with_output(now, &mut [[0.; 4]; 48])
    }
    /// Same provider pump, with caller-owned observation of the FOH/monitor block.
    /// This is software evidence; it does not open a hardware endpoint.
    pub fn tick_with_output(&mut self, now: u64, output: &mut [[f64; 4]; 48]) -> Result<()> {
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
            let result = self.clients[index].receive(now);
            let mut keep = true;
            match result {
                Err(_) => keep = false,
                Ok(None) => (),
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
                self.clients.remove(index);
            }
        }
        #[cfg(feature = "hardware-host")]
        self.module_completions(now)?;
        output.fill([0.; 4]);
        let mut inputs = [[0.125, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]; 48];
        let source_frame = self.frame();
        if self.synthetic_fouraux {
            let raw = crate::analysis_stream::synthetic_inputs(source_frame);
            if let Some((tap, worker)) = &mut self.analysis
                && let Ok(mono) = crate::analysis_stream::local::monotonic_ms()
            {
                let _ = tap.offer(source_frame, &raw, mono);
                worker.update_losses(tap.dropped_windows);
            }
            inputs = raw.map(|f| f.map(|v| f64::from(v) / 8_388_608.0));
        }
        let completions = self.engine.process(&inputs, output, now)?;
        #[cfg(feature = "hardware-host")]
        if let Some(graph) = &mut self.modules {
            let _ = graph.process(self.epoch.0, source_frame, &inputs, output);
        }
        for reply in completions {
            if let Some((ticket, owner)) = self.pending_owner.take()
                && reply.ticket.is_none_or(|t| t.0 == ticket)
                && let Some(index) = self.clients.iter().position(|c| c.id == owner)
                && self.clients[index].queue(&reply, now).is_err()
            {
                self.clients.remove(index);
            }
        }
        for reply in self.engine.take_processing_completions() {
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
                        version: 1,
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
                    client.telemetry = Some(Packet::new(reply.encode()?)?);
                }
            }
        }
        self.clients.retain_mut(|c| c.flush(now).is_ok());
        Ok(())
    }
}
