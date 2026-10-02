//! Deterministic offline review. Metrics identify candidates, not musical defects.
//! Decisions are conservative, logged, and never modify earlier causal soundcheck samples.
use super::{
    config::{EqBand, Session},
    dsp::db,
    write_json,
};
use crate::inventory::Result;
use serde::{Deserialize, Serialize};
use std::{fs::File, io::BufReader, path::Path};
const SIZE: usize = 8192;
const BANDS: usize = 25;
fn centers() -> [f64; BANDS] {
    std::array::from_fn(|i| 1000. * 2_f64.powf((i as f64 - 15.) / 3.))
}
/// In-place radix-2 forward FFT, for analysis only.
pub(super) fn fft(x: &mut [[f64; 2]]) {
    let n = x.len();
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if i < j {
            x.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= n {
        let angle = -std::f64::consts::TAU / len as f64;
        let root = [angle.cos(), angle.sin()];
        for start in (0..n).step_by(len) {
            let mut w = [1., 0.];
            for k in 0..len / 2 {
                let u = x[start + k];
                let b = x[start + k + len / 2];
                let v = [b[0] * w[0] - b[1] * w[1], b[0] * w[1] + b[1] * w[0]];
                x[start + k] = [u[0] + v[0], u[1] + v[1]];
                x[start + k + len / 2] = [u[0] - v[0], u[1] - v[1]];
                w = [
                    w[0] * root[0] - w[1] * root[1],
                    w[0] * root[1] + w[1] * root[0],
                ];
            }
        }
        len *= 2;
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub activity_floor_dbfs: f64,
    pub minimum_active_seconds: f64,
    pub prominence_db: f64,
    pub occupancy_fraction: f64,
    pub minimum_band_power_fraction: f64,
    pub max_eq_cut_db: f64,
    pub max_eq_bands: usize,
    pub max_master_reduction_db: f64,
    pub max_return_adjustment_db: f64,
}
impl Default for Policy {
    fn default() -> Self {
        Self {
            activity_floor_dbfs: -60.,
            minimum_active_seconds: 8.,
            prominence_db: 4.,
            occupancy_fraction: 0.40,
            minimum_band_power_fraction: 0.04,
            max_eq_cut_db: 1.5,
            max_eq_bands: 2,
            max_master_reduction_db: 3.,
            max_return_adjustment_db: 12.,
        }
    }
}
impl Policy {
    pub fn validate(&self) -> Result<()> {
        let valid = |x: f64, a: f64, b: f64| x.is_finite() && (a..=b).contains(&x);
        if !valid(self.activity_floor_dbfs, -90., -30.)
            || !valid(self.minimum_active_seconds, 1., 60.)
            || !valid(self.prominence_db, 3., 12.)
            || !valid(self.occupancy_fraction, 0.25, 1.)
            || !valid(self.minimum_band_power_fraction, 0.01, 0.3)
            || !valid(self.max_eq_cut_db, 0., 2.)
            || self.max_eq_bands > 2
            || !valid(self.max_master_reduction_db, 1., 6.)
            || !valid(self.max_return_adjustment_db, 0., 12.)
        {
            return Err("invalid automated review policy".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct Band {
    pub hz: f64,
    pub power_dbfs: f64,
    pub power_fraction: f64,
    pub local_prominence_db: f64,
    pub hot_window_fraction: f64,
    pub hot_time_span_fraction: f64,
    pub eligible_range: bool,
}
#[derive(Clone, Debug, Serialize)]
pub struct Spectrum {
    pub sample_rate: u32,
    pub frames: u64,
    pub active_windows: u64,
    pub active_seconds: f64,
    pub peak_dbfs: f64,
    pub rms_dbfs: f64,
    pub bands: Vec<Band>,
}
pub fn analyze(path: &Path, policy: &Policy) -> Result<Spectrum> {
    policy.validate()?;
    let mut reader = hound::WavReader::open(path)?;
    let spec = reader.spec();
    if spec.channels != 2
        || spec.sample_format != hound::SampleFormat::Int
        || ![16, 24, 32].contains(&spec.bits_per_sample)
    {
        return Err("spectral review requires stereo PCM WAV".into());
    }
    let frames = reader.duration() as u64;
    let rate = spec.sample_rate;
    if rate < 8000 {
        return Err("unsupported analysis sample rate".into());
    }
    let scale = 2_f64.powi(spec.bits_per_sample as i32 - 1);
    let mut samples = reader.samples::<i32>();
    let window: Vec<_> = (0..SIZE)
        .map(|i| 0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / SIZE as f64).cos())
        .collect();
    let window_power = window.iter().map(|w| w * w).sum::<f64>();
    let mut block = vec![[0.; 2]; SIZE];
    let mut transform = vec![[0.; 2]; SIZE];
    let mut cursor = 0usize;
    let mut active = 0u64;
    let mut sums = [0.; BANDS];
    let mut hot = [0u64; BANDS];
    let mut spans = [0u16; BANDS];
    let mut peak = 0_f64;
    let mut energy = 0.;
    let hz = centers();
    let mut frames_read = 0u64;
    loop {
        while cursor < SIZE {
            match samples.next() {
                Some(left) => {
                    let x = [
                        left? as f64 / scale,
                        samples.next().ok_or("incomplete stereo frame")?? as f64 / scale,
                    ];
                    for v in x {
                        peak = peak.max(v.abs());
                        energy += v * v;
                    }
                    block[cursor] = x;
                    cursor += 1;
                    frames_read += 1;
                }
                None => break,
            }
        }
        if cursor < SIZE {
            break;
        }
        let block_power = block.iter().flatten().map(|v| v * v).sum::<f64>() / (SIZE * 2) as f64;
        if db(block_power.sqrt()) > policy.activity_floor_dbfs {
            let mut powers = [0.; BANDS];
            for c in [0usize, 1] {
                for i in 0..SIZE {
                    transform[i] = [block[i][c] * window[i], 0.];
                }
                fft(&mut transform);
                for (k, v) in transform.iter().enumerate().take(SIZE / 2).skip(1) {
                    let frequency = k as f64 * rate as f64 / SIZE as f64;
                    let index = (3. * (frequency / 1000.).log2() + 15.).round() as i32;
                    if (0..BANDS as i32).contains(&index) {
                        powers[index as usize] +=
                            (v[0] * v[0] + v[1] * v[1]) / (SIZE as f64 * window_power);
                    }
                }
            }
            let total = powers.iter().sum::<f64>();
            let span = ((frames_read as f64 / frames.max(1) as f64 * 10.).floor() as usize).min(9);
            for i in 0..BANDS {
                sums[i] += powers[i];
                if i > 0 && i < BANDS - 1 {
                    let prominence = 10.
                        * (powers[i].max(1e-24)
                            / (powers[i - 1].max(1e-24) * powers[i + 1].max(1e-24)).sqrt())
                        .log10();
                    if prominence >= policy.prominence_db
                        && powers[i] >= total * policy.minimum_band_power_fraction
                    {
                        hot[i] += 1;
                        spans[i] |= 1 << span;
                    }
                }
            }
            active += 1;
        }
        block.copy_within(SIZE / 2..SIZE, 0);
        cursor = SIZE / 2;
    }
    let total = sums.iter().sum::<f64>();
    let bands = (0..BANDS)
        .map(|i| {
            let prominence = if i > 0 && i < BANDS - 1 {
                10. * (sums[i].max(1e-24)
                    / (sums[i - 1].max(1e-24) * sums[i + 1].max(1e-24)).sqrt())
                .log10()
            } else {
                0.
            };
            Band {
                hz: hz[i],
                power_dbfs: 10. * (sums[i] / active.max(1) as f64).max(1e-24).log10(),
                power_fraction: sums[i] / total.max(1e-24),
                local_prominence_db: prominence,
                hot_window_fraction: hot[i] as f64 / active.max(1) as f64,
                hot_time_span_fraction: spans[i].count_ones() as f64 / 10.,
                eligible_range: (140. ..=500.).contains(&hz[i])
                    || (2000. ..=6300.).contains(&hz[i]),
            }
        })
        .collect();
    Ok(Spectrum {
        sample_rate: rate,
        frames,
        active_windows: active,
        active_seconds: active as f64 * (SIZE / 2) as f64 / rate as f64,
        peak_dbfs: db(peak),
        rms_dbfs: db((energy / (frames.max(1) * 2) as f64).sqrt()),
        bands,
    })
}
#[derive(Debug, Serialize)]
pub struct Decision {
    pub parameter: String,
    pub before: f64,
    pub after: f64,
    pub reason: String,
}
/// One bounded correction pass. Existing EQ is never compounded by repeated reviews.
pub fn decisions(
    s: &mut Session,
    spectrum: &Spectrum,
    metrics: &serde_json::Value,
    p: &Policy,
) -> Result<Vec<Decision>> {
    p.validate()?;
    s.validate()?;
    let fx = s
        .effects
        .as_mut()
        .ok_or("review requires an FX configuration")?;
    let mut log = Vec::new();
    if spectrum.active_seconds >= p.minimum_active_seconds {
        let mut candidates: Vec<_> = spectrum
            .bands
            .iter()
            .filter(|b| {
                b.eligible_range
                    && b.local_prominence_db >= p.prominence_db
                    && b.hot_window_fraction >= p.occupancy_fraction
                    && b.hot_time_span_fraction >= 0.4
                    && b.power_fraction >= p.minimum_band_power_fraction
            })
            .collect();
        candidates.sort_by(|a, b| b.local_prominence_db.total_cmp(&a.local_prominence_db));
        for b in candidates.into_iter().take(p.max_eq_bands) {
            if fx.master_eq.len() >= 4
                || fx
                    .master_eq
                    .iter()
                    .any(|e| (e.hz / b.hz).log2().abs() < 0.5)
            {
                continue;
            }
            let cut = ((b.local_prominence_db - p.prominence_db) * 0.5 + 0.5).min(p.max_eq_cut_db);
            if cut == 0. {
                continue;
            }
            fx.master_eq.push(EqBand {
                kind: Default::default(),
                hz: b.hz,
                q: 1.4,
                db: -cut,
            });
            log.push(Decision{parameter:format!("effects.master_eq@{:.2}Hz",b.hz),before:0.,after:-cut,reason:format!("Persistent band prominence {:.2} dB; {:.1}% of active windows; {:.0}% of timeline spans; {:.1}% of band power. Conservative candidate cut, not a diagnosis of musical quality.",b.local_prominence_db,100.*b.hot_window_fraction,100.*b.hot_time_span_fraction,100.*b.power_fraction)});
        }
    }
    if let Some(gr) = metrics["effects"]["maximizer_max_reduction_db"].as_f64()
        && gr > p.max_master_reduction_db
    {
        let before = fx.maximizer_drive_db;
        fx.maximizer_drive_db = (before - (gr - p.max_master_reduction_db)).max(0.);
        log.push(Decision {
            parameter: "effects.maximizer_drive_db".into(),
            before,
            after: fx.maximizer_drive_db,
            reason: format!(
                "Measured peak reduction {gr:.2} dB exceeds {} dB budget; back off drive.",
                p.max_master_reduction_db
            ),
        });
    }
    // Reverb algorithms have different wet transfer gains. Calibrate each return from
    // source/return energy, independent of spectral decisions, with a strict one-pass cap.
    for (i, b) in fx.buses.iter_mut().enumerate() {
        if let (Some(dry), Some(wet)) = (
            metrics["effects"]["send_reference_meters"][i]["rms_dbfs"].as_f64(),
            metrics["effects"]["return_meters_before_master"][i]["rms_dbfs"].as_f64(),
        ) && dry > -70.
            && wet > -110.
        {
            let delta = (b.target_wet_db - (wet - dry))
                .clamp(-p.max_return_adjustment_db, p.max_return_adjustment_db);
            let before = b.return_db;
            b.return_db = (before + delta).clamp(-60., 18.);
            if (before - b.return_db).abs() > 0.1 {
                log.push(Decision{parameter:format!("effects.buses.{i}.return_db"),before,after:b.return_db,reason:format!("{} measured wet/source {:.2} dB; target {:.2} dB; bounded return calibration.",b.name,wet-dry,b.target_wet_db)});
            }
        }
    }
    s.validate()?;
    Ok(log)
}
/// CLI-owned complete feedback pass: preliminary render -> code review -> new render -> measurements.
pub fn finish(mut s: Session, root: &Path, out: &Path, policy: Policy) -> Result<()> {
    s.validate()?;
    policy.validate()?;
    if !s.prepared || s.effects.is_none() {
        return Err("finish requires prepared FX settings".into());
    }
    std::fs::create_dir(out)?;
    write_json(&out.join("review-policy.json"), &policy)?;
    super::run(s.clone(), root, &out.join("before-review"), None)?;
    let spectrum = analyze(&out.join("before-review/processed.wav"), &policy)?;
    let metrics: serde_json::Value = serde_json::from_reader(BufReader::new(File::open(
        out.join("before-review/measurements.json"),
    )?))?;
    let changes = decisions(&mut s, &spectrum, &metrics, &policy)?;
    write_json(&out.join("spectral-before.json"), &spectrum)?;
    write_json(&out.join("decisions.json"), &changes)?;
    write_json(&out.join("reviewed-settings.json"), &s)?;
    super::run(s, root, &out.join("final"), None)?;
    let after = analyze(&out.join("final/processed.wav"), &policy)?;
    write_json(&out.join("spectral-after.json"), &after)?;
    let final_metrics: serde_json::Value = serde_json::from_reader(BufReader::new(File::open(
        out.join("final/measurements.json"),
    )?))?;
    let final_reduction = final_metrics["effects"]["maximizer_max_reduction_db"]
        .as_f64()
        .ok_or("missing maximizer meter")?;
    write_json(
        &out.join("review-result.json"),
        &serde_json::json!({
            "algorithm": "bounded_review_v1", "actions": changes.len(),
            "post_review_maximizer_reduction_db": final_reduction,
            "within_reduction_budget": final_reduction <= policy.max_master_reduction_db + 0.01,
            "remaining_spectral_candidates": after.bands.iter().filter(|b| b.eligible_range && b.local_prominence_db >= policy.prominence_db && b.hot_window_fraction >= policy.occupancy_fraction && b.hot_time_span_fraction >= 0.4 && b.power_fraction >= policy.minimum_band_power_fraction).collect::<Vec<_>>(),
            "note": "One bounded review; residual prominence is allowed. No iterative spectrum flattening or listening-quality claim."
        }),
    )?;
    super::compare(
        &out.join("before-review/processed.wav"),
        &out.join("final/processed.wav"),
        &out.join("review-comparison"),
    )?;
    println!(
        "Applied {} bounded code decisions; logs in {}",
        changes.len(),
        out.display()
    );
    Ok(())
}
