use super::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PassageCheck {
    pub split: String,
    pub span: [f64; 2],
    pub before_error_db: f64,
    pub after_error_db: f64,
    pub group_rms_change_db: f64,
    pub body_presence_change_db: f64,
    pub mix_rms_change_db: f64,
    pub mix_low_change_db: f64,
    pub max_group_crest_loss_db: f64,
    pub max_added_compression_db: f64,
    pub max_stereo_correlation_change: f64,
    pub return_change_db: Vec<f64>,
    pub max_added_master_reduction_db: f64,
    pub event_count: usize,
    pub attack_change_db: Option<f64>,
    pub body_change_db: Option<f64>,
    pub sustain_change_db: Option<f64>,
    pub passed: bool,
    pub reasons: Vec<String>,
}
fn event_changes(a: &Measurement, b: &Measurement, span: [f64; 2]) -> (usize, [Option<f64>; 3]) {
    let mut result = [Vec::new(), Vec::new(), Vec::new()];
    let mut last = -1.;
    for i in 1..a.moments.len() {
        let m = &a.moments[i];
        let previous = &a.moments[i - 1];
        if !inside(m.start, m.end, &[span])
            || m.start - last < 0.3
            || (m.start - previous.end).abs() > 0.001
            || db(m.raw_power.sqrt()) < -60.
            || 10. * (m.raw_power / previous.raw_power.max(1e-24)).log10() < 4.
        {
            continue;
        }
        last = m.start;
        for (stage, (lo, hi)) in [(0., 0.04), (0.04, 0.12), (0.12, 0.3)]
            .into_iter()
            .enumerate()
        {
            let pairs = a
                .moments
                .iter()
                .zip(&b.moments)
                .filter(|(x, _)| {
                    inside(x.start, x.end, &[[m.start + lo, m.start + hi]])
                        && inside(x.start, x.end, &[span])
                })
                .collect::<Vec<_>>();
            if pairs.is_empty() {
                continue;
            }
            let before = pairs.iter().map(|(x, _)| x.group_power).sum::<f64>();
            let after = pairs.iter().map(|(_, x)| x.group_power).sum::<f64>();
            if before > 1e-16 {
                result[stage].push(10. * (after / before).max(1e-24).log10());
            }
        }
    }
    (
        result[0].len(),
        result.map(|v| if v.is_empty() { None } else { Some(median(&v)) }),
    )
}
/// Checks observe the frozen proposal once; a failure never starts another fit.
pub fn checks(
    a: &Measurement,
    b: &Measurement,
    request: &Request,
    map: &ToneMap,
    p: &Proposal,
    amount: f64,
) -> Vec<PassageCheck> {
    let train = envelope(a, &request.training, None);
    request
        .training
        .iter()
        .map(|s| ("training", s))
        .chain(request.held_out.iter().map(|s| ("held_out", s)))
        .map(|(split, &span)| {
            let before = envelope(a, &[span], Some(train.activity_threshold_dbfs));
            let after = envelope_locked(b, a, &[span], train.activity_threshold_dbfs);
            let mask = p
                .supported
                .iter()
                .zip(&before.supported)
                .map(|(a, b)| *a && *b)
                .collect::<Vec<_>>();
            let target = if map.kind == MapKind::RelativeIntent {
                before
                    .values_db
                    .iter()
                    .zip(&map.values_db)
                    .map(|(a, d)| a + d * amount / 100.)
                    .collect::<Vec<_>>()
            } else {
                p.target_db.clone()
            };
            let be = error(&before.values_db, &target, &p.tolerance_db, &mask);
            let ae = error(&after.values_db, &target, &p.tolerance_db, &mask);
            let pairs = a
                .windows
                .iter()
                .zip(&b.windows)
                .filter(|(x, _)| {
                    inside(x.start, x.end, &[span])
                        && x.raw_rms_dbfs > train.activity_threshold_dbfs
                })
                .collect::<Vec<_>>();
            let med = |f: fn(&Window, &Window) -> f64| {
                median(&pairs.iter().map(|(a, b)| f(a, b)).collect::<Vec<_>>())
            };
            let max = |f: fn(&Window, &Window) -> f64| {
                pairs.iter().map(|(a, b)| f(a, b)).fold(0., f64::max)
            };
            let added_comp = max(|a, b| {
                a.compressor_max_db
                    .iter()
                    .zip(&b.compressor_max_db)
                    .map(|(a, b)| b - a)
                    .fold(0., f64::max)
            });
            let crest_loss = max(|a, b| {
                (a.group_peak_dbfs - a.group_rms_dbfs) - (b.group_peak_dbfs - b.group_rms_dbfs)
            });
            let stereo = max(|a, b| (a.stereo_correlation - b.stereo_correlation).abs());
            let master = max(|a, b| b.master_reduction_db - a.master_reduction_db);
            let group_change = med(|a, b| b.group_rms_dbfs - a.group_rms_dbfs);
            let mix_change = med(|a, b| b.mix_rms_dbfs - a.mix_rms_dbfs);
            let returns = (0..a.windows.first().map_or(0, |w| w.return_rms_dbfs.len()))
                .map(|i| {
                    median(
                        &pairs
                            .iter()
                            .filter(|(a, _)| a.return_rms_dbfs[i] > -90.)
                            .map(|(a, b)| b.return_rms_dbfs[i] - a.return_rms_dbfs[i])
                            .collect::<Vec<_>>(),
                    )
                })
                .collect::<Vec<_>>();
            let (events, stages) = event_changes(a, b, span);
            let mut reasons = Vec::new();
            if before.active_seconds < 1.5 || mask.iter().filter(|v| **v).count() < 8 {
                reasons.push("Insufficient passage evidence".into());
            }
            if ae > be + 0.15 || be > 0.5 && ae > be - 0.1 * amount / 100. {
                reasons.push("No useful measured spectral progress in this passage".into());
            }
            if max(|a, b| (b.group_rms_dbfs - a.group_rms_dbfs).abs()) > 3.05
                || crest_loss > 1.5
                || added_comp > 1.
            {
                reasons.push("Group level, crest or compressor guard failed".into());
            }
            if max(|a, b| (b.mix_rms_dbfs - a.mix_rms_dbfs).abs()) > 2.
                || max(|a, b| b.mix_peak_dbfs - a.mix_peak_dbfs) > 2.
                || stereo > 0.10
                || master > 0.1
            {
                reasons.push("Ensemble level, stereo or master guard failed".into());
            }
            if stages.iter().flatten().any(|v| v.abs() > 3.) {
                reasons.push("Fixed raw-event attack/body/sustain guard failed".into());
            }
            if pairs
                .iter()
                .filter(|(a, _)| a.pcm_contact_fraction >= 0.001)
                .count()
                >= 3
            {
                reasons.push("Repeated PCM contact; no reconstruction claim".into());
            }
            PassageCheck {
                split: split.into(),
                span,
                before_error_db: be,
                after_error_db: ae,
                group_rms_change_db: group_change,
                body_presence_change_db: med(|a, b| {
                    b.group_body_presence_db - a.group_body_presence_db
                }),
                mix_rms_change_db: mix_change,
                mix_low_change_db: med(|a, b| b.mix_low_power_dbfs - a.mix_low_power_dbfs),
                max_group_crest_loss_db: crest_loss,
                max_added_compression_db: added_comp,
                max_stereo_correlation_change: stereo,
                return_change_db: returns,
                max_added_master_reduction_db: master,
                event_count: events,
                attack_change_db: stages[0],
                body_change_db: stages[1],
                sustain_change_db: stages[2],
                passed: reasons.is_empty(),
                reasons,
            }
        })
        .collect()
}
