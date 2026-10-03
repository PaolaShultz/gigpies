//! Same-recording EQ evidence. Paired dispersion is descriptive, never a map tolerance.
use super::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComparisonRequest {
    pub group: InputGroup,
    pub training: Vec<[f64; 2]>,
    pub held_out: Vec<[f64; 2]>,
    pub routing_basis: String,
    pub excluded_fx_returns: Vec<std::path::PathBuf>,
}
impl ComparisonRequest {
    fn request(&self) -> Request {
        Request {
            group: self.group.clone(),
            training: self.training.clone(),
            held_out: self.held_out.clone(),
            maps: vec!["keep-current".into()],
            routing_basis: self.routing_basis.clone(),
            excluded_fx_returns: self.excluded_fx_returns.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PairedBand {
    pub hz: f64,
    pub paired_windows: usize,
    pub supported: bool,
    /// Changed minus reference, after removing each pair's common offset in analysis.
    pub shape_change_db: Option<f64>,
    /// Scaled median absolute deviation; not a confidence interval or fit tolerance.
    pub paired_spread_db: Option<f64>,
    pub reference_phrase_spread_db: Option<f64>,
    pub changed_phrase_spread_db: Option<f64>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PairedSpectrum {
    pub spans: Vec<[f64; 2]>,
    pub eligible_windows: usize,
    pub active_seconds: f64,
    pub representative_phrase: bool,
    pub common_offset_db: Option<f64>,
    pub bands: Vec<PairedBand>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComparisonPassage {
    pub split: String,
    pub spectrum: PairedSpectrum,
    pub active_level_windows: usize,
    pub group_rms_change_db: Option<f64>,
    pub body_presence_change_db: Option<f64>,
    pub mix_rms_change_db: Option<f64>,
    pub mix_low_change_db: Option<f64>,
    pub max_group_crest_loss_db: Option<f64>,
    pub max_added_compression_db: Option<f64>,
    pub max_stereo_correlation_change: Option<f64>,
    pub max_added_master_reduction_db: Option<f64>,
    pub return_change_db: Vec<Option<f64>>,
    pub event_count: usize,
    pub attack_change_db: Option<f64>,
    pub body_change_db: Option<f64>,
    pub sustain_change_db: Option<f64>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComparisonReport {
    pub schema_version: u32,
    pub analysis: AnalysisSettings,
    pub reference_id: String,
    pub changed_id: String,
    pub source_files: Vec<SourceIdentity>,
    pub reference: Session,
    pub changed: Session,
    pub request: ComparisonRequest,
    pub activity_threshold_dbfs: f64,
    pub training: PairedSpectrum,
    pub passages: Vec<ComparisonPassage>,
    pub limitations: Vec<String>,
}

fn mid(v: &[f64]) -> Option<f64> {
    (!v.is_empty()).then(|| median(v))
}
fn spread(v: &[f64]) -> Option<f64> {
    mid(v).map(|m| median(&v.iter().map(|x| (x - m).abs()).collect::<Vec<_>>()) * 1.4826)
}
fn paired(a: &Measurement, b: &Measurement, spans: &[[f64; 2]], threshold: f64) -> PairedSpectrum {
    let mut differences = vec![Vec::new(); BANDS];
    let mut reference = differences.clone();
    let mut changed = differences.clone();
    let mut offsets = Vec::new();
    let mut seconds = 0.;
    let frequencies = frequencies();
    for (a, b) in a.windows.iter().zip(&b.windows) {
        if !inside(a.start, a.end, spans)
            || a.raw_rms_dbfs <= threshold
            || a.flatness >= 0.35
            || a.largest_bin_fraction >= 0.30
        {
            continue;
        }
        let mask = (0..BANDS)
            .map(|i| {
                (125. ..=6300.).contains(&frequencies[i])
                    && a.band_supported[i]
                    && b.band_supported[i]
                    && a.cancellation_db[i] > -9.
                    && b.cancellation_db[i] > -9.
            })
            .collect::<Vec<_>>();
        if mask.iter().filter(|on| **on).count() < 8 {
            continue;
        }
        let selected = |values: &[f64]| {
            values
                .iter()
                .zip(&mask)
                .filter(|(_, on)| **on)
                .map(|(x, _)| *x)
                .collect::<Vec<_>>()
        };
        let delta = a
            .envelope_db
            .iter()
            .zip(&b.envelope_db)
            .map(|(a, b)| b - a)
            .collect::<Vec<_>>();
        let offset = median(&selected(&delta));
        let ac = median(&selected(&a.envelope_db));
        let bc = median(&selected(&b.envelope_db));
        offsets.push(offset);
        seconds += a.end - a.start;
        for i in 0..BANDS {
            if mask[i] {
                differences[i].push(delta[i] - offset);
                reference[i].push(a.envelope_db[i] - ac);
                changed[i].push(b.envelope_db[i] - bc);
            }
        }
    }
    let bands = (0..BANDS)
        .map(|i| {
            let n = differences[i].len();
            let supported = n >= 8 && n as f64 >= offsets.len() as f64 * 0.7;
            PairedBand {
                hz: frequencies[i],
                paired_windows: n,
                supported,
                shape_change_db: supported.then(|| median(&differences[i])),
                paired_spread_db: supported.then(|| spread(&differences[i]).unwrap()),
                reference_phrase_spread_db: supported.then(|| spread(&reference[i]).unwrap()),
                changed_phrase_spread_db: supported.then(|| spread(&changed[i]).unwrap()),
            }
        })
        .collect::<Vec<_>>();
    let env = envelope(a, spans, Some(threshold));
    PairedSpectrum {
        spans: spans.to_vec(),
        eligible_windows: offsets.len(),
        active_seconds: seconds,
        representative_phrase: seconds >= 3.
            && env.phrase_variation_db >= 0.35
            && bands.iter().filter(|b| b.supported).count() >= 8,
        common_offset_db: mid(&offsets),
        bands,
    }
}

fn observe(
    a: &Measurement,
    b: &Measurement,
    span: [f64; 2],
    split: &str,
    threshold: f64,
) -> ComparisonPassage {
    let pairs = a
        .windows
        .iter()
        .zip(&b.windows)
        .filter(|(a, _)| inside(a.start, a.end, &[span]) && a.raw_rms_dbfs > threshold)
        .collect::<Vec<_>>();
    let med = |f: fn(&Window, &Window) -> f64| {
        mid(&pairs.iter().map(|(a, b)| f(a, b)).collect::<Vec<_>>())
    };
    let max = |f: fn(&Window, &Window) -> f64| {
        (!pairs.is_empty()).then(|| pairs.iter().map(|(a, b)| f(a, b)).fold(0., f64::max))
    };
    let (events, stages) = check::event_changes(a, b, span);
    ComparisonPassage {
        split: split.into(),
        spectrum: paired(a, b, &[span], threshold),
        active_level_windows: pairs.len(),
        group_rms_change_db: med(|a, b| b.group_rms_dbfs - a.group_rms_dbfs),
        body_presence_change_db: med(|a, b| b.group_body_presence_db - a.group_body_presence_db),
        mix_rms_change_db: med(|a, b| b.mix_rms_dbfs - a.mix_rms_dbfs),
        mix_low_change_db: med(|a, b| b.mix_low_power_dbfs - a.mix_low_power_dbfs),
        max_group_crest_loss_db: max(|a, b| {
            (a.group_peak_dbfs - a.group_rms_dbfs) - (b.group_peak_dbfs - b.group_rms_dbfs)
        }),
        max_added_compression_db: max(|a, b| {
            a.compressor_max_db
                .iter()
                .zip(&b.compressor_max_db)
                .map(|(a, b)| b - a)
                .fold(0., f64::max)
        }),
        max_stereo_correlation_change: max(|a, b| {
            (b.stereo_correlation - a.stereo_correlation).abs()
        }),
        max_added_master_reduction_db: max(|a, b| b.master_reduction_db - a.master_reduction_db),
        return_change_db: (0..a.windows.first().map_or(0, |w| w.return_rms_dbfs.len()))
            .map(|i| {
                mid(&pairs
                    .iter()
                    .filter(|(a, _)| a.return_rms_dbfs[i] > -90.)
                    .map(|(a, b)| b.return_rms_dbfs[i] - a.return_rms_dbfs[i])
                    .collect::<Vec<_>>())
            })
            .collect(),
        event_count: events,
        attack_change_db: stages[0],
        body_change_db: stages[1],
        sustain_change_db: stages[2],
    }
}

/// Only the named group's EQ may differ. One source root and identical remaining
/// settings establish paired sample/routing identity; hashes bracket both measurements.
/// This observer writes no settings, maps, audio or acceptance decision.
pub fn compare_eq(
    reference: Session,
    changed: Session,
    root: &Path,
    out: &Path,
    request: ComparisonRequest,
) -> Result<ComparisonReport> {
    let r = validated_pair(&reference, &changed, &request)?;
    if out.exists() {
        return Err("comparison output must be a new directory".into());
    }
    let sources = source_identities(&reference, root)?;
    let a = measure(&reference, root, &r.group, &spans(&r))?;
    let b = measure(&changed, root, &r.group, &spans(&r))?;
    // Never silently zip truncated, reordered or differently aligned observations.
    if a.sample_rate != b.sample_rate
        || a.windows.len() != b.windows.len()
        || a.moments.len() != b.moments.len()
        || a.windows
            .iter()
            .zip(&b.windows)
            .any(|(a, b)| a.start != b.start || a.end != b.end || a.raw_rms_dbfs != b.raw_rms_dbfs)
        || a.moments
            .iter()
            .zip(&b.moments)
            .any(|(a, b)| a.start != b.start || a.end != b.end || a.raw_power != b.raw_power)
        || sources != source_identities(&reference, root)?
    {
        return Err("paired observations or source identities changed".into());
    }
    let threshold = envelope(&a, &r.training, None).activity_threshold_dbfs;
    let report = ComparisonReport {
        schema_version: 1, analysis: AnalysisSettings::default(),
        reference_id: identity(&reference)?, changed_id: identity(&changed)?,
        source_files: sources, reference, changed,
        training: paired(&a, &b, &r.training, threshold),
        passages: r.training.iter().map(|s| ("training", *s))
            .chain(r.held_out.iter().map(|s| ("held_out", *s)))
            .map(|(split, span)| observe(&a, &b, span, split, threshold)).collect(),
        request, activity_threshold_dbfs: threshold,
        limitations: vec![
            "Identical recordings and routing only; independent performances cannot use this evidence.".into(),
            "Differences are changed minus reference. Spectra use the pre-compressor EQ tap; levels/events/returns observe production DSP before export.".into(),
            "Paired spread is descriptive scaled MAD, not reference uncertainty, a target, a fit tolerance or musical acceptance.".into(),
            "Common offsets are removed only in spectral analysis. No gain control or source sample is normalized.".into(),
            "Training sets activity only. Held-out passages do not choose filters, masks for training or tolerances.".into(),
            "Export gain is unmeasured: it requires complete independently peak-finalized renders. Null observations indicate missing evidence.".into(),
        ],
    };
    std::fs::create_dir(out)?;
    write_json(&out.join("reference-measurement.json"), &a)?;
    write_json(&out.join("changed-measurement.json"), &b)?;
    write_json(&out.join("comparison.json"), &report)?;
    super::review_comparison(&out.join("comparison.json"), &out.join("COMPARISON.md"))?;
    Ok(report)
}

pub(super) fn validated_pair(
    reference: &Session,
    changed: &Session,
    request: &ComparisonRequest,
) -> Result<Request> {
    let r = request.request();
    r.validate(reference)?;
    r.validate(changed)?;
    let mut restored = changed.clone();
    for input in &request.group.inputs {
        restored.channels[input.channel].eq = reference.channels[input.channel].eq.clone();
    }
    if identity(&restored)? != identity(reference)? {
        return Err("paired comparison permits only the named group's EQ to differ".into());
    }
    Ok(r)
}
