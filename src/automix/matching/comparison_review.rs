//! A readable view of recorded paired diagnostics; no fitting or audio access.
use super::{
    AnalysisSettings, ComparisonReport, PairedSpectrum, Result, compare::validated_pair,
    frequencies, identity,
};
use crate::automix::identity::bytes_hash;
use std::{fmt::Write as _, fs::OpenOptions, io::Write, path::Path};

fn finite(values: impl IntoIterator<Item = Option<f64>>) -> bool {
    values.into_iter().flatten().all(f64::is_finite)
}

fn validate_spectrum(s: &PairedSpectrum, spans: &[[f64; 2]], rate: u32) -> Result<()> {
    let duration = spans.iter().map(|s| s[1] - s[0]).sum::<f64>();
    let expected_seconds = s.eligible_windows as f64 * super::FFT_SIZE as f64 / rate as f64;
    // The serialized start/end times are floating point. Allow one sample for
    // accumulated timestamp rounding; this does not alter any fitting threshold.
    let time_epsilon = 1. / rate as f64;
    if s.spans != spans
        || !s.active_seconds.is_finite()
        || s.active_seconds < 0.
        || s.active_seconds > duration + time_epsilon
        || (s.active_seconds - expected_seconds).abs() > time_epsilon
        || !finite([s.common_offset_db])
        || s.common_offset_db.is_some() != (s.eligible_windows > 0)
        || s.bands.len() != super::BANDS
    {
        return Err("inconsistent saved spectral passage or support".into());
    }
    for (b, hz) in s.bands.iter().zip(frequencies()) {
        let values = [
            b.shape_change_db,
            b.paired_spread_db,
            b.reference_phrase_spread_db,
            b.changed_phrase_spread_db,
        ];
        if b.hz != hz
            || b.paired_windows > s.eligible_windows
            || !finite(values)
            || values.iter().any(|v| v.is_some() != b.supported)
            || [
                b.paired_spread_db,
                b.reference_phrase_spread_db,
                b.changed_phrase_spread_db,
            ]
            .into_iter()
            .flatten()
            .any(|v| v < 0.)
            || (b.supported
                && (!(125. ..=6300.).contains(&hz)
                    || b.paired_windows < 8
                    || (b.paired_windows as f64) < s.eligible_windows as f64 * 0.7))
        {
            return Err("inconsistent saved paired-band evidence".into());
        }
    }
    if s.representative_phrase
        && (s.active_seconds < 3. || s.bands.iter().filter(|b| b.supported).count() < 8)
    {
        return Err("saved representative-phrase flag lacks its recorded support".into());
    }
    Ok(())
}

impl ComparisonReport {
    /// Validate the saved settings, scope and report structure. This cannot
    /// authenticate measurements or check current source bytes without audio I/O.
    pub fn validate_saved(&self) -> Result<()> {
        validated_pair(&self.reference, &self.changed, &self.request)?;
        if self.schema_version != 1
            || self.analysis != AnalysisSettings::default()
            || self.reference_id != identity(&self.reference)?
            || self.changed_id != identity(&self.changed)?
            || !self.activity_threshold_dbfs.is_finite()
            || self.source_files.len() != self.reference.channels.len()
            || self
                .source_files
                .iter()
                .zip(&self.reference.channels)
                .any(|(source, channel)| {
                    source.file != channel.file
                        || source.bytes == 0
                        || source.sha256.len() != 64
                        || !source.sha256.bytes().all(|c| c.is_ascii_hexdigit())
                })
            || self.passages.len() != self.request.training.len() + self.request.held_out.len()
        {
            return Err("unsupported or inconsistent saved EQ comparison".into());
        }
        let rate = self.reference.sample_rate;
        validate_spectrum(&self.training, &self.request.training, rate)?;
        let expected = self
            .request
            .training
            .iter()
            .map(|s| ("training", s))
            .chain(self.request.held_out.iter().map(|s| ("held_out", s)));
        let returns = self
            .reference
            .effects
            .as_ref()
            .map_or(0, |fx| fx.buses.len());
        for (p, (split, span)) in self.passages.iter().zip(expected) {
            validate_spectrum(&p.spectrum, &[*span], rate)?;
            let levels = [
                p.group_rms_change_db,
                p.body_presence_change_db,
                p.mix_rms_change_db,
                p.mix_low_change_db,
                p.max_group_crest_loss_db,
                p.max_added_compression_db,
                p.max_stereo_correlation_change,
                p.max_added_master_reduction_db,
            ];
            if p.split != split
                || p.return_change_db.len() != returns
                || !finite(levels)
                || !finite(p.return_change_db.iter().copied())
                || !finite([p.attack_change_db, p.body_change_db, p.sustain_change_db])
                || levels
                    .iter()
                    .any(|v| v.is_some() != (p.active_level_windows > 0))
                || p.active_level_windows < p.spectrum.eligible_windows
                || (p.active_level_windows == 0 && p.return_change_db.iter().any(Option::is_some))
            {
                return Err("inconsistent saved production-DSP observations".into());
            }
        }
        Ok(())
    }

    fn markdown(&self) -> String {
        let group = &self.request.group;
        let instrument = match group.context.instrument {
            super::Instrument::ElectricGuitar => "electric guitar",
            super::Instrument::AcousticGuitar => "acoustic guitar",
            super::Instrument::BassDi => "bass DI",
            super::Instrument::Voice => "voice",
        };
        let capture = match group.context.capture {
            super::Capture::RecordedTrack => "recorded track; capture identity not inferred",
            super::Capture::AmplifierMicrophone => "declared amplifier microphone",
            super::Capture::AcousticMicrophone => "declared acoustic microphone",
            super::Capture::DirectInput => "declared direct input",
        };
        let register = group
            .context
            .register_hz
            .map(|r| format!("{:.1}–{:.1} Hz", r[0], r[1]))
            .unwrap_or_else(|| "unknown".into());
        let context = format!(
            "{instrument}; {capture}; tuning: {}; technique: {}; declared register: {register}",
            group.context.tuning, group.context.technique
        );
        let history = if crate::automix::training_precedes_held_out(
            &self.request.training,
            &self.request.held_out,
        ) {
            ""
        } else {
            "The recorded passages are **interleaved**. Continuous DSP can carry earlier held-out audio into later training observations. This diagnostic does not establish independent validation.\n\n"
        };
        let mut text = format!(
            "# Paired EQ comparison\n\nGroup: **{}**. Level and shape changes are **changed minus reference**.\n\nThis is a diagnostic of the same recordings. Technical eligibility, fitting targets, export gain and listener acceptance are **not determined** by this report.\n\n{}## Recorded identity and scope\n\n- Reference settings: `{}`\n- Changed settings: `{}`\n- Recorded source hashes: {} files. Current recordings are not checked when formatting this summary.\n- Recorded context: {}\n- Input identity basis: {}\n- Routing basis: {}\n- Representative-phrase description: {}\n- Reference-training activity threshold: {:.3} dBFS.\n\nOnly the named group's EQ differs in the recorded settings. Trims, faders, compressor controls, master controls, FX controls and timing retain their values; the observed compressor and FX signals can still change.\n\n| Settings channel index (0-based) | Recorded file | Reference EQ | Changed EQ |\n|---:|---|---|---|\n",
            cell(&group.name),
            history,
            self.reference_id,
            self.changed_id,
            self.source_files.len(),
            cell(&context),
            cell(&group.identity_basis),
            cell(&self.request.routing_basis),
            cell(&group.representative_phrases),
            self.activity_threshold_dbfs
        );
        for input in &group.inputs {
            let eq = |bands: &[super::EqBand]| {
                if bands.is_empty() {
                    return "None".into();
                }
                bands
                    .iter()
                    .map(|b| format!("{:?} {:.1} Hz, {:+.3} dB, Q {:.3}", b.kind, b.hz, b.db, b.q))
                    .collect::<Vec<_>>()
                    .join("; ")
            };
            let _ = writeln!(
                text,
                "| {} | {} | {} | {} |",
                input.channel,
                cell(&input.file.to_string_lossy()),
                eq(&self.reference.channels[input.channel].eq),
                eq(&self.changed.channels[input.channel].eq)
            );
        }
        if !self.request.excluded_fx_returns.is_empty() {
            let _ = writeln!(
                text,
                "\nDeclared supplied FX-return exclusions: {}.\n",
                self.request
                    .excluded_fx_returns
                    .iter()
                    .map(|p| cell(&p.to_string_lossy()))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
        text.push_str("\n## Spectral evidence\n\nSpectra observe the EQ output before compression. The common offset is removed only for shape analysis; no audio gain is normalized. The representative-phrase flag describes recorded measurement support, not preferred sound.\n\n| Split / passages s | Paired windows | Active s | Supported bands | Common offset dB | Representative phrase |\n|---|---:|---:|---:|---:|---|\n");
        for (split, s) in std::iter::once(("training pooled", &self.training)).chain(
            self.passages
                .iter()
                .map(|p| (p.split.as_str(), &p.spectrum)),
        ) {
            let _ = writeln!(
                text,
                "| {} / {} | {} | {:.3} | {} | {} | {} |",
                split,
                spans(&s.spans),
                s.eligible_windows,
                s.active_seconds,
                s.bands.iter().filter(|b| b.supported).count(),
                number(s.common_offset_db),
                if s.representative_phrase { "yes" } else { "no" }
            );
        }
        text.push_str("\n### Pooled training shape\n\nSpread is scaled median absolute deviation on the paired windows. It is descriptive, not a confidence interval or a fitting tolerance. Frequencies outside the diagnostic's 125–6300 Hz range are omitted here. Full per-passage spectra remain in the JSON report.\n\n| Hz | Paired support | Shape change dB | Paired spread dB | Reference phrase spread dB | Changed phrase spread dB |\n|---:|---:|---:|---:|---:|---:|\n");
        for b in self
            .training
            .bands
            .iter()
            .filter(|b| (125. ..=6300.).contains(&b.hz))
        {
            let _ = writeln!(
                text,
                "| {:.1} | {} | {} | {} | {} | {} |",
                b.hz,
                b.paired_windows,
                number(b.shape_change_db),
                number(b.paired_spread_db),
                number(b.reference_phrase_spread_db),
                number(b.changed_phrase_spread_db)
            );
        }
        text.push_str("\n## Production level changes\n\nThese use all reference-active level windows, including windows that lack spectral support. Values are median per-window changes before export. Body/presence is the group's 100–400 Hz to 800–3200 Hz power ratio; mix low is coherent ensemble power at 30–160 Hz.\n\n| Split / passage s | Level windows | Group RMS dB | Body/presence dB | Mix RMS dB | Mix low dB |\n|---|---:|---:|---:|---:|---:|\n");
        for p in &self.passages {
            let _ = writeln!(
                text,
                "| {} / {} | {} | {} | {} | {} | {} |",
                p.split,
                spans(&p.spectrum.spans),
                p.active_level_windows,
                number(p.group_rms_change_db),
                number(p.body_presence_change_db),
                number(p.mix_rms_change_db),
                number(p.mix_low_change_db)
            );
        }
        text.push_str("\n## Compressor, crest and stereo consequences\n\nAdded reduction and crest loss are nonnegative maxima across active windows. Zero does not mean the compressor was inactive: these fields report added action relative to the reference. Stereo change is the largest absolute correlation difference.\n\n| Split / passage s | Added compression dB | Added master reduction dB | Crest loss dB | Stereo correlation change |\n|---|---:|---:|---:|---:|\n");
        for p in &self.passages {
            let _ = writeln!(
                text,
                "| {} / {} | {} | {} | {} | {} |",
                p.split,
                spans(&p.spectrum.spans),
                number(p.max_added_compression_db),
                number(p.max_added_master_reduction_db),
                number(p.max_group_crest_loss_db),
                number(p.max_stereo_correlation_change)
            );
        }
        text.push_str("\n## Generated FX return changes\n\nMedian return-level change uses reference-active group windows where that reference return exceeds −90 dBFS. A missing observation is unmeasured. The FX settings themselves are unchanged.\n\n| Split / passage s | Bus | Return change dB |\n|---|---|---:|\n");
        if let Some(fx) = &self.reference.effects {
            for p in &self.passages {
                for (bus, delta) in fx.buses.iter().zip(&p.return_change_db) {
                    let _ = writeln!(
                        text,
                        "| {} / {} | {} | {} |",
                        p.split,
                        spans(&p.spectrum.spans),
                        cell(&bus.name),
                        number(*delta)
                    );
                }
            }
        }
        if self
            .reference
            .effects
            .as_ref()
            .is_none_or(|fx| fx.buses.is_empty())
        {
            text.push_str("\nNo generated FX returns are configured.\n");
        }
        text.push_str("\n## Raw-event stage proxies\n\nRaw energy rises select fixed attack (0–40 ms), body (40–120 ms) and sustain (120–300 ms) windows. The count records supported attack observations; body and sustain support can differ. These are energy proxies, not identified notes or musical acceptance.\n\n| Split / passage s | Attack observations | Attack dB | Body dB | Sustain dB |\n|---|---:|---:|---:|---:|\n");
        for p in &self.passages {
            let _ = writeln!(
                text,
                "| {} / {} | {} | {} | {} | {} |",
                p.split,
                spans(&p.spectrum.spans),
                p.event_count,
                number(p.attack_change_db),
                number(p.body_change_db),
                number(p.sustain_change_db)
            );
        }
        text.push_str("\n## Limits\n\nExport gain requires complete, independently peak-finalized renders. Musical acceptance requires listening feedback. Missing observations remain **unmeasured**, including silence; they are never displayed as zero change.\n\n");
        for limit in &self.limitations {
            let _ = writeln!(text, "- {}", cell(limit));
        }
        text
    }
}

fn cell(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\n' | '\r' => out.push(' '),
            '\\' | '|' | '`' | '*' | '_' | '[' | ']' | '(' | ')' | '!' => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out
}
fn number(value: Option<f64>) -> String {
    value
        .map(|v| format!("{v:+.3}"))
        .unwrap_or_else(|| "unmeasured".into())
}
fn spans(spans: &[[f64; 2]]) -> String {
    spans
        .iter()
        .map(|s| format!("{:.3}–{:.3}", s[0], s[1]))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Format saved evidence into a new text file. No recordings are opened and no
/// measurement, target, choice, setting, map or playback is produced.
pub fn review_comparison(input: &Path, output: &Path) -> Result<()> {
    if output.exists() {
        return Err("comparison review output already exists".into());
    }
    let bytes = std::fs::read(input)?;
    let report: ComparisonReport = serde_json::from_slice(&bytes)?;
    report.validate_saved()?;
    let mut text = report.markdown();
    let _ = writeln!(
        text,
        "\nRecorded comparison SHA-256: `{}`.\nThis summary validates saved settings and report structure; it does not authenticate measurement history or current source bytes.",
        bytes_hash(&bytes)
    );
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)?;
    file.write_all(text.as_bytes())?;
    Ok(())
}
