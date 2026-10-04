# Next-thread prompt: audio host integration and connected-device measurements

**Historical execution brief, retained for provenance.** The corresponding
stereo host phase is recorded in [the hardware plan](AUDIO_HARDWARE_PLAN.md) and
[measurements](AUDIO_HARDWARE.md). The quoted authorization below applied to that
session; this file does not open a new hardware scope. Use [the current handoff](NEXT_SESSION.md).

```text
Work in /home/shome/p/gigpies. Continue the completed two-Pi transport work into
real audio-host/module integration and measurements with the connected USB audio
hardware. Work autonomously while I watch TV. Do not ask me questions, request
another go, or wait for me to choose tests or implementation details. Make
reasonable engineering decisions, investigate, implement, measure, fix, verify,
document and make scoped local commits. Keep concise progress updates.

CURRENT AUTHORIZATION AND SCOPE

The soundcard is disconnected from the amplifier. I authorize actual playback
and recording through the connected USB audio devices for this task, including
bounded generated signals, microphone captures, full-duplex tests and useful
two-card measurements. This supersedes the previous task's synthetic-only audio
restriction. Do not request another playback/recording confirmation.

I also have a USB class-compliant Superlux condenser microphone with headphone
output, effectively another input/output soundcard. Discover whether it is
connected and its exact capabilities; do not assume its model, rate, bit depth,
clock behavior, gain controls or an existing loopback connection. Its input is
noisy and I cannot reduce its gain. Use it where useful for real capture,
independent-device clock observations or a second output. Digital attenuation
cannot repair ADC clipping or establish that the analogue input gain changed.
It is not a calibrated measurement microphone. Keyboard/singing applications
are a different project and are outside this task.

Leave Bluetooth disconnected from these Pis so I can keep watching TV. Preserve
TV audio, unrelated applications and existing interactive sessions. Select USB
devices and channels explicitly; do not send test audio to the system default,
Bluetooth or HDMI. Start generated signals at a conservative level with bounded
durations, then choose levels from the measurement. Avoid an accidental live
microphone feedback route. No listening preference or manual action from me is
needed to proceed.

For this continuation, necessary scoped implementation changes are authorized in
GigPies and the owning /home/shome/p/shr-pa, shr-fx and shr-rec repositories.
Read their instructions and preserve their existing work. Other siblings remain
read-only. GigPies owns transport/contracts and integration; PA, FX and recording
algorithms stay in their owners. Reuse their existing code and expose the smallest
useful interfaces. Do not build duplicate engines in GigPies or turn this into a
DAW, console, MIDI or lighting project. Keep independent builds and avoid sibling
path dependencies and machine-specific dependency paths in committed code.

Authorized operations include bounded peer workers, fresh mutually reserved
Ethernet/audio experiments, project builds, necessary temporary settings on the
selected devices, application-level faults, termination/restart of task-owned
processes, private ledger commits/pushes and scoped local source commits. Preserve
and restore temporary settings changed by this task without overwriting later
user changes. Do not publish to GitHub or make releases. Do not reboot, change
NICs/host clock daemons, install persistent services, kill unrelated processes or
reconnect Bluetooth. Arrange checkouts and coordination internally; do not ask me
to manage clones or copy messages.

If a device, physical cable or route is unavailable, record that exact limit and
continue every independent implementation and measurement. Do not wait for me
to plug in or repatch hardware. Stop only the affected experiment on a concrete
conflict or failure; keep the rest moving. Never label an unavailable physical
measurement passed or replace it with a synthetic result under the same name.

START FROM THE ACCEPTED STATE

Inspect live Git state and hostname first. Read /home/shome/p/AGENTS.md, repository
AGENTS.md, README.md, docs/STATUS.md, docs/COMPONENTS.md, docs/ARCHITECTURE.md,
docs/NODE_LAB.md, docs/NEXT_SESSION.md, docs/AUDIO_TRANSPORT_PLAN.md,
docs/AUDIO_TRANSPORT.md and docs/PUBLICATION.md. Read owning module documents
before changing each module. Earlier console documentation and archive drafts
are still uncommitted; preserve them and exclude them from new commits.

Accepted GigPies commit: ac5c436326c9c4d12da16c3d1cbf92fc1ca8148a, local only.
Refresh this against live history. Tasks 0001, 0002 and 0003 are accepted in the
private ledger. Last recorded coordinator acceptance was ce9a3f2; refresh the
ledger rather than assuming it has not advanced. All old reservations are released.

Selected protocol: GPA1 UDP audio with packed PCM24 analysis, float32 FX sends
and wet returns; separate GPH1/GPC1/GPK1 acknowledged UDP control. PA owns the
48 kHz source-frame timeline. Brain without its own audio device follows source
frames. A second soundcard is not required on Brain just to run effects.

Observed synthetic design envelope: 64 analysis channels + 16 FX sends/16 wet
returns, 1 ms packets, 8 ms return admission, 600 s. All 7.2 million forward and
2.4 million return packets arrived with exact samples and no late admission.
RTT p99 .362 ms, max 4.031 ms. The software playout worker was up to 4.410 ms
late separately: this does NOT establish physical output at 8 ms. Stretch
128+32/32 failed 203 deadlines at 8 ms. Its 16 ms/30 s rerun is only a short
observation. Keep these failures and limits explicit.

Existing code includes bounded queues, jitter admission, wet fades, source epochs,
control retries and fresh-state recovery. Fixes addressed socket capacity,
mutable parsed metadata, stale ACK identity, exhausted retries, RTT accounting
and overdue admission. The optimized experiment imported real transport modules
but used identity FX and recorder block IDs. It did not run real PA/FX DSP,
USB capture/output or an NVMe audio recorder. Do not repeat unchanged baseline
benchmarks or relabel that old evidence as hardware acceptance.

Retained evidence: artifacts/audio-transport/2026-10-03/README.md,
completion.json, measurements.json, retained-manifest.json and peer reviews.
173 normal Rust tests, 37 Python tests, formatting, Clippy and release build passed.
At handoff target was about 4.5 GiB and free space 39 GiB; refresh before builds.

PEER COORDINATION

Use the installed /home/shome/.local/bin/gigpies-peer and private task ledger.
Check the runner and use only pinned gigpies-pi4/gigpies-pi5 SSH aliases. Refresh
clean ledger clones with pull --ff-only; preserve dirty clones and immutable
records. Assign the next available task ID, expected 0004, after checking.

Pi 5 coordinates/integrates; Pi 4 receives exact bounded assignments with owned
paths/resources, repository/base revisions, validation, deadline and stopping
rules. One peer receiver per node, no recursive dispatch, no takeover of an
existing interactive session. Keep prompts/logs private. Obtain both nodes'
acknowledgments of fresh reservations before link/device experiments. Include
exact devices, ports, routes, duration and cleanup; a worker lock is not a device
reservation. Do not reuse the old scripts' expired absolute deadlines.

The preceding Pi 4 worker could write ordinary files but its sandbox blocked Git
metadata and host network inspection. Its evidence is in
/home/shome/p/gigpies-audio-exchange. The established fallback was peer-owned
ordinary immutable findings plus coordinator-owned authorized host orchestration
and separate hash-verified acceptance records. Use that division if still needed;
do not change sandbox permissions or blindly repeat blocked operations.
Final source patch and validation handoff are at
artifacts/0003/final-completion/ in that peer checkout; the patch was verified to
apply cleanly, but the peer project itself was left at the earlier source base.
Refresh actual revisions and integrate only in an explicitly owned work area.

EXECUTE THE NEXT USEFUL INTEGRATED MILESTONE

1. Create docs/AUDIO_HARDWARE_PLAN.md as the owning continuation plan. Keep
   docs/AUDIO_TRANSPORT_PLAN.md as the completed preceding phase and link them.
   State actual devices/connections, software boundaries, hypotheses, numeric
   targets, short experiments, owners and acceptance gates before measuring.

2. Inventory connected USB audio devices on both Pis without disturbing other
   owners. Record stable device identity, native capture/playback formats,
   channel maps, supported rates, full-duplex behavior, available clocks/timestamp
   semantics, period/buffer constraints and existing users. Enumeration alone
   does not prove channel wiring, physical clock lock or simultaneous operation.
   Target remains 48 kHz, 24-bit capture and f64 DSP. If the USB microphone only
   supports 16-bit capture, record that honestly: it can validate a limited path,
   not the 24-bit acceptance requirement. Never manufacture bit depth by padding.

3. Inspect the existing PA host, FX library/host and recorder code, then implement
   the smallest real vertical slice: selected-device capture, local PA processing
   and dry output, independent local recording worker, PA-to-Brain sends and real
   source-frame FX returns. Build part by part. The recorder shell may still need
   its first actual recording path; put that work in SHR REC, with bounded queues,
   clear gap/error reporting and clean file finalization. Keep one owner per
   physical audio device. Use existing DSP rather than inventing new algorithms.
   Establish explicit startup, prefill, epoch replacement, worker restart, saved
   authority and fresh snapshot rules required by the integrated host. Keep
   allocation, locks, filesystem/network I/O and unbounded work out of callbacks.

4. Run brief actual-device smoke tests before longer experiments. Verify actual
   opened formats, capture/playback channel identity, frame continuity, buffer
   occupancy, under/overruns and recorded sample/file integrity. Use deterministic
   signals where possible and bounded microphone recordings where useful. All
   captures stay private. Measure through existing physical or genuine device
   loopback routes when available. Software loopback, device monitor capture and
   electrical/acoustic loopback are different paths; label them accurately.
   Without a return route, test independent ADC/DAC operation and file continuity
   but leave physical end-to-end latency unresolved. Do not invent a cable or
   ask me to connect one while I am away.

5. Replace simulated PA pacing with the actual device's sample timeline. Measure
   packet arrival, source/output frame alignment, actual processing delay, worker
   scheduling, audio callback deadlines and hardware latency separately. The
   earlier 8 ms budget is a transport admission target, not an established
   capture-to-speaker latency. Declare the real FX algorithm's delay and record
   any revised integrated budget with its reason; do not quietly lower targets.

6. If two devices are present, measure their independent rate behavior only with
   a defensible method and stated timestamp uncertainty. NTP/PTP does not clock
   USB converters. If the chosen architecture actually bridges two independent
   device clocks, evaluate a reviewed ASRC implementation and implement bounded
   occupancy control in the owning host if required. Do not invent a resampler
   or impose a second device clock on a source-following Brain unnecessarily.
   Separate actual clock measurements from simulated drift and arrival jitter.
   The noisy mic can help test capture/clock behavior; do not infer calibrated
   frequency response, room alignment, microphone quality or PA tuning from it.

7. Exercise Brain/transport stalls, controlled packet faults and restart while
   real PA processing and recording remain active. Verify real dry-frame and
   recorded-sample continuity, all-channel wet loss/fade/recovery, stale audio
   refusal and fresh control state. Record actual xruns, queue drops, disk errors
   and discontinuities. Do not substitute continuously increasing counters for
   inspection of the recorded audio. Only terminate processes owned by this task.

8. After repairs and short comparisons, run an initially bounded 10-minute soak
   of the accepted actual-device configuration with real recording and declared
   FX/network work. Use only physical channels the hardware provides; the old
   synthetic 64-channel capacity is not a claim about this card or full DSP load.
   Separate synthetic expansion from actual I/O in results. No competing builds
   during timing measurements. Extend a soak only to answer a concrete unresolved
   question, under a fresh/adequate reservation. No full-show reliability claim.

9. Diagnose failures, fix their causes and rerun affected checks. Continue beyond
   the first successful playback or first failed experiment. If hardware limits
   block a measurement, complete the software contracts, integrations, tests,
   recovery work and documentation that can proceed independently. Record the
   exact unavailable connection/capability and the smallest remaining physical
   action at the end, without interrupting me for it.

10. Run focused tests during development and each changed owner's complete normal
    suite for shared engine/schema/concurrency/safety changes. Use Rust 1.97.1,
    edition 2024, locked dependencies and CARGO_INCREMENTAL=0. Keep hardware,
    long studies and media experiments opt-in. Do not rerender unrelated music.
    Verify final formatting, relevant lint/build checks and publication guards.

FINISH AND PRESERVE THE HANDOFF

Write docs/AUDIO_HARDWARE.md with actual topology, routing/format/clock choices,
measured budgets, failures/fixes, evidence, supported configuration and remaining
physical limits. Update the owning plans, architecture, status, next-session notes,
documentation links and affected modules' own documents. Distinguish implemented,
offline-tested, synthetic two-node, actual-device and acoustic acceptance.

Keep exact commits, commands, versions, reservations, hashes and concise metrics
under ignored task artifacts. Preserve useful short recordings and unique failed
evidence; size recording durations before running. Check disk use and respect the
20 GiB free / 5 GiB build-output review thresholds. Clean only known task-created
disposables after checking active work. Preserve original recordings, prior
evidence, user sessions, unrelated edits and current required executables.

Obtain peer findings and post a separate coordinator acceptance in the private
ledger. Release reservations, stop temporary processes, finalize recording files
and restore task-owned temporary routing/settings. Leave both Pis powered and
network-reachable, and leave TV/Bluetooth alone. Inspect the complete staged index,
run publication guards, and make scoped local commits in each changed owner.
No GitHub push/release and no private recordings or machine state in source Git.

Finish with the commits, real configuration exercised, measured continuity/timing,
fault recovery, unresolved physical limits and evidence paths. Do not stop after
planning, a peer acknowledgment or a single successful test. No questions for me;
finish the authorized work and report the concrete outcome.
```
