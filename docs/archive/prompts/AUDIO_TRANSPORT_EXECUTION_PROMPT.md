# Next-thread prompt: synchronized two-Pi audio transport

**Historical execution brief, retained for provenance.** The transport phase is
completed in [its plan](../plans/AUDIO_TRANSPORT_PLAN.md), followed by the qualified stereo
hardware phase. The quoted scope below is not new authorization to rerun it.
Use [the current handoff](../plans/NEXT_SESSION.md). Both Brain consoles now have independent
offline foundations; native/live integration remains separate work.

```text
Work in /home/shome/p/gigpies. Plan and execute the next two-Pi development task:
choose, prototype and verify the synchronized audio/control transport between
the PA/Stagebox and Brain. Use the installed peer communication instead of asking
me to copy prompts between machines. Continue through implementation, experiments,
repairs, verification, documentation and a scoped local commit. Do not stop after
writing a plan, obtaining the first successful packet or finding the first bug.

CONTEXT AND SCOPE

I have disconnected from the other Pi's interactive session. Do not assume the
machine or Ethernet cable is disconnected: check reachability. The peer worker
can start its own session while the Pi is powered and reachable. Do not resume,
interrupt or take ownership of another interactive Codex session.

The target is strictly 48 kHz, 24-bit capture and f64 mixer DSP. PA owns physical
audio I/O, mixing, monitors, protection and local NVMe recording. Brain owns the
full-HD HDMI console, analysis/doctor, lighting coordination and richer FX.
PA sends analysis audio and FX sends; Brain returns wet FX and bounded commands.
Critical PA audio and recording must continue through Brain/network failure.
Physical Pi model assignments remain a measured design choice. Pi 5 coordinates
development and Pi 4 is the peer worker; these are not fixed runtime assignments.

This task authorizes software implementation in GigPies, direct bounded peer
workers, synthetic PCM transport over the dedicated Ethernet link, bounded link
load tests after mutual reservations, application-level fault injection, and
restart/termination of processes created for these experiments. It also authorizes
private ledger commits/pushes and scoped local source commits. Do not publish to
GitHub or create a release. Existing recordings and retained evidence are protected.

Keep sibling source read-only. Keep PA/FX/lighting/recorder algorithms in their
owning projects and document any required owner-side follow-up. Do not implement
the HDMI console, a new mixer, an FX engine or a full recorder as part of this task.
No speaker playback, audio/MIDI/DMX device activation, cable pulls, NIC shutdowns,
reboots, persistent services, clock-daemon changes or system network tuning are
authorized by this brief. Use process-level faults and synthetic clocks first.
If physical clock/audio acceptance needs those operations, finish the independent
software work and identify the exact remaining hardware check and permission.

1. READ THE CURRENT STATE AND ESTABLISH OWNERSHIP

Read /home/shome/p/AGENTS.md, repository AGENTS.md, README.md, docs/STATUS.md,
docs/architecture/ARCHITECTURE.md, docs/architecture/COMPONENTS.md, docs/development/NODE_LAB.md, docs/archive/plans/NEXT_SESSION.md,
docs/architecture/BRAIN_CONSOLES.md and docs/PUBLICATION.md. Inspect live Git status first.
Preserve existing uncommitted console-plan/documentation changes. Do not reset,
stash away, overwrite or absorb unrelated work into your commit.

Determine hostname. Use only the pinned SSH aliases specified by the parent
AGENTS.md. Check the installed /home/shome/.local/bin/gigpies-peer with --check.
In the clean private exchange clone, pull --ff-only and read its README, current
tasks, source-revisions.json and new node records. Reconcile stale prose against
the installed runner and current parent instructions; never rewrite old records.

At prompt preparation, 0001-bootstrap was recorded as accepted and
0002-link-baseline was queued without a measurement reservation. Refresh this;
do not repeat accepted work without a changed condition or missing evidence.
Inventory both nodes: exact source revisions, dirty state, CPU/RAM, kernel,
tools, free disk, NIC/link/MTU, clock/timestamp capabilities and competing work.
Do not equate capability enumeration with working hardware timestamping.

Use the installed helper's prompt interface, not Codex's native -p profile flag:
  gigpies-peer --prompt-file /absolute/path/to/task.txt
For explicitly owned writes, select --write and --cwd for an isolated checkout.
Follow the helper's allowed sandbox and timeout limits. Do not bypass them.
Assign exact task ID, repository/base SHA, owned paths/resources, deliverable,
allowed commands, validation, deadline and stopping rules. Only one receiver may
run on a node; no recursive worker dispatch or Pi-to-Pi loops. Keep prompts and
logs private. Verify results and exact revisions rather than trusting a summary.

If the peer is unreachable, retain the concrete error, attempt only bounded
non-disruptive diagnosis, and continue research/headless implementation locally.
Report the exact reconnect action needed. Never invent remote measurements or
call a local loopback result a two-node acceptance test.

2. WRITE THE OWNING PLAN, THEN EXECUTE IT

Create or update docs/archive/plans/AUDIO_TRANSPORT_PLAN.md as the owning plan. Record current
facts, hypotheses, chosen workloads, numeric acceptance targets, test durations,
failure/stop conditions, responsibilities and the dependency-ordered backlog.
Keep it current after each meaningful result. Planning is the first phase of this
authorized execution; do not request another go merely to start implementation.

Use the existing private ledger as the operational task queue. Continue 0002 if
appropriate and create the next available task IDs for transport work. Require
both nodes to acknowledge the same exclusive resource reservation before load
measurements. Pi 5 owns clients/coordinating results; Pi 4 owns its assigned
bounded servers. Reserve exact ports, interfaces, resources and release conditions.
An idle peer session or the worker lock is not a hardware reservation.

Arrange server readiness without deadlocking dispatch: return a readiness record
for an owned, time-limited process, or orchestrate the helper call asynchronously
while observing readiness. Record PID, bound address/port and cleanup deadline.
Never leave an unbounded background server or kill unrelated processes by name.
On timeout, inspect partial work and the ledger before retrying a mutation.

3. MEASURE THE LINK BEFORE TUNING IT

Execute the existing 0002 baseline under its current accepted reservation and
installed-version commands. Include idle/declared-load RTT, loss, TCP throughput
in both directions, bounded UDP rate steps, CPU load and post-test SSH/Git health.
Preserve its baseline settings; do not silently tune MTU, governors or clocks.
Use existing tools where available. If a missing system dependency cannot be
replaced within scope, name it precisely and continue independent work.

Record durations, sample counts and percentile definitions. Distinguish measured
RTT, packet-arrival variation, buffering, audio frame position and processing time.
RTT/2 is not measured one-way latency. End-to-end latency requires a defensible
common timebase or a same-clock round-trip method with known turnaround behavior.

4. SELECT THE TRANSPORT AND CLOCK MODEL FROM EVIDENCE

Research current primary documentation and inspect relevant existing code. Compare
a small shortlist: RTP/UDP with a defined PCM payload/profile, a minimal custom
UDP transport, and a suitable existing Linux network-audio implementation if it
fits the architecture. Evaluate AES67 interoperability requirements separately
from using RTP; never claim conformance merely because packets use RTP or PTP.
Explain why TCP/QUIC/retransmission is or is not appropriate for each traffic class.
Do not benchmark every available stack: document the shortlist and implement the
best-supported candidate, with one alternative if evidence exposes a real limit.

Select and document each layer separately:
- Audio packet protocol, payload encoding and channel grouping.
- Audio sample-clock ownership and drift handling.
- Optional network-clock synchronization and what it actually establishes.
- Control/status transport, acknowledgment and state recovery.
- SSH/Git development coordination, which remains separate from live transport.

Separate 24-bit capture, f64 internal processing and the wire representation.
Compare packed PCM24 and float32 where useful; quantify precision, bandwidth and
conversion costs. No need to send f64 simply because mixer arithmetic uses f64.
Define byte order, stream/session IDs, channel mapping, source frame counters,
timestamps, sequence/wrap behavior, packetization, MTU-safe sizing and negotiation.
Large channel counts may require channel groups; never assume one whole audio
block fits a datagram or rely on IP fragmentation as the normal operating mode.
Account for actual headers, packet rate and simultaneous send/return traffic.

Use the PA audio sample timeline as the reference design. Explain how Brain FX
follows that timeline and how returned frames map to their intended output time.
Brain processing without an independent audio device must not acquire a second
audio clock merely from a software timer. Where independent clocks really exist,
evaluate bounded buffering and asynchronous sample-rate conversion or another
explicit solution. NTP/PTP time agreement alone does not clock a USB ADC/DAC.
Do not invent a resampler if a reviewed implementation is suitable.

Define jitter-buffer depth/limits, late/lost/reordered/duplicate packet behavior,
clock discontinuities, startup/prefill, stream replacement and reconnect policy.
Control messages require bounded parsing, identity, revisions, idempotent retry
and stale-command refusal. Audio overload cannot starve control or grow queues.

5. BUILD AND VERIFY A SYNTHETIC AUDIO PROTOTYPE

Implement reusable transport code/tests in GigPies and keep disposable experiment
runners/evidence private. Public scripts require a reviewed publication-policy
entry. Use Rust 1.97.1, edition 2024, Cargo.lock and CARGO_INCREMENTAL=0. Preserve
the existing offline CLI and avoid sibling path dependencies. Keep allocation,
locks and I/O out of any future live audio callback boundary.

Generate deterministic per-channel patterns, impulses, tones and frame markers.
Verify channel identity, alignment, ordering, missing/duplicate frames and sample
integrity. Require exact decoded samples for lossless integer transport without
conversion/resampling; declare justified error bounds for other paths. Test the
real-time pacing path, not only a fastest-possible packet sender.

Declare a compact workload matrix before measurement, for example:
- Small profile: 8 mono channels with a wet-return stream.
- Design profile: 64 mono analysis channels plus 8 stereo FX sends and 8 stereo
  returns, with simultaneous control/status traffic.
- Stretch profile: 128 mono analysis channels plus 16 stereo sends/returns,
  attempted only after the design profile passes with headroom.
State the actual direction, encoding, aggregate stream count and packet rate of
each profile. Avoid counting an intentionally shared source subscription twice.
UI channel capacity and synthetic transport capacity do not certify DSP capacity.

Compare only enough packet durations/buffer sizes to choose a defensible setting.
Use brief smoke runs first, then bounded repeatable measurements and an initially
10-minute soak of the accepted design profile. Keep this duration bounded and
describe its limits; it does not establish full-show reliability. If a profile
fails, diagnose it, fix the cause, rerun the affected checks and document the
stable operating envelope rather than silently lowering the original target.

Exercise controlled loss, burst loss, jitter, reordering, duplication, malformed
packets, rate mismatch, sequence/timestamp wrap, stream restart and stalls. Use
in-process injection/synthetic clocks instead of system-wide network/clock changes.
Include realistic small drift offsets in both directions and a declared stress
case; distinguish simulation from measured hardware clock drift.

Measure p50/p95/p99/max timing, packet loss/late/drop counts, buffer occupancy,
drift estimate/correction, memory, CPU, thermal/throttling state where observable,
and network throughput. Use a same-clock echo/turnaround path for round-trip audio
timing when cross-node time uncertainty prevents a one-way claim. Report timing
uncertainty, known turnaround and hardware/software timestamp origin explicitly.

Verify that Brain/transport stalls cannot block the simulated local PA processing
and recorder queues. Test wet-return fade/recovery and fresh-state resynchronizing
without old commands or old audio being replayed. If real PA/recorder integration
does not exist, label this contract/simulation evidence and leave physical audio
and NVMe continuity as a concrete integration gate. Do not claim a full failover
system merely because a synthetic counter continues.

6. FIX, DOCUMENT, CLEAN UP AND COMMIT

Run focused normal tests while changing each path. Run the complete normal
production suite for the shared schemas, transport/concurrency, persistence and
safety changes. Keep long benchmarks, network/hardware tests and historical media
studies opt-in. Do not rerender existing music for unrelated transport work.
State which test classes ran and which were intentionally skipped.

Keep docs/archive/plans/AUDIO_TRANSPORT_PLAN.md current and write a readable
docs/reference/AUDIO_TRANSPORT.md describing the selected protocol, packet/control formats,
clock model, timing budgets, measurements, failures, fixes and acceptance limits.
Update architecture, node-lab, status, next-session notes and documentation links.
Preserve useful superseded drafts in docs/archive. Cite primary protocol/library
sources and distinguish planned, implemented, simulated and hardware-verified work.

Retain concise machine-readable evidence with exact revisions, commands, versions,
conditions, durations and checksums under ignored task artifacts. Record failed
experiments as well as successful ones. Post immutable peer findings and a separate
coordinator acceptance record in the private ledger. A delivered reply or Git push
does not mean accepted evidence. Keep task states honest and release reservations.

Check disk use before substantial builds/generation. Remove this task's disposable
intermediates and stop its temporary processes after preserving useful evidence.
Keep source recordings, original mixes, unrelated edits and other sessions intact.
No broad cache cleanup, home mirroring or private data in source history.

Inspect live Git state and staged content, follow docs/development/PUBLICATION.md, run the
publication guard against the complete index and make a scoped local commit of
verified source/documentation. Do not silently stage earlier unrelated changes.
No GitHub push/release. Synchronize accepted private ledger records as required.

Finish with the commit, selected protocols and why, measured supported workload,
timing/clock limitations, failures fixed, evidence locations and the exact next
hardware acceptance step. If an external blocker prevents a phase, finish all
independent work and report that phase precisely; do not call it passed or leave
an untracked experiment running. Keep concise progress updates during execution.
```
