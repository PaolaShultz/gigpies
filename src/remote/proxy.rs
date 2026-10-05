//! Bounded SPSC handoff between the network worker and the existing authority
//! owner. `AuthorityMailbox::service` runs outside render, before its next source
//! block. Neither network activity nor a stalled client can lock the callback.
use super::*;
use rtrb::{Consumer, Producer, RingBuffer};
use serde_json::Value;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

enum Job {
    Command(Value),
    Negotiate(MediaDescriptor),
}
enum Event {
    Reply(Result<Value>),
    Negotiated(Result<MediaDescriptor>),
}
struct Shared {
    disconnected: AtomicBool,
    retiring: AtomicBool,
    source_frame: AtomicU64,
}

pub struct AuthorityProxy {
    context: AuthenticatedContext,
    identity: EngineIdentity,
    shared: Arc<Shared>,
    commands: Producer<Job>,
    events: Consumer<Event>,
    media_in: Producer<Vec<u8>>,
    media_out: Consumer<Vec<u8>>,
    replies: std::collections::VecDeque<Result<Value>>,
    negotiations: std::collections::VecDeque<Result<MediaDescriptor>>,
    refusals: std::collections::VecDeque<String>,
}
pub struct AuthorityMailbox {
    context: AuthenticatedContext,
    identity: EngineIdentity,
    shared: Arc<Shared>,
    commands: Consumer<Job>,
    events: Producer<Event>,
    media_in: Consumer<Vec<u8>>,
    media_out: Producer<Vec<u8>>,
    retired: bool,
}
/// Per authenticated connection: 16 command slots, 128 completion slots and 256
/// independent media slots in each direction. Construction/all destruction is
/// outside render. Queue exhaustion retires control; media exhaustion drops wet.
pub fn authority_channel(
    context: &AuthenticatedContext,
    identity: EngineIdentity,
) -> Result<(AuthorityProxy, AuthorityMailbox)> {
    identity.validate()?;
    let shared = Arc::new(Shared {
        disconnected: AtomicBool::new(false),
        retiring: AtomicBool::new(false),
        source_frame: AtomicU64::new(u64::MAX),
    });
    let (commands_tx, commands_rx) = RingBuffer::new(16);
    let (events_tx, events_rx) = RingBuffer::new(128);
    let (media_in_tx, media_in_rx) = RingBuffer::new(256);
    let (media_out_tx, media_out_rx) = RingBuffer::new(256);
    Ok((
        AuthorityProxy {
            context: context.clone(),
            identity: identity.clone(),
            shared: shared.clone(),
            commands: commands_tx,
            events: events_rx,
            media_in: media_in_tx,
            media_out: media_out_rx,
            replies: std::collections::VecDeque::with_capacity(128),
            negotiations: std::collections::VecDeque::with_capacity(1),
            refusals: std::collections::VecDeque::with_capacity(32),
        },
        AuthorityMailbox {
            context: context.clone(),
            identity,
            shared,
            commands: commands_rx,
            events: events_tx,
            media_in: media_in_rx,
            media_out: media_out_tx,
            retired: false,
        },
    ))
}
impl AuthorityProxy {
    fn check(&self, context: &AuthenticatedContext) -> Result<()> {
        if context != &self.context
            || self.shared.disconnected.load(Ordering::Acquire)
            || self.shared.retiring.load(Ordering::Acquire)
        {
            return Err("authority session retired".into());
        }
        Ok(())
    }
    fn drain(&mut self) -> Result<()> {
        if self.shared.disconnected.load(Ordering::Acquire) {
            return Err("authority session retired".into());
        }
        for _ in 0..128 {
            let Ok(event) = self.events.pop() else {
                break;
            };
            match event {
                Event::Reply(Err(reason)) if self.refusals.len() < 32 => {
                    self.refusals.push_back(reason)
                }
                Event::Reply(Ok(reply)) if self.replies.len() < 128 => {
                    self.replies.push_back(Ok(reply))
                }
                Event::Negotiated(reply) if self.negotiations.is_empty() => {
                    self.negotiations.push_back(reply)
                }
                _ => {
                    self.shared.disconnected.store(true, Ordering::Release);
                    return Err("authority proxy result capacity".into());
                }
            }
        }
        Ok(())
    }
}
impl Drop for AuthorityProxy {
    fn drop(&mut self) {
        self.shared.disconnected.store(true, Ordering::Release);
    }
}
impl AuthorityEndpoint for AuthorityProxy {
    fn retiring(&self) -> bool {
        self.shared.retiring.load(Ordering::Acquire)
    }
    fn identity(&self) -> EngineIdentity {
        self.identity.clone()
    }
    fn dispatch(
        &mut self,
        context: &AuthenticatedContext,
        payload: Value,
        _now_ms: u64,
    ) -> Result<Option<Value>> {
        self.check(context)?;
        self.commands
            .push(Job::Command(payload))
            .map_err(|_| "authority command queue full")?;
        Ok(None)
    }
    fn negotiate(
        &mut self,
        context: &AuthenticatedContext,
        descriptor: &MediaDescriptor,
    ) -> Result<bool> {
        self.check(context)?;
        self.commands
            .push(Job::Negotiate(descriptor.clone()))
            .map_err(|_| "authority command queue full")?;
        Ok(false)
    }
    fn poll_refusal(&mut self) -> Result<Option<String>> {
        self.drain()?;
        Ok(self.refusals.pop_front())
    }
    fn poll_negotiation(&mut self) -> Result<Option<MediaDescriptor>> {
        self.drain()?;
        self.negotiations.pop_front().transpose()
    }
    fn media(
        &mut self,
        context: &AuthenticatedContext,
        packet: crate::transport::Packet<'_>,
        _now_ms: u64,
    ) -> Result<()> {
        self.check(context)?;
        // Drop on bounded media pressure. Missing wet follows the host's fade;
        // it never stalls source progress or consumes REC capacity.
        let _ = self.media_in.push(packet.bytes().to_vec());
        Ok(())
    }
    fn disconnect(&mut self, _context: &AuthenticatedContext) {
        self.shared.disconnected.store(true, Ordering::Release);
    }
    fn source_frame(&self) -> Option<u64> {
        match self.shared.source_frame.load(Ordering::Acquire) {
            u64::MAX => None,
            frame => Some(frame),
        }
    }
    fn poll_reply(
        &mut self,
        _context: &AuthenticatedContext,
        _now_ms: u64,
    ) -> Result<Option<Value>> {
        self.drain()?;
        self.replies.pop_front().transpose()
    }
    fn poll_media(&mut self, _context: &AuthenticatedContext) -> Result<Option<Vec<u8>>> {
        Ok(self.media_out.pop().ok())
    }
}
impl AuthorityMailbox {
    pub fn retired(&self) -> bool {
        self.retired
    }
    fn retire<A: AuthorityEndpoint>(&mut self, authority: &mut A) {
        self.shared.disconnected.store(true, Ordering::Release);
        if !self.retired {
            authority.disconnect(&self.context);
            self.retired = true;
        }
        while self.commands.pop().is_ok() {}
        while self.media_in.pop().is_ok() {}
    }
    /// Owner must call this outside render before processing its next block,
    /// including after a network worker exits. Retirement cannot be lost behind
    /// a full queue. The owner's monotonic clock is used for all actual leases.
    pub fn service<A: AuthorityEndpoint>(&mut self, authority: &mut A, now_ms: u64) -> Result<()> {
        if self.retired {
            return Ok(());
        }
        if self.shared.disconnected.load(Ordering::Acquire) || self.context.check_current().is_err()
        {
            self.retire(authority);
            return Ok(());
        }
        if authority.identity() != self.identity {
            // Drain the applied structural result before invalidating its map.
            for _ in 0..128 {
                let Some(reply) = authority.poll_reply(&self.context, now_ms)? else {
                    break;
                };
                if self.events.push(Event::Reply(Ok(reply))).is_err() {
                    self.retire(authority);
                    return Err("generation completion capacity".into());
                }
            }
            authority.disconnect(&self.context);
            self.retired = true;
            while self.commands.pop().is_ok() {}
            while self.media_in.pop().is_ok() {}
            self.shared.retiring.store(true, Ordering::Release);
            return Ok(());
        }
        self.shared.source_frame.store(
            authority.source_frame().unwrap_or(u64::MAX),
            Ordering::Release,
        );
        let result = self.service_inner(authority, now_ms);
        if result.is_err() {
            self.retire(authority);
        }
        result
    }
    fn service_inner<A: AuthorityEndpoint>(
        &mut self,
        authority: &mut A,
        now_ms: u64,
    ) -> Result<()> {
        for _ in 0..16 {
            let Ok(job) = self.commands.pop() else {
                break;
            };
            // Observe disconnect between queued intents as well as at entry.
            if self.shared.disconnected.load(Ordering::Acquire) {
                self.retire(authority);
                return Ok(());
            }
            let event = match job {
                Job::Command(payload) => {
                    Event::Reply(authority.dispatch(&self.context, payload, now_ms).and_then(
                        |reply| reply.ok_or_else(|| "nested deferred authority unsupported".into()),
                    ))
                }
                Job::Negotiate(descriptor) => {
                    Event::Negotiated(authority.negotiate(&self.context, &descriptor).and_then(
                        |ready| {
                            if ready {
                                Ok(descriptor)
                            } else {
                                Err("nested deferred negotiation unsupported".into())
                            }
                        },
                    ))
                }
            };
            self.events
                .push(event)
                .map_err(|_| "authority result queue full")?;
        }
        for _ in 0..32 {
            let Some(reply) = authority.poll_reply(&self.context, now_ms)? else {
                break;
            };
            self.events
                .push(Event::Reply(Ok(reply)))
                .map_err(|_| "authority completion queue full")?;
        }
        for _ in 0..64 {
            let Ok(bytes) = self.media_in.pop() else {
                break;
            };
            let packet =
                crate::transport::Packet::parse(&bytes).map_err(|_| "host media packet")?;
            if packet.spec().session != self.context.session {
                return Err("host media session".into());
            }
            if authority
                .source_frame()
                .is_some_and(|frame| packet.output_frame() < frame)
            {
                continue;
            }
            authority.media(&self.context, packet, now_ms)?;
        }
        for _ in 0..64 {
            let Some(bytes) = authority.poll_media(&self.context)? else {
                break;
            };
            let _ = self.media_out.push(bytes);
        }
        Ok(())
    }
}
