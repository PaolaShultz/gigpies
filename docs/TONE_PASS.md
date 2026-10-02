# Instrument tone preparation

Implemented and tested offline for guitar body/presence correction. Source identity
comes from setup: one instrument may have a primary microphone and secondary paths.
The algorithm does not infer a room microphone, microphone position, or independent
performance from filenames. It does not classify source quality.

This pass follows listener feedback that the Complainiacs guitars lacked body even
after the manufacturer-reference mix improved overall balance. The earlier role
labels `rhythm_guitar` and `lead_guitar` did not establish separate performances.
The local experiment now explicitly treats these files as two paths for one guitar.

## Commands

```sh
cargo build --locked --release
# Analyze, choose a correction, and validate actual DSP; no audio export:
target/release/gigpies tone-analyze prepared.json sources new-analysis tone-policy.json
# Same preparation, then render one final mix using accepted corrections:
target/release/gigpies tone-pass prepared.json sources new-result tone-policy.json
```

Both commands require a new output directory. They preserve all prior settings and
renders. The source session must satisfy the unity-source contract: prepared,
unmatched −0.01 dBFS sample-peak export, unity input trims, 90 Hz channel HPFs except
kick/bass, 40 Hz master HPF and no bass amp. Effects must be disabled for this initial
pass; it has no FX-return validation. Neither command plays audio.

Example policy (indices and filenames must match the supplied session):

```json
{
  "instruments": [{
    "name": "guitar station",
    "primary": 8,
    "primary_file": "10_ElecGtr1.wav",
    "secondary": [9],
    "secondary_files": ["11_ElecGtr2.wav"],
    "intent": "balanced"
  }]
}
```

The primary path receives correction. Secondary paths contribute to the actual
coherent stereo sum used for validation, but their processing and level remain
unchanged. Every index/file mapping is checked. Groups cannot overlap; at most eight
instruments and four secondary paths per instrument are supported. Two free channel
EQ slots are required. Original EQ remains in the audit; corrections are appended.
Use a saved baseline for each experiment, rather than chaining repeated passes.

## What “thin” means here

We measure **100–400 Hz power relative to 800–3200 Hz power** after the existing
channel EQ/compressor and routing. This is a tone feature, not a universal good-sound
rule. Intent supplies the acceptable range:

| Intent | Body/presence range |
|---|---:|
| `balanced` (default) | −2 to +4 dB |
| `thin` | −12 to −4 dB |
| `dark` | +1 to +7 dB |
| `full` | +2 to +8 dB |

These are our initial engineering hypotheses, not manufacturer presets, calibrated
perceptual scores, or genre-specific voicings. “Dark” currently means a higher body
ratio; it does not model every aspect of dark guitar tone. A musician's explicit
thin intent changes the target instead of being treated as a fault.

## Measurement and decision

- Read native-rate audio with existing BWF alignment and production linked channel
  DSP. Preserve pan, timing, stereo channels, faders, trims and makeup gain.
- Analyze non-overlapping 8192-sample Hann windows, summing left/right spectral power
  without folding stereo to mono. The instrument sum includes microphone phase and
  covariance before its FFT. Incomplete trailing windows are omitted from analysis,
  while the renderer retains the complete song.
- Even 12-second sections select the correction; odd sections validate it. Windows
  straddling those boundaries are excluded, including from training activity statistics. Activity
  uses primary raw RMS above −65 dBFS and within 24 dB of its **training-only** p95.
  Combined secondary routed energy must be at least 3 dB below primary energy.
  Each comparison band must contain at least 1% of primary spectral power.
  Sparse notes/decays with too little energy in either band are insufficient
  evidence for this broad-tone judgement; their dynamics still need separate review.
  Body/presence below −30 dB also fails eligibility.
- Require at least three windows and three active seconds per split by default.
  At least 70% of eligible training windows must agree on the direction of the
  deficit/excess. This consistency fraction is a heuristic, not a probability.
  Silence, insufficient evidence, dominant secondary paths and inconsistent tone
  produce no correction. Quiet playing above the activity floor remains eligible.
- Search a fixed 546-position grid: broad Q=0.7 bells centred at 160/220/300 Hz,
  with gains bounded to ±6 dB, and 1600/2400 Hz, bounded to ±3 dB. Gains move in
  1 dB steps. Only directions appropriate to the measured deficit/excess are scored.
  The objective is squared distance from the intent range plus `0.025 * sum(gain²)`.
  No change wins ties. At most two bands are appended. Before scoring a candidate,
  predict every eligible training window and reject candidates that worsen any
  training-section median violation by more than 1 dB. This prevents a favourable
  average from hiding a conflicting section; the actual-DSP check still follows.
- Candidate prediction applies the biquad magnitude responses to the training
  spectrum. This linear prediction is not trusted to predict compression: rerun
  the **actual production DSP** once with the selected correction.
- Keep the baseline activity mask fixed. Both split medians must improve their
  distance from the target by at least 0.25 dB. Reject if coherent group tone
  regresses more than 1 dB; p95 window-maximum gain reduction increases more than
  1 dB; or median/p10 window crest falls more than 1.5 dB. Individual sections with
  at least three eligible windows also cannot regress more than 1 dB in primary
  or coherent-group tone. No retry or retuning follows a held-out rejection.

The spectral cache is capped at 2000 windows (about 64 MiB per measurement). This
limits long/high-rate material; use an appropriate soundcheck excerpt. It is an
offline allocation/I/O path and cannot run in an audio callback.

## Evidence and limits

`policy.json`, `before-settings.json`, `decisions.json` and `settings.json` retain
identity, intent, original/candidate/applied EQ, confidence evidence, proposal and
acceptance reason. Per-instrument baseline/candidate JSON includes window levels,
body ratios, coherent-group ratios, crest and mean/maximum compressor action.
`instrument-N-eligible.json` preserves exact window selection. `final/` contains
normal render/export meters and audio only for `tone-pass`.

An accepted correction is a measured improvement against this policy. It may still
mask bass/vocals, change picking texture or fail listener preference. This first
pass does not optimize the full-band balance. Window crest and reduction protect
coarse dynamics; they do not establish sample-exact pick-transient preservation.
The activity/secondary gates cannot reliably distinguish all bleed from quiet direct
playing. Known source assignments and an intentional soundcheck remain important.

**Narrow ringing detection and correction remain unimplemented.** A persistent FFT
peak can be a played fundamental or harmonic. This pass explicitly logs abstention
from notching rather than claiming a microphone defect or cutting musical notes.
Pitch changes and repeated decay evidence are the next distinct detector to build.
There are no automatic master notches, source normalization, LUFS targets, added
makeup compensation, time alignment or polarity changes.

## Validation and listening

`cargo test --locked --test tone` covers measured tone correction, explicit thin
intent, quiet playing, silence, insufficient evidence, bleed-only pauses, dominant
secondary paths, correlated sources, sparse musical decays, split-boundary exclusion, stereo input, held-out veto, dynamics guard
rejection, deterministic proposals, identity validation and evidence non-overwrite.
The biquad response prediction is checked against measured sine gain. These fast
synthetic regressions remain in the normal suite. Private music is opt-in through
the commands above; no media downloads or listening happen in tests.

The local experiment used a 24-second pilot before full-song rendering. The final
whole-song search selected **+1 dB at 300 Hz and −1 dB at 2400 Hz**, both Q=0.7.
It found a body deficit in 93.4% of eligible training windows. Actual held-out
primary body/presence improved from −8.49 to −6.87 dB; the coherent two-path sum
improved from −5.67 to −4.44 dB. P95 window-maximum compressor reduction fell from
1.64 to 1.37 dB; median crest changed from 13.95 to 13.88 dB.

**The tone target remains unmet.** The body-heavy ending constrains a static EQ
shared with the thinner main playing sections. Larger pilot corrections were vetoed
for the whole song. The final algorithm includes training-section constraints in
selection, then uses held-out sections only for acceptance. This song informed
algorithm development; these are within-song diagnostics, not an independent blind
evaluation or proof of generalization. No new listener preference is claimed.

Validation: 53 normal Rust tests and four Python tests passed, plus formatting,
Clippy and release build. Three historical private-media tests were intentionally
skipped. This task's pilot/full-song runs, unchanged-original hashes, exact excerpt
PCM slices and independent −0.01 dBFS sample peaks were checked explicitly.

New comparisons should contain **one current final mix → one corrected final mix**
for the same 12-second section. Each full export is independently finalized to
−0.01 dBFS sample peak. Excerpts retain that gain, with no loudness matching. Extra
candidates must be explained before offering another comparison.
