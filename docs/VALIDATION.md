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

## First offline automixer — 2026-10-01

Passed on the same pinned Rust toolchain:

- Complete normal suite: 14 synthetic DSP, file, schema, persistence, alignment and
  CLI tests. Calibration prefix equality checks causality; linked stereo tests check
  image preservation; output checks enforce finite samples and sample-peak headroom.
- Formatting, locked check, warning-denied Clippy across all targets and release build.
- Explicit opt-in local source variations: low/high levels, silence, pauses, jumps and
  clipped material. Trim stays bounded, silence does not raise trim, clipping is reported.
- Complete native-rate band soundcheck with an explicit stop, then a frozen render.
- Exported full-song listening files remeasured for matched loudness, length and
  headroom. All 13 original WAVs compared byte-for-byte by hash against the archive.

Private-media tests remain ignored in the normal suite and were run explicitly for
this change. [On-demand commands](AUTOMIX.md#tests-and-limits) are documented. No
historical/exhaustive benchmark suite was needed. Detailed measurements, source hashes,
prepared settings and audio are ignored local artifacts, not publication content.
Listening, hardware acceptance, true-peak validation and live-device tests remain
unperformed; these tests do not establish musical quality or acoustic safety.

## Effects and deterministic review pass — 2026-10-01

- Complete normal suite passed: 23 synthetic tests, including oversampled excitation,
  distinct reverb decay, vocal predelay constraints, deterministic wet routing,
  maximizer linking/bounds and the full automated review/save/compare workflow.
- The spectral policy accepts sustained synthetic buildup and rejects silence,
  smooth noise, brief peaks and protected bass fundamentals. Corrections are bounded.
- Formatting, locked check, warning-denied Clippy and release build passed.
- Explicit full-band `finish` experiment completed. Its Rust code selected and logged
  spectral/return corrections, then rendered and checked the result. No plot-derived
  correction was supplied to that algorithm. Detailed findings remain local.
- The opt-in FX export check passed for timeline, equal loudness and clipping/headroom.
- Rendering the original settings with effects disabled reproduced all four first-pass
  WAVs byte-for-byte, verified locally by SHA-256.

The normal suite ignores all three private-media tests. Only the new FX export check
was explicitly invoked in this pass. Historical source variations and the old export
check were intentionally skipped; a direct legacy equivalence check covered the
changed renderer. No playback, hardware or true-peak acceptance was performed.

```sh
cargo test --locked --release --test private_automix fx_pass_listening_files -- --ignored --nocapture
```

See [FX_PASS.md](FX_PASS.md) for reproducing the automatic review and local comparison.
