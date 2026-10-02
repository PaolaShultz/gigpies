# Status and next steps

## v0.1.0 — first public foundation

Implemented:

- Standalone Rust library and CLI; pinned toolchain and locked dependencies.
- Read-only WAV header inventory for a file or a flat directory, with JSON output.
- Synthetic regression tests and Linux CI; no hardware required.
- Architecture, source-material notes, dependency map and preserved concept sources.

## First offline automixer — implemented, offline-validated

- Synchronized native-rate streaming soundcheck with bounded activity-aware trim,
  linked calibration groups, causal history and explicit freeze/save.
- Editable role presets: HPF, parametric EQ, stereo-linked compression, pan/faders.
- BWF time-reference alignment, zero-padded tails and conservative master peak control.
- Frozen-settings rendering, full-song local A/B WAVs and loudness-matched copies.
- Measurements and gain/reduction histories; input clipping is reported, not repaired.
- Synthetic DSP/routing/persistence tests; opt-in deterministic private-media variations.

The first full-band experiment has been rendered and its file/measurement contracts
checked. Listening is the next evaluation; tests do not establish musical quality.
See [workflow, presets and limitations](AUTOMIX.md).

Not implemented: live audio or device transport, networking, web/TUI/controller,
recording, lighting integration, gates or true-peak limiting.
No live latency, acoustic safety, listening acceptance or Pi headroom is claimed.

Next: review the new manufacturer-reference SOURCE → OUR MIX comparisons and record
listener preference separately from policy compliance. Earlier rejected candidates
remain preserved as historical evidence. Hardware integration remains separate.

The [archived blueprint](archive/blueprints/blueprint-v2.md) retains the wider scope.

## Optional effects and automatic review pass

Implemented: separate plate/chamber/hall reverbs, vocal predelay validation, chorus,
filtered delay, oversampled excitation and modest master maximization. The Rust
`finish` command analyzes the preliminary mix, applies bounded spectral/return/dynamics
rules, rerenders and records its decisions and post-checks. Corrections are made by
code; no spectrogram interpretation or AI decision is in that loop.
[Commands, algorithms and limitations](FX_PASS.md). Listening acceptance remains pending.

## Unity-source revision

Implemented and offline-validated: DI-only bass, unity input trims and faders,
90 Hz channel HPFs except kick/bass, 40 Hz processed master HPF, measured compressor
thresholds and loss compensation, independent final PCM output leveling to −0.01 dBFS, preserved
float sums, and code-generated envelope/spectral reports. No loudness matching in
this workflow. [Commands and limits](UNITY_PASS.md). Listening acceptance remains open.

## Musical balance pass

Implemented and offline-validated: synchronized role-band and event measurements,
explicit tom-bleed uncertainty, microphone covariance, bounded static kit/group
fader search, held-out section regression checks, event-level compressor/master
reduction, and exact unmatched OLD/A/B excerpts. The local processing candidate
partially restores guitar low-mid cuts after rendering the fader-only candidate.
Initial policy targets remain hypotheses and some remain unmet; listener preference
is pending. No playback or hardware changes. [Workflow and limits](BALANCE_PASS.md).

## Manufacturer-reference experiment

Implemented and offline-validated: explicit low/high shelves, broad-Q bells,
traceable local Yamaha parameter mapping, unsupported-parameter audits and a bounded
training-only compression adaptation. Six local sessions were assessed; five received
full-song processing and separate fader experiments. Exact SOURCE → OUR MIX excerpts
and two independent produced-reference comparisons are ready locally. The normal
suite passes (41 Rust and four Python tests); listener preference remains unknown.

Only Complainiacs' fader proposal was accepted. Rainfall met its applicable balance
policy without a fader change; other band targets remain unmet, and Phoenix has no
applicable ensemble policy. Dark Ride's full-band render is deferred without a
verified bass DI. [Methods, reference-use limits and results](PRESET_EXPERIMENT.md).
Manufacturer collections, recordings, settings and detailed private evidence remain
local. No console emulation or hardware verification is claimed.

## Instrument tone preparation

Implemented offline: explicit known guitar input groups, intent-conditioned
body/presence measurement, a fixed-budget broad-EQ search, actual DSP validation and
held-out rejection. Original routing, faders and secondary paths are preserved.
The listener reported improved overall balance from the manufacturer-reference pass
but insufficient Complainiacs guitar body. The new pass addresses that automatically;
its full-song correction improved held-out body/presence by 1.6 dB while the
selected tone range remains unmet. Listening preference remains pending. Other fault
classes are future work; no ringing fault is asserted in this source. See [tone preparation](TONE_PASS.md).

## Source rules and source-first advice

Implemented offline: explicit numerical profiles with instrument/capture matching,
body/presence correction, sustained-compression relief, PCM full-scale-contact
abstention, combined-rule validation and separate remaining-target reports. Profile
names do not select processing. Large live-capture mismatches produce setup-specific
source-adjustment advice and retain current settings for a repeat soundcheck.
Historical recordings can receive a bounded improvement with limitations recorded.
Style catalogues, other instrument families, live musician UI and physical capture
control remain planned. See [source rules](SOURCE_RULES.md).

Offline validation: 68 Rust and four Python tests pass. Five local guitar pilots
and a full-song Complainiacs check preserve the existing accepted full-song settings;
a labelled injected compression fault demonstrates bounded partial relief. Reports
separate remaining target deviations from accepted changes. No new listening round
or playback was added. Detailed outcomes are in [source rules](SOURCE_RULES.md).

## Guitar decay revision

The prior ending veto treated a fading spectrum as steady tone. The new temporal
rule preserves dynamics checks on that fade while allowing a stronger static guitar
correction. Complainiacs now accepts +5 dB at 300 Hz / −3 dB at 2400 Hz; held-out
body/presence improves from −8.49 to −2.36 dB. One current → new mix pair is ready
locally. Listener preference is pending. The four other saved session pilots retain
their settings. 71 Rust and four Python tests pass. See [evidence, rules and
limitations](TONE_DECAY.md).
