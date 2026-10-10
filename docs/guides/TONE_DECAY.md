# Guitar correction when the ending fades

Implemented offline, 2026-10-02. This revises the temporal eligibility used by
`tone-analyze` / `tone-pass` and the [source-rule coordinator](SOURCE_RULES.md).
It does not add changing EQ during playback or infer microphone placement.

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
in either split, the correction is rejected.

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

## Validation and limits

Synthetic regressions cover quiet fades, temporal classification, steady dark
playing, uniform fading, abrupt steps, repeated attacks, insufficient evidence,
split isolation and deliberately broken level/compression/crest guards on fades.
The existing bleed, correlated-mic, silence, stereo-link and transient tests pass.

The fixed FFT size means low sample rates can have too few windows in six seconds;
those spans abstain. Boundary placement can also miss a fade. Similar level/spectral
trajectories could occur in other musical gestures; no universal articulation
classifier is claimed. Broader recordings and listening are needed before promoting
these thresholds to general instrument rules. Current limitations do not justify
loosening the remaining guards or automatically increasing the EQ search budget.

## Historical evidence

The [dated study and original results](../archive/studies/TONE_DECAY.md) retain experiment-specific settings and validation history.
