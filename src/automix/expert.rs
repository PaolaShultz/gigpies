//! Small offline source-rule coordinator. Labels have no processing semantics.
//! Plans use training audio only; one actual-DSP validation accepts or rejects the bundle.
use super::{
    config::{Role, Session},
    tone::{self, Frame, Instrument, Policy, Proposal},
    write_json,
};
use crate::inventory::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
const SIZE: f64 = 8192.;
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Family {
    ElectricGuitar,
    AcousticGuitar,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capture {
    RecordedTrack,
    AmplifierMicrophone,
    DirectInput,
    AcousticMicrophone,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DynamicsRule {
    pub max_mean_reduction_db: f64,
    pub max_p95_reduction_db: f64,
    pub max_threshold_raise_db: f64,
    pub max_output_rise_db: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SourceFirstRule {
    pub eq_change_for_recheck_db: f64,
    pub body_deviation_for_recheck_db: f64,
}
impl Default for SourceFirstRule {
    fn default() -> Self {
        Self {
            eq_change_for_recheck_db: 4.,
            body_deviation_for_recheck_db: 6.,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub name: String,
    pub family: Family,
    pub capture: Capture,
    /// None explicitly disables automatic tone changes; labels never choose these values.
    pub body_presence_db: Option<[f64; 2]>,
    pub dynamics: Option<DynamicsRule>,
    #[serde(default)]
    pub source_first: SourceFirstRule,
}
impl Profile {
    pub fn validate(&self, s: &Session, g: &Instrument) -> Result<()> {
        let finite = |x: f64, a: f64, b: f64| x.is_finite() && (a..=b).contains(&x);
        if self.name.trim().is_empty()
            || self.name.len() > 120
            || g.primary >= s.channels.len()
            || g.capture != Some(self.capture)
        {
            return Err("profile name or capture applicability mismatch".into());
        }
        let matches = match self.family {
            Family::ElectricGuitar => matches!(
                s.channels[g.primary].role,
                Role::RhythmGuitar | Role::LeadGuitar
            ),
            Family::AcousticGuitar => matches!(s.channels[g.primary].role, Role::AcousticGuitar),
        };
        if !matches {
            return Err("profile instrument family mismatch".into());
        }
        if let Some([lo, hi]) = self.body_presence_db
            && (!finite(lo, -24., 18.) || !finite(hi, -18., 24.) || hi - lo < 2. || hi - lo > 30.)
        {
            return Err("invalid profile body/presence range".into());
        }
        if !finite(self.source_first.eq_change_for_recheck_db, 2., 6.)
            || !finite(self.source_first.body_deviation_for_recheck_db, 3., 18.)
        {
            return Err("invalid source-first review limits".into());
        }
        if let Some(d) = &self.dynamics
            && (!finite(d.max_mean_reduction_db, 0., 12.)
                || !finite(d.max_p95_reduction_db, d.max_mean_reduction_db, 18.)
                || !finite(d.max_threshold_raise_db, 0., 12.)
                || !finite(d.max_output_rise_db, 0., 6.))
        {
            return Err("invalid dynamics rule".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum State {
    Disabled,
    NotApplicable,
    InsufficientEvidence,
    WithinTarget,
    Deviation,
    Blocked,
}
#[derive(Serialize)]
pub struct Finding {
    pub rule: &'static str,
    pub state: State,
    pub training_windows: usize,
    /// Empirical agreement fraction, not a calibrated probability.
    pub consistency: Option<f64>,
    pub excess_db: Option<f64>,
    pub reason: String,
}
#[derive(Clone, Serialize)]
pub struct DynamicsSummary {
    pub windows: usize,
    pub median_mean_reduction_db: f64,
    pub p95_max_reduction_db: f64,
    pub median_output_dbfs: f64,
    pub p10_crest_db: f64,
    pub median_crest_db: f64,
}
#[derive(Serialize)]
pub struct DynamicsPlan {
    pub finding: Finding,
    pub threshold_before_db: f64,
    pub threshold_proposed_db: f64,
    pub before: [DynamicsSummary; 2],
    #[serde(skip)]
    pub eligible: Vec<bool>,
}
fn summary(frames: &[Frame], mask: &[bool], p: &Policy, held: bool) -> DynamicsSummary {
    let rows = frames
        .iter()
        .zip(mask)
        .filter(|(f, m)| **m && tone::held(f, p) == held)
        .map(|(f, _)| f)
        .collect::<Vec<_>>();
    DynamicsSummary {
        windows: rows.len(),
        median_mean_reduction_db: tone::percentile(rows.iter().map(|f| f.mean_reduction_db), 0.5),
        p95_max_reduction_db: tone::percentile(rows.iter().map(|f| f.max_reduction_db), 0.95),
        median_output_dbfs: tone::percentile(rows.iter().map(|f| f.primary_dbfs), 0.5),
        p10_crest_db: tone::percentile(rows.iter().map(|f| f.crest_db), 0.1),
        median_crest_db: tone::percentile(rows.iter().map(|f| f.crest_db), 0.5),
    }
}
/// Activity for dynamics is independent of whether a note has a broad spectrum.
fn activity(frames: &[Frame], rate: u32, p: &Policy) -> Vec<bool> {
    let high = tone::percentile(
        frames
            .iter()
            .filter(|f| !tone::held(f, p) && tone::within_section(f, rate, p))
            .map(|f| f.raw_dbfs),
        0.95,
    );
    frames
        .iter()
        .map(|f| {
            tone::within_section(f, rate, p)
                && f.raw_dbfs > p.activity_floor_dbfs
                && f.raw_dbfs > high - p.relative_activity_db
                && f.raw_secondary_relative_db <= p.max_secondary_relative_db
        })
        .collect()
}
pub fn full_scale_finding(frames: &[Frame], rate: u32, p: &Policy) -> Finding {
    let mask = activity(frames, rate, p);
    let rows = frames
        .iter()
        .zip(&mask)
        .filter(|(f, m)| **m && !tone::held(f, p))
        .map(|(f, _)| f)
        .collect::<Vec<_>>();
    if rows.iter().any(|f| f.input_is_float) {
        return Finding{rule:"input_full_scale_contact",state:State::NotApplicable,training_windows:rows.len(),consistency:None,excess_db:None,reason:"Float input can exceed unity without clipping; this PCM contact detector does not classify it".into()};
    }
    let affected = rows
        .iter()
        .filter(|f| f.input_full_scale_fraction >= 0.001)
        .count();
    Finding{rule:"input_full_scale_contact",state:if affected>=3 {State::Blocked}else if rows.is_empty(){State::InsufficientEvidence}else{State::WithinTarget},training_windows:rows.len(),consistency:(!rows.is_empty()).then_some(affected as f64/rows.len().max(1) as f64),excess_db:None,reason:if affected>=3{"Repeated full-scale input contact; recoverability and cause unknown. No automatic EQ/compression repair; inspect input gain or recording path."}else{"No repeated full-scale contact above this detector's threshold; this does not prove an unclipped recording."}.into()}
}
pub fn propose_dynamics(s: &Session, g: &Instrument, frames: &[Frame], p: &Policy) -> DynamicsPlan {
    let mask = activity(frames, s.sample_rate, p);
    let before = [
        summary(frames, &mask, p, false),
        summary(frames, &mask, p, true),
    ];
    let old = s.channels[g.primary].compressor.threshold_db;
    let mut q = DynamicsPlan {
        finding: Finding {
            rule: "sustained_compression",
            state: State::Disabled,
            training_windows: before[0].windows,
            consistency: None,
            excess_db: None,
            reason: "Rule disabled by profile".into(),
        },
        threshold_before_db: old,
        threshold_proposed_db: old,
        before,
        eligible: mask,
    };
    let Some(d) = g.profile.as_ref().and_then(|p| p.dynamics.as_ref()) else {
        return q;
    };
    if q.before[0].windows < 3
        || q.before[0].windows as f64 * SIZE / (s.sample_rate as f64) < p.minimum_active_seconds
    {
        q.finding.state = State::InsufficientEvidence;
        q.finding.reason = "Too little eligible training activity".into();
        return q;
    }
    let fit = frames
        .iter()
        .zip(&q.eligible)
        .filter(|(f, m)| **m && !tone::held(f, p))
        .map(|(f, _)| f)
        .collect::<Vec<_>>();
    let agree = fit
        .iter()
        .filter(|f| {
            f.mean_reduction_db > d.max_mean_reduction_db
                || f.max_reduction_db > d.max_p95_reduction_db
        })
        .count() as f64
        / fit.len() as f64;
    let excess = (q.before[0].median_mean_reduction_db - d.max_mean_reduction_db)
        .max(q.before[0].p95_max_reduction_db - d.max_p95_reduction_db)
        .max(0.);
    q.finding.consistency = Some(agree);
    q.finding.excess_db = Some(excess);
    if excess < 0.5 {
        q.finding.state = State::WithinTarget;
        q.finding.reason = "Compressor action within profile limits".into();
        return q;
    }
    if agree < p.minimum_consistency {
        q.finding.state = State::InsufficientEvidence;
        q.finding.reason = "Excess is intermittent; sustained-action rule abstains".into();
        return q;
    }
    q.finding.state = State::Deviation;
    // Static slope is a proposal only. Bound predicted level rise without adding makeup.
    let slope = 1. - 1. / s.channels[g.primary].compressor.ratio;
    if slope <= 0. {
        q.finding.state = State::Blocked;
        q.finding.reason = "Measured action conflicts with bypassed compressor".into();
        return q;
    }
    let desired = ((excess / slope) * 2.).ceil() / 2.;
    let bound = d
        .max_threshold_raise_db
        .min(d.max_output_rise_db / slope)
        .min(-old);
    let raise = desired.min((bound * 2.).floor() / 2.).max(0.);
    q.threshold_proposed_db = old + raise;
    q.finding.reason=if raise>0.{"Threshold relief selected from measured sustained gain reduction; actual DSP and output rise must pass validation"}else{"Excess detected but the allowed correction/output budget is zero"}.into();
    q
}
fn excess(s: &DynamicsSummary, d: &DynamicsRule) -> f64 {
    (s.median_mean_reduction_db - d.max_mean_reduction_db).max(0.)
        + (s.p95_max_reduction_db - d.max_p95_reduction_db).max(0.)
}
#[derive(Serialize)]
pub struct Review {
    pub accepted: bool,
    pub reason: String,
    pub after: [DynamicsSummary; 2],
}
pub fn validate_dynamics(
    before: &[Frame],
    after: &[Frame],
    q: &DynamicsPlan,
    g: &Instrument,
    rate: u32,
    p: &Policy,
) -> Review {
    let mut r = Review {
        accepted: false,
        reason: "No threshold correction".into(),
        after: [
            summary(after, &q.eligible, p, false),
            summary(after, &q.eligible, p, true),
        ],
    };
    let threshold_changed = q.threshold_proposed_db != q.threshold_before_db;
    let Some(d) = g.profile.as_ref().and_then(|p| p.dynamics.as_ref()) else {
        return r;
    };
    if before.len() != after.len()
        || before
            .iter()
            .zip(after)
            .any(|(a, b)| a.seconds != b.seconds)
    {
        r.reason = "Timeline mismatch".into();
        return r;
    }
    for (old, new) in q.before.iter().zip(&r.after) {
        if new.windows < 3 || new.windows as f64 * SIZE / (rate as f64) < p.minimum_active_seconds {
            r.reason = "Insufficient held-out dynamics evidence".into();
            return r;
        }
        // Already-compliant held-out material need not acquire a new fault to pass.
        let allowed = if threshold_changed {
            excess(old, d) - 0.25_f64.min(excess(old, d))
        } else {
            excess(old, d) + 0.25
        };
        if excess(new, d) > allowed {
            r.reason = "Actual sustained reduction did not improve or remain compliant".into();
            return r;
        }
        if new.median_output_dbfs > old.median_output_dbfs + d.max_output_rise_db + 0.05 {
            r.reason = "Output rise exceeded profile budget".into();
            return r;
        }
        if new.p10_crest_db < old.p10_crest_db - p.max_crest_loss_db
            || new.median_crest_db < old.median_crest_db - p.max_crest_loss_db
        {
            r.reason = "Dynamics correction lost excessive crest".into();
            return r;
        }
    }
    // Reject concentrated side effects even if split medians hide them.
    let sections = before
        .last()
        .map(|f| (f.seconds / p.section_seconds) as usize + 1)
        .unwrap_or(0);
    for section in 0..sections {
        let ids = before
            .iter()
            .enumerate()
            .filter(|(i, f)| q.eligible[*i] && (f.seconds / p.section_seconds) as usize == section)
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        if ids.len() < 3 {
            continue;
        }
        let rise = tone::percentile(
            ids.iter()
                .map(|&i| after[i].primary_dbfs - before[i].primary_dbfs),
            0.5,
        );
        if rise > d.max_output_rise_db + 0.05 {
            r.reason = "Individual section exceeded output budget".into();
            return r;
        }
        for i in ids {
            if after[i].max_reduction_db > before[i].max_reduction_db + p.max_added_reduction_db {
                r.reason = "Local compressor action increased excessively".into();
                return r;
            }
        }
    }
    r.accepted = true;
    r.reason =
        "Actual compression improved or remained compliant within profile level and crest limits"
            .into();
    r
}
fn body_finding(q: &Proposal, enabled: bool) -> Finding {
    Finding {
        rule: "body_presence",
        state: if !enabled {
            State::Disabled
        } else if q.reason == "within chosen intent" {
            State::WithinTarget
        } else if !q.proposed_eq.is_empty() || q.reason == "no bounded improvement" {
            State::Deviation
        } else {
            State::InsufficientEvidence
        },
        training_windows: q.before[0].windows,
        consistency: enabled.then_some(q.consistency),
        excess_db: enabled.then_some(tone::violation(
            q.before[0].median_body_presence_db,
            q.range_db,
        )),
        reason: q.reason.clone(),
    }
}
#[derive(Serialize)]
pub struct SourceAdvice {
    pub state: &'static str,
    pub blocks_automatic_changes: bool,
    pub reason: String,
    pub requested_steps: Vec<String>,
    pub suspected_physical_cause: Option<String>,
    pub repeat_soundcheck: bool,
}
/// Advice follows evidence and known setup; no inference of mic position or amp controls.
pub fn source_advice(
    g: &Instrument,
    q: &Proposal,
    p: &Policy,
    input_contact: bool,
) -> SourceAdvice {
    let mut a = SourceAdvice {
        state: "no_source_action",
        blocks_automatic_changes: false,
        reason: "No confident source-level intervention identified by these rules".into(),
        requested_steps: vec![],
        suspected_physical_cause: None,
        repeat_soundcheck: false,
    };
    let Some(profile) = &g.profile else {
        return a;
    };
    if input_contact {
        a.state = "input_path_review";
        a.blocks_automatic_changes = true;
        a.reason="Repeated full-scale input contact; this pass cannot reconstruct missing waveform detail or establish the cause".into();
        a.requested_steps = if profile.capture == Capture::RecordedTrack {
            vec![
                "Check for an unclipped original or a new recording before further correction."
                    .into(),
            ]
        } else {
            vec!["Check gain and overload indicators along the instrument, input and recording path.".into(),"Reduce the gain at the stage that overloads, then repeat the same soundcheck phrase.".into()]
        };
        a.repeat_soundcheck = profile.capture != Capture::RecordedTrack;
        return a;
    }
    let excess = tone::violation(q.before[0].median_body_presence_db, q.range_db);
    let largest = q.proposed_eq.iter().map(|e| e.db.abs()).fold(0., f64::max);
    if profile.body_presence_db.is_none()
        || q.consistency < p.minimum_consistency
        || q.before[0].windows < 3
        || (excess < profile.source_first.body_deviation_for_recheck_db
            && largest < profile.source_first.eq_change_for_recheck_db)
    {
        return a;
    }
    a.reason = format!(
        "Confident training evidence is {excess:.2} dB outside the requested body range; largest proposed EQ change is {largest:.1} dB. Review the capture before a large corrective curve."
    );
    if profile.capture == Capture::RecordedTrack {
        a.state = "recorded_source_limitation";
        a.requested_steps=vec!["If available, compare another original capture. Keep any accepted software correction bounded and record the remaining mismatch.".into()];
        return a;
    }
    a.state = "repeat_soundcheck_after_source_adjustment";
    a.blocks_automatic_changes = true;
    a.repeat_soundcheck = true;
    let lean = q.before[0].median_body_presence_db < q.range_db[0];
    a.requested_steps=match profile.capture {
        Capture::AmplifierMicrophone=>vec![
            "Compare the amp heard at the player's position with the primary microphone signal.".into(),
            if lean {"If the amp itself is lean, add a little bass or low-mid on the amp. If the amp already sounds right, try a small mic-position change first.".into()}else{"If the amp itself is too bass-heavy for the intended tone, reduce a little bass or low-mid. If it sounds right in the room, check mic position/distance first.".into()},
            "Change one thing, then play the same quiet, normal and strong soundcheck phrases again.".into()],
        Capture::DirectInput=>vec!["Check the instrument pickup/tone controls and DI/input routing against the intended sound.".into(),"Make one small source adjustment, then repeat the soundcheck phrases.".into()],
        Capture::AcousticMicrophone=>vec!["Compare the instrument at the player's position with the primary microphone signal.".into(),"Try a small microphone position/distance change, then repeat the soundcheck phrases.".into()],
        Capture::RecordedTrack=>unreachable!(),
    };
    a
}
/// Apply one combined training proposal; no retry based on held-out data.
pub fn run(s: Session, root: &Path, out: &Path, p: Policy, render: bool) -> Result<()> {
    p.validate(&s)?;
    if p.instruments.iter().any(|g| g.profile.is_none()) {
        return Err(
            "source review requires explicit numerical profiles; display names do not select rules"
                .into(),
        );
    }
    std::fs::create_dir(out)?;
    write_json(&out.join("policy.json"), &p)?;
    write_json(&out.join("before-settings.json"), &s)?;
    let mut accepted = s.clone();
    let mut reports = vec![];
    for (n, g) in p.instruments.iter().enumerate() {
        let before = tone::measure(&s, root, g)?;
        let full_scale = full_scale_finding(&before, s.sample_rate, &p);
        let body = tone::propose(&before, s.sample_rate, g, &p);
        let dynamics = propose_dynamics(&s, g, &before, &p);
        let profile = g.profile.as_ref().unwrap();
        let mut candidate = s.clone();
        let eq_capacity_blocked = s.channels[g.primary].eq.len() + body.proposed_eq.len() > 8;
        if !eq_capacity_blocked {
            candidate.channels[g.primary]
                .eq
                .extend(body.proposed_eq.clone());
        }
        candidate.channels[g.primary].compressor.threshold_db = dynamics.threshold_proposed_db;
        candidate.validate()?;
        let body_changed = !body.proposed_eq.is_empty();
        let dynamics_changed = dynamics.threshold_before_db != dynamics.threshold_proposed_db;
        let proposed = body_changed || dynamics_changed;
        let held_contact = before
            .iter()
            .filter(|f| {
                tone::held(f, &p)
                    && tone::within_section(f, s.sample_rate, &p)
                    && !f.input_is_float
                    && f.input_full_scale_fraction >= 0.001
            })
            .count()
            >= 3;
        let input_blocked = full_scale.state == State::Blocked || held_contact;
        let advice = source_advice(g, &body, &p, input_blocked);
        let blocked = input_blocked || advice.blocks_automatic_changes || eq_capacity_blocked;
        let after = if proposed && !blocked {
            tone::measure(&candidate, root, g)?
        } else {
            before.clone()
        };
        let body_check = if blocked {
            None
        } else if body_changed {
            Some(tone::validate_candidate(
                &before,
                &after,
                &body,
                s.sample_rate,
                &p,
            ))
        } else if dynamics_changed && profile.body_presence_db.is_some() {
            Some(tone::validate_changes(
                &before,
                &after,
                &body,
                s.sample_rate,
                &p,
                false,
            ))
        } else {
            None
        };
        let dynamics_check = if !blocked && proposed && profile.dynamics.is_some() {
            Some(validate_dynamics(
                &before,
                &after,
                &dynamics,
                g,
                s.sample_rate,
                &p,
            ))
        } else {
            None
        };
        let ok = proposed
            && !blocked
            && body_check.as_ref().is_none_or(|r| r.accepted)
            && dynamics_check.as_ref().is_none_or(|r| r.accepted);
        if ok {
            accepted.channels[g.primary] = candidate.channels[g.primary].clone();
        }
        write_json(&out.join(format!("instrument-{n}-before.json")), &before)?;
        write_json(&out.join(format!("instrument-{n}-candidate.json")), &after)?;
        write_json(
            &out.join(format!("instrument-{n}-body-mask.json")),
            &body.eligible,
        )?;
        write_json(
            &out.join(format!("instrument-{n}-dynamics-mask.json")),
            &dynamics.eligible,
        )?;
        let applied_frames = if ok { &after } else { &before };
        let applied_body = [
            tone::summary(applied_frames, &body.eligible, &p, false),
            tone::summary(applied_frames, &body.eligible, &p, true),
        ];
        let applied_dynamics = [
            summary(applied_frames, &dynamics.eligible, &p, false),
            summary(applied_frames, &dynamics.eligible, &p, true),
        ];
        let body_target_met = profile.body_presence_db.and_then(|range| {
            applied_body
                .iter()
                .all(|x| {
                    x.windows >= 3
                        && x.windows as f64 * SIZE / (s.sample_rate as f64)
                            >= p.minimum_active_seconds
                })
                .then(|| {
                    applied_body
                        .iter()
                        .all(|x| tone::violation(x.median_body_presence_db, range) < 1e-6)
                })
        });
        let dynamics_target_met = profile.dynamics.as_ref().and_then(|rule| {
            applied_dynamics
                .iter()
                .all(|x| {
                    x.windows >= 3
                        && x.windows as f64 * SIZE / (s.sample_rate as f64)
                            >= p.minimum_active_seconds
                })
                .then(|| applied_dynamics.iter().all(|x| excess(x, rule) < 1e-6))
        });
        reports.push(serde_json::json!({"identity":g,"profile":profile,"findings":[body_finding(&body,profile.body_presence_db.is_some()),dynamics.finding,full_scale],"body_plan":body,"dynamics_plan":dynamics,"body_validation":body_check,"dynamics_validation":dynamics_check,"accepted":ok,"applied_metrics":{"body":applied_body,"dynamics":applied_dynamics},"body_target_met":body_target_met,"dynamics_target_met":dynamics_target_met,"proposal_changes_settings":proposed,"settings_changed":ok,"blocked_by_input_contact":input_blocked,"eq_capacity_blocked":eq_capacity_blocked,"source_advice":advice,"held_out_full_scale_contact":held_contact,"candidate_measured":proposed&&!blocked,"original_channel":s.channels[g.primary],"candidate_channel":candidate.channels[g.primary],"applied_channel":accepted.channels[g.primary],"listener_preference":null,"tradeoffs":"Threshold relief can raise channel level; makeup and faders stay fixed. EQ can change masking. These rules do not reconstruct clipped input or establish full-band preference."}));
    }
    accepted.validate()?;
    write_json(&out.join("decisions.json"), &reports)?;
    write_json(&out.join("settings.json"), &accepted)?;
    if render {
        super::run(accepted, root, &out.join("final"), None)?;
    }
    Ok(())
}
