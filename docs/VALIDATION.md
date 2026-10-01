# Foundation validation — 2026-10-01

Local host: AArch64 Linux; rustc 1.97.1 (8bab26f4f), LLVM 22.1.6.

Passed for v0.1.0:

- Formatting, locked Cargo check and warning-denied Clippy across all targets.
- Complete normal suite: four synthetic integration/CLI regression tests.
- Locked release build.
- Source archive SHA-256 matched the existing SHR Lux provenance before local extraction.
- Release CLI inventoried all 13 original Complainiacs WAVs: 15 audio channels,
  24-bit PCM, 44.1 kHz. Inventory is local and excluded from publication.
- Local Markdown link targets and publication boundary checks.

The WAV inventory reads headers; it does not validate the complete sample payload or
prove alignment. Archive extraction checked ZIP member integrity while reading.

No historical/exhaustive tests exist yet. Live audio, hardware, listening, long-running
benchmarks and automixing acceptance were intentionally outside this foundation pass.
GitHub Actions separately records the hosted Linux checks for the published commit.
