//! Offline foundations for GigPies. No live audio or hardware is opened.
pub mod inventory;

/// Intended live processing rate; source recordings retain their original rate.
pub const LIVE_SAMPLE_RATE_HZ: u32 = 48_000;

pub mod automix;

/// Synthetic network audio and control contracts; no hardware is opened.
pub mod transport;

/// Bounded source-frame host contracts; opening hardware is explicitly opt-in.
pub mod host;

/// Pure C-ROLE:1 intended assignments; native ownership is a later adapter.
pub mod roles;
/// Portable C-SHOW:1 metadata and private checkpoints, outside callbacks.
pub mod show;

/// C-AUDIO:1 pure offline authority harness and bounded codec.
pub mod control_model;

/// Fixed-storage eight-input offline mixer; no host/PA binding.
pub mod mixer;
/// Control-side deterministic offline authority/render pump.
pub mod mixer_control;

/// Explicit opt-in same-UID private Unix binding for the offline engine.
#[cfg(target_os = "linux")]
pub mod local_audio;

/// Bounded named raw analysis subscriptions.
pub mod analysis_stream;

/// Explicit trusted owner-library synthetic graph; opens no hardware endpoints.
#[cfg(feature = "hardware-host")]
pub mod module_graph;

/// Separately discriminated read-only module health and scoped recorder envelope.
pub mod module_wire;

/// Versioned fixed-storage channel EQ and dynamics.
pub mod channel_processing;
/// Strict GP07 processing extension.
pub mod processing_wire;
