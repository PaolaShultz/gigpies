# Status and next steps

## v0.1.0 — first public foundation

Implemented:

- Standalone Rust library and CLI; pinned toolchain and locked dependencies.
- Read-only WAV header inventory for a file or a flat directory, with JSON output.
- Synthetic regression tests and Linux CI; no hardware required.
- Architecture, source-material notes, dependency map and preserved concept sources.

Not implemented: sample analysis, automatic balance, mix rendering, live audio,
network transport, web/TUI/controller operation, recording or lighting integration.
No live latency, listening quality, acoustic safety or Pi headroom is claimed.

## First development thread: automixing

Start with The Complainiacs — Etc: 13 WAV files, including stereo drum tracks.

1. Inspect original files and establish their alignment and usable contents.
2. Measure activity, peaks, average levels and spectral behavior without altering audio.
3. Produce a simple, reproducible balance and a local stereo comparison render.
4. Listen, assess and refine one processing decision at a time.
5. Bring useful DSP from the related projects into explicit, tested integration paths.

This is a sequence of small experiments, not a commitment to automatic EQ, gates,
compression or effects before the first balance is understood. The 40-track Dark Ride
session is additional material when useful. Live hardware integration follows later.

The [archived blueprint](archive/blueprints/blueprint-v2.md) retains the wider scope.
