# Audio hardware execution plan

**Live latency acceptance is unmet.** The user rejected the 64 ms buffer / 56 ms
prefill configuration as unsuitable for live use. Its clean soak is retained as
a correctness benchmark. The next work fixes partial-read pacing and tests the
smallest practical period, capacity and prefill; it must pass the same continuity
and recovery checks before live-latency acceptance.

Owning continuation of the completed [transport phase](AUDIO_TRANSPORT_PLAN.md).
Started 2026-10-03 from local GigPies `ac5c436`. Prior console documents and
archive drafts are preserved separately. Source commits remain local.

## Scope and ownership

Pi 5 coordinates and exclusively owns the selected USB PCM device. Pi 4 runs
source-following FX, with no second audio clock. GigPies owns host integration,
GPA1/control and evidence. SHR PA owns processing; SHR FX owns wet DSP; SHR REC
owns recording, file integrity and recovery. Each module has one assigned worker.
Independent versioned C interfaces connect separately built libraries; no sibling
path dependencies or duplicate DSP engines. No changes to TV, Bluetooth, MIDI,
HDMI, NICs, clock services or unrelated sessions.

## Initial device and connection facts

Pi 5 exposes PreSonus AudioBox USB 96, USB 194f:0303, ALSA ID A96: two capture
and two playback channels, S32_LE with 24 descriptor-declared bits, FL/FR, native 44.1/48/88.2/96
kHz. Both streams were stopped; WirePlumber held only its control device.
Pi 4 exposes onboard/HDMI playback only. The Superlux microphone is absent on
both nodes. No electrical/acoustic return connection is established. The user
has disconnected the amplifier. Enumeration does not establish analogue wiring,
converter lock or latency. Do not route live capture to outputs automatically.

## Hypotheses and gates declared before measurements

1. Explicit raw USB 48 kHz, stereo full-duplex operation can preserve 24 valid
   capture bits and source-frame continuity. Negotiate exact format/rate and
   record actual period/buffer; begin with 192 frames (4 ms), four periods.
   Compare smaller periods only if justified. Zero xruns, gaps and file errors
   is the acceptance target. Generated output starts at −36 dBFS.
2. Existing PA DSP can process local dry audio while independent recorder and
   network workers run. The bounded render section must have no allocation,
   locks or I/O. ALSA transfers belong to the host driver outside that section.
   Require render p99 <50% of period and maximum <one period; measure transfer
   waits, scheduling and output queue delay separately.
3. Real stereo FX follows capture frame IDs, using 48-frame GPA1 packets.
   Initial transport admission remains 384 frames (8 ms); a late return must
   fade on both channels. FX intentional delay is separately declared by its
   adapter. Account for capture batching and DAC buffering in the host budget;
   physical round-trip latency remains unresolved without a return route.
4. Loss, malformed/duplicate/stale packets, a bounded Brain stall and an actual
   Brain process restart must leave raw recording and local dry processing
   intact. Check actual files/sample hashes, all-channel fade and fresh control
   snapshots; increasing counters alone are insufficient evidence.
5. After 3–10 s smoke and 30 s fault trials, run one bounded 600 s actual-device
   soak with real PA/FX/recording and no competing builds. Two physical channels
   only. Target zero unexplained xruns, recorder drops/errors and late returns.
   Preserve any failures and rerun only affected checks after diagnosis.

## Reservation and evidence

Private task 0004 records mutually acknowledged reservations before measurements.
H1 reserves the selected Pi 5 USB device and Ethernet UDP45300/45301 through
18:30 UTC, subject to fresh conflict checks; each run is at most 660 s.
Private commands, source/binary hashes, device metadata, original captures and
failed evidence stay under `artifacts/audio-hardware/2026-10-03/`.
Recordings are sized before trials; initial free space is 39 GiB and GigPies
target 4.5 GiB. Preserve existing recordings/evidence. Build with Rust 1.97.1,
locked dependencies and incremental disabled. Normal full suites follow shared
concurrency/engine changes; historical music/auditions remain opt-in.

## Remaining physical gates

No second USB device means independent converter-clock drift cannot be measured.
No return connection means analogue channel identity and capture-to-output
latency cannot be accepted. No calibrated microphone/connected speakers means
acoustic/PA alignment and speaker protection cannot be accepted. Continue all
independent software and ADC/DAC measurements without waiting for repatching.

Results and supported configuration belong in [AUDIO_HARDWARE.md](AUDIO_HARDWARE.md).

## Execution checkpoints

H1 received a hash-verified Pi 4 acknowledgment and separate Pi 5 acceptance.
This hardware task uses isolated candidate snapshots and exact
binary/library hashes to identify the code actually executed on Pi 4. A separate
operator-authorized task 0005 later synchronized peer module checkouts; those
checkout revisions do not identify the isolated binaries used by this task.

The PA/FX/REC adapters and optional ALSA host are implemented. Module normal
suites passed (PA 65 Rust/4 Python, FX 99 Rust, REC 23 Rust), as did 180 GigPies
Rust and 37 Python tests, formatter, warning-denied Clippy and release builds.
Four GigPies opt-ins and one unrelated FX cost matrix remain intentionally skipped.
PA is native f64; FX reuses its existing f32 engine and declares 960 frames of
intentional delay. This precision limit is retained explicitly.

Actual 3 s generated and 10 s capture-plus-probe smokes passed. Two fault trials
were bounded at 15 s instead of the initially proposed 30 s: each completed its
programmed events, fade and sustained recovery. A 250 ms Brain stall, controlled
loss/duplicate/wrong-epoch packets and actual Brain termination/replacement all
preserved exact recorded samples and dry replay, with no device xruns. A deliberate
100 ms driver stall produced the expected xrun and an explicitly incomplete take.
A fresh 30 s session then passed with exact raw/dry/combined-output samples and no
missing wet packets, establishing explicit restart with a new epoch.

Review fixed direction-specific USB precision validation, separate frame counters,
allocation-free render fault returns, complete host-service accounting and fresh
control-state gating. A metric defect in candidate v3 reported the maximum when a
percentile exceeded its 10 ms histogram range. Candidate v4 reports that percentile
as unavailable; original maxima, overflow counts and failed evidence are retained.

Pi 4's independent static/evidence review passed with stated limitations. The
first 600 s actual-device soak retained 28.8 million frames without xruns or
recorder drops, but one of 600000 wet returns expired: **the original zero-loss
8 ms integrated target failed**. All packets arrived; maximum RTT was 7.717 ms.
Four-millisecond capture batching and host wakeup variation consume additional
budget. Candidate v5 explicitly selects **768 frames / 16 ms return admission**
for comparison and a new bounded soak. The intentional FX delay remains 20 ms;
physical latency is still unmeasured. The failed 8 ms result is preserved.

A separate two-second native S32 capture investigated nonzero low container bits.
USB descriptors say 24 bits, while ALSA's negotiated significant-bits query
returns 32. The low byte contained only 0 or 3; conversion to PCM24 by rounding
matched upper-24-bit extraction for every sample in that short capture. This is
not evidence of a 32-bit converter. Candidate v5 explicitly extracts the upper
24 bits and adds an independent byte-domain ADC hash, so unused bits cannot
influence raw storage or source arithmetic. The native S32 recording is retained.

No physical return/second interface exists, so those affected measurements remain
unavailable. Candidate v5 normal validation and affected hardware checks precede
final acceptance; no full-show or acoustic claim follows from this bench.

A later period 192/buffer 768 attempt at 16 ms admission stopped after 10.28 s on a
playback underrun. No wet loss occurred; all 493440 retained frames verified, while
only 493248 frames were fully submitted to playback. The incomplete take and
supervisor cleanup failure are preserved. The latter is fixed in the private
runner; no host binary changed. This is an unresolved scheduling/device-budget
failure, not evidence that 16 ms transport admission failed.

Both nodes then acknowledged period 384/buffer 1536 (8/32 ms), retaining 16 ms return
admission and the exact v5 host/modules. A 30 s comparison passed sample/file and
zero-xrun/zero-wet-loss checks. The new 600 s run completed 28.8 million frames with no xruns, queue drops
or missing/expired wet returns; independent file verification follows. No OS scheduling or device-control
settings were changed. The original smaller-buffer target is not accepted.

The initial owner adapter reused f32 FX arithmetic behind an f64 interface. To
complete the requested DSP precision target, the owner is specializing its shared
existing delay implementation for native f64 state/arithmetic while retaining the
rack's f32 specialization. No duplicate delay algorithm or wire-format change is
needed. The initial f32 library is archived under its original hash. New normal
FX checks, sample replay and affected hardware trials must identify the new
library separately; old hardware evidence cannot be relabelled native f64.

The native f64 smoke and packet-stall test passed, but the 32 ms-buffer Brain
restart trial stopped at 5 s on another playback xrun. All 240384 stored frames
verified; only 240000 were fully submitted. This target remains failed. Candidate
v7 explicitly accepts four/eight device periods, with a tested bounded selector;
it adds failed-playback-cycle timing and flushes the selected buffer length.
The revised bench uses 8 periods/3072 frames/64 ms, retaining 8 ms processing period
and 16 ms wet admission. Additional device latency is an explicit tradeoff.
H2 requests the same resources through 19:00 UTC, <=660 s each, with fresh peer
acknowledgment and full normal host checks before new hardware tests.

The complete v7 host suite passed 181 normal Rust tests; four unrelated opt-ins
remain skipped. A final report-only fix makes both ADC full-scale endpoints
independent of unused low container bits. The focused precision tests and Clippy
cover that predicate; the final release is rebuilt before hardware use. The change
does not alter recorded samples, DSP or output protection. Earlier quiet captures
have no samples near either endpoint, so their absence-of-clipping result stands.

## Completed native f64 bench gate

The final v7 384-frame period / 3072-frame buffer / 768-frame wet-admission
configuration passed the 600 s gate with 28.8 million frames, all 600000 returns,
zero detected xruns, missing/expired wet packets or queue errors. Independent
verification matched direct ADC bytes, eight stored stems, every journal frame
and complete dry/combined-output replay. Render p99/max 1.123/4.949 ms and
post-read service 1.290/5.233 ms satisfy the declared render gate for this run;
RTT p99/max 0.565/1.977 ms. Earlier smaller-buffer and 8 ms wet failures remain.

Native f64 FX is complete at 6510ead; the final normal suites total 370 Rust and
41 Python tests. The relevant FX cost matrix ran after shared primitives changed;
its heavy sixteen-hall case remains failed. Four unrelated GigPies opt-ins stayed
skipped. Final source manifests distinguish prebuild and measured v7 binaries.

After the soak, the user connected both outputs to inputs, minimum input gain,
Mixer Playback and Main noon. H3 separately acknowledged generated-only physical
checks through 19:15 UTC, <=30 s each, beginning at −72 dBFS with an input-level
stop and level review before increasing. These new results are recorded separately
in AUDIO_HARDWARE.md; they do not retroactively add a return path to old trials.

H3 physical checks established the left output-to-input route. Two independent
pseudorandom bursts measured 2739 frames / 57.0625 ms of buffered reference-to-ADC
offset. The integrated generated-only 12 s take passed all 576000 frames and
file/replay checks. Right return was 68.77 dB weaker; the user suspects the cable
and directed use of the working channel. Physical acceptance is limited to
channel 1, with isolated converter latency and clock lock still unmeasured.

## H4: smallest-buffer acceptance after user correction

The user requires a live engine optimized for the lowest practical buffers and
explicitly rejects solving underruns by filling a large device buffer. Preserve
all earlier evidence; its digital correctness does not satisfy live latency.
H4 reserves the same USB device and peer endpoints through 20:15 UTC, with
physical verification restricted to working channel 1. Start at 48-frame periods,
96-frame capacity and 48-frame silence prefill. Increase a budget only after a
measured failure; test device capacity, prefill and wet admission separately.
Short trials are at most 30 s, followed by a 600 s accepted-candidate soak.
Target measured local dry buffered loopback at most 10 ms, seeking 5 ms or less.

The pacing repair waits for a complete capture block before consuming partial
availability and retains typed ALSA I/O handles after setup. A deterministic
regression reproduces the previous missed-period mechanism. Playback starts only
after the first captured/processed block is queued behind the explicit silence;
initial playback occupancy is prefill plus one period. Ring capacity therefore
is not the same as queued latency. The direct path has no additional application
playout queue; recorder/network queues remain independent consumers.

The host now permits whole-packet wet budgets of 1–16 ms, at least one capture
period. Both endpoints use the new binary: the unreleased GPA1 validator's lower
bound changes from 192 to 48 frames; older endpoints reject the smaller values.
The 20 ms first echo in the chosen test effect is intentional and remains separate.
A quiet deterministic coded probe emits on channel 1 only and refuses capture
monitoring. Correlate its real stored ADC and DAC/source stems through the actual
PA/FX/REC host, without treating a standalone probe as the integrated latency.

Run full normal checks and source review before new hardware. Begin with ordinary
scheduling; only if measured need justifies it may the owned audio thread use
FIFO priority 20 under H4, with recorded permission/settings and restoration by
thread exit. No system-wide scheduling or service changes. On the first xrun or
conflict stop the trial, retain the incomplete take and diagnose before retrying.

The first H4 trial at 48/96 frames with one silent period failed after 528 frames:
capture EPIPE, render max 0.105 ms, write wait max 1.404 ms. Retained samples and
journal/replay remain exact but incomplete. Capture-first startup already queues
a processed block, so the silent block filled the two-period ring. The separately
acknowledged H4 amendment permits zero silent prefill; the next candidate starts
with one actual processed block queued, retaining the same two-period capacity.
V9 also removes per-second JSON allocation from the audio thread and distinguishes
configured startup queue size from successfully queued frames in failed reports.

At zero silent prefill, the two-period ring still failed after 816 frames. Its
write wait reached 1.401 ms despite 0.105 ms maximum render time. A three-period
ring first passed 8 s with exact recorded output and no wet loss. Channel-1
physical correlation gave 5.19–5.35 ms in two trusted one-second windows; another
window had weak correlation, so this is not a stable-delay or clock-lock claim.
The following 30 s attempt stopped after 108672 frames with a capture xrun;
render reached 2.333 ms versus 0.113 ms p99, while write wait stayed below 0.034 ms.
All retained ADC/stem/dry hashes and journal frames still match; take incomplete.

V10 therefore adds an explicit optional `audio_fifo_priority: 20`. It applies
only to the calling audio thread after recorder/network workers exist, before
PCM starts, and restores the saved policy after PCM stops and before worker
joins. Other values fail validation; missing/null leaves scheduling alone. A
failed activation or restoration is a run fault, not a silent fallback. H4
already permits this scoped comparison after an observed ordinary-scheduler
failure. Keep the three-period ring and zero prefill for the first comparison.

FIFO20 at 48/144/zero prefill passed 30 s without xruns, but 1 ms wet admission
missed two returns. The 2 ms comparison missed four; its physical lag stepped from
257 to 364–365 frames around 18–19 s, with the playback-delay snapshot rising
from 166 to 287 frames. This is a physical timing failure despite clean ALSA
xrun counters and exact software stems. The burst probe was silent during part
of that interval, so it cannot prove continuity through the transition. The
4 ms comparison then hit a playback xrun after 317904 fully written frames.
One-period prefill passed 8 s but overran after 1351152 frames in its 30 s follow-up.
No low-latency reliability gate has passed yet.

H5 proposes the same minimum-buffer trials through 21:00 UTC, with a continuous
quiet channel-1 reference after startup and bounded preallocated event timing.
Record read/render/write wall and thread-CPU time, both PCM queue observations
and failed transfer stages. This distinguishes expensive DSP from time spent
waiting or descheduled without claiming converter timestamps. Compare explicit
CPU affinity for the audio thread only (CPU3 from its existing allowed mask),
saving/restoring the previous mask and scheduling policy. Existing USB IRQ
counters are concentrated on CPU0; affinity does not isolate a CPU or relocate
interrupts. No global or persistent tuning is authorized or needed. Fresh peer
acknowledgment, source review and full normal checks precede new hardware.

H5 was mutually accepted through 21:00 UTC. Candidate v12 passed source review,
200 normal Rust tests, formatting, Clippy and release build. At 48/144/zero prefill,
30 s runs with and without CPU3 affinity passed; the CPU3 fault trial then overran
capture with a 3.558 ms read wait. Two-period CPU3 failed through blocked playback
writes and growing capture backlog. Four-period capacity, still 48-frame blocks
and zero prefill, passed packet/Brain-stall, Brain restart, deliberate device-fault
finalization and fresh 30 s recovery. Physical offset stayed at 249 frames/5.1875 ms.
Its following 600 s attempt failed at 36.914 s: render wall 3.597 ms versus thread
CPU 0.191 ms, followed by playback xrun. All 1771920 recorded frames verified;
1771872 were completely written. H5 establishes short low latency, not reliability.

H6 replaces the released H5 audio reservation, mutually acknowledged through
22:00 UTC. Retain the same USB identity, ports, channel-1 reference, gain controls,
level guard, minimum-buffer comparisons and fault bounds. Add render-only
RUSAGE_THREAD fault/context-switch deltas and an optional owned-process memory
lock with verified initial/final state. Do not assume the cause of off-CPU time.
Optional process-filtered perf diagnostics are bounded to 60 s and labelled for
their overhead. No global tracing, permissions, IRQ, governor, scheduler, memory,
TV/Bluetooth or service changes. Review implementation and run full normal tests
before fresh short trials; any 600 s retry must fit its full bound before expiry.

H6 traced a 6.219611 ms wait for page migration inside a PA linkage-entry data
access despite process memory locking. This is a kernel blocking observation,
not measured DSP computation or evidence of disk swap-in. H7 separately reserved
one temporary kernel comparison through 22:00 UTC: with other locked-memory
owners absent, change only `compact_unevictable_allowed` from 1 to 0 while the
owned process locks its memory, then restore 1 after each bounded trial. This
explicit exception supersedes H6's no-global-setting rule only for that key.
The peer rejected the first helper's signal handling before use, then accepted
the corrected helper and lifecycle checks; the coordinator separately accepted.
All other settings and the v13 runtime are unchanged. Acceptance requires actual
memory locking, final locked memory zero, helper exit zero and confirmed key 1.

The first 48/192/zero-prefill 30 s trial passed with all physical windows at
249 frames (5.1875 ms). The 48/144 comparison also passed software checks, but
three startup windows weakened while the physical offset settled at 257 frames.
The 192-frame ring is selected for recovery and soak because its observed delay
is lower and stable, while processing stays at 48 frames with zero silent prefill.
The prior 96-frame failure was blocked playback and capture backlog; H7's memory
comparison does not repair that synchronous transfer limitation. No extra
application playout queue is introduced.

H7 completed the ten-minute local-device gate: 28.8 million exact dry/recorded
frames, zero xruns/queue drops, and all 11977 physical windows at 249 frames.
No observed render fault/context switch recurred. Its integrated 4 ms wet gate
failed with two missing/expired returns, RTT maximum 4.156 ms, and 189 recorded
DAC sample differences from uninterrupted FX replay. Keep this failure explicit.
H8 received a mutually accepted reservation through 22:30 UTC with only wet
admission revised
to 288 frames / 6 ms. The dry period, capacity, zero prefill, native f64 libraries
and v13 runtime stay fixed. Original settings were restored after H7. New private
supervisors preserve the prior scripts and change only labels/name paths/expiry;
the same five fake-key restoration lifecycle tests passed again.

H8 finished: short comparison, packet/stall, Brain restart, intentional device
xrun/incomplete finalization and fresh recovery all passed their scoped checks.
The 600 s take retained 28.8 million exact frames with zero xruns, late/missing
wet returns, network errors or queue drops. Physical delay was 249–251 frames
(5.1875–5.2292 ms), with two weak 100 ms windows around one-frame changes.
Both neighborhoods were examined at 10 ms resolution; exact physical continuity
and the cause remain unresolved, and the original review flag is preserved.
This passes the bounded USB/software/wet gate with a qualified physical latency
measurement, not a perfectly fixed-offset or full-show gate. Original settings
were restored and both-node resources released before 22:30 UTC. Final peer
review and local documentation commits close the handoff without further audio.
