//! Explicit worker-side socket setup. Never called by an audio callback.
use socket2::SockRef;
use std::{io, net::UdpSocket};

/// Bounded per-socket request. Linux reports twice this for bookkeeping.
/// This changes only the supplied socket, not host sysctls or interface settings.
pub const AUDIO_RECEIVE_BYTES: usize = 512 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SocketBuffers {
    pub requested_receive: usize,
    pub actual_receive: usize,
    pub actual_send: usize,
}
/// Apply the measured audio-worker buffer size and nonblocking mode. A host
/// that cannot supply the requested capacity must fail setup rather than claim
/// the measured operating envelope. Datagram deadlines still bound audio age.
pub fn configure_audio_socket(socket: &UdpSocket) -> io::Result<SocketBuffers> {
    let reference = SockRef::from(socket);
    reference.set_recv_buffer_size(AUDIO_RECEIVE_BYTES)?;
    let actual_receive = reference.recv_buffer_size()?;
    // Linux doubles SO_RCVBUF both on set and get. Other targets report capacity
    // differently; they need their own network acceptance before deployment.
    let minimum = if cfg!(target_os = "linux") {
        AUDIO_RECEIVE_BYTES * 2
    } else {
        AUDIO_RECEIVE_BYTES
    };
    if actual_receive < minimum {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "audio receive buffer was clamped below the validated capacity",
        ));
    }
    socket.set_nonblocking(true)?;
    Ok(SocketBuffers {
        requested_receive: AUDIO_RECEIVE_BYTES,
        actual_receive,
        actual_send: reference.send_buffer_size()?,
    })
}
