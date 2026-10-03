# Summing mixer investigation and implementation plan

Prepared 2026-10-03. **Status: planned, awaiting later execution.**

This plan investigates whether GigPies loses body, punch or clarity while combining
sources, then implements the changes supported by the experiments. The current
64-bit summing loop is a reasonable foundation. The first priorities are an
independent check of neutral summing, true-peak delivery headroom, explicit export
controls and measurements that explain how processing changes the ensemble.

The planning task changes documentation only. When the user later requests execution,
follow the phases below through implementation, offline validation and preparation
of review material. Record musical acceptance separately; measurements cannot supply
a listener preference. Preserve original recordings and the frozen settings/evidence
that identify earlier selected mixes. Generated audio was retired as described below.

## Render retirement and execution baseline — 2026-10-03

The user subsequently requested removal of **all generated renders**, including old
SOURCE sums, six FINALs, float buses, diagnostics and listening excerpts, before
this engine work. Original recordings, notices, source archives, frozen settings,
measurements and identity manifests remain. The ignored retirement manifest is
`artifacts/automix/render-retirement-2026-10-03/manifest.json`.
Earlier listening indexes are historical and no longer supply playable files.
This instruction supersedes the initial plan's physical retention of old mixes;
the original draft is preserved in [the archive](archive/summing-plan-initial-2026-10-03.md).

Begin engineering from the saved settings, original-source identities and scalar
evidence. Use generated fixtures during development; synthetic test WAVs and temporary
test renders are allowed. Evaluate real sources without writing listening audio
during phases 0–4. Create new complete listening renders after implementation and technical
validation, in phase 5. At that point reproduce only the legacy baselines needed
for compatibility/comparison, verify their retained hashes, and label them as
reconstructed historical baselines. Do not rerun old interleaved planners to recover
settings that are already frozen. A failed historical hash comparison is an explicit
compatibility failure, never silently a new accepted baseline.

The [execution prompt](SUMMING_EXECUTION_PROMPT.md) carries this scope into the next
session. Do not create equal-LUFS listening copies under the present instruction;
comparison-copy controls can be implemented and tested synthetically. The proposed
new independent delivery is minus 1 dBTP after estimator validation; legacy replay
keeps its recorded minus 0.01 dBFS contract. Neither ceiling establishes better sound.

## Scope and ownership

- GigPies owns its offline routing, rendering, export policy, observations and
  comparison workflow. Use Rust 1.97.1, edition 2024 and the committed Cargo.lock.
- Follow [source preservation](SOURCE_PRESERVATION.md). Existing channel tone,
  faders, pan and requested GigPies effects form the starting point. Unknown intent
  preserves settings. A failed experiment retains the baseline.
- Follow [component ownership](COMPONENTS.md). PA processing, measurement and
  alignment algorithms belong in SHR PA. Sibling repositories remain read-only
  unless the execution session explicitly authorizes changes there. An experiment
  here must not become a second production PA alignment implementation.
- FX routing and integration are inspected here. An underlying shared FX algorithm
  issue needs an ownership decision under the component map before implementation.
- This is an offline task. Playback, hardware operation and host audio, MIDI, DMX or
  service changes need the applicable session authorization. An instruction to execute
  this plan covers its offline work; it does not itself request audible playback.
- Commit, push and release follow the user's publication scope and
  [publication policy](PUBLICATION.md). New reusable scripts require a reviewed
  entry in `scripts/publication-policy.json`. One-off runners and private evidence
  remain ignored.

## Evidence available at planning time

These observations are dated starting evidence. Recheck the relevant implementation
and file identities in phase 0 because another task is changing the working tree.

| Observation | Evidence and limit |
|---|---|
| Main addition uses `f64`, with floating buses retained above full scale | `src/automix/render.rs`, `route` and `run`; no independent full neutral null experiment was performed in the research task |
| Mono centre is approximately minus 3.01 dB per side; stereo centre passes L/R unchanged | `render::route`; stereo pan attenuates the opposite side as a balance control |
| Unmatched export independently scales each complete bus to its largest sample peak | `render::run`; static gain preserves internal ratios and crest but changes absolute listening level |
| `Matched` enables the final sample limiter and `Unmatched` skips it | `render::run`; output comparison mode currently affects processing |
| The FX rack has its own master limiter | `effects::Rack::master`; bypassing the final sample limiter does not bypass this separate processor |
| All six current expert FX FINAL reports record zero action from both master limiters | Saved `prepared.json` and `measurements.json` under `artifacts/automix/expert-fx-v1`; this excludes master gain reduction as an explanation for those particular exports |
| Phoenix FINAL measured approximately plus 0.3 dBTP at a configured minus 0.01 dBFS sample ceiling | Fresh full-file FFmpeg 7.1.5 scan in the research task; the other five FINALs rounded to approximately zero dBTP at 0.1 dB display precision; no audible clipping was established |
| Offline review and checkpoint preparation require unmatched minus 0.01 dBFS export | `balance::validate_baseline`, `scripts/listening_checkpoint.py` and their tests; changing only a default ceiling would break this contract |
| Earlier processing caused measurable ensemble and export costs | [Dark Ride audit](MULTITRACKS.md#dark-ride-source-and-processing-audit--2026-10-02) and [Complainiacs reassessment](COMPLAINIACS_REASSESSMENT.md); these concern superseded mixes, not a fresh defect in current FINALs |

The research task inspected source/settings and measured existing files without
playback or rendering. Its FFmpeg output is recorded in the conversation, not a
new retained evidence file. Fresh measurements must be recorded during execution.
The historical audio has since been retired; phase 0 starts from the preserved reports and phase 5 measures
newly rendered files.

## Required outcomes and conditional changes

Complete the following engineering work unless current code already satisfies it:

1. Independent neutral summing and routing checks with focused production regressions.
2. Validated true-peak measurement and an explicit offline delivery policy.
3. Separation of comparison gain from enabled master processing, with legacy replay.
4. Descriptive stage, peak and finished-file measurements sufficient to explain gain
   and processing consequences without assigning musical preference.
5. Frozen comparisons, complete export checks and a concise result for all six examples.

Change summation, routing or DSP only when a reproducible defect justifies it.
Changes to microphone delay/polarity, filters, compression, makeup, faders, FX amount
or bus coloration require a stated problem and a bounded successful comparison.
Do not install automatic source normalization, universal HPFs, per-sample bus
normalization or saturation as a generic summing improvement.

## Execution order

| Phase | Work | Required exit |
|---|---|---|
| 0 | Reconcile current state and freeze inputs | Complete identities, retained evidence, pending scan list and declared budget |
| 1 | Verify neutral arithmetic and routing independently | Explained numerical residuals; confirmed defects fixed before musical trials |
| 2 | Observe processing, correlation and peak costs | Reconciled stage report and specific hypotheses or explicit no-change findings |
| 3 | Implement explicit delivery and comparison controls | Legacy replay preserved; true-peak and mode-independence checks pass |
| 4 | Run bounded musical and phase experiments | Frozen candidate passes held-out checks or baseline retained |
| 5 | Verify complete outputs and prepare listening | Reproducible offline result, exact review material and pending or recorded preference |

## Phase 0 Freeze the execution baseline

Read `AGENTS.md`, `README.md`, `docs/STATUS.md`, this plan and the owning documents
before changing code. Relevant owners include `AUTOMIX.md`, `UNITY_PASS.md`,
`SOURCE_PRESERVATION.md`, `ARTISTIC_FX.md`, `BALANCE_PASS.md`,
`REFERENCE_REVIEW.md` and `DEVELOPMENT.md`.

1. Inspect current Git state without resetting, stashing or staging unrelated work.
   Identify overlapping files and active builds. Use an agreed stable snapshot for
   evidence; do not attribute another task's incomplete edits to a mixer defect.
   An isolated checkout must include any deliberately selected local baseline changes,
   rather than silently testing an older HEAD. Do not commit solely to make a snapshot.
2. Create a fresh ignored run directory under `artifacts/automix/summing-study/`,
   using a unique dated child directory for each study.
   Record commit identity, relevant working-file hashes, tool versions, settings,
   export policies and SHA-256 identities for sources and retained comparison files.
   Record a clean completion marker only after all required outputs validate.
3. Resolve the retired index at `artifacts/automix/expert-fx-v1/LISTEN.md` and
   the retirement manifest. Verify all six frozen SOURCE/FINAL settings and original
   input locations. Keep the later EQ diagnostics separate from the clean baseline.
4. Read saved full-file, gain-reduction and export-gain reports. Mark unavailable
   fresh PCM measurements as pending until phase 5; never invent them from settings.
   Prepare the reproduction inputs and expected hashes without rebuilding old audio.
5. Freeze training and held-out regions, event definitions, bands, thresholds and
   candidate budget before tuning. Use the existing region declarations where valid.
   For stateful DSP, all training ends before held-out material begins. Replay from
   sample zero with continuous state even when collecting only short observations.

Use Complainiacs for correlated microphones and the previous snare/export concern,
Dark Ride for dense layering and designed source sounds, and Phoenix for true-peak
headroom. Rainfall, Wild & Co and Catbite supply contrasting production regressions.
Do not infer acoustic-microphone relationships for sampled or unidentified tracks.

Before substantial builds or renders, check free space and relevant output sizes.
Review cleanup below 20 GiB free or above 5 GiB of project build output. Check active
processes, open files and locks before deleting known task-created caches. Reuse the
normal target directory with `CARGO_INCREMENTAL=0`. These thresholds do not authorize
removing unknown data or another task's output.

**Exit:** a verified baseline manifest, saved measurements with fresh scans pending, frozen region
policy and a list of implementation differences from the planning-time observations.

## Phase 1 Prove the neutral signal path

Build a small independent reference calculation from the documented routing equations.
It must not call `render::route`, the production sum loop or production export-gain
calculation. Use analytical expectations for simple fixtures and a compensated or
higher-precision sum for the general reference. Sharing decoded source samples is
acceptable if WAV decoding is checked separately against known PCM/float values.

Run comparisons before export gain with channel/master processing disabled. Compare
the `f64` result where observable, retained float32 bus and PCM boundary separately.
Record the precise conversion and rounding rules at each boundary.

| Experiment | Expected behaviour |
|---|---|
| One channel, zero gain and silence cases | Exact routing and silence; no invented gain or channel contribution |
| Two identical and two opposite-polarity inputs | Twice the amplitude and cancellation respectively, before export |
| Known uncorrelated sequences | Sum power agrees with the actual cross terms; approximately plus 3 dB for equal uncorrelated powers is a statistical check, not a peak bound |
| 1, 2, 16, 40 and 64 channels | Correct weighted addition through the supported channel limit |
| High-level and very quiet signals; severe cancellation; reordered channels | Only bounded numerical residuals, with no perceptible level-dependent transfer |
| Gain sweep through full scale on the float bus | No hidden clipping, saturation or normalization in the neutral sum |
| Add an all-zero channel | Existing output unchanged; channel count cannot control gain |
| Mono pan at endpoints, centre and intermediate positions | Sine/cosine law and expected total electrical power |
| Stereo basis signals and duplicated mono | Correct L/R mapping and balance attenuation; document the existing 3.01 dB per-side mono-versus-duplicated-stereo difference |
| BWF offsets, unequal lengths and block boundaries | Correct sample positions, padding and identical frozen output across block sizes |
| FX routing impulses and zero amounts | Dry path counted once, wet returns counted once, no accidental feedback or channel leakage |

Declare numerical limits before looking at results. For neutral `f64` addition,
derive an absolute forward-error bound from the number of operations and the sum
of absolute terms. Relative error against a nearly cancelled output is unsuitable.
Include routing-product rounding and one float32 rounding step when comparing the
stored bus. For deterministic undithered PCM, require the expected rounding result;
permit a one-LSB difference only where an independently specified boundary tie
explains it. Do not adopt a generous audio-level tolerance to hide a routing error.

Use short fixtures at 44.1, 48 and 96 kHz in normal tests, with focused endpoint
coverage for the schema's 8 to 192 kHz range. Keep long sweeps opt-in. Reuse existing
tests in `tests/automix.rs`, `tests/unity.rs`, `tests/balance.rs` and
`tests/preservation.rs` where they protect the same contract.

Also compare the neutral production stream against an independent reconstruction
of fixed real-source passages. Before phase 5, use in-memory or bounded-window
observations instead of recreating complete real-song audio. Match initial SOURCE faders and pan explicitly;
later FINAL settings cannot redefine SOURCE. Any DAW comparison must match its
pan law, stereo interpretation, timing, gains and bypass state.

**Change gate:** retain `f64` addition if the results meet the declared bounds.
Fix confirmed arithmetic, routing, alignment or conversion errors and add a minimal
regression for each. Stop dependent musical comparisons until the technical path
is understood. Pairwise or compensated production summation needs demonstrated
benefit; oversampling a linear summer is not a required change.

If a confirmed defect requires different historical output, version the corrected
behaviour and document the exact intentional difference. Retain a legacy replay
path or a verified baseline executable/source snapshot for historical reproduction.
Do not weaken a test and claim the changed waveform is byte-identical.

## Phase 2 Explain the ensemble and export costs

Reuse the production strips, effects and existing observers. Extend their observations
where necessary instead of introducing a competing renderer. Every observation must
name its stage and settings/source identities.

### Required observations

- Source, post-filter, post-compressor before makeup, post-makeup/fader/pan,
  direct ensemble, individual wet returns, combined ensemble, master stages and PCM.
- Sample and true peaks with channel and frame/time; RMS, integrated and short-term
  loudness where valid; crest distributions and fixed-window band energies.
- Compressor/master gain-reduction envelopes or bounded summaries tied to musical
  events, including quiet/strong passages and recoveries. Maxima alone are insufficient.
- Coherent group powers and signed cross terms for known related inputs. Measure
  stereo spectra without folding opposite-polarity content out of the analysis.
  Report mono compatibility separately with an explicit fold-down gain convention.
- Export gain, its controlling peak and finished-file measurements. Distinguish
  static finalization from time-varying gain reduction and compressor makeup.

For each important peak, report signed source/group/return contributions at a
specified additive stage. If attributing a peak after a common linear filter,
propagate the contributions through that filter consistently and reconcile their
sum. A leave-one-source-out render through nonlinear master processing is a
counterfactual, because it can change the gain envelope; label it accordingly.
Do not present source powers as additive percentages of correlated mix power.

### Diagnostic experiments

1. **Peak cost:** repeat the known mechanism on synthetic audio: increase one brief
   transient, retain other sources, then compare the bus and independently finalized
   output. On real material, observe actual controlling peaks and untouched-source
   level changes. Do not boost drums merely to recreate an old rejected mix.
2. **Master gain cancellation:** compare two fixed master gains with downstream
   processing linear, first before export and then after legacy independent peak
   normalization. Verify that final gains cancel as predicted. Repeat with active
   master dynamics to demonstrate the limit of that statement.
3. **Processing attribution:** use stage taps and a few explicitly labelled bypass
   probes to isolate filters, compression, makeup and returns. Match gain for the
   diagnostic question and retain the actual unmatched export as separate evidence.
   A bypass probe is not automatically a musical candidate or a replacement FINAL.
4. **Microphone relationships:** examine known close/overhead/room or DI/amp groups
   in common active windows. Report band-specific cross-spectrum phase/coherence,
   plausible delay ranges and ambiguous cases. Broadband correlation alone cannot
   prove useful alignment or preferred tone.
5. **FX and stereo:** confirm direct-path retention, return levels, mono behaviour,
   intentional predelay and any processing latency. A new latency-compensating
   mechanism must preserve artistic delays and original source timestamps.

**Exit:** each concern has an observed mechanism, confidence, affected regions and
proposed experiment, or an explicit no-change finding. Reconcile the current zero
master-reduction reports before blaming master dynamics. Do not reinstate historical
Dark Ride processing or old Complainiacs faders to explain current files.

## Phase 3 Implement delivery and comparison controls

### Preserve legacy rendering and identities

Introduce a **versioned render policy alongside frozen session settings**. Prefer a
sidecar policy so new delivery choices do not rewrite source identities or invalidate
existing settings hashes. Resolve it once at render startup and save its contents,
hash and effective processing settings in the report.

Keep the current `render` invocation and version 1 sessions on an explicit legacy
path that reproduces their processing, sample ceiling, export gains and file samples.
Provide an explicit new policy-aware rendering entry point; finalize its CLI spelling
with the existing parser and document it before use. Do not present a proposed command
as available before implementation.

The new policy must independently express:

| Control | Required meaning |
|---|---|
| Final sample limiter | Disabled or explicitly enabled with its own threshold/release; comparison choice cannot toggle it |
| Existing FX master processing | Retain the frozen rack configuration and report its action separately |
| Delivery gain | Fixed gain for controlled comparisons, or independent whole-program peak finalization |
| Peak basis and ceiling | Explicit sample peak in dBFS or validated true peak in dBTP |
| Additional comparison copies | None or explicitly requested static loudness matching; reuse the same processed bus |
| Metering and quantization | Named algorithm/version, sample format, dither choice and measured output stage |

Changing only the comparison-copy option must leave the processed float bus and
primary delivery unchanged. Changing a delivery ceiling must leave channel/master
DSP unchanged. Test all combinations with active and inactive final/FX limiters,
using synthetic audio; the current zero-reduction music cannot expose this coupling.

Audit every consumer of `output_mode`, `ceiling_db`, session hashes and render
reports. In particular, preserve the legacy contracts in `balance::validate_baseline`,
source preparation, tone/bass/drum reviews, ambience/matching persistence and
`scripts/listening_checkpoint.py`. New policy-aware checkpoints must declare their
contract and verify the policy, report and final PCM together. They must not accept
an old sample-peak report as proof of true-peak compliance. Preserve refusal of stale,
missing or contradictory evidence and historical byte-exact SOURCE identities.

### Add true peak measurement and static finalization

Implement or integrate an offline estimator following ITU-R BS.1770 Annex 2.
Choose the algorithm/dependency from correctness, licensing and the pinned toolchain;
keep the project independently buildable and record attribution. Validate every
supported rate, stereo sides, filter state across blocks, start/end boundaries,
short files, silence and nonfinite rejection. Process interpolation tails internally
without changing the exported file's timeline or adding unexplained audio frames.
Stream with bounded memory and record elapsed time and memory use on the complete
examples. The offline meter must not alter the samples it observes.

Use an effective interpolation rate of at least 192 kHz and a higher-resolution
independent reference. Cover phased high-frequency tones, impulses, multitone sums,
near-Nyquist stress cases and known intersample overs. Distinguish reference accuracy
from display rounding. Derive acceptance limits from the chosen standard's applicable
conditions and reference error before running the suite. Retain a small discriminating
synthetic set in normal tests; extensive phase/frequency/rate sweeps are opt-in.

For new review/delivery copies, start with an explicit **minus 1 dBTP ceiling** and
independent static peak finalization, without a loudness target. This is a delivery
proposal, not a musical optimum. Preserve legacy minus 0.01 dBFS renders and their
contracts. Any boost limit for nearly silent material must be explicit in the policy;
digital silence stays silent with zero export gain.

Measure the actual retained float bus used by the exporter, apply one gain, quantize
and remeasure the final PCM. Confirm the new ceiling with the independent meter too.
Budget interpolation and quantization uncertainty before export; do not tolerate an
over-ceiling result simply because it fits the meter's nominal tolerance. If needed,
allow one additional computed attenuation and reverify; unresolved disagreement fails
completion. Record target, measured peak, margin, algorithm and every gain step.

Static attenuation should satisfy this delivery requirement without reshaping
transients. A lookahead true-peak limiter is conditional on a separately justified
loudness/dynamics objective. If needed, compare attack/release distortion, transient
loss, stereo linking, intermodulation, latency and bypass alignment. Keep live/monitor
latency and PA protection out of any offline acceptance claim.

### Check precision and final quantization

Quantify the float32 intermediate and undithered 24-bit conversion error using phase
1 fixtures, quiet fades and null residuals. These are low-priority hypotheses for
the reported broad tonal concerns. Preserve legacy samples. If adopting TPDF dither
for the new policy, apply it once at final integer conversion, record its settings,
use reproducible test seeds and verify low-level distortion/noise and peak margin.
Do not add dither at every channel or sum, or double-dither exact excerpt copies.

**Exit:** validated policy-aware rendering, explicit legacy replay, verified final
true peaks and tests showing that comparison gain cannot alter master DSP.

## Phase 4 Run bounded musical experiments

Only concerns supported by phase 2 enter this phase. Record a compact experiment
card before tuning: observation, source identity, mechanism, expected benefit,
musical cost, baseline, exact parameter candidates, training/held-out regions,
protection limits and acceptance question.

For each pilot song, allow the baseline plus **at most three musical candidates in
total**, shared across the applicable families below. This is a proposed study budget,
not an audibility threshold. Freeze it in phase 0. Diagnostic stage measurements do
not consume candidate slots, but any proposal offered for selection does. Do not form
a Cartesian product of EQ, phase, fader and dynamics alternatives.

| Family | Experiment and resulting change gate |
|---|---|
| Filter and coherent-source interaction | Compare the existing treatment with one supported bypass or group-consistent correction; retain it only if the identified issue improves without unacceptable body, articulation or mono costs |
| Compression and makeup | Isolate dynamics from makeup; examine attack/body/sustain and quiet/strong events; modify the responsible stage rather than restoring RMS blindly |
| Fader and peak ownership | Test a bounded musical level change only for a stated balance need; report both within-mix relationships and its global export cost |
| Microphone polarity and delay | Test only identified related captures with repeatable evidence; preserve an unchanged option, limit the delay search in advance and assess all affected kit/source events |
| Stereo interpretation | Correct verified misrouting or wrong source metadata; alternate pan laws or stereo placement are explicit artistic candidates and cannot silently reinterpret old sessions |
| FX masking and latency | Compare only a supported return/routing change; retain requested spatial production and distinguish processing latency from intentional effect delay |
| Bus dynamics or coloration | Run only for a specific requested sonic objective after the transparent path passes; include a gain-controlled bypass, and quantify added distortion, transient change and any aliasing |

For delay/polarity investigations, reuse the owning module or established external
analysis where available. Research evidence in this run may specify requirements;
production measurement/alignment algorithm work belongs in SHR PA and requires
authorization there. Define any future GigPies channel control, latency declaration
and integration interface before adding it. If that dependency is unavailable,
finish independent phases and record the precise deferred owner task.

Continuous source motion, narrow-band ambiguity, bleed or conflicting instrument
events can invalidate a single delay. Withhold correction in those cases. A phase
change can increase peaks even when it restores body, so re-evaluate delivery gain.
Preserve stereo pairs and intentional room depth. Never align unrelated performances
or automatically assign identity from a filename.

Select at most one candidate using training evidence, then freeze it before held-out
checks. Held-out failure retains the baseline and ends that study; it cannot trigger
retuning or wider guards. Further work requires a separately declared hypothesis and
evaluation design. Numerical success qualifies a listening candidate, not a preferred
mix. Unsupported masking targets, equal instrument levels and universal spectral
ratios cannot supply missing musical intent.

## Phase 5 Verify outputs and prepare review

After engineering checks pass, render new complete outputs from the preserved
originals and frozen settings into fresh directories. Reconstruct the required
legacy comparison buses once and check against the retained retirement hashes.
Reuse each new bus for delivery-only views; preserve the newly selected files.

For every retained candidate and all six regression examples, verify full duration,
source offsets, channel assignments, expected FX tails, finite samples, sample and
true peaks, finished loudness, processing action and source/file identities. A change
to shared routing/rendering must also reproduce the legacy path on all six examples.
Prioritize compact metrics and hashes over duplicate unchanged audio.

Prepare these labelled comparison views from the same frozen buses where applicable:

1. Reconstructed historical SOURCE and selected FINAL, verified against retained
   identities and labelled with their original processing/gain contract.
2. Baseline/candidate at one common fixed attenuation sufficient for both true peaks,
   to expose processing and relative level changes without independent finalization.
3. New independent true-peak deliveries, to assess the proposed finished output.
4. Static loudness-matched copy controls are tested with synthetic sources only.
   Real-audio equal-LUFS listening copies require a new explicit user request.

Retain the established passages: Complainiacs 24 to 36 and 90 to 102 seconds,
Dark Ride 48 to 60, Rainfall 240 to 252, Wild & Co 168 to 180,
Catbite 156 to 168 and Phoenix 36 to 48. Add only the predeclared diagnostic peak or
failure passage when needed. Exact clips inherit their parent's gain and samples;
label transformed comparison copies separately. Limit the initial listening queue
to the comparisons needed to answer the selected hypotheses.

Record preference as better, worse, no reliable difference or not reviewed, with the
specific passage and concern. Include body, transient impact, bass/kit support, vocal
clarity, stereo/mono behaviour and FX depth where relevant. If blind ordering is used,
retain its mapping and repeatability. Listener acceptance remains pending until the
user supplies it. Playback follows the separately authorized review session.

## Validation and test classification

The executing agent selects tests from changed behaviour. No user test selection is
needed. During implementation run the smallest relevant synthetic regressions.
Rendering, shared policy/schema, routing and persistence changes require the complete
normal suite before handoff. Use the normal target directory and consistent flags.

```sh
rustc -vV
CARGO_INCREMENTAL=0 cargo fmt --all -- --check
CARGO_INCREMENTAL=0 cargo check --locked
CARGO_INCREMENTAL=0 cargo clippy --locked --all-targets -- -D warnings
CARGO_INCREMENTAL=0 cargo test --locked --all-targets
python3 -m unittest discover -s scripts -p 'test_*.py'
CARGO_INCREMENTAL=0 cargo build --locked --release
```

Keep short arithmetic/routing, true-peak counterexample, mode-independence, legacy
replay, identity, failed-output recovery and policy-validation regressions in the
default suite. Reuse current tests and avoid duplicate coverage of implementation
details. Never put private recordings or downloaded media in default tests or CI.

Historical media tests, complete study renders, exhaustive interpolation sweeps,
long listening matrices and performance benchmarks remain opt-in. Run those directly
protecting a changed assumption and document their exact command. If a slow one-time
study is encountered in the default suite within this scope, retain its concise
evidence and move it behind an explicit opt-in without removing production coverage.

After phase 5 has produced a verified file, scan it without playback or another WAV:

```sh
ffmpeg -hide_banner -nostdin -nostats -threads 1 -filter_threads 1 \
  -i NEW_VERIFIED_RENDER/processed.wav \
  -af 'ebur128=peak=sample+true:framelog=verbose' -f null -
```

This command is a diagnostic cross-check, not certification of the new estimator.
Capture tool versions and sufficient numerical precision in the execution evidence.

## Deliverables and completion

Keep the following concise artifacts in the ignored run directory:

- Baseline identity manifest and frozen experiment/region policy.
- Numerical null/routing results and true-peak validation summary.
- Stage/peak findings, experiment cards, rejected outcomes and selection reasons.
- New delivery policy, resolved settings and compatibility/migration evidence.
- Complete export verification, exact clip manifest and a small listening index.
- Final results with implemented, offline-validated, listening-pending and
  hardware-unverified status stated separately.

Update the focused public owner documents, documentation index and status with
behaviour, reproducible commands and limitations. Keep private settings, source
manifests, recordings and one-off evidence out of published content. Preserve earlier drafts
under `docs/archive` if a substantive rewrite replaces useful historical context.

Before any authorized publication, inspect live Git state and the complete staged
diff, preserve unrelated work, check/enable the versioned hooks, and run the guard
against the complete index. Do not stage the whole dirty repository.

At completion remove only known disposable artifacts from this run after checking
active processes, open files and locks. Preserve selected deliverables, source data,
unique evidence and newly selected mixes. Earlier renders were explicitly retired;
do not interpret their absence as accidental loss. Report recovered space and any
unusually large output retained. Do not rebuild solely to verify cleanup.

### Completion checklist

- [ ] Baseline identities and applicable current contracts re-established.
- [ ] Neutral sum and routing verified independently; confirmed defects resolved.
- [ ] Stage and peak observations explain each pursued concern or record uncertainty.
- [ ] True-peak estimator and new delivery policy validated on final PCM.
- [ ] Comparison controls independent of DSP; legacy output remains reproducible.
- [ ] Every musical hypothesis accepted for review, rejected or withheld with evidence.
- [ ] Required normal tests and relevant opt-in experiments passed; skipped classes listed.
- [ ] All six examples checked, historical selections reproducible and new review material verified.
- [ ] Listening preference recorded or explicitly pending; hardware acceptance separate.
- [ ] Owner documentation and concise handoff complete; disposable output cleaned.

The engineering work can be complete while musical acceptance remains pending.
An unavailable sibling capability or unresolved technical failure must be named as
deferred work; do not mark that item implemented or hide it in a general success claim.

## Research references

- [ITU-R BS.1770-5](https://www.itu.int/rec/R-REC-BS.1770-5-202311-I/en): loudness measurement and Annex 2 true-peak estimation; measurement does not determine a preferred mix.
- [EBU R128](https://tech.ebu.ch/docs/r/r128.pdf): distinct loudness and maximum true-peak constraints; minus 1 dBTP is a delivery starting point here, with no adopted broadcast LUFS target.
- [Giannoulis, Massberg and Reiss 2012](https://joshreiss.github.io/documents/2012/GiannoulisMassbergReiss-dynamicrangecompression-JAES2012.pdf): compressor topology, detector smoothing and distortion; matching parameter labels does not establish identical dynamics.
- [Clifford and Reiss 2013](https://joshreiss.github.io/documents/2013/clifford%20reiss%20-%20JAES.pdf): musical-source delay estimation and comb filtering; estimator/window limitations matter to alignment.
- [Hafezi and Reiss 2015](https://joshreiss.github.io/documents/2015/Hafezi%20Reiss%20-%202015.pdf): multitrack masking reduction and limits of objective masking measures as predictors of preference.
- [W3C Audio EQ Cookbook](https://www.w3.org/TR/audio-eq-cookbook/): the biquad transfer functions underlying magnitude and phase behaviour.
- [FFmpeg ebur128 documentation](https://ffmpeg.org/ffmpeg-filters.html#ebur128): independent diagnostic loudness and oversampled peak measurement.

These sources informed the 2026-10-03 research. Recheck the chosen implementation's
dependencies, standard revision and acceptance conditions when executing the plan.
