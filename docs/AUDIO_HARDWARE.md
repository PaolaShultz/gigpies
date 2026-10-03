# USB host integration and measurements

**Live latency acceptance is unmet.** The user rejected the 64 ms buffer / 56 ms
prefill configuration as unsuitable for live use. Its clean soak is retained as
a correctness benchmark. The next work fixes partial-read pacing and tests the
smallest practical period, capacity and prefill; it must pass the same continuity
and recovery checks before live-latency acceptance.

Unreleased, bounded bench integration of the independently built SHR modules.
The [execution plan](AUDIO_HARDWARE_PLAN.md) continues the preceding
[synthetic transport phase](AUDIO_TRANSPORT.md). This implements an explicit ALSA host and real stereo module processing/recording.
The live console, complete mixer, multichannel stagebox and recorder UI remain
separate planned integrations.

## Topology and device scope

Pi 5 owns a PreSonus AudioBox USB 96 (USB 194f:0303, ALSA ID A96); Pi 4 runs
source-following SHR FX over the reserved Ethernet link. The interface has
two capture and two playback channels. Kernel descriptors list FL/FR,
S32_LE with **24 descriptor-declared bits**, and 44.1/48/88.2/96 kHz. The final
configuration opens both directions at **48 kHz, two channels, 384-frame periods
(8 ms) and 3072-frame buffers (64 ms)**. Earlier 16/32 ms buffers had retained
playback underruns. A separate negotiated ALSA query returned **32 significant bits**,
despite the USB descriptor's 24-bit declaration. A retained native S32 capture
contained only 0 or 3 in the low byte; its rounded PCM24 codes exactly matched
upper-24-bit extraction. This does not establish a 32-bit converter. The host now
explicitly extracts the upper 24 bits and hashes those bytes independently of
floating-point conversion. No resampling or host clock change occurs.

No Superlux microphone or other USB audio interface is connected to either Pi.
The amplifier is disconnected. All soak/fault trials initially had no established
physical return. Afterward the user connected both outputs to inputs; the separate
physical checks below verified the left route and found a severely attenuated
right return. Converter clock lock and calibrated capture-to-output latency remain
unverified. No independent-device drift or ASRC experiment is possible or needed
for this source-following Brain.

The command requires a stable explicit raw USB card ID and matching kernel USB
identity/stream descriptor. It never chooses the default device, Bluetooth or
HDMI. No mixer controls, persistent services, NIC settings or clock daemons are
changed. ALSA status timestamps use CLOCK_MONOTONIC; reported audio timestamps
and USB feedback frequency are driver observations, not calibrated converter
timestamps or measured clock lock. See the
[ALSA format/status contract](https://www.alsa-project.org/alsa-doc/alsa-lib/pcm.html).

## Implemented slice

`src/host/` owns the bounded frame/routing contracts and the optional host.
`gigpies-hardware` is built only with `--features hardware-host`; the ordinary
offline CLI and normal tests open no hardware. Libraries are explicitly supplied
trusted local executables with versioned C symbols. No sibling path dependency
or second DSP engine implementation is added to GigPies.

| Owner | Interface used and precision |
| --- | --- |
| SHR PA | `shr_pa_v1_*`: native f64 stereo input, six logical outputs, full-range unity processing, 5 ms startup ramp and existing −1 dBFS linked sample limiter; no fixed algorithmic delay |
| SHR FX | `shr_fx_v1_*`: existing wet-only stereo Digital Delay, **20 ms / 960 frames** first tap, 0.25 feedback, 0.35 damping and 0.5 wet gain; shared delay implementation specializes **f64 DSP/state** for integration; the existing rack retains f32 |
| SHR REC | `shr_rec_v1_*`: preallocated whole-block queue, independent disk worker, mono PCM24 stems, source epoch/frame journal, explicit gap/fault counters, no-replace finalization and recovery into a new directory |

The initial smoke used generated audio only. Subsequent trials explicitly chose
`capture_with_probe`: captured stereo at a digital factor of 0.125 plus distinct
997/1499 Hz signals bounded at −36 dBFS, quantized to PCM24 before DSP. This
digital gain does not change analogue input gain or repair ADC clipping.
The captured inputs were quiet and no ADC full-scale samples were observed in
the short trials. There is no implicit microphone monitoring mode.

One PA instance supplies a dry audit tap. A separate instance processes the
same source plus wet returns at a factor of 0.25, so the final combined path
passes through existing PA protection. The bench additionally caps submitted
output at ±0.25 (about −12 dBFS). Logical PA outputs 0/1 map directly to physical
outputs 0/1; the other four logical channels are not summed into the stereo card.

The take has **eight software stems, only two of which are physical inputs**:
ADC L/R, actual PA source L/R, PA dry L/R, and DAC-submitted L/R. The last pair
records the intended PCM block, not a return measurement from the DAC. After a
failed partial write, its physical delivery is unknown. Reports distinguish
captured, recorder-accepted and fully written playback blocks within the requested
take. The bounded quiet shutdown transfers are outside that recorded interval.

## Timeline, startup and recovery

Actual USB capture transfers pace source frames. The host splits each capture
block into 48-frame GPA1 float32 FX packets: four at period 192, eight at period 384. Brain has no audio
device and follows these IDs. The original **8 ms transport admission target**
failed once in the first 600 s run. The revised candidate explicitly assigns wet
output to source frame +768 (**16 ms**); the intentional 20 ms echo is additional.
Capture batching, host service and DAC queue delay are measured separately.
Their sum is not presented as a measured physical round-trip latency.

Before starting the PCM streams, the host prepares libraries, files and queues,
then requires a fresh control snapshot followed by an acknowledged command.
Playback primes the configured buffer minus one period of silence: three
periods for the initial buffer, seven for the revised eight-period buffer. The audio-owned jitter buffer admits
queued returns before rendering their output block; earlier frames are refused.
Arrival never advances its cursor. Each wet channel independently fades the last
valid sample to zero over 240 frames (5 ms); fresh data ramps back in. Brain
rejects wrong epochs, duplicate/backward frames and inconsistent sequence IDs;
forward gaps reset its FX history. It withholds returns while control is unsynced.

The network worker and recorder worker have independent bounded queues. A full
network queue cannot block dry processing or recording. The render section,
including fault returns, performs bounded arithmetic and queue/module calls
without allocation, deallocation, locks or I/O. ALSA reads/writes, waits, status
queries, audit hashing and diagnostic construction are outside that section.
This is a direct ALSA driver loop, not a claim about a JACK callback or hard
real-time scheduling guarantees. Full host service time and capture intervals
are reported separately from render time.

Brain process replacement leaves the PA process, source epoch, DSP configuration,
recorder and authoritative prototype control state alive. A new monotonically
increasing lease obtains a fresh snapshot. **Control values remain the existing
prototype test parameter; acknowledgments do not apply a real mixer setting.**
The bench DSP configuration is fixed for the run. PA/device restart requires
new handles, a new recording and a fresh source epoch. There is no automatic
xrun concealment or attempt to append discontinuous capture as a complete take.

## Actual-device checks

Release libraries and host ran without competing builds under private task 0004,
reservations H1/H2. The first tests below used period 192/buffer 768 and 8 ms return
admission and the initial **f32 FX** library, with no system tuning.

| Trial | Captured/retained frames | Result |
| --- | ---: | --- |
| 3 s generated-source smoke | 144000 | Zero xruns, drops or missing wet packets; all eight PCM hashes and exact PA/FX/DAC replay match |
| 10 s capture + probe | 480000 | Zero xruns, drops or missing wet packets; exact dry and combined-output replay match |
| 15 s packet faults + 250 ms Brain stall | 720000 | Raw/tap hashes and dry replay match; 295 missing wet packets, 260 expired returns refused; both channels recover |
| 15 s actual Brain termination/replacement | 720000 | Raw/tap hashes and dry replay match; 1144 missing wet packets, three socket errors during outage, two fresh snapshots; both channels recover |
| Deliberate 100 ms local driver stall | 96000 before stop | One ALSA capture xrun; nonzero process result and **incomplete** finalized take with host fault retained |

File verification reads every stem, checks format/length and payload SHA-256
against bytes submitted by the host, examines every source/file journal entry,
and independently replays the owning PA library from the recorded source.
The no-fault trials also exactly reproduce the final recorded DAC blocks through
the owning FX and final PA libraries. Thus continuity evidence includes actual
stored samples; it is not just increasing block counters. Without a known
physical return signal, it cannot certify an undetected converter-side anomaly.

For the known 20 ms packet-loss burst, measured residual wet output in **both**
recorded channels reaches exactly zero after 240 frames. Maximum deviation from
the expected linear fade using the quantized last sample is 0.50/0.692 PCM24 LSB.
Both channels have sustained zero wet output during outages and nonzero wet output
after recovery. Dry replay remains bit-exact throughout. The local xrun take is
preserved and verifies exactly through its final retained block; it is classified
as incomplete. The xrun counter counts detected ALSA errors before immediate
stop, not every possible underrun/overrun in both streams after a stall.

The 10 s capture trial measured render p99/max 0.540/0.722 ms and complete
post-read host service p99/max 0.629/0.843 ms against a 4 ms period. Network RTT
p99/max was 0.459/1.303 ms. Capture service intervals reached 7.609 ms despite
zero xruns; buffers absorb scheduling variation. These values do not establish
physical output at an 8 ms deadline or guarantee every host wakeup within 4 ms.

Candidate v3's fault RTT histogram stored only 0–10 ms bins. A rank above that
range incorrectly displayed the maximum as a percentile; that value is only an
upper bound. Candidate v4 reports such percentiles as unavailable, retaining
overflow counts and the uncapped maximum. Earlier evidence remains unchanged.

The first **600 s soak at 8 ms admission** retained 28.8 million frames with zero
xruns, recorder drops or journal gaps. All 600000 network packets returned, but
**one expired and produced one missing wet block**. The original zero-loss target
therefore failed. All eight PCM hashes and dry replay match; 190 DAC-submitted
samples differ from uninterrupted FX replay. The original take is retained.

RTT p99/max was 0.455/7.717 ms; render p99/max 0.654/1.150 ms; complete post-read
host service p99/max 0.734/1.286 ms. Capture intervals reached 8.588 ms. The
7.717 ms RTT leaves little margin in an 8 ms budget with 4 ms capture batches and
host wakeup variation. These aggregate observations motivate the declared 16 ms
comparison; they do not identify a guaranteed worst case or prove a particular
scheduler cause. The revised candidate also makes upper-24-bit capture extraction
explicit and checks a separate native ADC byte hash.

### Revised 16 ms admission and device-buffer comparison

At period 192, a fresh 30 s/1.44-million-frame comparison passed with zero xruns,
missing wet packets or file/replay differences. Repeated 15 s packet-stall and
Brain-restart trials each preserved 720000 frames, direct ADC hashes and exact
dry replay. Faults caused 283 missing/248 expired returns; restart caused 1148
missing returns, four socket errors and two fresh snapshots. Both wet channels
recovered. The known loss burst again faded to zero in 240 frames, with maximum
0.525/0.600 PCM24 LSB error against the quantized previous sample.

The first attempted 600 s run at 16 ms admission **failed with a playback EPIPE**
after 10.28 s: 493248 frames fully submitted, 493440 captured/recorded. All retained
samples match direct ADC hashes and dry/intended-output replay, with no journal
gaps or wet losses, but the last block's physical delivery is unknown. REC marks
the take incomplete with an external host fault. Render maximum was 3.709 ms,
with 0.554 ms p99; the last completed host-service maximum was 0.932 ms. Those
aggregates do not prove the underrun's scheduling cause or include the failed
cycle in the completed-cycle maximum. The host stopped; no gap was concealed.
The private supervisor originally timed out waiting for Brain after this early
exit. Its original log is retained; exact task-process cleanup followed. The
supervisor now persists local observations before bounded peer cleanup.

The revised device condition is period 384/buffer 1536 (8/32 ms), retaining 16 ms
wet admission and the same exact v5 binaries. This gives additional buffering
margin without clock/service tuning. A 30 s comparison passed with 1.44 million
frames, no xruns, no missing wet returns and exact ADC/dry/DAC verification.
Render p99/max was 1.148/1.264 ms; full post-read service 1.319/1.437 ms. Capture
intervals reached 16.007 ms, overflowing the 10 ms histogram; p95/p99 are therefore
unavailable, not substituted with a maximum.

The subsequent 600 s run at that condition completed 28.8 million frames with
zero xruns, recorder/network queue drops, missing wet packets or expired returns.
All 600000 FX packets returned. Render p99/max was 1.164/5.979 ms and complete
post-read service 1.329/6.257 ms. RTT p99/max was 0.574/11.018 ms; one RTT value
overflowed the histogram, while p99 remained in range. Capture intervals reached
15.995 ms, with p95/p99 outside the histogram. Observed playback queue delay was
408–1152 frames (8.5–24 ms). This run still uses the initial f32 FX library;
all direct ADC hashes, eight stem hashes, journal entries and full dry/DAC replay
match exactly. The native f64 follow-up is recorded separately below.

The failed 8 ms wet deadline and failed 4 ms device-period run remain failed
observations. Changing these separate budgets does not establish physical
capture-to-speaker latency or hard real-time guarantees.

### Native f64 precision completion

The final FX implementation shares the existing delay algorithm through static
f32/f64 specialization. Integration coefficients, delay buffers, feedback/filter
history and arithmetic are f64; the existing rack remains f32. The C ABI, 960-frame
first tap and effect settings are unchanged. GPA1 still deliberately encodes FX
sends and returns as float32. This separates internal DSP precision from the
chosen transport format.

SHR FX commit `6510ead9cea8e28a92d992a4147cda17a074c86f` passed 101 normal tests,
including sub-f32 input detail, f64 feedback coefficients, no-allocation/reset
checks and exact original f32 rack sample bits. Formatter, Clippy, release and
dynamic ABI checks passed. Because shared hot paths changed, the opt-in ten-case
cost matrix was rerun: finite/fault checks passed, but the sixteen-hall rack's
8.566 ms maximum exceeded its 5.333 ms nominal period. That heavy configuration
remains unaccepted; it is separate from this stereo delay bench.

Candidate v6 keeps the exact v5 host and PA/REC libraries and supplies this new FX
library under a separate immutable filename/hash. Its 30 s comparison preserved
1.44 million frames with exact direct ADC/stem hashes, dry/DAC replay and zero
xruns or wet losses. The 15 s packet-stall test preserved 720000 frames with
exact ADC/dry/journal checks, 299 missing and 264 expired returns.

The next Brain-restart attempt **failed with another playback underrun** at
5 seconds: 240000 frames were fully submitted, 240384 retained. All retained
samples and intended-output replay match, but the incomplete take cannot count
as successful restart continuity. Render maximum was 6.826 ms and capture-read
maximum 14.974 ms. The driver had initially primed 24 ms of output and its lead
varied with USB batching. These observations justify more device buffering;
they do not prove a particular kernel scheduling cause or implicate FX precision.
A separate deliberate 100 ms stall again stopped cleanly at 96000 frames and
retained an exact incomplete take.

Candidate v7 adds an explicit `buffer_periods` field, restricted to four or eight,
and verifies exact device negotiation. The revised setting is eight periods:
**3072 frames / 64 ms buffer**, with 2688 frames / 56 ms of initial silence.
Period remains 384 frames / 8 ms; wet admission remains 768 frames / 16 ms.
This increases device latency and is a conservative bench setting, not acceptance
of the smaller-buffer latency target. Shutdown flushes the selected buffer length.
Failed playback cycles now contribute to host-service timing and get a separate
report field; earlier completed-cycle maxima omitted the failed cycle.

H2 renews the same selected device/link reservation through 19:00 UTC, each trial
still bounded to 660 s. The v7 30 s smoke retained 1.44 million frames with exact ADC/stem hashes,
dry/DAC replay and zero xruns or missing wet returns. Both 15 s fault trials
retained 720000 frames each with exact raw/dry/journal checks and no xruns.
Packet faults caused 299 missing/264 expired returns; restart caused 1136 missing
returns, three socket errors and two fresh snapshots. Both wet channels recovered: the final continuous nonzero wet intervals were
22272 frames (0.464 s) after the last packet fault and 408960 frames (8.52 s)
after restart. The known loss burst reached zero in 240 frames, within 0.821/0.767 PCM24 LSB of
the expected fade. Recovery is checked over the final 9600 source frames after
the last injected fault; a previous block-count window included another programmed
loss when the period doubled, so that original assessment is retained and corrected.
The deliberate 100 ms driver stall again retained an exact 96000-frame incomplete
take.

The **final native f64 v7 600 s soak passed**: 28.8 million captured, recorded
and fully submitted frames; zero detected xruns, missing/expired wet packets,
recorder/network/return queue drops or file errors. All 600000 packets returned.
Direct ADC byte hashes, all eight stored PCM hashes, the complete source journal,
dry replay and combined DAC-submitted replay match exactly. No ADC full-scale
samples or recorder clipping were observed.

Render p99/max was **1.123/4.949 ms** against an 8 ms period; complete post-read
host service was **1.290/5.233 ms**. Network RTT p99/max was **0.565/1.977 ms**;
Brain FX processing p99/max was 0.029/0.122 ms. Playback queue delay ranged from
2018 to 2646 frames (**42.042–55.125 ms**). Capture service intervals reached
17.053 ms; 37500 of 74999 exceeded the histogram's 10 ms range, so even p50 is
unavailable. Buffering absorbed that service variation in this bounded run.

Pi 5 process CPU ended at 20.2% of one core, maximum RSS 12.31 MiB and observed
temperature 57.3–61.7 °C. Pi 4 CPU was 13.37% of one core over its observed
589.71 s interval, RSS 4.04 MiB and temperature 50.1–54.5 °C. Peer observation
began about 14 s after process start. These values describe this stereo delay
configuration, not a full rack, maximum channel count or full-show guarantee.


## Subsequent physical loopback

After the final soak, the user reported cables from both main outputs to the
corresponding inputs, input gain at minimum, Mixer fully Playback and Main at
noon. H3 separately reserved this changed condition. The host did not alter those
physical controls. The AudioBox inputs are mic/instrument inputs; its
[manual, pages 3–4](https://pae-web.presonusmusic.com/downloads/products/pdf/AudioBoxUSB96_OwnersManual_EN_29122021.pdf)
warns against direct line-level sources. This user-installed connection was tested
with conservative generated levels, starting at −72 dBFS peak. Captured audio
never feeds playback in these physical trials.

A private generated-only ALSA probe sent independent left/right pseudorandom
bursts for eight seconds at 48 kHz, period 384/buffer 3072, initially −72 dBFS and
then −54 dBFS after reviewing levels. It stops on any ALSA fault or captured
sample above 0.02 FS. Both runs retained 384000 frames without faults. At −54 dBFS,
ADC peaks were −49.01/−67.71 dBFS, well below the stop level. The left output maps
to input1: both separated bursts have correlation 0.799 and the same 2739-frame
(**57.0625 ms**) reference-to-capture offset. This includes 2688 frames of output
prefill, separately issued stream starts, USB transfers and converters. It is a
buffered physical loopback observation, not isolated converter latency or a
calibrated capture-to-speaker measurement. Do not subtract prefill and call the
remainder converter delay. The small/noisy right correlation does not establish
a usable right-channel route or a reliable latency.

The final unmodified PA/FX/REC host then ran generated-only for 12 s with the
same 384/3072/768 configuration. A measured-level gate predicted a peak below
0.037 FS before the −36 dBFS source trial. All 576000 frames, native ADC hashes,
eight stems, dry/DAC replay and journal entries verified exactly; no xruns,
missing/expired wet returns or clipping occurred. Actual ADC peaks were
0.017325/0.000828 FS. Analysis of the stable 997/1499 Hz tones measured left return
gain −0.674 dB and right return gain −69.440 dB relative to each submitted output.
The right path is **68.77 dB below the left** and remains unresolved. Those tone
measurements do not distinguish a cable/connector/input issue from another
analogue route problem; submitted digital channels are both present. The user
suspects a faulty cable and directed further physical acceptance to use channel 1. Continuous
tones alone do not disambiguate delay.

Original raw captures, probe sources/hashes and sample/tone analyses stay private.
The first probe's elapsed field includes file-save time and its startup timestamps
precede the start calls; neither is a hardware timestamp. The second probe fixes
the elapsed measurement and labels call-before timestamps explicitly. Timing
claims above use sample correlation, not those first-run wall-clock fields.

## Build, test and evidence

Build each owner independently with Rust 1.97.1 and its committed lockfile, then:

```sh
CARGO_INCREMENTAL=0 cargo build --locked --release --features hardware-host --bin gigpies-hardware
target/release/gigpies-hardware --help
# Only during an explicitly authorized device/link reservation:
target/release/gigpies-hardware brain /private/brain-config.json
target/release/gigpies-hardware pa /private/pa-config.json
```

The JSON schemas are `PaConfig` in `src/host/device.rs` and `Brain` in the binary;
unknown fields fail. Explicit paths/endpoints, bounded duration, fresh epoch and
create-only report/take destinations are required. `stall_device_after_frames`
is an explicit bench fault injection; leave it null for ordinary measurements.
The private supervisor records exact commands/configurations and bounds each
process; it is not installed as a service or included in source publication.

The final normal suites passed: GigPies 181 Rust/37 Python, SHR PA 65 Rust/4
Python, SHR FX 101 Rust, and SHR REC 23 Rust tests. Four GigPies historical/media
or socket-capacity opt-ins were intentionally skipped because their protected
paths were unchanged; the directly affected FX cost matrix ran separately.
Formatting, warning-denied Clippy, locked release builds and publication checks
passed. The final ADC endpoint counter also ignores unused low container bits
for both signs, with focused endpoint tests. Earlier quiet captures were far from
full scale. A focused regression also rejects a fresh, non-late packet carrying the wrong
return budget. Candidate v7 adds bounded device-buffer validation and final
endpoint reporting; each measured binary has its own retained source manifest.

Normal tests cover wet routing/loss, PCM precision descriptors, bounded metrics
and all preceding transport contracts without devices. Owner suites cover ABI
capacity/precision, allocation freedom, resets, recorder gaps, disk failure,
partial publication and recovery after killing an owned child. Historical music,
auditions, exhaustive cost matrices and unrelated media studies remain opt-in.

Private evidence: `artifacts/audio-hardware/2026-10-03/`, including candidate
source/binary manifests, original captures, failed xrun take, trial commands,
opened ALSA parameters, sample verification, fault assessment and peer records.
No recordings or machine configuration enter source Git. No public push/release.

## Remaining acceptance

The left electrical return is verified at the stated buffered condition; the
right return remains about 69 dB weaker and needs a working physical route before
stereo channel/latency acceptance. Isolated converter latency and clock lock remain
unmeasured. The absent second interface prevents independent-device clock measurements. Calibrated microphone/speaker measurement and acoustic PA
acceptance remain separate. Full channel count, 18/64-channel recording, mixer
automation, live control parameter adapters, UI, analysis fanout, device hotplug
recovery and full-show reliability are not certified by this stereo bench.
