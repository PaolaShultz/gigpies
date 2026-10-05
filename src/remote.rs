//! Opt-in authenticated command and media transport. All methods are worker-side;
//! none belong in an audio callback. TLS identity is an authorization ceiling,
//! never a replacement for the processing authority's leases and revisions.
mod authority;
#[cfg(feature = "hardware-host")]
mod brain;
#[cfg(all(target_os = "linux", feature = "hardware-host"))]
mod host;
mod media;
mod policy;
mod proxy;
#[cfg(all(target_os = "linux", feature = "hardware-host"))]
mod runner;
mod session;
mod tls;
mod wire;

pub use authority::*;
#[cfg(feature = "hardware-host")]
pub use brain::*;
#[cfg(all(target_os = "linux", feature = "hardware-host"))]
pub use host::*;
pub use media::*;
pub use policy::*;
pub use proxy::*;
#[cfg(all(target_os = "linux", feature = "hardware-host"))]
pub use runner::*;
pub use session::*;
pub use tls::*;
pub use wire::*;

pub type Result<T> = std::result::Result<T, String>;
pub const ALPN: &[u8] = b"gigpies-remote/1";
pub const MAX_FRAME: usize = 65_536;
pub const MAX_PEERS: usize = 64;
pub const MAX_CONNECTIONS: usize = 4;
pub const IO_TIMEOUT_MS: u64 = 2_000;

#[cfg(test)]
mod tests;

/// Shared process-local lease clock. It is independent of source audio frames;
/// every controller endpoint on this host uses the same monotonic origin.
pub fn monotonic_ms() -> u64 {
    static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    START
        .get_or_init(std::time::Instant::now)
        .elapsed()
        .as_millis()
        .min(u64::MAX as u128) as u64
}
