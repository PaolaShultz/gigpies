# Independent mix-reference review and prepared playback

Implemented offline, 2026-10-02. `reference-review` compares a finished local mix
with user-supplied audio. It measures differences and checks timing; it never
selects EQ, compressor settings, faders or loudness targets. A released or “official”
mix is comparison material, with its own production decisions and possible faults.
The filename does not establish provenance or quality.

## Repeatable command

```sh
CARGO_INCREMENTAL=0 cargo build --locked --release
target/release/gigpies reference-review ours.wav supplied.wav new-review 24
```

The optional last argument requests a 12-second passage beginning at that position
in our mix. The output directory must be new. The tool writes `review.json` and
native-rate measurements for both files. When both global timing and the requested
passage pass the confidence checks, it also writes exactly:

1. `01-OUR-MIX.wav`
2. `02-SUPPLIED-REFERENCE.wav`

These retain each file's sample format, rate and original gain. The reference is
not peak-normalized or loudness-matched. PCM samples are copied exactly; different
native rates are supported. The timing search changes the excerpt's start position,
not its playback speed. Without sufficient confidence, reports are written but
comparison clips are withheld. The command itself never starts playback.

## Timing evidence and refusal rules

- Stream mono/stereo 16/24/32-bit PCM or 32-bit float at 8–192 kHz. Inputs are limited
  to ten minutes; malformed/nonfinite samples and changed files fail the run.
- Measure 20 ms windows using rational native sample boundaries. Create an event
  fingerprint from consecutive RMS changes, clipped to ±12 dB. Windows below
  −65 dBFS contribute no event evidence. Input audio remains unchanged.
- Search offsets within ±20 seconds for each full 12-second section, at 20 ms
  resolution. Pearson correlation is gain-invariant numerical comparison, not
  gain compensation applied to an audio copy.
- A local match needs correlation at least 0.35, and a margin of at least 0.10
  over the strongest alternative more than 200 ms away. A best match touching
  the search boundary is withheld.
- Require three reliable sections and at least 75% of the sections to be reliable.
  All reliable offsets must agree within 60 ms. Disagreement is reported as an
  inconsistent timeline, which can arise from edits, drift or repeated passages.
- The requested excerpt gets its own local search and must agree with the global
  offset. Weak sections do not inherit a confident verdict from other passages.

These thresholds are initial engineering choices informed by this development
example and synthetic tests. They are not calibrated probabilities. Repeated
rhythms, nearly constant levels, silence, very quiet material, another performance
or edits can defeat the method. Sub-20-ms offsets and small timing changes are not
resolved. A consistent event-envelope offset is not proof of sample alignment or
an identical performance. BWF timestamps are not used to override content matching.

## What is measured

Aligned sections report RMS, sample peak, median 20 ms crest, L/R correlation,
mono-sum power loss, and band powers at 40–100, 100–400, 400–800, 800–3200 and
3200–12000 Hz. The upper analysis limit is shared between both sample rates.
Spectral power is averaged across L/R; opposite-polarity channels cannot disappear
through an analysis mono fold-down. Mono compatibility is measured separately.

`reference_minus_ours` contains explicit deltas. Band-share differences are relative
to each mix's own broadband power; they distinguish spectral distribution from a
simple level difference without changing listening gain. These remain **whole-mix**
measurements. Bass, kick, guitar, room and processing can contribute to the same
band; a difference cannot identify an instrument or justify a guitar correction.
`processing_recommendation` and listener preference remain unset.

PCM full-scale contacts are counted separately from float headroom. A handful of
contacts does not establish sustained clipping, a faulty source or which processing
stage caused them. Lower crest is a measured shape difference, not proof of
excessive compression. No true-peak, source-separation or musical-quality claim is
made. Source repair continues to use [known inputs and profiles](../../guides/SOURCE_RULES.md).

## Playback: finish preparation first

```sh
python3 scripts/play_pair.py new-review/01-OUR-MIX.wav new-review/02-SUPPLIED-REFERENCE.wav \
  --target YOUR_PIPEWIRE_SINK_NODE_NAME --gap 1
```

Only invoke this when playback is requested. Before starting either clip, the
helper reads and verifies both PCM payloads, rejects duplicate/unequal-duration
clips, and checks that the explicit sink and `pw-play` are available. It then runs
one clip, waits the requested gap, and runs the other in the same process. There
are no assistant/tool round-trips or media preparation between them. The one-second
gap is between player processes; Bluetooth buffering may affect the audible gap.

No volume, default device, pairing, service or host-audio setting is changed. A
missing sink prevents playback; a failed first clip stops the sequence without a
retry or second clip. Hardware is never opened by the normal tests.

## Local Complainiacs result

User-supplied `The Complainiacs - Etc.` was found among accidentally added root WAVs.
Unrelated files were left untouched. Its provenance is recorded as supplied and
described by the user as the official mix; the source/mastering chain is unverified.
The user's dislike of this mix is recorded separately from technical measurements.
All media stays ignored and local, including incomplete downloads. Nothing from
this work was pushed.

The supplied file lasts 125.18 seconds; our export lasts 114.24 seconds. Eight main
song sections support an offset of +0.08 to +0.10 seconds. The 96-second fading
section is ambiguous and does not enter the aligned comparison. The requested
24-second passage matches at 24.10 seconds in the supplied file.

| Same musical passage | Our final | Supplied mix |
|---|---:|---:|
| RMS | −17.93 dBFS | −12.79 dBFS |
| Sample peak | −1.88 dBFS | 0.00 dBFS |
| Median 20 ms crest | 9.75 dB | 8.35 dB |
| 100–400 / 800–3200 Hz power | 3.88 dB | 7.13 dB |
| L/R correlation | 0.95 | 0.92 |

The supplied passage has about 5.13 dB higher RMS. Relative to each mix's broadband
power, it has about 3.49 dB more 40–100 Hz energy, almost the same 100–400 Hz share
(−0.31 dB difference), and less 400–800 Hz / presence energy (−4.84 / −3.56 dB).
Thus its higher whole-mix body/presence ratio does not establish fuller guitars.
No source EQ was changed to pursue those reference measurements.

Fourteen supplied PCM samples touch full scale across the complete file. That count
alone does not diagnose the master. The prior guitar correction remains the one
validated against source evidence in [TONE_DECAY.md](../../guides/TONE_DECAY.md).

Local evidence is in `artifacts/automix/reference-review-v1/`, including provenance,
file hash, reports, exact aligned excerpts and test logs. Final reports/clips are
in `review-final/`. The requested raw → final replay and the separate final →
supplied-reference comparison both completed as prepared pairs with one-second
process gaps. Playback completion does not establish listener preference. The initial complete
release review took 1.72 seconds on this machine, including feature extraction,
matching and output. This is one observed offline run, not a live latency guarantee.

## Validation

76 normal Rust tests and eight Python tests pass, alongside Clippy, formatting and
a release build. Three historical private-media tests remain ignored; this supplied
reference was evaluated explicitly. New regressions cover gain changes, leading and
removed audio, silence, quiet material, unrelated audio, repetitive patterns, edits,
drift, search boundaries, insufficient duration, opposite-polarity stereo, unequal
native rates, exact excerpt samples, nonfinite float input, duration limits, unchanged
inputs, output non-overwrite, preflight failures and playback order without hardware.

This adds repeatable comparison and exception handling. It does not turn one disliked
reference into a training target or a general musical preference model.

The supplied WAV has since moved into `../waves`; use the shared paths in
[LOCAL_MEDIA.md](../../development/LOCAL_MEDIA.md). Historical report paths are preserved in the
relocation manifest.
