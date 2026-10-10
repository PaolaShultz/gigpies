# Brain operator audio control

`GP15-brain:1` extends the existing C-AUDIO v2 show, epoch, writer, lease,
request ID and revision authority. It does not grant audio rights merely from
an authenticated connection. `brain_snapshot` is observational. Mutations need
connection-bound readback no older than 250 ms and are acknowledged only after
application at the next 48-frame Stagebox boundary. Invalid preparation leaves
state and revision unchanged. Duplicate IDs bind the complete request body.

Grant scopes are `local_operator_monitor`, `talkback_destinations`, and the
separately protected `talkback_foh`; they cannot edit band mix parameters.
`monitor_set` selects exactly one `none`, `main`, `monitor {index}`,
`pfl {input}`, or `afl {input}` source. Indices are zero based and validated
against admitted topology, including higher input banks. Selection changes
increment `selection_generation`, clear the old tap, and must be disarmed.
A separate readback and explicit arm follow acquisition. Monitoring settings
`gain_cdb` (−9000 through 0), `mute`, `dim`, and `armed` apply to the Brain
playback owner; the Stagebox sends the selected source without those gains.
Selecting a source never changes audience or performer samples.

PFL is post channel EQ and compressor, before shared mute, fader and pan,
centered mono at −3.0103 dB per stereo side. AFL is post processing, shared mute,
fader and pan, preserving stereo position. Main is actual dry plus wet before
PA and before talkback. Performer monitoring is the selected mono monitor bus,
centered at −3.0103 dB per side. The selection does not sum unrelated sources. Disarming increments the selection
generation even when the source is unchanged, invalidating queued old audio.
Mute, dim and gain remain local playback changes while the bridge keeps consuming.
No implicit talkback sidetone exists, including when listening to main/monitor.

`talkback_set` supplies explicit `monitors` indices, bounded `gain_cdb`, and
`mute`; startup is muted with no destinations. `talkback_foh {enabled}` requires
its own scope, whose lease is checked again during rendering. Route changes
close any hold. `hold`, `heartbeat`, and `release` carry a nonzero held-action generation;
each new hold must exceed the previous generation. A released generation cannot reopen. Holds are single owner,
with matching writer and lease; release, revoke, session/device fault and source
recovery close them. Reconnect requires fresh identity and new explicit hold.

Send heartbeat every 50 ms. Its `observed_frame` is the Stagebox snapshot frame
seen by that controller, not its own clock. Reject future observations, regressed
absolute deadlines, and observations more than 2400 frames (50 ms) old at the
actual application boundary. A fixed 64-entry rendered-frame/monotonic-time
history also checks that observation is at most 50 ms old in actual consumer time;
its monotonic expiry cannot exceed observation time plus 150 ms. Source scheduling
stalls therefore cannot make old heartbeats fresh again. Expiry is at most
`observed_frame + 7200`; receiving
a delayed in-flight heartbeat never grants a new full 150 ms. The consumer closes
at that absolute source deadline or the corresponding remaining monotonic delay.
The initial hold closes at 150 ms by either monotonic
control time or the Stagebox source-frame deadline, so stalled Brain scheduling
cannot indefinitely extend a hold. The fade is at most 240 frames (5 ms at
48 kHz), plus the declared one-block application boundary. Missing media produces
silence; samples are never replayed from a previous block. Bounded microphone
samples are clamped to ±1 before gain and an additive destination injection.
The operator must reserve destination headroom for the extra signal; no hidden
normalization or ducking changes band levels. Meters report actual received
microphone and injected samples, not proof of an acoustic path. Wire fields
`microphone_peak_nano`, `outgoing_peak_nano` and `monitor_peak_nano` are unsigned
integer linear amplitude in billionths of digital full scale: 1.0 is 1,000,000,000.
Snapshots obey the existing strict integer-only JSON grammar.

The production LocalAudio path captures the distinct dry FX send before injection.
ModuleGraph computes actual local/external wet, preserves the program monitor
feed, adds talkback, then invokes SHR PA. FOH talkback requires an attached PA
module with admitted outputs and is never injected into a direct Main bypass.
Monitor talkback reaches only selected buses and the existing final output patch
and global output safety gain. The current monitor model has per-input sends
and shared input mute, but no separate monitor-master gain/mute; talkback uses
its explicit bounded gain/mute and does not falsely advertise such a master.
Raw recording and analysis consume the original input buffer and never talkback.

LocalAudio is a controller-side pump; parsing, authority staging/cloning, buffer
preparation and old-state destruction happen outside the allocation-guarded Mixer,
`render_talkback_block`, and ModuleGraph render sections. The talkback helper
receives a borrowed view of prepared gain, destination indices, safety ramp and
consumer deadline; it cannot retire strings, leases or buffer owners. The mixer tap uses a fixed 48-frame stereo ring;
operator and talkback buffers are prepared at bind. The source graph never opens
a Brain device or changes its own clock on a Brain failure. Transport/bridge/device
owners provide epoch/map validation, prefill, local output transitions and media
readiness. `talkback_path_ready` and `monitor_path_ready` require current authenticated
descriptor, fresh matching device observation, live device, acquired/armed bridge
and corresponding active hold or operator output. `audible_path_ready` is their
OR; all start false and clear on closure or selection/hold identity change.
Configured/held alone is not proof of readiness.

Producer fixtures live in `tests/fixtures/brain-v1`. Regenerate explicitly with
`GIGPIES_UPDATE_BRAIN_FIXTURES=1 cargo +1.97.1 test --locked -j1 --test brain_control
producer_fixtures_match_strict_wire_and_failures`, under the host build lock and
standard nonincremental build environment. Normal tests compare without writing.
Physical card/channel verification and acoustic acceptance remain separate gates.

Actual owner acceptance is opt-in because trusted owner shared libraries are
explicit artifacts, not downloaded fixtures. Run under the standard build lock:

```sh
GP05_MANIFEST=/absolute/verified/modules.json \
GP14_PA_FIXTURES=/absolute/shr-pa/tests/fixtures/cpa/v2 \
CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=1 \
cargo +1.97.1 test --locked -j1 --features hardware-host --test brain_owners -- --include-ignored
```

These tests use the production LocalAudio/Mixer/ModuleGraph and actual SHR PA/FX/REC,
16/32/48-input profiles, every monitor destination, distinct FOH permission,
independently assembled PA input samples, an independently computed −24 dBFS
protection ceiling, actual raw recorded PCM, published raw analysis windows,
and allocation/deallocation guards on Mixer, the actual talkback gain/fade/injection
helper, and ModuleGraph. Dynamic-library
internal allocation protection remains covered by the owner's own guard suite;
the host allocator cannot instrument a separately linked allocator by implication.

## Compact held proof (GP15-held-proof:1)

The additive read-only contract serves only TalkbackDestinations initial press
and held heartbeats. Other scopes refuse. Lease maintenance uses the independent
atomic operation below; configuration/review uses full paired readback. Existing
provider paths remain supported. Combined producer/consumer validation and final
network acceptance remain required.

Each query carries an explicit positive query ID, authenticated session/writer,
show/source epoch, capability/map generations, exact live scoped lease and a
previously reviewed TB configuration digest. One synchronous owner observation
returns committed revision/source frame, lease remaining, fixed Brain scalar
state and separate current FOH/media authorization. No channel/monitor matrix is
cloned. Only successful proofs update held-read freshness; they never grant,
renew, rearm, mutate revision or mark the full raw UI snapshot fresh.

The digest is independently computable from the existing full raw topology and
GP15-brain:1 snapshot. Desk pins it while unheld, before the explicit gesture;
compact readback cannot replace it. Changed destinations, TB gain/mute, protected
FOH intent or attachment identity refuse the old digest. Unrelated operator
monitor edits do not invalidate TB configuration. Destination hashing is prepared
off render; source-boundary reapplication carries the prepared hash without
recomputing it. Existing controller-boundary state cloning is not described as an
allocation-free audio callback.

The strict schema and canonical byte encoding are defined in
[src/held_proof.rs](../../src/held_proof.rs). Common digest identity includes show,
source epoch, capability/map generations and six topology dimensions; sorted
unique destinations have a separate domain-separated SHA-256. The final digest
includes that cached hash and TB gain/mute/FOH intent. Existing GP15-brain:1 and
device:1 schemas remain unchanged. Producer-generated success/refusal fixtures
and the explicit generation command live in
[held-proof fixtures](../../tests/fixtures/held-proof-v1/README.md).

Original 30 ms probe/20 ms send margin, 50 ms observation/heartbeat, 150 ms deadman
and 240-frame fade bounds remain. The heartbeat uses revision and observed frame
from the same own-query witness. Lease remaining is conservatively anchored at
query send-start, not response arrival. Canceled query identity must drain before
a new nonce; late replies/finals cannot authorize another gesture. Unsupported,
stale, mismatched or refused proofs fail held admission closed, with no silent
full-raw fallback during a hold. Initial press retains its existing prerequisites;
it does not require bridge readiness that can arise only after hold activation.


## Atomic lease maintenance and paired readback

`GP15-lease-maintenance:1` maintains an existing canonical C-AUDIO scoped lease:
`foh`, `monitor1`, `monitor2`, `{"monitor": N}` (u16 N >= 3), `pa_configuration`,
`output_routes`, `local_operator_monitor`, `talkback_destinations` or `talkback_foh`.
Each requires its own current scope permission. A dynamic monitor lease must
already have been granted against the configured topology; the wire number is
not a topology grant or a product monitor cap. Noncanonical `{"monitor": 0/1/2}`
values refuse scope, while unknown/malformed representations fail decoding.
Named unit scopes use strings, never object aliases. The three existing Brain
scope wire spellings and valid fixture bytes are unchanged. Maintenance
never acquires authority, advances revision/source frame, changes configuration,
rearms output or extends a held heartbeat. The authenticated connection identity,
show/epoch, capability/map generations, writer, exact scope and live lease must
match at each call, including retries. Monotonic control time is checked first.
A new admissible `maintenance_id` extends only expiry to checked `now + 2000 ms`.

For consumers, the three Brain scopes retain their existing maintenance path.
Other scopes use this operation after explicit GP15 paired mode is requested on
that authenticated attachment, including a failed paired-read attempt. A generic
QUIC/held identity does not enable this mode. Passive and pre-command renewals
must make the same choice and need no full topology read solely for maintenance.
Ordinary non-GP15 clients retain legacy revision renewal. Once GP15 is requested,
refusal or an unsupported operation fails closed, without legacy fallback,
regrant or replay under a fresh identity. Reconnect clears attachment mode and
cannot restore an expired lease. Older providers may refuse the added scopes;
this additive admission does not change envelope version or imply compatibility.

Maintenance has a separate per-live-lease ID namespace and 64 exact cached
request/results plus a highwater counter. Current admission precedes cache lookup;
exact replay returns the original result without extending expiry again. Changed
IDs refuse `reused_id`, evicted/older IDs refuse `expired_id`. New IDs refuse and
cache `unavailable` while any ordinary, processing or external authority work is
pending. Cached success still replays during staging, and cached busy refusal
still refuses after commit/cancellation. Authority clones carry the same live
lease history. Authentication/context failures allocate no cache entries.

`GP15-paired-readback:1` returns one existing raw `RenderedSnapshot` and one
`GP15-brain` snapshot from the same committed owner boundary, without a tick or
await between them. Pending commands expose both old views; after commit they
expose both new views. Both revision and source frame must match. Read-only TLS
peers can use this operation without a lease or write permission. Its increasing
`query_id` namespace is independent from maintenance, mutations and held proof,
bounded by the existing connection budget and removed on disconnect. An
authenticated, attachment-matching nonce is consumed on admission even if the
subsequent observation fails; it never caches a large response.

The complete paired envelope, including echoed request context, must fit the
existing 1 MiB assembly bound. Only a successfully constructed envelope admits the
raw/Brain server read latches. Processing-read freshness is preserved unchanged.
No topology, transport page, frame, connection or render-resource bound increases.
Maintenance request/reply envelopes are at most 8192 bytes. Unknown versions and
malformed envelopes use explicit outer refusals; neither operation silently falls
back to another authority or readback path.

The maintenance reply is an outcome, not held proof or full UI freshness.
Consumers must correlate its exact context and anchor confirmed expiry to the
original send-start plus 2000 ms, including exact retries. Full paired readback
retains one original 250 ms deadline and 64-document budget; a consumer must
validate both complete schemas and install both or neither. The compact held
lane retains its separate 30 ms probe, 20 ms send margin, 50 ms observation and
heartbeat, 150 ms deadman and 240-frame fade bounds. Source-boundary readiness is
not current wall-clock authorization proof.

Producer schemas: [maintenance](../../src/lease_maintenance.rs) and
[paired readback](../../src/paired_readback.rs). Producer fixtures and generation
commands: [atomic fixtures](../../tests/fixtures/atomic-control-v1/README.md).
This increment is offline software behavior; combined Desk reconciliation,
network acceptance and physical qualification are separate gates.
