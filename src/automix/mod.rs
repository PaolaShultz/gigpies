//! Offline, causal soundcheck and frozen-settings stereo rendering.
pub mod config;
pub mod dsp;
mod render;
pub use render::{run, write_json};
