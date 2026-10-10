# Authenticated processing-node and Brain endpoints

The opt-in `remote` module uses maintained Quinn/rustls QUIC with mutual TLS.
Explicit CA trust and pinned leaf SHA256 peer records establish identity; the
peer permission set is a ceiling over the actual mixer/processing/PA authority.
No default public listener, hard-coded credentials, 0-RTT command replay or SSH
transport substitute is provided. The private same-UID Unix endpoint remains.
Credentials and pairing configuration belong in private deployment state.

Remote payloads use the same versioned authority schemas. A connection reads a
snapshot before acquiring a grant. Processing and structural edits require fresh
readback. Writer identity is bound to the authenticated session; leases, scope,
request IDs, expected revisions and application-frame results remain authoritative
on the processing node. Revocation and disconnect cancel queued intent. New
sessions read state and acquire new authority rather than replaying old writes.

Control framing is bounded to 64 KiB. Large immutable replies use
`GP14-snapshot-pages:1`: 8 KiB chunks, SHA256 identity of the serialized complete
reply, at most 1 MiB and a two-second assembly window. Consumers reject mixed,
duplicate, missing or stale pages rather than combining revisions. Network tasks
use bounded command/result/media queues; a slow peer does not perform work in DSP.

GPA1 media travels inside authenticated QUIC datagrams. A separate unauthenticated
UDP packet is not accepted just because a TLS control session exists. Negotiated
stream descriptors bind session, source epoch, capability/map generations, role,
channel IDs, grouping and return deadline. Analysis uses selected raw pre-fader
sources; FX sends use actual FOH buses, and actual SHR FX runs on Brain. Wet returns
join dry FOH before the actual PA owner. Missing returns fade through the existing
bounded gate; local recording and dry/monitor paths do not depend on Brain.
Algorithmic delay does not synchronize physical converters.

The bounded `gigpies-remote --config PRIVATE.json` provider/Brain executable defaults
to synthetic acceptance. Its explicit physical provider configuration additionally
requires `--activate-physical` and separate session authorization; see
[device composition](BRAIN_AUDIO.md#executable-device-composition). Nothing installs
or starts a service.
Use explicit private bind addresses, per-run identities and duration limits.
The provider begins muted; operator authority controls patch/PA/rearm. Both nodes
must reserve any functional network run. Tests and qualification are recorded in
[acceptance](../acceptance/MODULAR_ENGINE_ACCEPTANCE.md); no network test proves physical-clock
lock or full-show realtime capacity.

The 16/32/48 software profiles have passed actual two-Pi operator, grouped analysis,
owner FX, REC and Brain loss/restart acceptance. Use an optimized Desk build for
the measured larger profiles; debug decode cost can exceed the unchanged250 ms
freshness fence and is refused safely. The finite Brain runner queues close but
does not await transport drain before process exit, so its peer may retain media
ownership until the 2 s idle timeout. Reconnect never steals an occupied media role;
accepted recovery requires a fresh session and observed old-owner retirement.

## Separate Brain local duplex owner

The Brain runner and loss/restart evidence above describe source-clock FX. The
complete Brain also has a [local duplex card](BRAIN_AUDIO.md) for microphone
capture and operator playback, with independent device epochs and two bounded
ASRC crossings. GP15-brain/device/media and GBA1 roles extend the authenticated
path in [REMOTE_TRANSPORT](REMOTE_TRANSPORT.md) without taking the existing FX
media owner. Desk never opens PCM. Integrated task0015 acceptance is tracked
separately; task0014's source-clock results are not physical two-card evidence.

Configured monitor tap control uses [GP18-sends:1](MONITOR_SENDS.md) on the same
Unix/authenticated endpoints. Each connection needs its own fresh GP18 readback
and matching monitor permission/lease. Configured channel processing is GP07:4;
GP07:3 requests receive typed unsupported-version refusals.
