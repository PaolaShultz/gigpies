//! GP03 control-side offline pump. Authority/JSON/heap work stays outside Mixer::process.
//! This is not a realtime multi-thread host integration.
use crate::{
    control_model::{Authority, Command, Edit, Reply, Request, Scope, Snapshot},
    mixer::{INPUTS, Mixer, Prepared},
    show::{Counter, Result},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub input: String,
    /// Unit: linear amplitude gain * 1e9 (rounded), order fader, pan L/R,
    /// shared mute, send1/2. Not millidecibels.
    pub current_nanogain: [u64; 6],
    pub ramp_target_nanogain: [u64; 6],
    pub held_nanogain: [Option<u64>; 6],
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RenderedSnapshot {
    pub capability: String,
    pub capability_version: u32,
    pub rendered_application: bool,
    pub release_commit: bool,
    pub frame: Counter,
    pub faulted: bool,
    pub protection: String,
    pub meters: Option<Vec<i32>>,
    /// Unchanged GP02 target metadata. Its unavailable actual values are deliberate.
    pub authority: Snapshot,
    pub coefficients: Vec<Observation>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequestContext {
    pub show_id: String,
    pub module: String,
    pub epoch: Counter,
    pub writer: Option<String>,
    pub lease: Option<Counter>,
    pub request_id: Option<Counter>,
    pub expected_revision: Option<Counter>,
}
impl RequestContext {
    fn request(r: &Request) -> Self {
        Self {
            show_id: r.show_id.clone(),
            module: r.module.clone(),
            epoch: r.epoch,
            writer: r.writer.clone(),
            lease: r.lease,
            request_id: r.request_id,
            expected_revision: r.expected_revision,
        }
    }
    fn reply(r: &Reply) -> Self {
        Self {
            show_id: r.show_id.clone(),
            module: r.module.clone(),
            epoch: r.epoch,
            writer: r.writer.clone(),
            lease: r.lease,
            request_id: r.request_id,
            expected_revision: r.expected_revision,
        }
    }
    fn validate(&self) -> Result<()> {
        let command = if self.writer.is_none() {
            Command::Snapshot {}
        } else if self.lease.is_none() {
            Command::Grant { scope: Scope::Foh }
        } else {
            Command::Renew {}
        };
        Request {
            contract: "C-AUDIO".into(),
            version: 1,
            show_id: self.show_id.clone(),
            module: self.module.clone(),
            epoch: self.epoch,
            writer: self.writer.clone(),
            lease: self.lease,
            request_id: self.request_id,
            expected_revision: self.expected_revision,
            command,
        }
        .encode()?;
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RenderedReply {
    pub context: RequestContext,
    pub capability: String,
    pub capability_version: u32,
    pub state: String,
    pub ticket: Option<Counter>,
    pub effective_frame: Option<Counter>,
    pub ramp_frames: Option<u32>,
    /// Absent while pending. Applied only after authority + DSP commit.
    pub outcome: Option<Reply>,
    pub snapshot: Option<RenderedSnapshot>,
}
impl RenderedSnapshot {
    pub fn validate(&self) -> Result<()> {
        self.authority.validate()?;
        if self.capability != "GP03-rendered"
            || self.capability_version != 1
            || !self.rendered_application
            || !self.release_commit
            || self.protection != "offline-unprotected"
            || self.meters.is_some()
            || self.coefficients.len() != 8
        {
            return Err("rendered snapshot capability".into());
        }
        for (i, observation) in self.coefficients.iter().enumerate() {
            if observation.input != format!("input-{:02}", i + 1) {
                return Err("rendered inventory".into());
            }
            for j in 0..6 {
                let max = if j == 0 || j >= 4 {
                    3_981_071_706
                } else {
                    1_000_000_000
                };
                if observation.current_nanogain[j] > max
                    || observation.ramp_target_nanogain[j] > max
                    || observation.held_nanogain[j].is_some_and(|v| v > max)
                {
                    return Err("rendered coefficient".into());
                }
            }
        }
        Ok(())
    }
}
impl RenderedReply {
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let reply: Self = crate::show::decode(bytes)?;
        reply.validate()?;
        Ok(reply)
    }
    pub fn validate(&self) -> Result<()> {
        if self.capability != "GP03-rendered" || self.capability_version != 1 {
            return Err("rendered capability".into());
        }
        self.context.validate()?;
        match self.state.as_str() {
            "backpressure"
                if self.ticket.is_none()
                    && self.effective_frame.is_none()
                    && self.ramp_frames.is_none()
                    && self.outcome.is_none()
                    && self.snapshot.is_none() => {}
            "pending"
                if self.context.lease.is_some()
                    && self.ticket.is_some_and(|v| v.0 > 0)
                    && self.effective_frame.is_some_and(|v| v.0 % 48 == 0)
                    && self.ramp_frames == Some(240)
                    && self.outcome.is_none()
                    && self.snapshot.is_none() => {}
            "final" => {
                let outcome = self.outcome.as_ref().ok_or("final outcome")?;
                if self.context != RequestContext::reply(outcome) {
                    return Err("reply correlation".into());
                }
                outcome.validate()?;
                if self.ticket.is_some() {
                    if self.ticket.is_some_and(|v| v.0 == 0)
                        || self.effective_frame.is_none_or(|v| v.0 % 48 != 0)
                        || self.ramp_frames != Some(240)
                        || outcome.kind != "applied"
                        || self.snapshot.is_none()
                    {
                        return Err("rendered timing".into());
                    }
                } else if self.effective_frame.is_some() || self.ramp_frames.is_some() {
                    return Err("rendered timing".into());
                }
                if let Some(snapshot) = &self.snapshot {
                    snapshot.validate()?;
                    if outcome.kind != "applied"
                        || snapshot.authority.revision != outcome.body.revision
                        || snapshot.authority.show_id != outcome.show_id
                        || snapshot.authority.epoch != outcome.epoch
                        || self
                            .effective_frame
                            .is_some_and(|f| f.0 >= snapshot.frame.0)
                    {
                        return Err("rendered snapshot binding".into());
                    }
                    if let Some(inner) = &outcome.body.snapshot
                        && inner != &snapshot.authority
                    {
                        return Err("snapshot coherence".into());
                    }
                }
            }
            _ => return Err("rendered reply state".into()),
        }
        Ok(())
    }
    pub fn encode(&self) -> Result<Vec<u8>> {
        self.validate()?;
        let bytes = serde_json::to_vec(self).map_err(|e| e.to_string())?;
        if bytes.len() > 65536 {
            return Err("capacity".into());
        }
        Ok(bytes)
    }
}
#[derive(Debug)]
struct Pending {
    request: Request,
    staged: Authority,
    reply: Reply,
    ticket: u64,
    frame: u64,
    scope: Scope,
}
#[derive(Debug)]
pub struct OfflineEngine {
    authority: Authority,
    mixer: Mixer,
    pending: Option<Pending>,
    next_ticket: u64,
    outcomes: BTreeMap<String, VecDeque<(Request, RenderedReply)>>,
    holds: [[Option<u64>; 6]; INPUTS],
    module_ids: BTreeMap<String, VecDeque<(Request, String, Option<String>)>>,
}
impl OfflineEngine {
    pub fn new(show: &str, epoch: Counter, revision: Counter, frame: u64) -> Result<Self> {
        let mut authority = Authority::new(show, epoch, revision)?;
        authority.enable_rendered_release();
        Ok(Self {
            authority,
            mixer: Mixer::new(frame),
            pending: None,
            next_ticket: 1,
            outcomes: BTreeMap::new(),
            holds: [[None; 6]; INPUTS],
            module_ids: BTreeMap::new(),
        })
    }
    pub fn frame(&self) -> u64 {
        self.mixer.frame()
    }
    pub fn revision(&self) -> Counter {
        self.authority.revision()
    }
    pub fn mixer(&self) -> &Mixer {
        &self.mixer
    }
    fn wrapped(reply: Reply) -> RenderedReply {
        RenderedReply {
            context: RequestContext::reply(&reply),
            capability: "GP03-rendered".into(),
            capability_version: 1,
            state: "final".into(),
            ticket: None,
            effective_frame: None,
            ramp_frames: None,
            outcome: Some(reply),
            snapshot: None,
        }
    }
    fn backpressure(r: &Request) -> RenderedReply {
        RenderedReply {
            context: RequestContext::request(r),
            capability: "GP03-rendered".into(),
            capability_version: 1,
            state: "backpressure".into(),
            ticket: None,
            effective_frame: None,
            ramp_frames: None,
            outcome: None,
            snapshot: None,
        }
    }
    fn remember(&mut self, r: Request, reply: RenderedReply) {
        let cache = self.outcomes.entry(r.writer.clone().unwrap()).or_default();
        cache.push_back((r, reply));
        if cache.len() > 64 {
            cache.pop_front();
        }
    }
    fn pending_reply(p: &Pending) -> RenderedReply {
        RenderedReply {
            context: RequestContext::request(&p.request),
            capability: "GP03-rendered".into(),
            capability_version: 1,
            state: "pending".into(),
            ticket: Some(Counter(p.ticket)),
            effective_frame: Some(Counter(p.frame)),
            ramp_frames: Some(240),
            outcome: None,
            snapshot: None,
        }
    }
    pub fn snapshot(&mut self) -> Result<RenderedSnapshot> {
        let current = self.mixer.coefficients();
        let target = self.mixer.targets();
        Ok(RenderedSnapshot {
            capability: "GP03-rendered".into(),
            capability_version: 1,
            rendered_application: true,
            release_commit: true,
            frame: Counter(self.frame()),
            faulted: self.mixer.faulted(),
            protection: "offline-unprotected".into(),
            meters: None,
            authority: self.authority.snapshot()?,
            coefficients: (0..INPUTS)
                .map(|i| Observation {
                    input: format!("input-{:02}", i + 1),
                    current_nanogain: current[i].map(nano),
                    ramp_target_nanogain: target[i].map(nano),
                    held_nanogain: self.holds[i],
                })
                .collect(),
        })
    }
    /// GP05 admission shares the authority's session high-water with GP03.
    /// Fingerprints include contract/kind/body. A private Renew reserves the ID;
    /// GP03 is fenced from replaying that internal reservation below. No graph
    /// mutation/revision occurs, and lifecycle completion remains provider-owned.
    pub fn authorize_module(
        &mut self,
        r: &Request,
        fingerprint: &str,
        now: u64,
    ) -> Result<(bool, Option<String>)> {
        r.encode()?;
        let snapshot = self.authority.snapshot()?;
        if r.show_id != snapshot.show_id
            || r.epoch != snapshot.epoch
            || self.authority.live_scope(r, now) != Some(Scope::Foh)
            || !matches!(r.command, Command::Renew {})
        {
            return Err("module authority identity/lease/scope".into());
        }
        self.module_ids
            .retain(|w, _| self.authority.writer_live(w, now));
        let writer = r.writer.as_ref().ok_or("writer")?;
        if let Some((old, hash, reason)) = self
            .module_ids
            .get(writer)
            .and_then(|v| v.iter().find(|(old, _, _)| old.request_id == r.request_id))
        {
            if self.authority.cached(r, now).is_none() {
                return Err("expired_id".into());
            }
            return if old == r && hash == fingerprint {
                Ok((true, reason.clone()))
            } else {
                Err("reused_id".into())
            };
        }
        // An earlier GP03 Renew with the same identity must not be interpreted
        // as a module admission. Other collisions are rejected by Authority.
        if self.authority.cached(r, now).is_some() {
            return Err("reused_id".into());
        }
        if self.pending.is_some() {
            return Err("backpressure".into());
        }
        let reply = self.authority.handle(r, now);
        let reason = if reply.kind == "applied" {
            None
        } else {
            Some(reply.body.reason.unwrap_or(reply.kind))
        };
        // Refusals consuming the shared ID (such as stale_revision) retain the
        // original GP05 fingerprint/outcome, just like successful admissions.
        if self.authority.cached(r, now).is_none() {
            return Err(reason.unwrap_or_else(|| "admission".into()));
        }
        let cache = self.module_ids.entry(writer.clone()).or_default();
        cache.push_back((r.clone(), fingerprint.into(), reason.clone()));
        if cache.len() > 64 {
            cache.pop_front();
        }
        Ok((false, reason))
    }
    pub fn handle_encoded(&mut self, bytes: &[u8], now_ms: u64) -> Result<Vec<u8>> {
        self.handle(&Request::decode(bytes)?, now_ms)?.encode()
    }
    pub fn handle(&mut self, r: &Request, now: u64) -> Result<RenderedReply> {
        // Decode validation also protects callers using the typed public entry.
        r.encode()?;
        self.module_ids
            .retain(|w, _| self.authority.writer_live(w, now));
        if self.authority.live_scope(r, now).is_some()
            && self
                .module_ids
                .get(r.writer.as_deref().unwrap_or(""))
                .is_some_and(|v| v.iter().any(|(old, _, _)| old.request_id == r.request_id))
        {
            return Ok(Self::wrapped(self.authority.reply(
                r,
                "rejected",
                Some("reused_id"),
            )));
        }
        if self.authority.observe_control_time(now).is_err() {
            return Ok(Self::wrapped(self.authority.reply(
                r,
                "rejected",
                Some("clock"),
            )));
        }
        self.outcomes
            .retain(|writer, _| self.authority.writer_live(writer, now));
        if let Some(cached) = self.authority.cached(r, now) {
            if let Some((_, reply)) = self
                .outcomes
                .get(r.writer.as_ref().unwrap())
                .and_then(|cache| cache.iter().find(|(old, _)| old == r))
            {
                return Ok(reply.clone());
            }
            return Ok(Self::wrapped(cached));
        }
        if let Some(p) = &self.pending {
            // Identity and current lease validity precede pending retry identity.
            // A retry after expiry must not reveal a pending ticket or become a
            // misleading reused-ID refusal.
            if !matches!(r.command, Command::Snapshot {} | Command::Grant { .. })
                && self.authority.live_scope(r, now).is_none()
            {
                let mut check = self.authority.clone();
                return Ok(Self::wrapped(check.handle(r, now)));
            }
            if p.request == *r && self.authority.live_scope(r, now).is_some() {
                return Ok(Self::pending_reply(p));
            }
            if p.request.writer == r.writer
                && p.request.request_id == r.request_id
                && p.request.lease == r.lease
                && p.request.show_id == r.show_id
                && p.request.epoch == r.epoch
            {
                return Ok(Self::wrapped(self.authority.reply(
                    r,
                    "rejected",
                    Some("reused_id"),
                )));
            }
            // No metadata mutation may overwrite a staged authority. Read snapshots
            // observe only the committed authority/graph, never the staged state.
            if !matches!(r.command, Command::Snapshot {}) {
                let mut check = self.authority.clone();
                let reply = check.handle(r, now);
                if matches!(
                    reply.body.reason.as_deref(),
                    Some("wrong_show" | "epoch" | "lease" | "clock" | "expired_id" | "reused_id")
                ) {
                    return Ok(Self::wrapped(reply));
                }
                return Ok(Self::backpressure(r));
            }
        }
        if matches!(r.command, Command::Snapshot {}) {
            let reply = self.authority.handle(r, now);
            let mut wrapped = Self::wrapped(reply);
            if wrapped
                .outcome
                .as_ref()
                .is_some_and(|p| p.kind == "applied")
            {
                let snapshot = self.snapshot()?;
                wrapped.outcome.as_mut().unwrap().body.snapshot = Some(snapshot.authority.clone());
                wrapped.snapshot = Some(snapshot);
            }
            return Ok(wrapped);
        }
        let graph = matches!(
            r.command,
            Command::Set { .. } | Command::SetMode { .. } | Command::ReleasePreview { .. }
        );
        if !graph {
            return Ok(Self::wrapped(self.authority.handle(r, now)));
        }
        let mut staged = self.authority.clone();
        let reply = staged.handle(r, now);
        if reply.kind != "applied" {
            return Ok(Self::wrapped(self.authority.handle(r, now)));
        }
        // Cached retry can be applied without advancing revision. Never reschedule.
        if staged.revision() == self.authority.revision() {
            return Ok(Self::wrapped(reply));
        }
        let scope = self.authority.live_scope(r, now).ok_or("lease")?;
        let prepared = match &r.command {
            Command::Set { targets } => Prepared::edits(targets)?,
            Command::SetMode { .. } => Prepared::freeze(scope),
            Command::ReleasePreview { .. } => {
                let (old, _) = self.authority.state();
                let (new, _) = staged.state();
                let edits: Vec<_> = new
                    .iter()
                    .zip(old)
                    .filter(|(n, o)| n.hold.is_none() && o.hold.is_some())
                    .map(|(n, _)| Edit {
                        target: n.target.clone(),
                        value: n.target_value.clone(),
                    })
                    .collect();
                Prepared::edits(&edits)?
            }
            _ => unreachable!(),
        };
        let Some(next) = self.next_ticket.checked_add(1) else {
            return Ok(Self::backpressure(r));
        };
        let frame = match self.mixer.schedule(prepared, self.next_ticket) {
            Ok(frame) => frame,
            Err(_) => return Ok(Self::backpressure(r)),
        };
        let p = Pending {
            request: r.clone(),
            staged,
            reply,
            ticket: self.next_ticket,
            frame,
            scope,
        };
        self.next_ticket = next;
        let reply = Self::pending_reply(&p);
        self.pending = Some(p);
        Ok(reply)
    }
    /// Offline control pump: split at pending boundary, validate lease before its
    /// sample, call fixed renderer, then commit metadata outside process. Injected
    /// now_ms is monotonic authority time, not the audio frame counter.
    pub fn process(
        &mut self,
        input: &[[f64; INPUTS]],
        output: &mut [[f64; 4]],
        now_ms: u64,
    ) -> Result<Vec<RenderedReply>> {
        if input.len() != output.len() {
            return Err("buffer length".into());
        }
        self.authority.observe_control_time(now_ms)?;
        let end = self
            .frame()
            .checked_add(input.len() as u64)
            .ok_or("clock exhausted")?;
        let mut replies = Vec::new();
        let split = self
            .pending
            .as_ref()
            .filter(|p| p.frame < end)
            .map(|p| (p.frame - self.frame()) as usize);
        if let Some(split) = split {
            self.mixer
                .process(&input[..split], &mut output[..split])
                .map_err(|e| format!("{e:?}"))?;
            if let Some(p) = &mut self.pending {
                // Re-run authority validation at commit, before any DSP application.
                // No mutating requests were admitted while pending, so only time and
                // observation sequence can differ. This validates preview expiry too.
                let mut current = self.authority.clone();
                let reply = current.handle(&p.request, now_ms);
                if reply.kind != "applied" {
                    let p = self.pending.take().unwrap();
                    self.mixer.cancel(p.ticket);
                    self.authority = current;
                    let final_reply = Self::wrapped(reply);
                    self.remember(p.request, final_reply.clone());
                    replies.push(final_reply);
                } else {
                    p.staged = current;
                    p.reply = reply;
                }
            }
            // Render exactly the boundary sample, then drain primitive completion.
            self.mixer
                .process(&input[split..split + 1], &mut output[split..split + 1])
                .map_err(|e| format!("{e:?}"))?;
            if let Some(done) = self.mixer.take_completion() {
                let mut p = self.pending.take().ok_or("completion without pending")?;
                if done.ticket != p.ticket || done.frame != p.frame {
                    return Err("completion identity".into());
                }
                self.update_holds(&p);
                p.reply.body.ramp_frames = None;
                p.staged.install_rendered_reply(&p.request, &p.reply);
                p.staged.preserve_observations(&self.authority);
                self.authority = p.staged;
                let mut reply = Self::wrapped(p.reply);
                reply.ticket = Some(Counter(p.ticket));
                reply.effective_frame = Some(Counter(p.frame));
                reply.ramp_frames = Some(240);
                reply.snapshot = Some(self.snapshot()?);
                self.remember(p.request, reply.clone());
                replies.push(reply);
            }
            self.mixer
                .process(&input[split + 1..], &mut output[split + 1..])
                .map_err(|e| format!("{e:?}"))?;
        } else {
            self.mixer
                .process(input, output)
                .map_err(|e| format!("{e:?}"))?;
        }
        Ok(replies)
    }
    fn update_holds(&mut self, p: &Pending) {
        let (new, _) = p.staged.state();
        let c = self.mixer.coefficients();
        let targets = self.mixer.targets();
        for (i, row) in self.holds.iter_mut().enumerate() {
            if matches!(p.request.command, Command::SetMode { .. }) {
                let indices: &[usize] = match p.scope {
                    Scope::Foh => &[0, 1, 2, 3],
                    Scope::Monitor1 => &[4],
                    Scope::Monitor2 => &[5],
                };
                for &j in indices {
                    row[j] = Some(nano(c[i][j]));
                }
            } else {
                for parameter in new.iter().filter(|v| match &v.target {
                    crate::control_model::Target::Fader { input }
                    | crate::control_model::Target::Pan { input }
                    | crate::control_model::Target::Mute { input }
                    | crate::control_model::Target::Send { input, .. } => {
                        input == &format!("input-{:02}", i + 1)
                    }
                }) {
                    let indices: &[usize] = match &parameter.target {
                        crate::control_model::Target::Fader { .. } => &[0],
                        crate::control_model::Target::Pan { .. } => &[1, 2],
                        crate::control_model::Target::Mute { .. } => &[3],
                        crate::control_model::Target::Send { monitor, .. }
                            if monitor == "monitor-1" =>
                        {
                            &[4]
                        }
                        _ => &[5],
                    };
                    for &j in indices {
                        if parameter.hold.is_none() {
                            row[j] = None;
                        } else if row[j].is_none()
                            || matches!(&p.request.command,Command::Set {targets} if targets.iter().any(|e|e.target==parameter.target))
                        {
                            row[j] = Some(nano(targets[i][j]));
                        }
                    }
                }
            }
        }
    }
}
fn nano(gain: f64) -> u64 {
    (gain * 1_000_000_000.0).round() as u64
}
