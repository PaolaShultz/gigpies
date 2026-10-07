//! Independently versioned live master-EQ readback. Fractional owner settings and
//! normalized banks stay in an opaque bounded string; outer codecs stay integer-only.
use crate::show::Counter;
use serde::{Deserialize, Serialize};
pub const CONTRACT: &str = "GP18-master-eq";
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub show_id: String,
    pub epoch: Counter,
    pub revision: Counter,
    pub map_revision: Counter,
    pub frame: Counter,
    pub live_supported: bool,
    pub live_available: bool,
    pub fault_latched: bool,
    pub source_recovery_required: bool,
    pub settled: bool,
    pub unavailable_reason: Option<String>,
    pub owner_instance: Counter,
    pub graph_generation: Counter,
    pub eq_generation: Counter,
    pub program_buses: Vec<usize>,
    pub master_input_indices: Option<[usize; 2]>,
    pub transition_remaining_frames: Counter,
    pub retirement_occupied: bool,
    pub owner_json: Option<String>,
}
// Shared implementation does not admit these commands on the GP14 contract.
pub type Request = crate::structural_control::Request;
pub type Reply = crate::structural_control::Reply;
