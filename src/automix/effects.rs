//! Optional offline effects rack. Sends are post-fader; returns contain wet signal only.
use super::{
    config::{EqBand, Role, Session},
    dsp::{Biquad, Limiter, gain},
    exciter::Exciter,
    fx_engines::{Chorus, Room},
};
use crate::inventory::Result;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReverbKind {
    Room,
    SmallRoom,
    Chamber,
    Plate,
    Hall,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReverbConfig {
    pub kind: ReverbKind,
    pub predelay_ms: f32,
    pub decay: f32,
    pub damping: f32,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChorusConfig {
    pub rate_hz: f32,
    pub depth_ms: f32,
    pub base_ms: f32,
    pub ensemble: bool,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExciterConfig {
    pub tune_hz: f32,
    pub drive: f32,
    pub tone: f32,
    pub bright: bool,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DelayConfig {
    pub left_ms: f32,
    pub right_ms: f32,
    pub feedback: f32,
    pub damping: f32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(
    tag = "type",
    content = "parameters",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Effect {
    Reverb(ReverbConfig),
    Chorus(ChorusConfig),
    Delay(DelayConfig),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Send {
    pub channel: usize,
    pub db: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bus {
    pub name: String,
    pub effect: Effect,
    pub sends: Vec<Send>,
    pub hpf_hz: f64,
    pub lowpass_hz: f64,
    pub return_db: f64,
    #[serde(default = "default_wet_target")]
    pub target_wet_db: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FxConfig {
    pub buses: Vec<Bus>,
    pub exciter: ExciterConfig,
    pub exciter_amount: f64,
    pub master_eq: Vec<EqBand>,
    pub maximizer_drive_db: f64,
    pub maximizer_threshold_db: f64,
    pub maximizer_release_ms: f64,
    pub tail_seconds: f64,
    pub listening_target_lufs: f64,
}
fn default_wet_target() -> f64 {
    -24.
}
fn in_range(x: f64, a: f64, b: f64) -> bool {
    x.is_finite() && (a..=b).contains(&x)
}
impl FxConfig {
    pub fn validate(&self, s: &Session) -> Result<()> {
        if self.buses.len() > 12
            || !in_range(self.exciter_amount, 0., 1.)
            || !in_range(
                self.exciter.tune_hz as f64,
                100.,
                s.sample_rate as f64 * 0.2,
            )
            || !in_range(self.exciter.drive as f64, 0., 0.6)
            || !in_range(self.exciter.tone as f64, 0., 1.)
            || !in_range(self.maximizer_drive_db, 0., 6.)
            || !in_range(self.maximizer_threshold_db, -30., -2.)
            || !in_range(self.maximizer_release_ms, 20., 1000.)
            || !in_range(self.tail_seconds, 0., 15.)
            || !in_range(self.listening_target_lufs, -30., -14.)
            || self.master_eq.len() > 4
            || self.master_eq.iter().any(|e| {
                !in_range(e.hz, 20., s.sample_rate as f64 * 0.4)
                    || !in_range(e.q, 0.3, 4.)
                    || !in_range(e.db, -3., 3.)
            })
        {
            return Err("invalid FX/master settings".into());
        }
        let mut names = std::collections::BTreeSet::new();
        for b in &self.buses {
            if b.name.is_empty()
                || !names.insert(&b.name)
                || !b
                    .name
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"_-".contains(&c))
                || b.sends.len() > s.channels.len()
                || !in_range(b.hpf_hz, 20., s.sample_rate as f64 * 0.4)
                || !in_range(b.lowpass_hz, b.hpf_hz, s.sample_rate as f64 * 0.45)
                || !in_range(b.return_db, -60., 18.)
                || !in_range(b.target_wet_db, -40., -12.)
                || b.sends
                    .iter()
                    .any(|v| v.channel >= s.channels.len() || !in_range(v.db, -60., 0.))
            {
                return Err("invalid FX bus/send".into());
            }
            let mut channels = std::collections::BTreeSet::new();
            if b.sends.iter().any(|v| !channels.insert(v.channel)) {
                return Err("duplicate FX send".into());
            }
            let valid = match b.effect {
                Effect::Reverb(r) => {
                    in_range(r.predelay_ms as f64, 0., 180.)
                        && in_range(r.decay as f64, 0., 0.95)
                        && in_range(r.damping as f64, 0., 0.95)
                        && (!b.sends.iter().any(|v| {
                            matches!(
                                s.channels[v.channel].role,
                                Role::LeadVocal | Role::VocalRoom
                            )
                        }) || r.predelay_ms >= 30.)
                }
                Effect::Chorus(c) => {
                    in_range(c.rate_hz as f64, 0.05, 3.)
                        && in_range(c.depth_ms as f64, 0., 5.)
                        && in_range(c.base_ms as f64, 6., 30.)
                        && c.base_ms > c.depth_ms
                }
                Effect::Delay(d) => {
                    in_range(d.left_ms as f64, 20., 1000.)
                        && in_range(d.right_ms as f64, 20., 1000.)
                        && in_range(d.feedback as f64, 0., 0.6)
                        && in_range(d.damping as f64, 0., 0.95)
                }
            };
            if !valid {
                return Err(
                    "invalid effect parameters (vocal reverb requires >=30 ms predelay)".into(),
                );
            }
        }
        Ok(())
    }
}
/// Musical starting pass; separate reverb instances keep vocal predelay independent.
pub fn add_pass(s: &mut Session) -> Result<()> {
    let sends = |test: fn(Role) -> bool, db: f64| {
        s.channels
            .iter()
            .enumerate()
            .filter(|(_, c)| test(c.role))
            .map(|(channel, _)| Send { channel, db })
            .collect()
    };
    let reverb = |kind, predelay_ms, decay, damping| {
        Effect::Reverb(ReverbConfig {
            kind,
            predelay_ms,
            decay,
            damping,
        })
    };
    s.effects = Some(FxConfig {
        buses: vec![
            Bus {
                name: "snare_plate".into(),
                effect: reverb(ReverbKind::Plate, 12., 0.68, 0.40),
                sends: sends(|r| matches!(r, Role::Snare), -13.),
                hpf_hz: 180.,
                lowpass_hz: 6500.,
                return_db: 0.,
                target_wet_db: -20.,
            },
            Bus {
                name: "tom_chamber".into(),
                effect: reverb(ReverbKind::Chamber, 18., 0.58, 0.55),
                sends: sends(|r| matches!(r, Role::Tom), -14.),
                hpf_hz: 140.,
                lowpass_hz: 5500.,
                return_db: 0.,
                target_wet_db: -22.,
            },
            Bus {
                name: "vocal_hall".into(),
                effect: reverb(ReverbKind::Hall, 45., 0.52, 0.62),
                sends: sends(|r| matches!(r, Role::LeadVocal), -15.),
                hpf_hz: 200.,
                lowpass_hz: 6000.,
                return_db: 0.,
                target_wet_db: -24.,
            },
            Bus {
                name: "vocal_plate".into(),
                effect: reverb(ReverbKind::Plate, 40., 0.46, 0.55),
                sends: sends(|r| matches!(r, Role::LeadVocal), -23.),
                hpf_hz: 250.,
                lowpass_hz: 5500.,
                return_db: 0.,
                target_wet_db: -30.,
            },
            Bus {
                name: "vocal_chorus".into(),
                effect: Effect::Chorus(ChorusConfig {
                    rate_hz: 0.35,
                    depth_ms: 1.2,
                    base_ms: 17.,
                    ensemble: false,
                }),
                sends: sends(|r| matches!(r, Role::LeadVocal), -23.),
                hpf_hz: 180.,
                lowpass_hz: 7000.,
                return_db: 0.,
                target_wet_db: -23.,
            },
            Bus {
                name: "vocal_delay".into(),
                effect: Effect::Delay(DelayConfig {
                    left_ms: 258.62,
                    right_ms: 344.83,
                    feedback: 0.18,
                    damping: 0.60,
                }),
                sends: sends(|r| matches!(r, Role::LeadVocal), -21.),
                hpf_hz: 250.,
                lowpass_hz: 4500.,
                return_db: 0.,
                target_wet_db: -21.,
            },
        ],
        exciter: ExciterConfig {
            tune_hz: 2500.,
            drive: 0.25,
            tone: 0.35,
            bright: false,
        },
        exciter_amount: 0.25,
        master_eq: vec![],
        maximizer_drive_db: 2.5,
        maximizer_threshold_db: -12.5,
        maximizer_release_ms: 120.,
        tail_seconds: 4.,
        listening_target_lufs: -20.5,
    });
    s.validate()
}
struct Echo {
    rings: [Vec<f64>; 2],
    positions: [usize; 2],
    low: [f64; 2],
    p: DelayConfig,
}
impl Echo {
    fn new(rate: u32, p: DelayConfig) -> Self {
        Self {
            rings: [
                vec![0.; (p.left_ms as f64 * rate as f64 / 1000.).round() as usize],
                vec![0.; (p.right_ms as f64 * rate as f64 / 1000.).round() as usize],
            ],
            positions: [0; 2],
            low: [0.; 2],
            p,
        }
    }
    fn tick(&mut self, x: [f64; 2]) -> [f64; 2] {
        std::array::from_fn(|c| {
            let y = self.rings[c][self.positions[c]];
            self.low[c] = self.low[c] * self.p.damping as f64 + y * (1. - self.p.damping as f64);
            self.rings[c][self.positions[c]] = x[c] + self.low[c] * self.p.feedback as f64;
            self.positions[c] = (self.positions[c] + 1) % self.rings[c].len();
            y
        })
    }
}
enum Engine {
    Reverb(Box<Room>, ReverbConfig),
    Chorus(Chorus, ChorusConfig),
    Delay(Echo),
}
struct Return {
    engine: Engine,
    hp: [Biquad; 2],
    low: [f64; 2],
    low_coefficient: f64,
    level: f64,
}
pub struct Rack {
    returns: Vec<Return>,
    pub inputs: Vec<[f64; 2]>,
    reference_inputs: Vec<[f64; 2]>,
    pub reference_meters: Vec<super::dsp::Meter>,
    sends: Vec<Vec<(usize, f64)>>,
    exciters: Vec<Exciter>,
    p: FxConfig,
    eq: Vec<[Biquad; 2]>,
    maximizer: Limiter,
    rate: u32,
    pub return_meters: Vec<super::dsp::Meter>,
    pub exciter_meter: super::dsp::Meter,
}
impl Rack {
    pub fn new(p: &FxConfig, channels: usize, rate: u32) -> Self {
        let returns = p
            .buses
            .iter()
            .map(|b| Return {
                engine: match b.effect {
                    Effect::Reverb(r) => {
                        Engine::Reverb(Box::new(Room::new(rate as f32, r.predelay_ms, r.kind)), r)
                    }
                    Effect::Chorus(c) => Engine::Chorus(Chorus::new(rate as f32, 0.17), c),
                    Effect::Delay(d) => Engine::Delay(Echo::new(rate, d)),
                },
                hp: [Biquad::highpass(b.hpf_hz, std::f64::consts::FRAC_1_SQRT_2, rate); 2],
                low: [0.; 2],
                low_coefficient: 1. - (-std::f64::consts::TAU * b.lowpass_hz / rate as f64).exp(),
                level: gain(b.return_db),
            })
            .collect();
        let mut sends = vec![Vec::new(); channels];
        for (i, b) in p.buses.iter().enumerate() {
            for send in &b.sends {
                sends[send.channel].push((i, gain(send.db)));
            }
        }
        Self {
            returns,
            inputs: vec![[0.; 2]; p.buses.len()],
            reference_inputs: vec![[0.; 2]; p.buses.len()],
            reference_meters: vec![super::dsp::Meter::default(); p.buses.len()],
            sends,
            exciters: (0..channels)
                .map(|_| Exciter::new(rate as f32, p.exciter))
                .collect(),
            p: p.clone(),
            eq: p
                .master_eq
                .iter()
                .map(|e| [Biquad::bell(e, rate); 2])
                .collect(),
            maximizer: Limiter::new(p.maximizer_threshold_db, p.maximizer_release_ms, rate),
            rate,
            return_meters: vec![super::dsp::Meter::default(); p.buses.len()],
            exciter_meter: Default::default(),
        }
    }
    pub fn excite(&mut self, i: usize, x: [f64; 2]) -> [f64; 2] {
        let wet = self.exciters[i]
            .tick(x.map(|v| v as f32), self.p.exciter)
            .map(|v| v as f64 * self.p.exciter_amount);
        for v in wet {
            self.exciter_meter.add(v);
        }
        std::array::from_fn(|c| x[c] + wet[c])
    }
    pub fn send(&mut self, i: usize, x: [f64; 2]) {
        for &(bus, g) in &self.sends[i] {
            for (c, v) in x.iter().enumerate() {
                self.inputs[bus][c] += v * g;
                self.reference_inputs[bus][c] += v;
            }
        }
    }
    pub fn returns(&mut self) -> [f64; 2] {
        let mut sum = [0.; 2];
        for (i, r) in self.returns.iter_mut().enumerate() {
            let x = self.inputs[i];
            for v in self.reference_inputs[i] {
                self.reference_meters[i].add(v);
            }
            let y = match &mut r.engine {
                Engine::Reverb(e, p) => e
                    .tick(
                        x.map(|v| v as f32),
                        p.predelay_ms * self.rate as f32 / 1000.,
                        p.decay,
                        p.damping,
                    )
                    .map(|v| v as f64),
                Engine::Chorus(e, p) => e
                    .tick(x.map(|v| v as f32), *p, self.rate as f32)
                    .map(|v| v as f64),
                Engine::Delay(e) => e.tick(x),
            };
            for c in 0..2 {
                let hp = r.hp[c].tick(y[c]);
                r.low[c] += r.low_coefficient * (hp - r.low[c]);
                let wet = r.low[c] * r.level;
                self.return_meters[i].add(wet);
                sum[c] += wet;
            }
        }
        self.inputs.fill([0.; 2]);
        self.reference_inputs.fill([0.; 2]);
        sum
    }
    pub fn master(&mut self, mut x: [f64; 2]) -> [f64; 2] {
        for f in &mut self.eq {
            for c in 0..2 {
                x[c] = f[c].tick(x[c]);
            }
        }
        self.maximizer
            .tick(x.map(|v| v * gain(self.p.maximizer_drive_db)))
    }
    pub fn max_reduction(&self) -> f64 {
        self.maximizer.max_reduction
    }
    pub fn affected_frames(&self) -> u64 {
        self.maximizer.affected_frames
    }
}
