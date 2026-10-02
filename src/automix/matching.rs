//! Offline artistic matching. A map supplies intent, never a diagnosis of bad sound.
//! The saved baseline owns every control except this explicitly appended EQ delta.
use super::{
    config::{EqBand, EqKind, Role, Session},
    dsp::{Biquad, db, gain},
    write_json,
};
use crate::inventory::Result;
use serde::{Deserialize, Serialize};
use std::{fs::File, path::Path};
mod check;
mod fit;
mod measure;
mod model;
mod review;
pub use check::*;
pub use fit::*;
pub use measure::*;
use model::valid;
pub use model::*;

pub fn validate_baseline(s: &Session) -> Result<()> {
    super::balance::validate_baseline(s)
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Choice {
    pub map: ToneMap,
    pub proposal: Proposal,
    pub checks: Vec<PassageCheck>,
    pub eligible: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    pub frozen_id: String,
    pub map_id: String,
    pub map_version: u32,
    pub amount_percent: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Attempt {
    pub selection: Selection,
    pub checks: Vec<PassageCheck>,
    pub applied: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct State {
    pub schema_version: u32,
    pub algorithm_version: u32,
    pub baseline: Session,
    pub baseline_id: String,
    pub source_files: Vec<SourceIdentity>,
    pub request: Request,
    pub choices: Vec<Choice>,
    pub frozen_id: String,
    pub selected_map: String,
    pub amount_percent: f64,
    pub last_attempt: Option<Attempt>,
}
impl State {
    fn fingerprint(&self) -> Result<String> {
        identity(&(
            &self.baseline_id,
            &self.source_files,
            &self.request,
            &self.choices,
        ))
    }
    pub fn validate(&self) -> Result<()> {
        self.request.validate(&self.baseline)?;
        if self.schema_version != 1
            || self.algorithm_version != 1
            || self.baseline_id != identity(&self.baseline)?
            || self.frozen_id != self.fingerprint()?
            || !valid(self.amount_percent, 0., 100.)
            || self.choices.is_empty()
            || self.choices.len() > 8
            || self.source_files.len() != self.baseline.channels.len()
        {
            return Err("invalid or stale frozen matching state".into());
        }
        let mut ids = std::collections::BTreeSet::new();
        for c in &self.choices {
            c.map.validate()?;
            if !ids.insert(&c.map.id)
                || c.proposal.supported.len() != BANDS
                || c.proposal.target_db.len() != BANDS
                || c.proposal.tolerance_db.len() != BANDS
                || c.proposal.boost_supported.len() != BANDS
                || c.eligible && c.checks.iter().any(|x| !x.passed)
                || !c.proposal.bands.is_empty()
                    && c.checks.len() != self.request.training.len() + self.request.held_out.len()
                || !fit::bounded(
                    &c.proposal.bands,
                    self.baseline.sample_rate,
                    &c.proposal.supported,
                    &c.proposal.boost_supported,
                )
            {
                return Err("invalid matching choice".into());
            }
            apply(&self.baseline, &self.request.group, &c.proposal.bands, 100.)?;
        }
        let selected = self.choice(&self.selected_map)?;
        if self.amount_percent > 0. && !selected.eligible {
            return Err("selected proposal did not pass validation".into());
        }
        if self.amount_percent > 0. {
            let attempt = self
                .last_attempt
                .as_ref()
                .ok_or("selected amount requires an actual validation record")?;
            if !attempt.applied
                || attempt.selection.frozen_id != self.frozen_id
                || attempt.selection.map_id != self.selected_map
                || attempt.selection.map_version != selected.map.version
                || attempt.selection.amount_percent != self.amount_percent
                || attempt.checks.iter().any(|c| !c.passed)
                || (!selected.proposal.bands.is_empty()
                    && attempt.checks.len()
                        != self.request.training.len() + self.request.held_out.len())
            {
                return Err("selected amount does not match its validation record".into());
            }
        }
        Ok(())
    }
    pub fn choice(&self, id: &str) -> Result<&Choice> {
        self.choices
            .iter()
            .find(|c| c.map.id == id)
            .ok_or_else(|| "unknown tone map selection".into())
    }
    pub fn settings(&self) -> Result<Session> {
        self.validate()?;
        apply(
            &self.baseline,
            &self.request.group,
            &self.choice(&self.selected_map)?.proposal.bands,
            self.amount_percent,
        )
    }
    pub fn reset(&mut self) {
        self.amount_percent = 0.;
        self.last_attempt = None;
    }
}
pub fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    if path.metadata()?.len() > 16 * 1024 * 1024 {
        return Err("matching JSON exceeds 16 MiB limit".into());
    }
    Ok(serde_json::from_reader(std::io::BufReader::new(
        File::open(path)?,
    ))?)
}
pub fn load_map(id_or_path: &str) -> Result<ToneMap> {
    let map = match builtin_maps().into_iter().find(|m| m.id == id_or_path) {
        Some(m) => m,
        None => read_json(Path::new(id_or_path))?,
    };
    map.validate()?;
    Ok(map)
}
fn spans(r: &Request) -> Vec<[f64; 2]> {
    let mut spans = r.training.clone();
    spans.extend(&r.held_out);
    spans
}
fn save(out: &Path, state: &State) -> Result<()> {
    let settings = state.settings()?;
    std::fs::create_dir(out)?;
    write_json(&out.join("state.json"), state)?;
    review::write(&out.join("review.html"), state)?;
    // Settings is the last ready artifact; a failed write leaves no overwritten selection.
    write_json(&out.join("settings.json"), &settings)
}
pub fn plan(s: Session, root: &Path, out: &Path, r: Request) -> Result<State> {
    r.validate(&s)?;
    if out.exists() {
        return Err("matching output must be a new directory".into());
    }
    let maps = r
        .maps
        .iter()
        .map(|m| load_map(m))
        .collect::<Result<Vec<_>>>()?;
    let mut ids = std::collections::BTreeSet::new();
    for m in &maps {
        if !ids.insert(&m.id)
            || m.kind != MapKind::Preserve && !m.context.compatible(&r.group.context)
        {
            return Err("duplicate map id or incompatible instrument/capture/register".into());
        }
        if m.kind == MapKind::MeasuredEnvelope
            && (m.context.tuning != r.group.context.tuning
                || m.context.technique != r.group.context.technique)
        {
            return Err(
                "measured map tuning/technique differs: import a comparable phrase/context instead"
                    .into(),
            );
        }
    }
    let sources = source_identities(&s, root)?;
    let before = measure(&s, root, &r.group, &spans(&r))?;
    let training = envelope(&before, &r.training, None);
    let capacity = r
        .group
        .inputs
        .iter()
        .map(|i| 8 - s.channels[i.channel].eq.len())
        .min()
        .unwrap_or(0);
    let mut choices = Vec::new();
    for map in maps {
        let mut proposal = fit(&training, &map, s.sample_rate, capacity)?;
        let validation = if proposal.bands.is_empty() {
            Vec::new()
        } else {
            let candidate = apply(&s, &r.group, &proposal.bands, 100.)?;
            let after = measure(&candidate, root, &r.group, &spans(&r))?;
            checks(&before, &after, &r, &map, &proposal, 100.)
        };
        let eligible = validation.iter().all(|c| c.passed);
        if !eligible {
            proposal.reason = "Actual DSP/held-out veto; baseline retained, no retry".into();
        }
        choices.push(Choice {
            map,
            proposal,
            checks: validation,
            eligible,
        });
    }
    if sources != source_identities(&s, root)? {
        return Err("source files changed during matching".into());
    }
    let mut state = State {
        schema_version: 1,
        algorithm_version: 1,
        baseline_id: identity(&s)?,
        baseline: s,
        source_files: sources,
        selected_map: choices[0].map.id.clone(),
        choices,
        request: r,
        frozen_id: String::new(),
        amount_percent: 0.,
        last_attempt: None,
    };
    state.frozen_id = state.fingerprint()?;
    save(out, &state)?;
    write_json(&out.join("baseline-measurement.json"), &before)?;
    Ok(state)
}
/// Validate nonlinear interactions at new amounts; reuse frozen checks only after
/// verifying the same state and recordings. Never refit or recalibrate.
pub fn select(mut state: State, root: &Path, out: &Path, selection: Selection) -> Result<State> {
    state.validate()?;
    if out.exists() {
        return Err("matching output must be new".into());
    }
    let choice = state.choice(&selection.map_id)?;
    if selection.frozen_id != state.frozen_id
        || selection.map_version != choice.map.version
        || !valid(selection.amount_percent, 0., 100.)
    {
        return Err("stale or invalid review selection".into());
    }
    if !choice.eligible && selection.amount_percent > 0. {
        return Err("proposal was withheld by production checks; use 0% or reset".into());
    }
    if source_identities(&state.baseline, root)? != state.source_files {
        return Err("recording identity changed; prepare a new frozen review".into());
    }
    let validation = if selection.amount_percent == 0. || choice.proposal.bands.is_empty() {
        Vec::new()
    } else if selection.amount_percent == 100. {
        // The planner already ran this exact production chain in both splits.
        choice.checks.clone()
    } else if let Some(previous) = state.last_attempt.as_ref().filter(|a| {
        a.selection.frozen_id == selection.frozen_id
            && a.selection.map_id == selection.map_id
            && a.selection.map_version == selection.map_version
            && a.selection.amount_percent == selection.amount_percent
    }) {
        previous.checks.clone()
    } else {
        let settings = apply(
            &state.baseline,
            &state.request.group,
            &choice.proposal.bands,
            selection.amount_percent,
        )?;
        let before = measure(
            &state.baseline,
            root,
            &state.request.group,
            &spans(&state.request),
        )?;
        let after = measure(
            &settings,
            root,
            &state.request.group,
            &spans(&state.request),
        )?;
        checks(
            &before,
            &after,
            &state.request,
            &choice.map,
            &choice.proposal,
            selection.amount_percent,
        )
    };
    if source_identities(&state.baseline, root)? != state.source_files {
        return Err("source changed during review".into());
    }
    let applied = validation.iter().all(|c| c.passed);
    state.selected_map = selection.map_id.clone();
    state.amount_percent = if applied {
        selection.amount_percent
    } else {
        0.
    };
    state.last_attempt = Some(Attempt {
        selection,
        checks: validation,
        applied,
    });
    save(out, &state)?;
    Ok(state)
}
pub fn reset(mut state: State, out: &Path) -> Result<()> {
    state.validate()?;
    state.reset();
    save(out, &state)
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportSpec {
    pub id: String,
    pub version: u32,
    pub label: String,
    pub basis: String,
    pub group: InputGroup,
    pub passages: Vec<[f64; 2]>,
    pub sources: Vec<Reference>,
    pub supported_hz: [f64; 2],
    pub limitations: Vec<String>,
}
/// Import only the chosen local phrase envelope, never copy/distribute source audio.
pub fn import(s: Session, root: &Path, out: &Path, spec: ImportSpec) -> Result<ToneMap> {
    validate_baseline(&s)?;
    spec.group.validate(&s)?;
    validate_spans(&spec.passages)?;
    if out.exists() {
        return Err("map destination must be new".into());
    }
    let sources = source_identities(&s, root)?;
    let m = measure(&s, root, &spec.group, &spec.passages)?;
    let e = envelope(&m, &spec.passages, None);
    if e.phrase_variation_db < 0.35
        || e.active_seconds < 3.
        || e.supported.iter().filter(|v| **v).count() < 8
    {
        return Err("insufficient representative non-noise reference evidence".into());
    }
    let map = ToneMap {
        schema_version: 1,
        id: spec.id,
        version: spec.version,
        label: spec.label,
        kind: MapKind::MeasuredEnvelope,
        provisional: true,
        basis: spec.basis,
        context: spec.group.context.clone(),
        sources: spec.sources,
        analysis: AnalysisSettings::default(),
        supported_hz: spec.supported_hz,
        values_db: e
            .values_db
            .iter()
            .zip(&e.supported)
            .map(|(v, on)| if *on { *v } else { 0. })
            .collect(),
        uncertainty_db: e
            .uncertainty_db
            .iter()
            .map(|x| x.clamp(0.75, 24.))
            .collect(),
        supported: e.supported,
        limitations: spec.limitations,
        measurement: Some(ReferenceMeasurement {
            baseline_id: identity(&s)?,
            source_files: sources.clone(),
            group: spec.group,
            passages: spec.passages,
            sample_rate: s.sample_rate,
            active_seconds: e.active_seconds,
        }),
    };
    map.validate()?;
    if source_identities(&s, root)? != sources {
        return Err("reference source changed during import".into());
    }
    write_json(out, &map)?;
    Ok(map)
}
