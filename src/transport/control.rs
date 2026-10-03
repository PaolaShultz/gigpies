use super::Error;
pub const CONTROL_BYTES: usize = 64;

/// Fixed-size prototype command. Socket peer pinning is required. Identity
/// fields reject stale traffic; they are not cryptographic authentication.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Command {
    pub session: u64,
    pub writer: u64,
    pub lease: u64,
    pub id: u64,
    pub expected_revision: u64,
    pub value_milli: i32,
}
impl Command {
    pub fn encode(self) -> [u8; CONTROL_BYTES] {
        let mut b = [0; CONTROL_BYTES];
        b[..4].copy_from_slice(b"GPC1");
        for (i, x) in [
            self.session,
            self.writer,
            self.lease,
            self.id,
            self.expected_revision,
        ]
        .into_iter()
        .enumerate()
        {
            b[8 + i * 8..16 + i * 8].copy_from_slice(&x.to_be_bytes());
        }
        b[48..52].copy_from_slice(&self.value_milli.to_be_bytes());
        b
    }
    pub fn parse(b: &[u8]) -> Result<Self, Error> {
        if b.len() != CONTROL_BYTES {
            return Err(Error::Size);
        }
        if &b[..4] != b"GPC1" || b[4..8] != [0; 4] || b[52..] != [0; 12] {
            return Err(Error::Format);
        }
        let c = Self {
            session: at(b, 8),
            writer: at(b, 16),
            lease: at(b, 24),
            id: at(b, 32),
            expected_revision: at(b, 40),
            value_milli: i32::from_be_bytes(b[48..52].try_into().unwrap()),
        };
        if c.session == 0
            || c.writer == 0
            || c.lease == 0
            || c.id == 0
            || !(-60_000..=12_000).contains(&c.value_milli)
        {
            return Err(Error::Format);
        }
        Ok(c)
    }
}
fn at(b: &[u8], i: usize) -> u64 {
    u64::from_be_bytes(b[i..i + 8].try_into().unwrap())
}
/// A fresh monotonically increasing lease within this PA epoch. The host must
/// retain the lease high-water mark across control-worker restarts, or rotate epoch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hello {
    pub session: u64,
    pub writer: u64,
    pub lease: u64,
}
impl Hello {
    pub fn encode(self) -> [u8; 32] {
        let mut b = [0; 32];
        b[..4].copy_from_slice(b"GPH1");
        b[8..16].copy_from_slice(&self.session.to_be_bytes());
        b[16..24].copy_from_slice(&self.writer.to_be_bytes());
        b[24..].copy_from_slice(&self.lease.to_be_bytes());
        b
    }
    pub fn parse(b: &[u8]) -> Result<Self, Error> {
        if b.len() != 32 {
            return Err(Error::Size);
        }
        if &b[..4] != b"GPH1" || b[4..8] != [0; 4] {
            return Err(Error::Format);
        }
        let h = Self {
            session: at(b, 8),
            writer: at(b, 16),
            lease: at(b, 24),
        };
        if h.session == 0 || h.writer == 0 || h.lease == 0 {
            return Err(Error::Identity);
        }
        Ok(h)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Disposition {
    Applied = 1,
    Duplicate = 2,
    Stale = 3,
    Identity = 4,
    Invalid = 5,
    Disconnected = 6,
    Snapshot = 7,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ack {
    pub session: u64,
    pub writer: u64,
    pub lease: u64,
    pub id: u64,
    pub revision: u64,
    pub value_milli: i32,
    pub disposition: Disposition,
}
impl Ack {
    pub fn encode(self) -> [u8; CONTROL_BYTES] {
        let mut b = Command {
            session: self.session,
            writer: self.writer,
            lease: self.lease,
            id: self.id,
            expected_revision: self.revision,
            value_milli: self.value_milli,
        }
        .encode();
        b[..4].copy_from_slice(b"GPK1");
        b[4] = self.disposition as u8;
        b
    }
    pub fn parse(b: &[u8]) -> Result<Self, Error> {
        if b.len() != CONTROL_BYTES {
            return Err(Error::Size);
        }
        if &b[..4] != b"GPK1" || b[5..8] != [0; 3] || b[52..] != [0; 12] {
            return Err(Error::Format);
        }
        Ok(Self {
            session: at(b, 8),
            writer: at(b, 16),
            lease: at(b, 24),
            id: at(b, 32),
            revision: at(b, 40),
            value_milli: i32::from_be_bytes(b[48..52].try_into().unwrap()),
            disposition: match b[4] {
                1 => Disposition::Applied,
                2 => Disposition::Duplicate,
                3 => Disposition::Stale,
                4 => Disposition::Identity,
                5 => Disposition::Invalid,
                6 => Disposition::Disconnected,
                7 => Disposition::Snapshot,
                _ => return Err(Error::Format),
            },
        })
    }
    pub fn matches(self, c: Command) -> bool {
        (self.session, self.writer, self.lease, self.id) == (c.session, c.writer, c.lease, c.id)
    }
}
/// One authoritative prototype parameter scope. Networking and timeout handling
/// run on a control worker. This does not apply commands to a real mixer.
pub struct Authority {
    session: u64,
    writer: u64,
    lease: u64,
    revision: u64,
    value: i32,
    last: Option<(Command, Ack)>,
    connected: bool,
}
impl Authority {
    pub fn new(session: u64, writer: u64) -> Result<Self, Error> {
        if session == 0 || writer == 0 {
            return Err(Error::Identity);
        }
        Ok(Self {
            session,
            writer,
            lease: 0,
            revision: 0,
            value: 0,
            last: None,
            connected: false,
        })
    }
    pub fn resync(&mut self, lease: u64) -> Result<(u64, i32), Error> {
        if lease <= self.lease {
            return Err(Error::Identity);
        }
        self.lease = lease;
        self.last = None;
        self.connected = true;
        Ok((self.revision, self.value))
    }
    /// An exact hello retry returns a snapshot only while this lease is live.
    /// A timed-out lease cannot reconnect: a new hello is mandatory.
    pub fn hello(&mut self, h: Hello) -> Result<Ack, Error> {
        if (h.session, h.writer) != (self.session, self.writer) || h.lease == 0 {
            return Err(Error::Identity);
        }
        if !(self.connected && h.lease == self.lease) {
            self.resync(h.lease)?;
        }
        Ok(self.ack(0, Disposition::Snapshot))
    }
    pub fn disconnect(&mut self) {
        self.connected = false;
    }
    pub fn snapshot(&self) -> (u64, i32) {
        (self.revision, self.value)
    }
    fn ack(&self, id: u64, disposition: Disposition) -> Ack {
        Ack {
            session: self.session,
            writer: self.writer,
            lease: self.lease,
            id,
            revision: self.revision,
            value_milli: self.value,
            disposition,
        }
    }
    fn reply(&self, c: Command, disposition: Disposition) -> Ack {
        Ack {
            session: c.session,
            writer: c.writer,
            lease: c.lease,
            ..self.ack(c.id, disposition)
        }
    }
    pub fn apply(&mut self, c: Command) -> Ack {
        if !self.connected {
            return self.reply(c, Disposition::Disconnected);
        }
        if (c.session, c.writer, c.lease) != (self.session, self.writer, self.lease) {
            return self.reply(c, Disposition::Identity);
        }
        if c.id == 0 || !(-60_000..=12_000).contains(&c.value_milli) {
            return self.reply(c, Disposition::Invalid);
        }
        if let Some((prev, old)) = self.last {
            if c == prev {
                return Ack {
                    disposition: Disposition::Duplicate,
                    ..old
                };
            }
            if c.id <= prev.id {
                return self.reply(c, Disposition::Stale);
            }
        }
        if c.expected_revision != self.revision {
            return self.reply(c, Disposition::Stale);
        }
        let Some(next) = self.revision.checked_add(1) else {
            return self.reply(c, Disposition::Invalid);
        };
        self.revision = next;
        self.value = c.value_milli;
        let ack = self.reply(c, Disposition::Applied);
        self.last = Some((c, ack));
        ack
    }
}

/// One in-flight command and bounded retries. All timestamps are local monotonic
/// nanoseconds. The host calls poll at its control rate, independently of audio.
pub struct ControlWriter {
    session: u64,
    writer: u64,
    lease: u64,
    revision: u64,
    next_id: u64,
    synchronized: bool,
    pending: Option<Command>,
    first_send: u64,
    last_send: u64,
    attempts: u8,
    last_time: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlRequest {
    Hello(Hello),
    Command(Command),
}
impl ControlRequest {
    pub fn encode(self, out: &mut [u8; CONTROL_BYTES]) -> usize {
        match self {
            Self::Hello(h) => {
                out[..32].copy_from_slice(&h.encode());
                32
            }
            Self::Command(c) => {
                *out = c.encode();
                CONTROL_BYTES
            }
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WriterEvent {
    Ignored,
    Snapshot,
    Applied { roundtrip_ns: u64 },
    Resync,
}
impl ControlWriter {
    pub fn new(session: u64, writer: u64, first_lease: u64) -> Result<Self, Error> {
        if session == 0 || writer == 0 || first_lease == 0 {
            return Err(Error::Identity);
        }
        Ok(Self {
            session,
            writer,
            lease: first_lease,
            revision: 0,
            next_id: 1,
            synchronized: false,
            pending: None,
            first_send: 0,
            last_send: 0,
            attempts: 0,
            last_time: 0,
        })
    }
    fn clock(&mut self, now: u64) -> Result<(), Error> {
        if now < self.last_time {
            return Err(Error::Timeline);
        }
        self.last_time = now;
        Ok(())
    }
    fn resync(&mut self) -> Result<(), Error> {
        self.lease = self.lease.checked_add(1).ok_or(Error::Timeline)?;
        self.pending = None;
        self.synchronized = false;
        self.attempts = 0;
        self.next_id = 1;
        Ok(())
    }
    pub fn poll(&mut self, now: u64, value_milli: i32) -> Result<Option<ControlRequest>, Error> {
        self.clock(now)?;
        if !(-60_000..=12_000).contains(&value_milli) {
            return Err(Error::Format);
        }
        if self.attempts > 0 && now - self.last_send < 100_000_000 {
            return Ok(None);
        }
        if self.attempts == 3 {
            self.resync()?;
        }
        let request = if !self.synchronized {
            ControlRequest::Hello(Hello {
                session: self.session,
                writer: self.writer,
                lease: self.lease,
            })
        } else {
            let c = match self.pending {
                Some(c) => c,
                None => {
                    let id = self.next_id;
                    self.next_id = id.checked_add(1).ok_or(Error::Timeline)?;
                    self.first_send = now;
                    let c = Command {
                        session: self.session,
                        writer: self.writer,
                        lease: self.lease,
                        id,
                        expected_revision: self.revision,
                        value_milli,
                    };
                    self.pending = Some(c);
                    c
                }
            };
            ControlRequest::Command(c)
        };
        self.attempts += 1;
        self.last_send = now;
        Ok(Some(request))
    }
    pub fn accept(&mut self, ack: Ack, now: u64) -> Result<WriterEvent, Error> {
        self.clock(now)?;
        if (ack.session, ack.writer, ack.lease) != (self.session, self.writer, self.lease) {
            return Ok(WriterEvent::Ignored);
        }
        if !self.synchronized
            && ack.disposition == Disposition::Snapshot
            && ack.id == 0
            && (-60_000..=12_000).contains(&ack.value_milli)
        {
            self.revision = ack.revision;
            self.synchronized = true;
            self.attempts = 0;
            return Ok(WriterEvent::Snapshot);
        }
        if let Some(c) = self.pending
            && ack.matches(c)
        {
            if matches!(
                ack.disposition,
                Disposition::Applied | Disposition::Duplicate
            ) && c.expected_revision.checked_add(1) == Some(ack.revision)
                && c.value_milli == ack.value_milli
            {
                self.revision = ack.revision;
                self.pending = None;
                self.attempts = 0;
                return Ok(WriterEvent::Applied {
                    roundtrip_ns: now - self.first_send,
                });
            }
            self.resync()?;
            return Ok(WriterEvent::Resync);
        }
        Ok(WriterEvent::Ignored)
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
}
