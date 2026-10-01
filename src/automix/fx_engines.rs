//! Static wet engines adapted from SHR FX; see THIRD_PARTY.md.
use super::effects::{ChorusConfig, ReverbKind};
fn clean(x: f32) -> f32 {
    if x.abs() < 1e-20 { 0.0 } else { x }
}
struct Ring {
    data: Vec<f32>,
    pos: usize,
    valid: usize,
}
impl Ring {
    fn new(len: usize) -> Self {
        Self {
            data: vec![0.0; len.max(4)],
            pos: 0,
            valid: 0,
        }
    }
    fn clear(&mut self) {
        self.pos = 0;
        self.valid = 0;
    }
    fn push(&mut self, value: f32) {
        self.data[self.pos] = clean(value);
        self.pos = (self.pos + 1) % self.data.len();
        self.valid = (self.valid + 1).min(self.data.len());
    }
    fn at(&self, delay: usize) -> f32 {
        if delay == 0 || delay > self.valid {
            0.0
        } else {
            self.data[(self.pos + self.data.len() - delay) % self.data.len()]
        }
    }
    fn read(&self, delay: f32) -> f32 {
        let delay = delay.clamp(1.0, (self.data.len() - 2) as f32);
        let n = delay as usize;
        let a = self.at(n);
        a + (self.at(n + 1) - a) * (delay - n as f32)
    }
}
/// Crossfade both read positions for 20 ms; rapid requests coalesce.
struct Tap {
    from: f32,
    to: f32,
    phase: f32,
    step: f32,
}
impl Tap {
    fn new(delay: f32, rate: f32) -> Self {
        Self {
            from: delay,
            to: delay,
            phase: 1.0,
            step: 1.0 / (rate * 0.02),
        }
    }
    fn advance(&mut self, desired: f32) {
        if self.phase >= 1.0 && (desired - self.to).abs() > 0.1 {
            self.from = self.to;
            self.to = desired;
            self.phase = 0.0;
        }
        self.phase = (self.phase + self.step).min(1.0);
    }
    fn read(&self, ring: &Ring) -> f32 {
        self.offset_read(ring, 1.0, 0.0)
    }
    fn offset_read(&self, ring: &Ring, fraction: f32, offset: f32) -> f32 {
        ring.read(self.from * fraction + offset) * (1.0 - self.phase)
            + ring.read(self.to * fraction + offset) * self.phase
    }
}
/// Smooth parabolic oscillator: deterministic phases, no random source or
/// per-sample transcendental work. Every call advances by less than one cycle.
fn oscillator(phase: &mut f32, step: f32) -> f32 {
    *phase += step;
    if *phase >= 1.0 {
        *phase -= 1.0;
    }
    let x = *phase * 2.0 - 1.0;
    4.0 * x * (1.0 - x.abs())
}
struct Allpass {
    ring: Ring,
    length: f32,
}
impl Allpass {
    fn new(rate: f32, ms: f32) -> Self {
        Self {
            ring: Ring::new((rate * 0.016) as usize + 4),
            length: (rate * ms / 1000.0).round(),
        }
    }
    fn tick(&mut self, input: f32) -> f32 {
        let delayed = self.ring.read(self.length);
        let out = delayed - input * 0.5;
        self.ring.push(input + out * 0.5);
        clean(out)
    }
}
struct Comb {
    ring: Ring,
    length: f32,
    low: f32,
}
impl Comb {
    fn new(rate: f32) -> Self {
        Self {
            ring: Ring::new((rate * 0.12) as usize + 4),
            length: 1.0,
            low: 0.0,
        }
    }
    fn tick(&mut self, input: f32, feedback: f32, damping: f32) -> f32 {
        let out = self.ring.read(self.length);
        self.low = clean(self.low * damping + out * (1.0 - damping));
        self.ring
            .push(input * (1.0 - feedback) + self.low * feedback);
        out
    }
}
pub(super) struct Room {
    predelay: [Ring; 2],
    tap: Tap,
    combs: [[Comb; 4]; 2],
    input_diffusers: [[Allpass; 2]; 2],
    diffusers: [[Allpass; 4]; 2],
    kind: ReverbKind,
}
impl Room {
    pub(super) fn new(rate: f32, predelay: f32, kind: ReverbKind) -> Self {
        let mut room = Self {
            predelay: std::array::from_fn(|_| Ring::new((rate * 0.2) as usize + 4)),
            tap: Tap::new((predelay * rate / 1000.0).max(1.0), rate),
            combs: std::array::from_fn(|_| std::array::from_fn(|_| Comb::new(rate))),
            input_diffusers: std::array::from_fn(|c| {
                std::array::from_fn(|n| Allpass::new(rate, [7.1, 3.3][n] + c as f32 * 0.2))
            }),
            diffusers: std::array::from_fn(|_| std::array::from_fn(|_| Allpass::new(rate, 1.0))),
            kind,
        };
        room.configure(rate, kind);
        room
    }
    fn configure(&mut self, rate: f32, kind: ReverbKind) {
        self.kind = kind;
        let (lengths, diffusion) = match kind {
            ReverbKind::Room => ([29.7, 37.1, 41.1, 43.7], [5.0, 1.7, 1.0, 1.0]),
            ReverbKind::SmallRoom => ([11.3, 13.7, 17.9, 19.3], [3.1, 0.9, 1.0, 1.0]),
            ReverbKind::Chamber => ([31.1, 39.7, 47.3, 53.9], [7.7, 3.1, 1.0, 1.0]),
            ReverbKind::Plate => ([17.3, 23.9, 31.1, 37.7], [9.1, 5.3, 2.7, 1.1]),
            ReverbKind::Hall => ([67.7, 79.3, 97.1, 113.7], [14.7, 9.3, 5.1, 2.3]),
        };
        for c in 0..2 {
            for (n, comb) in self.combs[c].iter_mut().enumerate() {
                comb.length = (rate * (lengths[n] + c as f32 * 1.3) / 1000.0).round();
            }
            for (n, a) in self.diffusers[c].iter_mut().enumerate() {
                a.length = (rate * (diffusion[n] + c as f32 * 0.3) / 1000.0).round();
            }
        }
    }
    pub(super) fn tick(
        &mut self,
        input: [f32; 2],
        predelay: f32,
        decay: f32,
        damping: f32,
    ) -> [f32; 2] {
        self.tap.advance(predelay.max(1.0));
        let dense = matches!(
            self.kind,
            ReverbKind::Chamber | ReverbKind::Plate | ReverbKind::Hall
        );
        let four = matches!(self.kind, ReverbKind::Plate | ReverbKind::Hall);
        let feedback = match self.kind {
            ReverbKind::Room => 0.45 + decay * 0.5,
            ReverbKind::SmallRoom => 0.32 + decay * 0.5,
            ReverbKind::Chamber => 0.52 + decay * 0.42,
            ReverbKind::Plate => 0.6 + decay * 0.32,
            ReverbKind::Hall => 0.62 + decay * 0.3,
        };
        std::array::from_fn(|c| {
            let mut delayed = self.tap.read(&self.predelay[c]);
            // The original Room and Small room preserve stereo channel isolation.
            self.predelay[c].push(if dense {
                input[c] * 0.85 + input[1 - c] * 0.15
            } else {
                input[c]
            });
            if dense {
                for a in &mut self.input_diffusers[c] {
                    delayed = a.tick(delayed);
                }
            }
            let mut out = 0.0;
            for comb in &mut self.combs[c] {
                out += comb.tick(delayed, feedback, damping) * 0.25;
            }
            for a in &mut self.diffusers[c][..if four { 4 } else { 2 }] {
                out = a.tick(out);
            }
            out * if four { 0.65 } else { 1.0 }
        })
    }
}
pub(super) struct Chorus {
    rings: [Ring; 2],
    phases: [f32; 3],
    seed: f32,
}
impl Chorus {
    pub(super) fn new(rate: f32, seed: f32) -> Self {
        let mut chorus = Self {
            rings: std::array::from_fn(|_| Ring::new((rate * 0.04) as usize + 4)),
            phases: [0.0; 3],
            seed,
        };
        chorus.clear();
        chorus
    }
    fn clear(&mut self) {
        for ring in &mut self.rings {
            ring.clear();
        }
        self.phases = std::array::from_fn(|n| (self.seed + n as f32 * 0.27).fract());
    }
    pub(super) fn tick(&mut self, input: [f32; 2], config: ChorusConfig, rate: f32) -> [f32; 2] {
        let voices = if config.ensemble { 3 } else { 1 };
        let mut out = [0.0; 2];
        for voice in 0..voices {
            oscillator(
                &mut self.phases[voice],
                config.rate_hz * (1.0 + voice as f32 * 0.13) / rate,
            );
            for (c, sample) in out.iter_mut().enumerate() {
                let mut phase = (self.phases[voice] + c as f32 * 0.25).fract();
                let modulation = oscillator(&mut phase, 0.0);
                let ms = config.base_ms + config.depth_ms * modulation;
                *sample += self.rings[c].read(ms * rate / 1000.0) / voices as f32;
            }
        }
        for (c, sample) in input.into_iter().enumerate() {
            self.rings[c].push(sample);
        }
        out
    }
}
