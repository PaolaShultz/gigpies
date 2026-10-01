//! Biquad and dynamics equations adapted from SHR PA/DAW; see THIRD_PARTY.md.
use super::config::{Calibration, Channel, Compressor, EqBand};
use serde::Serialize;
use std::f64::consts::{FRAC_1_SQRT_2, PI};
pub fn gain(db: f64) -> f64 {
    10_f64.powf(db / 20.)
}
pub fn db(x: f64) -> f64 {
    20. * x.max(1e-12).log10()
}
#[derive(Clone, Copy)]
pub struct Biquad {
    b: [f64; 3],
    a: [f64; 2],
    z: [f64; 2],
}
impl Biquad {
    pub fn new(b: [f64; 3], a: [f64; 3]) -> Self {
        Self {
            b: b.map(|v| v / a[0]),
            a: [a[1] / a[0], a[2] / a[0]],
            z: [0.; 2],
        }
    }
    pub fn highpass(hz: f64, q: f64, rate: u32) -> Self {
        let w = 2. * PI * hz / rate as f64;
        let c = w.cos();
        let alpha = w.sin() / (2. * q);
        Self::new(
            [(1. + c) / 2., -(1. + c), (1. + c) / 2.],
            [1. + alpha, -2. * c, 1. - alpha],
        )
    }
    pub fn bell(e: &EqBand, rate: u32) -> Self {
        let a = gain(e.db / 2.);
        let w = 2. * PI * e.hz / rate as f64;
        let alpha = w.sin() / (2. * e.q);
        Self::new(
            [1. + alpha * a, -2. * w.cos(), 1. - alpha * a],
            [1. + alpha / a, -2. * w.cos(), 1. - alpha / a],
        )
    }
    pub fn tick(&mut self, x: f64) -> f64 {
        let y = self.b[0] * x + self.z[0];
        self.z = [
            self.b[1] * x - self.a[0] * y + self.z[1],
            self.b[2] * x - self.a[1] * y,
        ];
        y
    }
}
pub fn compression_db(level: f64, p: &Compressor) -> f64 {
    let over = level - p.threshold_db;
    let slope = 1. / p.ratio - 1.;
    if p.knee_db > 0. && over.abs() < p.knee_db / 2. {
        slope * (over + p.knee_db / 2.).powi(2) / (2. * p.knee_db)
    } else if over > 0. {
        slope * over
    } else {
        0.
    }
}
pub struct Strip {
    filters: Vec<[Biquad; 2]>,
    p: Compressor,
    attack: f64,
    release: f64,
    reduction: f64,
    pub max_reduction: f64,
}
impl Strip {
    pub fn new(ch: &Channel, rate: u32) -> Self {
        let mut filters = vec![[Biquad::highpass(ch.hpf_hz, FRAC_1_SQRT_2, rate); 2]];
        filters.extend(ch.eq.iter().map(|e| [Biquad::bell(e, rate); 2]));
        Self {
            filters,
            p: ch.compressor.clone(),
            attack: (-1. / (ch.compressor.attack_ms * 0.001 * rate as f64)).exp(),
            release: (-1. / (ch.compressor.release_ms * 0.001 * rate as f64)).exp(),
            reduction: 0.,
            max_reduction: 0.,
        }
    }
    pub fn tick(&mut self, mut x: [f64; 2]) -> [f64; 2] {
        for f in &mut self.filters {
            for i in 0..2 {
                x[i] = f[i].tick(x[i]);
            }
        }
        let target = compression_db(db(x[0].abs().max(x[1].abs())), &self.p);
        let c = if target < self.reduction {
            self.attack
        } else {
            self.release
        };
        self.reduction = target + c * (self.reduction - target);
        self.max_reduction = self.max_reduction.max(-self.reduction);
        let g = gain(self.reduction + self.p.makeup_db);
        x.map(|v| v * g)
    }
}
/// Decisions inspect completed blocks only, and affect the following block.
/// Recent loud activity decays by 1 dB/s; silence cannot push the trim upward.
#[derive(Clone)]
pub struct Calibrator {
    pub trim_db: f64,
    pub active_seconds: f64,
    pub active: bool,
    average_power: f64,
    recent_db: f64,
    held_peak: f64,
    p: Calibration,
}
impl Calibrator {
    pub fn new(p: &Calibration) -> Self {
        Self {
            trim_db: p.initial_trim_db,
            active_seconds: 0.,
            active: false,
            average_power: 0.,
            recent_db: -120.,
            held_peak: 0.,
            p: p.clone(),
        }
    }
    pub fn observe(&mut self, rms: f64, peak: f64, dt: f64) {
        let level = db(rms);
        self.recent_db = (self.recent_db - dt).max(level);
        self.active = level > self.p.activity_floor_db
            && level > self.recent_db - self.p.relative_activity_db;
        // Peak hold with a slow release guards sparse percussion's crest factor.
        self.held_peak = (self.held_peak * gain(-dt)).max(peak);
        let peak_cap = self.p.peak_headroom_db - db(self.held_peak);
        let mut desired = self.trim_db;
        if self.active {
            self.active_seconds += dt;
            let c = (-dt / 1.5).exp();
            self.average_power = if self.average_power == 0. {
                rms * rms
            } else {
                c * self.average_power + (1. - c) * rms * rms
            };
            if self.active_seconds >= 0.25 {
                desired = self.p.target_rms_db - db(self.average_power.sqrt());
            }
        }
        desired = desired
            .min(peak_cap)
            .clamp(self.p.min_trim_db, self.p.max_trim_db);
        self.trim_db += (desired - self.trim_db).clamp(
            -self.p.down_db_per_second * dt,
            self.p.up_db_per_second * dt,
        );
    }
}
pub struct Limiter {
    envelope: f64,
    release: f64,
    ceiling: f64,
    pub max_reduction: f64,
    pub affected_frames: u64,
}
impl Limiter {
    pub fn new(ceiling_db: f64, release_ms: f64, rate: u32) -> Self {
        Self {
            envelope: 1.,
            release: (-1. / (release_ms * 0.001 * rate as f64)).exp(),
            ceiling: gain(ceiling_db),
            max_reduction: 0.,
            affected_frames: 0,
        }
    }
    pub fn tick(&mut self, x: [f64; 2]) -> [f64; 2] {
        let peak = x[0].abs().max(x[1].abs());
        let required = (self.ceiling / peak.max(1e-20)).min(1.);
        self.envelope = required.min(1. - (1. - self.envelope) * self.release);
        self.max_reduction = self.max_reduction.max(-db(self.envelope));
        if self.envelope < 0.9999 {
            self.affected_frames += 1;
        }
        x.map(|v| v * self.envelope)
    }
}
#[derive(Default, Clone, Serialize)]
pub struct Meter {
    pub peak: f64,
    pub sum_squares: f64,
    pub samples: u64,
    pub clipped_samples: u64,
}
impl Meter {
    pub fn add(&mut self, x: f64) {
        self.peak = self.peak.max(x.abs());
        self.sum_squares += x * x;
        self.samples += 1;
        if x.abs() >= 1. - 1. / 8388608. {
            self.clipped_samples += 1;
        }
    }
    pub fn rms(&self) -> f64 {
        (self.sum_squares / self.samples.max(1) as f64).sqrt()
    }
    pub fn report(&self) -> serde_json::Value {
        serde_json::json!({"peak_dbfs":db(self.peak),"rms_dbfs":db(self.rms()),"samples":self.samples,"clipped_or_full_scale_samples":self.clipped_samples})
    }
}
/// Stereo K weighting, 400 ms windows / 100 ms hops, absolute and relative gates.
/// Filter coefficients use the bilinear form of the BS.1770 reference filters.
pub struct Loudness {
    filters: [[Biquad; 2]; 2],
    hop: usize,
    count: usize,
    power: f64,
    quarters: std::collections::VecDeque<f64>,
    blocks: Vec<f64>,
}
impl Loudness {
    pub fn new(rate: u32) -> Self {
        let k = (PI * 1681.974450955533 / rate as f64).tan();
        let q = 0.7071752369554196;
        let vh = 10_f64.powf(3.999843853973347 / 20.);
        let vb = vh.powf(0.4996667741545416);
        let shelf = Biquad::new(
            [
                vh + vb * k / q + k * k,
                2. * (k * k - vh),
                vh - vb * k / q + k * k,
            ],
            [1. + k / q + k * k, 2. * (k * k - 1.), 1. - k / q + k * k],
        );
        let k = (PI * 38.13547087602444 / rate as f64).tan();
        let q = 0.5003270373238773;
        let a0 = 1. + k / q + k * k;
        let hp = Biquad::new(
            [1., -2., 1.],
            [1., 2. * (k * k - 1.) / a0, (1. - k / q + k * k) / a0],
        );
        Self {
            filters: [[shelf, hp]; 2],
            hop: rate as usize / 10,
            count: 0,
            power: 0.,
            quarters: Default::default(),
            blocks: Vec::new(),
        }
    }
    pub fn add(&mut self, x: [f64; 2]) {
        for (f, v) in self.filters.iter_mut().zip(x) {
            let v = f[0].tick(v);
            self.power += f[1].tick(v).powi(2);
        }
        self.count += 1;
        if self.count == self.hop {
            self.quarters.push_back(self.power / self.hop as f64);
            self.power = 0.;
            self.count = 0;
            if self.quarters.len() == 4 {
                self.blocks.push(self.quarters.iter().sum::<f64>() / 4.);
                self.quarters.pop_front();
            }
        }
    }
    pub fn integrated(&self) -> Option<f64> {
        let abs: Vec<_> = self
            .blocks
            .iter()
            .copied()
            .filter(|p| -0.691 + 10. * p.max(1e-30).log10() > -70.)
            .collect();
        if abs.is_empty() {
            return None;
        }
        let gate = abs.iter().sum::<f64>() / abs.len() as f64 / 10.;
        let rel: Vec<_> = abs.into_iter().filter(|p| *p > gate).collect();
        Some(-0.691 + 10. * (rel.iter().sum::<f64>() / rel.len() as f64).log10())
    }
}
