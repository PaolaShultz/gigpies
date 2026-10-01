//! Unity-source comparison: measured compressor setup, explicit peak headroom, no loudness matching.
use super::{
    config::{Channel, Group, OutputMode, Role, Session},
    dsp::{Meter, Strip, db},
    write_json,
};
use crate::inventory::Result;
use serde::Serialize;
use std::{
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};

fn walk(path: &Path, expected_rate: u32, mut visit: impl FnMut([f64; 2], usize)) -> Result<()> {
    let mut r = hound::WavReader::open(path)?;
    let p = r.spec();
    if p.sample_rate != expected_rate || !(1..=2).contains(&p.channels) {
        return Err("unsupported source format/rate".into());
    }
    let scale = 2_f64.powi(p.bits_per_sample as i32 - 1);
    for _ in 0..r.duration() {
        let mut x = [0.; 2];
        for v in x.iter_mut().take(p.channels as usize) {
            *v = match p.sample_format {
                hound::SampleFormat::Int => {
                    r.samples::<i32>().next().ok_or("truncated source")?? as f64 / scale
                }
                hound::SampleFormat::Float => {
                    r.samples::<f32>().next().ok_or("truncated source")?? as f64
                }
            };
            if !v.is_finite() {
                return Err("nonfinite source".into());
            }
        }
        if p.channels == 1 {
            x[1] = x[0];
        }
        visit(x, p.channels as usize);
    }
    Ok(())
}
fn percentile(values: &[f64], fraction: f64) -> f64 {
    if values.is_empty() {
        return -240.;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    sorted[((sorted.len() - 1) as f64 * fraction).round() as usize]
}
#[derive(Serialize)]
pub struct EnvelopeSummary {
    pub window_ms: u32,
    pub windows: usize,
    pub active_windows: usize,
    pub rms_p10_dbfs: f64,
    pub rms_p50_dbfs: f64,
    pub rms_p90_dbfs: f64,
    pub crest_p10_db: f64,
    pub crest_p50_db: f64,
    pub rms_range_p90_p10_db: f64,
    pub peak_dbfs: f64,
}
/// 100 ms envelope audit: peak, RMS, crest, left/right levels and correlation.
/// Measurement only; this does not normalize, match or rebalance the file.
pub fn audit(path: &Path, out: &Path) -> Result<EnvelopeSummary> {
    let rate = hound::WavReader::open(path)?.spec().sample_rate;
    let hop = rate as usize / 10;
    let mut writer = BufWriter::new(File::create(out)?);
    writeln!(
        writer,
        "end_frame,peak_dbfs,rms_dbfs,crest_db,left_rms_dbfs,right_rms_dbfs,correlation"
    )?;
    let mut count = 0usize;
    let mut rms = Vec::new();
    let mut crest = Vec::new();
    let mut peak = 0_f64;
    let mut meter = Meter::default();
    let mut lr = [0.; 2];
    let mut cross = 0.;
    let mut rows = Vec::new();
    walk(path, rate, |x, _| {
        for v in x {
            meter.add(v);
        }
        for c in 0..2 {
            lr[c] += x[c] * x[c];
        }
        cross += x[0] * x[1];
        count += 1;
        if count.is_multiple_of(hop) {
            let level = db(meter.rms());
            let cf = db(meter.peak) - level;
            peak = peak.max(meter.peak);
            if level > -60. {
                rms.push(level);
                crest.push(cf);
            }
            rows.push((
                count,
                db(meter.peak),
                level,
                cf,
                db((lr[0] / hop as f64).sqrt()),
                db((lr[1] / hop as f64).sqrt()),
                cross / (lr[0] * lr[1]).sqrt().max(1e-24),
            ));
            meter = Meter::default();
            lr = [0.; 2];
            cross = 0.;
        }
    })?;
    // Include a final partial window rather than discarding a late transient.
    let tail = count % hop;
    if tail > 0 {
        let level = db(meter.rms());
        let cf = db(meter.peak) - level;
        peak = peak.max(meter.peak);
        if level > -60. {
            rms.push(level);
            crest.push(cf);
        }
        rows.push((
            count,
            db(meter.peak),
            level,
            cf,
            db((lr[0] / tail as f64).sqrt()),
            db((lr[1] / tail as f64).sqrt()),
            cross / (lr[0] * lr[1]).sqrt().max(1e-24),
        ));
    }
    for r in &rows {
        writeln!(
            writer,
            "{},{:.5},{:.5},{:.5},{:.5},{:.5},{:.6}",
            r.0, r.1, r.2, r.3, r.4, r.5, r.6
        )?;
    }
    writer.flush()?;
    Ok(EnvelopeSummary {
        window_ms: 100,
        windows: rows.len(),
        active_windows: rms.len(),
        rms_p10_dbfs: percentile(&rms, 0.1),
        rms_p50_dbfs: percentile(&rms, 0.5),
        rms_p90_dbfs: percentile(&rms, 0.9),
        crest_p10_db: percentile(&crest, 0.1).max(0.),
        crest_p50_db: percentile(&crest, 0.5).max(0.),
        rms_range_p90_p10_db: percentile(&rms, 0.9) - percentile(&rms, 0.1),
        peak_dbfs: db(peak),
    })
}
pub fn prepare(s: &mut Session) -> Result<()> {
    s.channels.retain(|ch| !matches!(ch.role, Role::BassAmp));
    s.groups.clear();
    for (i, ch) in s.channels.iter_mut().enumerate() {
        ch.fader_db = 0.;
        ch.hpf_hz = if matches!(ch.role, Role::Kick | Role::BassDi) {
            0.
        } else {
            90.
        };
        if !s.groups.iter().any(|g| g.name == ch.group) {
            s.groups.push(Group {
                name: ch.group.clone(),
                reference: i,
                trim_db: 0.,
            });
        }
    }
    s.calibration.initial_trim_db = 0.;
    s.master_db = 0.;
    s.master_hpf_hz = 40.;
    s.ceiling_db = -0.01;
    s.output_mode = OutputMode::Unmatched;
    s.prepared = true;
    if s.effects.is_some() {
        super::effects::add_pass(s)?;
        let fx = s.effects.as_mut().unwrap();
        fx.master_eq.clear();
        for bus in &mut fx.buses {
            bus.hpf_hz = 90.;
        }
        fx.maximizer_drive_db = 0.;
        fx.maximizer_threshold_db = 24.;
    }
    s.validate()
}
fn measure_strip(ch: &Channel, path: &Path, rate: u32) -> Result<(Meter, f64)> {
    let mut strip = Strip::new(ch, rate);
    let mut meter = Meter::default();
    walk(path, rate, |x, n| {
        let y = strip.tick(x);
        for v in y.iter().take(n) {
            meter.add(*v);
        }
    })?;
    Ok((meter, strip.max_reduction))
}
/// Preserves each source's own level; no common RMS target and no input normalization.
pub fn calibrate_compressors(s: &mut Session, root: &Path) -> Result<Vec<serde_json::Value>> {
    let mut log = Vec::new();
    let rate = s.sample_rate;
    for ch in &mut s.channels {
        let mut flat = ch.clone();
        flat.compressor.ratio = 1.;
        flat.compressor.makeup_db = 0.;
        let mut strip = Strip::new(&flat, rate);
        let mut meter = Meter::default();
        let mut block = Meter::default();
        let mut windows = Vec::new();
        let mut frames = 0;
        walk(&root.join(&ch.file), rate, |x, n| {
            let y = strip.tick(x);
            for v in y.iter().take(n) {
                meter.add(*v);
                block.add(*v);
            }
            frames += 1;
            if frames % (rate / 10) == 0 {
                windows.push((db(block.rms()), db(block.peak)));
                block = Meter::default();
            }
        })?;
        if block.samples > 0 {
            windows.push((db(block.rms()), db(block.peak)));
        }
        let highest_rms = windows.iter().map(|w| w.0).fold(-240., f64::max);
        let active: Vec<_> = windows
            .iter()
            .filter(|w| w.0 > -65. && w.0 > highest_rms - 18.)
            .map(|w| w.1)
            .collect();
        let reference = percentile(&active, 0.9);
        let desired = match ch.role {
            Role::Snare => 5.,
            Role::Kick => 3.,
            Role::BassDi | Role::BassAmp => 4.,
            Role::LeadVocal => 3.,
            Role::Tom => 2.,
            Role::Overheads => 0.5,
            _ => 1.,
        };
        let old_threshold = ch.compressor.threshold_db;
        ch.compressor.makeup_db = 0.;
        if active.is_empty() {
            ch.compressor.ratio = 1.;
        } else if ch.compressor.ratio > 1. {
            ch.compressor.threshold_db =
                (reference - desired / (1. - 1. / ch.compressor.ratio)).clamp(-60., 0.);
        }
        let (mut after, mut max_gr) = measure_strip(ch, &root.join(&ch.file), rate)?;
        let budget = desired + 1.;
        if max_gr > budget && ch.compressor.ratio > 1. {
            ch.compressor.threshold_db = (ch.compressor.threshold_db
                + (max_gr - budget) / (1. - 1. / ch.compressor.ratio))
                .min(0.);
            (after, max_gr) = measure_strip(ch, &root.join(&ch.file), rate)?;
        }
        let loss = (db(meter.rms()) - db(after.rms())).max(0.);
        ch.compressor.makeup_db = loss.min(6.);
        log.push(serde_json::json!({"file":ch.file,"input_trim_db":0.,"fader_db":0.,"hpf_hz":ch.hpf_hz,"active_100ms_windows":active.len(),"post_eq_peak_dbfs":db(meter.peak),"post_eq_rms_dbfs":db(meter.rms()),"active_peak_p90_dbfs":reference,"old_threshold_dbfs":old_threshold,"measured_threshold_dbfs":ch.compressor.threshold_db,"nominal_reduction_at_reference_db":desired,"measured_max_reduction_db":max_gr,"measured_compression_rms_loss_db":loss,"makeup_db":ch.compressor.makeup_db,"reason":"Threshold follows this source's measured post-EQ peaks; makeup compensates only its measured compressor loss, capped at 6 dB. No shared input-level target."}));
    }
    s.validate()?;
    Ok(log)
}
pub fn run(mut s: Session, root: &Path, out: &Path) -> Result<()> {
    prepare(&mut s)?;
    std::fs::create_dir(out)?;
    write_json(&out.join("unity-settings-before-measurement.json"), &s)?;
    let calibration = calibrate_compressors(&mut s, root)?;
    write_json(&out.join("measured-channel-decisions.json"), &calibration)?;
    super::run(s.clone(), root, &out.join("measurement-pass"), None)?;
    let metrics: serde_json::Value =
        serde_json::from_reader(File::open(out.join("measurement-pass/measurements.json"))?)?;
    let mut actions = Vec::new();
    if let Some(fx) = &mut s.effects {
        // A two-dB peak shave in the floating internal bus; output headroom is separate.
        let peak = metrics["processed_bus"]["peak_dbfs"]
            .as_f64()
            .ok_or("missing peak meter")?;
        fx.maximizer_threshold_db = (peak - 2.).clamp(-30., 24.);
        fx.maximizer_drive_db = 0.;
        actions.push(serde_json::json!({"parameter":"maximizer_threshold_db","value":fx.maximizer_threshold_db,"measured_unlimited_peak_dbfs":peak,"reason":"Two dB below the measured processed bus peak; no additional drive or loudness target."}));
        for (i, b) in fx.buses.iter_mut().enumerate() {
            if let (Some(d), Some(w)) = (
                metrics["effects"]["send_reference_meters"][i]["rms_dbfs"].as_f64(),
                metrics["effects"]["return_meters_before_master"][i]["rms_dbfs"].as_f64(),
            ) && d > -70.
                && w > -110.
            {
                b.return_db = (b.target_wet_db - (w - d)).clamp(-12., 12.);
                actions.push(serde_json::json!({"bus":b.name,"return_db":b.return_db,"measured_wet_source_db":w-d,"reason":"Measured FX return calibration, not source gain normalization."}));
            }
        }
    }
    write_json(&out.join("measured-bus-decisions.json"), &actions)?;
    write_json(&out.join("settings.json"), &s)?;
    super::run(s, root, &out.join("final"), None)?;
    let raw = audit(&out.join("final/bypass.wav"), &out.join("raw-envelope.csv"))?;
    let processed = audit(
        &out.join("final/processed.wav"),
        &out.join("processed-envelope.csv"),
    )?;
    write_json(
        &out.join("envelope-review.json"),
        &serde_json::json!({"raw":raw,"processed":processed,"median_crest_change_db":processed.crest_p50_db-raw.crest_p50_db,"rms_range_change_db":processed.rms_range_p90_p10_db-raw.rms_range_p90_p10_db,"flag_crest_loss_over_3db":processed.crest_p50_db<raw.crest_p50_db-3.,"flag_dynamic_range_loss_over_3db":processed.rms_range_p90_p10_db<raw.rms_range_p90_p10_db-3.,"note":"Envelope measurements do not establish subjective instrument balance. No gains are adjusted to make the files equally loud."}),
    )?;
    let p = super::analysis::Policy::default();
    write_json(
        &out.join("raw-spectrum.json"),
        &super::analysis::analyze(&out.join("final/bypass.wav"), &p)?,
    )?;
    write_json(
        &out.join("processed-spectrum.json"),
        &super::analysis::analyze(&out.join("final/processed.wav"), &p)?,
    )?;
    println!("Unity-source pass complete: {}", out.display());
    Ok(())
}
