# Foundation validation — 2026-10-01

## 0.2.2 publication checks — 2026-10-03

Rust 1.97.1, committed lockfile and `CARGO_INCREMENTAL=0`:

- Formatting, locked check, all-target Clippy with warnings denied and optimized
  build passed. The executable reports `gigpies 0.2.2`; package versions agree.
- Complete normal Rust suite: **145 passed**, three historical private-media tests
  intentionally ignored. Complete normal Python suite: **34 passed**.
- The complete staged tree passes the publication guard with versioned hooks enabled.
  Reviewed source, tests and documentation are staged explicitly. No private settings,
  recordings, generated audio or one-off runner is published.
- Cargo source-package listing contains no private/media paths. Staged bytes match
  the reviewed working files; local documentation links and whitespace checks pass.

The release includes the existing paired EQ diagnostic and the new FX calibration,
master/coverage, recovery and saved-report work. It does not implement the separately
planned summing/true-peak engine changes or claim a listener preference.

The user explicitly retired all old generated renders before the next engine study:
234 regular render paths and 52 links, recovering about 4.19 GiB. Original media,
frozen settings, hashes and scalar evidence remain. Checks preserved 428 original/
library/test-input metadata records and 2,473 nonaudio evidence files. The private
retirement manifest is under `artifacts/automix/render-retirement-2026-10-03/`.

No new real-audio evaluation, full-song render, playback, exhaustive historical
study or hardware test was run for this publication. Synthetic tests use temporary
media and clean it up. Filesystem counters remained zero; the prior crash cause
is unresolved and a forced offline full scan has not been established.
Publication logs are under `artifacts/publication-0.2.2/`. Remote CI is separate
from these local results and must be checked for the pushed revision.

## 0.2.1 publication checks — 2026-10-03

Rust 1.97.1, committed lockfile and `CARGO_INCREMENTAL=0`:

- Formatting, locked check, warning-denied all-target Clippy and release build passed.
- Complete normal Rust suite: **126 passed**, including nine matching regressions;
  three historical private-media tests intentionally ignored. Complete normal
  Python suite: **34 passed**.
- Release executable reports `gigpies 0.2.1`; manifest and lockfile versions agree.
- The complete staged tree passes the publication guard. The local review page's
  inline script was reviewed and added to the explicit script list; it downloads
  a selection request and has no network, playback or hardware control path.
- Cargo source-package listing contains no private directories, recordings or audio
  archives. Changed-document local link targets and staged whitespace checks pass.

The [matching evidence](EQ_MATCHING.md#validation-and-bounded-pilot--2026-10-03)
records synthetic recovery, the real pilot's conservative miss and preserved mix
comparators. New full-song auditions, historical exhaustive renderers, playback and
hardware checks were intentionally skipped for publication. Musical acceptance is
pending; no true-peak claim is made. Existing recordings, maps, private evidence and
complete exports remain local. Publication logs are retained under ignored
`artifacts/publication-0.2.1/`.

Versioned commit/push hooks are enabled locally. Remote CI is separate from these
local results and must be checked for the pushed revision.

## 0.2.0 publication checks — 2026-10-03

Rust 1.97.1, committed lockfile and `CARGO_INCREMENTAL=0`:

- Formatting, locked check, warning-denied all-target Clippy and release build passed.
- Complete normal Rust suite: 117 passed; three historical private-media tests
  intentionally ignored. Complete normal Python suite: 34 passed.
- Release executable reports `gigpies 0.2.0`; application and lockfile versions agree.
- Complete staged Git tree and existing commit history passed the publication
  guard. No tracked path matches the private/generated ignore rules. Synthetic
  ignore probes cover user state, nested artifacts, recordings, caches and media.
- Cargo source-package listing contains no private directories, recordings or
  generated archives. No binary release attachment or local audio is published.
- Publication regressions cover force-added private paths, unreviewed scripts,
  executable files without an extension, staged secrets hidden by clean working
  copies, renamed audio, symlinks and leaks deleted by a later outgoing commit.

Versioned commit/push hooks are enabled locally; CI also checks publication
boundaries. Fresh clones need the documented [hook setup](PUBLICATION.md).
The script list contains maintained tools and synthetic tests. One-off session
runners, recordings and complete listening exports remain ignored local data.

Historical media auditions, new full-song rendering, benchmarks, playback and
physical hardware checks were intentionally skipped for publication. Prior media
evidence is retained in its owning documents; this release check does not renew
listener acceptance, PA validation or a true-peak claim. Remote CI is separate from
these local results and must be checked for the pushed revision.

## Original foundation evidence

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

## Unity-source revision — 2026-10-01

- All 28 normal synthetic tests passed, including five new unity routing, shared
  peak export, measured compression, partial-window envelope and workflow tests.
- Formatting, warning-denied Clippy and locked release build passed.
- Full local DI-only band pass rendered twice to fresh destinations. Code checked
  envelope and spectral measurements; maximizer reduction remained about 2 dB.
- Local PCM export verification checks finite samples, length, sample headroom and
  identical measured export gain against the preserved float sums.
- Three older private-media tests were intentionally skipped: their historical
  matched comparisons/source variations do not describe this new workflow.

Measurements do not establish listening acceptance. See [UNITY_PASS.md](UNITY_PASS.md).

## Independent final output level correction

Unmatched exports now use their own measured sample peaks, with a −0.01 dBFS
ceiling in the unity workflow. The regression deliberately lowers the processed
bus by 6 dB and verifies that final export compensates it independently of raw.
All 29 normal tests passed (including the focused silence regression), with Clippy; old private matching
auditions remain intentionally skipped. A fresh full-song render checks both PCM
peaks and finite output. Raw float samples are identical to the preceding pass;
processed differs by at most 1.5e-8 after JSON settings reload (floating rounding).
