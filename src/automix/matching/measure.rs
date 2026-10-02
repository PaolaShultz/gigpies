use super::*;
use crate::automix::{
    analysis::fft,
    dsp::{Meter, Strip},
    effects::Rack,
    render::{Source, route},
};
use serde::{Deserialize, Serialize};
use std::f64::consts::{FRAC_1_SQRT_2, TAU};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Window {
    pub start: f64,
    pub end: f64,
    pub envelope_db: Vec<f64>,
    pub band_supported: Vec<bool>,
    pub cancellation_db: Vec<f64>,
    pub flatness: f64,
    pub largest_bin_fraction: f64,
    pub raw_rms_dbfs: f64,
    pub group_rms_dbfs: f64,
    pub group_peak_dbfs: f64,
    pub mix_rms_dbfs: f64,
    pub mix_peak_dbfs: f64,
    pub group_body_presence_db: f64,
    pub mix_low_power_dbfs: f64,
    pub stereo_correlation: f64,
    pub return_rms_dbfs: Vec<f64>,
    pub compressor_max_db: Vec<f64>,
    pub master_reduction_db: f64,
    pub pcm_contact_fraction: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Moment {
    pub start: f64,
    pub end: f64,
    pub raw_power: f64,
    pub group_power: f64,
    pub mix_power: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Measurement {
    pub sample_rate: u32,
    pub windows: Vec<Window>,
    pub moments: Vec<Moment>,
}

pub fn inside(start: f64, end: f64, spans: &[[f64; 2]]) -> bool {
    spans
        .iter()
        .any(|s| start + 1e-9 >= s[0] && end <= s[1] + 1e-9)
}
fn transform(x: &[[f64; 2]], side: usize) -> Vec<[f64; 2]> {
    let mut y = x
        .iter()
        .enumerate()
        .map(|(i, v)| {
            [
                v[side] * (0.5 - 0.5 * (TAU * i as f64 / (FFT_SIZE - 1) as f64).cos()),
                0.,
            ]
        })
        .collect::<Vec<_>>();
    fft(&mut y);
    y.truncate(FFT_SIZE / 2 + 1);
    y
}
fn power(x: &[[f64; 2]]) -> Vec<f64> {
    x.iter().map(|v| v[0] * v[0] + v[1] * v[1]).collect()
}
fn stereo_power(x: &[[f64; 2]]) -> Vec<f64> {
    let mut p = power(&transform(x, 0));
    for (v, r) in p.iter_mut().zip(power(&transform(x, 1))) {
        *v += r;
    }
    p
}
struct Smoother {
    kernels: Vec<Vec<(usize, f64)>>,
    rate: u32,
}
impl Smoother {
    fn new(rate: u32) -> Self {
        let kernels = frequencies()
            .iter()
            .map(|hz| {
                let mut k = (1..FFT_SIZE / 2)
                    .filter_map(|b| {
                        let d = ((b as f64 * rate as f64 / FFT_SIZE as f64) / hz)
                            .log2()
                            .abs();
                        (d < 0.5).then_some((b, 1. - 2. * d))
                    })
                    .collect::<Vec<_>>();
                let norm = k.iter().map(|x| x.1).sum::<f64>().max(1e-20);
                for x in &mut k {
                    x.1 /= norm;
                }
                k
            })
            .collect();
        Self { kernels, rate }
    }
    fn smooth(&self, p: &[f64]) -> Vec<f64> {
        self.kernels
            .iter()
            .map(|k| k.iter().map(|&(i, w)| p[i] * w).sum())
            .collect()
    }
    fn features(&self, p: &[f64]) -> (f64, f64) {
        let relevant = p
            .iter()
            .enumerate()
            .filter(|(i, _)| {
                let hz = *i as f64 * self.rate as f64 / FFT_SIZE as f64;
                (125.0..self.rate as f64 * 0.4).contains(&hz)
            })
            .map(|(_, v)| *v)
            .collect::<Vec<_>>();
        let sum = relevant.iter().sum::<f64>();
        let peak = relevant.iter().copied().fold(0., f64::max) / sum.max(1e-30);
        // Local flatness resists mistaking smoothly coloured broadband noise for tone.
        let max = self.smooth(p).into_iter().fold(0., f64::max);
        let mut flat = Vec::new();
        for (hz, k) in frequencies().iter().zip(&self.kernels) {
            if *hz < 125. || *hz > self.rate as f64 * 0.35 || k.len() < 8 {
                continue;
            }
            let arithmetic = k.iter().map(|&(i, w)| p[i] * w).sum::<f64>();
            if arithmetic < max * 0.001 {
                continue;
            }
            let geometric = k
                .iter()
                .map(|&(i, w)| w * p[i].max(1e-30).ln())
                .sum::<f64>()
                .exp();
            flat.push(geometric / arithmetic.max(1e-30));
        }
        (median(&flat), peak)
    }
    fn band_power(&self, p: &[f64], lo: f64, hi: f64) -> f64 {
        p.iter()
            .enumerate()
            .filter(|(i, _)| {
                let hz = *i as f64 * self.rate as f64 / FFT_SIZE as f64;
                hz >= lo && hz < hi
            })
            .map(|(_, v)| *v)
            .sum::<f64>()
            / (FFT_SIZE * FFT_SIZE) as f64
            / 0.375
    }
}
/// Continuous production strips, excitation, sends, returns, master and native BWF timeline.
/// No reset at passage boundaries. Only observations inside declared passages are retained.
pub fn measure(
    s: &Session,
    root: &Path,
    group: &InputGroup,
    spans: &[[f64; 2]],
) -> Result<Measurement> {
    validate_baseline(s)?;
    group.validate(s)?;
    validate_spans(spans)?;
    let mut sources = s
        .channels
        .iter()
        .map(|c| Source::open(&root.join(&c.file), s.sample_rate))
        .collect::<Result<Vec<_>>>()?;
    let refs = sources
        .iter()
        .filter_map(|s| s.reference)
        .collect::<Vec<_>>();
    if !refs.is_empty() && refs.len() != sources.len() {
        return Err("mixed BWF references".into());
    }
    let origin = refs.iter().min().copied().unwrap_or(0);
    for src in &mut sources {
        src.offset = src.reference.unwrap_or(0) - origin;
    }
    let total = sources
        .iter()
        .map(|s| s.offset.checked_add(s.frames).ok_or("timeline overflow"))
        .collect::<std::result::Result<Vec<_>, _>>()?
        .into_iter()
        .max()
        .unwrap_or(0);
    let end = (spans.iter().map(|s| s[1]).fold(0., f64::max) * s.sample_rate as f64).round() as u64;
    if end > total || end == 0 {
        return Err("matching passages exceed source timeline".into());
    }
    let mut which = vec![None; s.channels.len()];
    for (j, i) in group.inputs.iter().enumerate() {
        which[i.channel] = Some(j);
    }
    let mut strips = s
        .channels
        .iter()
        .map(|c| Strip::new(c, s.sample_rate))
        .collect::<Vec<_>>();
    let mut rack = s
        .effects
        .as_ref()
        .map(|fx| Rack::new(fx, s.channels.len(), s.sample_rate));
    let mut hp = [Biquad::highpass(s.master_hpf_hz, FRAC_1_SQRT_2, s.sample_rate); 2];
    let faders = s
        .channels
        .iter()
        .map(|c| gain(c.fader_db))
        .collect::<Vec<_>>();
    let mut taps = vec![vec![[0.; 2]; FFT_SIZE]; group.inputs.len()];
    let mut cooked = vec![[0.; 2]; FFT_SIZE];
    let mut mixed = cooked.clone();
    let mut raw = Meter::default();
    let mut gm = Meter::default();
    let mut mm = Meter::default();
    let nb = s.effects.as_ref().map_or(0, |fx| fx.buses.len());
    let mut returns = vec![Meter::default(); nb];
    let mut gr = vec![0f64; s.channels.len()];
    let mut master_gr = 0f64;
    let mut contacts = 0u64;
    let mut contact_samples = 0u64;
    let mut windows = Vec::new();
    let mut moments = Vec::new();
    let mut moment = Moment {
        start: 0.,
        end: 0.,
        raw_power: 0.,
        group_power: 0.,
        mix_power: 0.,
    };
    let moment_size = (s.sample_rate / 50) as u64;
    let smooth = Smoother::new(s.sample_rate);
    for t in 0..end {
        let slot = (t % FFT_SIZE as u64) as usize;
        let mut dry = [0.; 2];
        let mut gout = [0.; 2];
        let mut raw_group = [0.; 2];
        for (i, src) in sources.iter_mut().enumerate() {
            let old_contact = src.full_scale_samples();
            let x = src.next(t)?;
            let (eq, y) = strips[i].tick_with_eq_tap(x);
            gr[i] = gr[i].max(strips[i].reduction_db());
            let y = if let Some(fx) = &mut rack {
                fx.excite(i, y)
            } else {
                y
            };
            let c = &s.channels[i];
            let y = route(y, src.channels, c.pan, faders[i]);
            if let Some(j) = which[i] {
                taps[j][slot] = route(eq, src.channels, c.pan, faders[i]);
                let r = route(x, src.channels, c.pan, faders[i]);
                for side in 0..2 {
                    gout[side] += y[side];
                    raw_group[side] += r[side];
                }
                if !src.is_float() {
                    contacts += src.full_scale_samples() - old_contact;
                    contact_samples += src.channels as u64;
                }
            }
            if let Some(fx) = &mut rack {
                fx.send(i, y);
            }
            for side in 0..2 {
                dry[side] += y[side] * gain(s.master_db);
            }
        }
        let wet = rack.as_mut().map_or([0.; 2], Rack::returns);
        if let Some(fx) = &rack {
            for (m, x) in returns.iter_mut().zip(&fx.return_outputs) {
                for &v in x {
                    m.add(v);
                }
            }
        }
        let mut mix =
            std::array::from_fn(|side| hp[side].tick(dry[side] + wet[side] * gain(s.master_db)));
        if let Some(fx) = &mut rack {
            mix = fx.master(mix);
            master_gr = master_gr.max(fx.reduction_db());
        }
        if mix.iter().chain(gout.iter()).any(|v| !v.is_finite()) {
            return Err("nonfinite matching DSP output".into());
        }
        cooked[slot] = gout;
        mixed[slot] = mix;
        for side in 0..2 {
            raw.add(raw_group[side]);
            gm.add(gout[side]);
            mm.add(mix[side]);
            moment.raw_power += raw_group[side].powi(2);
            moment.group_power += gout[side].powi(2);
            moment.mix_power += mix[side].powi(2);
        }
        if (t + 1) % moment_size == 0 {
            moment.end = (t + 1) as f64 / s.sample_rate as f64;
            for p in [
                &mut moment.raw_power,
                &mut moment.group_power,
                &mut moment.mix_power,
            ] {
                *p /= 2. * moment_size as f64;
            }
            let next = Moment {
                start: moment.end,
                end: 0.,
                raw_power: 0.,
                group_power: 0.,
                mix_power: 0.,
            };
            if inside(moment.start, moment.end, spans) {
                moments.push(moment);
            }
            moment = next;
            if moments.len() > 30000 {
                return Err("matching evidence exceeds ten-minute observation budget".into());
            }
        }
        if slot + 1 == FFT_SIZE {
            let start = (t + 1 - FFT_SIZE as u64) as f64 / s.sample_rate as f64;
            let stop = (t + 1) as f64 / s.sample_rate as f64;
            if inside(start, stop, spans) {
                let mut coherent = vec![0.; FFT_SIZE / 2 + 1];
                let mut independent = coherent.clone();
                for side in 0..2 {
                    let mut sum = vec![[0.; 2]; FFT_SIZE / 2 + 1];
                    for tap in &taps {
                        let spectrum = transform(tap, side);
                        for ((dst, ind), v) in sum.iter_mut().zip(&mut independent).zip(spectrum) {
                            dst[0] += v[0];
                            dst[1] += v[1];
                            *ind += v[0] * v[0] + v[1] * v[1];
                        }
                    }
                    for (p, v) in coherent.iter_mut().zip(power(&sum)) {
                        *p += v;
                    }
                }
                let (flatness, largest_bin_fraction) = smooth.features(&coherent);
                let sp = smooth.smooth(&coherent);
                let ind = smooth.smooth(&independent);
                let max = sp.iter().copied().fold(0., f64::max).max(1e-30);
                let cancel = sp
                    .iter()
                    .zip(ind)
                    .map(|(p, i)| 10. * (p / i.max(1e-30)).max(1e-24).log10())
                    .collect();
                let supported = sp
                    .iter()
                    .zip(frequencies())
                    .map(|(p, hz)| {
                        *p > max * 0.001
                            && hz >= s.sample_rate as f64 / FFT_SIZE as f64 * 4.
                            && hz < s.sample_rate as f64 * 0.4
                    })
                    .collect();
                let gp = stereo_power(&cooked);
                let mp = stereo_power(&mixed);
                let (ll, rr, lr) = mixed.iter().fold((0., 0., 0.), |(l, r, c), v| {
                    (l + v[0] * v[0], r + v[1] * v[1], c + v[0] * v[1])
                });
                windows.push(Window {
                    start,
                    end: stop,
                    envelope_db: sp.iter().map(|p| 10. * p.max(1e-24).log10()).collect(),
                    band_supported: supported,
                    cancellation_db: cancel,
                    flatness,
                    largest_bin_fraction,
                    raw_rms_dbfs: db(raw.rms()),
                    group_rms_dbfs: db(gm.rms()),
                    group_peak_dbfs: db(gm.peak),
                    mix_rms_dbfs: db(mm.rms()),
                    mix_peak_dbfs: db(mm.peak),
                    group_body_presence_db: 10.
                        * (smooth.band_power(&gp, 100., 400.)
                            / smooth.band_power(&gp, 800., 3200.).max(1e-24))
                        .max(1e-24)
                        .log10(),
                    mix_low_power_dbfs: 10. * smooth.band_power(&mp, 30., 160.).max(1e-24).log10(),
                    stereo_correlation: lr / (ll * rr).sqrt().max(1e-30),
                    return_rms_dbfs: returns.iter().map(|m| db(m.rms())).collect(),
                    compressor_max_db: gr.clone(),
                    master_reduction_db: master_gr,
                    pcm_contact_fraction: contacts as f64 / contact_samples.max(1) as f64,
                });
            }
            raw = Meter::default();
            gm = Meter::default();
            mm = Meter::default();
            returns.fill(Meter::default());
            gr.fill(0.);
            master_gr = 0.;
            contacts = 0;
            contact_samples = 0;
        }
    }
    Ok(Measurement {
        sample_rate: s.sample_rate,
        windows,
        moments,
    })
}
