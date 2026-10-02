# Source rules and soundcheck advice

Implemented offline: an explicit profile supplies the intended numerical limits,
a known instrument/setup supplies applicability, and individual rules detect
measured deviations. The coordinator proposes one bounded correction bundle and
accepts it only after actual DSP and held-out checks. Source advice can stop that
bundle before automatic processing changes.

This is the beginning of a small expert system, not a finished catalogue of styles.
Names such as normal, dark, powerful or reggae do not have built-in meanings here.
We will define and test profiles instrument by instrument. The older `tone-pass`
named examples remain readable for compatibility; `source-pass` requires explicit
numerical profiles. A profile's display name never selects settings.

## Flow

```mermaid
flowchart TD
    A[Known instrument, input paths and capture setup] --> B[Musician intent expressed by a profile]
    B --> C[Measure soundcheck and classify evidence]
    C --> D{Source adjustment needed?}
    D -->|Live capture| E[Suggest one source change and repeat soundcheck]
    E --> C
    D -->|Recorded source| F[Record limitation and keep correction bounded]
    D -->|No| G[Propose enabled processing changes]
    F --> G
    G --> H[Measure actual DSP and held-out sections]
    H --> I{All enabled guards pass?}
    I -->|Yes| J[Save accepted settings and remaining deviations]
    I -->|No| K[Keep previous settings and explain rejection]
```

The current implementation reads local recordings and writes JSON/settings, with
optional offline rendering. The live capture loop, musician web page, QR stations
and monitor controls remain planned. No command sends a message to a player,
changes hardware gain, controls an amplifier or starts playback.

## Commands and profile

```sh
cargo build --locked --release
target/release/gigpies source-analyze prepared.json sources new-review source-policy.json
target/release/gigpies source-pass prepared.json sources new-result source-policy.json
```

Both commands analyze, propose and validate. Only `source-pass` renders. Outputs
must be new directories. The [tone pass's input contract](TONE_PASS.md) still
applies: unity trims, preserved native rate/timing/pan, linked stereo processing,
90/40 Hz channel/master HPFs, effects disabled and unmatched −0.01 dBFS sample-peak
finalization. Neither faders nor compressor makeup are changed by these rules.

Example policy, with **temporary laboratory values**, not a manufacturer preset or
an approved style definition:

```json
{
  "instruments": [{
    "name": "guitar station",
    "primary": 8,
    "primary_file": "10_ElecGtr1.wav",
    "secondary": [9],
    "secondary_files": ["11_ElecGtr2.wav"],
    "capture": "amplifier_microphone",
    "profile": {
      "name": "guitar laboratory profile",
      "family": "electric_guitar",
      "capture": "amplifier_microphone",
      "body_presence_db": [-2, 4],
      "dynamics": {
        "max_mean_reduction_db": 3,
        "max_p95_reduction_db": 6,
        "max_threshold_raise_db": 12,
        "max_output_rise_db": 3
      },
      "source_first": {
        "eq_change_for_recheck_db": 4,
        "body_deviation_for_recheck_db": 6
      }
    }
  }]
}
```

Implemented families are `electric_guitar` and `acoustic_guitar`; they must match
the known channel role. Capture must match in the station and profile. Supported
capture contexts are `amplifier_microphone`, `direct_input`, `acoustic_microphone`
and `recorded_track`. Use the last for historical recordings whose physical chain
is unverified; it does not identify a microphone. Other instrument families need
their own validated rules before being added.

Setting `body_presence_db` or `dynamics` to `null` disables that correction family.
Disabled rules do not rewrite those parameters. Existing EQ is retained; if a new
curve needs more than the engine's eight slots, diagnosis remains available but the
bundle is blocked. Do not delete a player's settings silently to make room.

## Implemented detectors and corrections

### Body/presence deviation

Uses the [existing guitar measurement/search](TONE_PASS.md), now with the profile's
explicit range. Training consistency, minimum evidence, sparse-spectrum exclusion,
coherent microphone summing and section guards still apply. The
[temporal revision](TONE_DECAY.md) separates fading spectral balance from steady
tone and retains actual level/compression/crest checks on the excluded fade. A confident deviation
is distinct from an available repair: an EQ budget or source action can prevent a
change even when the detector has enough evidence.

### Sustained excessive compressor action

Uses the primary path's actual linked compressor envelopes. This is a processing
fault relative to the selected profile, not a claim that a guitar was badly played
or recorded. Deliberately heavy compression can have wider limits in its profile.

- Reuse native 8192-sample windows and alternating training/held-out sections;
  exclude windows crossing section boundaries. Dynamics activity uses raw primary
  level and the raw primary/secondary energy relationship. Secondary raw powers
  are summed without cancellation; compressor action and faders cannot conceal
  the input relationship. It does not require a
  broad spectrum, so sustained notes remain measurable.
- Require at least three windows and the policy's minimum active duration in
  training. A median mean-reduction or p95 window-maximum excess of at least
  0.5 dB is needed. At least the configured consistency fraction (default 70%) of
  training windows must exceed a mean or peak-action limit. An occasional high
  peak does not by itself trigger this sustained-action rule.
- Estimate threshold relief from excess and the compressor's static slope
  `1 - 1/ratio`. Round the request up to 0.5 dB; round allowed bounds down. Bound
  it by the profile's threshold budget, predicted output-rise budget and 0 dBFS
  threshold ceiling. This estimate only proposes a change; it is not a claim to
  predict time-varying gain reduction exactly.
- Keep attack, release, ratio, knee, makeup, trims and faders unchanged. There is
  no second makeup calculation, source normalization or loudness matching.
- Re-measure the complete candidate through production DSP. Actual excess must
  improve in both splits, or remain compliant in an already-compliant split.
  Median output rise cannot exceed the profile budget plus 0.05 dB numerical
  tolerance. Split median/p10 crest cannot fall beyond the existing crest guard.
  Individual sections also have output-rise limits, and no eligible window may
  acquire excessive additional compressor action.

Threshold relief may raise the source's contribution to the band mix. The level
budget limits that tradeoff; it does not prove preferred musical balance. If the
budget prevents full relief, the report retains the remaining deviation.

### Repeated PCM full-scale contact

At least three eligible training windows with 0.1% or more input samples touching
PCM full scale block the repair bundle. Repeated held-out contact also vetoes it.
This can indicate an input/recording problem, but does not identify where it happened
or prove the waveform can be recovered. The system requests an input-path review
instead of claiming to repair missing detail with EQ.

Floating-point input is explicitly **not applicable** to this PCM detector: valid
float headroom can exceed unity. Clipping recorded below full scale, analog clipping
and arbitrary distortion are not detected by this rule. Passing it does not certify
source quality.

## Adjust the source first

With adequate training confidence, either a proposed EQ band at least 4 dB in
magnitude or a body-range deviation at least 6 dB requests source review by default.
These operational thresholds are editable in `source_first`; they are not laws of
acoustics. Both boosting and cutting can trigger review.

For a known amplifier microphone, the report asks the musician to:

1. Compare the amp heard at the player's position with the primary mic signal.
2. If the amp itself is lean, add a little bass/low-mid. If the amp sounds right,
   try a small mic-position change first. For excess body, use the corresponding
   small reduction or placement check.
3. Change one thing, then repeat the same quiet, normal and strong phrases.

The sound at the player's position and the captured signal are different evidence.
The report does not infer cone position, proximity effect, microphone directivity,
room contribution or faulty hardware from a spectral ratio. `suspected_physical_cause`
is deliberately unset. These are controlled source-check suggestions, not a claimed
physical diagnosis or a prescribed knob amount on an unknown amp.

A DI gets instrument pickup/tone and input-routing advice, not amp instructions.
An acoustic mic gets instrument-versus-mic and placement/distance advice. A historical
recording gets a limitation report and may still receive a bounded, validated
software improvement. Repeated PCM full-scale contact blocks both contexts and asks
for an input-path check or a better original.

When live-capture advice requests a repeat, the current offline decision keeps the
previous processing. A subsequent run must measure the new capture against the
saved baseline; do not stack corrections from the old attempt. In the future live
workflow, monitor controls stay usable while automatic source preparation waits.
This does not require a new consent screen for each routine soundcheck step.

## Rule interaction and evidence

Body EQ and threshold relief are selected from the same original training data.
The combined candidate is measured once. Enabled rules are checked even when they
did not propose a change, so solving tone cannot silently violate the compressor
profile and vice versa. Reject the entire candidate if a guard fails; there is no
held-out retry, fallback search or optimization against a produced reference.
A source-action request, input-contact veto or unavailable EQ capacity prevents
candidate measurement/application and is reported separately.

`decisions.json` contains:

- Profile, known identity, detector states and empirical consistency (not probability).
- Separate plans, original/candidate/applied settings and whether the candidate was
  actually measured. Validation is null when a prior gate blocked it.
- Source advice, requested steps, repeat-soundcheck status and applicability.
- Actual applied metrics and independent `body_target_met` / `dynamics_target_met`
  values. Null means disabled or insufficient evidence; an accepted partial
  correction can still leave a target false.
- An unset listener-preference field and the level/masking tradeoffs.

Per-instrument files preserve original/candidate windows and separate body/dynamics
activity masks. `settings.json` contains only accepted changes. Media and detailed
private evidence stay ignored; output audio is never published with code.

## Validation and next work

Normal synthetic tests cover actual compressor relief, linked stereo rendering,
output/crest limits, quiet playing, silence, isolated peaks, source preservation,
profile applicability, disabled rules, arbitrary display names, combined-rule
acceptance/rejection, full-scale contact, float headroom, held-out independence,
EQ-capacity abstention and setup-specific source advice. Tone tests additionally
cover correlated microphones, sparse musical decays, transient guards and section
regressions. Historical private-media tests stay opt-in.

Future detector families include unwanted narrow resonances, noise/bleed, broader
spectral problems and attack/sustain problems. Their presence is not assumed in
these recordings. Style names and instrument-specific rule sets are future design
work, not a preset taxonomy implied by earlier examples. Full-band and monitor
context, measurement-driven speaker calibration and the live QR flow also remain
separate work. No console emulation or hardware validation is claimed.

## Local experiment, 2026-10-02

Evidence is retained locally under `artifacts/automix/source-rules-v1/`; final
reports use `review-v2`, `full-review-v2` and `injected-review-v2`. The first reports
are retained to explain a detector regression: using the processed secondary/primary
ratio hid severe primary compression. Dynamics eligibility now uses raw input powers,
with a synthetic regression for both an overcompressed primary and excessive raw
secondary contribution.

Five synchronized 24-second pilots used existing manufacturer-study processing and
fixed faders. Offsets below are relative to each session's common start. Historical
capture context was explicitly `recorded_track`; filenames did not establish physical
mic placement. Both MedleyDB pilots used RAW audio. The trial body range was −2 to +4 dB;
compression limits were 3 dB median mean action / 6 dB p95 window-maximum action,
with at most 3 dB output rise. These are experimental limits, not approved musician
profiles or manufacturer settings.

| Material | Window | Result |
|---|---|---|
| Complainiacs, known two-path guitar | 24–48 s | Body deficit; a strong local proposal passes this short window but does not establish whole-song suitability. Source limitation reported. |
| Complainiacs, full song | Entire source | +1 dB at 300 Hz and −1 dB at 2400 Hz, both Q 0.7, accepted. Held-out body/presence improves from −8.49 to −6.87 dB; target remains unmet. |
| Wild & Co, selected EGT2tf11 path | 168–192 s | Tone varies too much for the consistency gate; no correction. |
| Catbite, selected guitar path | 156–180 s | Within trial body/compression limits; no correction. |
| Phoenix, selected RAW acoustic guitar | 36–60 s | −3 dB at 220 Hz and +3 dB at 1600 Hz, Q 0.7, meets the trial range in both splits. This is a pilot policy result, not an approved acoustic tone or full-song result. |
| Rainfall, separate RAW acoustic/electric guitars | 36–60 s | Insufficient eligible body evidence for both paths; electric dynamics also abstains. No correction. |

None of the five baseline pilots triggered sustained-compression repair or repeated
PCM-contact blocking. No source-quality grade follows from that result. Rainfall's insufficient
evidence is an abstention, not a clean bill of health. Dark Ride's full-band experiment
remains deferred without a verified bass DI.

A separate **deliberately injected processing fault** changed the Complainiacs
primary compressor to threshold −30 dB, ratio 8:1; recorded samples stayed unchanged.
With body correction disabled, the rule raised threshold to −27 dB. Held-out median
mean reduction fell from 18.38 to 15.84 dB, with 2.56 dB output rise and negligible
crest change. The bounded partial correction passed, while `dynamics_target_met`
correctly remained false. This validates detection/limited relief, not an allegation
that the original recording or baseline mix had that fault.

The full-song accepted settings are identical to the previous `guitar-tone-v1/full-v3`
settings. Its existing 12-second CURRENT MIX → AUTOMATIC TONE MIX pair remains the
listening evidence; no duplicate render or listening round was added. New acoustic
pilot settings are analysis evidence only. Listener preference remains separate from
all these technical/policy results; no audio was played or claimed heard.

Validation: 68 normal Rust tests and four Python tests pass; Clippy with warnings
as errors and formatting pass. Three historical private-media tests remain ignored;
the five local pilots and full-song analysis above were run explicitly. Original
source hashes were verified. Temporary pilot media were removed after retaining
input manifests, configurations, measurements and a local reproduction script.

The earlier Complainiacs listening evidence above is superseded by the stronger
correction and single comparison pair in [TONE_DECAY.md](TONE_DECAY.md).
