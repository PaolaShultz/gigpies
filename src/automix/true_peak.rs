//! Offline, streaming windowed-sinc peak estimation following BS.1770 Annex 2.
//! This observes samples; its interpolation delay/tail never changes an audio file.
use super::dsp::{Loudness, Meter, db};
use crate::inventory::Result;
use serde::{Deserialize, Serialize};
use std::{f64::consts::PI, path::Path};

pub const ALGORITHM: &str = "gigpies-blackman-sinc64-8x-min192k-v1";
const TAPS: usize = 64;
const HALF: usize = TAPS / 2;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Peak {
    pub amplitude: f64,
    pub channel: usize,
    /// Fractional input frame. Negative/outside-file positions describe zero extension.
    pub frame: f64,
}
impl Default for Peak {
    fn default() -> Self {
        Self {
            amplitude: 0.,
            channel: 0,
            frame: 0.,
        }
    }
}

pub struct TruePeak {
    coefficients: Vec<[f64; TAPS]>,
    ring: [[f64; 2]; TAPS * 2],
    cursor: usize,
    pub frames: u64,
    pub sample_peak: Peak,
    pub true_peak: Peak,
}
impl TruePeak {
    pub fn new(rate: u32) -> Result<Self> {
        if !(8000..=192000).contains(&rate) {
            return Err("true-peak rate must be 8–192 kHz".into());
        }
        let factor = 192000_u32.div_ceil(rate).next_power_of_two().max(8) as usize;
        let coefficients = (1..factor)
            .map(|phase| {
                let fraction = phase as f64 / factor as f64;
                let mut row = std::array::from_fn(|k| {
                    let t = k as f64 - (HALF - 1) as f64 - fraction;
                    let sinc = if t.abs() < 1e-14 {
                        1.
                    } else {
                        (PI * t).sin() / (PI * t)
                    };
                    let window = 0.42
                        + 0.5 * (PI * t / HALF as f64).cos()
                        + 0.08 * (2. * PI * t / HALF as f64).cos();
                    sinc * window
                });
                let dc: f64 = row.iter().sum();
                for v in &mut row {
                    *v /= dc;
                }
                row
            })
            .collect();
        Ok(Self {
            coefficients,
            ring: [[0.; 2]; TAPS * 2],
            cursor: 0,
            frames: 0,
            sample_peak: Peak::default(),
            true_peak: Peak::default(),
        })
    }
    pub fn factor(&self) -> usize {
        self.coefficients.len() + 1
    }
    pub fn add(&mut self, x: [f64; 2]) -> Result<()> {
        if x.iter().any(|v| !v.is_finite() || v.abs() > 1e100) {
            return Err("nonfinite/overflowing meter input".into());
        }
        for (channel, v) in x.into_iter().enumerate() {
            if v.abs() > self.sample_peak.amplitude {
                self.sample_peak = Peak {
                    amplitude: v.abs(),
                    channel,
                    frame: self.frames as f64,
                };
            }
            if v.abs() > self.true_peak.amplitude {
                self.true_peak = Peak {
                    amplitude: v.abs(),
                    channel,
                    frame: self.frames as f64,
                };
            }
        }
        self.interpolate(x);
        Ok(())
    }
    fn interpolate(&mut self, x: [f64; 2]) {
        self.ring[self.cursor] = x;
        self.ring[self.cursor + TAPS] = x;
        self.cursor = (self.cursor + 1) % TAPS;
        let history = &self.ring[self.cursor..self.cursor + TAPS];
        for (phase, row) in self.coefficients.iter().enumerate() {
            let mut sum = [0.; 2];
            for (a, h) in row.iter().zip(history) {
                sum[0] += a * h[0];
                sum[1] += a * h[1];
            }
            for (channel, v) in sum.into_iter().enumerate() {
                if v.abs() > self.true_peak.amplitude {
                    self.true_peak = Peak {
                        amplitude: v.abs(),
                        channel,
                        frame: self.frames as f64 - HALF as f64
                            + (phase + 1) as f64 / self.factor() as f64,
                    };
                }
            }
        }
        self.frames += 1;
    }
    pub fn finish(mut self) -> (Peak, Peak) {
        // Includes the complete zero-extended FIR tail, even for one-frame files.
        for _ in 0..TAPS {
            self.interpolate([0.; 2]);
        }
        (self.sample_peak, self.true_peak)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Measurement {
    pub algorithm: String,
    pub sample_rate: u32,
    pub frames: u64,
    pub interpolation_factor: usize,
    pub sample_peak: Peak,
    pub true_peak: Peak,
    pub sample_peak_dbfs: f64,
    pub true_peak_dbtp: f64,
    pub rms_dbfs: f64,
    pub integrated_lufs: Option<f64>,
    pub short_term_max_lufs: Option<f64>,
}
pub fn measure(path: &Path) -> Result<Measurement> {
    let mut reader = hound::WavReader::open(path)?;
    let spec = reader.spec();
    if spec.channels != 2
        || reader.len() % 2 != 0
        || !(spec.sample_format == hound::SampleFormat::Float && spec.bits_per_sample == 32
            || spec.sample_format == hound::SampleFormat::Int
                && [16, 24, 32].contains(&spec.bits_per_sample))
    {
        return Err("meter requires stereo float32 or PCM16/24/32".into());
    }
    let mut tp = TruePeak::new(spec.sample_rate)?;
    let factor = tp.factor();
    let mut meter = Meter::default();
    let mut loud = Loudness::new(spec.sample_rate);
    let frames = reader.duration() as u64;
    for _ in 0..frames {
        let mut x = [0.; 2];
        for v in &mut x {
            *v = if spec.sample_format == hound::SampleFormat::Float {
                reader.samples::<f32>().next().ok_or("short WAV")?? as f64
            } else {
                reader.samples::<i32>().next().ok_or("short WAV")?? as f64
                    / 2_f64.powi(spec.bits_per_sample as i32 - 1)
            };
        }
        tp.add(x)?;
        for v in x {
            meter.add(v);
        }
        loud.add(x);
    }
    let (sample_peak, true_peak) = tp.finish();
    Ok(Measurement {
        algorithm: ALGORITHM.into(),
        sample_rate: spec.sample_rate,
        frames,
        interpolation_factor: factor,
        sample_peak_dbfs: db(sample_peak.amplitude),
        true_peak_dbtp: db(true_peak.amplitude),
        sample_peak,
        true_peak,
        rms_dbfs: db(meter.rms()),
        integrated_lufs: loud.integrated(),
        short_term_max_lufs: loud.short_term_max(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn phased_tones_sides_rates_and_known_intersample_over() {
        // EBU Tech3341 cases 15–19; tighter 0.2 dB engineering limit declared
        // before evaluation. Tapering avoids conflating abrupt-edge reconstruction.
        for rate in [8000, 11025, 22050, 44100, 48000, 88200, 96000, 192000] {
            for (period, phase, amplitude) in [
                (4., 0., 0.5),
                (4., 45., 0.5),
                (6., 60., 0.5),
                (8., 67.5, 0.5),
                (4., 45., 1.41),
            ] {
                let mut m = TruePeak::new(rate).unwrap();
                assert!(m.factor() * rate as usize >= 192000);
                for n in 0..2048 {
                    let fade = (n as f64 / 512.).min((2047 - n) as f64 / 512.).min(1.);
                    let x =
                        amplitude * fade * (2. * PI * n as f64 / period + phase * PI / 180.).sin();
                    m.add([x * 0.25, x]).unwrap();
                }
                let (s, t) = m.finish();
                assert_eq!(t.channel, 1);
                assert!((db(t.amplitude) - db(amplitude)).abs() < 0.2);
                if period == 4. && phase == 45. {
                    assert!(db(t.amplitude / s.amplitude) > 2.9);
                }
            }
        }
    }
    #[test]
    fn silence_short_edges_nonfinite_and_block_independence() {
        assert!(TruePeak::new(7999).is_err());
        assert!(TruePeak::new(192001).is_err());
        for len in [0, 1, 2, 31, 64, 65, 1001] {
            let data: Vec<_> = (0..len)
                .map(|n| {
                    if n == 0 || n + 1 == len {
                        [0., 0.9]
                    } else {
                        [0.; 2]
                    }
                })
                .collect();
            let mut a = TruePeak::new(48000).unwrap();
            let mut b = TruePeak::new(48000).unwrap();
            for x in &data {
                a.add(*x).unwrap();
            }
            for chunk in data.chunks(17) {
                for x in chunk {
                    b.add(*x).unwrap();
                }
            }
            let a = a.finish();
            assert_eq!(a, b.finish());
            assert!(a.1.amplitude >= a.0.amplitude);
            if len == 0 {
                assert_eq!(a.1.amplitude, 0.);
            } else {
                assert!(a.1.amplitude >= 0.9);
            }
        }
        let mut x = TruePeak::new(48000).unwrap();
        assert!(x.add([f64::NAN, 0.]).is_err());
        assert!(x.add([0., f64::INFINITY]).is_err());
    }
}
