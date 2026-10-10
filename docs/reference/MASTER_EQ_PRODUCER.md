# Live master EQ producer — task0018 software validated

GP18-master-eq:1 is independent of GP18-sends:1 and GP14-structure:1. Its outer
JSON remains integer-only. Fractional EQ settings and owner normalized coefficient
banks are bounded opaque JSON strings. Historical GP03/GP14/GP07 and corrected
sends fixture bytes are unchanged. Coordinator review and fixture freeze completed
at producer checkpoint `42e038a80373fd9317692d5da770b7079d0205d0` with SHR PA
`85ed759499b5223e30a53e2dbea1e6fec03f04f9`. Exact generating-source and artifact
pins remain in their fixture provenance; Desk integration is a separate gate.

## Request and readback

Use the existing envelope keys: contract, version, show_id, module, epoch, writer,
lease, request_id, expected_revision, kind, body. `master_eq_snapshot` has an empty
body and null mutation credentials. `master_eq_set` body has `patch_json`,
`program_buses`, `owner_instance`, `graph_generation`, `eq_generation`, `map_revision`.
The patch string has the owner EQ v1 schema, exactly two distinct actual input
indices, eight PEQ bands and 31 GEQ gains/enable flags per selected input.
It must contain the two unique owner input indices actually mapped from bus0/1.
Reversed and nonadjacent owner indices are valid. Missing or duplicate master-bus
mapping refuses live availability; physical channels are a separate contract.
The request bus vector is an identity pin, never a route mutation.

Replies use context, state, reason, effective_frame, revision, snapshot (null for
this contract), and `master_eq` on snapshots. Existing GP14 replies omit that new
optional field entirely. A master_eq snapshot identifies epoch/global revision/frame,
actual owner lifetime, graph/EQ generations, bus map/selected input indices, capability,
eligibility, fault, settlement, remaining source frames, retirement occupancy and
opaque `owner_json`. That owner-produced string contains current/target settings,
normalized coefficients, denominator convention, rate, generations and fingerprints.
Current and target are different during a transition. After a fault settlement is
false; committed target is recovery intent, never proof that DSP finished settling.

The request shares existing PaConfiguration authority, global revision, request-ID
history, deduplication and external pending reservation with PA/output/device/Brain
operations. Preparation can happen unmuted; full configuration still requires
mute/quiescence and a separate explicit rearm. A successful live EQ operation never
changes mute, hold or rearm state. A fully identical accepted patch increments both
normal global revision and independent owner EQ generation while preserving DSP
histories. Rejections preserve prior owner settings and host metadata.

## Boundary and transport

Both Unix and authenticated remote dispatch use independent per-session master-EQ
snapshot freshness (250ms), scoped writer/lease checks and original completion
routing. GP14 snapshots cannot substitute for live EQ freshness. Authentication
requires PaConfiguration permission on mutations; reads require the normal null
writer. Existing global external reservation revalidates lease/revision/expiry at
the strict next48-frame boundary. Revoke/disconnect cancels reserved work. Cached
final outcomes do not acquire new pending completion ownership. GP15 maintenance
uses the same external reservation and existing PaConfiguration lease; it never
adds permission or edits the transaction.

The host prepares the complete retained PA JSON offRT. It changes only selected
EQ settings in that metadata and pins the full bus vector. The narrow boundary
commit calls the optional owner extension directly, then infallibly swaps owned
metadata after owner success. It does not use full-graph commit, replace render
buffers or toggle PA hold. Old metadata stays in the caller's prepared wrapper
for offRT destruction. Snapshots, persisted composed intent and muted setup reopen
therefore agree with committed target. Source recovery reconstructs that target
muted/disarmed with no replay. REC/FX artifacts and the original PA fallback stay
unchanged. Optional symbols load as one validated capability; missing/partial/
incompatible extensions advertise unavailable while muted configuration remains.

## Evidence and consumer boundary

Normal focused tests live in `tests/master_eq.rs`; the explicitly selected actual
owner test additionally requires GP_EQ_MANIFEST, GP_EQ_OLD_MANIFEST,
GP_PA_V2_FIXTURES and GP_EQ_CORPUS. It opens no physical device. Actual owner
artifact/corpus hashes and source pins must accompany coordinator acceptance.
Desk renders a display response using these exact coefficient fixtures; it must
not implement PA audio DSP or plot committed target as settled current during fade.
No physical mapping, microphone, audio deadline, listening or acoustic claim follows.

## Scheduling and allocation evidence boundary

Loaded-owner allocation guards bracket the direct owner apply/process/status/retire
entrypoints, including endpoint, fault, busy/stale and managed-span refusals. The
ModuleGraph narrow commit contains owner acceptance plus an owned String swap; it
is separate from full control orchestration. LocalAudio's control/source worker
accepts sockets, parses/queues requests, prepares JSON/filter banks, clones authority
and builds replies outside the owner DSP contract. HostAuthority::process_source
is that source-worker entry, not an allocation-free hardware device callback.
These existing orchestration allocations and control I/O are outside the guarded
owner kernel; no whole-host hard realtime or physical deadline claim is made.
Actual transport evidence here uses Unix sockets and a policy-authenticated test
context through the receive codec/endpoint. It adds no TLS/QUIC handshake campaign.

## Historical evidence

The [dated study and original results](../archive/studies/MASTER_EQ_PRODUCER.md) retain experiment-specific settings and validation history.
