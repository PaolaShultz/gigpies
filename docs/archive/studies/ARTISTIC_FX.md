# Expert selection of artistic effects

**Audio availability, 2026-10-03:** the [summing-engine work](../plans/SUMMING_PLAN.md)
produced six new complete mixes with the frozen selected GigPies effects and
independently verified true-peak headroom. Use
`artifacts/automix/summing-study/2026-10-03-engine/LISTEN.md`.
Earlier generated audio was retired; the results below retain its historical
selection evidence. Original recordings, frozen settings and reports remain unchanged.

The listener clarified that the requested deliverables are finished musical mixes,
including GigPies-generated effects selected by the expert system. The previous
six-example checkpoint had no GigPies FX rack. Calling it complete described its
duration and export verification; it did not meet this artistic requirement.
The [FX provenance audit](FX_PASS.md#where-the-audible-ambience-comes-from) records
which older versions did contain generated effects.

Source preservation and spatial production are separate decisions. A good recorded
tone can remain intact while an explicit artistic request justifies added space.
An effect needs an intended musical benefit, not a fabricated technical defect.
This pass adds parallel effects to the selected channel tone and balance. It does
not restore the rejected Dark Ride presence EQ/compression/makeup chain.

## Implemented workflow

```sh
CARGO_INCREMENTAL=0 cargo +1.97.1 build --release --locked
target/release/gigpies ambience-plan BASELINE.json SOURCES NEW_PLAN POLICY.json
target/release/gigpies ambience-check NEW_PLAN SOURCES
target/release/gigpies render NEW_PLAN/settings.json SOURCES NEW_RENDER
```

`ambience-plan` is an explicit artistic step, separate from corrective `source-pass`
and the historical fixed `fx-preset`. It writes no audio. The production renderer
uses its frozen settings to make the complete mix. It creates `settings.json` only
after training and held-out checks pass; failed plans retain evidence and the
baseline, without publishing a ready selection. Use fresh output directories.

Policy supplies:

- A musical style and its basis; optional **verified** BPM. No genre/tempo detector
  runs. Without BPM, synchronized echoes are withheld.
- Named source groups with exact channel/file assignments and identity provenance.
  Every retained channel needs a disposition; grouping compatible FX sends does
  not assert one performance, source separation or microphone identity.
- Explicit reports of existing space, when available. This flag can temper added
  space; a waveform or filename never sets it automatically.
- An FX amount from zero to one, and separate frozen training/held-out passages.

For new plans, **all training passages must end before the first held-out passage
starts**. The two sets may meet at a boundary, but they cannot be interleaved.
The production DSP runs continuously from sample zero: an earlier held-out phrase
can otherwise affect later training through reverb/delay tails or channel state.
Nonoverlapping measurement windows alone do not prevent that dependency.

The program owns the effect choices and parameters after those inputs are supplied:

1. Measure the accepted direct signal using production channel DSP, timing, faders,
   pan and master HPF. Retain 20 ms energy windows for the training passages only.
   Activity and energy-rise observations are density proxies, not note transcription
   or an “already reverberant” diagnosis.
2. Select a spatial recipe from the instrument family and requested style. Busy
   signals get shorter, quieter space. Reported existing ambience also reduces the
   proposed decay and amount. Insufficient activity withholds a bus.
3. Convert the requested decay duration to **our engine's actual control**. The
   engine uses a unitless feedback parameter. Six bounded bisection steps compare
   filtered production-rack impulse decays; the report retains desired and measured
   duration. The measurement is remaining-energy decay to −60 dB after predelay
   within a six-second impulse capture, not a measured physical room RT60.
4. Run the proposed rack continuously from sample zero. Measure each wet return
   against its coherent feeding group during active training windows. Choose a
   bounded return gain for the recipe's explicit artistic amount. This calibrates
   the engine transfer level; it changes no input trim, compressor makeup or fader.
5. Freeze the resulting settings, then check the combined ensemble in training and
   held-out passages. Held-out failure vetoes the proposal and cannot trigger a
   parameter retry. Passing checks makes an audition eligible; it does not prove
   musical improvement.

The first catalogue has **ten style choices**: metal, rock, punk, ska, pop, acoustic,
folk, jazz, electronic and ambient. These are independently chosen musical starting
hypotheses, not a validated library covering every subgenre or performer.

| Source family | Initial spatial direction |
|---|---|
| Snare | Plate for amplified styles; chamber for acoustic/folk/jazz |
| Toms/percussion | Filtered chamber, restrained tail |
| Lead vocal | Plate for tight/amplified styles; hall for acoustic or spacious treatments; separate vocal predelay |
| Backing vocal | Chamber for shared depth |
| Rhythm guitar | Short room; plate for the ska profile |
| Lead guitar | Plate; restrained tempo echo when style and verified BPM support it |
| Acoustic guitar | Chamber |
| Keys | Chamber or ambient hall; subtle chorus for pop/electronic profiles |
| Winds/strings | Chamber for natural ensemble styles; hall otherwise |
| Kick/bass | Retain direct low-end support; no default spatial bus |
| Recorded ambience/unknown | Preserve the supplied path; no speculative additional send |
| Supplied FX return | Refuse it as a mix input under this workflow |

The exact current recipe values and their adaptations live in
`src/automix/ambience.rs`, with every selected value saved in the plan. The catalogue
can grow from instrument/genre listening evidence. Automatic compression, excitation,
master EQ or loudness maximization are not bundled into a spatial request.

The broad effect vocabulary follows established practice: the engine developer's
[reverb-type discussion](https://valhalladsp.com/2018/05/14/effect-o-pedia-reverb-types/)
describes differing spaces and warns that long tails blur fast notes. Our recipes
are hypotheses for our own engines; no third-party preset collection was imported
and no emulation equivalence is claimed.

## Preservation, protection and provenance

- Preserve files, source offsets, sample rates, stereo routing, artistic faders,
  channel EQ/compression/makeup and the accepted master HPF. Zero amount preserves
  the complete baseline settings; unknown identity can also yield no new bus.
- Only GigPies' reverb/chorus/delay engines generate new returns. Do not substitute
  supplied FX tracks, processed stems or finished reference mixes. Existing acoustic
  room captures are source recordings; printed effects inseparable from an instrument
  remain a stated limitation. The program cannot certify dry capture or remove such
  processing by inference. Supplied FX can be a later comparison, outside rendering.
- Keep the established vocal reverb minimum predelay of 30 ms. Return gain correction
  is bounded to ±18 dB before the requested amount, within the existing FX schema.
  No channel protection threshold is widened.
- The combined proposal must keep wet ensemble RMS at most −12 dB relative to the
  direct ensemble, RMS and peak rises at most 2 dB, and crest loss at most 3 dB in
  each evaluated passage. These are explicit experiment guards, not preferred mix
  targets or evidence of acoustic safety. Silent references cannot set a wet ratio.
- FX rack master drive is zero, master EQ/excitation are absent, and its existing
  high-headroom limiter threshold is 24 dBFS on the floating bus. Require zero
  reduction in plan observations and complete-render verification; final
  sample-peak export is separate. The plan observer now runs the actual FX master
  stage, so limiting cannot hide behind relative ensemble checks. Legacy plan
  reports did not measure that stage; their complete-render meters remain the
  evidence for their recorded zero reduction.
- Render six seconds of effect tail and inspect the ending. Each full export is
  independently finalized to −0.01 dBFS sample peak. No source normalization,
  equal-LUFS target or true-peak compliance claim applies.

The checkpoint writer now accepts a longer FINAL only with explicit `allow_fx_tail`,
matching source routing/offsets/lengths, consistent per-channel padding and production
FX metadata matching the added frames. The established SOURCE file/hash stays pinned;
no silent padding or gain change is applied to that reference. Every excerpt still
matches its parent's exact samples, format, position and gain.

## Individual target reports (0.2.2)

New plans write `review.json` and `REVIEW.md`, and the CLI summarizes calibration
limits. Ensemble eligibility, individual artistic targets, full export verification
and listener acceptance have separate fields. A passed ensemble check does not
turn a missed target into a success. The existing six productions are unchanged.

- **Decay:** each reverb reports the requested and measured duration, signed
  residual, selected control, and measured durations at controls 0 and 0.9.
  Targets below/above those endpoint observations are labelled explicitly. The
  original six-step search and its selected controls are retained. Inside the
  measured range the result is labelled an approximate fit, with its residual
  visible; no new tolerance declares the artistic target achieved.
- **Return level:** the report retains the seed measurement, requested correction
  and final return. It adds the ±18 dB correction bound, requested amount, any
  −60 dB schema floor, predicted wet/source level and signed remaining mismatch.
  The effective target is the recipe target plus `20 log10(amount)`. At zero
  amount there are no added buses and the complete baseline is preserved.
- **Actual bus outcomes:** production DSP measures the frozen candidate over
  pooled training and each training/held-out passage. Each bus keeps its training
  activity gate. Its active-window wet/source ratio is compared with the
  amount-adjusted target; positive residual means wetter than requested. Less
  than 0.5 active seconds or insufficient return energy yields an explicit
  unmeasured result, never a zero-error claim. Held-out observations cannot change
  the calibration, recipe or gain limits.

These ratios describe equal-weight active 20 ms windows, not a whole-song meter
or finished export. The decay measurement still uses a six-second capture; it
does not establish a physical room RT60 or an unlimited decay range. No listener
approval or export gain is inferred. Complete rendering, finalization and tail
verification remain separate steps. Existing reports retain their original
meaning and are not silently rewritten with new measurements.

Incomplete trailing windows are omitted, so a short fragment cannot receive a
complete window's weight. Ensemble checks report coverage and named failure
reasons; empty, missing, overlapping or nonfinite evidence cannot pass. Invalid
or incomplete passage metrics are null. All original numerical ensemble limits
are retained, with the explicit zero-master-reduction requirement above.
Small positive amounts retain their full logarithmic target in the report and
use scientific notation when fixed decimals would misleadingly display zero.
The existing return floor can still prevent reaching that target.

### Frozen inputs and recovery

New plans hash the baseline, policy and every routed recording before measurement.
Source hashes are checked again after the frozen candidate's held-out observations.
A changed recording prevents readiness. `frozen-before-held-out.json` pins the
candidate and input identity, so its passage results describe one declared proposal.
Shared SHA-256 helpers also serve EQ matching, preserving its existing public API.

`ready.json` is written last and pins the complete set of reports and settings.
Ready JSON files are serialized and synced before becoming visible, with no
overwrite of an existing destination. The read-only `ambience-check` command
verifies that completion record, saved bytes, direct-path preservation and current
source hashes. It does no fitting, audio measurement, rendering or playback.
The hashes detect stale or changed content; they are not digital signatures.

An error leaves `failure.json` when it can be written, plus the available frozen
inputs and evidence. An interrupted plan without `ready.json` is incomplete even
if a settings file is present. Keep that evidence and use a fresh directory for
a retry. These writes improve recovery and verification; they do not establish
that the unresolved Pi filesystem fault is fixed.

Legacy plans have no completion record and remain supported as historical evidence
with their original full-export verification. The new check refuses to infer
missing identity or master-stage measurements. No current production needs to be
remade merely to obtain new report fields.

### Audit an existing plan without remeasurement

```sh
target/release/gigpies ambience-audit SAVED_PLAN NEW_AUDIT
```

`ambience-audit` reads the saved policy, decisions, seed FX, calibration, selected
settings and passage checks. It writes `AUDIT.md` and `audit.json` to a new
directory. This works without the recordings. Every consumed report is hashed;
inconsistent routing, targets, return bounds, settings or recorded pass flags
cause refusal before the output directory is created.

The audit reports the recorded decay residual, including a target still exceeded
at minimum control, and the wet-level residual derived from the seed measurement
and applied return. These are saved observations and arithmetic predictions.
They are separate from fresh per-passage measurements in a new plan's review.
The report also states whether training preceded held out and whether the saved
checks observed the master stage and retained window counts and measured durations.
Missing legacy measurements and passage coverage remain unknown.
It cannot establish current recording identity, new technical eligibility,
complete-export gain or listener acceptance. Use `ambience-check` for a new
plan's completion record and current source hashes.

The six retained plans were audited with this command during the reporting work.
The results reproduce the earlier parameter audit: two minimum-decay limits in
Dark Ride, two in Complainiacs, one in Rainfall and one in Catbite. Dark Ride's
vocal echo retains a +4.023632 dB predicted wet/source residual. All six original
policies interleave their training and held-out passages; the audit labels that
limitation explicitly. Their complete render records remain separate evidence.
Saved reports are under
`artifacts/automix/fx-calibration-review-2026-10-03/saved-audits/`.

### Evidence for chronological admission

A synthetic counterexample demonstrated why the new ordering is required:
training at 0–4 and 8–10 s, with held out at 4–8 and 10–12 s, allowed a change
confined to 7–8 s to alter the vocal reverb calibration by 0.036 dB and its delay
seed ratio by 1.806 dB. Direct training observations were identical. This is
causal state carryover, not a reason to reset DSP or tune against held-out results.
The new admission check refuses such plans before creating output. Chronological
synthetic tests verify that held-out changes leave training calibration unchanged.
Historical interleaved policies and their existing mixes retain their original
evidence; reproducing that old method requires its recorded software revision.
New real-audio work needs newly declared chronological passages and its own budget.

## Current six-example experiment

The frozen plan is `artifacts/automix/expert-fx-v1/plan.json`. One rule-generated
proposal and one training-only return calibration per song are allowed, followed
by combined validation; no held-out retries. It starts from the source-preservation
checkpoint's selected direct mixes: unchanged SOURCE for Dark Ride and the retained
five channel/balance selections. Styles and source assignments are explicit in each
song's `policy.json`. No imported effect returns are added. Dark Ride's two bass
parts retain unknown capture identity and their `other` channel roles.

**All six complete renders passed technical verification.** The normal suite has
117 passing Rust tests and 30 passing Python tests, with three historical media
checks intentionally ignored. Clippy with warnings denied, formatting and the locked
Rust 1.97.1 release build pass. Unrelated exhaustive auditions were skipped.

Regressions cover actual rack/renderer agreement, preserved channel settings, exact
zero amount, unknown-source abstention, style/tempo choices, held-out isolation,
invalid identity/FX-return refusal, ensemble overload rejection and checkpoint tail
contracts. Full media checks verify original recording hashes, unchanged direct
settings, exact bypass PCM/float with a zero-only extension, independent export
peaks, zero master reduction and endings below −90 dBFS over the final 250 ms.
The catalogue's musical acceptance remains pending.

### Parameter audit — 2026-10-03

The saved settings, file-to-bus routing and calibration figures for all six songs
are collected locally in `artifacts/automix/fx-review-2026-10-03/FX.md` and
`fx-exact.json`. These choices come from the hand-authored instrument/style
recipes above, followed by engine measurements. They have not been approved by
listening. An unreported existing-space flag does not establish a dry recording.

Requested values can exceed the bounded calibration's reach. In Dark Ride, the
hi-hat and tom chambers measure 0.512 seconds at the minimum decay control against
a 0.450-second target. Its vocal delay requests about −22.024 dB return gain;
the −18 dB bound leaves the calibrated wet/source ratio at about −19.976 dB,
4.024 dB above the −24 dB target. Other short-tail targets also encounter engine
minimums, as recorded in the audit. Passing the combined ensemble guards does
not establish that every individual artistic target was reached. No FX setting
or bound was changed during this audit.

| Example | FX buses | FINAL export gain | Gain change from previous FINAL | Finished LUFS |
|---|---:|---:|---:|---:|
| complainiacs | 4 | -6.305838 dB | -0.099495 dB | -15.6 |
| dark-ride | 8 | -2.980157 dB | -0.122513 dB | -14.8 |
| rainfall | 4 | +5.377307 dB | -0.027205 dB | -16.9 |
| wild-co | 7 | -5.312877 dB | +0.022441 dB | -16.0 |
| catbite | 6 | -2.888254 dB | -0.010514 dB | -17.0 |
| phoenix | 3 | +8.987501 dB | -0.066600 dB | -16.1 |

These gain changes follow the finished mix's sample peak. They are not source
normalization or compensation for a taste target. Dark Ride preserves the source
channel power/body and adds eight FX buses; the new export requires about 0.12 dB
more attenuation than SOURCE. It retains roughly 3.30 dB more export gain than the
older rejected presence-heavy FINAL.

No proposal required a held-out retry or widened guard. All six new settings contain
our generated returns. Supplied room/capture paths remain part of the direct routing;
printed effects are not removed or newly certified. Existing source contacts and
unknown Dark Ride bass capture identity remain limitations. Listen for clarity,
distance, drum tail length, guitar articulation, low-end support and whether the
combined supplied and generated ambience is appropriate. Technical checks do not
resolve those preferences.

The complete verified index is `artifacts/automix/expert-fx-v1/LISTEN.md`; it contains
only the established SOURCE → selected finished FINAL pairs, with the existing
second Complainiacs passage. Full FINALs include six seconds of generated tail;
SOURCE files and their clip samples remain unchanged. Original recordings and
previous direct selections are retained for provenance and recovery.

After dependency/open-file checks, cleanup removed only the new duplicate bypass
exports (verified identical to retained prior files plus zero tails) and compressed
measurement histories losslessly. It recovered **1.104 GiB**. Current deliverables
and evidence retain about **1.1 GiB**; the normal build output is **761 MiB**, with
about **42 GiB** free. Existing uncommitted work was snapshotted and preserved.
No commit, push or host-audio configuration change was made. Dark Ride playback
was explicitly requested separately; it does not establish listener acceptance.
