//! Bounded network workers; never called in the render section.
use super::{
    Datagram, Histogram,
    adapters::{Dsp, Result},
    spec, spec_with_delay,
};
use crate::transport::*;
use rtrb::{Consumer, Producer};
use serde_json::json;
use std::{
    net::{SocketAddr, UdpSocket},
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};
fn socket(bind: &str, peer: &str) -> Result<UdpSocket> {
    let s = UdpSocket::bind(bind)?;
    s.connect(peer)?;
    configure_audio_socket(&s)?;
    Ok(s)
}

pub struct ClientConfig {
    pub audio_bind: String,
    pub audio_peer: String,
    pub control_bind: String,
    pub control_peer: String,
    pub epoch: u64,
    pub return_delay_frames: u32,
    pub faults: bool,
}
pub fn client(
    c: ClientConfig,
    mut sends: Consumer<Datagram>,
    mut returns: Producer<Datagram>,
    stop: Arc<AtomicBool>,
    ready: Arc<AtomicBool>,
) -> Result<impl FnOnce() -> serde_json::Value + Send> {
    let audio = socket(&c.audio_bind, &c.audio_peer)?;
    let control = socket(&c.control_bind, &c.control_peer)?;
    let mut authority = Authority::new(c.epoch, 1).map_err(|_| "authority identity")?;
    Ok(move || {
        let start = Instant::now();
        let mut send = 0u64;
        let mut recv = 0u64;
        let mut errors = 0;
        let mut return_drops = 0;
        let mut injected = 0;
        let mut snapshots = 0;
        let mut commands = 0;
        let mut rejected = 0;
        let mut rtt = Histogram::default();
        let mut sent = [None; 2048];
        let mut last_control = Instant::now();
        let mut connected = false;
        while !stop.load(Ordering::Acquire) {
            for _ in 0..32 {
                let Ok(d) = sends.pop() else { break };
                let Ok(p) = Packet::parse(&d.bytes[..d.len]) else {
                    errors += 1;
                    continue;
                };
                let f = p.source_frame();
                if c.faults && ((96000..96960).contains(&f) || f % 48000 == 24000) {
                    injected += 1;
                    continue;
                }
                sent[(p.sequence() % 2048) as usize] = Some((f, Instant::now()));
                if audio.send(&d.bytes[..d.len]).is_ok() {
                    send += 1;
                } else {
                    errors += 1;
                }
                if c.faults && f % 48000 == 12000 {
                    let _ = audio.send(&d.bytes[..d.len]);
                    injected += 1;
                }
                if c.faults && f % 48000 == 18000 {
                    let mut bad = d;
                    bad.bytes[16..24].copy_from_slice(&c.epoch.wrapping_add(1).to_be_bytes());
                    let _ = audio.send(&bad.bytes[..bad.len]);
                    injected += 1;
                }
            }
            for _ in 0..64 {
                let mut d = Datagram::default();
                match audio.recv(&mut d.bytes) {
                    Ok(n) => {
                        d.len = n;
                        if let Ok(p) = Packet::parse(&d.bytes[..n]) {
                            if p.spec()
                                != spec_with_delay(c.epoch, Role::WetReturn, c.return_delay_frames)
                            {
                                rejected += 1;
                                continue;
                            }
                            if let Some((f, t)) = sent[(p.sequence() % 2048) as usize]
                                && f == p.source_frame()
                            {
                                rtt.observe(t.elapsed().as_secs_f64() * 1e6);
                            }
                            recv += 1;
                            if returns.push(d).is_err() {
                                return_drops += 1;
                            }
                        } else {
                            rejected += 1;
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                    Err(_) => {
                        errors += 1;
                        break;
                    }
                }
            }
            for _ in 0..16 {
                let mut b = [0; 128];
                match control.recv(&mut b) {
                    Ok(n) => {
                        let ack = if let Ok(h) = Hello::parse(&b[..n]) {
                            authority.hello(h).ok().inspect(|_| {
                                snapshots += 1;
                            })
                        } else if let Ok(cmd) = Command::parse(&b[..n]) {
                            let ack = authority.apply(cmd);
                            if ack.disposition == Disposition::Applied {
                                commands += 1;
                                ready.store(true, Ordering::Release);
                            }
                            Some(ack)
                        } else {
                            None
                        };
                        if let Some(ack) = ack {
                            last_control = Instant::now();
                            connected = true;
                            let _ = control.send(&ack.encode());
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                    Err(_) => {
                        errors += 1;
                        break;
                    }
                }
            }
            if connected && last_control.elapsed() > Duration::from_millis(500) {
                authority.disconnect();
                connected = false;
            }
            thread::sleep(Duration::from_micros(80));
        }
        json!({"sent":send,"received":recv,"errors":errors,"return_queue_drops":return_drops,"injected":injected,"rejected":rejected,"snapshots":snapshots,"commands":commands,"authority":authority.snapshot(),"rtt":rtt.summary(),"seconds":start.elapsed().as_secs_f64()})
    })
}

pub struct BrainConfig<'a> {
    pub bind: &'a str,
    pub peer: &'a str,
    pub control_bind: &'a str,
    pub control_peer: &'a str,
    pub epoch: u64,
    pub return_delay_frames: u32,
    pub seconds: f64,
    pub lease: u64,
    pub faults: bool,
    pub library: &'a Path,
}
pub fn brain(c: BrainConfig<'_>) -> Result<serde_json::Value> {
    if ![384, 768].contains(&c.return_delay_frames) {
        return Err("return delay must be 384 or 768 frames".into());
    }
    let audio = socket(c.bind, c.peer)?;
    let control = socket(c.control_bind, c.control_peer)?;
    let mut fx = Dsp::load(c.library, "fx", 48)?;
    let mut writer = ControlWriter::new(c.epoch, 1, c.lease).map_err(|_| "control identity")?;
    let start = Instant::now();
    let mut next = None;
    let mut processed = 0;
    let mut rejected = 0;
    let mut gaps = 0;
    let mut errors = 0;
    let mut snapshots = 0;
    let mut commands = 0;
    let mut process = Histogram::default();
    let mut last_poll = Instant::now();
    let mut stall = false;
    let mut input = [0.; 96];
    let mut output = [0.; 96];
    eprintln!(
        "ready pid={} audio={} control={}",
        std::process::id(),
        c.bind,
        c.control_bind
    );
    while start.elapsed().as_secs_f64() < c.seconds {
        if c.faults && !stall && start.elapsed().as_secs_f64() > 4.0 {
            thread::sleep(Duration::from_millis(250));
            stall = true;
        }
        for _ in 0..32 {
            let mut d = Datagram::default();
            match audio.recv(&mut d.bytes) {
                Ok(n) => {
                    let Ok(p) = Packet::parse(&d.bytes[..n]) else {
                        rejected += 1;
                        continue;
                    };
                    if p.spec() != spec(c.epoch, Role::FxSend)
                        || p.sequence() != (p.source_frame() / 48) as u32
                        || next.is_some_and(|f| p.source_frame() < f)
                    {
                        rejected += 1;
                        continue;
                    }
                    if next.is_some_and(|f| p.source_frame() != f) {
                        fx.reset();
                        gaps += 1;
                    }
                    next = Some(p.source_frame() + 48);
                    for (i, x) in input.iter_mut().enumerate() {
                        *x = p.sample(i).unwrap();
                    }
                    if !writer.synchronized() {
                        fx.reset();
                        continue;
                    }
                    let t = Instant::now();
                    if !fx.process(&input, &mut output, 2) {
                        errors += 1;
                        fx.reset();
                        continue;
                    }
                    process.observe(t.elapsed().as_secs_f64() * 1e6);
                    let samples = output.map(|x| x as f32);
                    let mut result = Datagram::default();
                    result.len = spec_with_delay(c.epoch, Role::WetReturn, c.return_delay_frames)
                        .encode_float(p.source_frame(), p.sequence(), &samples, &mut result.bytes)
                        .unwrap();
                    if audio.send(&result.bytes[..result.len]).is_err() {
                        errors += 1;
                    }
                    processed += 1;
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(_) => {
                    errors += 1;
                    break;
                }
            }
        }
        let now = start.elapsed().as_nanos() as u64;
        if last_poll.elapsed() >= Duration::from_millis(20) {
            if let Ok(Some(request)) = writer.poll(now, -6000) {
                let mut bytes = [0; CONTROL_BYTES];
                let n = request.encode(&mut bytes);
                let _ = control.send(&bytes[..n]);
            }
            last_poll = Instant::now();
        }
        for _ in 0..16 {
            let mut b = [0; 128];
            match control.recv(&mut b) {
                Ok(n) => {
                    if let Ok(ack) = Ack::parse(&b[..n]) {
                        match writer.accept(ack, start.elapsed().as_nanos() as u64) {
                            Ok(WriterEvent::Snapshot) => snapshots += 1,
                            Ok(WriterEvent::Applied { .. }) => commands += 1,
                            _ => {}
                        }
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(_) => break,
            }
        }
        thread::sleep(Duration::from_micros(80));
    }
    Ok(
        json!({"processed":processed,"rejected":rejected,"gaps_reset":gaps,"errors":errors,"snapshots":snapshots,"commands":commands,"fx_delay_frames":fx.delay,"return_delay_frames":c.return_delay_frames,"process":process.summary(),"seconds":start.elapsed().as_secs_f64()}),
    )
}
/// Validate explicit unicast addresses; wildcard/default routing is never selected.
pub fn validate_addresses(bind: &str, peer: &str) -> Result<()> {
    let b: SocketAddr = bind.parse()?;
    let p: SocketAddr = peer.parse()?;
    if b.ip().is_unspecified()
        || p.ip().is_unspecified()
        || b.port() == 0
        || p.port() == 0
        || b.ip().is_multicast()
        || p.ip().is_multicast()
    {
        return Err("explicit unicast endpoints required".into());
    }
    Ok(())
}
