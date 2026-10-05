//! The authenticated endpoint for the existing composed LocalAudio graph. It
//! never starts a device or advances time from packet arrival. The source owner
//! calls `process_source` with the device/fake-device epoch and first frame.
use super::*;
use crate::{
    local_audio::LocalAudio,
    show::Counter,
    transport::{JitterBuffer, Packet, Role, StreamSpec, WetGate},
};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, VecDeque};
struct WetGroup {
    spec: StreamSpec,
    first: u64,
    buffer: JitterBuffer,
}

#[derive(Default, Serialize)]
pub struct HostMediaStats {
    pub accepted_wet_packets: u64,
    pub late_wet_packets: u64,
    pub rejected_wet_packets: u64,
    pub nonzero_received_wet_samples: u64,
    pub nonzero_rendered_wet_samples: u64,
    pub rendered_wet_energy: f64,
    pub first_wet_source_frame: Option<u64>,
    pub last_wet_source_frame: Option<u64>,
    pub first_wet_output_frame: Option<u64>,
    pub last_wet_output_frame: Option<u64>,
}

pub struct HostAuthority {
    provider: LocalAudio,
    reads: BTreeMap<u64, ReadState>,
    structural_reads: BTreeMap<u64, u64>,
    descriptor: Option<MediaDescriptor>,
    media_owner: Option<u64>,
    wet_groups: Vec<WetGroup>,
    gates: [WetGate; 2],
    wet: [f64; 96],
    outgoing: VecDeque<Vec<u8>>,
    completions: VecDeque<Value>,
    analysis_indices: Vec<usize>,
    analysis: Vec<f64>,
    fx_send: [f64; 96],
    capability_generation: u64,
    media_stats: HostMediaStats,
    accepted_wet_hash: Sha256,
    observed_source_epoch: u64,
}
impl HostAuthority {
    pub fn new(provider: LocalAudio, capability_generation: u64) -> Result<Self> {
        if capability_generation == 0 {
            return Err("zero capability generation".into());
        }
        let observed_source_epoch = provider.source_epoch();
        Ok(Self {
            provider,
            reads: BTreeMap::new(),
            structural_reads: BTreeMap::new(),
            descriptor: None,
            media_owner: None,
            wet_groups: Vec::new(),
            gates: [WetGate::default(); 2],
            wet: [0.; 96],
            outgoing: VecDeque::with_capacity(256),
            completions: VecDeque::with_capacity(128),
            analysis_indices: Vec::new(),
            analysis: Vec::new(),
            fx_send: [0.; 96],
            capability_generation,
            media_stats: HostMediaStats::default(),
            accepted_wet_hash: Sha256::new(),
            observed_source_epoch,
        })
    }
    fn clear_media_history(&mut self) {
        self.descriptor = None;
        self.media_owner = None;
        self.wet_groups.clear();
        self.gates = [WetGate::default(); 2];
        self.wet.fill(0.);
        self.outgoing.clear();
    }
    pub fn media_stats(&self) -> &HostMediaStats {
        &self.media_stats
    }
    pub fn accepted_wet_sha256(&self) -> String {
        format!("{:x}", self.accepted_wet_hash.clone().finalize())
    }
    pub fn provider(&self) -> &LocalAudio {
        &self.provider
    }
    pub fn provider_mut(&mut self) -> &mut LocalAudio {
        &mut self.provider
    }
    pub fn descriptor(&self) -> Option<&MediaDescriptor> {
        self.descriptor.as_ref()
    }
    /// Source worker entry, not a socket/TLS callback. The only timeline advance
    /// is inside the same production provider used by local/headless adapters.
    pub fn process_source(
        &mut self,
        now_ms: u64,
        epoch: u64,
        frame: u64,
        capture: &[f64],
        playback: &mut [f64],
    ) -> Result<()> {
        playback.fill(0.);
        if self.observed_source_epoch != self.provider.source_epoch() {
            self.clear_media_history();
            self.observed_source_epoch = self.provider.source_epoch();
        }
        if epoch != self.provider.source_epoch() || frame != self.provider.frame() {
            self.clear_media_history();
            self.provider.quiesce_source("source_discontinuity")?;
            return Err("source timeline".into());
        }
        if self.descriptor.as_ref().is_some_and(|d| {
            d.source_epoch.0 != epoch || d.map_generation.0 != self.provider.topology().map_revision
        }) {
            self.clear_media_history();
        }
        let mut incoming = [None; 96];
        for group in &mut self.wet_groups {
            if frame
                < group
                    .first
                    .saturating_add(u64::from(group.spec.delay_frames))
            {
                continue;
            }
            if group
                .buffer
                .next_frame()
                .checked_add(u64::from(group.spec.delay_frames))
                != Some(frame)
            {
                self.provider.quiesce_source("wet_source_timeline")?;
                return Err("wet source timeline".into());
            }
            let mut bytes = [0; crate::transport::MAX_DATAGRAM];
            if let Some(length) = group
                .buffer
                .pop(&mut bytes)
                .map_err(|e| format!("wet queue {e:?}"))?
            {
                let packet =
                    Packet::parse(&bytes[..length]).map_err(|e| format!("wet packet {e:?}"))?;
                for f in 0..48 {
                    for ch in 0..usize::from(group.spec.channels) {
                        incoming[f * 2 + usize::from(group.spec.first_channel) + ch] = Some(
                            packet
                                .sample(f * usize::from(group.spec.channels) + ch)
                                .map_err(|e| format!("wet sample {e:?}"))?,
                        );
                    }
                }
            }
        }
        for f in 0..48 {
            for ch in 0..2 {
                self.wet[f * 2 + ch] = self.gates[ch].step(incoming[f * 2 + ch]);
            }
        }
        for sample in self.wet {
            self.media_stats.nonzero_rendered_wet_samples += u64::from(sample != 0.);
            self.media_stats.rendered_wet_energy += sample * sample;
        }
        // Always supply the explicit external wet path, including silence after
        // disconnect. This prevents fallback to another local FX instance.
        self.provider.tick_with_capture_and_wet(
            now_ms,
            epoch,
            frame,
            capture,
            playback,
            Some(&self.wet),
        )?;
        for reply in self.provider.take_remote_completions() {
            if self.completions.len() == 128 {
                return Err("remote final queue capacity".into());
            }
            self.completions.push_back(reply);
        }
        if let Some(descriptor) = &self.descriptor {
            if descriptor.source_epoch.0 != epoch
                || descriptor.map_generation.0 != self.provider.topology().map_revision
            {
                self.descriptor = None;
                self.wet_groups.clear();
                self.outgoing.clear();
                return Ok(());
            }
            let capture_channels = self.provider.topology().capture_channels;
            for f in 0..48 {
                for (index, slot) in self.analysis_indices.iter().enumerate() {
                    self.analysis[f * self.analysis_indices.len() + index] =
                        capture[f * capture_channels + slot];
                }
            }
            let buses = self.provider.last_bus_samples();
            let width = self.provider.topology().monitors + 2;
            for f in 0..48 {
                self.fx_send[f * 2..f * 2 + 2].copy_from_slice(&buses[f * width..f * width + 2]);
            }
            let mut packets = Vec::new();
            if !self.analysis_indices.is_empty() {
                packets.extend(encode_grouped(
                    descriptor,
                    MediaRole::Analysis,
                    frame,
                    &self.analysis,
                )?);
            }
            if descriptor.channels(MediaRole::FxSend) > 0 {
                packets.extend(encode_grouped(
                    descriptor,
                    MediaRole::FxSend,
                    frame,
                    &self.fx_send,
                )?);
            }
            for packet in packets {
                if self.outgoing.len() == 256 {
                    self.outgoing.pop_front();
                }
                self.outgoing.push_back(packet);
            }
        }
        Ok(())
    }
}
impl AuthorityEndpoint for HostAuthority {
    fn identity(&self) -> EngineIdentity {
        EngineIdentity {
            source_epoch: Counter(self.provider.source_epoch()),
            capability_generation: Counter(self.capability_generation),
            map_generation: Counter(self.provider.topology().map_revision),
        }
    }
    fn dispatch(
        &mut self,
        context: &AuthenticatedContext,
        payload: Value,
        now_ms: u64,
    ) -> Result<Option<Value>> {
        context.check_writer(&payload)?;
        if self.reads.len() >= MAX_CONNECTIONS && !self.reads.contains_key(&context.session()) {
            return Err("remote read-state capacity".into());
        }
        if payload.get("contract").and_then(Value::as_str) == Some("GP05-modules") {
            let request = crate::module_wire::ModuleRequest::decode(&super::encode(&payload)?)?;
            if request.version != self.provider.engine_mut().wire_version() {
                return Err("module version incompatible with admitted topology".into());
            }
            if !matches!(
                request.command,
                crate::module_wire::ModuleCommand::ModuleStatus {}
            ) {
                if !self
                    .reads
                    .get(&context.session())
                    .is_some_and(ReadState::audio_ready)
                {
                    return Err("remote snapshot required before recorder mutation".into());
                }
                context.require(&Permission::Foh)?;
                if self
                    .provider
                    .engine_mut()
                    .writer_scope(&request.authority_request(), now_ms)
                    != Some(crate::control_model::Scope::Foh)
                {
                    return Err("recorder FOH lease required".into());
                }
            }
            return self
                .provider
                .dispatch_module(request, now_ms, None)
                .map(Some);
        }
        if payload.get("contract").and_then(Value::as_str) == Some("GP14-structure") {
            let request = crate::structural_control::Request::decode(&super::encode(&payload)?)?;
            if let Some(scope) = request.scope() {
                context.require(&scope_permission(scope))?;
                if self
                    .provider
                    .engine_mut()
                    .writer_scope(&request.authority_request(), now_ms)
                    != Some(scope)
                {
                    return Err("structural scoped lease required".into());
                }
            }
            let fresh = self
                .structural_reads
                .get(&context.session())
                .is_some_and(|t| now_ms >= *t && now_ms - *t <= 250);
            let snapshot = matches!(
                request.command,
                crate::structural_control::Command::StructuralSnapshot {}
            );
            let reply = self
                .provider
                .structural_request(request, now_ms, fresh, None)?;
            if snapshot && reply.snapshot.is_some() {
                self.structural_reads.insert(context.session(), now_ms);
            }
            return serde_json::to_value(reply)
                .map(Some)
                .map_err(|e| e.to_string());
        }
        let state = self.reads.entry(context.session()).or_default();
        dispatch_engine(self.provider.engine_mut(), context, payload, now_ms, state).map(Some)
    }
    fn negotiate(
        &mut self,
        context: &AuthenticatedContext,
        descriptor: &MediaDescriptor,
    ) -> Result<bool> {
        if self
            .media_owner
            .is_some_and(|session| session != context.session())
        {
            return Err("Brain media owner already attached".into());
        }
        descriptor.validate(context, &self.identity(), crate::transport::MAX_DATAGRAM)?;
        if descriptor.streams.iter().any(|s| s.frames != 48) {
            return Err("composed host currently advertises 48-frame media blocks".into());
        }
        let ordered_ids = |role| {
            let mut streams: Vec<_> = descriptor
                .streams
                .iter()
                .filter(|s| s.role == role)
                .collect();
            streams.sort_by_key(|s| s.first_channel);
            streams
                .into_iter()
                .flat_map(|s| s.channel_ids.iter().cloned())
                .collect::<Vec<_>>()
        };
        let analysis_ids = ordered_ids(MediaRole::Analysis);
        let mut indices = Vec::new();
        for id in &analysis_ids {
            indices.push(
                self.provider
                    .topology()
                    .inputs
                    .iter()
                    .find(|input| &input.id == id)
                    .ok_or("unknown admitted raw source")?
                    .capture_slot,
            );
        }
        for role in [MediaRole::FxSend, MediaRole::WetReturn] {
            let ids = ordered_ids(role);
            if !ids.is_empty() && ids != ["foh-left", "foh-right"] {
                return Err("composed FOH FX ports must be explicit left/right".into());
            }
        }
        let first = self.provider.frame();
        let mut wet_groups = Vec::new();
        for stream in descriptor
            .streams
            .iter()
            .filter(|s| s.role == MediaRole::WetReturn)
        {
            let spec = stream.spec(context.session())?;
            let buffer =
                JitterBuffer::new(spec, first, (first / u64::from(spec.frames)) as u32, 32)
                    .map_err(|e| format!("wet admission {e:?}"))?;
            wet_groups.push(WetGroup {
                spec,
                first,
                buffer,
            });
        }
        self.analysis = vec![0.; 48 * indices.len()];
        self.analysis_indices = indices;
        self.wet_groups = wet_groups;
        self.gates = [WetGate::default(); 2];
        self.wet.fill(0.);
        self.outgoing.clear();
        self.observed_source_epoch = self.provider.source_epoch();
        self.descriptor = Some(descriptor.clone());
        self.media_owner = Some(context.session());
        Ok(true)
    }
    fn media(
        &mut self,
        context: &AuthenticatedContext,
        packet: Packet<'_>,
        _now_ms: u64,
    ) -> Result<()> {
        context.require(&Permission::Fx)?;
        if self.media_owner != Some(context.session()) || packet.spec().role != Role::WetReturn {
            return Err("wet owner/direction".into());
        }
        let group = self
            .wet_groups
            .iter_mut()
            .find(|g| g.spec == packet.spec())
            .ok_or("wet stream identity")?;
        // Deadline/loss statistics belong to the reused GPA1 queue. Drops are
        // expected recovery input, never a source failure or recorder stall.
        match group.buffer.insert_wet_at(packet, self.provider.frame()) {
            Ok(()) => {
                self.media_stats.accepted_wet_packets += 1;
                self.media_stats
                    .first_wet_source_frame
                    .get_or_insert(packet.source_frame());
                self.media_stats.last_wet_source_frame = Some(packet.source_frame());
                self.media_stats
                    .first_wet_output_frame
                    .get_or_insert(packet.output_frame());
                self.media_stats.last_wet_output_frame = Some(packet.output_frame());
                for index in 0..packet.spec().samples() {
                    self.media_stats.nonzero_received_wet_samples +=
                        u64::from(packet.sample(index).is_ok_and(|sample| sample != 0.));
                }
                self.accepted_wet_hash.update(packet.bytes());
            }
            Err(crate::transport::Error::Late) => self.media_stats.late_wet_packets += 1,
            Err(_) => self.media_stats.rejected_wet_packets += 1,
        }
        Ok(())
    }
    fn disconnect(&mut self, context: &AuthenticatedContext) {
        self.provider.revoke_writer(context.writer());
        self.reads.remove(&context.session());
        self.structural_reads.remove(&context.session());
        self.completions.retain(|r| {
            r.get("context")
                .and_then(|c| c.get("writer"))
                .and_then(Value::as_str)
                != Some(context.writer())
        });
        if self.media_owner == Some(context.session()) {
            self.media_owner = None;
            self.descriptor = None;
            self.wet_groups.clear();
            self.outgoing.clear();
        }
        // Keep WetGate history solely for its bounded five-ms fade to silence.
    }
    fn source_frame(&self) -> Option<u64> {
        Some(self.provider.frame())
    }
    fn poll_reply(
        &mut self,
        context: &AuthenticatedContext,
        _now_ms: u64,
    ) -> Result<Option<Value>> {
        let index = self.completions.iter().position(|r| {
            r.get("context")
                .and_then(|c| c.get("writer"))
                .and_then(Value::as_str)
                == Some(context.writer())
        });
        Ok(index.and_then(|index| self.completions.remove(index)))
    }
    fn poll_media(&mut self, context: &AuthenticatedContext) -> Result<Option<Vec<u8>>> {
        if self.media_owner == Some(context.session()) {
            Ok(self.outgoing.pop_front())
        } else {
            Ok(None)
        }
    }
}
