# Snare bleed: additional evidence and failure handling

Implemented and offline-validated, 2026-10-02. The listener confirms snare bleed.
That observation is recorded as evidence; the uncertainty is **which components
and events can be attenuated without damaging intended snare playing**.

**No gate, expander or reference subtraction is enabled.** Three static processing
candidates failed the predeclared protection/confidence checks. Their settings and
actual production-strip measurements are retained, but none was applied or added
to a listening queue. The [separate +2 dB rhythmic-emphasis candidate](../../guides/DRUMS.md)
was revalidated after the detector fix and remains the only prepared comparison.
It does not claim to clean the bleed.

## A real event-detector failure

The initial drum detector accepted a small rise and then imposed an 80 ms lockout.
A stronger hit arriving during that interval could be misreported as the small
rise's body or tail. At 38.75 s, for example, raw snare RMS in the 10 ms frame is
about −30.3 dBFS; at 38.79 s it reaches −13.7 dBFS. The old anchor stayed at 38.75 s.
Another example occurs around 41.02–41.06 s. This invalidates treating those early
anchors as isolated quiet snare attacks or evidence of excessive sustain.

The corrected offline rule:

- Collects raw rise candidates in a bounded 80 ms neighborhood and chooses the
  strongest role-band peak. It cannot merge across a training/held-out boundary.
- Retains **every competing rise time**. Candidates separated by at least 30 ms
  mark the event as ambiguous, rather than silently erasing a possible flam or
  ghost note. This is an analysis-resolution limit, not a musical classification.
- Excludes ambiguous/overlapping events from isolated recovery and compressor-relief
  votes. An uninterpretable attack/body contrast cannot veto or justify processing.
- Keeps independent 10 ms crest and level guards on the actual signal, including
  compound events. This replaces the invalid comparison with another protection;
  it does not remove transient protection or loosen the numerical budgets.
- Bounds the final anchor so the complete requested event window exists. A regression
  also covers a rise near the incomplete trailing region.

`drum-events` can re-run diagnosis from saved synchronized measurements without
rereading media, changing DSP or overwriting the earlier evidence. The corrected
full-source analysis has 431 snare event candidates, including 72 compound events.
These are candidates, not a verified count of played snare hits.

The original [drum report](../../guides/DRUMS.md) preserves the first investigation. Its quiet-hit
and sustain interpretations are qualified by this correction. The source peaks,
actual reduction maxima, channel settings, fader changes and export measurements
are unaffected. No original recording or rendered audio was changed by this fix.

## Additional ways of examining the source

The new `automix/bleed.rs` measures known kick, snare and overhead paths, with explicit
index/filename checks. It streams from the common BWF origin at native rate and
preserves earlier filter/compressor history before a pilot. The new views are:

1. **Conditional frequency and decay measurements.** Compare 0–30, 30–80, 80–160
   and 160–240 ms regions around each corrected event. Measure five bands at
   40–120, 120–400, 400–1600, 1600–4800 and 4800–12000 Hz, separately for raw
   kick/snare/overheads, snare EQ output and snare compression with makeup.
   Bands subtract second-order HPFs and overlap; they are not rectangular spectra.
2. **Relative snare shape.** Strong, unambiguous training events define a five-band
   power-share reference. Compare quieter events without changing their audio gain.
   A quiet matching shape is protected as *snare-like*, not declared to be a snare
   hit. The reference level and shape are frozen before whole-song checks.
3. **Lagged waveform similarity.** Compare snare with kick and overheads in the
   approximate 40–400 Hz waveform over −20 to +80 ms. Search ±15 ms, preserving
   sign and channel pair. Stereo power is retained; opposite L/R polarity cannot
   disappear through a mono sum. A block-averaged representation near 6 kHz is
   used only for this analysis; the audio is not resampled or time-shifted.
4. **High-band envelope correlation.** Compare snare/overhead 4.8–12 kHz energy
   trajectories around events. Require enough variation and samples; a steady
   floor yields no correlation claim. Shared energy does not establish which
   microphone received spill from which source.
5. **Frozen waveform prediction.** Fit one delayed, signed low-band coefficient
   using only weak kick-coincident training events, then test it on strong and weak
   events in other sections. This tests whether a component is reproducibly
   predictable, rather than merely sharing a frequency band. It does not subtract
   that component from any source or render.

These are our diagnostic methods and budgets, not manufacturer-published settings,
source-separation guarantees, calibrated probabilities or microphone-placement
inferences. The produced comparison mix supplies no targets or features.

## What the measurements show

### Processing exposes the residual

A conservative residual mask protects every detected rise, including weak precursors,
from 20 ms before to 120 ms after it. Remaining frames must be at least 18 dB below
the training snare-reference level. This leaves **4.42 seconds** in the 24–48 s pilot.
It is an inter-event residual mask, not a claim that every retained sample is bleed:
quiet playing and natural snare decay can still be present.

Median raw snare broadband level there is −36.81 dBFS. The existing snare EQ raises
4.8–12 kHz energy by **5.63 dB**; EQ plus existing makeup raises it by **9.13 dB**.
Compression is doing very little in those gaps, so the +3.5 dB makeup is largely
unopposed. This provides a concrete signal-path explanation for exposing residual
sound. It does not establish that all high-frequency snare content is unwanted.

An earlier 5.37-second gap estimate protected only the selected cluster peaks.
Including the retained weak precursors reduces coverage to 4.42 seconds. Those
precursors must not become automatic attenuation opportunities merely because a
stronger hit replaced their original anchor.

### Static EQ trades residual reduction for snare loss

The budget was recorded before these trials: unchanged processing plus exactly
three variants, with all faders and makeup fixed. Only the snare changes:

- High relief: existing 5 kHz shelf **+4.5 → +2.5 dB**.
- Body relief: existing 132 Hz/Q 1.2 bell **−0.5 → −2.5 dB**.
- Combined: both changes above.

| Candidate | Median residual broadband reduction | Residual high-band reduction | Worst protected body loss, training / held-out |
|---|---:|---:|---:|
| High relief | 0.22 dB | 1.24 dB | 0.014 / 0.009 dB |
| Body relief | 0.62 dB | ~0 dB | 1.37 / 1.42 dB |
| Combined | 0.99 dB | 1.24 dB | 1.38 / 1.43 dB |

The predeclared body-loss budget is 0.75 dB. The body/combined cuts fail it; these
are genuine protection failures, not a reason to relax the limit. The high shelf
preserves body but loses up to **1.62 dB** of protected high-band attack against a
1.5 dB budget. Its worst training event is at 32.17 s; body relief's is at 29.93 s.
Every result records the specific failure codes and worst-event timestamps.

All candidates also lack enough confidently separated spill events. None qualifies
on training data, so there is **no chosen candidate and no held-out fallback**.
The held-out sensitivity results remain diagnostic evidence, not a route for
selecting another candidate. Missing compatible EQ bands or proposals outside
session limits produce explicit abstentions rather than a partial DSP run.

### The broad spectral classifier cannot settle the event labels

The strongest unambiguous training events give a reference level near −13.99 dBFS.
Many weaker kick-coincident events have nearly the same relative five-band shape.
The full-song frozen classifier consequently protects 359 events as snare-like and
72 as compound; it finds **zero confidently separated spill-only event candidates**.

That is a limitation of this representation, **not evidence that bleed is absent**.
A regression explicitly supplies confirmed bleed with a snare-like shape and verifies
that the system keeps the ambiguity, reports insufficient evidence and refuses a
correction. It must not rename those events as bleed simply to make a candidate pass.

High-band snare/overhead envelope correlation is often strong (pilot median about
0.85 for snare-like events), but overheads contain genuine snare too. Correlation
alone cannot assign the shared signal to spill or justify removing it.

### Waveform prediction supplies additional, limited evidence

Two low-band predictor hypotheses were fitted to **20 weak kick-coincident training
windows at 24–36 s**. “Weak” means at least 8 dB below the fixed reference, within
40 ms of a kick event, with no ambiguous onset. These are hypothesis windows,
not independently labelled bleed-only examples.

| Frozen reference predictor | Coefficient | Fitted lag | Held-out weak-event median energy explained | Held-out strong-event median energy explained |
|---|---:|---:|---:|---:|
| Kick | −0.14159 | +1.75 ms | 21.86% | 0.52% |
| Overheads | −0.13647 | −5.87 ms | 40.40% | −2.21% |

The full evaluation contains **42 held-out weak events and 124 held-out strong
events**. The same coefficients, delays and channel selections were reused without
refitting. The original 36–48 s pilot had only three weak held-out examples; it was
insufficient to claim generalization by itself. Whole-song checks expanded diagnostic
coverage without choosing new parameters or retrying a processing candidate.

“Explained energy” is `1 − prediction_error_energy / original_energy` in this low-band
representation. It is not a measured percentage of all bleed removed from the mix.
A negative value means the hypothetical subtraction **increases** residual energy.
The overhead model does this on strong snare events; its held-out p10 is −11.36%.
Its negative fitted delay would also require future reference samples if implemented
literally. Neither delay nor sign was applied to audio; no polarity correction,
physical propagation time or live cancellation capability is inferred.

The repeatable weak/strong difference is useful evidence for a shared component.
It is still insufficient to certify safe removal: references can contain genuine
snare, and weak ghost notes can be correlated with the rest of the kit. The current
quiet-note protection remains valid. The models are retained as analysis candidates;
no reference cancellation processor was added to the renderer.

## Failure handling now distinguishes three cases

| Case | Evidence | System response |
|---|---|---|
| Implementation defect | Weak precursor locks out a later stronger hit | Fix event segmentation; retain competing rises; add regression and revalidate the unchanged rhythmic candidate |
| Valid protection failure | Static cut exceeds body/high-attack budget at identified events | Reject; record event and measured loss; keep the limit |
| Insufficient representation or labels | Known bleed can resemble quiet snare; references can contain wanted snare | Preserve listener evidence, protect ambiguous events, keep diagnostic models, abstain from automatic removal |

An accepted numerical fit is not automatically a processing permission. The report
keeps listener-confirmed bleed, instrument applicability, proposed changes, confidence,
abstention reasons, actual measured tradeoffs and null listener preference separate.
Silence, quiet snare-like hits and compound events do not become noise by default.

A more selective temporal or reference-based processor remains a possible next
experiment, but these results do not justify silently replacing the quiet-hit guard
with a permissive classifier. We have documented why the tested static options fail;
we have not demonstrated that gating or subtraction preserves the ambiguous playing.

For a future controlled soundcheck, first verify the recording tap and which controls
actually affect it. Capture isolated kick, isolated quiet/strong snare, cymbals alone,
rapid snare repeats and intentional unisons at unchanged gains. That would test the
reference transfer and protect labelled ghost notes. If a control, damping or capture
change is relevant to that path, compare one change and **repeat the measurements**.
Do not prescribe a gate on a console when the recording is tapped before it, or infer
mic position, drum tuning or defective hardware from these correlations.

## Commands and evidence

```sh
CARGO_INCREMENTAL=0 cargo build --locked --release
# Recompute event diagnosis from saved measurements; no source/DSP rerun.
target/release/gigpies drum-events MEASUREMENT.json NEW-DIAGNOSIS.json
# Fixed static budget, production strip measurement and explicit abstentions.
target/release/gigpies snare-bleed BASELINE.json SOURCES NEW-PILOT BLEED-POLICY.json 24 48
# Frozen training reference and saved predictor parameters; no new search.
target/release/gigpies snare-bleed-verify SOURCES NEW-PILOT NEW-FULL 0 120
python3 scripts/snare_bleed_evidence.py NEW-PILOT
```

`snare-bleed` writes `prediction-probes.json` with the pilot. The separate
`snare-predict SOURCES PILOT NEW-PREDICTIONS.json 24 48` command can add the same
analysis to an older saved pilot. To include those models in a full frozen review,
place the saved file at `PILOT/prediction-probes.json`. Outputs must be new; neither
command renders or plays audio.

The policy nests the existing drum identity/intent policy under `drums`, plus:

```json
{
  "overhead": 2,
  "overhead_file": "03_Overheads.wav",
  "listener_confirmed_snare_bleed": true
}
```

The fragment supplements the nested `drums` object; it is not a standalone complete
policy. The actual local policy is in `artifacts/automix/snare-bleed-v1/policy.json`.
This analyzer requires at least 32 kHz so all five bands remain below Nyquist.
Its streaming filters operate at source rate; the correlation representation and
feature cache are bounded to the ten-minute offline limit. It is not an audio callback.

Local evidence is under `artifacts/automix/snare-bleed-v1/`:

- `intent.json`, `static-plan.json`: listener evidence, split and fixed budgets.
- `event-probes.json`, `revised-events.json`: initial independent investigation and
  corrected event anchors. The exploratory Python filters differ from the Rust
  production-linked analyzer; their numbers are not silently interchanged.
- `static-pilot/`: source features, frozen shape reference, all three settings,
  production-strip measurements, detailed failures and unchanged applied settings.
- `residual-summary.json`: fixed mask including every retained precursor and
  candidate tradeoffs. `scripts/snare_bleed_evidence.py` reproduces it.
- `prediction-probes.json`: training coefficients and the initial held-out results.
- `full-review-v2/`: full-source frozen diagnosis and frozen predictor evaluation.
  No full-song candidate selection was run.
- `rhythmic-revalidation/`: the unchanged +2 dB kick/snare candidate passes revised
  event/crest guards over the full source timeline. Export metrics remain those in
  [the drum report](../../guides/DRUMS.md).

No new bleed-corrected audio is claimed. The already verified **24–36 s current
bass-corrected baseline → +2 dB rhythmic mix** pair remains in
`artifacts/automix/drums-v1/listen/`; it retains independent full-export gain and
has not been played. Rejected EQ settings and diagnostic predictors are not queued.
The guitar correction, bass correction, DI-only routing, timing, pan, stereo
relationships, unity trims, required HPFs and independent makeup/faders remain intact.

## Validation

The normal suite covers native production measurement, history continuity, stereo
polarity, delay/sign recovery, silence, injected kick spill, quiet/compound-event
protection, fixed training templates, frozen predictor validation, actual failure
locations, excessive/missing-band abstention and refusal to overwrite. A new
regression demonstrates the important identifiability limit: confirmed spill with
snare-like features still cannot be removed automatically on those features alone.

The full normal suite passes **94 Rust tests and eight Python tests**, along with
formatting, Clippy with warnings as errors and the release build. Three historical
private-media/exhaustive tests remain intentionally ignored. Current pilots,
frozen full-source checks and unchanged-render verification were run explicitly.
All media stays local; no playback, hardware change, commit or push occurred.

## Temporal follow-up: frozen representation audit

Implemented and offline-validated, 2026-10-02. Evidence lives in
`artifacts/automix/snare-temporal-v1/`. The listener's bleed observation remains
confirmed. **No new correction or listening export was selected.** The remaining
uncertainty concerns safe removal, not whether the listener heard bleed.

### Missing evidence and bounded experiment

The saved hypotheses lack independently labelled spill-only intervals, labelled
quiet snare/sustain, and isolated snare measurements of contamination in the kick
and overhead references. A repeatable prediction cannot supply those missing labels.
The recording tap and which physical controls affect it also remain unverified.

Before evaluation, `plan.json` fixed two diagnostic representations:

- Five raw snare attack-band shares, using the existing 0–30 ms features.
- All five raw bands in four phases through 240 ms, each relative to the same
  attack energy. This preserves relative decay instead of independently scaling
  each phase. It changes no audio gain.

Only 24–36 s can train. The existing pilot's frozen reference level defines strong
examples (within 3 dB); eight examples are required. Compounds and split-crossing
support are excluded. Neighboring retained rises within ±30 ms exclude an attack
reference; within ±240 ms exclude a temporal reference. This symmetric support
requirement conservatively excludes both previous sustain and subsequent attacks.
It is an experimental eligibility rule, not proof that sound outside 240 ms has
ended. Missed onsets can still defeat it.

`scripts/snare_temporal_evidence.py` separates fitting from evaluation. It saves the
coordinate-wise median template and a leave-one-out training p95 distance envelope.
Distance is median absolute band difference in dB. The envelope describes training
variation; **it is not a source classifier or an audio-loss limit**. The script
never writes settings. All events retain unresolved/protected source identity.
Odd 12-second sections evaluate frozen choices; other even sections provide
additional diagnostics. This song has informed development, so these are within-song
checks rather than blind validation on another recording. No held-out retry occurred.

### Outcome and specific events

| Check | Result |
|---|---:|
| Strong attack training examples | 28 |
| Attack training distance envelope | 0.815 dB |
| Strong training examples with isolated temporal support | 0 |
| Full-song events / events with temporal support | 431 / 13 |
| Held-out quiet kick-coincident hypotheses, including compounds | 50 |
| Those hypotheses inside the attack envelope | 18 |

The temporal model explicitly abstains for insufficient training support. The 13
full-song supported events do not become additional training examples. The attack
model still overlaps quiet kick-coincident events; neither an inside nor outside
result identifies spill. No temporal processor was proposed, so candidate production
DSP validation is recorded as null, not passed.

The previous static vetoes were inspected without repeating their sweep:

- **29.93 s**, worst body-loss example: attack distance 0.917 dB, outside the
  descriptive envelope. Neighbor rises at 29.82/29.84 and 30.10 s invalidate
  isolated-decay interpretation. They do not invalidate protecting the measured
  0–80 ms body or identify that body as expendable spill. The original loss veto stays.
- **32.17 s**, worst high-attack-loss example: distance 0.718 dB, inside the envelope;
  neighbors at 31.99 and 32.35 s prevent an isolated temporal reference. The high-band
  loss veto stays. Being predictable or quiet does not authorize removing its attack.
- **38.79 and 41.06 s** remain compound/protected, even though attack distances
  (0.227 and 0.369 dB) resemble the strong template.
- **91.16 s**, the detected snare event near the export-controlling drum peak,
  has isolated support and a 0.447 dB attack distance. It is held out and cannot
  rescue the missing training template.

A synthetic identifiability regression constructs identical observed reference and
snare signals with two different underlying decompositions: all spill versus spill
plus wanted quiet unison snare. Perfect prediction cancels wanted playing in the
second case. This demonstrates a failure mode; it establishes no real-recording
removal success and validates no cancellation DSP.

### Recovery defect, separate from protection failures

The previous detector preserved the next cluster's weak precursor but still measured
`next_seconds` to its strongest peak. That could admit contaminated recovery/tail
votes for the preceding event. It now measures to the **earliest retained rise of
the next cluster**. Event anchors and competing rises remain unchanged.

A synthetic regression failed before the fix: a hit at 0.40 s, precursor at 0.60 s
and next peak at 0.66 s were incorrectly treated as 260 ms of isolation. The corrected
200 ms interval withholds recovery and tail votes. On the full recording this removes
six kick and eight snare recovery votes. For example, snare at 38.55 s now stops at
the 38.75 s precursor (200 ms), rather than the 38.79 s peak (240 ms). At 42.61 s it
stops at 42.81 rather than 42.87 s. No protection threshold was loosened.

The frozen rhythmic candidate was rerun through actual production strips, routing
and master HPF across the full source. It still passes, with zero added compression
or crest loss and exactly +2 dB drum event contribution before finalization. The mix
peak rise remains 1.681762 dB. Its existing finalized exports therefore retain the
previous +0.318 dB absolute drum change and −1.682 dB change to unchanged instruments.
No new render was needed for this analysis-only fix. Original exports and exact
listening excerpts remain hash-verified and unplayed.

### Gap, ending and bass context

The audit also compares the existing 120 ms residual exclusion with a fixed 240 ms
exclusion, retaining every precursor and the 20 ms pre-rise protection. Both require
raw snare below frozen reference minus 18 dB. These are coverage measurements,
**not gate settings or spill-only labels**.

Full-song coverage falls from 44.162 to 32.422 seconds. In 24–36 s it falls from
1.43 to 0.22 s; in 36–48 s, from 2.99 to 1.13 s. Fully 18.242 s of the remaining
mask starts at 96 s, encompassing the ending and fade. This shows why apparently
large gap coverage cannot establish useful bleed-only training material. Raw snare
below −90 dBFS totals 5.672 s, mostly at the beginning and end; that threshold is
not a silence label for source separation.

Raw kick above −45 dBFS and the existing processed bass contribution above −65 dBFS
overlap for 80.29 s. Dense sections thus need deliberate kick/bass/unison checks;
coactivity alone supports neither bass changes nor cancellation. The original
bass DI correction, amp exclusion, guitar corrections, unity trims, pan, timing,
stereo relationships, channel HPFs, 40 Hz master HPF and independent makeup/faders
are retained. Compressor maxima and coactivity do not establish artistic balance.

### What would resolve the abstention

First identify the recording tap: pre/post console EQ, dynamics and fader; any DAW
processing; and whether the available kick/overhead tracks use the same timeline.
Keep a gain/routing record. A control downstream of that tap cannot improve this
recording. For the historical files, seek synchronized performance annotations or
an isolated passage whose source activity can be independently verified; a new
soundcheck cannot retroactively prove their event labels.

For a repeatable capture on the relevant path, retain synchronized unprocessed
snare, kick, stereo overheads and verified bass DI, with amp still excluded:

1. Record kit/bass silence, isolated kick at several strengths, then cymbals alone
   while the snare is deliberately unplayed. Include ringing tails beyond 240 ms.
   These supply known spill-only intervals and a floor measurement.
2. Record labelled quiet, normal and strong snare, including long natural decays,
   flams, rapid repeats and recovery pauses. Measure wanted snare in each reference
   track as well as in the snare channel; this tests reference contamination.
3. Record intentional kick/snare unisons, bass-only notes, kick/bass overlaps,
   then dense kit/band phrases and an ending. Preserve native timing and stereo.
4. Reserve whole repeat takes before fitting (at least eight examples of each
   relevant isolated class in training and eight in held-out takes as an initial
   evidence budget, not a statistical guarantee). Freeze the transfer model and
   candidate budget before opening those held-out takes. Check drift and repeatability
   at the same gains. If trying damping, placement or instrument controls, first
   verify they affect this tap, change one thing, and repeat the capture.

A future candidate needs labelled spill reduction **and** protected quiet/strong
attack, body, sustain, repeat and unison checks. The existing 0.75 dB body/protected
loss and 1.5 dB high-attack limits remain. Actual production-DSP validation at fixed
faders must precede combined rhythmic evaluation, full rendering and independent
sample-peak finalization. An unstable reference transfer or missing labels requires
abstention; a valid wanted-signal loss requires rejection. Listener preference remains
an independent, currently unknown result.

### Reproduction and validation

```sh
python3 scripts/snare_temporal_evidence.py fit \
  artifacts/automix/snare-bleed-v1/full-review-v2 NEW-MODEL.json
python3 scripts/snare_temporal_evidence.py evaluate \
  artifacts/automix/snare-bleed-v1/full-review-v2 NEW-MODEL.json NEW-AUDIT.json \
  --drum-measurement artifacts/automix/snare-bleed-v1/rhythmic-revalidation/baseline.json
CARGO_INCREMENTAL=0 cargo build --locked --release
target/release/gigpies drum-verify \
  artifacts/automix/snare-bleed-v1/rhythmic-revalidation/before-settings.json \
  artifacts/automix/snare-bleed-v1/rhythmic-revalidation/candidate-settings.json \
  ../waves/recordings/sessions/complainiacs-etc NEW-RHYTHMIC-REVIEW \
  artifacts/automix/snare-bleed-v1/rhythmic-revalidation/policy.json 0 120
CARGO_INCREMENTAL=0 cargo test --locked --all-targets
python3 -m unittest discover -s scripts -p 'test_*.py'
```

Outputs refuse overwrite and include input hashes. `frozen-model.json`,
`full-audit-context.json`, `recovery-fix-impact.json` and `rhythmic-revalidation/`
retain the experiment's useful evidence. Exact source/settings/export preservation
is recorded in `final-verification.json`; redundant new measurement files identical
to retained previous evidence are represented by `retained-measurements.json`.

Validation: **95 Rust and 16 Python tests pass**; formatting, Clippy with warnings
as errors and release build pass, with `CARGO_INCREMENTAL=0`. Three historical
private-media/exhaustive tests remain intentionally ignored. This task explicitly
ran the frozen temporal audit and full-song production-DSP rhythmic revalidation.
No playback, commit, push or hardware action occurred.

## Joint kick/overhead reference experiment

The next bounded experiment is owned by `automix/bleed_reference.rs`. It evaluates
low-band reference predictability while keeping source labels unresolved. It adds
no audio processor and cannot save or apply mix settings.

### Frozen method and failure handling

`snare-reference-fit` accepts an explicit training span; this experiment freezes
24–36 s. The existing kick and overhead predictors supply their saved delays and
channel choices. There is no new delay/channel search. Exactly three hypotheses
are audited: the two existing scalar models and one joint two-reference model.
The joint model fits unambiguous weak kick-coincident hypotheses (at least 8 dB
below the frozen snare reference, within 40 ms of kick), which remain unlabelled.

The joint fit standardizes reference energy in its normal equations, adds a fixed
0.001 diagonal penalty and solves for two coefficients. A normalized condition
number above 100 withholds the fit because the references do not provide adequately
independent evidence. A silent reference, fewer than eight supported training events
or any coefficient magnitude above 4 also withholds it. These are fixed numerical
experiment budgets, not calibrated confidence or source-separation guarantees.

Complete −20/+80 ms target windows **and every shifted reference sample** must
remain inside the training span and one 12-second section. Overlapping windows
count each sample once. Full-song evaluation uses saved coefficients, delays,
channels, analysis rate, source identities, settings and reference level. It cannot
refit. Incompatible settings/identities, invalid values, wrong analysis rate and
existing output files fail explicitly. Models and reports remain separate files.

The analyzer reuses the production source reader, native filters and linked strip
measurement. Prediction runs only on the existing block-averaged low-band waveform;
it is not the production signal path. The chosen snare/reference channels remain
explicit; stereo is not folded to mono. A negative reference delay requires future
samples in this offline diagnostic and establishes no physical capture delay.

Evaluation separates strong events, weak kick-coincident hypotheses, other quiet
events and compounds. Fixed origin-aligned 100 ms intervals also cover missed events,
silence, long sustain and the ending. Every row records exclusions, explained energy,
prediction energy and hypothetical residual-level change. Ratios near silence need
particular caution; exact/near-numerical silence has no ratio claim, and reference
energy injected into a silent target is reported separately. Split-crossing windows
are excluded, not shortened. None of these ratios measures total bleed removed or
wanted snare preserved. A model may increase energy, predict wanted playing, or do both.

Normal synthetic regressions cover independent spills, collinear/silent references,
held-out transfer reversal, overlapping-window weighting, delayed split support,
opposite-polarity stereo, contaminated quiet/unison snare, compounds, an eventless
ending, incompatible models and output reuse. A perfect fit to a contaminated
reference still withholds processing; explained energy alone is never permission.

The failure categories remain separate:

- Invalid support, timeline or saved model: reject the input/measurement contract.
- Inadequate event count or independent reference energy: abstain from fitting.
- Wanted signal cancelled in a known-source test: demonstrate a protection failure;
  a good mixture prediction cannot overrule it.
- Real recording without independent source labels: abstain from processing even
  when the numerical model fits.
- Preferred tone, rhythmic emphasis and listening acceptance: independent intent.

### Joint-reference result

The saved joint model uses kick coefficient **−0.080441** at **+1.746 ms** and
left-overhead coefficient **−0.112336** at **−5.873 ms**, predicting the left snare
channel. The snare source is mono; its duplicated channel is not summed. The delays
and channels are unchanged from the previous probes. Twenty training hypotheses
supply 12,600 unique analysis samples (2.0 seconds at 6300 Hz). The normalized
condition number is 2.350, so correlated references do not block this numerical fit.
The models were saved and hash-frozen before the full-source evaluation.

| Held-out low-band diagnostic | Kick | Overheads | Joint |
|---|---:|---:|---:|
| Weak kick-coincident events measured | 42 | 42 | 42 |
| Median explained energy, weak hypotheses | 21.86% | 40.40% | 47.75% |
| Median explained energy, strong events | 0.52% | −2.21% | −0.81% |
| Strong events where hypothetical subtraction increases energy | 26/123 | 88/123 | 76/123 |
| Compound events where it increases energy | 10/39 | 28/39 | 25/39 |

One of the 124 strong held-out events, at exactly 84.00 s, is excluded because its
pre-attack window crosses the split. The earlier scalar audit did not exclude that
window; this explains the slightly different strong-event sample count. Weak-event
counts and scalar medians reproduce the prior values. No coefficients were adjusted
after seeing these results. Imported scalar models have empty `training_events` and
zero `training_samples` in this new wrapper because it does not refit them; their
original 20-window training provenance remains in `static-pilot/prediction-probes.json`.

The joint predictor improves the weak-hypothesis median by 7.35 percentage points
over overheads alone. It does **not** establish that 47.75% of bleed can be removed.
The inferred residual is a low-band analysis waveform; no audio was subtracted,
rendered, peak-finalized or queued for listening.

Inspection of specific held-out failures gives concrete reasons to withhold it:

- **70.50 vs 71.18 s:** both are unambiguous, kick-coincident and protected by the
  previous snare-shape classifier. Hypothetical low-band attenuation varies from
  0.83 to 4.07 dB. The model still cannot distinguish spill from quiet unison snare.
- **14.70 s:** the compound cluster retains rises at 14.70 and 14.77 s, with further
  playing at 14.83/14.87 s. Joint subtraction would attenuate its low-band window
  by 3.00 dB. Collapsing those rises into one removable event would discard protection.
- **41.06 s:** the previously repaired compound includes the 41.02 s precursor.
  Hypothetical subtraction instead raises low-band energy by 0.81 dB.
- **90.65 s:** a strong protected snare event would rise by 0.76 dB. References are
  predicting a component with an unsuitable relationship to the observed target.
- **92.55 s:** a strong protected event coincident with kick would lose 2.68 dB
  in the diagnostic low-band window. Bass is also active (processed contribution
  median −19.07 dBFS in that window), so this remains a kit/bass overlap requiring
  wanted-playing evidence. No bass processing change follows.
- **85.10–85.20 s:** a fixed window between detected events would rise by 5.71 dB.
  This directly challenges an assumption that reference subtraction is harmless
  whenever the event detector is inactive. The nearest detected snare at 85.21 s
  is recorded as nearby context, not a source label for that interval.

The strong/compound changes are warnings about unconditional subtraction, not
measurements of isolated wanted-source loss. In particular, the 100 ms, roughly
40–400 Hz waveform metric is **not interchangeable** with the production 120–400 Hz
body or high-attack protection metrics. Its 0.75 dB threshold count is descriptive;
it cannot pass or fail the existing production-DSP body guard. Known-source tests
separately demonstrate that perfect prediction can cancel wanted quiet snare.
The real-recording verdict remains **insufficient source identity**, with observed
transfer failures; it is not an implementation defect or listener-preference result.

The fixed 100 ms view keeps the eventless ending visible. Reference-dependent
residual changes persist there; low levels make ratios especially unsuitable as
removal targets. All 431 detected snare events remain represented, including
compound and other quiet events that were outside the original predictor's two
summary groups. No stronger classifier, gain envelope or fitted delay was tried
as a held-out fallback.

### Evidence, reproduction and next condition

Evidence is retained under `artifacts/automix/snare-reference-v1/`:
`plan.json`, `frozen-models.json`, `freeze-sha256.json`, `full-evaluation.json`,
`summary.json`, `event-inspection.json`, validation logs and preservation hashes.
`reproduce.sh` reruns the fixed fit/evaluation into a new directory.

```sh
CARGO_INCREMENTAL=0 cargo build --locked --release
target/release/gigpies snare-reference-fit \
  ../waves/recordings/sessions/complainiacs-etc \
  artifacts/automix/snare-bleed-v1/static-pilot NEW-MODELS.json 24 36
target/release/gigpies snare-reference-evaluate \
  ../waves/recordings/sessions/complainiacs-etc \
  artifacts/automix/snare-bleed-v1/full-review-v2 NEW-MODELS.json NEW-EVALUATION.json
```

The next useful evidence is the labelled capture described above, especially
snare-only signals in each reference and independently confirmed spill-only pauses.
For this recording, independent annotation of the inspected weak events could also
resolve source ambiguity. More fitting of the same unlabelled mixtures cannot by
itself validate wanted-signal preservation. Any proposed selective processor must
still pass actual production DSP at fixed faders, then separate rhythmic/full-mix
checks before a new independently finalized export. No setting has been invented
from these predictor coefficients.

The complete normal suite passes **102 Rust and 16 Python tests**. The final CLI
training-span change also passes the focused reference suite; formatting, Clippy
with warnings as errors and the release build pass. Three historical
private-media/exhaustive tests remain intentionally ignored. Native source/strip
measurement and the frozen full-song experiment ran explicitly. Existing guitar,
bass and rhythmic settings, all original source audio, final exports and exact
listening clips remain hash-verified. Listener acceptance is unknown. No playback,
hardware change, commit or push occurred.
