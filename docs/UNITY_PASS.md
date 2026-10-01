# Unity-source comparison without loudness matching

This revision follows the listening feedback on the first mixes. It removes the
old input-level targets, negative musical faders and fixed −6 dB master from this
workflow. Earlier renders and commands remain available for historical comparison.

## Reproduce

```sh
cargo build --locked --release
target/release/gigpies unity-pass \
  artifacts/automix/fx-pass/automatic-final/final/prepared.json \
  recordings/sessions/complainiacs-etc artifacts/automix/unity-pass-final
# Reuse the resulting frozen settings, without measuring them again:
target/release/gigpies render artifacts/automix/unity-pass-final/settings.json \
  recordings/sessions/complainiacs-etc artifacts/automix/unity-replay
```

Use new output directories. For a fresh installation, create settings with `preset`,
then optionally `fx-preset`, and supply those to `unity-pass`. Media remain local.

## Routing and processing

- Remove bass amp; keep DI. All remaining input trims, faders and fixed master gain
  are 0 dB. Keep original pan positions and stereo channels. Mono centre pan remains
  constant-power (−3 dB per side); stereo centre passes both sides unchanged.
- Raw is the unity source sum through that pan routing, without EQ, compression or FX.
- Processed channel HPFs: **90 Hz on every source except kick and bass DI**. Kick and
  DI have no channel HPF. Processed master HPF: **40 Hz**, after dry/wet summing.
  All HPFs are second-order Butterworth, 12 dB/octave. FX return HPFs are also 90 Hz.
- Retain the supplied channel bell EQ, compressor ratios, knees and attack/release.
  Clear the previous automatic master bell correction. Complete parameters are saved
  in `settings.json`; these retained EQ choices still need listening evaluation.
- Measure each source after its EQ in 100 ms windows. Reject windows below −65 dBFS
  or 18 dB below that source's highest window RMS. The 90th percentile active peak
  sets its compressor threshold. Nominal static reduction at that reference is
  snare 5 dB, bass 4, kick/vocal 3, toms 2, overheads 0.5, other roles 1 dB.
  Attack/release mean actual reduction can be substantially smaller; it is reported.
  A second measurement raises the threshold if maximum reduction exceeds the nominal
  amount by more than 1 dB. Thresholds are bounded to −60…0 dBFS.
- Makeup compensates only measured RMS loss through that compressor, capped at 6 dB.
  It does not normalize sources to each other or compensate HPF/EQ losses. An inactive
  source gets no compression or makeup. Stereo detection remains linked.
- If FX were enabled, rebuild the standard rack with corrected source indices.
  Keep distinct snare plate, tom chamber, vocal hall/plate, chorus, delay and exciter.
  Vocal hall/plate predelays remain **45/40 ms**. Measure return/source ratios and
  adjust returns toward their documented wet targets, bounded to ±12 dB.
- Set maximizer threshold 2 dB below the preliminary processed bus peak, with zero
  drive. Report actual final reduction because changing returns can change the peak.
  No additional legacy master limiter is used in unmatched mode.

This is whole-recording offline preparation followed by a frozen render. It does
**not** claim causal calibration of those same earlier samples. The original causal
`soundcheck` workflow remains separate and unchanged.

## Output levels and evidence

The internal float sums may exceed 0 dBFS without clipping. Preserve them as
`final/bypass-unity-float.wav` and `final/processed-unity-float.wav`. Play the PCM
files `final/bypass.wav` and `final/processed.wav`: both receive exactly one shared
export attenuation calculated from the larger peak, leaving 0.5 dB sample headroom.
No per-file loudness gain, target LUFS or matched copies are produced. Loudness is
measured for reporting only. Output protection is sample-peak, not true-peak.

For the first local revision, raw unity sum peak was +7.0468 dBFS, requiring shared
−7.5468 dB export attenuation. Raw/processed PCM peaks were −0.5000/−3.9801 dBFS;
actual maximum maximizer reduction was 1.9996 dB. The processed mix retains its
natural level difference. No individual source gain was reduced to achieve these peaks.

`measured-channel-decisions.json` and `measured-bus-decisions.json` record measured
reasons. `settings.json` saves all processing; `final/` includes channel/master
histories, clipping/full-scale counts, alignment, padding and export measurements.
Both outputs are 44.1 kHz stereo, 5,214,480 frames (118.242 seconds), including the
same four-second FX tail. Original starts and BWF offsets remain intact.

Rust writes 100 ms `raw-envelope.csv` / `processed-envelope.csv`: RMS, sample peak,
crest factor, left/right levels and correlation, including the final partial window.
`envelope-review.json` reports distributions and flags losses exceeding 3 dB in median
crest or RMS percentile range. This run changed median crest by +0.187 dB and RMS
p90–p10 range by +0.780 dB. Different active windows can affect these summaries.
They cannot determine subjective instrument balance or whether a mix sounds good.

`raw-spectrum.json` / `processed-spectrum.json` use the existing Rust FFT analyzer.
This revision records candidates without adding another automatic EQ correction.
Short comparisons should cut the same timestamps from the PCM files with **no gain
changes**. Listen for drum impact, guitar body, vocal/room balance and reverb clutter.
