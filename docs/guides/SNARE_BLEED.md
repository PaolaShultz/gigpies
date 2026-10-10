# Snare bleed diagnostics

Analyze kick, snare and overhead relationships while protecting ambiguous quiet and compound events. These tools do not enable a gate, expander or reference subtraction. A numerical fit alone does not establish that a component can be removed safely.

## Event detection

The offline rule:

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
rereading media, changing DSP or overwriting the earlier evidence.

## Additional ways of examining the source

`automix/bleed.rs` measures known kick, snare and overhead paths, with explicit
index/filename checks. It streams from the common BWF origin at native rate and
preserves earlier filter/compressor history before a pilot. The available views are:

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

## Failure handling

| Case | Evidence | System response |
|---|---|---|
| Implementation defect | Weak precursor locks out a later stronger hit | Fix event segmentation; retain competing rises; add regression and revalidate the unchanged rhythmic candidate |
| Valid protection failure | Static cut exceeds body/high-attack budget at identified events | Reject; record event and measured loss; keep the limit |
| Insufficient representation or labels | Known bleed can resemble quiet snare; references can contain wanted snare | Preserve listener evidence, protect ambiguous events, keep diagnostic models, abstain from automatic removal |

An accepted numerical fit is not automatically a processing permission. The report
keeps listener-confirmed bleed, instrument applicability, proposed changes, confidence,
abstention reasons, actual measured tradeoffs and null listener preference separate.
Silence, quiet snare-like hits and compound events do not become noise by default.

For a controlled soundcheck, first verify the recording tap and which controls
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
policy. Supply explicit source identities and listener evidence for the session.
This analyzer requires at least 32 kHz so all five bands remain below Nyquist.
Its streaming filters operate at source rate; the correlation representation and
feature cache are bounded to the ten-minute offline limit. It is not an audio callback.

## Validation

The normal suite covers native production measurement, history continuity, stereo
polarity, delay/sign recovery, silence, injected kick spill, quiet/compound-event
protection, fixed training templates, frozen predictor validation, actual failure
locations, excessive/missing-band abstention and refusal to overwrite. A new
regression demonstrates the important identifiability limit: confirmed spill with
snare-like features still cannot be removed automatically on those features alone.

## Historical evidence

The [dated study and original results](../archive/studies/SNARE_BLEED.md) retain experiment-specific settings and validation history.
