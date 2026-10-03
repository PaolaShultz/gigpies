//! Explicit opt-in socket setup check; normal tests never open a socket.
use gigpies::transport::{AUDIO_RECEIVE_BYTES, configure_audio_socket};
#[test]
#[ignore = "opt-in local UDP socket setup; no audio hardware or external traffic"]
fn audio_socket_has_declared_capacity_and_never_waits_for_a_packet() {
    let socket = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    let limits = configure_audio_socket(&socket).unwrap();
    assert_eq!(limits.requested_receive, AUDIO_RECEIVE_BYTES);
    assert!(limits.actual_receive >= AUDIO_RECEIVE_BYTES);
    let error = socket.recv(&mut [0; 1]).unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::WouldBlock);
}
