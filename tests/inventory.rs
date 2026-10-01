use std::{
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "gigpies-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn wav(&self, name: &str, channels: u16, rate: u32, frames: u32) -> PathBuf {
        let path = self.0.join(name);
        let spec = hound::WavSpec {
            channels,
            sample_rate: rate,
            bits_per_sample: 24,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(&path, spec).unwrap();
        for i in 0..frames * u32::from(channels) {
            writer.write_sample((i % 101) as i32 - 50).unwrap();
        }
        writer.finalize().unwrap();
        path
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn stereo_duration_counts_frames_not_interleaved_samples() {
    let temp = Scratch::new();
    let path = temp.wav("stereo.wav", 2, 44_100, 441);
    let before = std::fs::read(&path).unwrap();
    let info = gigpies::inventory::inspect_wav(&path).unwrap();
    assert_eq!(
        (
            info.channels,
            info.sample_rate_hz,
            info.bits_per_sample,
            info.frames
        ),
        (2, 44_100, 24, 441)
    );
    assert!((info.duration_seconds - 0.01).abs() < 1e-12);
    assert_eq!(std::fs::read(path).unwrap(), before);
}

#[test]
fn directory_is_sorted_nonrecursive_and_handles_uppercase() {
    let temp = Scratch::new();
    temp.wav("b.WAV", 1, 48_000, 32);
    temp.wav("a.wav", 1, 48_000, 16);
    std::fs::write(temp.0.join("source.txt"), "notice").unwrap();
    std::fs::create_dir(temp.0.join("nested.wav")).unwrap();
    let infos = gigpies::inventory::inspect(&temp.0).unwrap();
    assert_eq!(infos.len(), 2);
    assert_eq!(infos[0].path.file_name().unwrap(), "a.wav");
    assert_eq!(infos[1].frames, 32);
}

#[test]
fn missing_empty_and_malformed_sources_fail() {
    let temp = Scratch::new();
    assert!(gigpies::inventory::inspect(&temp.0).is_err());
    assert!(gigpies::inventory::inspect(&temp.0.join("missing.wav")).is_err());
    temp.wav("valid.wav", 1, 48_000, 10);
    std::fs::write(temp.0.join("bad.wav"), b"not a WAV").unwrap();
    assert!(gigpies::inventory::inspect(&temp.0).is_err());
}

#[test]
fn cli_outputs_json_and_rejects_bad_arguments_without_partial_report() {
    let temp = Scratch::new();
    temp.wav("mono.wav", 1, 48_000, 48);
    let output = Command::new(env!("CARGO_BIN_EXE_gigpies"))
        .arg("inspect")
        .arg(&temp.0)
        .output()
        .unwrap();
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json[0]["frames"], 48);
    for args in [vec!["inspect"], vec!["--version", "extra"], vec!["unknown"]] {
        let output = Command::new(env!("CARGO_BIN_EXE_gigpies"))
            .args(args)
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
    }
    std::fs::write(temp.0.join("broken.wav"), b"bad").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_gigpies"))
        .arg("inspect")
        .arg(&temp.0)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
}
