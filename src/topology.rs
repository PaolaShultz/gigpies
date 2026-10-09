//! Validated resource admission and physical mapping. Profiles are software references,
//! never observations of an attached interface. All provisioning is off render.
use crate::show::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputPort {
    pub id: String,
    pub capture_slot: usize,
    pub physical_port: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum OutputSource {
    Main { channel: usize },
    Monitor { index: usize },
    Pa { index: usize },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutputPort {
    pub id: String,
    pub playback_slot: usize,
    pub physical_port: String,
    pub source: Option<OutputSource>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineTopology {
    pub identity: String,
    pub mapping_evidence: String,
    pub map_revision: u64,
    pub sample_rate: u32,
    pub max_block_frames: usize,
    pub capture_channels: usize,
    pub playback_channels: usize,
    pub inputs: Vec<InputPort>,
    pub measurement_slots: Vec<usize>,
    pub monitors: usize,
    pub pa_outputs: usize,
    pub outputs: Vec<OutputPort>,
}
/// Deployment policy, not a product channel ceiling. Memory accounts for strip state,
/// two prepared/control banks and maximum-block PCM scratch. Work units bound the
/// admitted strip/send multiply count per block; this is not a deadline claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceBudget {
    pub bytes: usize,
    pub sample_operations: usize,
}
impl Default for ResourceBudget {
    fn default() -> Self {
        Self {
            bytes: 64 * 1024 * 1024,
            sample_operations: 16 * 1024 * 1024,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Admission {
    pub estimated_bytes: usize,
    pub sample_operations: usize,
    pub estimated_snapshot_bytes: usize,
}
impl EngineTopology {
    pub fn legacy() -> Self {
        let mut t = Self::software(8, 2, 0).expect("legacy topology");
        t.identity = "legacy8-2".into();
        t
    }
    pub fn software(inputs: usize, monitors: usize, pa_outputs: usize) -> Result<Self> {
        if inputs == 0
            || inputs > u16::MAX as usize
            || monitors > u16::MAX as usize
            || pa_outputs > u16::MAX as usize
        {
            return Err("wire port index capacity".into());
        }
        let ops = inputs
            .checked_mul(monitors + 4)
            .and_then(|n| n.checked_mul(4096))
            .ok_or("resource overflow")?;
        if ops > ResourceBudget::default().sample_operations {
            return Err("work admission".into());
        }
        let playback_channels = 2usize
            .checked_add(monitors)
            .and_then(|v| v.checked_add(pa_outputs))
            .ok_or("dimension overflow")?;
        let scratch = (inputs * 2 + playback_channels + monitors + 2)
            .checked_mul(4096 * 8)
            .ok_or("resource overflow")?;
        let snapshot = inputs
            .checked_mul(monitors + 3)
            .and_then(|n| n.checked_mul(1200))
            .and_then(|n| n.checked_add(inputs * 3072 + playback_channels * 512 + 16384))
            .ok_or("snapshot resource overflow")?;
        if scratch > ResourceBudget::default().bytes {
            return Err("memory admission before provisioning".into());
        }
        if snapshot > crate::snapshot_pages::ASSEMBLY_BYTES {
            return Err("snapshot admission before provisioning".into());
        }
        let t = Self {
            identity: format!("software-{inputs}-{monitors}-{pa_outputs}"),
            mapping_evidence: "synthetic".into(),
            map_revision: 1,
            sample_rate: 48000,
            max_block_frames: 4096,
            capture_channels: inputs,
            playback_channels,
            inputs: (0..inputs)
                .map(|i| InputPort {
                    id: format!("input-{:02}", i + 1),
                    capture_slot: i,
                    physical_port: format!("synthetic-capture-{}", i + 1),
                })
                .collect(),
            measurement_slots: vec![],
            monitors,
            pa_outputs,
            outputs: (0..playback_channels)
                .map(|i| OutputPort {
                    id: format!("output-{:02}", i + 1),
                    playback_slot: i,
                    physical_port: format!("synthetic-playback-{}", i + 1),
                    source: Some(if i < 2 {
                        OutputSource::Main { channel: i }
                    } else if i < 2 + monitors {
                        OutputSource::Monitor { index: i - 2 }
                    } else {
                        OutputSource::Pa {
                            index: i - 2 - monitors,
                        }
                    }),
                })
                .collect(),
        };
        t.validate(ResourceBudget::default())?;
        Ok(t)
    }
    pub fn reference_16_18(pa_outputs: usize, monitors: usize) -> Result<Self> {
        let mut t = Self::software(16, monitors, pa_outputs)?;
        t.identity = "umc1820-ada8200-analog-48k-reference".into();
        t.mapping_evidence = "manufacturer-reference-unverified".into();
        t.capture_channels = 18;
        t.playback_channels = 20;
        for (i, p) in t.inputs.iter_mut().enumerate() {
            p.capture_slot = if i < 8 { i } else { i + 2 };
            p.physical_port = if i < 8 {
                format!("umc1820-analog-in-{}", i + 1)
            } else {
                format!("ada8200-analog-in-{}", i - 7)
            };
        }
        t.outputs = (0..18)
            .map(|i| OutputPort {
                id: format!("analog-out-{:02}", i + 1),
                playback_slot: if i < 10 { i } else { i + 2 },
                physical_port: if i == 0 {
                    "umc1820-main-out-left".into()
                } else if i == 1 {
                    "umc1820-main-out-right".into()
                } else if i < 10 {
                    format!("umc1820-line-out-{}", i + 1)
                } else {
                    format!("ada8200-analog-out-{}", i - 9)
                },
                source: None,
            })
            .collect();
        t.validate(ResourceBudget::default())?;
        Ok(t)
    }
    pub fn validate(&self, budget: ResourceBudget) -> Result<Admission> {
        if ![
            "synthetic",
            "manufacturer-reference-unverified",
            "operator-verified",
        ]
        .contains(&self.mapping_evidence.as_str())
        {
            return Err("mapping evidence".into());
        }
        if self.identity.is_empty()
            || self.identity.len() > 128
            || self.map_revision == 0
            || self.sample_rate != 48000
            || self.max_block_frames == 0
            || self.inputs.is_empty()
            || self.capture_channels == 0
            || self.playback_channels == 0
        {
            return Err("topology identity/rate/dimensions".into());
        }
        // Port IDs use the wire's unsigned 16-bit index domain, independently of fixtures.
        if self.inputs.len() > u16::MAX as usize || self.monitors > u16::MAX as usize {
            return Err("wire port index capacity".into());
        }
        let mut physical_inputs = BTreeSet::new();
        let mut capture = BTreeSet::new();
        for (i, p) in self.inputs.iter().enumerate() {
            if p.physical_port.is_empty()
                || p.physical_port.len() > 128
                || !physical_inputs.insert(&p.physical_port)
                || p.id != format!("input-{:02}", i + 1)
                || p.capture_slot >= self.capture_channels
                || !capture.insert(p.capture_slot)
            {
                return Err("input mapping".into());
            }
        }
        for &slot in &self.measurement_slots {
            if slot >= self.capture_channels || !capture.insert(slot) {
                return Err("measurement mapping overlaps program".into());
            }
        }
        let mut physical_outputs = BTreeSet::new();
        let mut slots = BTreeSet::new();
        let mut ids = BTreeSet::new();
        for p in &self.outputs {
            if p.physical_port.is_empty()
                || p.physical_port.len() > 128
                || !physical_outputs.insert(&p.physical_port)
                || p.id.is_empty()
                || p.id.len() > 128
                || !ids.insert(&p.id)
                || p.playback_slot >= self.playback_channels
                || !slots.insert(p.playback_slot)
            {
                return Err("duplicate/invalid output mapping".into());
            }
            match p.source {
                Some(OutputSource::Main { channel }) if channel >= 2 => {
                    return Err("main port".into());
                }
                Some(OutputSource::Monitor { index }) if index >= self.monitors => {
                    return Err("monitor port".into());
                }
                Some(OutputSource::Pa { index }) if index >= self.pa_outputs => {
                    return Err("PA port".into());
                }
                _ => (),
            }
        }
        let coefficients = self
            .inputs
            .len()
            .checked_mul(
                4usize
                    .checked_add(self.monitors)
                    .ok_or("resource overflow")?,
            )
            .ok_or("resource overflow")?;
        let sample_operations = coefficients
            .checked_mul(self.max_block_frames)
            .ok_or("resource overflow")?;
        let scratch = self
            .capture_channels
            .checked_add(self.playback_channels)
            .and_then(|n| n.checked_add(self.inputs.len()))
            .and_then(|n| n.checked_add(self.monitors + 2))
            .and_then(|n| n.checked_mul(self.max_block_frames))
            .and_then(|n| n.checked_mul(8))
            .ok_or("resource overflow")?;
        let estimated_bytes = coefficients
            .checked_mul(128)
            .and_then(|n| {
                n.checked_add(
                    self.inputs
                        .len()
                        .checked_mul(std::mem::size_of::<crate::channel_processing::Strip>() * 2)?,
                )
            })
            .and_then(|n| n.checked_add(scratch))
            .ok_or("resource overflow")?;
        if estimated_bytes > budget.bytes {
            return Err(format!(
                "memory admission: {estimated_bytes} bytes exceeds {}",
                budget.bytes
            ));
        }
        if sample_operations > budget.sample_operations {
            return Err(format!(
                "work admission: {sample_operations} exceeds {}",
                budget.sample_operations
            ));
        }
        // Conservative serialized worst-case including repeated authority in the
        // rendered envelope, held/proposed metadata and processing configuration.
        let estimated_snapshot_bytes = self
            .inputs
            .len()
            .checked_mul(self.monitors + 3)
            .and_then(|n| n.checked_mul(1200))
            .and_then(|n| n.checked_add(self.inputs.len().checked_mul(3072)?))
            .and_then(|n| n.checked_add(self.outputs.len().checked_mul(512)?))
            .and_then(|n| n.checked_add(16384))
            .ok_or("snapshot resource overflow")?;
        if estimated_snapshot_bytes > crate::snapshot_pages::ASSEMBLY_BYTES {
            return Err(format!(
                "snapshot admission: estimated {estimated_snapshot_bytes} exceeds one MiB coherent assembly"
            ));
        }
        Ok(Admission {
            estimated_snapshot_bytes,
            estimated_bytes,
            sample_operations,
        })
    }
    /// Independent telemetry admission. Audio remains admitted when this fails;
    /// legacy resource/readback contracts remain byte/schema compatible.
    pub fn meter_admission(&self, budget: ResourceBudget) -> Result<usize> {
        let audio = self.validate(budget)?;
        let taps =
            crate::metering::capacity(self.inputs.len(), self.monitors).ok_or("meter capacity")?;
        let bytes = taps
            .checked_mul(std::mem::size_of::<crate::metering::Accumulator>() * 3 + 512)
            .and_then(|n| n.checked_add(self.inputs.len().checked_mul(48 * 8)?))
            .and_then(|n| n.checked_add(8192))
            .ok_or("meter resource overflow")?;
        if audio
            .estimated_bytes
            .checked_add(bytes)
            .is_none_or(|n| n > budget.bytes)
        {
            return Err("meter memory capacity".into());
        }
        Ok(bytes)
    }
    pub fn is_legacy(&self) -> bool {
        self.identity == "legacy8-2"
            && self.inputs.len() == 8
            && self.monitors == 2
            && self.pa_outputs == 0
    }
    /// Scatter only validated logical outputs; every unmapped stream slot is zero.
    pub fn patch_outputs(&self, buses: &[f64], pa: &[f64], output: &mut [f64]) -> Result<()> {
        if buses.len() != self.monitors + 2
            || pa.len() != self.pa_outputs
            || output.len() != self.playback_channels
        {
            return Err("patch shape".into());
        }
        output.fill(0.);
        for p in &self.outputs {
            output[p.playback_slot] = match p.source {
                None => 0.,
                Some(OutputSource::Main { channel }) => buses[channel],
                Some(OutputSource::Monitor { index }) => buses[index + 2],
                Some(OutputSource::Pa { index }) => pa[index],
            };
        }
        Ok(())
    }
}
