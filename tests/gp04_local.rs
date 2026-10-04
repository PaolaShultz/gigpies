#![cfg(target_os = "linux")]
use gigpies::{
    analysis_stream::{Descriptor, WIRE_BYTES, WindowCursor, local::monotonic_ms},
    local_audio::LocalAudio,
    show::Counter,
};
use std::{
    io::{Read, Write},
    os::unix::{fs::PermissionsExt, net::UnixStream},
    process::{Command, Stdio},
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
fn attach(path: &std::path::Path) -> UnixStream {
    let mut s = UnixStream::connect(path).unwrap();
    s.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
    let b = br#"{"subscription":"lux.aux.v1","version":1}"#;
    s.write_all(&(b.len() as u32).to_be_bytes()).unwrap();
    s.write_all(b).unwrap();
    s
}
fn frame(s: &mut UnixStream) -> Vec<u8> {
    let mut n = [0; 4];
    s.read_exact(&mut n).unwrap();
    let n = u32::from_be_bytes(n) as usize;
    assert!(n <= WIRE_BYTES);
    let mut bytes = vec![0; n];
    s.read_exact(&mut bytes).unwrap();
    bytes
}
#[test]
fn real_process_delivers_actual_signed_pcm_and_survives_reader_loss() {
    let dir = Directory::new();
    let mut child = Command::new(env!("CARGO_BIN_EXE_gigpies-headless"))
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
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let socket = dir.0.join("analysis.sock");
    for _ in 0..200 {
        if socket.exists() {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    let mut s = attach(&socket);
    let d: Descriptor = serde_json::from_slice(&frame(&mut s)).unwrap();
    d.validate().unwrap();
    let mut cursor = WindowCursor::new(d).unwrap();
    let bytes = frame(&mut s);
    let w = cursor.accept(&bytes, monotonic_ms().unwrap()).unwrap();
    let p = gigpies::transport::Packet::parse(&w.packets[..624]).unwrap();
    let raw = gigpies::analysis_stream::synthetic_inputs(p.source_frame());
    assert_eq!(p.pcm(0).unwrap(), raw[0][0]);
    assert_ne!(p.pcm(1).unwrap(), 0);
    // Stalled consumer cannot stop another attachment or the real engine child.
    let _stalled = attach(&socket);
    drop(s);
    std::thread::sleep(Duration::from_millis(150));
    assert!(child.try_wait().unwrap().is_none());
    let mut fresh = attach(&socket);
    let d: Descriptor = serde_json::from_slice(&frame(&mut fresh)).unwrap();
    let bytes = frame(&mut fresh);
    WindowCursor::new(d)
        .unwrap()
        .accept(&bytes, monotonic_ms().unwrap())
        .unwrap();
    child.kill().unwrap();
    child.wait().unwrap();
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
