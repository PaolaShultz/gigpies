# Source preservation and evidence for intervention

This is the current offline decision contract. It supersedes the universal 90 Hz
channel / 40 Hz master HPF requirement in the earlier experiment documents.
Historical settings and results remain evidence of those experiments.

**Subsequent artistic clarification:** the listener requests finished mixes with
GigPies-generated effects. The [artistic FX pass](../../guides/ARTISTIC_FX.md) retains the selected
channel sound while adding explicitly requested spatial production. The effect-free
six-example checkpoint below is now its baseline, not fulfilment of that later
finished-mix requirement. Source preservation does not forbid requested effects.

## What we are deciding

A successful recording can already contain the musician's intended sound. A good
source and a good ensemble are valid finished choices. A role identifies a source's
job; it does not establish a defect, processing requirement or preferred balance.
The selected baseline must compete with every proposed intervention.

Keep five questions separate:

| Question | Useful evidence | Limit |
|---|---|---|
| Where did this sound come from? | Original files and notices, acquisition hashes, session notes, musician/producer accounts, verified input assignments | File integrity and a `raw` label do not certify untreated capture. A waveform cannot reconstruct its prior chain. |
| Is there a technical problem? | Invalid/nonfinite data, repeated PCM contact, known routing error, contextual noise/overload observations, actual processor envelopes | Clipping below full scale, intentional distortion, ringing versus a played note, and recoverability often remain unknown. Report contact without claiming repair. |
| Is a musical change wanted? | Musician intent, listener comparison, attack/body/sustain and articulation in the song, arrangement and section transitions | Crest, spectra, occupancy and whole-song medians cannot establish preferred tone or balance. |
| What protection is required? | File/schema/routing integrity and export limits; separately, a live system's verified operating limits | Offline HPFs do not establish feedback stability, loudspeaker protection or safe acoustic level. |
| Could a processor help? | A specific problem, a plausible mechanism, a bounded proposal, actual DSP comparison and contextual listening | Technical success is eligibility for listening, not musical improvement. Effects are optional artistic decisions. |

The performer, listener and operator supply intention and preference. The software
owns reproducible measurements, bounded changes, provenance and failure reporting.
Unknown printed EQ/compression is recorded as unknown. Low crest or a shaped spectrum
can be deliberate sound design; neither enables an “already processed” detector.
Already designed sounds may still need a supported correction in a new ensemble.
Earlier user-prescribed knob moves and numerical mixing preferences remain withdrawn.
Their historical provenance is retained without treating them as current objectives.

## Required reasoning for a proposed correction

Record this before selecting parameters:

1. **Problem and confidence:** what was observed, in which source/section and by whom;
   distinguish supplied listening evidence, reported provenance, measured behavior
   and inference. A threshold exceedance is relative to its declared policy.
2. **Mechanism and expected benefit:** why this processor or fader could address that
   observation. Do not prescribe compression from an instrument label or reverb from
   an assumed dry capture.
3. **Musical cost:** attack, sustain, body, articulation, bass support, bleed, stereo
   interaction, section development and export-headroom consequences as applicable.
4. **Baseline victory:** insufficient evidence, no requested change, no material
   supported benefit, a protection failure or adverse listening retains the baseline.
   Equal scores also retain it. Preserve the previous selection for recovery.
5. **Bounded test:** freeze a small candidate budget and sections, compare processing
   at fixed faders, assess artistic balance separately, then check the combined
   selection. Holdout can veto; it cannot choose a retry. A failed correction does
   not justify widening a guard or silently changing the baseline.

Measure raw events/sections once and reuse those anchors. Include quiet/strong and
sparse/dense passages where relevant. Follow HPF → EQ → compression → makeup →
fader/pan → coherent ensemble → master → constant export gain. Independent powers
omit phase interaction; evaluate actual sums. For sampled drums, energy rises remain
proxies for musical events. For ambiguous microphone mixtures, abstain from claiming
source separation. Do not optimize a track while ignoring its place in the song.

## Failures found and changes implemented

- **The unchanged source was ineligible.** `balance::validate_baseline`, also used
  by tone/bass/drum reviews, demanded exact 90/40 Hz filters. SOURCE could lose before
  evaluation. It now validates prepared/unmatched settings, unity trims, DI-only
  routing where asserted and the −0.01 dBFS export ceiling; HPFs may be zero or any
  schema-valid explicit setting. Validation never inserts or removes them.
- **Analyzers assumed the old contract.** Bass/drum master measurements always ran
  40 Hz regardless of settings. They now use `master_hpf_hz`. A zero-frequency
  `Biquad::highpass` is an exact identity filter, avoiding cancelling poles at DC.
  Positive-frequency coefficients and existing saved rendering are unchanged.
- **Missing intent supplied taste.** Default balance constructed numerical role
  relationships; missing tone intent became `balanced`. Default balance now has no
  relationships and performs zero search evaluations. Missing tone intent is
  `unknown`, its target is null, and it proposes no EQ. Explicit old intents,
  profiles and policy files remain readable. Historical balance hypotheses are
  available through `Policy::trial_for_session`, deliberately named as a trial.
- **Spectral/FX review could act without a diagnosis.** Default `finish` now observes:
  EQ and return-movement budgets are zero, and the master-reduction budget is null.
  Explicit policies can enable bounded experiments. Existing musical master dynamics
  are preserved unless a supplied budget requests review. Export protection remains.
- **Preservation required manual parameter surgery.** `preserve-source` now prepares
  the supplied source sum as a legitimate FINAL and writes a decision record.
  It clears channel HPF/EQ, bypasses compression, zeros makeup and removes optional
  master HPF/FX. It keeps files, roles, groups, faders, pan, rate and timing; it rejects
  incompatible trim/export/routing settings. It never infers DI from unknown bass.
- **EQ capacity could block abstention.** Tone review now measures and preserves an
  already suitable or unspecified source even with all EQ slots occupied. An actual
  over-capacity proposal is rejected without deleting the musician's existing EQ.

The renderer applies frozen settings; it does not reinterpret old mixes under new
defaults. `preset`, `unity-pass`, `fx-preset` and manufacturer mapping remain explicit
historical experiment entry points. `unity-pass` still imposes its documented role
compression, 90/40 Hz filters, measured makeup and optional FX calibration; it is
**not source preservation**. Its measured RMS compensation can change attack/balance
and is not a neutral musical requirement. Do not use it automatically on ingestion.
Causal RMS trim calibration is also a separate experiment, not offline source quality.

No new target ratio replaces the old defaults. Explicit tone, bass, dynamics and
balance objectives still produce proposals when supported by their evidence gates.
Their existing numerical safety/recovery/transient guards remain unchanged. Passing
those guards cannot fill in missing musical intent or establish listener acceptance.

## Foundational filter decision

The old filters aimed to restrict low-frequency content, leave kick/DI space and
provide a consistent processed baseline. They originated in a unity-source experiment,
not a demonstrated requirement for every offline file. The schema and renderer already
supported bypass. A 90 Hz filter can remove useful tom, guitar, keyboard or vocal body;
a 40 Hz filter can attenuate fundamentals and change coherent low-frequency phase.
Those costs matter even when whole-band RMS changes little.

Replacement: select a filter for documented unwanted content or an explicit artistic
choice, test it in context and keep bypass eligible. Frequency content below a corner
is not itself unwanted. The Dark Ride audit attributes only about 0.32 dB of excerpt
bass RMS loss to the master HPF; it does not blame filters for all lost power. The
larger inherited EQ/makeup/export interaction remains the established finding.

This revision is an offline admissibility change, made before new candidate evaluation.
It does not increase correction budgets, remove PCM-contact reporting, relax existing
crest guards or silently waive a failed candidate. SOURCE preservation does not need
to “improve” on the rejected FINAL's crest: the listener-preferred SOURCE is the
unchanged musical reference. Old crest failures against other baselines remain failures.

On stage, if the instrument and captured signal already sound good, keep that tone
and adjust balance only for a stated ensemble need. A real input overload calls for
checking the stage that overloads; EQ cannot reconstruct missing detail. Monitor/PA
feedback control and system-specific protection remain independent, continuously
required functions in the intended live architecture. This offline work neither
implements nor validates them. No hardware claim follows from filter bypass offline.

## Use and recovery

```sh
CARGO_INCREMENTAL=0 cargo +1.97.1 build --locked --release
# Supply the pinned, initial SOURCE routing with prepared unity input trims,
# unity master, verified selected paths and unmatched −0.01 dBFS export settings.
target/release/gigpies preserve-source INITIAL_SOURCE.json NEW_SELECTION_DIRECTORY
target/release/gigpies render NEW_SELECTION_DIRECTORY/settings.json SOURCE_DIRECTORY NEW_RENDER_DIRECTORY
# No supplied balance policy: measure and preserve artistic faders.
target/release/gigpies balance-analyze NEW_SELECTION_DIRECTORY/settings.json SOURCE_DIRECTORY NEW_ANALYSIS_DIRECTORY
```

This command does not know which later faders were the original SOURCE faders.
The listening checkpoint's existing pinned SOURCE settings/WAV hashes remain
mandatory. Select original routing deliberately; never reset a liked mix merely
because preservation is now available. There is no implicit reset inside `render`.

All outputs require fresh destinations. Failed preparation has no ready checkpoint;
recover with the frozen inputs and a new directory. Playback remains a separate
explicit request. Original sources and previous settings/exports remain available.
An unchanged source export may be byte-identical to FINAL; label that as the outcome
instead of inventing a second mix to make the comparison look productive.

## Validation scope

Normal synthetic tests cover exact SOURCE/FINAL PCM and float equality with mixed
mono/stereo routing, bass/drum measurement agreement with production renders at
0/40/65 Hz, preserved unknown source identity, nonunity/routing/nonfinite/export
refusal, missing tone/balance intent, no default master/FX shaping, and a known
injected compressor fault that still receives bounded actual-DSP relief. Existing
BWF, clipping/contact, transient, linked compression, fader/makeup, recovery and
held-out tests remain in the complete normal suite.

Private Dark Ride probes and selected complete export are explicit opt-in evidence.
Old auditions, exhaustive searches and unrelated snare-identifiability experiments
are not rerun. Verified results of the bounded pass are recorded below.
No listening or hardware acceptance is implied.

## Dark Ride: frozen reassessment, 2026-10-02

**Selected: SOURCE unchanged as FINAL.** This is a preservation decision supported
by the listener's preference, with complete export verification; it is not a new
claim of improved source quality. The other five liked examples keep their mixes.
The current complete set is `artifacts/automix/source-preservation-v1/LISTEN.md`.

The original audit in [MULTITRACKS.md](MULTITRACKS.md#dark-ride-source-and-processing-audit--2026-10-02)
remains intact: original archive/acquisition hashes match; sampled drums and designed
sounds are established by producer accounts; the exact printed chains are partly
unknown. SOURCE was never an earlier GigPies render fed back as input. Its 48–60 s
stage attribution remains valid, including approximately 9.60 dB kick reduction,
inactive guitar compression, active EQ/+2.5 dB makeup and 4.15 dB more guitar relative
to bass in FINAL. Further work addressed section consistency and preservation, not
another provenance investigation.

### Budget, sections and selection

`plan.json` was written before evaluating candidates. Budget: **two processing
proposals, zero artistic fader proposals, one combined selection, zero held-out
retries**. The source and current FINAL are explicit comparators. Training spans
are **48–60, 96–108, 192–204 s**; held-out spans are **60–72, 108–120, 204–216 s**.
Full selected-output verification also covers the ending through **300.365011 s**.
These are within-song diagnostics on previously studied material, not a blind study.
Production filters/compressors run continuously from sample zero. No individual
source is trimmed, normalized, resampled, aligned again or polarity-flipped.

| Selection | Evidence and outcome |
|---|---|
| SOURCE / P1 preservation | Removes optional inherited shaping at fixed initial faders/pan. Expected benefit is retaining the preferred supplied sound. Risk is retaining any source limitation. All training/held-out group, stereo and event metrics remain identical to SOURCE. Selected before held-out inspection. |
| P2 HPF-only control | Retains the inherited 90/40 Hz filters, with EQ/compression/makeup bypassed. No demonstrated unwanted low-frequency component justifies this blanket treatment. Training other-drums RMS falls 1.05–1.58 dB, guitar 0.24–0.54 dB and bass 0.13–0.32 dB before export. These losses do not prove audible damage. Reject for insufficient benefit; no held-out retry or complete candidate export. |
| Unchanged current FINAL | Eligible comparator, rejected because it conflicts with supplied listening evidence and retains unsupported shaping. Its settings and complete exports remain intact. Both diagnostic float buses reproduce every stored sample from zero through 216 s. |
| Artistic balance | Keep every existing unity fader and pan. No evidence supports another move. No ratio target supplies one. |
| Combined result | P1 at unchanged artistic balance. No extra makeup, EQ, compressor relief, reverb or master processing is stacked. |

The prior source and current FINAL have the same initial faders and pan. Therefore
preservation restores the preferred ensemble as well as individual sources. Both
complementary bass files remain generic `other`: capture identity is still unknown.
The supplied SnareFX exclusion is retained. Guitar capture/performance relationships
are not reclassified. Verified DI-only choices in the other examples remain intact.

### Contextual evidence

Current FINAL's pre-export guitar-minus-bass change is **4.00–4.51 dB** in the
three training spans and **4.22–4.57 dB** in held-out spans. This consistency supports
the attribution of a substantial relative shift, without turning it into a desired
ratio. In 48–60 s, current processing raises coherent guitar RMS 3.826 dB before
export and reduces bass 0.323 dB. That plus the common export attenuation reproduces
the earlier finished-contribution finding. It does not diagnose the guitar source.

The fixed raw-event observer finds **112/167 kick** and **47/48 snare** training /
held-out events. These are 10 ms energy-rise anchors, not musical transcription;
rapid events can overlap a measured decay. Current FINAL's held-out median kick
attack/body/decay contributions change approximately **+0.57/−0.28/+0.98 dB** before
export; snare changes **+1.76/+5.65/+6.64 dB**. P1 changes every corresponding value
by **0 dB**. The snare tail rises far more than its attack under the current chain;
this is measurable envelope redistribution, not proof of a printed source compressor
or the sole cause of the listener's objection. Makeup cannot undo envelope changes.

Coherent sums also preserve the actual stereo consequences. SOURCE/current FINAL
L/R correlations are about **0.652/0.741**, **0.756/0.814**, **0.900/0.871** across the
held-out spans. Direction varies with arrangement; no correlation target chooses a
mix. P1 preserves SOURCE exactly. Group powers, signed interaction, low-band support,
section development and raw-event context remain in the private summaries. Numerical
spectral occupancy never selects a tone or identifies a preferred source.

### Protection and remaining failures

No new protection failure remains for the selected output relative to its unchanged
SOURCE baseline. Full file identity protects its attack, sustain, stereo, timing,
section transitions and ending more strongly than a median tolerance would. The
regular source/schema/finite-sample, routing and export checks also pass. There is
no compressor, limiter or FX action in the selection.

There are **205 snare and one Tom3 PCM full-scale contacts** in the originals.
They are isolated single samples; the largest fraction in an 8192-sample window
is 0.0488%, below the existing 0.1% repeated-contact rule. No threshold changed.
These observations remain source limitations; they neither certify unclipped capture
nor justify a reconstruction claim. The unknown bass identity keeps specialized
DI-based drum/bass correction inapplicable. It was not relabelled to enter that path.

Unselected current FINAL retains its measured **10.56 dB** full-song kick reduction.
That is a processing-budget concern, not proof of live-system danger or universally
excessive musical compression. Existing rejected crest/tone/balance candidates and
remaining policy failures in the other songs remain recorded in their owning reports.
Default abstention does not retroactively make those old policies pass. No candidate
was admitted by widening a guard.

### Export consequences and checkpoint

The selected full export has **13,246,097 frames at 44.1 kHz**, stereo 24-bit PCM.
Its PCM file and float bus are byte-identical to the established complete SOURCE.
Both output files were independently finalized by production export to **−0.01 dBFS
sample peak**. No source normalization, equal-LUFS target, matched copy or true-peak
compliance claim applies.

| Dark Ride complete export | Gain | Finished integrated loudness |
|---|---:|---:|
| SOURCE / selected FINAL | −2.857644 dB | −14.7 LUFS |
| Previous FINAL, retained comparator | approximately −6.279 dB | −15.5 LUFS |

Selecting preservation recovers **3.4215 dB of export gain**. Relative to previous
FINAL in the established 48–60 s passage, bass contribution rises about **3.74 dB**,
other drums **4.28 dB**, kick **2.09 dB**, while guitars fall about **0.40 dB**.
The removed channel processing determines the relative changes; export gain scales
all contributions together. The selection does not seek equal loudness.

The complete set retains the established main excerpts: Complainiacs **24–36 s**,
Dark Ride **48–60 s**, Rainfall **240–252 s**, Wild & Co **168–180 s**, Catbite
**156–168 s**, Phoenix **36–48 s**. Complainiacs retains **90–102 s** for its existing
late-event/decay tradeoff. Dark Ride needs no second passage to expose a new tradeoff:
selected FINAL and SOURCE are identical throughout. The index contains only
SOURCE → selected FINAL, clearly labelling that identity. Every clip retains its
parent's exact PCM, gain and sample position.

### Assumptions retained, overturned and unresolved

- **Retained:** originals and notices, pinned SOURCE identity, unity trims, timing,
  stereo, verified routing, explicit makeup independent of faders, bounded proposals,
  actual production DSP, fixed evaluation sections, independent peak export and
  separate listener acceptance.
- **Overturned:** universal offline 90/40 Hz filtering; implied `balanced` tone;
  automatic role balance targets; default spectral/FX adjustment; the notion that
  adding processing is progress or that a manufacturer starting point proves fitness.
- **Unresolved:** exact prior processing of each Dark Ride file, bass capture identity,
  perceptual benefit of any future source-specific filter or spatial effect, live PA
  behavior, and acceptance of the prepared checkpoint. Sampled/designed sounds do
  not establish that every track is already compressed/EQ'd, and no reverb need follows.

Remaining listening questions are whether the preserved full Dark Ride arrangement
holds the power/body the listener values, and whether any specific passage calls
for a clearly identified intervention. Spatial effects remain an optional future
listening question. The five positive examples are retained; this task supplies no
reason to remake them.

### Completed validation and retention

**112 normal Rust tests and 29 Python tests pass.** Formatting, Clippy with warnings
denied, the locked Rust 1.97.1 release build and whitespace checks pass. Three
historical private-media tests remain intentionally ignored. Current production
probes, the full selected render and five other complete rerenders ran. Each of the
five reproduces both prior PCM exports and both float buses byte for byte, with all
previous measurement fields equal. All six SOURCE/FINAL exports and fourteen exact
clips are verified. Finished PCM loudness was measured separately for reporting.

The offline workflow checks support a clear selection, retained intent/routing,
inspectable decision records and recovery through frozen inputs and fresh output
directories. Admission failures leave no output directory; repeated preparation
refuses to overwrite an existing selection. Existing render/recovery tests cover
the shared paths. No performer interface or accessibility/hardware validation is
claimed; those interfaces remain planned.

The initial 47 uncommitted files were inventoried, hashed and archived before work.
Their existing changes are preserved; the task's additions are separately reviewable.
Cleanup checked open files and output dependencies, then recovered **2.140 GiB** from
verified task-created duplicates and lossless CSV compression. After independent
rendering and equality checks, Dark Ride's new export paths reuse the retained
SOURCE files through symlinks. The previous FINAL remains available as evidence.
The task retains about **70 MiB**, including the complete clip set, decision evidence
and reproducible probes; the normal build directory is about **724 MiB**, with
**43 GiB** free. Originals, notices and all current deliverables remain available.
No playback, host-audio change, commit or push occurred.
