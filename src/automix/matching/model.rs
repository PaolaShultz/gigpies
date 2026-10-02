use super::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, io::Read, path::PathBuf};

pub const BANDS: usize = 28;
pub const FFT_SIZE: usize = 8192;
pub fn frequencies() -> Vec<f64> {
    (0..BANDS)
        .map(|i| 31.25 * 2_f64.powf(i as f64 / 3.))
        .collect()
}
pub fn identity(value: &impl Serialize) -> Result<String> {
    // serde_json Value orders keys and its round-trip representation is canonical here.
    let bytes = serde_json::to_vec(&serde_json::to_value(value)?)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SourceIdentity {
    pub file: PathBuf,
    pub bytes: u64,
    pub sha256: String,
}
pub fn source_identities(s: &Session, root: &Path) -> Result<Vec<SourceIdentity>> {
    s.channels
        .iter()
        .map(|c| {
            let mut file = std::fs::File::open(root.join(&c.file))?;
            let bytes = file.metadata()?.len();
            let mut hash = Sha256::new();
            let mut buf = [0u8; 65536];
            loop {
                let n = file.read(&mut buf)?;
                if n == 0 {
                    break;
                }
                hash.update(&buf[..n]);
            }
            Ok(SourceIdentity {
                file: c.file.clone(),
                bytes,
                sha256: format!("{:x}", hash.finalize()),
            })
        })
        .collect()
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Instrument {
    ElectricGuitar,
    AcousticGuitar,
    BassDi,
    Voice,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Capture {
    RecordedTrack,
    AmplifierMicrophone,
    AcousticMicrophone,
    DirectInput,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Context {
    pub instrument: Instrument,
    pub capture: Capture,
    /// Explicit description or "unknown"; never inferred from filenames.
    pub tuning: String,
    pub register_hz: Option<[f64; 2]>,
    pub technique: String,
}
impl Context {
    fn validate(&self) -> Result<()> {
        if self.tuning.trim().is_empty()
            || self.technique.trim().is_empty()
            || self
                .register_hz
                .is_some_and(|r| !valid(r[0], 20., 4000.) || !valid(r[1], r[0], 8000.))
        {
            return Err("invalid tuning/register/technique context".into());
        }
        Ok(())
    }
    pub fn compatible(&self, other: &Self) -> bool {
        self.instrument == other.instrument
            && self.capture == other.capture
            && match (self.register_hz, other.register_hz) {
                (Some(a), Some(b)) => a[1] >= b[0] && b[1] >= a[0],
                _ => true, // Unknown is recorded, never converted to a pitch estimate.
            }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Distribution {
    PrivateOnly,
    Redistributable,
    OriginalAuthored,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reference {
    pub citation: String,
    pub locator: String,
    pub license: String,
    pub distribution: Distribution,
    pub capture_notes: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AnalysisSettings {
    pub revision: u32,
    pub fft_size: usize,
    pub window: String,
    pub smoothing_octaves: f64,
    pub hop_frames: usize,
    pub aggregation: String,
}
impl Default for AnalysisSettings {
    fn default() -> Self {
        Self {
            revision: 1,
            fft_size: FFT_SIZE,
            window: "hann".into(),
            smoothing_octaves: 1.,
            hop_frames: FFT_SIZE,
            aggregation: "median_of_level_centered_log_psd".into(),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MapKind {
    Preserve,
    RelativeIntent,
    MeasuredEnvelope,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToneMap {
    pub schema_version: u32,
    pub id: String,
    pub version: u32,
    pub label: String,
    pub kind: MapKind,
    pub provisional: bool,
    pub basis: String,
    pub context: Context,
    pub sources: Vec<Reference>,
    pub analysis: AnalysisSettings,
    pub supported_hz: [f64; 2],
    /// Fixed third-octave grid; one-octave smoothed log PSD or authored shape delta.
    pub values_db: Vec<f64>,
    pub uncertainty_db: Vec<f64>,
    pub supported: Vec<bool>,
    pub limitations: Vec<String>,
    pub measurement: Option<ReferenceMeasurement>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceMeasurement {
    pub baseline_id: String,
    pub source_files: Vec<SourceIdentity>,
    pub group: InputGroup,
    pub passages: Vec<[f64; 2]>,
    pub sample_rate: u32,
    pub active_seconds: f64,
}
impl ToneMap {
    pub fn validate(&self) -> Result<()> {
        self.context.validate()?;
        if self.schema_version != 1
            || self.version == 0
            || !slug(&self.id)
            || self.label.trim().is_empty()
            || self.basis.trim().is_empty()
            || self.sources.is_empty()
            || self.limitations.is_empty()
            || self.analysis != AnalysisSettings::default()
            || !valid(self.supported_hz[0], 31.25, 8000.)
            || !valid(self.supported_hz[1], self.supported_hz[0] * 2., 16000.)
            || self.values_db.len() != BANDS
            || self.uncertainty_db.len() != BANDS
            || self.supported.len() != BANDS
            || self.values_db.iter().any(|x| !valid(*x, -120., 120.))
            || self.uncertainty_db.iter().any(|x| !valid(*x, 0.5, 24.))
            || self.sources.iter().any(|r| {
                [
                    r.citation.as_str(),
                    r.locator.as_str(),
                    r.license.as_str(),
                    r.capture_notes.as_str(),
                ]
                .iter()
                .any(|x| x.trim().is_empty())
            })
            || self.kind == MapKind::RelativeIntent
                && (!self.provisional || self.values_db.iter().any(|x| x.abs() > 6.))
            || self.kind == MapKind::Preserve && self.values_db.iter().any(|x| *x != 0.)
        {
            return Err(
                "invalid tone map, unsupported analysis revision or missing provenance".into(),
            );
        }
        if self.kind == MapKind::MeasuredEnvelope {
            let m = self
                .measurement
                .as_ref()
                .ok_or("measured map requires capture evidence")?;
            validate_spans(&m.passages)?;
            if m.active_seconds < 3.
                || !m.active_seconds.is_finite()
                || m.baseline_id.len() != 64
                || m.source_files.is_empty()
                || !(8000..=192000).contains(&m.sample_rate)
                || m.group.context != self.context
                || self.supported_hz[1] > m.sample_rate as f64 * 0.4
            {
                return Err("invalid measured reference evidence".into());
            }
        } else if self.measurement.is_some() {
            return Err("authored maps cannot claim recorded evidence".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    pub channel: usize,
    pub file: PathBuf,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputGroup {
    pub name: String,
    pub inputs: Vec<Input>,
    pub identity_basis: String,
    pub context: Context,
    pub representative_phrases: String,
}
impl InputGroup {
    pub fn validate(&self, s: &Session) -> Result<()> {
        self.context.validate()?;
        if self.inputs.is_empty()
            || self.inputs.len() > 16
            || self.name.trim().is_empty()
            || self.identity_basis.trim().is_empty()
            || self.representative_phrases.trim().is_empty()
        {
            return Err(
                "matching requires an explicit input group and representative phrase description"
                    .into(),
            );
        }
        let mut seen = BTreeSet::new();
        for i in &self.inputs {
            let c = s
                .channels
                .get(i.channel)
                .ok_or("invalid matching channel")?;
            let role_ok = match self.context.instrument {
                Instrument::ElectricGuitar => {
                    matches!(c.role, Role::RhythmGuitar | Role::LeadGuitar)
                }
                Instrument::AcousticGuitar => matches!(c.role, Role::AcousticGuitar),
                Instrument::BassDi => {
                    matches!(c.role, Role::BassDi) && self.context.capture == Capture::DirectInput
                }
                Instrument::Voice => matches!(c.role, Role::LeadVocal | Role::BackingVocal),
            };
            if !seen.insert(i.channel) || i.file != c.file || !role_ok {
                return Err("group file, instrument or verified DI identity mismatch".into());
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub group: InputGroup,
    pub training: Vec<[f64; 2]>,
    pub held_out: Vec<[f64; 2]>,
    /// Local maps or built-in ids. Selection does not imply listener acceptance.
    pub maps: Vec<String>,
    pub routing_basis: String,
    pub excluded_fx_returns: Vec<PathBuf>,
}
impl Request {
    pub fn validate(&self, s: &Session) -> Result<()> {
        super::validate_baseline(s)?;
        self.group.validate(s)?;
        validate_spans(&self.training)?;
        validate_spans(&self.held_out)?;
        let mut all = self.training.clone();
        all.extend(&self.held_out);
        validate_spans(&all)?;
        if self.maps.is_empty()
            || self.maps.len() > 8
            || self.routing_basis.trim().is_empty()
            || self
                .excluded_fx_returns
                .iter()
                .any(|f| s.channels.iter().any(|c| c.file == *f))
        {
            return Err("invalid map budget or supplied FX-return exclusion".into());
        }
        Ok(())
    }
}
pub fn validate_spans(spans: &[[f64; 2]]) -> Result<()> {
    if spans.is_empty() || spans.len() > 32 {
        return Err("need 1..32 explicit passages".into());
    }
    let mut sorted = spans.to_vec();
    sorted.sort_by(|a, b| a[0].total_cmp(&b[0]));
    if sorted
        .iter()
        .any(|s| !valid(s[0], 0., 86400.) || !valid(s[1], s[0] + 0.5, 86400.))
        || sorted.windows(2).any(|w| w[0][1] > w[1][0])
    {
        return Err("invalid or overlapping passages".into());
    }
    Ok(())
}
pub(super) fn valid(x: f64, lo: f64, hi: f64) -> bool {
    x.is_finite() && (lo..=hi).contains(&x)
}
fn slug(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 80
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
}

/// Original relative musical directions. These are explicitly not measured genre spectra.
pub fn builtin_maps() -> Vec<ToneMap> {
    [("keep-current", "Keep current tone", MapKind::Preserve, Instrument::ElectricGuitar),
     ("dark-metal-guitar", "Dark metal guitar — provisional direction", MapKind::RelativeIntent, Instrument::ElectricGuitar),
     ("gentle-acoustic", "Gentler acoustic attack — provisional direction", MapKind::RelativeIntent, Instrument::AcousticGuitar)]
        .into_iter().map(|(id,label,kind,instrument)| {
            let dark = id == "dark-metal-guitar";
            let values_db = frequencies().iter().map(|&hz| if kind == MapKind::Preserve { 0. } else {
                // Authored broad presence reduction, smoothly returning to zero at the limits.
                -(if dark { 3. } else { 2. }) * (-0.5 * ((hz / if dark { 2500. } else { 3200. }).log2() / 0.85).powi(2)).exp()
            }).collect();
            ToneMap { schema_version:1, id:id.into(), version:1, label:label.into(), kind,
                provisional:true, basis:"Original bounded musical direction relative to the frozen phrase; no reference recording establishes a genre standard. Dark means reduced broad upper-mid presence here.".into(),
                context: Context { instrument, capture:Capture::RecordedTrack, tuning:"unknown".into(), register_hz:None, technique:"representative phrases; unknown exact technique".into() },
                sources:vec![Reference { citation:"GigPies authored intent, 2026-10-03".into(), locator:"docs/EQ_MATCHING.md#initial-map-collection".into(), license:"MIT; original numerical direction only".into(), distribution:Distribution::OriginalAuthored, capture_notes:"No audio capture; frozen source-relative direction, not a reference spectrum or imported EQ preset".into() }],
                analysis:AnalysisSettings::default(), supported_hz:[125.,6300.], values_db, uncertainty_db:vec![0.75;BANDS], supported:vec![true;BANDS],
                limitations:vec!["Provisional: no musician acceptance; cannot reproduce distortion, dynamics, performance or ambience.".into(), "Tuning/register and technique must be reviewed. Unknowns restrict interpretation; no universal style classification.".into()], measurement:None }
        }).collect()
}
