# Audio transport execution plan

Publication note, 2026-10-04: this completed phase is included in 0.2.3. Its
original session/commit scope below is historical; new network/load work still
requires its own reservation. Dual-console integration continues under the
[Brain plan](../../architecture/BRAIN_CONSOLES.md).

Completed preceding phase. The authorized [hardware continuation plan](AUDIO_HARDWARE_PLAN.md)
and [actual-device results](../../acceptance/AUDIO_HARDWARE.md) now own PA/FX/REC integration;
this plan retains its synthetic measurements and original limits.

Owning plan for the 2026-10-03 authorized two-Pi synthetic transport task.
[AUDIO_TRANSPORT.md](../../reference/AUDIO_TRANSPORT.md) will own the resulting protocol and evidence.
Base: `fbcc9cb551508ef8d3e6533127928226e59e019a`. Existing console documentation
edits are preserved separately and excluded from this task's local commit.

## Completion

Completed the authorized software/synthetic task. The local commit containing
this plan owns implementation, documentation and normal tests; its exact SHA is
recorded in the private ledger after commit. No public push or release. Both
reservations are released and no experiment process remains. The observed design
envelope is 64 analysis channels + 16 FX sends/16 returns, 1 ms packets and
8 ms return admission over 600 s. Physical audio/DSP/NVMe and full-show acceptance remain explicit gates.

## Initial facts and scope

Pi 5 coordinates; Pi 4 is reachable through pinned SSH and the installed
`gigpies-peer` runner. Bootstrap 0001 is accepted; baseline 0002 was unreserved
at start. Pi 5: four Cortex-A76 cores, 2 GiB RAM, 40 GiB free, 3.4 GiB target,
Rust 1.97.1, kernel 6.18.50, MTU 1500, gigabit full duplex. Refresh both node
inventories in private evidence. Hardware timestamp enumeration is not accuracy.
PA owns the 48 kHz audio timeline, physical I/O, dry mixing/protection and local
recording. Brain follows source frames. Sibling source is read-only.

Authorized: software, private ledger commits, synthetic Ethernet PCM, mutually
reserved bounded load, application faults and task-owned process restart/cleanup.
Physical audio, clock/network configuration and public publication are excluded.
Retain originals and failed evidence; no music rerenders.

## Ownership and order

| Phase | Owner | State / exit condition |
|---|---|---|
| Inventory and reservations | Pi 5 ledger/tasks; Pi 4 immutable acknowledgments | Complete; reservation B on eth0 UDP45200/45201 through 16:20 UTC |
| 0002 untuned baseline | Pi 5 clients, Pi 4 one-shot servers | Accepted by both nodes; all baseline targets met |
| Protocol research and contracts | Pi 5 | Complete; custom UDP and PA frame clock selected |
| Reusable transport and fast tests | Pi 5 | Implemented; 19 focused tests and socket regression pass |
| Two-node synthetic trials | Pi 5 clients; Pi 4 bounded receiver/return | Comparisons complete; design 1/2 ms and 4/8 ms passed 30 s |
| Faults, repairs, 600 s soak | Pi 5 integration; Pi 4 peer review | Complete; design 600 s passed; stretch 8 ms failed; recovery verified |
| Verification/documentation/commit | Pi 5 | Complete; 173 Rust/37 Python, release/fmt/Clippy, peer review and local commit; reservations released |

## Predeclared workload and targets

All rates are 48 kHz; integers are packed PCM24. Small: 8 analysis + 2 FX-send
channels forward and 2 wet-return channels reverse. Design: 64 analysis + 16 FX
sends forward, 16 returns reverse. Stretch: 128 + 32 forward, 32 reverse, only
after design headroom. Channel groups will fit an ordinary 1500-byte IP MTU.
Packet durations: compare 1 ms and 2 ms; initial buffering candidates 4/8/16 ms.
Synthetic returns perform a declared identity turnaround, not an effects engine.

Baseline: no configuration tuning, 10 s per iperf step, 200 ping samples at
50 ms; nearest-rank p50/p95/p99/max. Idle RTT p99 target <2 ms; UDP 100 Mbit/s
loss target <0.01%; both TCP directions target >700 Mbit/s. Failures inform
capacity and do not get relabelled passes.

Transport: smoke 5 s, comparative trials 30 s, final design soak 600 s. Target
zero sample/channel corruption, zero unexplained loss in accepted design runs,
p99 same-clock PCM round trip <4 ms and <8 ms playout deadline, bounded memory
and queues, no sustained growth, and process CPU <200% per node (two cores).
Targets may fail; document original target, diagnosis and accepted envelope.
No one-way latency claim without a validated common timebase. Timestamp origin
is user-space monotonic unless explicitly measured otherwise.

Faults: isolated and burst loss, reorder/duplicate, malformed/truncated/oversize,
late/future/wrong session, restart, stall, sequence/timestamp wrap, simulated
±10/±100 ppm and ±1000 ppm stress. Fresh state must precede writes after reconnect;
old audio/commands cannot revive. Simulated PA/recorder progress must remain
independent. Physical dry-audio/NVMe continuity remains an integration gate.

Stop on resource conflict, wrong bind, loss of SSH, unbounded memory, unexpected
hardware access or deadline expiry. Never change host clocks/NICs or kill unrelated
processes. Logs, exact commands/revisions and hashes stay in ignored
`artifacts/audio-transport/2026-10-03/` and private ledger node records.

## Chronological checkpoints (latest completion below)

Initial reads and runner check complete. Next: acknowledge baseline reservation
on both nodes, execute 0002, then choose and implement the protocol.

### Protocol checkpoint

Implemented GPA1 custom UDP, fixed stream descriptors, PCM24/float32 codecs,
bounded jitter storage, wet envelopes, drift diagnostics and GPH1/GPC1/GPK1
control. Selected rtrb 0.4.0 for independent callback-to-worker queues. Thirteen
focused normal tests pass. The measured matrix now uses PCM24 analysis and
float32 FX (headroom); exact rates are declared in AUDIO_TRANSPORT.md before runs.
The peer worker can write evidence but its sandbox cannot inspect host networking
or write Git metadata. No worker permission changes were made. Its failure record
is retained; coordinator host preflight and a fresh peer acknowledgment are needed.

### Baseline result

0002 measurements completed 15:06 UTC; reservation released early. All eight
one-shot servers exited and post-test SSH/Git succeeded. Idle RTT p99 .306 ms;
saturated TCP p99 2.18 ms, 200/200 replies each. TCP sender throughput
933.344/941.705 Mbit/s with no retransmits. UDP25/50/100 Mbit/s both ways reports
zero loss; reverse endpoint totals differ by 1/2/3 boundary packets, retained in
the evidence. No network/clock tuning. Pi 4 historical thermal flag is 0x80000,
not a current throttling assertion. Peer result review is pending.

Fourteen focused tests and Clippy pass. Full normal suite is running. The private
loopback smoke preserved all samples but one return missed 8 ms while compilation
competed for CPU; retained as a failed timing experiment, not two-node evidence.
The paced producer was corrected to publish a block after its simulated capture
period; playout still maps source+delay. Two-node runs wait for builds to finish.

### First design failure and fix

Both small and design 5 s smoke passed. The 30 s design run with the default
212992-byte receive socket lost 11 forward datagrams and eight wet returns.
Pi 4's UDP receive-buffer error counter was exactly 11; NIC drops/errors and
Pi 5 receive errors were zero. No corruption, control failure or recorder gap.
This is a failed original zero-loss target, retained with source hashes.

Added `configure_audio_socket`, using socket2 0.6.5 to request a bounded 512 KiB
receive capacity (Linux reports 1 MiB including bookkeeping) and refuse clamping.
No host network settings changed. An explicit opt-in local socket regression
passes; normal tests still open no sockets. Per-run UDP counters now accompany
NIC/CPU/RSS/temperature records. Repeating design and packet/buffer comparisons
before stretch or soak. Peer review corrected process-supervisor readiness and
cleanup checks before any transport measurement; hashes are in the ledger.

### Review and timing checkpoint (15:31 UTC)

Peer reviewed candidate v4 and confirms all five findings fixed: immutable packet
metadata, rejection ACK identity, bounded writer resnapshot after lost replies,
RTT measured from the original send, and deadline admission against current PA
frames. The last change invalidated earlier deadline claims; repeated v4 design
trials passed 30 s with zero missing returns at 1 ms/8 ms, 2 ms/8 ms and 1 ms/4 ms.
Their RTT p99 values were .361/.527/.360 ms. Earlier raw results remain retained.

Stretch at 128 analysis + 32 FX + 32 returns received all packets and exact samples,
but missed 203 playout deadlines at 8 ms; RTT maximum 10.547 ms, p99 .535 ms.
No UDP receive errors or queue drops. This fails the original 8 ms stretch target;
a 16 ms comparison will characterize its limit, without replacing that failure.
No builds compete during timed trials. Next: controlled faults, server stall,
Brain process restart with PA continuing, then design soak.

User separately requested Bluetooth disconnection for TV use; the sole connected
Pi 5 device was disconnected successfully. Synthetic tests use no audio device.

### Recovery and soak checkpoint

Fault trial recovered through all first-command ACKs lost and a 750 ms control
outage. Injected 840 forward losses produced exactly 280 missing wet packets;
276 duplicates, 51 reordered packets, eight malformed packets and eight old epochs
were handled without sample corruption. A 250 ms receiver stall kept controls
responsive, refused 125 expired returns and recovered after bounded socket loss.
Both runs kept all 5000 simulated recorder blocks with no queue drops.

An actual Brain process was terminated and replaced while the same PA process
and epoch ran for 10 s: all 10000 recorder blocks, no gaps; wet and fresh control
resumed. Its 1754 unavailable wet packets remain explicit. Stretch at 16 ms passed
30 s; this is a short envelope comparison, not acceptance of stretch at 8 ms or
a long-run guarantee. Design 600 s soak at 1 ms packets/8 ms deadline is running.
Peer candidate-v4 review accepted; final measurement review remains pending.

### Final measurements (15:43 UTC)

Candidate v4 design soak passed all original synthetic arrival/queue targets:
7.2M forward/2.4M wet packets, zero corruption/missing/late, all600000 recorder
blocks, zero queue/network errors. RTT p50/p95/p99/max .269/.351/.362/4.031ms.
The playout worker itself was up to4.410ms late; this does not certify physical
output at8ms. CPU22.24/24.37%one-core, no sustained sampled RSS growth.
Retain1ms packets/8ms for design-profile integration. Stretch8ms remains failed;
16ms only has a30s passing observation. Full normal checks are running after all
trial processes exited; peer measurement review and scoped commit remain.

Final arithmetic review corrected a documentation error: GPA48+UDP8+IPv4 20+
Ethernet38 is114 bytes overhead per packet, not94. Correct aggregate wire rates
are18.096/137.472/274.944Mbit/s for small/design/stretch; source packet sizes and
measured counts were already correct. `wire-arithmetic.json` retains the derivation.

### Verification checkpoint

Full normal Rust suite and all Python synthetic tests passed after the final
600s run; formatter and all-target Clippy passed. Release build and peer final
measurement review are in progress. Both nodes have no test probe processes or
45200/45201 listeners; pinned SSH, remote source integrity and private Git remain
healthy. ReservationB released early; future load needs a new reservation.

Build output crossed5GiB with38GiB free. Identified task-created obsolete test
executables by the earlier full-suite log and timestamps; remove only those after
the final build is idle, preserving current executables, debug information and
all preexisting output/evidence. No recordings or shared caches are candidates.

### Completion and retained limits

173 normal Rust tests (including 19 transport contracts) and 37 Python tests
passed; rustfmt, all-target Clippy and the locked release build passed. Four
opt-in tests were skipped by default: three historical/media studies and the
socket-capacity test, which was separately run and passed. No music rerenders
or physical playback took place.

Pi 4's final review verified 40 evidence hashes, matched its original server
files, recomputed resource figures and accepted the design synthetic envelope.
Record `0003-final-review-20261003T154938Z-6d7070ed.md` has SHA-256
`b69e680b5dbdd34bc68231cdcbfff37a0467d9cb30f4eb3ed63591b09bffbcd1`.
Pi 5 reviewed that exact record and the later completed full-suite/build evidence.
The private coordinator acceptance links the final source commit and evidence.

Cleanup removed only 22 superseded task-created test executables and the conversion
benchmark executable after checking idle processes, open files and build locks:
733847552 allocated bytes recovered, target 4.5 GiB, free space 39 GiB. Current
builds, source, earlier documentation edits, recordings and all unique success and
failure evidence remain. The scoped commit excludes the prior console plan, its
archive drafts and diagram.

Next owner: the SHR modules for real PA/FX/recorder adapters. Then an explicitly
authorized device session with muted outputs at 48 kHz/24-bit must measure clocks,
channels, loopback latency and actual dry-audio/NVMe continuity through a Brain
restart. The detailed gate is in AUDIO_TRANSPORT.md. Further synthetic runs need
a changed candidate, changed conditions or a concrete unresolved transport issue.
