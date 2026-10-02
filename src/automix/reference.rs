//! Read-only comparison of finished mixes. A reference never selects processing.
use super::{analysis::fft, dsp::db, render::Source, tone::percentile, write_json};
use crate::inventory::{Result, inspect_wav};
use serde::Serialize;
use std::path::Path;
const HZ: usize = 50;
const WINDOW: usize = 12 * HZ;
const LAG: i32 = 20 * HZ as i32;
const BANDS: [(f64, f64); 5] = [
    (40., 100.),
    (100., 400.),
    (400., 800.),
    (800., 3200.),
    (3200., 12000.),
];
#[derive(Clone, Serialize)]
pub struct Frame {
    pub seconds: f64,
    pub rms_dbfs: f64,
    pub peak_dbfs: f64,
    pub band_power: [f64; 5],
    pub left_power: f64,
    pub right_power: f64,
    pub cross_power: f64,
}
#[derive(Serialize)]
pub struct Measurement {
    pub rate: u32,
    pub channels: usize,
    pub frames: u64,
    pub full_scale_samples: u64,
    pub float_input: bool,
    pub windows: Vec<Frame>,
}
/// Native-rate samples stay unchanged. Only bounded 20 ms analysis frames are retained.
pub fn measure(path: &Path, common_high_hz: f64) -> Result<Measurement> {
    let info = inspect_wav(path)?;
    if !(8000..=192000).contains(&info.sample_rate_hz)
        || info.duration_seconds > 600.
        || !common_high_hz.is_finite()
        || !(4000. ..=96000.).contains(&common_high_hz)
    {
        return Err("reference review requires 8–192 kHz, at most ten minutes, and a valid common analysis limit".into());
    }
    let rate = info.sample_rate_hz;
    let mut source = Source::open(path, rate)?;
    let count = source.frames * HZ as u64 / rate as u64;
    let mut windows = Vec::with_capacity(count as usize);
    let mut sample = 0;
    for bin in 0..count {
        let end = (bin + 1) * rate as u64 / HZ as u64;
        let n = (end - sample) as usize;
        let size = n.next_power_of_two();
        let mut block = vec![[0.; 2]; n];
        let mut energies = [0.; 3];
        let mut peak = 0_f64;
        for x in &mut block {
            *x = source.next(sample)?;
            sample += 1;
            energies[0] += x[0] * x[0];
            energies[1] += x[1] * x[1];
            energies[2] += x[0] * x[1];
            peak = peak.max(x[0].abs()).max(x[1].abs());
        }
        let hann = (0..n)
            .map(|i| 0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / n as f64).cos())
            .collect::<Vec<_>>();
        let norm = size as f64 * hann.iter().map(|x| x * x).sum::<f64>();
        let mut powers = [0.; 5];
        let mut transform = vec![[0.; 2]; size];
        for c in [0_usize, 1] {
            transform.fill([0.; 2]);
            for i in 0..n {
                transform[i][0] = block[i][c] * hann[i];
            }
            fft(&mut transform);
            for (k, x) in transform.iter().enumerate().take(size / 2).skip(1) {
                let hz = k as f64 * rate as f64 / size as f64;
                for (i, (lo, hi)) in BANDS.iter().enumerate() {
                    if hz >= *lo && hz < hi.min(common_high_hz) {
                        powers[i] += (x[0] * x[0] + x[1] * x[1]) / norm;
                    }
                }
            }
        }
        windows.push(Frame {
            seconds: bin as f64 / HZ as f64,
            rms_dbfs: db(((energies[0] + energies[1]) / (2 * n) as f64).sqrt()),
            peak_dbfs: db(peak),
            band_power: powers,
            left_power: energies[0] / n as f64,
            right_power: energies[1] / n as f64,
            cross_power: energies[2] / n as f64,
        });
    }
    // Read and validate a partial tail too; it has no full analysis window.
    while sample < source.frames {
        source.next(sample)?;
        sample += 1;
    }
    Ok(Measurement {
        rate,
        channels: source.channels,
        frames: source.frames,
        full_scale_samples: source.full_scale_samples(),
        float_input: source.is_float(),
        windows,
    })
}
fn fingerprint(rows: &[Frame]) -> Vec<f64> {
    rows.iter()
        .enumerate()
        .map(|(i, x)| {
            if i == 0 || x.rms_dbfs < -65. || rows[i - 1].rms_dbfs < -65. {
                0.
            } else {
                (x.rms_dbfs - rows[i - 1].rms_dbfs).clamp(-12., 12.)
            }
        })
        .collect()
}
fn correlation(a: &[f64], b: &[f64]) -> Option<f64> {
    let n = a.len() as f64;
    let (mut sa, mut sb, mut aa, mut bb, mut ab) = (0., 0., 0., 0., 0.);
    for (&a, &b) in a.iter().zip(b) {
        sa += a;
        sb += b;
        aa += a * a;
        bb += b * b;
        ab += a * b;
    }
    let va = aa - sa * sa / n;
    let vb = bb - sb * sb / n;
    if va / n < 0.01 || vb / n < 0.01 {
        None
    } else {
        Some(((ab - sa * sb / n) / (va * vb).sqrt()).clamp(-1., 1.))
    }
}
#[derive(Clone, Serialize)]
pub struct Anchor {
    pub ours_start_seconds: f64,
    pub reference_offset_seconds: Option<f64>,
    pub correlation: Option<f64>,
    pub distinct_peak_margin: Option<f64>,
    pub confident: bool,
    pub reason: &'static str,
}
fn anchor(a: &[f64], b: &[f64], start: usize) -> Anchor {
    let mut out = Anchor {
        ours_start_seconds: start as f64 / HZ as f64,
        reference_offset_seconds: None,
        correlation: None,
        distinct_peak_margin: None,
        confident: false,
        reason: "insufficient varying audio or overlap",
    };
    if start + WINDOW > a.len() {
        return out;
    }
    let mut scores = vec![];
    for lag in -LAG..=LAG {
        let j = start as i64 + lag as i64;
        if j < 0 || j as usize + WINDOW > b.len() {
            continue;
        }
        if let Some(c) = correlation(
            &a[start..start + WINDOW],
            &b[j as usize..j as usize + WINDOW],
        ) {
            scores.push((lag, c));
        }
    }
    let Some(&(lag, best)) = scores.iter().max_by(|a, b| a.1.total_cmp(&b.1)) else {
        return out;
    };
    let alternative = scores
        .iter()
        .filter(|(l, _)| (l - lag).abs() > 10)
        .map(|(_, c)| *c)
        .max_by(f64::total_cmp)
        .unwrap_or(-1.);
    out.reference_offset_seconds = Some(lag as f64 / HZ as f64);
    out.correlation = Some(best);
    out.distinct_peak_margin = Some(best - alternative);
    out.confident = best >= 0.35 && best - alternative >= 0.10 && lag.abs() < LAG;
    out.reason = if lag.abs() == LAG {
        "best match touches search boundary"
    } else if out.confident {
        "distinct local envelope match"
    } else {
        "weak or ambiguous envelope match"
    };
    out
}
#[derive(Serialize)]
pub struct Alignment {
    pub status: &'static str,
    pub reference_offset_seconds: Option<f64>,
    pub resolution_seconds: f64,
    pub anchors: Vec<Anchor>,
    pub reason: &'static str,
}
pub fn align(a: &[Frame], b: &[Frame]) -> Alignment {
    let af = fingerprint(a);
    let bf = fingerprint(b);
    let anchors = (0..a.len().saturating_sub(WINDOW) + 1)
        .step_by(WINDOW)
        .take(50)
        .map(|s| anchor(&af, &bf, s))
        .collect::<Vec<_>>();
    let reliable = anchors.iter().filter(|x| x.confident).collect::<Vec<_>>();
    let offsets = reliable
        .iter()
        .map(|x| x.reference_offset_seconds.unwrap())
        .collect::<Vec<_>>();
    let enough = reliable.len() >= 3 && reliable.len() * 4 >= anchors.len() * 3;
    let spread = offsets.iter().copied().max_by(f64::total_cmp).unwrap_or(0.)
        - offsets.iter().copied().min_by(f64::total_cmp).unwrap_or(0.);
    let accepted = enough && spread <= 0.060001;
    Alignment {
        status: if accepted {
            "consistent_offset"
        } else if enough {
            "inconsistent_timeline"
        } else {
            "insufficient_confidence"
        },
        reference_offset_seconds: accepted.then(|| percentile(offsets.iter().copied(), 0.5)),
        resolution_seconds: 1. / HZ as f64,
        anchors,
        reason: if accepted {
            "Coarse event-envelope offset agrees across sections; not sample alignment or proof of identical performance"
        } else if enough {
            "Local offsets disagree; possible edit, drift, or wrong repeated section"
        } else {
            "Too few distinct matches; no automatic comparison excerpts"
        },
    }
}
#[derive(Serialize)]
pub struct Summary {
    pub rms_dbfs: f64,
    pub sample_peak_dbfs: f64,
    pub median_window_crest_db: f64,
    pub body_presence_db: f64,
    pub band_power_dbfs: [f64; 5],
    pub stereo_correlation: Option<f64>,
    pub mono_power_loss_db: Option<f64>,
}
fn summarize(rows: &[Frame]) -> Summary {
    let n = rows.len() as f64;
    let sum = |f: fn(&Frame) -> f64| rows.iter().map(f).sum::<f64>() / n;
    let l = sum(|x| x.left_power);
    let r = sum(|x| x.right_power);
    let cross = sum(|x| x.cross_power);
    let power = (l + r) / 2.;
    let bands =
        std::array::from_fn(|i| db((rows.iter().map(|x| x.band_power[i]).sum::<f64>() / n).sqrt()));
    Summary {
        rms_dbfs: db(power.sqrt()),
        sample_peak_dbfs: rows
            .iter()
            .map(|x| x.peak_dbfs)
            .max_by(f64::total_cmp)
            .unwrap_or(-240.),
        median_window_crest_db: percentile(rows.iter().map(|x| x.peak_dbfs - x.rms_dbfs), 0.5),
        body_presence_db: bands[1] - bands[3],
        band_power_dbfs: bands,
        stereo_correlation: (l * r > 1e-24).then(|| (cross / (l * r).sqrt()).clamp(-1., 1.)),
        mono_power_loss_db: (power > 1e-12)
            .then(|| db((((l + r + 2. * cross) / 4.).max(0.) / power).sqrt())),
    }
}
fn excerpt(input: &Path, output: &Path, start_seconds: f64) -> Result<()> {
    let mut r = hound::WavReader::open(input)?;
    let spec = r.spec();
    let start = (start_seconds * spec.sample_rate as f64).round() as u32;
    let frames = 12 * spec.sample_rate;
    if start
        .checked_add(frames)
        .is_none_or(|end| end > r.duration())
    {
        return Err("excerpt exceeds input".into());
    }
    r.seek(start)?;
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)?;
    let mut w = hound::WavWriter::new(std::io::BufWriter::new(file), spec)?;
    for _ in 0..frames as usize * spec.channels as usize {
        if spec.sample_format == hound::SampleFormat::Float {
            w.write_sample(r.samples::<f32>().next().ok_or("short excerpt")??)?;
        } else {
            w.write_sample(r.samples::<i32>().next().ok_or("short excerpt")??)?;
        }
    }
    w.finalize()?;
    Ok(())
}
pub fn run(ours: &Path, reference: &Path, out: &Path, start: Option<f64>) -> Result<()> {
    let started = std::time::Instant::now();
    if start.is_some_and(|x| !x.is_finite() || !(0. ..=588.).contains(&x)) {
        return Err("invalid excerpt start".into());
    }
    if ours.canonicalize()? == reference.canonicalize()? {
        return Err("comparison needs two different input files".into());
    }
    let ainfo = inspect_wav(ours)?;
    let binfo = inspect_wav(reference)?;
    let meta = |p: &Path| -> Result<_> {
        let m = std::fs::metadata(p)?;
        Ok((m.len(), m.modified()?))
    };
    let original = [meta(ours)?, meta(reference)?];
    if out.exists() {
        return Err("output already exists".into());
    }
    let high = (ainfo.sample_rate_hz.min(binfo.sample_rate_hz) / 2) as f64;
    let a = measure(ours, high)?;
    let b = measure(reference, high)?;
    if original != [meta(ours)?, meta(reference)?] {
        return Err("input changed during review".into());
    }
    let alignment = align(&a.windows, &b.windows);
    let mut sections = vec![];
    for anchor in &alignment.anchors {
        if !anchor.confident {
            continue;
        }
        let i = (anchor.ours_start_seconds * HZ as f64).round() as usize;
        let j = ((anchor.ours_start_seconds + anchor.reference_offset_seconds.unwrap()) * HZ as f64)
            .round() as usize;
        let ours = summarize(&a.windows[i..i + WINDOW]);
        let reference = summarize(&b.windows[j..j + WINDOW]);
        let band_share_difference: [f64; 5] = std::array::from_fn(|k| {
            (reference.band_power_dbfs[k] - reference.rms_dbfs)
                - (ours.band_power_dbfs[k] - ours.rms_dbfs)
        });
        sections.push(serde_json::json!({"ours_start_seconds":anchor.ours_start_seconds,"reference_start_seconds":j as f64/HZ as f64,
            "reference_minus_ours":{"rms_db":reference.rms_dbfs-ours.rms_dbfs,
                "median_window_crest_db":reference.median_window_crest_db-ours.median_window_crest_db,
                "body_presence_db":reference.body_presence_db-ours.body_presence_db,
                "band_share_db":band_share_difference},
            "ours":ours,"reference":reference,"processing_recommendation":null,
            "interpretation":"Whole-mix measurements; band shares are relative to each mix's broadband power, without altering audio. Differences cannot identify an isolated instrument or establish preferred sound"}));
    }
    let requested = start.map(|s| {
        anchor(
            &fingerprint(&a.windows),
            &fingerprint(&b.windows),
            (s * HZ as f64).round() as usize,
        )
    });
    let eligible = requested.as_ref().is_some_and(|x| {
        x.confident
            && alignment
                .reference_offset_seconds
                .is_some_and(|lag| (x.reference_offset_seconds.unwrap() - lag).abs() <= 0.060001)
    });
    std::fs::create_dir(out)?;
    write_json(&out.join("ours-windows.json"), &a)?;
    write_json(&out.join("reference-windows.json"), &b)?;
    let mut exports = vec![];
    if eligible {
        let s = requested.as_ref().unwrap().ours_start_seconds;
        let rs = s + requested
            .as_ref()
            .unwrap()
            .reference_offset_seconds
            .unwrap();
        excerpt(ours, &out.join("01-OUR-MIX.wav"), s)?;
        excerpt(reference, &out.join("02-SUPPLIED-REFERENCE.wav"), rs)?;
        exports = vec![
            serde_json::json!({"file":"01-OUR-MIX.wav","start_seconds":s}),
            serde_json::json!({"file":"02-SUPPLIED-REFERENCE.wav","start_seconds":rs}),
        ];
    }
    if original != [meta(ours)?, meta(reference)?] {
        return Err("input changed during export; discard this incomplete review".into());
    }
    write_json(
        &out.join("review.json"),
        &serde_json::json!({"ours":ainfo,"reference":binfo,"alignment":alignment,"sections":sections,
        "requested_excerpt":requested,"excerpts":exports,"excerpt_duration_seconds":12,"gain_changes":false,"settings_written":false,
        "reference_authority":"User supplied comparison material; official provenance and quality are not inferred from filename",
        "spectral_bands_hz":BANDS,"common_analysis_high_hz":high,"elapsed_seconds":started.elapsed().as_secs_f64(),
        "listener_preference":null,"limits":"20 ms envelope alignment, +/-20 s search, ten-minute inputs, mono/stereo PCM or float. No source separation, reference optimization, loudness matching, true-peak claim or playback."}),
    )?;
    Ok(())
}
