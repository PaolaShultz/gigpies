# Intended architecture

## Implemented USB bench boundary

The [hardware host](AUDIO_HARDWARE.md) now opens one explicitly selected stereo
USB interface, runs independently built SHR PA/FX/REC libraries through versioned
C interfaces and records real samples on the PA node. Brain follows the PA's USB
source-frame timeline without an audio device. The render section uses bounded
module calls/queues; driver I/O and disk/network work stay outside it. This is a
measured stereo bench, while the broader live mixer/console below remains planned.
The host preserves dry processing and recording through Brain failure, fades both
wet channels and requires fresh control state. Device faults close the old take
as incomplete and require a fresh epoch. Prototype control does not yet change
real mixer parameters. Physical latency/acoustic acceptance remain separate.

Status: live node boundaries remain design direction. Offline soundcheck/rendering
is implemented in `src/automix/`; see [the automixer](AUTOMIX.md).
The [component map](COMPONENTS.md) records module ownership across the `../shr-*`
projects and their intended integration into GigPies.

## Stagebox / Mixer

Own physical audio I/O, channel processing, mixing, monitor buses, internal effects,
PA processing and local protection. Keep the monitor and main audio paths local.
Prepare structural changes outside the audio callback and apply bounded changes at
block boundaries. Continuous parameters need defined smoothing.

The live callback must avoid allocation, deallocation, locks, filesystem access,
logging and unbounded work. These are implementation requirements, not performance
claims about the current skeleton.

## Brain / Show

Own soundcheck analysis and prepared mix calculation, operator and performer controls,
lighting decisions, recording and show metadata. Send validated parameter/state
updates to the Stagebox. The Brain is outside the critical audio path.

The intended failure behavior is to preserve the last valid Stagebox mix and local
protection when Brain communication disappears. Reconnection and manual override
semantics must be implemented and tested when we build that boundary.

## Connection and timing

A dedicated Ethernet link carries source audio for analysis/recording and control
messages. Separate Wi-Fi serves performer access and optional venue services.
The [audio transport contract](AUDIO_TRANSPORT.md) selects GPA1 UDP unicast,
packed PCM24 analysis, float32 FX sends/wet returns and separate acknowledged
UDP control. PA owns the 48 kHz source-frame timeline; Brain follows it.
Packets carry epochs, channel groups and explicit wet output deadlines.
Bounded queues, loss fades and fresh-state control recovery are implemented
and synthetically tested; physical mixer/FX/recorder integration is pending.
The USB audio device supplies the local audio clock. No network clock or
resampler is installed; independent device clocks need measured ASRC acceptance.

## Audio and soundcheck

The intended live rate is 48 kHz. Preserve source recording rates and precision for
initial offline work; resampling will be a deliberate, documented preparation step.
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
