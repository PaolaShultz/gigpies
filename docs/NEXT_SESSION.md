# Next session: listening review and planned hardware

## Current hardware handoff

Read [AUDIO_HARDWARE.md](AUDIO_HARDWARE.md) and its
[owning plan](AUDIO_HARDWARE_PLAN.md) before continuing. The selected USB host now
uses 48-frame / 1 ms processing, independent device capacity and zero silent
prefill, with real PA/FX/REC modules and sample-verified fault recovery. Do not
use the historical 56 ms prefill as live acceptance or infer card latency from
its 57 ms loopback. The final ten-minute working-channel run measured
5.19–5.23 ms, with exact software output and zero xruns/late wet returns. Two one-frame physical offset
changes remain unresolved; preserve that qualification. The next useful physical
check is a narrowly scoped USB transfer/feedback trace around those changes under
a fresh reservation. Existing one-second driver snapshots are too coarse to
identify their cause. Keep the direct audio buffers fixed while investigating.

Read the final H7/H8 gates and exact host/kernel conditions before extending the
tested scope. Earlier low-latency xruns, the traced locked-page migration stall and all
failed takes are retained. New measurements need current ownership, fresh
epoch/output paths and a mutually acknowledged reservation. Temporary settings
must be restored; no persistent low-latency system profile was installed.

The user connected stereo returns and authorized use of the working channel.
The right return is about 69 dB weaker and needs a working route before stereo
physical acceptance. The USB microphone remains absent. Acoustic PA acceptance,
clock measurements, mixer controls and UI integration remain distinct tasks.
Do not repeat unchanged baseline or music studies. The older offline/listening
and console notes below keep their original scope.

## Offline handoff — unreleased summing work, 2026-10-03

The [summing-engine plan](SUMMING_PLAN.md) has been implemented and offline-validated
on top of 0.2.2. The new [delivery contract](SUMMING_DELIVERY.md) separates comparison
gain and final limiting, observes production stages and verifies final true peaks.
The existing f64 summer, channel choices and requested GigPies effects are retained.
No supported new musical candidate was selected.

Use `artifacts/automix/summing-study/2026-10-03-engine/LISTEN.md`: six new complete
mixes at a −1 dBTP ceiling, seven exact established excerpts and historical
SOURCE/FINAL references reconstructed with exact PCM/float hashes. Independent
measurements range from −1.400441 to −1.395178 dBTP. The largest production versus
independent difference is 0.004922 dB. New deliveries use static gain on the same
selected buses, with 0.4001 dB reserved margin and no loudness target.

**Next action:** obtain a fresh playback go, then record preference by passage and
concern. Listening is not reviewed and hardware is unverified. Do not rerender merely
to open the queue. Do not rerun historical interleaved planners. Any new musical
study needs a declared hypothesis, fresh chronological passages and stopping rules.

Formatting, locked check/release build, Clippy, 154 normal Rust tests and 37 Python
tests pass. Three unrelated historical tests remain opt-in; the applicable 300-case
meter study passes. `REPORT.md` and `complete.json` beside the index bind the result.
Unused companion WAVs were removed and observation CSVs compressed losslessly;
`cleanup.json` records the exact scope. All 2,473 retained evidence files and metadata
for 428 original/library/test files are unchanged. Preserve these originals and
selected outputs. Earlier retired queues remain historical.

The work is unreleased. No playback, hardware, service or sibling
changes occurred. Publication would need its own authorized scope and the checks in
[publication policy](PUBLICATION.md).

The unresolved Pi crash still needs separate diagnosis; zero current filesystem
counters and boot recovery do not establish a complete offline scan. Keep siblings
read-only and PA measurement/alignment algorithms in `../shr-pa`.

## Planned hardware session

Recorded from the user's ideas on 2026-10-03, for tomorrow's hardware session.
**Planned only.** No MIDI mapping, network service, host-audio change or hardware
measurement has been performed or authorized by this note.

## A small musician-facing console

Combine a 14-inch screen with the user's small Arturia MiniLab controller.
The proposed control vocabulary is:

- Each keyboard key selects a channel; 12 keys make one octave of channel choices.
- Octave switches select additional channel banks so a small keyboard can address
  many inputs. Confirm how the exact controller model reports those switches.
- Eight coloured, pressable pads show states and trigger named actions.
- Sixteen rotary controls operate the selected channel's parameters.
- The screen shows the selected channel, bank, parameter names/values and pad states.

This could serve as the physical control surface for the larger GigPies system.
The controller's exact model, available controls, feedback messages and encoder/pot
behaviour still need inspection. The counts above describe the user's proposal.
Do not assume MIDI keys or octave buttons expose particular messages until tested.

Design questions for that session: visible bank/channel feedback, protection from
changing the wrong channel, knob pickup or relative encoder behaviour after a bank
change, an obvious reset, and a usable screen/keyboard alternative. Colour must not
be the only state indication. Channel selection must retain the instrument's verified
input-group assignment. A controller action's scope (shared mix, monitor or offline
preview) needs to be visible. The offline EQ amount/reset contract is a useful first
control to exercise without controlling live audio.

## Two-Pi transport: next integration gate

The [transport execution plan](AUDIO_TRANSPORT_PLAN.md) now records completed
research, implementation and two-node synthetic work. The
[protocol and evidence](AUDIO_TRANSPORT.md) select GPA1 UDP audio, separate
acknowledged UDP control and a PA-owned 48 kHz source-frame timeline.
SSH/Git remains the development channel. Read the accepted private ledger
records and obtain a new resource reservation before repeating load tests.

Next integrate SHR PA/FX/REC in their owning projects. Then, with explicit
hardware-session authorization, use muted outputs to verify 24-bit capture,
physical ADC/DAC/ADAT clocks and channel mapping, same-clock loopback latency
and gap-free NVMe recording while an owned Brain process is restarted.
Synthetic block counters do not establish physical dry-audio continuity.
Independent device clocks may require a reviewed asynchronous resampler.
No fixed runtime Pi assignment follows from identity-return benchmarks.

## Coordinating the two Codex sessions

Arrange a deliberate handoff workflow on the two Pis tomorrow. Start with a shared,
versioned task/interface document: one owner per file/module, exact repository and
revision, requested change, expected message/schema contract, reproducible commands,
results and unresolved questions. Exchange small patches or commits after reviewing
local work; do not let both sessions independently rewrite the same files or run
hardware actions concurrently. A task ledger can distinguish queued, running,
ready-for-review and accepted work, with explicit evidence attached to each handoff.

The installed `gigpies-peer` now launches bounded workers over pinned SSH.
Follow `/home/shome/p/AGENTS.md` and [the node lab](NODE_LAB.md); workers own
explicit scopes, while immutable ledger records retain separate review and
acceptance. Existing interactive sessions keep their work and ownership.
