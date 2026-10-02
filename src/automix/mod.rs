//! Offline, causal soundcheck and frozen-settings stereo rendering.
pub mod config;
pub mod dsp;
mod render;
pub use render::{compare, run, write_json};

pub mod ambience;
pub mod effects;
mod exciter;
mod fx_engines;

pub mod analysis;

pub mod unity;

pub mod preservation;

pub mod balance;

/// Intent-conditioned offline guitar tone preparation.
pub mod tone;

/// Frozen spectral matching, artistic maps and offline musician review.
pub mod matching;

/// Explicit source profiles and bounded processing rules.
pub mod expert;

/// Independent produced-reference measurements; never selects mix settings.
pub mod reference;

/// DI bass evidence, note guards and bounded correction.
pub mod bass;

/// Kick/snare processing evidence and explicit rhythmic balance.
pub mod drums;

/// Conditional snare spill evidence and protected static trials.
pub mod bleed;

/// Frozen multi-reference identifiability diagnostics; never changes audio.
pub mod bleed_reference;
