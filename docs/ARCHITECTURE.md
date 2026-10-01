# Intended architecture

Status: live node boundaries remain design direction. Offline soundcheck/rendering
is implemented in `src/automix/`; see [the automixer](AUTOMIX.md).

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
Wire format, synchronization, buffering and reconnection protocols are undecided.
RTP/UDP and PTP in the original concept are candidates, not existing dependencies.
The USB audio device supplies the local audio clock; network timing does not replace it.

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

## Boundaries to grow later

Keep offline source reading and analysis separate from DSP rendering, device transport,
control/UI, lighting and recording. Start in one Rust package; extract crates when an
actual integration boundary benefits from it. There are no empty live-engine crates.
Extra DSP nodes and venue business services remain optional future work.
