# Authenticated remote commands and media

The `remote` module carries actual GigPies authority requests and GPA1 audio over
one mutually authenticated QUIC connection. Quinn and rustls require TLS 1.3,
explicit CA trust and a separately provisioned SHA256 leaf-certificate permission
record. Early data and TLS session resumption are disabled. Bind addresses must be
explicit private or loopback addresses. Credentials, peer policies and synthetic
run reports belong outside Git.

`RemoteServer` authenticates the peer before constructing `AuthenticatedContext`.
Its permissions limit which existing authority scopes the peer may acquire; they
are not themselves write grants. Mutations use the session-derived writer and a
live engine lease. C-AUDIO readback is required before writes, and processing and
structural writes require their corresponding recent snapshots. Policy replacement
invalidates contexts at owner dispatch as well as on the network worker, including
commands already queued when revocation occurs. Disconnect cancels matching pending
engine and structural work without stopping local source processing or recording.

`authority_channel` connects a network session to the source/control owner through
bounded SPSC queues. `AuthorityMailbox::service` runs outside render before the next
source block. `HostAuthority` wraps the same `LocalAudio` used by local providers;
there is no second mixer or authority. Network workers never own the audio clock.
Generation changes drain completed replies, retire queued intent and require a new
connection and explicit grants. No command is automatically replayed.

Control frames use a four-byte big-endian length followed by strict JSON, at most
65536 bytes. Large replies use the existing GP14 snapshot-page protocol with a
1 MiB assembly bound. Duplicate keys and depth checks apply after reassembly too.
GP-REMOTE version 1 wraps the provider's exact C-AUDIO, GP07-processing, GP05-modules
and GP14-structure contracts; unsupported versions are refused. Legacy contracts
remain explicit and cannot truncate a dynamic provider into a legacy shape.

Media negotiation binds the fresh TLS-exporter session, source epoch, capability
and map generations, stable channel IDs, roles, encoding and source-frame delay.
GPA1 datagrams are protected by QUIC. Groups fit the negotiated QUIC datagram size;
there are at most 64 groups and 256 selected channels per role. These are bounded
transport resources, not a product channel ceiling. Analysis is exact PCM24, FX
send/return is float32. The composed host advertises 48-frame blocks; the standalone
transport also supports 96. Replayed, wrong-direction, stale-generation and late
wet packets cannot advance the source clock or enter a replacement source epoch.

The host taps admitted raw capture slots for analysis and actual pre-wet FOH buses
for FX. `BrainWorker` assembles source-frame groups and calls the owning FX library
through `host::BrainFx`; wet return enters the existing graph before PA. Its 960-frame
intentional FX delay is separate from negotiated network return delay. Missing wet
fades through `WetGate` while local dry processing and REC continue. Source recovery
clears previous media and fade history and requires explicit output rearm.

With `hardware-host`, `gigpies-remote --config PRIVATE.json` provides bounded,
device-free provider and Brain modes for functional acceptance. Provider duration
is at most 60 seconds and its synthetic source is driven independently of clients.
Optional `record_take` starts the real REC owner, immediately releases its initial
FOH grant, and finalizes an explicitly incomplete take at source end. Initial
outputs remain safe until a client explicitly rearms them. Reports include exact
analysis sample comparisons, per-channel counts, packet hashes, wet acceptance and
source-frame evidence. These runs do not establish physical synchronization or
realtime hardware qualification.

Focused normal tests: `cargo +1.97.1 test --locked --features hardware-host --lib remote::tests`.
Actual owner REC evidence is opt-in with `GP05_MANIFEST` set to an accepted manifest:
`cargo +1.97.1 test --locked --features hardware-host --lib actual_host_recording_survives_brain_loss_and_finalizes_clock_fault -- --ignored`.
Run the complete normal default and hardware-host suites for integration changes.
