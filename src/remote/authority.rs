//! Adapter to the actual mixer authority. This owns no second DSP/control model.
//! A host drives `process_interleaved` from its existing source timeline, outside
//! network tasks. Use the bounded proxy when the controller runs independently.
use super::*;
use crate::{
    control_model::{Command, Request as AudioRequest, Scope},
    mixer_control::OfflineEngine,
    processing_wire::{ProcessingCommand, ProcessingRequest},
};
use serde_json::Value;
use std::collections::{BTreeMap, VecDeque};

pub struct EngineAuthority {
    engine: OfflineEngine,
    capability_generation: u64,
    replies: VecDeque<Value>,
    received_media: VecDeque<Vec<u8>>,
    outgoing_media: VecDeque<Vec<u8>>,
    descriptor: Option<MediaDescriptor>,
    fx_channels: Vec<String>,
    read_states: BTreeMap<u64, ReadState>,
}
impl EngineAuthority {
    pub fn new(engine: OfflineEngine, capability_generation: u64) -> Result<Self> {
        if capability_generation == 0 {
            return Err("zero capability generation".into());
        }
        Ok(Self {
            engine,
            capability_generation,
            replies: VecDeque::with_capacity(128),
            received_media: VecDeque::with_capacity(256),
            outgoing_media: VecDeque::with_capacity(256),
            descriptor: None,
            fx_channels: vec!["foh-left".into(), "foh-right".into()],
            read_states: BTreeMap::new(),
        })
    }
    pub fn engine(&self) -> &OfflineEngine {
        &self.engine
    }
    pub fn engine_mut(&mut self) -> &mut OfflineEngine {
        &mut self.engine
    }
    pub fn descriptor(&self) -> Option<&MediaDescriptor> {
        self.descriptor.as_ref()
    }
    pub fn take_media(&mut self) -> Option<Vec<u8>> {
        self.received_media.pop_front()
    }
    pub fn queue_media(&mut self, bytes: Vec<u8>) -> Result<()> {
        if bytes.len() > crate::transport::MAX_DATAGRAM || self.outgoing_media.len() == 256 {
            return Err("outgoing media queue capacity".into());
        }
        self.outgoing_media.push_back(bytes);
        Ok(())
    }
    pub fn process_interleaved(
        &mut self,
        input: &[f64],
        output: &mut [f64],
        now_ms: u64,
    ) -> Result<()> {
        let completed = self.engine.process_interleaved(input, output, now_ms)?;
        for reply in completed {
            self.queue_reply(serde_json::to_value(&reply).map_err(|e| e.to_string())?)?;
        }
        for reply in self.engine.take_processing_completions() {
            self.queue_reply(serde_json::to_value(&reply).map_err(|e| e.to_string())?)?;
        }
        for reply in self.engine.take_sends_completions() {
            self.queue_reply(serde_json::to_value(&reply).map_err(|e| e.to_string())?)?;
        }
        Ok(())
    }
    fn queue_reply(&mut self, value: Value) -> Result<()> {
        if self.replies.len() == 128 {
            return Err("authority completion queue capacity".into());
        }
        self.replies.push_back(value);
        Ok(())
    }
    pub fn dispatch_command(
        &mut self,
        context: &AuthenticatedContext,
        payload: Value,
        now_ms: u64,
    ) -> Result<Value> {
        if self.read_states.len() >= MAX_CONNECTIONS
            && !self.read_states.contains_key(&context.session())
        {
            return Err("remote read-state capacity".into());
        }
        let state = self.read_states.entry(context.session()).or_default();
        dispatch_engine(&mut self.engine, context, payload, now_ms, state)
    }
}
/// Readback gates belong to each authenticated connection, never persisted show
/// intent. Reconnect starts empty and cannot grant from an old snapshot.
#[derive(Default)]
pub struct ReadState {
    audio_snapshot: bool,
    sends_snapshot_ms: Option<u64>,
    processing_snapshot_ms: Option<u64>,
}

impl ReadState {
    /// Admit a completely constructed paired snapshot without discarding the
    /// independent processing observation or marking it fresh again.
    #[cfg(all(target_os = "linux", feature = "hardware-host"))]
    pub(crate) fn admit_audio_snapshot(&mut self) {
        self.audio_snapshot = true;
    }
    #[cfg(all(target_os = "linux", feature = "hardware-host"))]
    pub(crate) fn audio_ready(&self) -> bool {
        self.audio_snapshot
    }
}

/// Shared production authority dispatch for standalone and composed hosts.
pub fn dispatch_engine(
    engine: &mut OfflineEngine,
    context: &AuthenticatedContext,
    payload: Value,
    now_ms: u64,
    state: &mut ReadState,
) -> Result<Value> {
    context.check_writer(&payload)?;
    let bytes = super::encode(&payload)?;
    match payload.get("contract").and_then(Value::as_str) {
        Some("C-AUDIO") => {
            let request = AudioRequest::decode(&bytes)?;
            if !matches!(request.command, Command::Snapshot {}) && !state.audio_snapshot {
                return Err("remote snapshot required before mutation".into());
            }
            let permission = match request.command {
                Command::Snapshot {} => None,
                Command::Grant { scope } => Some(scope_permission(scope)),
                _ => Some(scope_permission(
                    engine
                        .writer_scope(&request, now_ms)
                        .ok_or("remote live scope required")?,
                )),
            };
            if let Some(permission) = permission {
                context.require(&permission)?;
            }
            let reply = engine.handle(&request, now_ms)?;
            if matches!(request.command, Command::Snapshot {}) && reply.snapshot.is_some() {
                state.audio_snapshot = true;
            }
            serde_json::to_value(&reply).map_err(|e| e.to_string())
        }
        Some(crate::processing_wire::CONTRACT) => {
            if let Some(unsupported) = crate::processing_wire::UnsupportedRequest::decode_for(
                &bytes,
                engine.processing_version(),
            )? {
                return Ok(unsupported.refusal(engine.revision()));
            }
            let request = ProcessingRequest::decode(&bytes)?;
            if !matches!(request.command, ProcessingCommand::ProcessingSnapshot {}) {
                context.require(&Permission::Foh)?;
                if engine.writer_scope(&request.authority_request(), now_ms) != Some(Scope::Foh) {
                    return Err("remote FOH lease required".into());
                }
            }
            let fresh = state
                .processing_snapshot_ms
                .is_some_and(|t| now_ms >= t && now_ms - t <= 250);
            let reply = engine.handle_processing_with_freshness(&request, now_ms, fresh)?;
            if matches!(request.command, ProcessingCommand::ProcessingSnapshot {})
                && reply.snapshot.is_some()
            {
                state.processing_snapshot_ms = Some(now_ms);
            }
            serde_json::to_value(&reply).map_err(|e| e.to_string())
        }
        Some(crate::sends_wire::CONTRACT) => {
            use crate::sends_wire::{SendsCommand, SendsRequest};
            let request = SendsRequest::decode(&bytes)?;
            let read = matches!(request.command, SendsCommand::SendsSnapshot {});
            if !read {
                let scope = request.scope()?;
                context.require(&scope_permission(scope))?;
                if !state.audio_snapshot
                    || engine.writer_scope(&request.authority_request(), now_ms) != Some(scope)
                {
                    return Err("remote monitor lease required".into());
                }
            }
            let fresh = state
                .sends_snapshot_ms
                .is_some_and(|t| now_ms >= t && now_ms - t <= 250);
            let reply = engine.handle_sends_with_freshness(&request, now_ms, fresh)?;
            if read && reply.snapshot.is_some() {
                state.sends_snapshot_ms = Some(now_ms);
            }
            serde_json::to_value(&reply).map_err(|e| e.to_string())
        }
        _ => Err("remote authority contract unavailable".into()),
    }
}
pub fn scope_permission(scope: Scope) -> Permission {
    match scope {
        Scope::Foh => Permission::Foh,
        Scope::Monitor1 => Permission::Monitor(1),
        Scope::Monitor2 => Permission::Monitor(2),
        Scope::Monitor(n) => Permission::Monitor(u32::from(n)),
        Scope::PaConfiguration => Permission::PaConfiguration,
        Scope::OutputRoutes => Permission::OutputRoutes,
        Scope::LocalOperatorMonitor => Permission::LocalOperatorMonitor,
        Scope::TalkbackDestinations => Permission::TalkbackDestinations,
        Scope::TalkbackFoh => Permission::TalkbackFoh,
    }
}
impl AuthorityEndpoint for EngineAuthority {
    fn identity(&self) -> EngineIdentity {
        EngineIdentity {
            source_epoch: crate::show::Counter(self.engine.clock_status().epoch),
            capability_generation: crate::show::Counter(self.capability_generation),
            map_generation: crate::show::Counter(self.engine.topology().map_revision),
        }
    }
    fn dispatch(
        &mut self,
        context: &AuthenticatedContext,
        payload: Value,
        now_ms: u64,
    ) -> Result<Option<Value>> {
        self.dispatch_command(context, payload, now_ms).map(Some)
    }
    fn negotiate(
        &mut self,
        context: &AuthenticatedContext,
        descriptor: &MediaDescriptor,
    ) -> Result<bool> {
        // The server already checks transport shape; this verifies actual engine
        // IDs/taps. It cannot authorize made-up analysis or effect channels.
        for stream in &descriptor.streams {
            match stream.role {
                MediaRole::Analysis => {
                    context.require(&Permission::Analysis)?;
                    if stream.channel_ids.iter().any(|id| {
                        !self
                            .engine
                            .topology()
                            .inputs
                            .iter()
                            .any(|input| &input.id == id)
                    }) {
                        return Err("analysis source not in admitted topology".into());
                    }
                }
                MediaRole::FxSend | MediaRole::WetReturn => {
                    context.require(&Permission::Fx)?;
                }
            }
        }
        for role in [MediaRole::FxSend, MediaRole::WetReturn] {
            let mut streams: Vec<_> = descriptor
                .streams
                .iter()
                .filter(|s| s.role == role)
                .collect();
            streams.sort_by_key(|s| s.first_channel);
            let ids: Vec<_> = streams
                .iter()
                .flat_map(|s| s.channel_ids.iter().cloned())
                .collect();
            if !ids.is_empty() && ids != self.fx_channels {
                return Err("FX route identity mismatch".into());
            }
        }
        self.received_media.clear();
        self.outgoing_media.clear();
        self.descriptor = Some(descriptor.clone());
        Ok(true)
    }
    fn media(
        &mut self,
        context: &AuthenticatedContext,
        packet: crate::transport::Packet<'_>,
        _now_ms: u64,
    ) -> Result<()> {
        context.require(&Permission::Fx)?;
        if self.received_media.len() == 256 {
            self.received_media.pop_front();
        }
        self.received_media.push_back(packet.bytes().to_vec());
        Ok(())
    }
    fn disconnect(&mut self, context: &AuthenticatedContext) {
        self.engine.revoke_writer(context.writer());
        self.read_states.remove(&context.session());
        self.replies.retain(|r| {
            r.get("context")
                .and_then(|c| c.get("writer"))
                .and_then(Value::as_str)
                != Some(context.writer())
        });
        self.received_media.clear();
        self.outgoing_media.clear();
        self.descriptor = None;
    }
    fn source_frame(&self) -> Option<u64> {
        Some(self.engine.frame())
    }
    fn poll_reply(
        &mut self,
        context: &AuthenticatedContext,
        _now_ms: u64,
    ) -> Result<Option<Value>> {
        let position = self.replies.iter().position(|r| {
            r.get("context")
                .and_then(|c| c.get("writer"))
                .and_then(Value::as_str)
                == Some(context.writer())
        });
        Ok(position.and_then(|i| self.replies.remove(i)))
    }
    fn poll_media(&mut self, _context: &AuthenticatedContext) -> Result<Option<Vec<u8>>> {
        Ok(self.outgoing_media.pop_front())
    }
}

#[cfg(all(test, target_os = "linux", feature = "hardware-host"))]
mod atomic_latch_tests {
    use super::ReadState;
    #[test]
    fn atomic_audio_admission_preserves_processing_freshness() {
        let mut state = ReadState {
            audio_snapshot: false,
            processing_snapshot_ms: Some(17),
            sends_snapshot_ms: None,
        };
        state.admit_audio_snapshot();
        assert!(state.audio_ready());
        assert_eq!(state.processing_snapshot_ms, Some(17));
        state.admit_audio_snapshot();
        assert_eq!(state.processing_snapshot_ms, Some(17));
    }
}
