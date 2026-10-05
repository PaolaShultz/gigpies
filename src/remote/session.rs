use super::*;
use crate::show::Counter;
use serde_json::Value;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::sync::{OwnedSemaphorePermit, Semaphore, mpsc};

/// Implement this on the actual processing authority, not on GPC1's scalar
/// prototype. Dispatch must check payload-derived permissions, writer, leases,
/// revision and retry history before scheduling a real sample boundary.
pub trait AuthorityEndpoint {
    fn retiring(&self) -> bool {
        false
    }
    fn identity(&self) -> EngineIdentity;
    fn dispatch(
        &mut self,
        context: &AuthenticatedContext,
        payload: Value,
        now_ms: u64,
    ) -> Result<Option<Value>>;
    /// Validate actual tap/channel IDs, FX routes and permissions against the
    /// admitted engine topology. Transport-only shape validation is insufficient.
    fn negotiate(
        &mut self,
        context: &AuthenticatedContext,
        descriptor: &MediaDescriptor,
    ) -> Result<bool>;
    fn poll_negotiation(&mut self) -> Result<Option<MediaDescriptor>> {
        Ok(None)
    }
    fn poll_refusal(&mut self) -> Result<Option<String>> {
        Ok(None)
    }
    /// Packet already authenticated, negotiated and duplicate checked. Host must
    /// admit it to its independent bounded worker queue, never advance its clock.
    fn media(
        &mut self,
        context: &AuthenticatedContext,
        packet: crate::transport::Packet<'_>,
        now_ms: u64,
    ) -> Result<()>;
    fn disconnect(&mut self, context: &AuthenticatedContext);
    fn source_frame(&self) -> Option<u64> {
        None
    }
    fn poll_reply(
        &mut self,
        _context: &AuthenticatedContext,
        _now_ms: u64,
    ) -> Result<Option<Value>> {
        Ok(None)
    }
    fn poll_media(&mut self, _context: &AuthenticatedContext) -> Result<Option<Vec<u8>>> {
        Ok(None)
    }
}

pub struct RemoteServer {
    endpoint: quinn::Endpoint,
    policy: PolicyStore,
    slots: Arc<Semaphore>,
}
impl RemoteServer {
    pub fn bind(
        address: SocketAddr,
        credentials: &Credentials,
        policy: PolicyStore,
    ) -> Result<Self> {
        Ok(Self {
            endpoint: server_endpoint(address, credentials)?,
            policy,
            slots: Arc::new(Semaphore::new(MAX_CONNECTIONS)),
        })
    }
    pub fn local_addr(&self) -> Result<SocketAddr> {
        self.endpoint.local_addr().map_err(|e| e.to_string())
    }
    pub fn close(&self) {
        self.endpoint.close(0u8.into(), b"operator stop");
    }
    pub async fn accept(&self) -> Result<ServerSession> {
        let incoming = self.endpoint.accept().await.ok_or("endpoint closed")?;
        let permit = match self.slots.clone().try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => {
                incoming.refuse();
                return Err("remote connection capacity".into());
            }
        };
        validate_private_address(incoming.remote_address())?;
        let connection = tokio::time::timeout(Duration::from_millis(IO_TIMEOUT_MS), incoming)
            .await
            .map_err(|_| "TLS handshake timeout")?
            .map_err(|e| e.to_string())?;
        let context = match authenticated(&connection, &self.policy) {
            Ok(context) => context,
            Err(error) => {
                connection.close(1u8.into(), b"peer authorization refused");
                return Err(error);
            }
        };
        Ok(ServerSession {
            connection,
            context,
            policy: self.policy.clone(),
            _permit: permit,
        })
    }
}
fn session_id(connection: &quinn::Connection) -> Result<u64> {
    let mut bytes = [0; 8];
    connection
        .export_keying_material(&mut bytes, b"gigpies-remote-session-v1", b"")
        .map_err(|_| "TLS session export")?;
    let session = u64::from_be_bytes(bytes);
    if session == 0 {
        return Err("zero TLS session; reconnect".into());
    }
    Ok(session)
}
fn authenticated(
    connection: &quinn::Connection,
    policy: &PolicyStore,
) -> Result<AuthenticatedContext> {
    let identity = connection
        .peer_identity()
        .ok_or("TLS peer certificate required")?;
    let certificates = identity
        .downcast::<Vec<rustls::pki_types::CertificateDer<'static>>>()
        .map_err(|_| "TLS peer identity type")?;
    let certificate = certificates
        .first()
        .ok_or("TLS peer certificate required")?;
    policy.authenticate(certificate.as_ref(), session_id(connection)?)
}

pub struct ServerSession {
    connection: quinn::Connection,
    context: AuthenticatedContext,
    policy: PolicyStore,
    _permit: OwnedSemaphorePermit,
}
impl ServerSession {
    pub fn context(&self) -> &AuthenticatedContext {
        &self.context
    }
    /// All child tasks are bounded and aborted/joined before returning. A slow
    /// network client can close this session but cannot block render/REC.
    pub async fn run<A: AuthorityEndpoint>(self, authority: &mut A) -> Result<()> {
        let result = self.run_inner(authority).await;
        authority.disconnect(&self.context);
        self.connection
            .close(0u8.into(), b"session ended; reacquire required");
        result
    }
    async fn run_inner<A: AuthorityEndpoint>(&self, authority: &mut A) -> Result<()> {
        let identity = authority.identity();
        identity.validate()?;
        self.policy.check(&self.context)?;
        let (mut send, mut recv) = tokio::time::timeout(
            Duration::from_millis(IO_TIMEOUT_MS),
            self.connection.accept_bi(),
        )
        .await
        .map_err(|_| "control stream timeout")?
        .map_err(|e| e.to_string())?;
        match read_frame::<Request>(&mut recv).await? {
            Request::Open { contract, version } if contract == "GP-REMOTE" && version == 1 => (),
            _ => return Err("remote protocol version refused".into()),
        }
        let max_datagram = self
            .connection
            .max_datagram_size()
            .ok_or("QUIC datagrams required")?
            .min(crate::transport::MAX_DATAGRAM);
        write_frame(
            &mut send,
            &Response::Hello {
                contract: "GP-REMOTE".into(),
                version: 1,
                session: Counter(self.context.session),
                writer: self.context.writer.clone(),
                peer_id: self.context.peer_id.clone(),
                policy_generation: Counter(self.context.policy_generation),
                identity: identity.clone(),
                permissions: self.context.permissions.clone(),
                max_datagram: max_datagram as u32,
            },
        )
        .await?;
        let (requests_tx, mut requests_rx) = mpsc::channel(8);
        let (replies_tx, mut replies_rx) = mpsc::channel::<Response>(32);
        let reader = tokio::spawn(async move {
            loop {
                let request = read_frame::<Request>(&mut recv).await;
                let failed = request.is_err();
                if requests_tx.try_send(request).is_err() || failed {
                    break;
                }
            }
        });
        let (writer_error_tx, mut writer_error_rx) = mpsc::channel(1);
        let mut writer = tokio::spawn(async move {
            while let Some(reply) = replies_rx.recv().await {
                if let Err(error) = write_response(&mut send, &reply).await {
                    let _ = writer_error_tx.try_send(error);
                    break;
                }
            }
            let _ = send.finish();
            let _ =
                tokio::time::timeout(Duration::from_millis(IO_TIMEOUT_MS), send.stopped()).await;
        });
        let result = async {
            let mut media = MediaRegistry::new();
            let mut pending_negotiation = None;
            let mut ticker = tokio::time::interval(Duration::from_millis(1));
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                self.policy.check(&self.context)?;
                if authority.retiring() {
                    for _ in 0..128 {
                        let Some(payload) = authority.poll_reply(&self.context, monotonic_ms())? else { break; };
                        tokio::time::timeout(Duration::from_millis(IO_TIMEOUT_MS), replies_tx.send(Response::Reply { session: Counter(self.context.session), payload }))
                            .await.map_err(|_| "generation completion timeout")?.map_err(|_| "generation completion writer")?;
                    }
                    return Ok(());
                }
                if authority.identity() != identity { return Err("engine epoch/capabilities/map changed; reconnect required".into()); }
                let now = monotonic_ms();
                tokio::select! {
                    request = requests_rx.recv() => {
                        let request = request.ok_or("control reader closed")??;
                        self.policy.check(&self.context)?;
                        let response = match request {
                            Request::Command { session, capability_generation, payload } => {
                                if session.0 != self.context.session || capability_generation != identity.capability_generation {
                                    return Err("stale command session/capabilities".into());
                                }
                                self.context.check_writer(&payload)?;
                                match authority.dispatch(&self.context, payload, now) {
                                    Ok(payload) => payload.map(|payload| Response::Reply { session, payload }),
                                    Err(reason) => Some(Response::Refused { session, reason }),
                                }
                            }
                            Request::Negotiate { session, capability_generation, descriptor } => {
                                if session.0 != self.context.session || capability_generation != identity.capability_generation {
                                    return Err("stale media session/capabilities".into());
                                }
                                let result = descriptor.validate(&self.context, &identity, max_datagram)
                                    .and_then(|()| {
                                        if pending_negotiation.is_some() { return Err("media negotiation pending".into()); }
                                        media.check_install(&descriptor)?;
                                        authority.negotiate(&self.context, &descriptor)
                                    });
                                match result {
                                    Ok(true) => {
                                        media.install(descriptor.clone(), &self.context, &identity, max_datagram)?;
                                        Some(Response::MediaAccepted { session, descriptor })
                                    }
                                    Ok(false) => { pending_negotiation = Some(descriptor); None }
                                    Err(reason) => Some(Response::Refused { session, reason }),
                                }
                            }
                            Request::Open { .. } => return Err("repeated remote open".into()),
                        };
                        if let Some(response) = response { replies_tx.try_send(response).map_err(|_| "slow client reply capacity")?; }
                    }
                    bytes = self.connection.read_datagram() => {
                        let bytes = bytes.map_err(|e| e.to_string())?;
                        self.policy.check(&self.context)?;
                        if let Ok(packet) = media.receive(&bytes, MediaSide::ProcessingNode, authority.source_frame()) {
                            authority.media(&self.context, packet, now)?;
                        }
                    }
                    error = writer_error_rx.recv() => return Err(error.unwrap_or_else(|| "control writer closed".into())),
                    _ = ticker.tick() => {
                        match authority.poll_negotiation() {
                            Ok(Some(descriptor)) => {
                                if pending_negotiation.take().as_ref() != Some(&descriptor) { return Err("unexpected media negotiation completion".into()); }
                                media.install(descriptor.clone(), &self.context, &identity, max_datagram)?;
                                replies_tx.try_send(Response::MediaAccepted { session: Counter(self.context.session), descriptor })
                                    .map_err(|_| "slow client reply capacity")?;
                            }
                            Ok(None) => (),
                            Err(reason) => {
                                if pending_negotiation.take().is_none() { return Err(reason); }
                                replies_tx.try_send(Response::Refused { session: Counter(self.context.session), reason })
                                    .map_err(|_| "slow client reply capacity")?;
                            }
                        }
                        for _ in 0..32 {
                            let Some(reason) = authority.poll_refusal()? else { break; };
                            replies_tx.try_send(Response::Refused { session: Counter(self.context.session), reason })
                                .map_err(|_| "slow client reply capacity")?;
                        }
                        // Bounded batches; independent host queues own overrun policy.
                        for _ in 0..32 {
                            let Some(payload) = authority.poll_reply(&self.context, now)? else { break; };
                            replies_tx.try_send(Response::Reply { session: Counter(self.context.session), payload })
                                .map_err(|_| "slow client reply capacity")?;
                        }
                        for _ in 0..64 {
                            let Some(bytes) = authority.poll_media(&self.context)? else { break; };
                            media.validate_outgoing(&bytes, MediaSide::ProcessingNode)?;
                            if bytes.len() > max_datagram { return Err("media datagram capacity".into()); }
                            self.connection.send_datagram(bytes.into()).map_err(|e| e.to_string())?;
                        }
                    }
                }
            }
        }.await;
        reader.abort();
        let _ = reader.await;
        drop(replies_tx);
        if result.is_ok() {
            if tokio::time::timeout(Duration::from_millis(IO_TIMEOUT_MS + 100), &mut writer)
                .await
                .is_err()
            {
                writer.abort();
                let _ = writer.await;
            }
        } else {
            writer.abort();
            let _ = writer.await;
        }
        result
    }
}

/// A client must explicitly create a fresh connection. There is no automatic
/// reconnect/replay, grant resurrection, output rearm or saved command queue.
pub struct RemoteClient {
    _endpoint: quinn::Endpoint,
    connection: quinn::Connection,
    send: quinn::SendStream,
    recv: quinn::RecvStream,
    context: AuthenticatedContext,
    policy: PolicyStore,
    hello: Response,
    identity: EngineIdentity,
    media: MediaRegistry,
    media_generation: Arc<std::sync::atomic::AtomicU64>,
    media_taken: bool,
}
impl RemoteClient {
    pub async fn connect(
        endpoint: quinn::Endpoint,
        address: SocketAddr,
        server_name: &str,
        policy: PolicyStore,
    ) -> Result<Self> {
        validate_private_address(address)?;
        let connection = tokio::time::timeout(
            Duration::from_millis(IO_TIMEOUT_MS),
            endpoint
                .connect(address, server_name)
                .map_err(|e| e.to_string())?,
        )
        .await
        .map_err(|_| "TLS handshake timeout")?
        .map_err(|e| e.to_string())?;
        let context = match authenticated(&connection, &policy) {
            Ok(context) => context,
            Err(error) => {
                connection.close(1u8.into(), b"peer authorization refused");
                return Err(error);
            }
        };
        let (mut send, mut recv) = connection.open_bi().await.map_err(|e| e.to_string())?;
        write_frame(
            &mut send,
            &Request::Open {
                contract: "GP-REMOTE".into(),
                version: 1,
            },
        )
        .await?;
        let hello = read_frame::<Response>(&mut recv).await?;
        let identity = match &hello {
            Response::Hello {
                contract,
                version: 1,
                session,
                writer,
                identity,
                max_datagram,
                ..
            } if contract == "GP-REMOTE"
                && session.0 == context.session
                && crate::show::id(writer)
                && *max_datagram > crate::transport::HEADER_BYTES as u32
                && *max_datagram as usize <= connection.max_datagram_size().unwrap_or(0) =>
            {
                identity.validate()?;
                identity.clone()
            }
            _ => {
                connection.close(1u8.into(), b"invalid remote hello");
                return Err("remote hello refused".into());
            }
        };
        Ok(Self {
            _endpoint: endpoint,
            connection,
            send,
            recv,
            context,
            policy,
            hello,
            identity,
            media: MediaRegistry::new(),
            media_generation: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            media_taken: false,
        })
    }
    pub fn hello(&self) -> &Response {
        &self.hello
    }
    pub fn identity(&self) -> &EngineIdentity {
        &self.identity
    }
    pub fn session(&self) -> u64 {
        self.context.session
    }
    pub fn writer(&self) -> &str {
        match &self.hello {
            Response::Hello { writer, .. } => writer,
            _ => unreachable!(),
        }
    }
    pub fn max_datagram(&self) -> usize {
        match &self.hello {
            Response::Hello { max_datagram, .. } => *max_datagram as usize,
            _ => unreachable!(),
        }
    }
    pub fn close(&self) {
        self.connection.close(0u8.into(), b"operator disconnect");
    }
    fn check(&self) -> Result<()> {
        if self.connection.close_reason().is_some() {
            return Err("remote connection closed".into());
        }
        if let Err(error) = self.policy.check(&self.context) {
            self.connection.close(1u8.into(), b"peer policy revoked");
            return Err(error);
        }
        Ok(())
    }
    pub async fn send_command(&mut self, payload: Value) -> Result<()> {
        self.check()?;
        write_frame(
            &mut self.send,
            &Request::Command {
                session: Counter(self.context.session),
                capability_generation: self.identity.capability_generation,
                payload,
            },
        )
        .await
    }
    pub async fn receive(&mut self) -> Result<Response> {
        // Drain the final reliable boundary result even if QUIC has subsequently
        // closed after acknowledging it. New sends/media still require a live connection.
        self.policy.check(&self.context)?;
        let response: Response = read_response(&mut self.recv).await?;
        self.policy.check(&self.context)?;
        let session = match &response {
            Response::Reply { session, .. }
            | Response::Refused { session, .. }
            | Response::MediaAccepted { session, .. } => *session,
            _ => return Err("unexpected remote hello".into()),
        };
        if session.0 != self.session() {
            return Err("remote response session".into());
        }
        if let Response::MediaAccepted { descriptor, .. } = &response {
            // These permissions describe the local peer's grant ceiling from the
            // authenticated server, not the local policy for server commands.
            self.media.install_received(
                descriptor.clone(),
                self.session(),
                &self.identity,
                self.max_datagram(),
            )?;
            self.media_generation
                .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
            self.media_taken = false;
        }
        Ok(response)
    }
    pub async fn negotiate(&mut self, descriptor: MediaDescriptor) -> Result<()> {
        self.check()?;
        write_frame(
            &mut self.send,
            &Request::Negotiate {
                session: Counter(self.context.session),
                capability_generation: self.identity.capability_generation,
                descriptor,
            },
        )
        .await
    }
    /// Move media consumption to an independent worker after negotiation. A new
    /// negotiated map invalidates every previous channel; control stays separate.
    pub fn media_channel(&mut self) -> Result<MediaChannel> {
        self.check()?;
        if self.media_taken {
            return Err("media channel already owned".into());
        }
        let descriptor = self
            .media
            .descriptor()
            .ok_or("media not negotiated")?
            .clone();
        let mut registry = MediaRegistry::new();
        registry.install_received(
            descriptor,
            self.session(),
            &self.identity,
            self.max_datagram(),
        )?;
        self.media_taken = true;
        Ok(MediaChannel {
            connection: self.connection.clone(),
            context: self.context.clone(),
            policy: self.policy.clone(),
            registry,
            generation: self
                .media_generation
                .load(std::sync::atomic::Ordering::Acquire),
            current_generation: self.media_generation.clone(),
            max_datagram: self.max_datagram(),
        })
    }
    pub fn send_media(&self, bytes: Vec<u8>) -> Result<()> {
        self.check()?;
        self.media.validate_outgoing(&bytes, MediaSide::Brain)?;
        if bytes.len() > self.max_datagram() {
            return Err("media datagram capacity".into());
        }
        self.connection
            .send_datagram(bytes.into())
            .map_err(|e| e.to_string())
    }
    pub async fn receive_media(&mut self) -> Result<Vec<u8>> {
        self.check()?;
        if self.media_taken {
            return Err("media owned by independent channel".into());
        }
        let bytes = self
            .connection
            .read_datagram()
            .await
            .map_err(|e| e.to_string())?;
        self.check()?;
        self.media.receive(&bytes, MediaSide::Brain, None)?;
        Ok(bytes.to_vec())
    }
}
impl Drop for RemoteClient {
    fn drop(&mut self) {
        self.close();
    }
}

/// Authenticated media worker handle. It performs no rendering and owns no clock.
pub struct MediaChannel {
    connection: quinn::Connection,
    context: AuthenticatedContext,
    policy: PolicyStore,
    registry: MediaRegistry,
    generation: u64,
    current_generation: Arc<std::sync::atomic::AtomicU64>,
    max_datagram: usize,
}
impl MediaChannel {
    fn check(&self) -> Result<()> {
        if self.generation
            != self
                .current_generation
                .load(std::sync::atomic::Ordering::Acquire)
            || self.connection.close_reason().is_some()
        {
            return Err("media channel stale or closed".into());
        }
        if let Err(error) = self.policy.check(&self.context) {
            self.connection.close(1u8.into(), b"peer policy revoked");
            return Err(error);
        }
        Ok(())
    }
    pub fn descriptor(&self) -> &MediaDescriptor {
        self.registry.descriptor().expect("admitted descriptor")
    }
    pub fn send(&self, bytes: Vec<u8>) -> Result<()> {
        self.check()?;
        self.registry.validate_outgoing(&bytes, MediaSide::Brain)?;
        if bytes.len() > self.max_datagram {
            return Err("media datagram capacity".into());
        }
        self.connection
            .send_datagram(bytes.into())
            .map_err(|e| e.to_string())
    }
    pub async fn receive(&mut self) -> Result<Vec<u8>> {
        self.check()?;
        loop {
            let bytes = self
                .connection
                .read_datagram()
                .await
                .map_err(|e| e.to_string())?;
            self.check()?;
            if self
                .registry
                .receive(&bytes, MediaSide::Brain, None)
                .is_ok()
            {
                return Ok(bytes.to_vec());
            }
            tokio::task::yield_now().await;
        }
    }
}
