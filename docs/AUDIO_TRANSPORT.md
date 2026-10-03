# Synthetic audio transport

The completed synthetic phase continues in [USB hardware integration](AUDIO_HARDWARE.md)
and its [owning plan](AUDIO_HARDWARE_PLAN.md). Results below retain their original
synthetic scope; actual-device measurements are recorded separately.

Status: implemented contracts and completed two-Pi synthetic validation. The
[owning plan](AUDIO_TRANSPORT_PLAN.md) tracks execution and acceptance.
This is a GigPies integration prototype; PA, FX and recorder algorithms remain
in their owning repositories. No physical audio acceptance is implied.

## Selected layers and alternatives

Use **GPA1, a minimal versioned UDP unicast audio protocol**, with packed PCM24
analysis and float32 FX sends/returns. Use a separate bounded UDP control socket
with GPH1 snapshots, GPC1 commands and GPK1 acknowledgments. The PA sample
position owns timing; Brain processes arriving source frames without a second
free-running audio clock. SSH/Git remains the development channel.

| Candidate | Useful properties | Decision |
|---|---|---|
| RTP with L24 payload | Standard packet sequence/timestamp, packed integer PCM, external ecosystem | Keep as interoperability alternative. GigPies still needs negotiated channel groups, epoch identity and absolute return-frame mapping; RTCP/session interoperability is outside this prototype |
| GPA1 custom UDP | Explicit epoch, 64-bit frame and output offset in a fixed bounded parser; matches a single PA clock and private point-to-point link | Selected for the synthetic integration contract; requires an adapter for other endpoints |
| Zita-njbridge | Existing Linux JACK network audio with adaptive resampling and independent device clocks | Suitable alternative when independent JACK devices are required; not selected for a headless Brain following PA frames without opening a second audio device |

[RFC 3190](https://www.rfc-editor.org/rfc/rfc3190.html) defines L24's packed
big-endian interleaved representation. [RTP](https://www.rfc-editor.org/rfc/rfc3550.html)
provides sequencing and sample timestamps, but does not guarantee delivery or timing.
[Zita's author documentation](https://kokkinizita.linuxaudio.org/linuxaudio/)
describes receiver resampling and up to 64 channels. Local SHR FX currently hosts
through JACK; adapting its pure wet engines belongs to SHR FX, not this transport.

TCP's ordered retransmission can hold newer audio behind expired samples. Audio
has a playout deadline, so late packets are discarded. A separate TCP connection
could serve future bulk snapshots/configuration; this small control prototype uses
bounded datagrams, explicit replies and idempotent retries. QUIC DATAGRAM offers
unreliable encrypted datagrams alongside reliable streams, but adds connection,
security and congestion machinery not needed for this isolated lab prototype.
[Its specification](https://www.rfc-editor.org/rfc/rfc9221.html) is an option for
future untrusted networks. Never expose this unauthenticated prototype to a venue
LAN or Wi-Fi as a finished control service. Bind and pin the dedicated peer.

RTP or PTP use alone would not establish AES67 conformance. AES67 has timing,
media, session and interoperability requirements; consult the
[AES67 standard preview](https://www.aes.org/publications/standards/preview.cfm?ID=96).
GPA1 makes no AES67 or RTP interoperability claim.

## Audio format and negotiation

All integers are big-endian. Each datagram contains this 48-byte header followed
by frame-major, channel-interleaved samples. No normal IP fragmentation.

| Bytes | Meaning |
|---|---|
| 0–3 | `GPA1`: protocol/version |
| 4 | Encoding: 1 packed signed PCM24; 2 IEEE float32 |
| 5 | Role: 1 analysis; 2 FX send; 3 wet return |
| 6–7 | Channel count in this group |
| 8–9 | Frames per packet, 48 or 96 |
| 10–11 | First channel index within the role, total range 0–255 |
| 12–15 | Nonzero stream ID, unique per group and direction |
| 16–23 | Nonzero session epoch, replaced on PA clock/stream restart |
| 24–31 | Absolute first source frame, aligned to packet duration |
| 32–35 | Sequence, wrapping unsigned 32-bit, one increment per group packet |
| 36–39 | Sample rate, exactly 48000 |
| 40–43 | Output delay in frames, only wet return; 192–1536 |
| 44–47 | Reserved zero |

The maximum UDP payload is 1232 bytes, also safe within IPv6's minimum 1280-byte
MTU. At 1 ms, eight PCM24 channels use 1200 bytes; four float32 channels use 816.
At 2 ms use four PCM24 or two float32 channels per group. Never split a sample.
Source-frame exhaustion requires a new epoch; it is not permitted to wrap u64.
Sequence wrap and crossing a 32-bit sample timestamp boundary are tested.

`StreamSpec` is the negotiated descriptor. The experiment configures identical
specs before opening sockets, with a fresh epoch per run and at most 64 groups. Automatic discovery and
network exchange of arbitrary channel maps are not implemented. `validate_session` refuses overlaps, duplicate stream IDs, gaps, mismatched
epochs/packet durations and unequal send/return channel counts. A host must agree
frame/sequence origins before accepting data. Packets cannot create or change a stream. Parsed metadata is immutable; getters
return copies so callers cannot change validated sample bounds.

Capture is 24-bit; local mixer arithmetic remains f64. Integer encoding/decoding
is bit-exact and refuses out-of-range values. PCM24 normalized to f64 loses no
information. FX uses float32 to retain headroom; values must be finite and within
±16. No implicit clipper is inserted. Float32 has 24 significant binary bits:
normalized PCM24 values are exactly representable; arbitrary f64 results incur
rounding (at amplitude ≤4, absolute error ≤2^-22). Float64 wire samples would
unnecessarily double float32 bandwidth. A Pi 5 optimized microbenchmark (1.92 million samples, median of three runs)
measured PCM24 encode/decode-to-f64 at 1.633/3.010 ns per sample and float32 at
1.468/3.237 ns. A bulk f64-to-f32 cast measured .219 ns per sample. These use tiny
repeated fixtures and compiler optimization; they do not represent full DSP,
cache pressure or callback deadlines. Source, command and raw values are retained.

## Declared workload

One analysis subscription per source; FX channels below are additional sends.
There is no implied mixer/FX-engine capacity from these transport counts.

| Profile | PA → Brain | Brain → PA | Aggregate packets/s at 1 ms | Wire Mbit/s |
|---|---|---|---:|---:|
| Small | 8 PCM24 analysis + 2 float32 FX | 2 float32 wet | 3000 | 18.096 |
| Design | 64 PCM24 analysis + 16 float32 FX | 16 float32 wet | 16000 | 137.472 |
| Stretch | 128 PCM24 analysis + 32 float32 FX | 32 float32 wet | 32000 | 274.944 |

At 1 ms the forward/return group counts are 2/1 (small), 12/4 (design) and
24/8 (stretch). At 2 ms the design uses 24/8 groups at 500 packets/s each.

Wire arithmetic includes 48-byte GPA1 + 8 UDP + 20 IPv4 + 38 Ethernet
(header/FCS/preamble/interpacket gap), no VLAN. Design forward/reverse are
109.248/28.224 Mbit/s; audio payload alone totals 122.880 Mbit/s. Control at 50 Hz
adds a small separately counted load. At 2 ms design grouping keeps the same
packet rate; larger packets cannot hold the original channel groups within MTU.

## Clock, buffering and recovery

The PA audio device is the reference. Brain without an independent device follows
source-frame progress; software pacing in this experiment simulates PA capture.
An identity wet return labels intended output as `source_frame + delay_frames`.
The declared initial return deadline is 384 frames (8 ms), including both network
legs and processing. Physical capture, conversion and output latency are additional.
A real FX adapter must declare its processing delay and meet the remaining budget.

The socket worker requests 512 KiB receive storage; Linux reports 1 MiB including
bookkeeping. `configure_audio_socket` refuses a clamped result. This is a bounded
per-socket setting, with unchanged host sysctls and MTU; deadlines still reject
old audio. [Linux socket documentation](https://man7.org/linux/man-pages/man7/socket.7.html)
explains receive-buffer accounting. Per-group jitter storage allocates once, with 2–32 slots. Playback advances from
the PA timeline even on loss; arrival never moves its cursor. Insertions refuse
wrong descriptors/epochs, inconsistent sequence/frame mapping, late, duplicate
and out-of-window data. Missing slots yield an explicit missing result, not old
audio. A host must prefill against a fixed deadline, handle grouped-channel gaps and
explicitly replace session/buffer state on a PA discontinuity. This runner agrees
descriptors before starting; dynamic stream negotiation is pending. Old queued audio must be discarded
on PA epoch replacement. A Brain-only restart keeps the PA timeline; retained
packets still have to meet the same deadline. `insert_wet_at` checks actual PA
frame position after validation, even if the worker has not caught up its playout
cursor. Scheduling lateness of that worker is reported separately. The wet envelope fades the last valid sample over 240 frames (5 ms)
to silence; recovery ramps from the current gain after fresh prefill. This is a
synthetic de-click contract; perceptual behavior awaits real FX acceptance.

`CaptureFanout` uses independent preallocated
[rtrb 0.4.0 queues](https://docs.rs/rtrb/0.4.0/rtrb/): bounded push/pop without locks,
waiting or allocation after setup. A full network queue drops the new block and
counts it; recorder overflow has its own counter and must surface as a recording
gap. Copy blocks cannot run allocating destructors at this boundary. No socket,
logging or allocation belongs in the future callback. Real PA and recorder
integration remain separate gates.

Small clock offsets (±10 and ±100 ppm) and ±1000 ppm stress are simulated. A
second free-running clock would accumulate 2880 frames at 100 ppm in 600 s.
Following PA frame IDs introduces no such rate mismatch. `DriftObserver` estimates
source progress against receiver monotonic arrival time for diagnostics only;
network variation contaminates it. It never adjusts the PA clock.

Where an independent device clock actually exists, use a reviewed asynchronous
resampler with bounded occupancy feedback and explicit delay, then validate audio
quality. [Rubato's asynchronous resamplers](https://docs.rs/rubato/latest/rubato/)
provide a candidate allocation-free processing interface; no resampler or inferred
clock correction is implemented here. PTP is optional future clock observation.
[Clock signalling](https://www.rfc-editor.org/rfc/rfc7273.html) does not itself
synchronize a USB ADC/DAC. Existing NTP stays unchanged. Pi 4 reports software
timestamping only; Pi 5 hardware capability does not prove working accuracy.

## Control contract

This prototype exposes one bounded test parameter (−60000..12000 millidB), not a
real mixer API. Identity is PA session + writer ID + monotonically increasing
lease. `GPH1` hello is 32 bytes: magic, four reserved zero bytes, then three u64
identity fields. A fresh lease returns the current snapshot before writes. An
exact hello retry is idempotent while connected. A timed-out lease cannot reconnect.
A control-worker restart must preserve the lease high-water mark or rotate epoch.

`GPC1` and `GPK1` are 64 bytes. Magic at 0; ACK disposition at 4 (commands zero);
reserved zeros through 7; five u64 fields at 8,16,24,32,40 for session, writer,
lease, command ID, expected/applied revision; i32 value at 48; zero through 63.
ACK dispositions: applied, duplicate, stale, identity, invalid, disconnected,
snapshot. Snapshot uses command ID zero. ACKs must match all command identity
fields before being considered. Parsing has an exact length limit.

One command is in flight per writer; retransmit that exact request with a bounded
retry count. `ControlWriter` retries after 100 ms, at most three sends per
request/hello, then advances its lease and requests a new snapshot. A matching
rejection also forces resynchronization. It refuses stale ACK identities and
measures command RTT from the first send, including retries. Rejection ACKs echo
the rejected request identity, so an old lease cannot acknowledge a new command.
The authority stores one last applied request/ACK: exact retry returns
its result; conflicting or older IDs are refused. Expected revision prevents stale
state changes. Disconnect preserves the authoritative value but disables writes;
reconnect obtains a new lease and snapshot, discarding queued commands. Scope
ownership, authentication, scene persistence and real parameter adapters are future
integration work. The runner services control with a separate bounded receive
budget so audio floods cannot consume an unbounded loop.

## Measurements and limits

Untuned baseline: 200/200 idle and 200/200 saturated-TCP pings. RTT
p50/p95/p99/max: idle 0.169/0.253/0.306/0.406 ms; loaded
2.01/2.12/2.18/2.21 ms. TCP sender forward/reverse 933.344/941.705 Mbit/s, no
retransmits. UDP 25/50/100 Mbit/s each way reports zero loss; reverse sender and
receiver boundary totals differ by 1/2/3 packets, retained explicitly. All
bounded servers exited; post-test ping, pinned SSH and Git handoff passed.

First failure: a 30 s design run lost 11 forward datagrams, matching Pi 4 UDP
receive-buffer errors; eight wet packets were missing. NIC errors/drops were
zero. The per-socket capacity fix above addresses this measured scheduling
burst. A prior local smoke during compilation also had one late return and is
excluded from two-node acceptance. Neither failed run is discarded.

Candidate-v4 comparisons below used 30 s paced capture, no competing builds,
and simultaneous nominal 50 Hz control. Small/design 5 s smokes also passed, but preceded
the final deadline-admission fix. RTT starts immediately before each FX send and
ends after return validation on PA; it includes identity turnaround and receive
work, and excludes capture packet fill time. Brain follows source frames.

| Profile / packet / return budget | Return packets | Missing playout | RTT p50 / p95 / p99 / max, ms |
|---|---:|---:|---|
| Design / 1 ms / 8 ms | 120000 | 0 | .275 / .355 / .361 / .546 |
| Design / 2 ms / 8 ms | 120000 | 0 | .306 / .464 / .527 / .912 |
| Design / 1 ms / 4 ms | 120000 | 0 | .271 / .354 / .360 / .513 |
| Stretch / 1 ms / 8 ms | 240000 | **203 late** | .265 / .353 / .535 / 10.547 |
| Stretch / 1 ms / 16 ms | 240000 | 0 | .244 / .316 / .442 / 1.042 |

All five runs received all forward and reverse packets with exact samples; the
stretch failure is expired delivery, with no UDP-buffer or NIC error. Its rare
10.547 ms tail exceeds the original 8 ms budget. The 16 ms run passed briefly,
but its largest observed RTT was only 1.042 ms: this rerun did not reproduce the
outlier and does not prove that increasing the buffer solved its cause. Retain
stretch as a short capacity observation, not an accepted 8 ms workload.

For the 30 s design/8 ms run, process CPU averaged 21.9% of one core on Pi 5
and 28.5% on Pi 4; sampled one-second peaks were 23.9/35.8%. RSS ranged
1.59–2.69 MB / 2.32 MB, and wet-buffer occupancy peaked at seven packets per
group. Stretch used about 34/32% of one core and occupancy 15 at 16 ms.
These costs include test generation/validation and identity returns, not real FX.

The final **600 s design soak passed** at 1 ms packets and 8 ms return admission:
7.2 million forward packets, 2.4 million returns, zero sample corruption, missing
returns, expired arrivals, send errors or queue drops. All 600000 simulated
recorder block IDs arrived without gaps. RTT p50/p95/p99/max was
.269/.351/.362/4.031 ms. There were 29883 acknowledged control commands, RTT
.209/.451/.540/2.391 ms. Brain packet processing was .009/.022/.034/.255 ms;
absolute packet-arrival interval variation was .026/.100/.117/3.779 ms.

The PA producer's scheduling lateness was .054/.057/.060/2.509 ms, and the
playout worker's was .042/.214/.228/4.410 ms. Packets already admitted before
their deadline can still be popped by a delayed worker. Thus this accepts
**transport arrival at the deadline**, not guaranteed physical output at 8 ms.
Peak wet occupancy was ten packets per group within the fixed 32-slot storage.
Observed arrival-based drift was +6.61 ppm, contaminated by scheduling/network
variation; no device drift or clock correction was measured.

Sampled CPU averaged 22.24/24.37% of one core on Pi 5/Pi 4 (one-second peaks
24.87/37.83%). Sampled RSS after startup stayed at 2.11/2.32 MB; temperatures
ranged 56.75–61.15 / 53.07–58.43 °C. Both retained flags `0x80000`, a previous
soft-temperature event, with no current throttle bits in the samples; see the
[firmware bit definitions](https://www.raspberrypi.com/documentation/computers/os.html#get_throttled).
NIC and UDP receive-buffer/send/checksum/memory error deltas were zero. Host NIC
byte deltas were 8.024 GB forward and about 2.03–2.06 GB reverse over the capture
windows, including SSH/control and differing endpoint windows; these counters
omit some wire overhead and are not a precision wire-rate measurement.

Choose **1 ms packets and an 8 ms return budget for further design-profile
integration**. A 4 ms run passed briefly, but the soak's 4.031 ms RTT maximum
leaves insufficient margin for it. No full-show reliability, real FX capacity
or stretch-at-8-ms acceptance is claimed. Private command logs, failed runs and checksum
manifest: `artifacts/audio-transport/2026-10-03/`. Percentiles use nearest rank with 1 µs ceiling bins, capped at 100 ms;
overflow counts and uncapped maxima remain explicit in fault runs.
User-space monotonic timestamps measure same-clock send-to-return RTT and local
processing time; RTT/2 is never presented as measured one-way latency.

## Repairs and recovery evidence

Peer review found five issues in the initial candidate, all fixed before final
comparisons: publicly mutable parsed metadata could invalidate bounds; rejection
ACKs could carry the current lease instead of the rejected lease; the prototype
writer stalled after all replies were lost; its retry RTT omitted earlier waits;
and receiving before advancing the playout cursor could count overdue audio as
on time. The last issue means earlier timing admission results are provisional.
Candidate v4 repeated the comparisons with the corrected deadline check.

The 5 s controlled-fault run injected 840 missing forward packets (isolated losses
and a 20 ms burst), giving exactly 280 missing wet returns. Brain rejected 276
duplicates, accepted 51 reordered packets and refused eight malformed/eight wrong
epoch packets. Every received sample matched its channel/frame pattern. Dropping
all replies for the first applied command and a separate 750 ms Brain-to-PA control-request outage
forced fresh leases/snapshots; commands resumed without applying stale state.

A 250 ms Brain audio-receive stall filled the bounded socket and lost 2631 forward
packets; PA discarded 125 expired returns. Missing wet playout totaled 1004.
Control continued (244 acknowledged commands, RTT p99 .451 ms) because its service
budget was independent. Both 5 s trials kept 5000/5000 simulated PA/recorder blocks
with no queue overflow. A separate pure contract test fills the network handoff
queue: 9992 drops cannot block 10000 local/recorder blocks.

The Brain process was also terminated and replaced during one continuous 10 s
PA run and epoch. All 10000 recorder block IDs arrived without gaps; 1754 wet
packets were unavailable during the outage, then exact wet audio and commands
resumed. The restarted server's totals cover only that process's lifetime; its
“missing” field also includes pre-restart frames. Tail control retries after PA
exit are not reconnect latency. The pure tests establish exact stale-command and
old-epoch refusal, sequence wrap, wet fade/reset and simulated drift behavior.

These are synthetic source-frame and queue contracts. The two-node runner exercises the wet envelope on the first channel of each
return group; exact sample checks cover every received channel. No actual mixer, FX engine,
USB capture/output or NVMe recorder ran. The worker calling the synthetic playout
loop is not a real-time audio callback. Its observed scheduling delay must not be
represented as guaranteed hardware output timing.

## Integration and physical acceptance gate

GigPies owns packet/session contracts and worker adapters. SHR PA must connect the
bounded fanout and wet return to its actual 48 kHz/f64 host, keeping dry mixing,
protection and recorder handoff independent. The recorder owner must write real
24-bit multitrack blocks to NVMe and surface gaps. SHR FX must provide source-frame
processing with a declared algorithmic delay; identity turnaround does not certify
FX CPU cost. Brain console parameter adapters, scope ownership/authentication,
persistent lease/epoch handling and negotiated network session setup are pending.

Next hardware session requires explicit authorization to activate the selected
48 kHz/24-bit interface with outputs muted, inspect its ADC/DAC/ADAT clock locks
and channel mapping, record a known loopback to NVMe and measure same-clock
capture-to-output latency. While real PA/recording runs, terminate only the owned
Brain/transport processes and verify continuous dry frame counts and gap-free
recorded samples, bounded wet fade and fresh-state recovery. Speaker listening,
cable pulls and clock/NIC changes need their own concrete session scope. Device
clock drift and any needed ASRC must be measured with actual independent devices.
Runtime Pi assignments remain open: no full mixer/FX/console load was profiled.

## Verification and retained evidence

Normal validation: 173 Rust tests passed, including 19 transport contracts;
37 Python synthetic tests passed; rustfmt, all-target Clippy and the locked
release build passed. The
normal suite intentionally skipped three historical/private-media studies and
the opt-in socket test. The socket test was run explicitly and passed when the
receive-capacity fix was added. Historical music renders, listening, playback and
hardware tests were not run. Commands are documented in
[DEVELOPMENT.md](DEVELOPMENT.md#synthetic-transport-validation).

The two-node experiment imported the actual transport modules into an optimized
private runner. `source-manifest-v4.json` pins base commit, all measured source
hashes and binary SHA-256; `build-command-v3.json` is the exact v3/v4 compiler
argument vector. Each trial records command, fresh epoch, ready PID/bound sockets,
watchdog exit, results and one-second resources. `measurements.json` summarizes
raw data, including failed runs. `design-soak-600s-v4-assessment.json` records
explicit target checks. Commands/samples/checksums remain under ignored
`artifacts/audio-transport/2026-10-03/`; private ledger tasks 0002/0003 carry
independent peer review and coordinator acceptance.

All test processes exited and UDP45200/45201 were free on both nodes after the
soak; reservations were released. Pinned SSH/Git and the untouched peer source
checkout were verified. Original recordings, previous documentation edits and
unique failure evidence remain preserved. The user separately requested and
received Bluetooth disconnection on Pi 5 for TV use. No synthetic test opened an
audio device or changed NICs, clocks, governors or persistent services.

Pi 4 verified all 40 files in the final review snapshot, independently recomputed
resource summaries and compared supplied server files with its local originals.
Its final review accepts the observed design envelope and rejects stretch at
8 ms. Pi 5 reviewed that record and the later completed normal-suite/build logs
separately. Peer review did not independently run the tests. Both review and
coordinator acceptance retain the source base and exact candidate hashes.

Cleanup removed 22 superseded test executables created during this task plus the
disposable conversion benchmark binary, after checking processes, open files and
Cargo locks. It recovered 733847552 allocated bytes (about 700 MiB). Current
executables/debug information and all unique evidence remain; target is 4.5 GiB,
free space 39 GiB, and private task evidence about 9 MiB. This is a scoped local
source commit; no GitHub push or release was performed.
