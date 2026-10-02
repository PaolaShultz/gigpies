//! Offline, causal soundcheck and frozen-settings stereo rendering.
pub mod config;
pub mod dsp;
mod render;
pub use render::{compare, run, write_json};

pub mod effects;
mod exciter;
mod fx_engines;

pub mod analysis;

pub mod unity;

pub mod balance;

/// Intent-conditioned offline guitar tone preparation.
pub mod tone;
