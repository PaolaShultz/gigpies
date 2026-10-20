//! Explicit deterministic software-audio evidence through the real provider. No devices.
use gigpies::{local_audio::LocalAudio, show::Counter, topology::EngineTopology};
use std::{
    path::Path,
    time::{Duration, Instant},
};
fn main() -> Result<(), String> {
    let a: Vec<_> = std::env::args().collect();
    if a.len() != 5 {
        return Err("spectrum-source PRIVATE_DIRECTORY SHOW_UUID EPOCH tones|sequence|hot".into());
    }
    let epoch = a[3].parse().map_err(|_| "epoch")?;
    if !matches!(a[4].as_str(), "tones" | "sequence" | "hot") {
        return Err("mode".into());
    }
    let t = EngineTopology::legacy();
    let mut capture = vec![0.; 48 * t.capture_channels];
    let mut playback = vec![0.; 48 * t.playback_channels];
    let mut owner = LocalAudio::bind(Path::new(&a[1]), "audio.sock", &a[2], Counter(epoch))?;
    let start = Instant::now();
    for tick in 0..60000u64 {
        let frame = owner.frame();
        let phase = if a[4] == "sequence" {
            (tick / 1500) % 4
        } else {
            0
        };
        let amplitude = match phase {
            0 if a[4] == "hot" => 0.891_250_938, // -1 dBFS peak, software evidence only
            0 => 0.5,
            1 => 0.125,
            2 => 0.,
            _ => 0.25,
        };
        for f in 0..48 {
            for (i, input) in t.inputs.iter().enumerate() {
                // Exact FFT-bin tones; distinct input identity and visible low/high components.
                let k = if i == 0 { 85. } else { 8. + i as f64 * 37. };
                let at = (frame + f as u64) as f64;
                capture[f * t.capture_channels + input.capture_slot] =
                    amplitude * (std::f64::consts::TAU * k * at / 4096.).sin();
            }
        }
        owner.tick_with_capture(
            start.elapsed().as_millis() as u64,
            epoch,
            frame,
            &capture,
            &mut playback,
        )?;
        // Software source pacing only; no PCM handle or output endpoint.
        std::thread::sleep(Duration::from_millis(tick + 1).saturating_sub(start.elapsed()));
    }
    Ok(())
}
