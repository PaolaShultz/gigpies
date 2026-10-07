#![cfg(target_os = "linux")]
use gigpies::{
    analysis_stream::{Descriptor, WIRE_BYTES, WindowCursor, local::monotonic_ms},
    local_audio::LocalAudio,
    show::Counter,
};
use std::{
    io::{Read, Write},
    os::unix::{fs::PermissionsExt, net::UnixStream},
    process::{Child, Command, Stdio},
    time::Duration,
};
static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
struct Directory(std::path::PathBuf);
impl Directory {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "gp04-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir(&p).unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o700)).unwrap();
        Self(p)
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
// Drain only a bounded tail, and always reap on failed assertions or reads.
struct Process {
    child: Child,
    stderr: Option<std::thread::JoinHandle<Vec<u8>>>,
}
impl Process {
    fn new(mut child: Child) -> Self {
        let mut pipe = child.stderr.take().unwrap();
        let stderr = std::thread::spawn(move || {
            let mut tail = Vec::new();
            let mut bytes = [0; 1024];
            while let Ok(n) = pipe.read(&mut bytes) {
                if n == 0 {
                    break;
                }
                tail.extend_from_slice(&bytes[..n]);
                if tail.len() > 8192 {
                    tail.drain(..tail.len() - 8192);
                }
            }
            tail
        });
        Self {
            child,
            stderr: Some(stderr),
        }
    }
}
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(reader) = self.stderr.take() {
            eprintln!(
                "headless stderr: {}",
                String::from_utf8_lossy(&reader.join().unwrap())
            );
        }
    }
}
fn attach(path: &std::path::Path) -> UnixStream {
    let mut s = UnixStream::connect(path).unwrap();
    s.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
    let b = br#"{"subscription":"lux.aux.v1","version":1}"#;
    s.write_all(&(b.len() as u32).to_be_bytes()).unwrap();
    s.write_all(b).unwrap();
    s
}
fn frame(s: &mut UnixStream, stage: &str) -> Vec<u8> {
    let mut n = [0; 4];
    s.read_exact(&mut n)
        .unwrap_or_else(|e| panic!("{stage}: length prefix: {e}"));
    let n = u32::from_be_bytes(n) as usize;
    assert!(n <= WIRE_BYTES);
    let mut bytes = vec![0; n];
    s.read_exact(&mut bytes)
        .unwrap_or_else(|e| panic!("{stage}: body {n} bytes: {e}"));
    bytes
}
#[test]
fn real_process_delivers_actual_signed_pcm_and_survives_reader_loss() {
    let dir = Directory::new();
    let mut child = Process::new(
        Command::new(env!("CARGO_BIN_EXE_gigpies-headless"))
            .args([
                "--directory",
                dir.0.to_str().unwrap(),
                "--show",
                "11111111-1111-4111-8111-111111111111",
                "--epoch",
                "9",
                "--synthetic-source",
                "fouraux",
                "--analysis",
                "--ticks",
                "2000",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let socket = dir.0.join("analysis.sock");
    for _ in 0..200 {
        if socket.exists() {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    let mut s = attach(&socket);
    let d: Descriptor = serde_json::from_slice(&frame(&mut s, "initial descriptor")).unwrap();
    d.validate().unwrap();
    let mut cursor = WindowCursor::new(d).unwrap();
    let bytes = frame(&mut s, "initial window");
    let w = cursor.accept(&bytes, monotonic_ms().unwrap()).unwrap();
    let p = gigpies::transport::Packet::parse(&w.packets[..624]).unwrap();
    let raw = gigpies::analysis_stream::synthetic_inputs(p.source_frame());
    assert_eq!(p.pcm(0).unwrap(), raw[0][0]);
    assert_ne!(p.pcm(1).unwrap(), 0);
    // Stalled consumer cannot stop another attachment or the real engine child.
    let _stalled = attach(&socket);
    drop(s);
    std::thread::sleep(Duration::from_millis(150));
    assert!(child.child.try_wait().unwrap().is_none());
    let mut fresh = attach(&socket);
    let d: Descriptor =
        serde_json::from_slice(&frame(&mut fresh, "reattached descriptor")).unwrap();
    let bytes = frame(&mut fresh, "reattached window");
    WindowCursor::new(d)
        .unwrap()
        .accept(&bytes, monotonic_ms().unwrap())
        .unwrap();
    drop(child);
}
#[test]
fn transport_stop_preserves_engine_and_analysis_does_not_select_source() {
    let dir = Directory::new();
    let mut audio = LocalAudio::bind(
        &dir.0,
        "audio.sock",
        "11111111-1111-4111-8111-111111111111",
        Counter(9),
    )
    .unwrap();
    assert!(audio.enable_analysis(&dir.0).is_err());
    audio.enable_synthetic_fouraux().unwrap();
    audio.enable_analysis(&dir.0).unwrap();
    audio.tick(0).unwrap();
    audio.stop_analysis();
    for _ in 0..100 {
        if !audio.analysis_alive() {
            break;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(!audio.analysis_alive());
    for i in 1..100 {
        audio.tick(i).unwrap();
    }
    assert_eq!(audio.frame(), 4800);
    assert!(audio.snapshot().is_ok());
}
