//! Offline foundations for GigPies. No live audio or hardware is opened.
pub mod inventory;

/// Intended live processing rate; source recordings retain their original rate.
pub const LIVE_SAMPLE_RATE_HZ: u32 = 48_000;
