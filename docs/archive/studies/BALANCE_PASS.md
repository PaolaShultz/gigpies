# Offline musical balance experiment

The [current workflow reassessment](COMPLAINIACS_WORKFLOW_REVIEW.md) treats the
numerical relationships below as historical engineering policies, not preferred
musical balance. No target or protection threshold was changed to admit a mix.

The new [manufacturer-reference experiment](PRESET_EXPERIMENT.md) separates published
parameters, DSP adaptations, processing evidence and static fader changes.

Implemented: synchronized source/group analysis, a bounded static fader optimizer,
and separate fader-only and processing comparisons. Offline preparation can inspect
the complete recording. It does not change or claim to replace causal soundcheck.
Technical validation and policy compliance do not establish listener preference.

## Commands

Use a new directory for every run. The baseline must already have DI-only bass,
unity input trims, explicit channel/master HPF settings (including bypass),
`output_mode: "unmatched"`, and a −0.01 dBFS ceiling. Incompatible settings are
rejected, not silently converted. Pan, BWF timing, stereo linking, compression,
EQ, FX and master settings are preserved by the fader pass.

```sh
cargo build --locked --release
# Measure without rendering or changing settings:
target/release/gigpies balance-analyze baseline-settings.json source-dir new-analysis-dir
# With an explicit reviewed musical policy, search and measure a fader proposal:
target/release/gigpies balance-pass baseline-settings.json source-dir new-pass-dir policy.json
# Without a policy, both commands observe and preserve faders; no taste targets.
```

`A-settings.json` is the frozen fader candidate. The `A/` directory contains the
full render, independent export gain, channel/master histories and bus/FX meters.
`optimization.json` records evaluations, proposed fit/held-out results and acceptance.
Rejected proposals produce unchanged faders; their proposed metrics remain available
for diagnosis. `status.json` separately records technical completion, whether all
measurable policy targets were met, and an unset listener-preference field.
`decisions.json` records old/new faders and evidence references.

After rendering A, inspect its evidence before editing a copy as `B-settings.json`.
Log each processing edit's old/new value, evidence and tradeoff. Keep all settings
editable and avoid adding compressor compensation a second time. Render B with
`render`, then remeasure with `balance-analyze` and the same policy. A processing
hypothesis can be rejected without changing A. Neither command adds master notches,
channel normalization, matched listening copies, time alignment or polarity flips.

## Measurements and confidence

Analysis runs the production channel strips, excitation, fader/pan routing, FX and
master processing at native rate. Each 20 ms window records:

- Per-channel input and pre/post-compressor RMS in dBFS, peak, instantaneous mean/max
  gain reduction in dB, stereo correlation, activity, confidence and event flags.
- Routed stereo covariance for broadband and approximate 40–120, 120–500, 500–1500
  and 1500–5000 Hz bands. Band filters subtract adjacent second-order Butterworth
  HPF outputs; they overlap and are not rectangular FFT bands. Group energies include
  covariance cross terms, so correlated microphone cancellation is retained.
- Separate close drums, overheads, drum room, bass, guitars, direct vocal, vocal room
  and summed generated-return energies in `groups.csv`. Excitation is included in
  each source; its separate residual meter and per-return totals are in the renderer's
  `measurements.json`. The analysis covers the source timeline; render meters also
  cover the configured FX tail.
- Master mean/max reduction and pre-export RMS in `master.csv`.

Activity requires post-EQ/pre-compressor RMS above −75 dBFS and within 30 dB of
that source's p95 window. Confidence is an explicit heuristic score, **not a calibrated
probability**: 0.85 for activity separated from the source's p10 by 6 dB, or sustained
activity with p10 at least 3 dB above the absolute floor; otherwise 0.55. Rejected
activity has zero confidence; ambiguous tom bleed is capped at 0.35. The default
search requires 0.65. Very quiet playing below the floor remains unresolved; the
floor and relative margin are editable. These rules do not identify words or notes.

Drum events require a 5 dB rise over the preceding 60 ms and 120 ms between triggers.
The event anchor is the strongest role-band window in the following 60 ms, with
20 ms resolution. Kick uses the low band; snare/toms use the body band. Reported
attack/body/decay windows are 0–20, 20–80 and 80–240 ms from that anchor. Thus the
attack measurement describes the early peak region, not a sample-exact acoustic
onset. Phrase-activity starts use 0–100, 100–300 and 300–600 ms envelopes.
Adjacent events can overlap the decay; inspect `events.json` and `windows.csv`.

Tom bleed rejection explicitly compares coincident kick/snare events (±20 ms).
With at least three reference events, it estimates a median tom/reference band-energy
ratio. A coincident tom candidate within 6 dB of that reference is marked ambiguous.
Only accepted tom events and their next 240 ms qualify as active tom playing.
This can reject genuine unison hits and miss delayed/independent bleed. It neither
separates sources nor estimates a clean tom waveform. `microphone-relationships.json`
reports simultaneous correlation, event ratios and actual summed power versus
incoherent power for related drum and vocal microphones. Negative sum differences
flag possible cancellation, not a proven phase defect. No automatic correction follows.

Active-source summaries describe each source; **they are not cross-instrument fader
targets**. Ratios used for optimization always use identical windows on both sides.
In particular, an intermittent tom's whole-song RMS is never compared to a continuous
guitar to choose its fader.

## Explicit historical balance experiment

`Policy::for_session` now supplies no relationships. `Policy::trial_for_session`
retains the historical ranges below for deliberate experiments. New default runs
never select these targets from role labels. See [source preservation](../../guides/SOURCE_PRESERVATION.md).

All the ranges below are engineering hypotheses for audition, not universal or
research-validated correct-mix values. The masking proxy is the dB ratio of simultaneous
role-band energy. Spectral overlap is normal and is not itself penalized.

| Relationship | Eligible windows and band | Acceptable ratio, dB |
|---|---|---:|
| Each close drum / overheads | That drum's early hit region; kick low, others body | −3 to +12 |
| Each close drum / bass and guitars | Same hit region and band | −6 to +12 |
| Drum room / close kit | Kick hit windows, broadband | −24 to −9 |
| Bass / guitars | Active bass and competing guitar, low band | −3 to +15 |
| Each guitar / drums | Active playing with kit activity, body band | −15 to 0 |
| Vocal / guitars | Vocal activity with guitar activity, presence band | 0 to +12 |
| Vocal room / direct | Direct vocal activity, broadband | −24 to −12 |

Relationships, channel indices, activity source, event requirement, band, stage and
ranges are editable in `policy.json`. Median, p10/p90, eligible counts and confidence
are reported; fewer than eight windows yields insufficient evidence, not success.
The room/kit default is conditioned on kick hits; snare/tom event and microphone
reports supplement it rather than claiming complete room perceptual coverage.

The deterministic coordinate search uses 0.5 dB moves bounded to ±6 dB from the
supplied faders and a default budget of 512 scored candidates per stage. The budget is editable;
the earlier local experiment used 1,024 per stage. Stage one moves kit
microphones and vocal room individually. Stage two moves the complete kit, bass,
rhythm guitar, lead guitar and direct vocal groups. These are musical fader moves;
calibration groups and their unity input trims do not change. Each stereo source
always gets one linked fader. A quadratic change penalty discourages unnecessary
moves. Relative ratios cannot improve through a common gain change; the penalty
makes such a move worse. Both cuts and boosts are available.

Even-numbered 12-second sections choose the settings. Odd-numbered sections are
held out of fader selection. Source activity statistics use the whole recording;
this is a within-recording check, not independent-session validation. Acceptance
requires improved fit cost, held-out evidence for each improved target, no increase in aggregate
held-out violation, and no measured relationship worsening by more than 1 dB in
aggregate or in any individual section with enough evidence. The search does not
retry based on held-out results. Static conflicts can remain at the bounds. No
section automation or continuous gain chasing is implemented.

The retained covariance cache is capped at 512 MB; oversized preparation sessions
are rejected. This offline implementation allocates and performs file I/O. It is
not suitable for an audio callback.

## Listening evidence

Each finished PCM file is independently finalized from its own sample peak to
−0.01 dBFS by the existing exporter. No raw-reference gain or LUFS target is used.
Float sums can exceed full scale without clipping. This is **sample-peak** protection;
no true-peak claim is made. Keep the baseline and previous renders intact.

```sh
python3 scripts/balance_excerpts.py OLD.wav A.wav B.wav new-excerpts-dir 12 36 72
```

This optional standard-library script checks matching formats/frame counts and the
finished sample peaks, then copies exact 12-second PCM slices. It writes a manifest
with full paths, hashes, frame positions and explicit **OLD → A → B** order at each
start. Excerpts retain their full-song gain; they are not individually normalized.
The script verifies the copied PCM bytes and never plays audio.

For the local experiment, settings, analysis, processing decisions and full-song
A/B renders are under `artifacts/automix/musical-balance-v2/`. Private state and media
stay ignored. The local `evidence/` report records measured outcomes and unresolved
conflicts; it is not a listening verdict. Listen for kit impact versus cymbal/room
level, tom fills, guitar weight during singing, vocal-room prominence and FX tails.

## Research applicability

Reviewed 2026-10-01:

- [Moffat and Sandler, AES 20355](https://aes.org/publications/elibrary-page/?id=20355):
  the accessible abstract identifies interference between live drum microphones as
  a problem for treating each track as an independent source. Its proposed equal
  perceptual loudness objective is explicitly outside this experiment's requirements.
  We inspected the abstract, not a subscriber-only full paper; no source-separation
  implementation or numerical target is attributed to it.
- [Ronan et al., arXiv:1803.09960](https://arxiv.org/abs/1803.09960): the current version
  is withdrawn, with an intellectual-property ownership note. Its abstract discusses
  a psychoacoustically inspired masking metric and subgrouping. Our ratio proxy does
  not implement that model, reproduce its results or inherit its validation.
- [De Man, PhD thesis](https://www.brechtdeman.com/publications/pdf/PhD-thesis.pdf):
  the retrievable indexed perceptual-evaluation section compares automatic and human
  mixes using listening tests. Full-PDF retrieval failed in this session; applicability
  is limited to that section. It supports keeping perceptual evaluation separate from
  technical measurements, not the numerical ranges above. The normalized-input
  comparison described there is not used here.

## Validation

`cargo test --locked --test balance` covers known buried-hit and excessive-room
faults, vocal/guitar competition, no-change, silence, low confidence, quiet playing,
coincident bleed, stereo/phase relationships, global-gain invariance, bounded and
repeatable search, held-out veto and evidence non-overwrite. Synthetic audio checks
activity and routing independently of the optimizer's objective. The full normal
production suite also covers BWF routing, linked DSP, FX, persistence and export.

Private full-song renders and excerpt verification are explicit opt-in preparation,
not normal CI tests. Historical source-variation and older audition tests remain
ignored unless their own protected behavior changes. No playback, host-audio change,
hardware verification or listener preference follows from passing these tests.

## SOURCE and paired ensemble review

The [Complainiacs reassessment](COMPLAINIACS_REASSESSMENT.md) adds
`balance-source-analyze INITIAL_SOURCE.json SOURCES NEW_DIRECTORY` and the
`scripts/ensemble_review.py` observer. SOURCE bypasses all DSP while retaining the
supplied initial pan/faders. Processed measurement and optimizer behavior remain.
Group CSVs add coherent `drums` and `vocal_sum` groups and an
`incoherent_broadband_dbfs` column; existing fields retain their values.

The paired report fixes activity to explicit SOURCE evidence with training-only
thresholds, checks identical raw windows/routing, and retains section and raw
quiet/strong strata. Export gain is separate from pre-master group energy. It never
turns a relationship into a preferred target or a listener verdict. Existing
optimizer failures remain failures even when a separately documented artistic
selection follows revised listener direction. See the reassessment for that
choice, its remaining room-ratio failures and the bounded rejected EQ probes.

Group CSVs also expose `snare`, `kick`, `toms`, `drum_ambience`,
`drums_without_snare`, `ensemble` and `ensemble_without_snare`. Both ensemble groups
include generated FX. Coherent sums retain signed microphone interaction; the
power of the sum need not equal the sum of separate powers. These are mixtures,
not separated snare or bleed estimates. All groups are before master/export.

`scripts/snare_context.py SPEC.json training|held_out NEW_REPORT.json` summarizes
fixed baseline events in these groups. SPEC supplies `rate`, `source_analysis`,
`diagnosis` and `variants` with `id`/`analysis`. It requires explicit SOURCE bypass
evidence, unchanged raw inputs and complete event windows; training determines
quiet/strong and vocal-activity thresholds. Reports retain section, rapid/compound
contexts, signed interaction and insufficient counts without selecting settings.
