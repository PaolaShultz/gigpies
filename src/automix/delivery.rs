//! Versioned delivery sidecar. Static exports never choose which DSP runs.
use super::{
    config::{OutputMode, Session},
    dsp::gain,
    identity, render,
    true_peak::{self, Measurement},
};
use crate::inventory::Result;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum FinalLimiter {
    Disabled,
    Enabled {
        threshold_dbfs: f64,
        release_ms: f64,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum DeliveryGain {
    IndependentPeak { max_boost_db: f64 },
    Fixed { gain_db: f64 },
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PeakBasis {
    Sample,
    True,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum Comparison {
    None,
    Loudness { target_lufs: f64 },
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub version: u32,
    pub final_sample_limiter: FinalLimiter,
    pub delivery_gain: DeliveryGain,
    pub peak_basis: PeakBasis,
    pub ceiling_db: f64,
    pub comparison: Comparison,
    pub meter: String,
    pub sample_format: String,
    pub dither: String,
}
impl Default for Policy {
    fn default() -> Self {
        Self {
            version: 1,
            final_sample_limiter: FinalLimiter::Disabled,
            delivery_gain: DeliveryGain::IndependentPeak { max_boost_db: 12. },
            peak_basis: PeakBasis::True,
            ceiling_db: -1.,
            comparison: Comparison::None,
            meter: true_peak::ALGORITHM.into(),
            sample_format: "pcm24".into(),
            dither: "none".into(),
        }
    }
}
impl Policy {
    pub fn validate(&self) -> Result<()> {
        let range = |x: f64, a: f64, b: f64| x.is_finite() && (a..=b).contains(&x);
        if self.version != 1
            || self.meter != true_peak::ALGORITHM
            || self.sample_format != "pcm24"
            || self.dither != "none"
            || !range(self.ceiling_db, -60., -0.01)
        {
            return Err("unsupported delivery policy/version/meter/quantization/ceiling".into());
        }
        if let FinalLimiter::Enabled {
            threshold_dbfs,
            release_ms,
        } = self.final_sample_limiter
            && (!range(threshold_dbfs, -24., 0.) || !range(release_ms, 10., 2000.))
        {
            return Err("invalid final limiter".into());
        }
        let valid = match self.delivery_gain {
            DeliveryGain::IndependentPeak { max_boost_db } => range(max_boost_db, 0., 24.),
            DeliveryGain::Fixed { gain_db } => range(gain_db, -120., 24.),
        };
        if !valid {
            return Err("invalid delivery gain".into());
        }
        if let Comparison::Loudness { target_lufs } = self.comparison
            && !range(target_lufs, -60., -5.)
        {
            return Err("invalid comparison target".into());
        }
        Ok(())
    }
    pub fn margin_db(&self) -> f64 {
        if self.peak_basis == PeakBasis::True {
            0.4001
        } else {
            0.0001
        }
    }
    fn peak(&self, m: &Measurement) -> f64 {
        if self.peak_basis == PeakBasis::True {
            m.true_peak_dbtp
        } else {
            m.sample_peak_dbfs
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Export {
    pub input: Measurement,
    pub output: Measurement,
    pub gain_steps_db: Vec<f64>,
    pub gain_db: f64,
    pub ceiling_db: f64,
    pub reserved_margin_db: f64,
    pub remaining_margin_db: f64,
    pub bus_sha256: String,
    pub pcm_sha256: String,
}
fn export(
    input: &Path,
    output: &Path,
    p: &Policy,
    m: Measurement,
    requested: Option<f64>,
) -> Result<Export> {
    let mut g = requested.unwrap_or_else(|| match p.delivery_gain {
        DeliveryGain::Fixed { gain_db } => gain_db,
        DeliveryGain::IndependentPeak { max_boost_db } => {
            if m.true_peak.amplitude == 0. {
                0.
            } else {
                (p.ceiling_db - p.margin_db() - p.peak(&m)).min(max_boost_db)
            }
        }
    });
    if p.peak(&m) + g > p.ceiling_db - p.margin_db() + 1e-10 && m.true_peak.amplitude > 0. {
        return Err("fixed gain lacks declared peak headroom".into());
    }
    let bus_sha256 = identity::file_hash(input)?;
    let mut steps = vec![g];
    render::export(input, output, gain(g))?;
    let mut measured = true_peak::measure(output)?;
    if p.peak(&measured) > p.ceiling_db {
        if matches!(p.delivery_gain, DeliveryGain::Fixed { .. }) || requested.is_some() {
            return Err("fixed export exceeds ceiling after quantization".into());
        }
        let correction = p.ceiling_db - p.margin_db() - p.peak(&measured);
        steps.push(correction);
        g += correction;
        // Only our new, incomplete output is replaced; no prior completed export.
        std::fs::remove_file(output)?;
        render::export(input, output, gain(g))?;
        measured = true_peak::measure(output)?;
    }
    if p.peak(&measured) > p.ceiling_db
        || measured.frames != m.frames
        || measured.sample_rate != m.sample_rate
        || identity::file_hash(input)? != bus_sha256
    {
        return Err("export verification failed or bus changed".into());
    }
    Ok(Export {
        remaining_margin_db: p.ceiling_db - p.peak(&measured),
        input: m,
        output: measured,
        gain_steps_db: steps,
        gain_db: g,
        ceiling_db: p.ceiling_db,
        reserved_margin_db: p.margin_db(),
        bus_sha256,
        pcm_sha256: identity::file_hash(output)?,
    })
}

pub(super) fn finish(
    s: &Session,
    root: &Path,
    out: &Path,
    p: &Policy,
    sources: &[identity::SourceIdentity],
) -> Result<()> {
    p.validate()?;
    render::write_json(&out.join("delivery-policy.json"), p)?;
    render::write_json(&out.join("sources.json"), &sources)?;
    let bus = out.join("processed-unity-float.wav");
    let input = true_peak::measure(&bus)?;
    let primary = export(&bus, &out.join("processed.wav"), p, input.clone(), None)?;
    let mut copies = BTreeMap::new();
    if let Comparison::Loudness { target_lufs } = p.comparison {
        let bypass = out.join("bypass-unity-float.wav");
        let dry = true_peak::measure(&bypass)?;
        let (a, b) = (
            dry.integrated_lufs.ok_or("bypass has no gated loudness")?,
            input
                .integrated_lufs
                .ok_or("processed bus has no gated loudness")?,
        );
        let target = target_lufs
            .min(a + p.ceiling_db - p.margin_db() - p.peak(&dry))
            .min(b + p.ceiling_db - p.margin_db() - p.peak(&input));
        copies.insert(
            "bypass-matched.wav",
            export(
                &bypass,
                &out.join("bypass-matched.wav"),
                p,
                dry,
                Some(target - a),
            )?,
        );
        copies.insert(
            "processed-matched.wav",
            export(
                &bus,
                &out.join("processed-matched.wav"),
                p,
                input,
                Some(target - b),
            )?,
        );
    }
    if identity::source_identities(s, root)? != sources {
        return Err("sources changed during delivery".into());
    }
    render::write_json(
        &out.join("delivery.json"),
        &serde_json::json!({"version":1,"contract":"gigpies-delivery-v1","policy_sha256":identity::identity(p)?,"settings_sha256":identity::identity(s)?,"effective_final_sample_limiter":p.final_sample_limiter,"fx_master":"retained_from_frozen_session","quantization":"round_away_from_zero_pcm24_no_dither","primary":primary,"comparison_copies":copies,"independent_meter_verification":"separate_required_for_study_acceptance","listener_preference":"not_reviewed"}),
    )?;
    // The engine report may have been copied from a legacy replay. Its old
    // finalization fields must not describe the new primary PCM by accident.
    let mut engine: serde_json::Value =
        serde_json::from_reader(std::fs::File::open(out.join("measurements.json"))?)?;
    let export_fields = [
        "common_export_gain_db",
        "bypass_export_gain_db",
        "processed_export_gain_db",
        "bypass_export_peak_dbfs",
        "processed_export_peak_dbfs",
        "matched_target_lufs",
        "bypass_matching_gain_db",
        "processed_matching_gain_db",
    ];
    if engine["render_contract"] != "delivery-policy-v1" {
        let historical: serde_json::Map<String, serde_json::Value> = export_fields
            .iter()
            .map(|key| (key.to_string(), engine[key].clone()))
            .collect();
        engine["historical_export_observations"] = historical.into();
    }
    for key in export_fields {
        engine[key] = serde_json::Value::Null;
    }
    engine["render_contract"] = "delivery-policy-v1".into();
    engine["effective_final_sample_limiter"] = serde_json::to_value(&p.final_sample_limiter)?;
    engine["export_report"] = "delivery.json".into();
    std::fs::write(
        out.join("measurements.json"),
        serde_json::to_vec_pretty(&engine)?,
    )?;
    std::fs::write(out.join("report.txt"), summary_text(p, &primary))?;
    let mut files = BTreeMap::new();
    for name in [
        "prepared.json",
        "measurements.json",
        "sources.json",
        "delivery-policy.json",
        "delivery.json",
        "report.txt",
        "processed-unity-float.wav",
        "processed.wav",
    ] {
        files.insert(name, identity::file_hash(&out.join(name))?);
    }
    for name in copies.keys() {
        files.insert(name, identity::file_hash(&out.join(name))?);
    }
    if !copies.is_empty() {
        files.insert(
            "bypass-unity-float.wav",
            identity::file_hash(&out.join("bypass-unity-float.wav"))?,
        );
    }
    // Publish completion only after every retained dependency is durable.
    for name in files.keys() {
        std::fs::File::open(out.join(name))?.sync_all()?;
    }
    let ready = serde_json::json!({"version":1,"contract":"gigpies-delivery-v1","files":files});
    render::write_json(&out.join("delivery-ready.pending.json"), &ready)?;
    std::fs::File::open(out.join("delivery-ready.pending.json"))?.sync_all()?;
    std::fs::hard_link(
        out.join("delivery-ready.pending.json"),
        out.join("delivery-ready.json"),
    )?;
    std::fs::remove_file(out.join("delivery-ready.pending.json"))?;
    std::fs::File::open(out)?.sync_all()?;
    Ok(())
}

/// Reuse a retained float bus. Refuses to change any enabled DSP retroactively.
pub fn finalize(render_dir: &Path, root: &Path, out: &Path, p: Policy) -> Result<()> {
    p.validate()?;
    let s: Session =
        serde_json::from_reader(std::fs::File::open(render_dir.join("prepared.json"))?)?;
    s.validate()?;
    let report: serde_json::Value =
        serde_json::from_reader(std::fs::File::open(render_dir.join("measurements.json"))?)?;
    if !render_dir.join("report.txt").is_file()
        || report["frames"].as_u64().is_none()
        || report["mode"] != "frozen_show"
    {
        return Err("incomplete/nonfrozen render".into());
    }
    let effective = if let Some(v) = report.get("effective_final_sample_limiter") {
        serde_json::from_value(v.clone())?
    } else if s.output_mode == OutputMode::Matched {
        FinalLimiter::Enabled {
            threshold_dbfs: s.ceiling_db,
            release_ms: s.limiter_release_ms,
        }
    } else {
        FinalLimiter::Disabled
    };
    if effective != p.final_sample_limiter {
        return Err("delivery policy cannot retroactively change rendered DSP".into());
    }
    let sources = identity::source_identities(&s, root)?;
    let m = true_peak::measure(&render_dir.join("processed-unity-float.wav"))?;
    if Some(m.frames) != report["frames"].as_u64() || m.sample_rate != s.sample_rate {
        return Err("retained bus timeline mismatch".into());
    }
    std::fs::create_dir(out)?;
    for name in ["prepared.json", "measurements.json", "report.txt"] {
        std::fs::copy(render_dir.join(name), out.join(name))?;
    }
    // Hard links share immutable retained buses; export never writes its input.
    std::fs::hard_link(
        render_dir.join("processed-unity-float.wav"),
        out.join("processed-unity-float.wav"),
    )?;
    if !matches!(p.comparison, Comparison::None) {
        std::fs::hard_link(
            render_dir.join("bypass-unity-float.wav"),
            out.join("bypass-unity-float.wav"),
        )?;
    }
    render::write_json(
        &out.join("reused-render.json"),
        &serde_json::json!({"render_directory":render_dir,"settings_sha256":identity::file_hash(&render_dir.join("prepared.json"))?,"report_sha256":identity::file_hash(&render_dir.join("measurements.json"))?,"bus_sha256":identity::file_hash(&render_dir.join("processed-unity-float.wav"))?}),
    )?;
    finish(&s, root, out, &p, &sources)
}

pub fn check(out: &Path, root: &Path) -> Result<()> {
    let ready: serde_json::Value =
        serde_json::from_reader(std::fs::File::open(out.join("delivery-ready.json"))?)?;
    if ready["version"] != 1 || ready["contract"] != "gigpies-delivery-v1" {
        return Err("missing delivery contract".into());
    }
    let files = ready["files"]
        .as_object()
        .ok_or("missing file identities")?;
    for name in [
        "prepared.json",
        "measurements.json",
        "sources.json",
        "delivery-policy.json",
        "delivery.json",
        "report.txt",
        "processed-unity-float.wav",
        "processed.wav",
    ] {
        if !files.contains_key(name) {
            return Err("incomplete delivery identities".into());
        }
    }
    for (name, hash) in files {
        if name.contains(['/', '\\'])
            || identity::file_hash(&out.join(name))? != hash.as_str().ok_or("invalid hash")?
        {
            return Err("delivery file identity mismatch".into());
        }
    }
    let s: Session = serde_json::from_reader(std::fs::File::open(out.join("prepared.json"))?)?;
    s.validate()?;
    let p: Policy =
        serde_json::from_reader(std::fs::File::open(out.join("delivery-policy.json"))?)?;
    p.validate()?;
    let sources: Vec<identity::SourceIdentity> =
        serde_json::from_reader(std::fs::File::open(out.join("sources.json"))?)?;
    let report: serde_json::Value =
        serde_json::from_reader(std::fs::File::open(out.join("delivery.json"))?)?;
    if identity::source_identities(&s, root)? != sources
        || report["policy_sha256"] != identity::identity(&p)?
        || report["settings_sha256"] != identity::identity(&s)?
        || report["contract"] != "gigpies-delivery-v1"
    {
        return Err("stale policy/settings/sources".into());
    }
    let e: Export = serde_json::from_value(report["primary"].clone())?;
    if std::fs::read_to_string(out.join("report.txt"))? != summary_text(&p, &e) {
        return Err("delivery summary contradicts measured PCM".into());
    }
    let engine: serde_json::Value =
        serde_json::from_reader(std::fs::File::open(out.join("measurements.json"))?)?;
    let effective = if let Some(v) = engine.get("effective_final_sample_limiter") {
        serde_json::from_value(v.clone())?
    } else if s.output_mode == OutputMode::Matched {
        FinalLimiter::Enabled {
            threshold_dbfs: s.ceiling_db,
            release_ms: s.limiter_release_ms,
        }
    } else {
        FinalLimiter::Disabled
    };
    if effective != p.final_sample_limiter
        || engine["render_contract"] != "delivery-policy-v1"
        || !engine["processed_export_gain_db"].is_null()
        || report["effective_final_sample_limiter"] != serde_json::to_value(&effective)?
        || engine["frames"].as_u64() != Some(e.output.frames)
        || engine["sample_rate"].as_u64() != Some(s.sample_rate as u64)
    {
        return Err("effective DSP/timeline contradicts delivery policy".into());
    }
    validate_export(
        &out.join("processed-unity-float.wav"),
        &out.join("processed.wav"),
        &p,
        &e,
    )?;
    let measured = true_peak::measure(&out.join("processed.wav"))?;
    if e.output != measured
        || p.peak(&measured) > p.ceiling_db
        || e.pcm_sha256 != identity::file_hash(&out.join("processed.wav"))?
        || e.bus_sha256 != identity::file_hash(&out.join("processed-unity-float.wav"))?
    {
        return Err("contradictory delivery measurement".into());
    }
    let copies = report["comparison_copies"]
        .as_object()
        .ok_or("missing comparison disposition")?;
    match p.comparison {
        Comparison::None if !copies.is_empty() => return Err("undeclared comparison copies".into()),
        Comparison::Loudness { .. } => {
            if copies.len() != 2 {
                return Err("incomplete comparison copies".into());
            }
            for (name, bus) in [
                ("processed-matched.wav", "processed-unity-float.wav"),
                ("bypass-matched.wav", "bypass-unity-float.wav"),
            ] {
                if !files.contains_key(name) || !files.contains_key(bus) {
                    return Err("unbound comparison artifact".into());
                }
                let e: Export =
                    serde_json::from_value(copies.get(name).ok_or("missing comparison")?.clone())?;
                validate_export(&out.join(bus), &out.join(name), &p, &e)?;
                let m = true_peak::measure(&out.join(name))?;
                if m != e.output || p.peak(&m) > p.ceiling_db {
                    return Err("contradictory comparison meter".into());
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn summary_text(p: &Policy, e: &Export) -> String {
    let basis = if p.peak_basis == PeakBasis::True {
        "dBTP"
    } else {
        "dBFS"
    };
    format!(
        "GigPies delivery v1\nStatic export gain: {:.6} dB\nFinal PCM sample peak: {:.6} dBFS\nFinal PCM true peak: {:.6} dBTP\nCeiling: {:.6} {basis}\nReserved margin: {:.6} dB\nFrozen channel and FX settings: prepared.json\nEffective limiter and export controls: delivery-policy.json\nEngine observations: measurements.json; final PCM measurements: delivery.json\nCompletion: delivery-ready.json; independent meter verification is separate.\nListener preference: not reviewed. Hardware: unverified.\n",
        e.gain_db,
        e.output.sample_peak_dbfs,
        e.output.true_peak_dbtp,
        p.ceiling_db,
        p.margin_db()
    )
}

fn validate_export(bus: &Path, pcm: &Path, p: &Policy, e: &Export) -> Result<()> {
    if e.ceiling_db != p.ceiling_db
        || e.reserved_margin_db != p.margin_db()
        || e.gain_steps_db.is_empty()
        || e.gain_steps_db.len() > 2
        || !e.gain_db.is_finite()
        || e.gain_steps_db.iter().any(|v| !v.is_finite())
        || (e.gain_steps_db.iter().sum::<f64>() - e.gain_db).abs() > 1e-10
        || e.input.frames != e.output.frames
        || e.input.sample_rate != e.output.sample_rate
        || e.bus_sha256 != identity::file_hash(bus)?
        || e.pcm_sha256 != identity::file_hash(pcm)?
        || (e.remaining_margin_db - (p.ceiling_db - p.peak(&e.output))).abs() > 1e-10
    {
        return Err("contradictory export controls/identities".into());
    }
    let mut a = hound::WavReader::open(bus)?;
    let mut b = hound::WavReader::open(pcm)?;
    if a.spec().sample_format != hound::SampleFormat::Float
        || a.spec().bits_per_sample != 32
        || b.spec().bits_per_sample != 24
        || b.spec().sample_format != hound::SampleFormat::Int
        || a.len() != b.len()
    {
        return Err("export format mismatch".into());
    }
    let scale = gain(e.gain_db);
    for (x, y) in a.samples::<f32>().zip(b.samples::<i32>()) {
        let x = x? as f64 * scale;
        if !x.is_finite() || x.abs() >= 1. || (x * 8388608.).round() as i32 != y? {
            return Err("PCM is not the declared static quantized bus".into());
        }
    }
    Ok(())
}
