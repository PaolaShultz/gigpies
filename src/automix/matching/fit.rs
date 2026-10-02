use super::*;
use serde::{Deserialize, Serialize};

pub fn median(x: &[f64]) -> f64 {
    percentile(x, 0.5)
}
fn percentile(x: &[f64], q: f64) -> f64 {
    let mut v = x.to_vec();
    v.sort_by(f64::total_cmp);
    if v.is_empty() {
        0.
    } else {
        v[((v.len() - 1) as f64 * q).round() as usize]
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Envelope {
    pub values_db: Vec<f64>,
    pub uncertainty_db: Vec<f64>,
    pub supported: Vec<bool>,
    pub boost_supported: Vec<bool>,
    pub active_seconds: f64,
    pub eligible_windows: usize,
    pub phrase_variation_db: f64,
    pub activity_threshold_dbfs: f64,
}
/// The training activity threshold is reused unchanged for held-out/candidate observations.
pub fn envelope(m: &Measurement, spans: &[[f64; 2]], threshold: Option<f64>) -> Envelope {
    summarize(m, m, spans, threshold)
}
pub fn envelope_locked(
    m: &Measurement,
    anchors: &Measurement,
    spans: &[[f64; 2]],
    threshold: f64,
) -> Envelope {
    summarize(m, anchors, spans, Some(threshold))
}
fn summarize(
    m: &Measurement,
    anchors: &Measurement,
    spans: &[[f64; 2]],
    threshold: Option<f64>,
) -> Envelope {
    let frames = m
        .windows
        .iter()
        .zip(&anchors.windows)
        .filter(|(_, f)| inside(f.start, f.end, spans))
        .collect::<Vec<_>>();
    let level = frames
        .iter()
        .map(|(_, f)| f.raw_rms_dbfs)
        .collect::<Vec<_>>();
    let threshold = threshold.unwrap_or((percentile(&level, 0.95) - 24.).max(-65.));
    let active = frames
        .iter()
        .filter(|(_, f)| {
            f.raw_rms_dbfs > threshold && f.flatness < 0.35 && f.largest_bin_fraction < 0.30
        })
        .copied()
        .collect::<Vec<_>>();
    let mut shapes = vec![Vec::new(); BANDS];
    let mut supported = vec![0usize; BANDS];
    let mut boosts = vec![0usize; BANDS];
    for (f, anchor) in &active {
        let center = median(
            &f.envelope_db
                .iter()
                .enumerate()
                .filter(|(i, _)| anchor.band_supported[*i])
                .map(|(_, v)| *v)
                .collect::<Vec<_>>(),
        );
        for i in 0..BANDS {
            shapes[i].push(f.envelope_db[i] - center);
            if anchor.band_supported[i] {
                supported[i] += 1;
                if anchor.cancellation_db[i] > -9. {
                    boosts[i] += 1;
                }
            }
        }
    }
    let values_db = shapes.iter().map(|v| median(v)).collect::<Vec<_>>();
    let uncertainty_db = shapes
        .iter()
        .zip(&values_db)
        .map(|(v, med)| median(&v.iter().map(|x| (x - med).abs()).collect::<Vec<_>>()) * 1.4826)
        .collect::<Vec<_>>();
    let phrase_variation_db = median(
        &uncertainty_db
            .iter()
            .zip(frequencies())
            .filter(|(_, hz)| *hz >= 125. && *hz <= 1000.)
            .map(|(v, _)| *v)
            .collect::<Vec<_>>(),
    );
    let n = active.len();
    Envelope {
        values_db,
        uncertainty_db: uncertainty_db.clone(),
        supported: supported
            .iter()
            .enumerate()
            .map(|(i, &v)| n >= 8 && v as f64 >= n as f64 * 0.7 && uncertainty_db[i] <= 6.)
            .collect(),
        boost_supported: boosts
            .iter()
            .map(|&v| n >= 8 && v as f64 >= n as f64 * 0.9)
            .collect(),
        active_seconds: active.iter().map(|(f, _)| f.end - f.start).sum(),
        eligible_windows: n,
        phrase_variation_db,
        activity_threshold_dbfs: threshold,
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Proposal {
    pub bands: Vec<EqBand>,
    pub reason: String,
    pub supported: Vec<bool>,
    pub boost_supported: Vec<bool>,
    pub target_db: Vec<f64>,
    pub tolerance_db: Vec<f64>,
    pub before_error_db: f64,
    pub predicted_error_db: f64,
    pub removed_level_offset_db: f64,
}
pub fn response(bands: &[EqBand], rate: u32, hz: f64, amount: f64) -> f64 {
    bands
        .iter()
        .map(|b| {
            let mut e = b.clone();
            e.db *= amount / 100.;
            10. * Biquad::equalizer(&e, rate)
                .power_response(hz, rate)
                .max(1e-24)
                .log10()
        })
        .sum()
}
fn centered_delta(values: &[f64], target: &[f64], mask: &[bool]) -> (Vec<f64>, f64) {
    let offset = median(
        &values
            .iter()
            .zip(target)
            .zip(mask)
            .filter(|(_, on)| **on)
            .map(|((a, b), _)| b - a)
            .collect::<Vec<_>>(),
    );
    (
        values
            .iter()
            .zip(target)
            .map(|(a, b)| b - a - offset)
            .collect(),
        offset,
    )
}
pub fn error(values: &[f64], target: &[f64], tolerance: &[f64], mask: &[bool]) -> f64 {
    let (diff, _) = centered_delta(values, target, mask);
    let mut count = 0;
    let mut sum = 0.;
    for i in 0..BANDS {
        if mask[i] {
            count += 1;
            sum += (diff[i].abs() - tolerance[i]).max(0.).powi(2);
        }
    }
    (sum / count.max(1) as f64).sqrt()
}
pub(super) fn bounded(bands: &[EqBand], rate: u32, mask: &[bool], boost: &[bool]) -> bool {
    // Dense grid checks bound total gain and unsupported-region leakage, including skirts.
    (0..241).all(|i| {
        let hz = 20. * (rate as f64 * 0.45 / 20.).powf(i as f64 / 240.);
        let r = response(bands, rate, hz, 100.);
        let idx = frequencies()
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| (hz / **a).log2().abs().total_cmp(&(hz / **b).log2().abs()))
            .unwrap()
            .0;
        r.abs() <= 6.000001 && (r <= 0.25 || mask[idx] && boost[idx])
    })
}
pub fn fit(source: &Envelope, map: &ToneMap, rate: u32, capacity: usize) -> Result<Proposal> {
    map.validate()?;
    if source.values_db.len() != BANDS
        || source.uncertainty_db.len() != BANDS
        || source.supported.len() != BANDS
        || source.boost_supported.len() != BANDS
        || source.values_db.iter().any(|v| !v.is_finite())
        || source
            .uncertainty_db
            .iter()
            .any(|v| !v.is_finite() || *v < 0.)
        || !valid(source.active_seconds, 0., 86400.)
        || !valid(source.phrase_variation_db, 0., 300.)
        || !(8000..=192000).contains(&rate)
        || capacity > 8
    {
        return Err("invalid source envelope, sample rate or EQ capacity".into());
    }
    let target = if map.kind == MapKind::MeasuredEnvelope {
        map.values_db.clone()
    } else {
        source
            .values_db
            .iter()
            .zip(&map.values_db)
            .map(|(a, b)| a + b)
            .collect()
    };
    let mask = frequencies()
        .iter()
        .enumerate()
        .map(|(i, &hz)| {
            source.supported[i]
                && map.supported[i]
                && hz >= map.supported_hz[0]
                && hz <= map.supported_hz[1]
                && hz < rate as f64 * 0.4
        })
        .collect::<Vec<_>>();
    let tolerance = map
        .uncertainty_db
        .iter()
        .zip(&source.uncertainty_db)
        .map(|(a, b)| {
            if map.kind == MapKind::RelativeIntent {
                *a
            } else {
                a.max(b.min(3.))
            }
        })
        .collect::<Vec<_>>();
    let before = error(&source.values_db, &target, &tolerance, &mask);
    let (_, offset) = centered_delta(&source.values_db, &target, &mask);
    let mut p = Proposal {
        bands: vec![],
        reason: String::new(),
        supported: mask,
        boost_supported: source.boost_supported.clone(),
        target_db: target,
        tolerance_db: tolerance,
        before_error_db: before,
        predicted_error_db: before,
        removed_level_offset_db: offset,
    };
    if map.kind == MapKind::Preserve {
        p.reason = "Explicit keep-current choice".into();
        return Ok(p);
    }
    if source.phrase_variation_db < 0.35
        || source.active_seconds < 3.
        || p.supported.iter().filter(|b| **b).count() < 8
    {
        p.reason = "Insufficient representative, non-noise spectral evidence".into();
        return Ok(p);
    }
    if before < 0.5 {
        p.reason = "Already compatible within uncertainty; no added EQ".into();
        return Ok(p);
    }
    let centers = [
        125., 200., 315., 500., 800., 1250., 2000., 3150., 5000., 8000.,
    ];
    let score = |bands: &[EqBand]| {
        let values = source
            .values_db
            .iter()
            .zip(frequencies())
            .map(|(v, hz)| v + response(bands, rate, hz, 100.))
            .collect::<Vec<_>>();
        let error = error(&values, &p.target_db, &p.tolerance_db, &p.supported);
        (
            error * error + 0.025 * bands.iter().map(|e| e.db * e.db).sum::<f64>(),
            error,
        )
    };
    let mut bands = Vec::<EqBand>::new();
    for _ in 0..3 {
        let mut best = score(&bands).0;
        let mut next = None;
        for hz in centers {
            if hz < map.supported_hz[0]
                || hz > map.supported_hz[1]
                || hz >= rate as f64 * 0.4
                || bands.iter().any(|b| b.hz == hz)
            {
                continue;
            }
            for step in -12..=12 {
                if step == 0 {
                    continue;
                }
                let mut candidate = bands.clone();
                candidate.push(EqBand {
                    kind: EqKind::Bell,
                    hz,
                    q: 0.7,
                    db: step as f64 * 0.25,
                });
                if !bounded(&candidate, rate, &p.supported, &p.boost_supported) {
                    continue;
                }
                let s = score(&candidate).0;
                if s < best - 1e-9 {
                    best = s;
                    next = Some(candidate);
                }
            }
        }
        match next {
            Some(c) => bands = c,
            None => break,
        }
    }
    let after = score(&bands).1;
    if bands.is_empty() || before - after < 0.3 || after > before * 0.8 {
        p.reason = "No useful bounded broad-filter fit; baseline retained".into();
    } else if bands.len() > capacity {
        p.reason = format!(
            "Filter capacity failure: proposal needs {} slots, only {capacity} remain; existing EQ retained",
            bands.len()
        );
    } else {
        p.reason = "Bounded training proposal; actual DSP checks required".into();
        p.bands = bands;
        p.predicted_error_db = after;
    }
    Ok(p)
}
/// Deterministic settings projection. 0% returns the baseline without appending zero-gain filters.
pub fn apply(
    baseline: &Session,
    group: &InputGroup,
    bands: &[EqBand],
    amount: f64,
) -> Result<Session> {
    validate_baseline(baseline)?;
    group.validate(baseline)?;
    if !valid(amount, 0., 100.)
        || bands.len() > 3
        || bands.iter().any(|b| {
            b.kind != EqKind::Bell
                || b.q != 0.7
                || !valid(b.db, -3., 3.)
                || !valid(b.hz, 31.25, baseline.sample_rate as f64 * 0.4)
        })
    {
        return Err("invalid matching amount or bounded proposal".into());
    }
    let mut s = baseline.clone();
    if amount == 0. {
        return Ok(s);
    }
    for i in &group.inputs {
        let c = &mut s.channels[i.channel];
        if c.eq.len() + bands.len() > 8 {
            return Err("matching filter capacity exceeded; baseline preserved".into());
        }
        c.eq.extend(bands.iter().map(|b| {
            let mut e = b.clone();
            e.db *= amount / 100.;
            e
        }));
    }
    s.validate()?;
    Ok(s)
}
