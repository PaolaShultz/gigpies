# GP07 channel processing, version 1

Accepted first GP-07 contract. Software-only, fixed 48000 Hz, eight mono inputs;
no physical/listening acceptance. Existing C-AUDIO:1 and GP03-rendered:1 bytes
remain unchanged. Processing is an explicit `GP07-processing:1` extension on the
same private local endpoint. Older providers reject it; clients mark it unavailable.

## Signal and state

Raw PCM24 REC and named analysis remain before all processing, byte-identical.
FOH: raw -> three-band EQ -> compressor (including explicit makeup) -> shared
mute -> FOH fader/pan -> existing sum -> existing FX -> existing PA.
Monitors: raw -> shared mute -> existing independent sends. Advertised processing
tap is `foh-post-eq-dynamics-v1`; monitor tap is `raw-post-mute-v1`.
No monitor selector, latency/lookahead, normalization or automatic makeup.

Reuse GigPies automix Biquad and compressor static law. Fixed storage; prepare
coefficients off render. Independent EQ and compressor bypass default true.
EQ bypass removes all three filters; compressor bypass removes reduction and
makeup. Neutral enabled EQ is identity; ratio1/makeup0 compressor is identity.
The compressor uses instantaneous mono magnitude, floor -240 dBFS, static soft
knee and existing dB reduction attack/release recursion. Gain reduction excludes
makeup and is positive attenuation, never an output-level meter.

Complete channel replacement commits at the strict next 48-frame boundary.
Old and freshly prepared target branches run with a linear output crossfade over
240 frames: boundary sample old, endpoint sample exactly target. Target filter
and detector histories start zero; old histories continue during the fade. No
coefficient interpolation. The local service always renders48 frames per tick.
The offline Mixer accepts equal-length caller-owned slices, with fixed per-sample
work and checked frame-counter growth; partitioning the same samples is equivalent. A new uncached processing edit while any processing fade is active
returns backpressure without consuming an ID; retry after fresh ready status.
Identity/live lease checks and exact pending/final retry lookup precede this check.
This bounds storage and prevents discontinuous midfade retargeting. Unchanged
config is a no-op DSP transition but still a valid authority transaction. No heap
ownership/destruction, locks, I/O or unbounded work in Mixer::process. Partitioning
must not affect output. Nonfinite source/arithmetic latches the existing zero-output
fault. Engine restart resets histories/settings to defaults; no automatic replay.

## Parameters

All values integers, all fields mandatory; booleans only for bypass. Stable IDs
are input-01 through input-08. One complete channel per atomic edit.

`config` has exactly `eq_bypass`, `low_hz`, `low_gain_mdb`, `mid_hz`,
`mid_gain_mdb`, `mid_q_milli`, `high_hz`, `high_gain_mdb`, `compressor_bypass`,
`threshold_mdb`, `ratio_milli`, `knee_mdb`, `attack_us`, `release_ms`, `makeup_mdb`.

| Fields | Range, inclusive | Default |
|---|---|---|
| eq_bypass, compressor_bypass | boolean | true |
| low_hz, mid_hz, high_hz | 20..20000 Hz, integer | 120, 1000, 8000 |
| low_gain_mdb, mid_gain_mdb, high_gain_mdb | -12000..12000 milli-dB, step100 | 0 |
| mid_q_milli | 100..10000, step100 | 1000 |
| threshold_mdb | -60000..0 milli-dBFS, step100 | -18000 |
| ratio_milli | 1000..20000, step100 | 1000 |
| knee_mdb | 0..18000 milli-dB, step100 | 6000 |
| attack_us | 100..200000 microseconds, step100 | 10000 |
| release_ms | 10..2000 milliseconds, step1 | 100 |
| makeup_mdb | -12000..12000 milli-dB, step100 | 0 |

Low/high are RBJ S=1 shelves; mid is bell. Makeup applies after compression and
only when compressor enabled. No separate output trim in this increment.

## Strict wire and authority

Requests have exactly the C-AUDIO envelope keys, with `contract` =
`GP07-processing`, `version` = 1 and `module` = `audio`. `kind` is
`processing_snapshot` with empty `body` and null writer/lease/request/revision,
or `processing_set` with body `{ "input": "input-01", "config": CONFIG }` and
all four authority fields present. Counter encodings and identity validation
are unchanged. Duplicate/unknown/missing keys, wrong integer types, invalid ranges,
nonfinite/trailing/oversized/deep input reject atomically. 64KiB/depth12 bounds.

Snapshot proves capability; a client must receive it before exposing edits.
Mutation requires the connection's existing fresh C-AUDIO FOH grant and fresh
processing snapshot. It shares C-AUDIO/GP05 writer high-water/dedup history, global
revision and one pending renderer transaction. Scope is FOH regardless of modes;
these are explicit manual processing settings, retained across mode/hold changes.
Processing automation/proposals are not advertised. No implicit lease renewal.
Identity/live lease before cache, cache before revision; changed payload or cross-
contract reuse refuses. Backpressure is not admission and consumes no ID.
At boundary revalidate lease/revision; cancelled/refused processing leaves DSP and
revision unchanged. Successful commit increments shared revision exactly once.
New terminal `stale_snapshot`/`faulted` refusals consume their shared ID without
changing revision or extending a lease; malformed codec input is not admission.
Connection loss alone does not cancel admitted work. Reconnect drops pending and
unsent intent; fresh snapshot/new writer/grant, no replay.

Reply exact keys: `contract`, `version`, `context`, `state`, `reason`, `ticket`,
`effective_frame`, `ramp_frames`, `revision`, `snapshot`. Contract/version as request;
context has exactly existing GP03 RequestContext keys and echoes full identity.
State `pending`, `final` or `backpressure`. Reason null on success/pending/pressure;
otherwise bounded lowercase ASCII/underscore refusal string, length1..64
(same authority reasons plus `stale_snapshot` and `faulted`). Ticket/frame
canonical decimal strings or null; ramp_frames 240 or null; revision canonical
string. Pending has timing, no snapshot. Pressure has no timing/snapshot. Final
successful set preserves pending timing and includes snapshot; final refused has
reason and no timing/snapshot. Snapshot success final has null timing and snapshot.
A final means boundary applied, not crossfade finished. Old replies cannot refresh
or regress newer readback. All identity/timing/revision coherence is validated.

Snapshot exact keys: `show_id`, `epoch`, `revision`, `sequence`, `frame`,
`sample_rate`, `foh_tap`, `monitor_tap`, `faulted`, `channels`.
Sequence shares monotonic provider observation sequence; frame is next sample.
Rate48000 and tap strings above are exact. Channels contains eight ordered objects
with exact keys `input`, `current`, `target`, `transition_remaining_frames`,
`ready`, `gain_reduction_mdb`. Current is last settled CONFIG; target is committed
CONFIG. During fade current stays old (the actual output is a blend); remaining
0..240 is measured at snapshot frame; ready iff zero. Current equals target when
ready. GR is nullable integer0..240000 milli-dB: target branch detector reduction
rounded to nearest and saturated at240000 for reporting only (DSP attenuation is
not clipped); this is bounded detector feedback, not a calibrated input meter.
Valid only when ready, compressor enabled and not faulted;
otherwise null. UI must label null bypassed/transition/unavailable as applicable.
Freshness250ms local receipt, frame/sequence monotonicity required. No signal
meters are invented. Faulted processing disables editing until explicit recovery.

## Operator acceptance

Desk Channel page edits a complete local draft, explicit Apply/review/confirm or
Cancel. Display units Hz/dB/Q/ratio/ms and bypass; provider-confirmed config and
transition/GR are separate from draft/pending/failed. Selection/page/bank/revision,
role, connection and generation fence drafts and confirmations. Keyboard and
injected controller use the same semantic actions. Physical devices remain closed.
Producer fixture version `tests/fixtures/gp07/v1` is frozen from real owner code
before consumer decoding acceptance; record file SHA256 and provider source pin.
Real frontend/provider/sample tests must prove changed FOH, second-channel
isolation, monitor independence, exact raw REC/analysis, FX/PA order and reconnect.

## Software acceptance — 2026-10-05

The real Desk frontend drove the actual provider through keyboard and injected
controller actions, confirmed channel 1 EQ/compression with positive detector GR,
then changed channel 2 independently. Context cancellation and reconnect retained
revision 2 without replay, including the subsequent lease-expiry interval.

The joint `gp07_frontend` test compared 1,698 blocks against eight independent
single-input mixer references, applying the observed configurations at their exact
boundaries. The entire FOH sample timeline matched bit-for-bit. Monitors matched
the neutral reference every block, and the actual FX wet-plus-dry → PA libraries
matched the observed module output. Raw analysis matched before, during and after
editing. All eight PCM24 stems matched 81,216 source frames each, covering the
complete frontend run, with complete finalization and no gaps, drops or faults.
Timing-dependent counts are evidence for this run, not fixed test thresholds.

The real release-provider Desk run also passed independently. Its complete
1920×1080 processing scene was visually checked for confirmed/draft separation,
units, readiness, GR and unavailable meter labels. These are synthetic software
checks; no physical audio, controller, display or listening acceptance is implied.

Normal regressions cover independent EQ frequency/impulse and compressor-law/
ballistics references, neutral/bypass, channel isolation, transitions/reset,
partitioning, nonfinite/extreme inputs, allocation-free processing, strict schemas,
shared IDs/authority/freshness and exact producer-fixture replay. See
[development](DEVELOPMENT.md) for the opt-in actual-owner/frontend commands.
