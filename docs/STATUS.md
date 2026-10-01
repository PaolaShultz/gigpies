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
recording, lighting integration, gates/effects or true-peak limiting.
No live latency, acoustic safety, listening acceptance or Pi headroom is claimed.

Next: audition the matched pair, assess vocal/snare/bass/guitar balance and revise one
setting at a time. A second source session and hardware integration follow when useful.

The [archived blueprint](archive/blueprints/blueprint-v2.md) retains the wider scope.
