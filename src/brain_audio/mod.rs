//! Brain duplex I/O and independent-clock bridges. Neither construction nor
//! normal tests activate physical audio; explicit device opening is feature gated.
pub mod bridge;
pub mod device;
pub mod host;
