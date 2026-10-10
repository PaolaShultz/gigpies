# Brain duplex talkback and operator monitoring

Task0015 production paths passed integrated software validation on 2026-10-06. See the dated [acceptance record](../acceptance/BRAIN_AUDIO_ACCEPTANCE.md). This
document records the corrected architecture and operating bounds. No physical
sound-card activation is authorized in this increment.

Brain owns one local duplex endpoint, with explicit validated microphone and
stereo monitor mappings. Stagebox owns its interface and ADAT expansion. Each
node requires its declared local duplex clock relationship; the two nodes are
independent clocks unless physical evidence proves otherwise. Equal nominal
48 kHz rates are insufficient. Stagebox DSP, PA, raw REC, analysis and source-frame
FX stay on the Stagebox clock. Only talkback and operator playback cross clocks.

The implementation uses the existing Mixer, LocalAudio authority, owner module
graph and mutual-TLS QUIC transport. Talkback must enter after the FX-send tap and
before configured PA protection. It cannot enter raw REC/analysis or silently loop
into local monitoring. Operator selection cannot change audience/performer mixes.
Channel PFL is post EQ/compression, pre mute/fader/pan and centered; AFL is post
processing/mute/fader/pan and stereo. Performer sends retain their established raw
post-mute semantics. One listen source is selected at a time.

The preimplementation acceptance contract requires both drift signs at10/100/1000ppm,
long accelerated independent-clock runs beyond uncompensated buffer exhaustion,
network jitter without drift, stalls and discontinuities. Numerical targets are
≤0.1dB through18kHz, ≥80dB declared stopband rejection and channel leakage below
−100dBFS. Actual configured delay, ratio/slew, memory and occupancy bounds must be
recorded with measured results before acceptance. No sample duplication/drop
substitutes for asynchronous conversion. Raw samples remain exact before ASRC;
converted samples require numerical comparisons.

PTT uses an explicit owner, destinations and held-action generation. The target
heartbeat is50ms, maximum hold expiry150ms, with a≤240-frame closing fade at48kHz.
Lost key-up must close within155ms plus one admitted render quantum. Release,
focus/controller/lease/session loss, revocation and device faults close the path;
late messages cannot resurrect a released hold. Startup, reconnect and device
reopen require fresh readback, prefill and explicit rearm. Persist intent only.

A listen-selection boundary clears ASRC queues and silences all host playback
staging from the first frame not yet submitted to the device. Partial-write cursors
and the duplex clock remain intact. Frames already submitted cannot be recalled;
the configured device buffer (at most48000frames/one nominal second) is a separate
maximum tail, not part of the5ms ASRC fade. Actual playback delay and closure need
physical measurement with the chosen period/buffer. The150ms talkback deadman and
5ms closing fade describe the Stagebox render boundary, not acoustic latency.

Software completion requires actual Desk actions against production endpoints on
the other Pi at16/32/48 inputs, actual PA/FX/REC libraries, independent sample
checks, allocation/destruction guards, owner normal suites, independent final
review, publication/CI and exact twelve-repository/ledger receipts. The dated acceptance record supplies the software results; exact publication/CI
and receiving revisions are tracked separately in the private task ledger.

## Executable device composition

Both physical paths are connected to the authenticated processing host. Brain uses
`gigpies-brain --config PRIVATE.json --activate-physical`. Stagebox uses
`gigpies-remote --config PRIVATE.json --activate-physical` with a `provider` config
and an explicit `physical_device` object. Omitting that object retains the bounded
synthetic provider; the activation switch alone cannot select a device.

The object contains `device`, full validated `topology`, a `DeviceAcceptance`
`acceptance` record, `period_frames`, `buffer_frames` and an absolute `epoch_file`
in an existing private 0700 directory. The supplied input, monitor and PA counts
must match that topology. A reference factory is not physical mapping evidence.
The raw endpoint and matching capture/playback card, device and subdevice are
checked without fallback. The current executable admits a 48-frame period and
96–48000-frame buffer; this packet-sized implementation limit is not a channel
or product-capacity limit. Larger device periods need an explicit executable
extension and validation.

For a physical provider, `source_epoch` is the requested minimum: the locked,
durable ledger reserves at least that value and strictly more than its previous
reservation. Use the actual epoch returned in readiness/handshake/readback.
No stored arm or grant is restored. Configured physical duration is finite,
up to 24 hours; fake provider and fake duplex runs remain capped at 60 seconds.
The limited diagnostic windows report overflow rather than growing without bound.
An additional opt-in `provider_timing` trace is restricted to bounded synthetic
providers; see [transport diagnostics](REMOTE_TRANSPORT.md) for memory, correlation
and measurement limits.

The separate source-clock FX runner defaults to `verify_synthetic_source: true`,
which retains the 60-second synthetic reference checks. Explicit `false` accepts
arbitrary admitted source IDs and valid silence, preserves actual owner processing,
packet/channel counts and hashes, and reports no synthetic comparison result.
It admits an explicitly configured duration up to 24 hours and owns no PCM.
Its clock still comes from Stagebox source frames.

The shared physical pump refreshes authority time after capture and control work.
Unsubmitted playback must be written within one configured period after capture,
or the source quiesces and both PCM directions stop. Already submitted buffer
latency is separate. These paths are software-tested without opening a PCM;
physical timing and exact device negotiation remain unqualified.

## Bounded physical followup

1. Read-only inventory on both nodes: actual card identity, advertised capture and
   playback formats/rates/channels, physical socket labels, kernel/driver details.
   Record uncertainty and hardware-monitoring controls; do not infer headphone DACs.
2. Separately authorize one explicit duplex activation per node, with amplification
   disconnected or conservatively muted, phantom power and mixers unchanged unless
   specifically authorized. Validate same-endpoint identity and declared local
   shared clock, periods/buffers and exact physical maps against observed resources.
3. With conservative output level and a bounded low-level signal, verify each mapped
   socket and headphone/direct-monitor path. Confirm hardware monitoring does not
   bypass software mute or PA protection. Establish safe microphone/destination
   routing before any voice test; no automatic FOH destination.
4. Measure independent-device drift and bridge occupancy/ratio stability over a
   reserved finite duration exceeding predicted uncorrected FIFO exhaustion,
   calculated from measured skew and available FIFO headroom. Fake runs remain
   limited to60seconds; physical configuration may explicitly request up to24hours
   and still requires `--activate-physical`. No run extends itself. The bounded
   2048-window diagnostic sample trace can overflow during a long run; its overflow
   counter must be reported, and it is not a complete long-run measurement record.
   Arrange separately bounded measurement capture for the reserved duration.
   Capture electrical references for roundtrip and monitor delay; report method,
   converter/filter/buffer contributions, uncertainty and clock-source evidence.
5. Exercise PTT release/lost key-up, device/stall/unplug/reopen and network faults
   under safe output conditions. Confirm closure, fresh epochs, no stale audio or
   automatic rearm, continued Stagebox dry/protected/recorded paths as specified.
6. Qualify sustained deadlines for the actual configured graph and retained safety
   margins. Record sound quality separately. Synthetic results cannot prove acoustic
   safety, feedback immunity, physical clock lock or hardware qualification.

### Bounded monitor selection transition

While Brain observes a new selection/disarm identity, Stagebox retires the old
authenticated monitor stream with real source-clock silence. Host-owned queued
packet payloads become zero without removing or renumbering any source frames;
new selected audio requires a new negotiation. The tail lasts at most 250 ms or
12,000 source frames, whichever expires first, and requires the same live owner,
permission, source/device epochs/maps and fresh, healthy armed device observation.
Further route changes and retries cannot renew that deadline. Loss of these guards
closes the stream. Normal new-identity prefill and explicit rearm still apply.
Already emitted network packets and device-submitted samples remain outside this
local replacement boundary. This software behavior does not establish physical
mute latency or clock qualification; lifetime ASRC fault counters remain intact.
