# DI bass definition and kick interaction

**Current contract:** [offline source preservation](SOURCE_PRESERVATION.md) permits
explicit channel/master HPF choices including bypass. Bass analysis now measures
the configured master filter. Verified DI identity and all correction guards remain.

Implemented and offline-validated, 2026-10-02. This extends the source-rule system
with a separate DI/kick policy and commands. Guitar policies remain unchanged.
The accepted Complainiacs guitar settings are preserved. Listening preference is
unmeasured; no audio was played, committed or pushed.

## What the evidence supports

The bass has substantial low-harmonic energy, weak midrange definition, and
note-dependent low-frequency support. “Empty” does not mean that the DI lacks all
bass frequencies. In the 24–48 s pilot, confidently pitched raw windows had median
100–200 Hz power around −21.5 dBFS, versus −41.6 dBFS at 400–800 Hz and −46.8 dBFS
at 800–1600 Hz. These are band powers, not equal-bandwidth perceptual scores.
The low-mid emphasis persists within several repeated pitch groups; it is not
explained solely by a different distribution of played notes.

The current processing contributes to that shape:

- Bass EQ: −7.5 dB low shelf at 35.5 Hz, +4.5 dB at 112 Hz/Q 5,
  +2.5 dB at 2 kHz/Q 4.5, and a neutral 4 kHz shelf.
- Compressor: threshold −12 dBFS, ratio 2:1, attack 15 ms, release 470 ms,
  explicit makeup +4.5 dB. Bass fader −1 dB; unity input trim.
- Master: 40 Hz second-order HPF and independent sample-peak export gain.
  There is no master limiter action in this unmatched render.

At 41.2 Hz, the existing channel EQ attenuates about 2.69 dB and the master HPF
another 2.76 dB. At 65.4 Hz those losses are about 0.51 and 0.57 dB; at 98 Hz the
channel EQ instead adds 1.48 dB and the HPF removes only 0.12 dB. These are filter
responses, excluding compression, makeup and the musical amplitude of each note.
The HPF does not make a sub boost ineffective, but substantial energy added below
its corner would be inefficient and still load the channel compressor beforehand.

Guitars overlap the definition band. Before correction, held-out repeated-note
medians put bass 400–1600 Hz power roughly 12–18 dB below the guitar sum. Vocals
add competition when active. Band overlap establishes a masking risk; it cannot
measure audibility or listener preference. Bass/kick 40–100 Hz relationships vary
with pitch and simultaneous playing: held-out C2-like windows put bass about
6.3 dB below kick; E2-like windows put it about 1.1 dB above. A whole-song average
would conceal that difference. No kick processing was changed.

Heavy bass compression is not supported by these measurements. Actual 10 ms mean
reduction has a 0.39 dB active median; the maximum instantaneous reduction is
2.32 dB. Quiet/strong active quartiles have median mean reductions of 0.15/0.80 dB.
A causal 40 ms energy-rise detector found 238 possible attacks: median reduction
in the first 30 ms was 0.42 dB. For 129 events with no next detected rise inside
300 ms, the 100–300 ms median was 0.62 dB. These are energy events, not verified
articulation labels. The ending sustains and fades with little compressor action.
There is no evidence here for raising makeup or changing attack/release.

Approximately 1.3 s of 10 ms frames combine low DI activity with kick above
−35 dBFS; median DI RMS there is −67.8 dBFS. That is limited evidence against
large kick bleed in those pauses, not proof of an isolated or noiseless DI.
An intentional bass-only/kick-only soundcheck is still needed to separate bleed
from correlated performance more reliably. The bass amp was excluded throughout;
its signal was not used to decide processing.

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
prepared DI-only unity-source contract, 90 Hz channel HPFs except bass/kick,
40 Hz master HPF, unmatched −0.01 dBFS finalization and no FX configuration.
Explicit indices **and filenames** must match bass DI and kick roles.

Example policy used for this local experiment:

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
mic placement, pickup choice or a defective source. The ending's apparent octave
changes remain uncertain. The song informed development: these held-out sections
are within-song checks, not an independent blind generalization study.

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

## Safeguard investigation

The first full-song candidate failed a new unconditional PCM-contact veto.
Inspection found one contiguous 46-sample positive full-scale plateau at 80.197 s,
about 1 ms. This is consistent with a clipped/limited event; its upstream cause
and recoverability are unknown. It cannot explain persistent weak midrange alone.

That veto incorrectly conflated an isolated event with the existing expert
system's **repeated** contact rule. Both modules now share its unchanged thresholds:
0.1% contact in at least three analysis windows. Bass checks all windows, including
unpitched events. Isolated contact is explicitly reported and remains subject to
all ordinary output, crest, compressor and headroom guards. Repeated contact
blocks correction and requests source review. Float headroom is not PCM clipping.

In the affected window, the unchanged candidate reduces bass RMS by 0.69 dB,
raises its after-HPF peak by 0.05 dB, increases crest, and reduces mean/max compressor
action. It does not recover the plateau. Synthetic checks cover both isolated and
repeated contact and exaggerated output/transient action. The rejected first
report and the waveform-run investigation are retained. No EQ, intent range or
level limit was retuned to obtain the second verdict.

Narrow boosts were investigated independently as well. A +4 dB/Q 3 bell at 45 Hz
adds about 3.11 dB at 41.2 Hz and 3.16 dB at 49 Hz, but only 0.15 dB at 98 Hz.
The 65 Hz bell adds about 3.99 dB at 65.4 Hz versus 0.45 dB at 41.2 Hz. All tested
narrow options exceed the note-spread budget; some also exceed the per-note boost
budget. They leave weak midrange definition unresolved. Restoring known low-note
attenuation could justify unequal gains under a future explicit support intent;
this guard alone does **not** prove that narrow EQ would sound bad. The evidence
here does not establish a suitable narrow centre or need for a substantial sub
boost. Broad shelf relief passed the prediction guards but scored worse for this
experiment's definition intent; it was not given an actual-DSP/full-song render.

## Accepted local result

Two bands appended to the bass DI only:

| Parameter | Change |
|---|---:|
| Broad bell | +4 dB at 700 Hz, Q 0.7 |
| Broad bell | −2 dB at 180 Hz, Q 0.7 |
| Bass fader and compressor | Unchanged |
| All guitars, drums, vocals, pan, trim, timing, HPFs and makeup | Unchanged |

The 24–48 s pilot abstained for inadequate training pitch coverage. Expanding to
0–48 s supplied three repeated training pitches and validated this candidate before
the full-song run. The full run selected the identical bands from 77 eligible
training windows/six repeated pitches; 55 held-out windows/five repeated pitches
validated them.

| Full-song check | Current → corrected |
|---|---:|
| Training definition/body median | −20.38 → −16.32 dB |
| Held-out definition/body median | −20.63 → −16.73 dB |
| Held-out repeated-note fundamental changes | About −0.77 to +0.17 dB |
| Maximum added compressor reduction | 0 dB |
| Full-mix sample peak before export | −0.15 dB change |
| Independent processed export gain | −6.503 → −6.355 dB |
| Integrated loudness, measured only | −9.34 → −9.41 LUFS |
| Final sample peak, both exports | −0.01 dBFS |

The provisional −12 to −4 dB definition range remains unmet. This is a bounded
partial improvement, not a completed tone match. The 180 Hz cut slightly reduces
upper-note fundamental support; the range above records that tradeoff. The global
export gain differs by about +0.15 dB, as required by independent peak finalization;
guitar processing, faders and relative pan relationships are preserved. There is
no source normalization, loudness matching or true-peak claim.

The separate +1.5 dB bass-fader option changes every bass band by +1.5 dB, leaves
compression unchanged and raises the mix peak only 0.10 dB. It passes those
technical budgets but does not change the bass definition/body ratio. Its settings
and measurements remain available; it was not rendered into another listening
queue or combined with the selected EQ.

For a future soundcheck, first establish where this DI is tapped. If pickup/tone
controls or a pre-DI pedal affect it, compare one modest source change using the
same quiet, strong and sustained phrases. Include bass-only, kick-only and overlap
passages, then **repeat the measurement**. Do not prescribe an amp adjustment
without evidence that its controls affect this DI. For the historical recording,
retain the limitation and avoid stacking another correction from these settings.

### Reference and rhythmic intent

The supplied produced mix was independently realigned and measured. At 24 s it
has about 3.49 dB more 40–100 Hz share and roughly 5.13 dB higher RMS than the
current mix, with lower mid/presence shares. These are whole-mix differences;
they do not isolate bass from kick or establish a desirable target. No reference
file or feature enters `bass-correct`.

The musician also described a preference for kick and snare about 2 dB forward
of a neutral balance, judged **after compression and its makeup gain**. That is
a reasonable rhythmic intent for a later balance experiment. A 2 dB rise alone
does not establish masking; heavy compression with makeup can also alter sustained
energy. Current kick/snare makeup is already +5.5/+3.5 dB. Preserve editable faders
and do not add a second makeup correction. This task left both drums unchanged
so the prepared pair isolates the bass-setting change.

## Files, verification and limits

Local evidence: `artifacts/automix/bass-kick-v1/`. Final decisions and plots are in
`full-review-v2/`; the first rejected report is in `full-review/`. The exact
12-second pair starts at 24 seconds of each independently finalized full export:

1. `listen/01-CURRENT-MIX-024s.wav`
2. `listen/02-BASS-CORRECTED-MIX-024s.wav`

Both PCM payloads were read and verified against their source slices. Playback
has not occurred. On request, use the established prepared-pair player with a
one-second gap, once per clip; finish file and sink preflight before either starts.
Additional candidates are settings/evidence only, not queued audio.

All 13 original media hashes and the accepted configuration/render hashes remain
unchanged. Existing uncommitted reference-review and media-library work is preserved.
The normal suite passed: 79 Rust and eight Python tests, Clippy with warnings as
errors, formatting and release build. Three historical private-media tests were
intentionally skipped; this task's pilots, full-source measurements, independent
reference check and final render were run explicitly. No hardware was opened.

Normal bass regressions cover quiet/changed notes, silence, noise, kick-only pauses,
pitch confidence, actual compressor action, held-out independence, identity/routing
contracts, excessive budgets, narrow-note emphasis, isolated/repeated PCM contact,
transient/output rejection and report non-overwrite. The detector still needs
other recordings, controlled soundchecks and listener feedback. Its numerical
thresholds and band intent should not become universal bass presets by implication.

## Later ensemble reassessment and meter correction

The [checkpoint reassessment](COMPLAINIACS_REASSESSMENT.md) reopens the definition
intent after listener feedback about bass/body. Its body-restoration probe remains
unselected; the chosen new FINAL retains this EQ and raises the DI fader by 1 dB
while revising the surrounding ensemble. Earlier policy acceptance is not listener
acceptance. The −9.34/−9.41 LUFS values above describe **pre-export buses**; the actual
finished guitar/bass exports both measure approximately −15.8 LUFS. The older values
are retained for provenance and must not be compared to finished-file meters.
