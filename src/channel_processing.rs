//! GP07 fixed-storage mono FOH processing, prepared outside the renderer.
use crate::automix::{
    config::{Compressor, EqBand, EqKind},
    dsp::{Biquad, compression_db, db, gain},
};
use crate::show::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub eq_bypass: bool,
    pub band1_hz: i32,
    pub band1_gain_mdb: i32,
    pub band1_q_milli: i32,
    pub band1_bypass: bool,
    pub band2_hz: i32,
    pub band2_gain_mdb: i32,
    pub band2_q_milli: i32,
    pub band2_bypass: bool,
    pub band3_hz: i32,
    pub band3_gain_mdb: i32,
    pub band3_q_milli: i32,
    pub band3_bypass: bool,
    pub band4_hz: i32,
    pub band4_gain_mdb: i32,
    pub band4_q_milli: i32,
    pub band4_bypass: bool,
    pub compressor_bypass: bool,
    pub threshold_mdb: i32,
    pub ratio_milli: i32,
    pub knee_mdb: i32,
    pub attack_us: i32,
    pub release_ms: i32,
    pub makeup_mdb: i32,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            eq_bypass: true,
            band1_hz: 120,
            band1_gain_mdb: 0,
            band1_q_milli: 1000,
            band1_bypass: false,
            band2_hz: 500,
            band2_gain_mdb: 0,
            band2_q_milli: 1000,
            band2_bypass: false,
            band3_hz: 2000,
            band3_gain_mdb: 0,
            band3_q_milli: 1000,
            band3_bypass: false,
            band4_hz: 8000,
            band4_gain_mdb: 0,
            band4_q_milli: 1000,
            band4_bypass: false,
            compressor_bypass: true,
            threshold_mdb: -18000,
            ratio_milli: 1000,
            knee_mdb: 6000,
            attack_us: 10000,
            release_ms: 100,
            makeup_mdb: 0,
        }
    }
}
impl Config {
    pub fn validate(&self) -> Result<()> {
        for (v, lo, hi, step) in [
            (self.band1_hz, 20, 20000, 1),
            (self.band1_gain_mdb, -12000, 12000, 100),
            (self.band1_q_milli, 100, 10000, 100),
            (self.band2_hz, 20, 20000, 1),
            (self.band2_gain_mdb, -12000, 12000, 100),
            (self.band2_q_milli, 100, 10000, 100),
            (self.band3_hz, 20, 20000, 1),
            (self.band3_gain_mdb, -12000, 12000, 100),
            (self.band3_q_milli, 100, 10000, 100),
            (self.band4_hz, 20, 20000, 1),
            (self.band4_gain_mdb, -12000, 12000, 100),
            (self.band4_q_milli, 100, 10000, 100),
            (self.threshold_mdb, -60000, 0, 100),
            (self.ratio_milli, 1000, 20000, 100),
            (self.knee_mdb, 0, 18000, 100),
            (self.attack_us, 100, 200000, 100),
            (self.release_ms, 10, 2000, 1),
            (self.makeup_mdb, -12000, 12000, 100),
        ] {
            if !(lo..=hi).contains(&v) || v % step != 0 {
                return Err("range".into());
            }
        }
        Ok(())
    }
}
/// Four prepared filters and detector coefficients. No heap ownership.
#[derive(Debug, Clone, Copy)]
pub struct Prepared {
    pub(crate) config: Config,
    filters: [Biquad; 4],
    attack: f64,
    release: f64,
}
impl Prepared {
    pub fn new(config: Config) -> Result<Self> {
        config.validate()?;
        let filters = [
            (
                config.band1_hz,
                config.band1_gain_mdb,
                config.band1_q_milli,
                config.band1_bypass,
            ),
            (
                config.band2_hz,
                config.band2_gain_mdb,
                config.band2_q_milli,
                config.band2_bypass,
            ),
            (
                config.band3_hz,
                config.band3_gain_mdb,
                config.band3_q_milli,
                config.band3_bypass,
            ),
            (
                config.band4_hz,
                config.band4_gain_mdb,
                config.band4_q_milli,
                config.band4_bypass,
            ),
        ]
        .map(|(hz, g, q, bypass)| {
            if bypass || g == 0 {
                Biquad::new([1., 0., 0.], [1., 0., 0.])
            } else {
                Biquad::equalizer(
                    &EqBand {
                        kind: EqKind::Bell,
                        hz: hz as f64,
                        q: q as f64 / 1000.,
                        db: g as f64 / 1000.,
                    },
                    48000,
                )
            }
        });
        Ok(Self {
            config,
            filters,
            attack: (-1. / (config.attack_us as f64 * 0.000001 * 48000.)).exp(),
            release: (-1. / (config.release_ms as f64 * 0.001 * 48000.)).exp(),
        })
    }
}
#[derive(Debug, Clone, Copy)]
struct Branch {
    prepared: Prepared,
    reduction: f64,
}
impl Branch {
    fn new(prepared: Prepared) -> Self {
        Self {
            prepared,
            reduction: 0.,
        }
    }
    fn tick(&mut self, mut x: f64) -> f64 {
        let p = &mut self.prepared;
        if !p.config.eq_bypass {
            for filter in &mut p.filters {
                x = filter.tick(x);
            }
        }
        if !p.config.compressor_bypass {
            let c = p.config;
            let target = compression_db(
                db(x.abs()),
                &Compressor {
                    threshold_db: c.threshold_mdb as f64 / 1000.,
                    ratio: c.ratio_milli as f64 / 1000.,
                    knee_db: c.knee_mdb as f64 / 1000.,
                    attack_ms: 0.,
                    release_ms: 0.,
                    makeup_db: 0.,
                },
            );
            let coefficient = if target < self.reduction {
                p.attack
            } else {
                p.release
            };
            self.reduction = target + coefficient * (self.reduction - target);
            x *= gain(self.reduction + c.makeup_mdb as f64 / 1000.);
        }
        x
    }
}
#[derive(Debug, Clone, Copy)]
pub(crate) struct Strip {
    old: Branch,
    target: Branch,
    begin: u64,
    end: u64,
}
impl Default for Strip {
    fn default() -> Self {
        let branch = Branch::new(Prepared::new(Config::default()).expect("valid default"));
        Self {
            old: branch,
            target: branch,
            begin: 0,
            end: 0,
        }
    }
}
impl Strip {
    pub(crate) fn apply(&mut self, prepared: Prepared, frame: u64) {
        if self.target.prepared.config == prepared.config {
            return;
        }
        self.old = self.target;
        self.target = Branch::new(prepared);
        self.begin = frame;
        self.end = frame + 240;
    }
    pub(crate) fn tick(&mut self, x: f64, frame: u64) -> f64 {
        let target = self.target.tick(x);
        if frame >= self.end {
            return target;
        }
        let old = self.old.tick(x);
        let t = (frame - self.begin) as f64 / 240.;
        // Detect invalid arithmetic in either branch even at an exact endpoint.
        if !old.is_finite() || !target.is_finite() {
            return f64::NAN;
        }
        if t == 0. {
            old
        } else {
            old + (target - old) * t
        }
    }
    pub(crate) fn observation(
        &self,
        frame: u64,
        fault: bool,
    ) -> (Config, Config, u32, Option<i32>) {
        let remaining = self.end.saturating_sub(frame) as u32;
        let config = self.target.prepared.config;
        let current = if remaining == 0 {
            config
        } else {
            self.old.prepared.config
        };
        let reduction = if remaining == 0 && !fault && !config.compressor_bypass {
            Some((-self.target.reduction * 1000.).round().clamp(0., 240000.) as i32)
        } else {
            None
        };
        (current, config, remaining, reduction)
    }
}
