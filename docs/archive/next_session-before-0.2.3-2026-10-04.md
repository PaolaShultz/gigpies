# Next session: listening review and planned hardware

## Dual-surface handoff — 2026-10-04

Read [Brain integration](../architecture/BRAIN_CONSOLES.md) and the new
[Lightdesk status](../../../shr-lightdesk/docs/STATUS.md). Brain is two Full-HD
monitors and two separately assigned controllers: audio Desk plus lighting
Lightdesk. Native surface/focus work and a read-only Lux adapter are next; Lux
still needs its own programmer/cue/fixture contracts. Do not infer physical DMX
or combined-load acceptance from Lightdesk's offline mock. Earlier single-console
notes below are historical and do not narrow the dual-console product.

## Audio surface handoff — SHR Desk, 2026-10-04

The Brain surface now lives in `../shr-desk`. Read its README, status and blueprint
through [the integration index](../architecture/BRAIN_CONSOLES.md). Its first Rust simulator
and three full-HD screen drafts are offline-validated. Next implement the native
renderer and complete controller navigation, then agree the real GigPies mixer
control API. Native HDMI/MIDI and audio integration remain separate gates; the
historical console notes below do not supersede the new owning plan.

## Current hardware handoff

Read [AUDIO_HARDWARE.md](../acceptance/AUDIO_HARDWARE.md) and its
[owning plan](plans/AUDIO_HARDWARE_PLAN.md) before continuing. The selected USB host now
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

The [summing-engine plan](plans/SUMMING_PLAN.md) has been implemented and offline-validated
on top of 0.2.2. The new [delivery contract](../guides/SUMMING_DELIVERY.md) separates comparison
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
[publication policy](../development/PUBLICATION.md).

The unresolved Pi crash still needs separate diagnosis; zero current filesystem
counters and boot recovery do not establish a complete offline scan. Keep siblings
read-only and PA measurement/alignment algorithms in `../shr-pa`.

## Planned Brain console and hardware follow-up

The [Brain console plan](../architecture/BRAIN_CONSOLES.md) owns the future interface task.
The Brain requires **1920 × 1080 HDMI**. The small display is for the PA unit.
The intended interface combines a native GPU renderer, a TUI-style visual design,
pixel analysis panels and a small MIDI keyboard/controller. PA owns the mixer,
protection and local NVMe recording; Brain owns richer FX and show services.

**Planning only:** no console implementation, dependency installation, worker
dispatch, MIDI mapping or hardware operation is authorized by this document.
The first future slice is the headless state/command contract and synthetic data,
followed by a full-HD offline console. Live modules are separate dependencies.
The [earlier hardware-session draft](next-session-before-brain-console-2026-10-03.md)
is preserved; its small-screen Brain proposal is superseded.

### Proposed controller vocabulary

The user's Arturia MiniLab idea remains a candidate control surface:

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

The [transport execution plan](plans/AUDIO_TRANSPORT_PLAN.md) now records completed
research, implementation and two-node synthetic work. The
[protocol and evidence](../reference/AUDIO_TRANSPORT.md) select GPA1 UDP audio, separate
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

Use the established private ledger and a versioned task/interface document:
one owner per file/module, exact repository and
revision, requested change, expected message/schema contract, reproducible commands,
results and unresolved questions. Exchange small patches or commits after reviewing
local work; do not let both sessions independently rewrite the same files or run
hardware actions concurrently. A task ledger can distinguish queued, running,
ready-for-review and accepted work, with explicit evidence attached to each handoff.

The installed `gigpies-peer` helper now launches a new bounded Codex worker on the
other Pi over pinned SSH and returns its reply. Follow `/home/shome/p/AGENTS.md`
and [the node lab](../development/NODE_LAB.md). The private Git ledger records task states and
explicit review/acceptance. There is no graphical Kanban or automatic wake-up of an
existing interactive session. Workers receive exact scope and revisions; existing
interactive sessions retain their ownership. No worker was dispatched for this plan.
The later transport task used bounded workers and synthetic load tests.
Runtime hardware assignments and physical audio acceptance remain open.
