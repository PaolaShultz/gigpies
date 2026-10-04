//! C-AUDIO:1 encoded, pure authority harness. No DSP, device or rendered ACK.
use crate::show::{Counter, Result, decode, id};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub const LEASE_MS: u64 = 2000;
pub const SESSION_HISTORY_CAPACITY: usize = 1024;
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    Foh,
    Monitor1,
    Monitor2,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Manual,
    Assist,
    Auto,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "parameter", deny_unknown_fields, rename_all = "snake_case")]
pub enum Target {
    Fader { input: String },
    Pan { input: String },
    Mute { input: String },
    Send { input: String, monitor: String },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    Integer(i32),
    Boolean(bool),
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Edit {
    pub target: Target,
    pub value: Value,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AutoBound {
    pub target: Target,
    pub min: Value,
    pub max: Value,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "body",
    deny_unknown_fields,
    rename_all = "snake_case"
)]
pub enum Command {
    Snapshot {},
    Grant { scope: Scope },
    Renew {},
    Release {},
    Set { targets: Vec<Edit> },
    SetMode { mode: Mode, bounds: Vec<AutoBound> },
    Propose { targets: Vec<Edit> },
    PreviewRelease { targets: Vec<Target> },
    CancelPreview { token: String },
    ReleasePreview { token: String },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Request {
    pub contract: String,
    pub version: u32,
    pub show_id: String,
    pub module: String,
    pub epoch: Counter,
    pub writer: Option<String>,
    pub lease: Option<Counter>,
    pub request_id: Option<Counter>,
    pub expected_revision: Option<Counter>,
    #[serde(flatten)]
    pub command: Command,
}
impl Request {
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        // Flatten + deny_unknown_fields is not supported reliably by serde;
        // check the complete envelope keys before typed decoding as well.
        let raw: serde_json::Value = decode(bytes)?;
        let obj = raw.as_object().ok_or("envelope")?;
        let keys = [
            "contract",
            "version",
            "show_id",
            "module",
            "epoch",
            "writer",
            "lease",
            "request_id",
            "expected_revision",
            "kind",
            "body",
        ];
        if obj.len() != keys.len() || obj.keys().any(|k| !keys.contains(&k.as_str())) {
            return Err("envelope fields".into());
        }
        let r: Self = serde_json::from_value(raw).map_err(|e| e.to_string())?;
        r.validate()?;
        Ok(r)
    }
    pub fn encode(&self) -> Result<Vec<u8>> {
        self.validate()?;
        let b = serde_json::to_vec(self).map_err(|e| e.to_string())?;
        Self::decode(&b)?;
        Ok(b)
    }
    fn validate(&self) -> Result<()> {
        if self.contract != "C-AUDIO" || self.version != 1 {
            return Err("version".into());
        }
        if !canonical_show(&self.show_id)
            || self.module != "audio"
            || self.epoch.0 == 0
            || self.writer.as_ref().is_some_and(|w| !id(w))
        {
            return Err("identity".into());
        }
        match self.command {
            Command::Snapshot {}
                if self.writer.is_none()
                    && self.lease.is_none()
                    && self.request_id.is_none()
                    && self.expected_revision.is_none() => {}
            Command::Grant { .. }
                if self.writer.is_some()
                    && self.lease.is_none()
                    && self.request_id.is_some_and(|n| n.0 > 0)
                    && self.expected_revision.is_some() => {}
            Command::Snapshot {} | Command::Grant { .. } => return Err("authority fields".into()),
            _ if self.writer.is_some()
                && self.lease.is_some_and(|n| n.0 > 0)
                && self.request_id.is_some_and(|n| n.0 > 0)
                && self.expected_revision.is_some() => {}
            _ => return Err("authority fields".into()),
        }
        match &self.command {
            Command::Set { targets } | Command::Propose { targets } => {
                for e in targets {
                    e.target.validate_identity()?;
                }
            }
            Command::SetMode { bounds, .. } => {
                for b in bounds {
                    b.target.validate_identity()?;
                }
            }
            Command::PreviewRelease { targets } => {
                for t in targets {
                    t.validate_identity()?;
                }
            }
            Command::CancelPreview { token } | Command::ReleasePreview { token } if !id(token) => {
                return Err("identity".into());
            }
            _ => (),
        }
        Ok(())
    }
}
fn canonical_show(s: &str) -> bool {
    s.len() == 36
        && s.bytes().enumerate().all(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                b == b'-'
            } else {
                b.is_ascii_digit() || (b'a'..=b'f').contains(&b)
            }
        })
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Parameter {
    pub target: Target,
    pub actual: Option<Value>,
    pub target_value: Value,
    pub proposal: Option<Value>,
    pub hold: Option<Value>,
    pub owner: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Preview {
    pub token: String,
    pub revision: Counter,
    pub scope: Scope,
    pub remaining_ms: u32,
    pub ramp_frames: u32,
    pub destinations: Vec<Edit>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub show_id: String,
    pub epoch: Counter,
    pub revision: Counter,
    pub sequence: Counter,
    pub page: u8,
    pub page_count: u8,
    pub durability: String,
    pub validity: String,
    pub acquisition_frame: Option<Counter>,
    pub age_ms: Option<u32>,
    pub rendered_application: bool,
    pub release_commit: bool,
    pub session_history_capacity: u32,
    pub inputs: Vec<String>,
    pub monitors: Vec<String>,
    pub modes: Vec<(Scope, Mode)>,
    pub automation_bounds: Vec<AutoBound>,
    pub parameters: Vec<Parameter>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplyBody {
    pub revision: Counter,
    pub reason: Option<String>,
    pub lease_remaining_ms: Option<u32>,
    pub granted_lease: Option<Counter>,
    pub scope: Option<Scope>,
    pub preview: Option<Preview>,
    pub snapshot: Option<Snapshot>,
    pub effective_frame: Option<Counter>,
    pub ramp_frames: Option<u32>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reply {
    pub contract: String,
    pub version: u32,
    pub show_id: String,
    pub module: String,
    pub epoch: Counter,
    pub writer: Option<String>,
    pub lease: Option<Counter>,
    pub request_id: Option<Counter>,
    pub expected_revision: Option<Counter>,
    pub kind: String,
    pub body: ReplyBody,
}
impl Reply {
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let r: Self = decode(bytes)?;
        r.validate()?;
        Ok(r)
    }
    pub fn encode(&self) -> Result<Vec<u8>> {
        self.validate()?;
        let b = serde_json::to_vec(self).map_err(|e| e.to_string())?;
        if b.len() > 65536 {
            return Err("capacity".into());
        }
        Ok(b)
    }
}
#[derive(Debug, Clone)]
struct Session {
    scope: Scope,
    lease: Counter,
    expires: u64,
    high: u64,
    cache: VecDeque<(Request, Reply)>,
}
#[derive(Debug, Clone)]
struct StoredPreview {
    preview: Preview,
    writer: String,
    lease: Counter,
    expires: u64,
}
/// Injected monotonic milliseconds; all operations synchronous, at most one mutation
/// per writer. Pending DSP work is unavailable, rather than falsely acknowledged.
#[derive(Debug, Clone)]
pub struct Authority {
    show: String,
    epoch: Counter,
    revision: Counter,
    now: u64,
    next_lease: u64,
    next_preview: u64,
    sequence: u64,
    sessions: BTreeMap<String, Session>,
    retired: BTreeSet<String>,
    previews: BTreeMap<String, StoredPreview>,
    parameters: Vec<Parameter>,
    modes: BTreeMap<Scope, Mode>,
    automation_bounds: BTreeMap<Scope, Vec<AutoBound>>,
    rendered_release: bool,
}
impl Target {
    fn validate_identity(&self) -> Result<()> {
        if !id(self.input()) {
            return Err("identity".into());
        }
        if let Self::Send { monitor, .. } = self
            && !id(monitor)
        {
            return Err("identity".into());
        }
        Ok(())
    }
    pub(crate) fn scope(&self) -> Result<Scope> {
        match self {
            Self::Send { monitor, .. } => match monitor.as_str() {
                "monitor-1" => Ok(Scope::Monitor1),
                "monitor-2" => Ok(Scope::Monitor2),
                _ => Err("target".into()),
            },
            _ => Ok(Scope::Foh),
        }
    }
    fn input(&self) -> &str {
        match self {
            Self::Fader { input }
            | Self::Pan { input }
            | Self::Mute { input }
            | Self::Send { input, .. } => input,
        }
    }
    fn value_valid(&self, v: &Value) -> bool {
        match (self, v) {
            (Self::Mute { .. }, Value::Boolean(_)) => true,
            (Self::Pan { .. }, Value::Integer(n)) => (-100..=100).contains(n),
            (Self::Fader { .. } | Self::Send { .. }, Value::Integer(n)) => {
                (-60000..=12000).contains(n) && n % 100 == 0
            }
            _ => false,
        }
    }
}
impl Authority {
    pub fn new(show_id: &str, epoch: Counter, revision: Counter) -> Result<Self> {
        if !canonical_show(show_id) || epoch.0 == 0 {
            return Err("identity".into());
        }
        let mut parameters = Vec::new();
        for i in 1..=8 {
            let input = format!("input-{i:02}");
            for (target, value) in [
                (
                    Target::Fader {
                        input: input.clone(),
                    },
                    Value::Integer(-6000),
                ),
                (
                    Target::Pan {
                        input: input.clone(),
                    },
                    Value::Integer(0),
                ),
                (
                    Target::Mute {
                        input: input.clone(),
                    },
                    Value::Boolean(false),
                ),
                (
                    Target::Send {
                        input: input.clone(),
                        monitor: "monitor-1".into(),
                    },
                    Value::Integer(0),
                ),
                (
                    Target::Send {
                        input,
                        monitor: "monitor-2".into(),
                    },
                    Value::Integer(-60000),
                ),
            ] {
                parameters.push(Parameter {
                    target,
                    actual: None,
                    target_value: value,
                    proposal: None,
                    hold: None,
                    owner: None,
                });
            }
        }
        Ok(Self {
            show: show_id.into(),
            epoch,
            revision,
            now: 0,
            next_lease: 1,
            next_preview: 1,
            sequence: 0,
            sessions: BTreeMap::new(),
            retired: BTreeSet::new(),
            previews: BTreeMap::new(),
            parameters,
            automation_bounds: BTreeMap::new(),
            rendered_release: false,
            modes: [
                (Scope::Foh, Mode::Manual),
                (Scope::Monitor1, Mode::Manual),
                (Scope::Monitor2, Mode::Manual),
            ]
            .into(),
        })
    }
    pub(crate) fn observe_control_time(&mut self, now: u64) -> Result<()> {
        if now < self.now {
            return Err("clock".into());
        }
        self.now = now;
        Ok(())
    }
    pub(crate) fn preserve_observations(&mut self, live: &Self) {
        self.now = self.now.max(live.now);
        self.sequence = self.sequence.max(live.sequence);
    }
    pub(crate) fn enable_rendered_release(&mut self) {
        self.rendered_release = true;
    }
    pub(crate) fn writer_live(&self, writer: &str, now: u64) -> bool {
        self.sessions.get(writer).is_some_and(|s| s.expires > now)
    }
    pub(crate) fn live_scope(&self, r: &Request, now: u64) -> Option<Scope> {
        let session = self.sessions.get(r.writer.as_ref()?)?;
        (now >= self.now && session.expires > now && r.lease == Some(session.lease))
            .then_some(session.scope)
    }
    pub(crate) fn cached(&self, r: &Request, now: u64) -> Option<Reply> {
        self.live_scope(r, now)?;
        self.sessions
            .get(r.writer.as_ref()?)?
            .cache
            .iter()
            .find(|(old, _)| old == r)
            .map(|(_, p)| p.clone())
    }
    pub(crate) fn install_rendered_reply(&mut self, r: &Request, p: &Reply) {
        if let Some(s) = r.writer.as_ref().and_then(|w| self.sessions.get_mut(w))
            && let Some((_, reply)) = s.cache.iter_mut().find(|(old, _)| old == r)
        {
            *reply = p.clone();
        }
    }
    pub fn revision(&self) -> Counter {
        self.revision
    }
    pub fn state(&self) -> (&[Parameter], &BTreeMap<Scope, Mode>) {
        (&self.parameters, &self.modes)
    }
    pub(crate) fn reply(&self, r: &Request, kind: &str, reason: Option<&str>) -> Reply {
        Reply {
            contract: "C-AUDIO".into(),
            version: 1,
            show_id: self.show.clone(),
            module: "audio".into(),
            epoch: self.epoch,
            writer: r.writer.clone(),
            lease: r.lease,
            request_id: r.request_id,
            expected_revision: r.expected_revision,
            kind: kind.into(),
            body: ReplyBody {
                revision: self.revision,
                reason: reason.map(str::to_owned),
                lease_remaining_ms: None,
                granted_lease: None,
                scope: None,
                preview: None,
                snapshot: None,
                effective_frame: None,
                ramp_frames: None,
            },
        }
    }
    pub fn handle_encoded(&mut self, bytes: &[u8], now_ms: u64) -> Result<Vec<u8>> {
        let r = Request::decode(bytes)?;
        self.handle(&r, now_ms).encode()
    }
    pub fn snapshot(&mut self) -> Result<Snapshot> {
        let sequence = self.sequence.checked_add(1).ok_or("sequence exhausted")?;
        self.sequence = sequence;
        Ok(Snapshot {
            show_id: self.show.clone(),
            epoch: self.epoch,
            revision: self.revision,
            sequence: Counter(sequence),
            page: 0,
            page_count: 1,
            durability: "volatile".into(),
            validity: "unavailable".into(),
            acquisition_frame: None,
            age_ms: None,
            rendered_application: false,
            release_commit: false,
            session_history_capacity: SESSION_HISTORY_CAPACITY as u32,
            inputs: (1..=8).map(|i| format!("input-{i:02}")).collect(),
            monitors: vec!["monitor-1".into(), "monitor-2".into()],
            modes: self.modes.iter().map(|(s, m)| (*s, *m)).collect(),
            automation_bounds: self.automation_bounds.values().flatten().cloned().collect(),
            parameters: self.parameters.clone(),
        })
    }
    pub fn handle(&mut self, r: &Request, now_ms: u64) -> Reply {
        if r.validate().is_err() {
            return self.reply(r, "rejected", Some("version"));
        }
        if r.show_id != self.show {
            return self.reply(r, "rejected", Some("wrong_show"));
        }
        if r.epoch != self.epoch {
            return self.reply(r, "rejected", Some("epoch"));
        }
        if now_ms < self.now {
            return self.reply(r, "rejected", Some("clock"));
        }
        self.now = now_ms;
        if matches!(r.command, Command::Snapshot {}) {
            return match self.snapshot() {
                Ok(s) => {
                    let mut p = self.reply(r, "applied", None);
                    p.body.snapshot = Some(s);
                    p
                }
                Err(_) => self.reply(r, "rejected", Some("capacity")),
            };
        }
        let writer = r.writer.as_ref().expect("validated");
        if matches!(r.command, Command::Grant { .. }) && !self.sessions.contains_key(writer) {
            if self.retired.contains(writer) {
                return self.reply(r, "rejected", Some("lease"));
            }
            if r.request_id != Some(Counter(1)) {
                return self.reply(r, "rejected", Some("range"));
            }
            if self.sessions.len() + self.retired.len() >= SESSION_HISTORY_CAPACITY {
                return self.reply(r, "busy", Some("capacity"));
            }
            if r.expected_revision != Some(self.revision) {
                return self.reply(r, "conflict", Some("stale_revision"));
            }
            let Command::Grant { scope } = r.command else {
                unreachable!()
            };
            if self
                .sessions
                .values()
                .any(|s| s.expires > now_ms && s.scope == scope)
            {
                return self.reply(r, "busy", Some("scope"));
            }
            // Retire expired sessions so capacity measures simultaneous live writers.
            let expired: Vec<_> = self
                .sessions
                .iter()
                .filter(|(_, s)| s.expires <= now_ms)
                .map(|(w, _)| w.clone())
                .collect();
            if self.sessions.len() - expired.len() >= 4 {
                return self.reply(r, "busy", Some("capacity"));
            }
            let Some(next) = self.next_lease.checked_add(1) else {
                return self.reply(r, "rejected", Some("capacity"));
            };
            let Some(expires) = now_ms.checked_add(LEASE_MS) else {
                return self.reply(r, "rejected", Some("capacity"));
            };
            for w in expired {
                self.sessions.remove(&w);
                self.previews.retain(|_, p| p.writer != w);
                self.retired.insert(w);
            }
            let lease = Counter(self.next_lease);
            self.next_lease = next;
            let mut p = self.reply(r, "applied", None);
            p.body.granted_lease = Some(lease);
            p.body.scope = Some(scope);
            p.body.lease_remaining_ms = Some(2000);
            self.sessions.insert(
                writer.clone(),
                Session {
                    scope,
                    lease,
                    expires,
                    high: r.request_id.unwrap().0,
                    cache: VecDeque::from([(r.clone(), p.clone())]),
                },
            );
            return p;
        }
        let Some(session) = self.sessions.get(writer) else {
            return self.reply(r, "rejected", Some("lease"));
        };
        // Grant retries must present the original null lease but still identify the
        // exact live session. Every other command needs its engine-issued lease.
        if session.expires <= now_ms
            || (r.lease != Some(session.lease) && !matches!(r.command, Command::Grant { .. }))
        {
            return self.reply(r, "rejected", Some("lease"));
        }
        let number = r.request_id.unwrap().0;
        if let Some((old, p)) = session
            .cache
            .iter()
            .find(|(old, _)| old.request_id == r.request_id)
        {
            return if old == r {
                p.clone()
            } else {
                self.reply(r, "rejected", Some("reused_id"))
            };
        }
        if number <= session.high {
            return self.reply(r, "rejected", Some("expired_id"));
        }
        let scope = session.scope;
        let p = if r.expected_revision != Some(self.revision) {
            self.reply(r, "conflict", Some("stale_revision"))
        } else {
            self.execute(r, scope, now_ms)
        };
        if let Some(s) = self.sessions.get_mut(writer) {
            s.high = number;
            s.cache.push_back((r.clone(), p.clone()));
            if s.cache.len() > 64 {
                s.cache.pop_front();
            }
        }
        p
    }
    fn edits(&self, targets: &[Edit], scope: Scope) -> Result<()> {
        if targets.is_empty() || targets.len() > 64 {
            return Err("capacity".into());
        }
        let mut seen = BTreeSet::new();
        for e in targets {
            if !seen.insert(&e.target)
                || !self.parameters.iter().any(|p| p.target == e.target)
                || !id(e.target.input())
            {
                return Err("target".into());
            }
            if e.target.scope()? != scope {
                return Err("scope".into());
            }
            if !e.target.value_valid(&e.value) {
                return Err("range".into());
            }
        }
        Ok(())
    }
    fn execute(&mut self, r: &Request, scope: Scope, now: u64) -> Reply {
        let writer = r.writer.as_ref().unwrap();
        match &r.command {
            Command::Renew {} => {
                let Some(expires) = now.checked_add(LEASE_MS) else {
                    return self.reply(r, "rejected", Some("capacity"));
                };
                self.sessions.get_mut(writer).unwrap().expires = expires;
                let mut p = self.reply(r, "applied", None);
                p.body.lease_remaining_ms = Some(2000);
                p.body.scope = Some(scope);
                p
            }
            Command::Release {} => {
                self.sessions.remove(writer);
                self.retired.insert(writer.clone());
                self.previews.retain(|_, p| p.writer != *writer);
                self.reply(r, "applied", None)
            }
            Command::Set { targets } | Command::Propose { targets } => {
                if let Err(e) = self.edits(targets, scope) {
                    return self.reply(r, "rejected", Some(&e));
                }
                if matches!(r.command, Command::Propose { .. })
                    && self.modes[&scope] != Mode::Assist
                {
                    return self.reply(r, "rejected", Some("scope"));
                }
                let Some(rev) = self.revision.0.checked_add(1) else {
                    return self.reply(r, "rejected", Some("capacity"));
                };
                for e in targets {
                    let p = self
                        .parameters
                        .iter_mut()
                        .find(|p| p.target == e.target)
                        .unwrap();
                    if matches!(r.command, Command::Propose { .. }) {
                        p.proposal = Some(e.value.clone());
                    } else {
                        p.target_value = e.value.clone();
                        p.hold = Some(e.value.clone());
                        p.owner = Some(writer.clone());
                    }
                }
                self.revision = Counter(rev);
                let mut p = self.reply(r, "applied", None);
                p.body.ramp_frames = Some(240);
                p
            }
            Command::SetMode { mode, bounds } => {
                if let Err(e) = validate_bounds(bounds) {
                    return self.reply(r, "rejected", Some(&e));
                }
                if (*mode == Mode::Auto && bounds.is_empty())
                    || (*mode != Mode::Auto && !bounds.is_empty())
                    || bounds.iter().any(|b| b.target.scope() != Ok(scope))
                {
                    return self.reply(r, "rejected", Some("scope"));
                }

                let Some(rev) = self.revision.0.checked_add(1) else {
                    return self.reply(r, "rejected", Some("capacity"));
                };
                for p in &mut self.parameters {
                    if p.target.scope() == Ok(scope) {
                        p.hold = Some(p.target_value.clone());
                        p.owner = Some(writer.clone());
                    }
                }
                self.modes.insert(scope, *mode);
                self.automation_bounds.insert(scope, bounds.clone());
                self.revision = Counter(rev);
                self.reply(r, "applied", None)
            }
            Command::PreviewRelease { targets } => {
                let mut seen = BTreeSet::new();
                if targets.is_empty() || targets.len() > 64 {
                    return self.reply(r, "rejected", Some("capacity"));
                }
                let mut destinations = Vec::new();
                for t in targets {
                    if !seen.insert(t) {
                        return self.reply(r, "rejected", Some("target"));
                    }
                    if t.scope() != Ok(scope) {
                        return self.reply(r, "rejected", Some("scope"));
                    }
                    let Some(p) = self.parameters.iter().find(|p| p.target == *t) else {
                        return self.reply(r, "rejected", Some("target"));
                    };
                    let Some(value) = p.proposal.clone() else {
                        return self.reply(r, "rejected", Some("unavailable"));
                    };
                    if p.hold.is_none() || matches!(t, Target::Mute { .. }) {
                        return self.reply(r, "rejected", Some("unavailable"));
                    }
                    destinations.push(Edit {
                        target: t.clone(),
                        value,
                    });
                }
                let (Some(next), Some(expires)) =
                    (self.next_preview.checked_add(1), now.checked_add(LEASE_MS))
                else {
                    return self.reply(r, "rejected", Some("capacity"));
                };
                // One preview per writer bounds transient storage.
                self.previews.retain(|_, p| p.writer != *writer);
                let preview = Preview {
                    token: format!("preview-{}", self.next_preview),
                    revision: self.revision,
                    scope,
                    remaining_ms: 2000,
                    ramp_frames: 240,
                    destinations,
                };
                self.next_preview = next;
                self.previews.insert(
                    preview.token.clone(),
                    StoredPreview {
                        preview: preview.clone(),
                        writer: writer.clone(),
                        lease: r.lease.unwrap(),
                        expires,
                    },
                );
                let mut p = self.reply(r, "applied", None);
                p.body.preview = Some(preview);
                p
            }
            Command::CancelPreview { token } | Command::ReleasePreview { token } => {
                let Some(p) = self.previews.get(token) else {
                    return self.reply(r, "rejected", Some("target"));
                };
                if p.writer != *writer || Some(p.lease) != r.lease || p.preview.scope != scope {
                    return self.reply(r, "rejected", Some("scope"));
                }
                if p.expires <= now || p.preview.revision != self.revision {
                    return self.reply(r, "conflict", Some("stale_revision"));
                }
                if matches!(r.command, Command::ReleasePreview { .. }) {
                    if !self.rendered_release {
                        return self.reply(r, "rejected", Some("unavailable"));
                    }
                    let Some(rev) = self.revision.0.checked_add(1) else {
                        return self.reply(r, "rejected", Some("capacity"));
                    };
                    let destinations = p.preview.destinations.clone();
                    for e in destinations {
                        let parameter = self
                            .parameters
                            .iter_mut()
                            .find(|p| p.target == e.target)
                            .unwrap();
                        parameter.target_value = e.value;
                        parameter.hold = None;
                        parameter.owner = None;
                    }
                    self.revision = Counter(rev);
                }
                self.previews.remove(token);
                self.reply(r, "applied", None)
            }
            _ => self.reply(r, "rejected", Some("lease")),
        }
    }
}
/// Receipt-clock collector. An incomplete or inconsistent set is discarded.
#[derive(Default, Debug)]
pub struct SnapshotPages {
    started: Option<u64>,
    pages: BTreeMap<u8, Snapshot>,
}
impl SnapshotPages {
    pub fn push(&mut self, page: Snapshot, receipt_ms: u64) -> Result<Option<Vec<Snapshot>>> {
        let result = self.accept(page, receipt_ms);
        if result.is_err() || matches!(result, Ok(Some(_))) {
            self.started = None;
            self.pages.clear();
        }
        result
    }
    fn accept(&mut self, p: Snapshot, now: u64) -> Result<Option<Vec<Snapshot>>> {
        p.validate()?;
        if self.started.is_some_and(|t| now < t || now - t > 2000) {
            return Err("expired snapshot".into());
        }
        if let Some(old) = self.pages.values().next()
            && (old.show_id != p.show_id
                || old.epoch != p.epoch
                || old.revision != p.revision
                || old.sequence != p.sequence
                || old.page_count != p.page_count
                || old.inputs != p.inputs
                || old.monitors != p.monitors
                || old.modes != p.modes
                || old.automation_bounds != p.automation_bounds
                || old.durability != p.durability
                || old.validity != p.validity
                || old.acquisition_frame != p.acquisition_frame
                || old.age_ms != p.age_ms
                || old.rendered_application != p.rendered_application
                || old.release_commit != p.release_commit
                || old.session_history_capacity != p.session_history_capacity)
        {
            return Err("mixed snapshot".into());
        }
        if self.pages.contains_key(&p.page) {
            return Err("duplicate page".into());
        }
        self.started.get_or_insert(now);
        let count = p.page_count;
        self.pages.insert(p.page, p);
        if self.pages.len() == count as usize {
            validate_inventory(self.pages.values().flat_map(|p| p.parameters.iter()))?;
            Ok(Some(self.pages.values().cloned().collect()))
        } else {
            Ok(None)
        }
    }
}
fn required_targets() -> BTreeSet<Target> {
    let mut targets = BTreeSet::new();
    for i in 1..=8 {
        let input = format!("input-{i:02}");
        targets.insert(Target::Fader {
            input: input.clone(),
        });
        targets.insert(Target::Pan {
            input: input.clone(),
        });
        targets.insert(Target::Mute {
            input: input.clone(),
        });
        for monitor in ["monitor-1", "monitor-2"] {
            targets.insert(Target::Send {
                input: input.clone(),
                monitor: monitor.into(),
            });
        }
    }
    targets
}
fn validate_inventory<'a>(parameters: impl Iterator<Item = &'a Parameter>) -> Result<()> {
    let mut seen = BTreeSet::new();
    for p in parameters {
        if !seen.insert(p.target.clone()) {
            return Err("duplicate parameter".into());
        }
    }
    if seen != required_targets() {
        return Err("parameter inventory".into());
    }
    Ok(())
}
impl Snapshot {
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let s: Self = decode(bytes)?;
        s.validate()?;
        Ok(s)
    }
    pub fn validate(&self) -> Result<()> {
        if !canonical_show(&self.show_id)
            || self.epoch.0 == 0
            || self.sequence.0 == 0
            || self.page_count == 0
            || self.page_count > 16
            || self.page >= self.page_count
        {
            return Err("snapshot identity".into());
        }
        if self.durability != "volatile"
            || self.validity != "unavailable"
            || self.acquisition_frame.is_some()
            || self.age_ms.is_some()
            || self.rendered_application
            || self.release_commit
            || self.session_history_capacity != SESSION_HISTORY_CAPACITY as u32
        {
            return Err("harness capabilities".into());
        }
        let inputs: BTreeSet<_> = self.inputs.iter().cloned().collect();
        let monitors: BTreeSet<_> = self.monitors.iter().cloned().collect();
        if self.inputs.len() != 8
            || inputs != (1..=8).map(|i| format!("input-{i:02}")).collect()
            || self.monitors.len() != 2
            || monitors != ["monitor-1".to_owned(), "monitor-2".to_owned()].into()
        {
            return Err("channel inventory".into());
        }
        if self.modes.len() != 3
            || self.modes.iter().map(|(s, _)| *s).collect::<BTreeSet<_>>()
                != [Scope::Foh, Scope::Monitor1, Scope::Monitor2].into()
        {
            return Err("modes".into());
        }
        validate_bounds(&self.automation_bounds)?;
        for (scope, mode) in &self.modes {
            let has_bounds = self
                .automation_bounds
                .iter()
                .any(|b| b.target.scope() == Ok(*scope));
            if (*mode == Mode::Auto) != has_bounds {
                return Err("automation bounds".into());
            }
        }
        if self.parameters.len() > 40 {
            return Err("capacity".into());
        }
        let allowed = required_targets();
        let mut seen = BTreeSet::new();
        for p in &self.parameters {
            if !allowed.contains(&p.target)
                || !seen.insert(&p.target)
                || p.actual.is_some()
                || !p.target.value_valid(&p.target_value)
                || p.hold.as_ref().is_some_and(|v| !p.target.value_valid(v))
                || p.proposal
                    .as_ref()
                    .is_some_and(|v| !p.target.value_valid(v))
                || p.owner.as_ref().is_some_and(|w| !id(w))
                || p.hold.is_some() != p.owner.is_some()
            {
                return Err("parameter".into());
            }
        }
        if self.page_count == 1 {
            validate_inventory(self.parameters.iter())?;
        }
        Ok(())
    }
}
impl Reply {
    pub fn validate(&self) -> Result<()> {
        if self.contract != "C-AUDIO"
            || self.version != 1
            || self.module != "audio"
            || !canonical_show(&self.show_id)
            || self.epoch.0 == 0
        {
            return Err("reply identity".into());
        }
        if self.writer.as_ref().is_some_and(|w| !id(w))
            || self.lease.is_some_and(|n| n.0 == 0)
            || self.request_id.is_some_and(|n| n.0 == 0)
        {
            return Err("reply authority".into());
        }
        if (self.writer.is_some()
            && (self.request_id.is_none() || self.expected_revision.is_none()))
            || (self.writer.is_none()
                && (self.lease.is_some()
                    || self.request_id.is_some()
                    || self.expected_revision.is_some()))
        {
            return Err("reply authority".into());
        }
        let b = &self.body;
        if self.kind == "applied"
            && ((self.writer.is_none() && b.snapshot.is_none())
                || (self.writer.is_some() && self.lease.is_none() && b.granted_lease.is_none()))
        {
            return Err("reply authority".into());
        }
        if b.scope.is_some() != b.lease_remaining_ms.is_some()
            || (b.ramp_frames.is_some()
                && (b.preview.is_some() || b.snapshot.is_some() || b.scope.is_some()))
        {
            return Err("reply body".into());
        }
        let reasons = [
            "wrong_show",
            "version",
            "epoch",
            "lease",
            "scope",
            "target",
            "range",
            "stale_revision",
            "reused_id",
            "expired_id",
            "unavailable",
            "capacity",
            "clock",
        ];
        match self.kind.as_str() {
            "applied" if b.reason.is_none() => (),
            "rejected"
                if b.reason
                    .as_ref()
                    .is_some_and(|r| reasons.contains(&r.as_str())) => {}
            "conflict" if b.reason.as_deref() == Some("stale_revision") => (),
            "busy" if matches!(b.reason.as_deref(), Some("scope" | "capacity")) => (),
            _ => return Err("reply kind".into()),
        }
        if b.effective_frame.is_some()
            || b.ramp_frames.is_some_and(|n| n != 240)
            || b.lease_remaining_ms.is_some_and(|n| n > 2000)
            || b.granted_lease.is_some_and(|n| n.0 == 0)
        {
            return Err("reply timing".into());
        }
        if self.kind != "applied"
            && (b.snapshot.is_some()
                || b.preview.is_some()
                || b.granted_lease.is_some()
                || b.scope.is_some()
                || b.lease_remaining_ms.is_some()
                || b.ramp_frames.is_some())
        {
            return Err("refusal body".into());
        }
        if let Some(s) = &b.snapshot {
            s.validate()?;
            if s.show_id != self.show_id
                || s.epoch != self.epoch
                || s.revision != b.revision
                || self.writer.is_some()
                || self.lease.is_some()
                || self.request_id.is_some()
                || self.expected_revision.is_some()
                || b.preview.is_some()
                || b.scope.is_some()
                || b.granted_lease.is_some()
                || b.lease_remaining_ms.is_some()
                || b.ramp_frames.is_some()
            {
                return Err("snapshot envelope".into());
            }
        }
        if b.granted_lease.is_some()
            && (self.lease.is_some()
                || self.writer.is_none()
                || self.request_id.is_none()
                || self.expected_revision.is_none()
                || b.scope.is_none()
                || b.lease_remaining_ms.is_none()
                || b.preview.is_some()
                || b.ramp_frames.is_some())
        {
            return Err("grant body".into());
        }
        if let Some(p) = &b.preview {
            if !id(&p.token)
                || p.revision != b.revision
                || p.remaining_ms > 2000
                || p.ramp_frames != 240
                || p.destinations.is_empty()
                || p.destinations.len() > 64
                || self.writer.is_none()
                || self.lease.is_none()
                || self.request_id.is_none()
                || self.expected_revision.is_none()
                || b.granted_lease.is_some()
                || b.scope.is_some()
                || b.lease_remaining_ms.is_some()
                || b.ramp_frames.is_some()
            {
                return Err("preview".into());
            }
            let allowed = required_targets();
            let mut seen = BTreeSet::new();
            for e in &p.destinations {
                if !allowed.contains(&e.target)
                    || !seen.insert(&e.target)
                    || e.target.scope() != Ok(p.scope)
                    || !e.target.value_valid(&e.value)
                    || matches!(e.target, Target::Mute { .. })
                {
                    return Err("preview destination".into());
                }
            }
        }
        Ok(())
    }
}
fn validate_bounds(bounds: &[AutoBound]) -> Result<()> {
    if bounds.len() > 64 {
        return Err("capacity".into());
    }
    let allowed = required_targets();
    let mut seen = BTreeSet::new();
    for b in bounds {
        if !allowed.contains(&b.target)
            || !seen.insert(&b.target)
            || matches!(b.target, Target::Mute { .. })
        {
            return Err("target".into());
        }
        if !b.target.value_valid(&b.min)
            || !b.target.value_valid(&b.max)
            || !matches!((&b.min,&b.max),(Value::Integer(min),Value::Integer(max)) if min<=max)
        {
            return Err("range".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    const SHOW: &str = "11111111-1111-4111-8111-111111111111";
    fn r(w: &str, command: Command, n: u64, rev: u64, lease: Option<Counter>) -> Request {
        Request {
            contract: "C-AUDIO".into(),
            version: 1,
            show_id: SHOW.into(),
            module: "audio".into(),
            epoch: Counter(9),
            writer: Some(w.into()),
            lease,
            request_id: Some(Counter(n)),
            expected_revision: Some(Counter(rev)),
            command,
        }
    }
    #[test]
    fn history_and_expired_previews_are_bounded_without_forgetting_writers() {
        let mut a = Authority::new(SHOW, Counter(9), Counter(0)).unwrap();
        for i in 0..SESSION_HISTORY_CAPACITY {
            let writer = format!("desk-{i}");
            let now = i as u64 * 2000;
            let rev = a.revision.0;
            let l = a
                .handle(
                    &r(&writer, Command::Grant { scope: Scope::Foh }, 1, rev, None),
                    now,
                )
                .body
                .granted_lease
                .unwrap();
            assert!(a.previews.is_empty());
            let t = Target::Fader {
                input: "input-01".into(),
            };
            let rev = a.revision.0;
            a.handle(
                &r(
                    &writer,
                    Command::SetMode {
                        mode: Mode::Assist,
                        bounds: vec![],
                    },
                    2,
                    rev,
                    Some(l),
                ),
                now,
            );
            let rev = a.revision.0;
            a.handle(
                &r(
                    &writer,
                    Command::Propose {
                        targets: vec![Edit {
                            target: t.clone(),
                            value: Value::Integer(-2000),
                        }],
                    },
                    3,
                    rev,
                    Some(l),
                ),
                now,
            );
            let rev = a.revision.0;
            assert!(
                a.handle(
                    &r(
                        &writer,
                        Command::PreviewRelease { targets: vec![t] },
                        4,
                        rev,
                        Some(l)
                    ),
                    now
                )
                .body
                .preview
                .is_some()
            );
            assert_eq!(a.previews.len(), 1);
            assert!(a.sessions.len() <= 4);
            assert_eq!(a.sessions.len() + a.retired.len(), i + 1);
        }
        let parameters = a.parameters.clone();
        let rev = a.revision;
        let p = a.handle(
            &r(
                "desk-new",
                Command::Grant { scope: Scope::Foh },
                1,
                rev.0,
                None,
            ),
            SESSION_HISTORY_CAPACITY as u64 * 2000,
        );
        assert_eq!(p.body.reason.as_deref(), Some("capacity"));
        assert_eq!(a.parameters, parameters);
        assert_eq!(a.revision, rev);
        assert_eq!(a.sessions.len() + a.retired.len(), 1024);
        assert_eq!(
            a.handle(
                &r(
                    "desk-0",
                    Command::Grant { scope: Scope::Foh },
                    1,
                    rev.0,
                    None
                ),
                SESSION_HISTORY_CAPACITY as u64 * 2000
            )
            .body
            .reason
            .as_deref(),
            Some("lease")
        );
    }
    #[test]
    fn internal_counter_exhaustion_never_wraps_or_changes_mix() {
        let mut a = Authority::new(SHOW, Counter(9), Counter(12)).unwrap();
        a.next_lease = u64::MAX;
        let state = a.parameters.clone();
        assert_eq!(
            a.handle(
                &r("desk", Command::Grant { scope: Scope::Foh }, 1, 12, None),
                0
            )
            .body
            .reason
            .as_deref(),
            Some("capacity")
        );
        assert!(a.sessions.is_empty());
        assert_eq!(a.parameters, state);
        a.sequence = u64::MAX;
        assert!(a.snapshot().is_err());
        assert_eq!(a.sequence, u64::MAX);
    }
}
