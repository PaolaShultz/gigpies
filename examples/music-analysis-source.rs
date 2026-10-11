//! Opt-in synthetic PCM24 through the production Tap/LocalAnalysis path; no devices.
use gigpies::analysis_stream::{
    Descriptor, Tap,
    local::{LocalAnalysis, monotonic_ms},
};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
fn main() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    if !(args.len() == 3 || args.len() == 4 && args[3] == "scenes") {
        return Err(
            "usage: music-analysis-source ABSOLUTE_PRIVATE_DIR SECONDS(1..120) [scenes]".into(),
        );
    }
    let scenes = args.len() == 4;
    let directory = PathBuf::from(&args[1]);
    let seconds: u64 = args[2].parse().map_err(|_| "seconds")?;
    if !(1..=120).contains(&seconds) {
        return Err("seconds bound".into());
    }
    let descriptor = Descriptor::new(9, 0);
    let (mut tap, input) = Tap::new(&descriptor).map_err(String::from)?;
    let mut publisher = LocalAnalysis::bind(&directory, descriptor, input)?;
    let start = Instant::now();
    let mut frame = 0u64;
    let mut next_packet = start;
    let mut reported_second = u64::MAX;
    while start.elapsed() < Duration::from_secs(seconds) {
        // C/F/G/C progression, with a contrasting A-minor section after 12 seconds.
        let second = frame / 48000;
        let phase = scene_phase(frame);
        if second != reported_second {
            eprintln!(
                "source frame={frame} phase={} tap_dropped_windows={}",
                if scenes { phase.0 } else { "harmonic" },
                tap.dropped_windows
            );
            reported_second = second;
        }
        let roots = if scenes || second < 12 {
            [0, 5, 7, 0]
        } else {
            [9, 5, 7, 9]
        };
        let root = roots[((second / 3) % 4) as usize];
        let minor = !scenes && second >= 12 && root == 9;
        let notes = [root, root + if minor { 3 } else { 4 }, root + 7];
        let raw = std::array::from_fn::<_, 48, _>(|offset| {
            let t = (frame + offset as u64) as f64 / 48000.;
            let sine = |midi: i32| {
                (std::f64::consts::TAU * 440. * 2f64.powf((midi as f64 - 69.) / 12.) * t).sin()
            };
            let chord = notes.iter().map(|n| sine(60 + n)).sum::<f64>() / 3.;
            let kick = (std::f64::consts::TAU * 60. * t).sin() * (-20. * (t % 0.5)).exp();
            let gain = if scenes { phase.1 } else { [0.18; 4] };
            let pcm = |x: f64, channel: usize| (x * gain[channel] * 8_388_607.).round() as i32;
            [
                pcm(kick, 0),
                pcm(sine(36 + root), 1),
                pcm(chord, 2),
                pcm(chord, 3),
                0,
                0,
                0,
                0,
            ]
        });
        let now = monotonic_ms()?;
        tap.offer(frame, &raw, now).map_err(String::from)?;
        publisher.tick(now, tap.dropped_windows)?;
        frame += 48;
        // A synthetic producer has no captured backlog. Never manufacture a burst
        // to repay scheduler delay: source frames continue, wall time may stretch.
        next_packet += Duration::from_millis(1);
        let now = Instant::now();
        if next_packet <= now {
            next_packet = now + Duration::from_millis(1);
        }
        std::thread::sleep(next_packet.duration_since(now));
    }
    Ok(())
}

/// Declared synthetic arrangements, never labels injected into Lux. The returning
/// quiet arrangement has identical balance/level and cycles the same C/F/G chords.
/// Chord roots change inside every phase, including stable quiet and drive.
fn scene_phase(frame: u64) -> (&'static str, [f64; 4]) {
    let seconds = frame as f64 / 48000.;
    match seconds {
        t if t < 3. => ("calibration", [0.18; 4]),
        t if t < 19. => ("quiet-a", [0.018, 0.018, 0.025, 0.008]),
        t if t < 35. => {
            let gain = 0.025 + (t - 19.) / 16. * 0.155;
            ("build", [gain; 4])
        }
        t if t < 51. => ("drive-b", [0.18; 4]),
        t if t < 67. => ("quiet-a-return", [0.018, 0.018, 0.025, 0.008]),
        t if t < 83. => ("floating-c", [0.006, 0.050, 0.035, 0.090]),
        _ => ("drive-b-return", [0.18; 4]),
    }
}

#[cfg(test)]
mod tests {
    use super::scene_phase;

    #[test]
    fn returning_arrangements_are_identical_without_transmitted_labels() {
        assert_eq!(scene_phase(4 * 48000).1, scene_phase(54 * 48000).1);
        assert_eq!(scene_phase(40 * 48000).1, scene_phase(90 * 48000).1);
        assert_ne!(scene_phase(4 * 48000).1, scene_phase(40 * 48000).1);
    }

    #[test]
    fn finite_gain_and_sustained_build_are_bounded() {
        let mut prior = 0.;
        for second in 0..120 {
            let (phase, gains) = scene_phase(second * 48000);
            assert!(
                gains
                    .iter()
                    .all(|v| v.is_finite() && (0.0..=0.18).contains(v))
            );
            if phase == "build" {
                assert!(gains[0] > prior);
                prior = gains[0];
            }
        }
    }
}
