# Monitor and aux sends — GP18-sends:1

Implemented software routing for configured engines; physical audio, converter
clock lock, listening, network load and sustained realtime deadlines remain
unqualified. Legacy `legacy8-2` keeps GP07-processing:2 and its original raw
monitor behavior. GP18 refuses that topology with `unsupported_topology`, including
reads: use the configured engine to enable selectable taps. Dimensions come from
validated topology, with finite memory/work/snapshot admission rather than a
product monitor count. The existing 128-byte reservation per coefficient is
preserved. Each send adds at most 32 bytes of tap state; a normal regression
checks live ramp/tap state plus both prepared/retiring change matrices against
that reservation. The admission work count measures coefficient visits across
the maximum block, not processor instructions, execution time or deadline proof.

## Exact signal positions

There is no trim. The existing EQ/compressor strip ticks once per input sample.
Each input/destination independently selects one mono signal:

| Tap | Signal before independent send gain |
| --- | --- |
| `raw_post_mute` | raw × common channel mute |
| `processed_pre_fader` | existing EQ/compressor result × common mute |
| `processed_post_fader` | same result × common mute × channel fader |

All taps are before pan. The selected signal is multiplied by that destination's
existing send gain, then summed. Default raw mode preserves the original arithmetic
and bytes. Shared channel mute affects every mode. Existing downstream talkback,
global output safety, FX, PA protection and physical patching keep their order.
REC and named source analysis still consume raw samples. GP03 coefficient vectors
report gains only; they do not report the selected signal or an effective combined
send gain. Read GP18 alongside GP03 at the same revision to describe routing.

## Wire and authority

Requests have the existing eleven-key envelope: `contract`, `version`, `show_id`,
`module`, `epoch`, `writer`, `lease`, `request_id`, `expected_revision`, `kind`,
`body`. Contract is `GP18-sends`, version 1, module `audio`. Counters are canonical
decimal strings. A `sends_snapshot` has empty body and null authority fields.
A `send_tap_set` body contains exactly `input`, `monitor`, `tap`; all authority
fields are mandatory. Input IDs are contiguous `input-01` etc.; monitor IDs are
`monitor-1` etc., with no leading zero. Unknown, missing, duplicate, oversized,
trailing, deeply nested or wrongly typed fields reject before authority admission.
Requests retain the 64 KiB / depth-12 bounds. A syntactically valid out-of-inventory
pair gets a final `target` refusal before admission.

Only that monitor's live scoped lease permits a tap edit. FOH does not. Tap edits
are explicit manual configuration, independent of C-AUDIO send-level modes,
proposals, human holds and review. A level and tap edit are separate transactions;
no combined atomicity is advertised. They share global revision, writer request-ID
high-water and 64-result history with C-AUDIO, GP07, GP05 and external contracts.
Exact retries return the original pending/final result; changed payload or
cross-contract ID reuse refuses. Identity/lease validation precedes cache lookup,
which precedes revision checks. Backpressure consumes no ID or revision.

Fresh actual GP18 observation within 250 ms is required per Unix connection or
authenticated remote session, in addition to the existing raw read/grant path.
Remote permissions cap the selected monitor scope and bind the writer to the TLS
session. Pending source-boundary work is revoked on connection/policy loss, writer
revocation or source recovery. Reconnection starts without freshness or authority
and must use a fresh writer/read/grant; no saved action is replayed. GP15 lease
maintenance treats pending GP18 as busy while preserving its own immutable retry
history, expiry and held-proof semantics.

One prepared transaction applies before the strict next 48-frame sample boundary,
revalidating scope, lease, shared revision and fault state at that boundary. The
240-frame convex crossfade blends the two tap signals before send gain: the
boundary sample is exactly old, the endpoint sample exactly target. Both signals
use the same existing strip result and common mute. No midfade retargeting is
admitted; pending/completion/retirement storage and an active send fade exert
bounded backpressure. Preparation and owned-storage retirement run off render.
The Mixer renderer allocates/frees nothing and performs no locks, I/O or string
validation. Caller partitions preserve every sample. The control pump remains
explicitly outside that realtime renderer.

Replies use the GP07-shaped eleven keys: `contract`, `version`, `context`, `state`,
`reason`, `ticket`, `effective_frame`, `ramp_frames`, `revision`, `snapshot`.
States are `pending`, `final`, `backpressure`; a successful mutation echoes its
nonzero ticket, boundary and ramp 240. Final means applied at boundary, while the
fade may continue. Refusals carry reason and null timing/snapshot. Pressure carries
null reason/timing/snapshot. Successful reads have null timing and a snapshot.

Snapshot keys are `show_id`, `epoch`, `revision`, `sequence`, `frame`, `sample_rate`,
`supported_modes`, `tap_positions`, `monitors`, `faulted`, `channels`. Supported
modes are the ordered table names above; matching position strings are
`raw-shared-mute-mono`, `eq-compressor-shared-mute-mono`,
`eq-compressor-shared-mute-fader-mono`. Channels are ordered `{input,sends}`;
each row has exactly one ordered `{monitor,current,target,
transition_remaining_frames,ready}` per advertised monitor. Current is the last
settled tap, target the committed target; during transition output is a blend.
Remaining is 0..240 at the next-sample frame; ready iff zero, then current=target.
Readback rejects same-tap fades, simultaneous fades, and overflowing or non-48-aligned endpoints; arbitrary midfade read frames remain valid.
Snapshots share provider sequence/revision/frame and immutable GP14 paging
(64 KiB frames, 8 KiB pages, 1 MiB assembly, two-second assembly deadline).

## Processing and persistence compatibility

Configured providers explicitly advertise `GP07-processing:4`, with unchanged
four-band EQ/compressor parameters and FOH semantics, but
`monitor_tap=per-send-gp18-v1`. The version depends on provider topology, never tap
state. Valid unsupported-version GP07 envelopes receive typed
`unsupported_version` without interpreting Config or admitting authority.
Configured GP07:3 is historical and refused; dynamic Desk processing must upgrade
to 4. Legacy GP07:2 and old fixture bytes remain unchanged.

EngineIntent version 2 requires an exact inputs × monitors `send_taps` matrix.
Version 1 accepts only its original shape and migrates every send to raw.
Only committed targets persist, including while fading; pending/draft edits do
not. Restore settles tap targets and starts disarmed, with fresh epoch, no leases,
queues, old request IDs or replay. Composed structural/Brain intent embeds this
versioned engine intent without changing its own version or PA configuration.
Malformed shapes and changed topology require explicit remapping.

## Validation and consumer handoff

Fast normal tests: `sends`, `sends_alloc`, `sends_local`, the authenticated sends
tests in `remote::tests`, and extended `atomic_control`. They independently
separate EQ center gain, compressor attack/static reduction, fader/pan, common
mute/global safety, independent send levels/destinations, exact default output,
transition partitions/endpoints, refusal/dedup/expiry and disarmed persistence.
Allocation/deallocation guards cover success, invalid shape, fault and quiescence.
Actual Unix and mutual-TLS tests use bounded synthetic software only. Mixed
provider tests cover cancellation in both ownership orders, unrelated writer
revocation, cached final retries while another tap is pending, and final-ticket
identity. Composed LocalAudio save/restore retains independent nondefault taps
and rejects authority replay while staying disarmed. A configured 16-input,
40-monitor Unix read exercises the actual oversized immutable GP18 reply and
production client assembly; its byte size and page count are measured by the test.
GP15 pending refusal and subsequent release remain covered by atomic_control.

The synthetic provider generates each containing fouraux block once per tick,
including a second block if source recovery starts unaligned. An independent
scalar reference checks the exact original PCM24 bytes across channels and block
boundaries, including extra-input mapping and permuted capture slots. GP04 child
failures label descriptor/window and prefix/body stages, retain a bounded stderr
tail and always kill/wait the child. Its 100 ms freshness limit and two-second
read timeout are unchanged. These tests establish software reliability and source
equivalence; they do not qualify physical synchronization or realtime deadlines.

The optional actual-owner regression
`actual_monitor_taps_feed_owner_matrix_after_send_gain_without_reprocessing_foh`
uses explicitly supplied hash-verified owner libraries and PA fixtures, assembling
changed monitor buses into the actual PA matrix alongside actual FX. Its artifacts
remain private. Historical research, exhaustive matrices, auditions, long tests
and physical endpoints remain opt-in.

Producer corpus generation is a focused ignored test `write_sends_producer_corpus`.
Its output must name an exact committed source revision, with fixture SHA256
manifest, before consumer freeze. The original `tests/fixtures/gp18/v1` corpus
and its producing revision remain historical evidence. Corrected-source output
is generated separately under `tests/fixtures/gp18/v1-corrected`; verifier selection
uses `GP18_FIXTURES` (defaulting to the historical directory). Coordinator acceptance of the actual diff and
corpus is required before Desk implementation. Consumers pair GP03/GP18 revisions,
keep draft/pending/confirmed state distinct, label current/target blends, review
levels and taps separately, refresh original-context observations before sending,
and drop unsent intent on focus/connection/epoch/generation loss.
