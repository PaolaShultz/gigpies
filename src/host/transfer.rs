//! Whole-block PCM admission, independent of the ALSA implementation.
//! A short eager read can move the application pointer past the period boundary
//! and make a period-sized avail_min wait miss the next interrupt.

#[derive(Debug, PartialEq)]
pub enum TransferError<E> {
    Device(E),
    Deadline,
    InvalidProgress,
}

/// Wait for the complete remaining block before transferring. Offsets are frames.
/// A rare partial transfer still advances exactly once; retry waits are shorter
/// when the remaining frames are below the original period-sized request.
pub fn transfer_frames<E>(
    frames: usize,
    mut expired: impl FnMut() -> bool,
    mut available: impl FnMut() -> Result<usize, E>,
    mut transfer: impl FnMut(usize) -> Result<Option<usize>, E>,
    mut wait: impl FnMut(u32) -> Result<(), E>,
) -> Result<(), TransferError<E>> {
    let mut offset = 0;
    while offset < frames {
        if expired() {
            return Err(TransferError::Deadline);
        }
        let remaining = frames - offset;
        let timeout = if offset == 0 { 20 } else { 1 };
        if available().map_err(TransferError::Device)? < remaining {
            wait(timeout).map_err(TransferError::Device)?;
            continue;
        }
        match transfer(offset).map_err(TransferError::Device)? {
            Some(n) if n > remaining => return Err(TransferError::InvalidProgress),
            Some(n) if n > 0 => offset += n,
            _ => wait(timeout).map_err(TransferError::Device)?,
        }
    }
    Ok(())
}
