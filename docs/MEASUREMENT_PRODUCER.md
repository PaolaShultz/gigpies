# GP20 measurement provider v1

This additive software-only control path connects reserved common-frame capture
pairs to the standalone SHR PA measurement owner. No device owner, excitation,
output route, gain, audio/MIDI/DMX action or automatic proposal application is added.
SHR PA owns analysis and candidate generation. Physical provider runners expose no
measurement enable option; acquisition/timing qualification remains separate.

## Frozen wire

Contract `GP20-measurement`, version integer1, uses the existing private same-UID
Unix audio endpoint and four-byte big-endian framing/paged replies. Every request
has exactly `contract`, `version`, `show_id`, `module`, `epoch`, `writer`, `lease`,
`request_id`, `expected_revision`, `kind`, `body`. Identity and canonical counter
strings follow C-AUDIO:2. Read requests require null writer/lease/request_id/expected_revision fields. Mutations
require the same connection's snapshot/writer/lease, PaConfiguration scope, current
revision and a measurement snapshot no older than250ms. The existing shared
begin_external/commit_external authority handles dedup/fingerprint and one boundary
reservation; this is not a synthetic lease-renew authorization side effect.

| kind | exact body |
|---|---|
| `measurement_snapshot` | `{}` |
| `measurement_result` | `{"id":"capture-or-proposal-id"}` |
| `capture_start` | `{"id":"a","reference_input":"input-01","mic_capture_slot":16,"output_index":0,"position_id":"p1","samples":32768,"options":{"max_arrival_samples":2048,"band_hz":[100,10000]}}` |
| `capture_cancel` | `{"id":"active-id"}` |
| `measurement_propose` | `{"id":"proposal-id","pairs":[["anchor-id","target-id"]]}` |

IDs are1..128 ASCII graphic bytes. Samples32768..65536; max arrival1..2048 samples;
integer-Hz band20..20000, ascending. Up to8 positions,16 retained records, one active
capture or owner job. No retries or automatic restarts. Result IDs cannot be reused
while retained. The microphone slot must belong to topology.measurement_slots and
cannot overlap any program strip; reference_input is an actual canonical strip ID.
Output index identifies an assigned output of the admitted PA v2 owner graph.

Replies contain exactly `contract`, `version`, `context`, `state`, `reason`,
`revision`, `effective_frame`, `snapshot`, `result`. Context is exactly the existing RequestContext fields: show_id, module, epoch,
writer, lease, request_id, expected_revision (no command kind). Reply states are `snapshot`, `result`, `pending`, `final`. Pending/final
boundary replies correlate exact request IDs; final reason null plus effective_frame
means accepted. It does not mean analysis succeeded. Read the record terminal state.

Snapshot fields: `available`, `software_only` (true), `reason`, `active_id`,
`capture_progress`, `results`, `current_basis`, `reserved_mic_slots`,
`reference_inputs`. Progress is null or `{id,received_samples,requested_samples,
output_index}`. Results is an array of summaries `{id,kind,state,reason}`;
kind is `capture` or `proposal`. Record states are `capturing`, `analyzing`,
`measured`, `proposed`, `no_change`, `refused`, `cancelled`, `invalidated`.
Cancellation/invalidation retain a reason and never silently erase the operation.

A detailed result has exactly `summary`, `basis`, `owner_result_json`,
`candidate_configuration_json`. Owner result is a nullable opaque string capped at
65536 bytes. Candidate is a nullable owner-validated graph JSON string capped at
49152 bytes. **All outer control JSON remains integer-only.** Finite floating-point
owner documents are decoded with a separate bounded duplicate-safe parser; shared
old control contracts are unchanged. Measured does not imply proposal eligibility;
owner_result_json preserves the PA result's eligibility and reasons.

Basis fields are `source_epoch`, `map_revision`, `graph_generation` (canonical
counter strings), `clock_domain` (`software-common-source`), `configuration_json`
(exact admitted owner graph), `program_buses` (unchanged integer indexes). Every
capture binds the actual PA status generation, exact graph/configuration/bus map,
source epoch and topology revision. Captures from different bases cannot combine.
Lease expiry, disconnect/revoke, cancellation, source discontinuity or changed basis
invalidate outstanding work/proposals; late worker results are discarded.

## Startup and execution

`gigpies-headless --measurement-owner ABS_JSON` explicitly enables the software
worker at startup, together with configured topology, reserved measurement slot and
actual module manifest/PA setup. The startup object is exactly:
`{"version":1,"mode":"software-only","owner_executable":"ABS_PATH",
"owner_sha256":"LOWERCASE_SHA256"}`. No environment search or fallback executable.
Default/old PA owners report unavailable. Current headless synthetic mic slot is
zero unless an explicit software witness injects a paired test source; it does not
invent microphone evidence or start physical capture.

The startup-created serial worker verifies the executable before spawning each
bounded job. Pipes are nonblocking, request<=8MiB, owner response<=256KiB, each child
has a5s deadline and is killed/reaped on failure/cancellation. Only the owned child
is affected. Capture has a5s control-time deadline, source copies are preallocated,
and work/completion queues are bounded. The offer path only copies at most48 pairs
and records faults; analysis, hashing, subprocess I/O and buffer retirement occur
outside it. These statements do not claim allocation-free whole-host control ticks.

PA `analyze` and `propose` operations are followed, for proposed/no-change outcomes,
by PA's owner-validated `candidate` operation. Returned exact capture metadata,
finite/bounded spectra, graph identity, sparse changes and protected-field equality
are checked before admission. Use candidate_configuration_json plus the retained
program_buses only through the existing GP14 muted whole-configuration review.
Check current basis again before offering review. Rearm remains a separate explicit
operator action. No-change does not require a redundant configuration transaction.

## Acceptance

Normal GP20 tests cover strict integer outer wire, reserved-slot admission, bounded
exact pair copying, clipping/timeline/cancellation/deadline and unavailable authority.
`gp20_owner` is explicit because it loads independently built trusted owner artifacts:

```sh
GP05_MANIFEST=ABS_MODULES GP20_OWNER_CONFIG=ABS_OWNER GP20_GRAPH=ABS_GRAPH \
GP20_CORPUS_DIR=ABS_PRIVATE_CORPUS cargo test --locked --features hardware-host \
  --test gp20_owner -- --ignored
```

Its deterministic source passes actual PA DSP with known delay/inversion into the
reserved slot, uses two positions, checks owner proposal/candidate/cancellation and
source isolation, and emits actual correlated producer exchanges. Signals are
invented; no hardware endpoints, audible output or clock qualification are involved.
