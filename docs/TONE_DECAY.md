# Guitar correction when the ending fades

Implemented offline, 2026-10-02. This revises the temporal eligibility used by
`tone-analyze` / `tone-pass` and the [source-rule coordinator](SOURCE_RULES.md).
It does not add changing EQ during playback or infer microphone placement.

## Why the previous veto was too broad

The previous Complainiacs experiment proposed +5 dB at 220 Hz and −3 dB at
2400 Hz. Both split medians improved, and compression/crest checks passed.
The whole-song veto came from the 96–108 second section: 13 eligible windows,
about 2.4 seconds, became farther from the trial body/presence range. One static
curve was consequently reduced to +1 dB at 300 Hz / −1 dB at 2400 Hz.

That result established a policy conflict, not an audible failure. Inspection
shows declining signal level and presence energy fading faster than body energy.
A growing body/presence ratio during such a fade is weak evidence for a stable
bass-heavy source. A common target for ordinary playing and this fade was an
unjustified assumption. The system should identify this condition before selecting
EQ, while continuing to check what the correction does to the fade.

This song informed the detector's development. Its held-out sections are useful
within-song checks, not an independent blind evaluation of the new algorithm.

## Temporal rule

The baseline measurements now produce `temporal_sections` and a fixed
`fading_mask` in the tone proposal. The rule has no filename, song-position,
profile-target or requested-EQ special cases.

1. Divide each existing training/held-out section into spans of at most six
   seconds. Exclude FFT windows crossing either boundary. Each span uses only
   evidence from its own split; the raw activity threshold remains training-only.
2. Require eight consecutive eligible analysis windows spanning at least three
   seconds. Raw primary activity and raw secondary contribution must pass the
   existing activity/dominance limits. Missing/gated windows break continuity.
3. Divide those windows into four chronological groups and measure median primary
   output RMS and median body/presence ratio in each group.
4. Require at least 6 dB of level fall and 6 dB of body/presence rise from first
   to last group. Level must fall at least 0.5 dB at each step. Ratio must rise
   at least 0.5 dB at two of the three steps, with no reversal larger than 1 dB.
   No adjacent analysis window may rise by more than 6 dB in primary level.

The small ratio tolerance allows a plateau/fluctuation as the weaker band approaches
residual background. Six-second spans avoid judging a whole long decay after that
ratio becomes unstable. These numerical thresholds are initial engineering choices,
not manufacturer settings or calibrated probabilities. Reports preserve the span,
window count, level fall, ratio rise and classification; baseline window measurements
allow the decision to be reconstructed.

Only classified windows leave the steady-tone mask. A steady dark source, a uniform
fade without spectral change, an abrupt tone step, renewed attacks or insufficient
history do not earn this exemption in the regression cases. Quiet fading input is
also tested. The classification is “fading spectral balance”; it does not prove
that a particular note was played or establish a physical cause.

The usual grid, EQ limits, target range, section-regression limits and held-out
acceptance requirements remain in force. If classification leaves too little playing
in either split, the correction is rejected. In particular, the short ending pilot
correctly refuses approval because its held-out section contains no eligible
steady-tone evidence.

## Checks on the excluded fade

The fixed baseline fade mask gets actual-DSP checks even though those windows no
longer define the body target. Every flagged window must satisfy:

- Primary RMS rise at most 6 dB, plus 0.05 dB numerical tolerance. The limit matches
  the maximum body boost in the existing search grid.
- Added maximum compressor reduction within the existing limit, normally 1 dB.
- Crest loss within the existing limit, normally 1.5 dB.

The source coordinator also retains its independent dynamics mask, output budget
and section checks. Candidate processing cannot redefine its own baseline masks.
These checks do not establish that a louder tail sounds better or that all pick
transients are preserved. No samples are muted or shortened.

## Experiment and result

Local evidence: `artifacts/automix/guitar-decay-v1/`. Short 24-second playing and
ending pilots preceded the full-song analysis and render. The full-song algorithm
selected **+5 dB at 300 Hz and −3 dB at 2400 Hz**, both Q 0.7, from the original
baseline. It replaces the previous appended correction rather than stacking it.
Existing manufacturer-derived settings, trims, makeup, faders, pans, stereo links,
source timing and master processing are preserved.

| Check | Result |
|---|---:|
| Classified fade | 32 windows, 96.04–101.80 s window starts |
| Fade first/last temporal-group level fall | 8.47 dB |
| Fade first/last temporal-group body/presence rise | 10.84 dB |
| Held-out primary body/presence, baseline → new | −8.49 → −2.36 dB |
| Previous modest correction, same held-out mask | −6.87 dB |
| Held-out coherent two-path body/presence, new | −0.80 dB |
| Held-out p95 maximum compressor action, new | 1.29 dB |
| Largest primary output rise in flagged fade | 3.46 dB |
| Largest added compressor action in flagged fade | 0.038 dB |
| Largest crest loss in flagged fade | 1.07 dB |

The stronger correction passes these checks. The primary target remains slightly
unmet: −2.36 dB is below the trial −2 dB lower bound. The range was not widened to
make that result pass. Source-first advice still reports a substantial capture
limitation; a live amp-mic setup would request source adjustment and another capture.

Re-running the saved Wild & Co, Catbite, Phoenix and Rainfall pilots produced no
new fading classifications and identical accepted settings. That is limited
regression evidence, not a broad validation across instruments and playing styles.

The full new mix is in `final/processed.wav`. Both the previous and new full exports
were checked at −0.01 dBFS sample peak, with independent export gains and no loudness
matching. The new export measures −9.34 LUFS versus −9.23 LUFS previously; loudness
was measured, not targeted. No true-peak claim is made.

One listening pair is in `listen/`: `01-CURRENT-MIX-024s.wav` then
`02-NEW-GUITAR-MIX-024s.wav`. Each is the exact 12-second PCM slice beginning at
24 seconds of its full export. There are no additional candidate sets. The local
`ending-evidence.png` shows the raw spectrogram and before/after measured envelopes.
No playback or listener preference is claimed.

## Validation and limits

71 normal Rust tests and four Python tests pass, alongside Clippy with warnings
as errors, formatting and a release build. Three historical private-media tests
remain opt-in; the targeted local pilots/full render above were explicitly run.
New synthetic regressions cover quiet fades, temporal classification, steady dark
playing, uniform fading, abrupt steps, repeated attacks, insufficient evidence,
split isolation and deliberately broken level/compression/crest guards on fades.
The existing bleed, correlated-mic, silence, stereo-link and transient tests pass.

The fixed FFT size means low sample rates can have too few windows in six seconds;
those spans abstain. Boundary placement can also miss a fade. Similar level/spectral
trajectories could occur in other musical gestures; no universal articulation
classifier is claimed. Broader recordings and listening are needed before promoting
these thresholds to general instrument rules. Current limitations do not justify
loosening the remaining guards or automatically increasing the EQ search budget.

Original media hashes were checked; temporary pilot copies and the disposable pilot
render were removed after preserving reports and reproduction scripts. Original
recordings, previous renders and the new full mix/listening pair remain local.
