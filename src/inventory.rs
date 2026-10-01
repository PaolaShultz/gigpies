//! Read-only WAV header inventory. This does not validate the sample payload,
//! decode audio, measure levels, or infer alignment from equal durations.
use serde::Serialize;
use std::path::{Path, PathBuf};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Debug, Serialize)]
pub struct WavInfo {
    pub path: PathBuf,
    pub channels: u16,
    pub sample_rate_hz: u32,
    pub bits_per_sample: u16,
    pub sample_format: &'static str,
    pub frames: u32,
    pub duration_seconds: f64,
}

pub fn inspect_wav(path: &Path) -> Result<WavInfo> {
    let reader = hound::WavReader::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let spec = reader.spec();
    if spec.channels == 0 || spec.sample_rate == 0 {
        return Err(format!("{}: invalid channel count or sample rate", path.display()).into());
    }
    let frames = reader.duration();
    Ok(WavInfo {
        path: path.to_owned(),
        channels: spec.channels,
        sample_rate_hz: spec.sample_rate,
        bits_per_sample: spec.bits_per_sample,
        sample_format: match spec.sample_format {
            hound::SampleFormat::Int => "integer",
            hound::SampleFormat::Float => "float",
        },
        frames,
        duration_seconds: f64::from(frames) / f64::from(spec.sample_rate),
    })
}

/// Inspect one WAV or the immediate WAV files in a directory, sorted by path.
/// A malformed member fails the whole inventory; partial output is not returned.
pub fn inspect(path: &Path) -> Result<Vec<WavInfo>> {
    if !path.is_dir() {
        return Ok(vec![inspect_wav(path)?]);
    }
    let mut paths = Vec::new();
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let candidate = entry.path();
        if entry.file_type()?.is_file()
            && candidate
                .extension()
                .is_some_and(|s| s.eq_ignore_ascii_case("wav"))
        {
            paths.push(candidate);
        }
    }
    paths.sort();
    if paths.is_empty() {
        return Err(format!("{}: no WAV files in directory", path.display()).into());
    }
    paths.iter().map(|p| inspect_wav(p)).collect()
}
