# Intended architecture

GigPies includes an **audio digital mixing surface and a lighting control surface**
for human operation. Both run on one Brain, with a separate Full-HD screen and
MIDI keyboard controller for each. Automation is an optional mode within them.

![GigPies system and module map](assets/architecture.svg)

## Implemented USB bench boundary

The [hardware host](AUDIO_HARDWARE.md) now opens one explicitly selected stereo
USB interface, runs independently built SHR PA/FX/REC libraries through versioned
C interfaces and records real samples on the PA node. Brain follows the PA's USB
source-frame timeline without an audio device. The render section uses bounded
module calls/queues; driver I/O and disk/network work stay outside it. This is a
measured stereo bench; the separately implemented local mixer and native consoles
have device-free software validation, described below.
The host preserves dry processing and recording through Brain failure, fades both
wet channels and requires fresh control state. Device faults close the old take
as incomplete and require a fresh epoch. Prototype control does not yet change
real mixer parameters. The host processes 48-frame / 1 ms blocks with independent
ALSA ring capacity and zero silent prefill; the dry path has no additional
application playout queue. Network and recording queues serve independent workers.
Channel-1 electrical delay and reliability are measured separately in the hardware
record; acoustic acceptance remains open.

Status: live node boundaries remain design direction. Offline soundcheck/rendering
is implemented in `src/automix/`; see [the automixer](AUTOMIX.md).
The [component map](COMPONENTS.md) records module ownership across the `../shr-*`
projects and their intended integration into GigPies.
The [Brain console integration plan](BRAIN_CONSOLE_PLAN.md) records the developing
`../shr-desk` audio and `../shr-lightdesk` lighting surfaces: two 1920×1080
monitors and two independently assigned MIDI keyboard controllers on one Brain,
local NVMe recording on PA and richer FX on Brain. The surfaces own their human
interfaces; GigPies owns integration/contracts and SHR Lux owns lighting authority.
Both surfaces now have optional native frontends and actual local provider
clients. The eight-input mixer, independent monitor sends, REC/FX/PA graph, named
analysis and null-output Lux authority are implemented and software-validated.
See [local integration](HEADLESS_INTEGRATION.md) for the contracts and evidence.
Physical console integration and production remote authentication remain pending.
The [preceding architecture draft](archive/architecture-before-brain-console-2026-10-03.md)
is preserved for context. The remaining sections describe the intended complete
system; software acceptance does not establish its physical topology.

## Stagebox / Mixer

Own physical audio I/O, f64 channel processing/mixing, monitor buses, PA processing,
local protection and multitrack recording to local NVMe. Keep the dry monitor and
main audio paths local. Own FX send taps and return routing; richer send/return
engines run on Brain, with a defined reduced local FX mode for Brain failure.
The small display serves local PA operation. Recording uses a bounded worker path
independent of Brain communication and the audio callback.
Prepare structural changes outside the audio callback and apply bounded changes at
block boundaries. Continuous parameters need defined smoothing.

The live callback must avoid allocation, deallocation, locks, filesystem access,
logging and unbounded work. These are implementation requirements, not performance
claims about the complete system's performance.

## Brain / Show

Two human-operated consoles run on Brain: audio in `../shr-desk`, lighting in
`../shr-lightdesk`. Each has its own Full-HD display/controller pairing, selection,
focus and independent process. Automation is a mode inside these consoles.
The audio desk displays applied state and sends scoped requests; the core mixes.
AUTO, ASSIST and MANUAL operation, parameter holds and explicit return to automation are
specified in its blueprint. Manual operation must not require an automixer.
Local mixer authority and bounded ramps are implemented; connecting that graph to
the complete physical Stagebox remains integration work. Physical Stagebox/PA
ownership does not make `shr-pa` the owner of a complete band-mixing engine.

Own soundcheck analysis and prepared mix calculation, both full-HD (1920 × 1080)
HDMI consoles, operator and performer controls, doctor/analysis services, the SHR
Lux lighting engine, richer send/return FX and shared show metadata. The audio
console commands and observes the PA recorder; audio is written on PA. Send validated parameter/state
updates to the Stagebox. Brain is outside the essential dry main/monitor path;
its FX have separate latency and failure behavior. Music-analysis extensions are
future work with their own task.

Lightdesk sends requests to SHR Lux; fixture evaluation, cue/effect execution,
lighting arbitration and physical output remain in Lux. Its first static mock
authority is retained as a simulator; the real local client uses Lux
fixture/programmer/Hold/cue authority with null output. MANUAL operation is independent of
audio-reactive automation. Shared show identity and named cross-system cues do
not grant one desk authority over the other. Display/controller assignment uses
verified roles and independent workers; lighting failure/redraw must not block
audio input, real-time processing, protection or recording. See the
[dual-console integration backlog](BRAIN_CONSOLE_PLAN.md) for missing contracts,
combined unverified budgets and separately scoped physical acceptance.

The intended failure behavior is to preserve the last valid Stagebox mix, local
protection and recording when Brain communication disappears. Define wet-return
fades and local fallback FX separately. A UI restart must refresh authoritative
state without recalling an old mix. Reconnection and manual override semantics
are tested for the local provider boundary; production remote and physical
recovery require their own acceptance.

## Connection and timing

A dedicated Ethernet link carries source audio for analysis, FX sends/returns and
control/status messages. Primary multitrack recording stays local to PA. Separate
Wi-Fi serves performer access and optional venue services.
The [audio transport contract](AUDIO_TRANSPORT.md) selects GPA1 UDP unicast,
packed PCM24 analysis, float32 FX sends/wet returns and separate acknowledged
UDP control. PA owns the 48 kHz source-frame timeline; Brain follows it.
Packets carry epochs, channel groups and explicit wet output deadlines.
Bounded queues, loss fades and fresh-state control recovery are implemented
and synthetically tested. The stereo USB host integrates PA/FX/REC modules;
the separate eight-input local mixer and both console adapters are implemented.
Full multichannel physical integration and wider mixer controls remain pending.
The USB audio device supplies the local audio clock. No network clock or
resampler is installed; independent device clocks need measured ASRC acceptance.

## Audio and soundcheck

The intended live rate is strictly 48 kHz, with 24-bit capture and f64 mixer DSP.
The stereo bench records PCM24 through SHR REC; broader show/tap persistence
contracts remain integration work. The network format is defined above.
Preserve source recording rates and precision for initial offline work; resampling
will be a deliberate, documented preparation step.
The reference hardware in the blueprint is UMC1820 + ADA8200 over ADAT, subject to
physical channel, clock and latency verification. Do not infer routing from enumeration.

Soundcheck produces a prepared baseline. Live rules react to meaningful exceptions
within defined bounds. Musical intent, mix objectives and correction policies will
be worked out through small experiments on real tracks.
A muted-PA pass can analyze source signals; room response, feedback and speaker
alignment require their own measurement/setup workflow.
PA measurement and alignment are developed in SHR PA. Its pending task adds
synchronized reference/mic capture and phase analysis to its existing generator,
pair delay/polarity and crossover DSP. GigPies is intended to integrate the finished
PA module; see [ownership and integration](COMPONENTS.md#pa-module-and-planned-integration).
This remains separate from artistic instrument EQ matching. Live integration is pending.

## Boundaries to grow later

Keep offline source reading and analysis separate from DSP rendering, device transport,
control/UI, lighting and recording. Start in one Rust package; extract crates when an
actual integration boundary benefits from it. There are no empty live-engine crates.
Extra DSP nodes and venue business services remain optional future work.

## Proposed instrument-station soundcheck

Design direction, not an implemented web/monitor feature: each instrument station
has a separate local QR address. Setup maps that station to its known primary and
secondary inputs, performer, stage position and monitor output. Scanning it opens
the correct instrument and an immediately editable default monitor mix. The
performer supplies instrument/style/tone intent and plays a soundcheck; the Brain
measures the known sources and prepares an editable first pass, followed by a band
context check. The offline [tone pass](TONE_PASS.md) implements the first limited
intent-to-processing step for guitar body and presence.

Personal monitor sends/tone and shared source/FOH processing need distinct control
scope. Performer edits must survive subsequent automatic analysis. Multiple phones
may remain connected and monitor controls remain available during soundcheck;
acoustic measurement steps must be coordinated. Monitor calibration belongs to the
mapped speaker and physical location and requires measurement; a stage-position
label or an initial visual EQ choice is not acoustic calibration. Phone/Brain loss
must preserve the Stagebox's last valid audio state.

### Review after soundcheck

The proposed [musician review after soundcheck](PERFORMER_REVIEW.md) uses existing
station connections to collect preferences for each instrument. Once everyone is
ready, the Brain prepares a shared ensemble preview for phones/headphones. Explicit
“keep my sound” and deterministic controls come first; optional model assistance is
future research. Preview acceptance remains separate from applying a live revision.
This extension, like the station interface, is planned.

### Source-first preparation

The [offline source-rule coordinator](SOURCE_RULES.md) can request a source
adjustment when confident measurements imply substantial EQ, or an input-path
review when PCM repeatedly touches full scale. For a known amp mic, distinguish
what the player hears at the amp from what the microphone captures: compare both,
change one amp/placement variable if needed, and repeat the same soundcheck phrases.
No physical cause is inferred from the spectrum alone. A live source-review request
keeps existing processing while waiting for a fresh capture; personal monitor
adjustments remain available. The current implementation produces the advice in
local reports; web delivery, capture-state coordination and hardware integration
remain planned.
