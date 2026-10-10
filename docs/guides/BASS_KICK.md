# DI bass definition and kick interaction

Analyze verified DI bass and kick relationships, then evaluate bounded corrections against explicit intent. Channel and master high-pass filters may be bypassed; measurements follow configured processing. Unknown identity or insufficient evidence prevents correction.

## Implemented measurement and decisions

`src/automix/bass.rs` owns the offline rule. It reuses production source reading,
BWF alignment, biquads, stereo-linked compression, routing and FFT. It does not
modify the renderer, session schema, master chain or hardware.

```sh
CARGO_INCREMENTAL=0 cargo build --locked --release
# Diagnosis only; start/end are seconds on the common source timeline.
target/release/gigpies bass-analyze prepared.json sources new-evidence bass-policy.json 0 120
# Propose once, validate actual DSP, save settings; does not render or play.
target/release/gigpies bass-correct prepared.json sources new-review bass-policy.json 0 120
# Render only after reviewing the recorded result.
target/release/gigpies render new-review/settings.json sources new-render
```

All output directories must be new. Both measurement commands require the existing
prepared DI-only unity-source contract, explicit channel/master HPF settings
(including bypass), unmatched −0.01 dBFS finalization and no FX configuration.
Explicit indices **and filenames** must match bass DI and kick roles.

Example policy; choose indices, filenames and intent for the supplied session:

```json
{
  "bass": 7,
  "bass_file": "08_BassDI.wav",
  "kick": 0,
  "kick_file": "01_Kick.wav",
  "definition_body_db": [-12, -4],
  "max_eq_db": 4,
  "max_output_rise_db": 3,
  "max_note_spread_db": 2,
  "max_added_reduction_db": 1,
  "max_fader_db": 1.5,
  "recorded_source": true
}
```

The numerical definition range is a provisional engineering interpretation of
“more useful midrange”, not an approved style profile. These are our adaptations;
none of the new settings is labelled manufacturer-published or official.
`recorded_source: false` with a substantial correction requests another capture and
retains the old settings. This flag does not infer the physical signal path.

### Evidence and confidence

- Stream from the common origin so a pilot retains earlier filter/compressor
  history. A requested interval cannot reset an envelope at its beginning.
  The end is bounded to ten minutes. No resampling or source normalization occurs.
- Use native 16384-sample non-overlapping Hann FFT windows (371.5 ms at 44.1 kHz).
  Average stereo powers without a mono cancellation. Log nine bands from 25 Hz
  to 8 kHz and eleven stages: raw bass, EQ, compressor with makeup, routing,
  bass after master HPF, raw kick, kick/guitars/vocals after master HPF, full mix
  before/after that HPF. Instrument contributions exclude common master gain;
  the complete mix includes it. Measurements omit incomplete trailing FFT windows;
  the render keeps the full source timeline and normal renderer padding.
- Log actual linked reduction at 10 ms intervals, including mean and maximum.
  Measurements separate compressor reduction from explicit makeup and fader gain.
- Pitch analysis averages short blocks to about 6–8 kHz **for analysis only**,
  then searches normalized autocorrelation between 30 and 160 Hz. It requires
  periodicity at least 0.80, an observed fundamental with at least 3% of
  25–3200 Hz power, at least 65% concentration around ten harmonics, and pitch
  within 40 cents of an equal-tempered note. No claim of concert tuning follows.
  The averaging filter is not a general resampler; high-frequency interference,
  missing fundamentals, simultaneous notes, detuning and octave ambiguity can
  defeat the estimate. Stereo pitch follows the left source channel; stereo
  spectral power still uses both channels. Unreliable windows cannot vote.
- Even 12-second sections train; odd sections validate. Boundary-crossing windows
  cannot vote. Raw activity must exceed −65 dBFS and training p95 minus 30 dB.
  Require 12 training windows and three pitches repeated at least three times;
  held-out validation requires eight windows and two repeated pitches. At least
  70% of training votes must be below the explicit definition/body range.
  These counts and fractions are evidence heuristics, not calibrated probabilities.

The current estimator does not identify note boundaries, instrument technique,
mic placement, pickup choice or a defective source. Within-song held-out checks do not establish generalization to other recordings.

### Fixed correction budget and guards

The 54-entry grid contains no change; broad +2/+4 dB definition bells at 700/1000 Hz;
optional broad −2/−4 dB at 180 Hz; optional +4 dB/Q 3 bells at 45/55/65 Hz;
three standalone narrow low boosts; and two broad low-shelf relief alternatives.
Gain/capacity limits can remove entries. No centre frequency is fitted to a
spectral peak. Existing EQ remains, with at most eight total channel bands.

Training spectra predict the linear response. The objective minimizes median
squared distance from the explicit range plus `0.025 * sum(gain²)`. Predictions
cannot add excessive output or worsen an eligible window's tone deviation by
more than 1 dB. Repeated-note median fundamental changes must stay within
−1 to +3 dB, with at most the policy's spread between pitches. Those limits are
conservative engineering budgets, not a claim that every note should have equal
amplitude or identical EQ gain.

The chosen candidate alone runs through actual DSP; held-out data never selects a
retry. Both split medians must improve by at least 0.25 dB. The same fixed baseline
note/activity mask validates support. All windows with raw level above −65 dBFS,
including unpitched transients and the fading ending, retain output-rise and
1.5 dB crest-loss guards. Below-activity windows cannot gain more than 1 dB.
Actual 10 ms peak compressor reduction cannot increase beyond the policy budget;
full-mix peak cannot increase more than 1 dB. These checks do not certify inaudible
distortion, phase relationships at every frequency or preferred articulation.

The system records diagnosis, all proposals, candidate measurement, outcome and
applied settings separately. A separately measured fader-only proposal is never
combined silently with EQ. Neither rule changes makeup or input trim.

## Historical evidence

The [dated study and original results](../archive/studies/BASS_KICK.md) retain experiment-specific settings and validation history.
