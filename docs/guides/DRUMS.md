# Kick, snare and rhythmic emphasis

Measure drum processing and kick/bass interaction, evaluate bounded tone/dynamics proposals and apply rhythmic fader offsets only with explicit intent. Measurements follow configured channel and master filters, including bypass. See [source preservation](SOURCE_PRESERVATION.md) and [bleed diagnostics](SNARE_BLEED.md).

## Reusable implementation

`automix/drums.rs` owns source measurement, drum evidence, bounded probes and joint
validation. `balance::rhythmic_emphasis_delta` owns the independently editable
artistic offset. They reuse the production `Source`, BWF alignment, `Strip`, routing,
biquads and FFT. Shared FFT helpers provide consistent measurements.

1. Validate explicit kick/snare/DI indices **and filenames**, native session rate,
   unity trims, pan/routing, explicit channel/master HPFs (including bypass), unmatched export and no FX.
   Measurement streams from the common origin, preserving compressor/filter history
   before a pilot. The requested span is bounded to ten minutes.
2. Record 15 synchronized stages. Ten-ms frames retain RMS, sample peak, linked mean
   and maximum reduction, plus raw role-band power. Rational boundaries retain every
   sample, including the final partial frame. Independent 16384-sample stereo-power
   Hann FFTs cover nine bands from 25 Hz to 8 kHz. Incomplete final FFT blocks are
   omitted; time-domain guards and the renderer still cover the complete timeline.
3. Detect a 5 dB raw role-band rise over the prior 40 ms, above −65 dBFS and training
   p95 minus 35 dB. Cluster the bounded 80 ms neighborhood, retain competing rises and
   anchor the strongest role-band peak. Kick uses approximate 40–120 Hz power, snare 120–500 Hz.
   These bands subtract second-order HPF outputs and are not rectangular FFT bands.
4. Compare 0–30 ms attack, 30–80 ms body and 150–240 ms recovery; log 80–240 ms tails.
   Recovery/tail statistics stop at the next cluster’s earliest precursor;
   isolated recovery requires the full 240 ms window.
   Exclude events crossing 12-second split boundaries. Even sections train; odd
   sections validate. Require eight events in each split for automatic acceptance.
   Sub-80-ms rolls and overlapping bleed can defeat the detector; no beat, fill,
   microphone-placement or source-separation classifier is claimed.
5. Test at most two static EQ probes, plus unchanged processing: reduce the first
   existing positive kick bell in 70–130 Hz by 1.5 dB, and the first existing positive
   snare shelf in 4–8 kHz by 2 dB. Keep faders fixed and add no EQ bands. These are
   our bounded sensitivity probes, not manufacturer settings. With no established
   tone target or isolated bleed evidence, the coordinator retains EQ and logs
   the reason. General automatic drum tone-target selection remains unimplemented.
6. Compressor relief requires at least eight isolated training events, attack/body
   contrast loss over 1.5 dB in at least 70% of training events, **and** median late
   reduction over 3 dB. Maxima alone cannot trigger it. Propose +2 dB threshold once;
   keep attack/release/ratio unchanged. Measure the change in 0–80 ms event output
   and adjust **existing** makeup by its negative training median, capped at ±2 dB.
   Both splits must show at least 0.25 dB median reduction relief and at most 0.5 dB
   residual event-level change. This path is an initial engineering rule, not a
   universal correct-compression definition.
7. Compute artistic fader delta as desired emphasis minus explicitly recorded current
   emphasis, never as desired minus absolute fader. Unknown current emphasis abstains;
   offsets and changes are bounded to 3 dB. Measure processing-only, fader-only and
   combined results separately. Identical processing is reused without redundant DSP.
8. Use baseline event masks and all time-domain frames for technical validation.
   Processing-only active output change stays within ±3 dB; quiet output rise stays
   within 1 dB; added peak reduction stays within 1 dB; every event's attack/body loss
   stays within 1.5 dB. Subtract the explicit artistic fader delta when assessing
   processing output, but retain the full actual mix-peak budget of +3 dB.
   Repeated PCM contact uses the existing source-rule fraction/count thresholds;
   isolated contacts remain reported. Float headroom never becomes that PCM diagnosis.
9. Freeze the pilot candidate, then run `drum-verify` on the complete source timeline.
   It cannot select a replacement or change protected settings. Rejected proposals
   retain the baseline. A successful technical check does not certify preference.

Reports separate baseline evidence/diagnosis, intent and basis, static proposals,
processing plan, makeup change, fader proposal, actual outcomes, applied settings
and null listener preference. No produced-reference features enter these commands.
Reference mixes supply comparison evidence, not automatic spectrum, loudness or balance targets.

## Commands

```sh
CARGO_INCREMENTAL=0 cargo build --locked --release
mkdir -p artifacts/automix/my-drum-review
# Inspect a pilot without proposing or rendering.
target/release/gigpies drum-analyze BASELINE.json SOURCES NEW-ANALYSIS POLICY.json 24 48
# Bounded processing probes, processing/fader proposals and held-out checks.
target/release/gigpies drum-correct BASELINE.json SOURCES NEW-PILOT POLICY.json 24 48
# Reanalyze a frozen candidate; no search against full-song/held-out results.
target/release/gigpies drum-verify BASELINE.json NEW-PILOT/settings.json SOURCES NEW-FULL POLICY.json 0 120
# Render only the accepted settings after reading the outcomes.
target/release/gigpies render NEW-FULL/settings.json SOURCES NEW-RENDER
python3 scripts/drum_evidence.py NEW-FULL --baseline
python3 scripts/drum_evidence.py NEW-FULL
```

Every command output directory must be new. Example policy; replace source identities and intent for the supplied session:

```json
{
  "kick": 0, "kick_file": "01_Kick.wav",
  "snare": 1, "snare_file": "02_Snare.wav",
  "bass": 7, "bass_file": "08_BassDI.wav",
  "neutral_basis": "Current processed fit, provisional; no previously added rhythmic offset",
  "current_emphasis_db": [0, 0],
  "desired_emphasis_db": [2, 2]
}
```

This does not turn a text assertion into perceptual evidence. Record the actual
basis and use `null` for unknown current emphasis. Always start from a saved baseline;
rerunning from corrected settings with current emphasis reset to zero would stack
an intentional gain twice.

## Historical evidence

The [dated study and original results](../archive/studies/DRUMS.md) retain experiment-specific settings and validation history.
