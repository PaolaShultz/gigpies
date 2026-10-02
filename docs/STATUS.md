# Status and next steps

## 0.2.1 — frozen EQ matching, offline-validated

Versioned measured maps and provisional artistic directions now feed a deterministic
broad-filter matcher using production EQ response. Saved baseline/source identities,
0–100% amount, exact reset, capacity refusal, CLI commands and a local HTML review
page are implemented. Existing EQ, effects, input groups, trims and routing are retained.
See [algorithm, research, controls and evidence](EQ_MATCHING.md).

126 normal Rust and 34 Python tests pass. Synthetic injected coloration receives a
bounded correction. The deliberately coloured Dark Ride pilot abstains because its
reference uncertainty masks the known change; this conservative miss is retained,
with no retuning or new audition. All six current complete productions remain unchanged;
five focused regressions and Dark Ride's clean control match retained production
observations. No listening improvement or hardware readiness is claimed.

See [release validation](VALIDATION.md#021-publication-checks--2026-10-03) and
[release notes](../CHANGELOG.md). Publication hooks and CI enforce private-data
and reviewed-script boundaries.

## 0.2.0 — previous offline release

The previous release introduced the source-preserving decision workflow, contextual
analysis, explicit artistic FX and verified listening tools described below.
Its local release checks passed: 117 Rust tests, 34 Python tests, formatting, Clippy
and the locked release build. See [0.2.0 validation](VALIDATION.md#020-publication-checks--2026-10-03).
Musical and hardware acceptance remain separate.

## Next hardware session

The user plans a second Pi connected by cable, initial LAN/protocol measurements,
and a compact screen/MiniLab control surface. The proposed mapping and two-Codex
handoff workflow are recorded in [next-session notes](NEXT_SESSION.md). These are
planned; no MIDI, network service or hardware operation was added.

## PA module ownership and planned integration

The user confirmed the modular direction on 2026-10-02: develop PA processing,
measurement and alignment in SHR PA, then integrate its finished module here.
The new reference/mic phase-measurement and delay/polarity task is recorded in
`../shr-pa/docs/PHASE_ALIGNMENT.md` and its P4/P5 roadmap. See
[module ownership and integration](COMPONENTS.md#pa-module-and-planned-integration).
Measurement, automatic alignment, GigPies integration and acoustic acceptance remain
pending. Only documentation changed; audio code, host settings and mixes are untouched.

## Current artistic requirement — finished mixes with GigPies effects

The listener clarified that full mixes include expert-selected GigPies effects.
The previous checkpoint's full-duration exports had no GigPies FX and did not meet
that artistic requirement. Source preservation remains the channel-tone baseline.
The new [artistic FX pass](ARTISTIC_FX.md) selects effects from explicit instrument
and style profiles, calibrates our production engines, and freezes settings before
held-out verification. All six complete renders and the exact-clip checkpoint are
verified in `artifacts/automix/expert-fx-v1/LISTEN.md`. Supplied FX-return tracks remain excluded;
no rendered reference mix supplies effects. Listener acceptance is pending.
117 Rust and 30 Python tests pass, plus Clippy, formatting and the locked release
build. Source hashes, direct-path retention, full peaks and generated tails pass.

## Current offline decision model — source preservation

Dark Ride now selects the unchanged SOURCE as FINAL. The bounded two-proposal
reassessment rejects the HPF-only control for lack of a demonstrated benefit and
retains the other five mixes. Complete production exports and the verified six-example
checkpoint are indexed in `artifacts/automix/source-preservation-v1/LISTEN.md`.
Listener acceptance remains separate and pending.

Offline reviews admit explicit HPF choices including bypass. Missing tone/balance
intent and default master/FX review preserve settings; `preserve-source` prepares
an unchanged ensemble without automatic role processing. Bass/drum measurement
uses the configured master filter. Prior numerical protection budgets are retained.
See [decision failures, contract revision and evidence](SOURCE_PRESERVATION.md).

## Planned musician review after soundcheck

The existing QR station connections could serve each musician a review of their
own instrument: keep the sound or submit preferences, mark ready, then receive a
shared mix preview once everyone is ready. The first version uses structured controls
and deterministic rules; optional small-model interpretation remains research.
Phone preview, personal monitors and applying changes to the live mix have distinct
scopes. See [the recorded proposal and validation needs](PERFORMER_REVIEW.md).
No network, performer UI, model runtime or live application was implemented here.

The sections below record earlier experiments and their original requirements.

## Listening follow-up — Dark Ride source audit

After authorized playback, the listener reported good progress overall but preferred
Dark Ride SOURCE, saying FINAL lost power. A source audit confirms authentic archive
files and producer-described sampled drums/designed tones; “raw” does not guarantee
untreated capture. Production-stage analysis reproduces both stored float buses
exactly and identifies strong kick compression, guitar EQ/makeup, and greater export
attenuation with reduced bass/kit support. No mix or engine change was made in this
audit. Dark Ride FINAL needs reassessment; the other five have no newly reported
objection. See [evidence and limits](MULTITRACKS.md#dark-ride-source-and-processing-audit--2026-10-02).

## Previous checkpoint — independent Complainiacs reassessment

The latest snare observation was investigated from the revised FINAL, whose extra
+2 dB drum step was already removed. A frozen pass selects snare makeup
+3.5 → +1.906326 dB at unchanged faders; a broad-EQ alternative fails protection.
The complete six-example SOURCE → selected FINAL set is verified, with a second
Complainiacs ending passage. Other examples retain their mixes. Listener acceptance
is pending; earlier numerical mixing preferences are withdrawn.

Implemented repairs separate unknown musical balance from fixed-fader processing
verification, expose coherent snare/ensemble contributions, pin historical SOURCE
identity and label pre-export loudness. 105 normal Rust and 29 Python tests pass;
five-example regressions and full export/clip verification pass. Superseded processed
audio is retired after verification, preserving raw/SOURCE mixes and decision evidence.
No playback, host-audio changes, commit or push. See
[current findings, selection and limits](COMPLAINIACS_WORKFLOW_REVIEW.md).

The sections below retain the history of earlier passes and their original intent.

## v0.1.0 — first public foundation

Implemented:

- Standalone Rust library and CLI; pinned toolchain and locked dependencies.
- Read-only WAV header inventory for a file or a flat directory, with JSON output.
- Synthetic regression tests and Linux CI; no hardware required.
- Architecture, source-material notes, dependency map and preserved concept sources.

## First offline automixer — implemented, offline-validated

- Synchronized native-rate streaming soundcheck with bounded activity-aware trim,
  linked calibration groups, causal history and explicit freeze/save.
- Editable role presets: HPF, parametric EQ, stereo-linked compression, pan/faders.
- BWF time-reference alignment, zero-padded tails and conservative master peak control.
- Frozen-settings rendering, full-song local A/B WAVs and loudness-matched copies.
- Measurements and gain/reduction histories; input clipping is reported, not repaired.
- Synthetic DSP/routing/persistence tests; opt-in deterministic private-media variations.

The first full-band experiment has been rendered and its file/measurement contracts
checked. Listening is the next evaluation; tests do not establish musical quality.
See [workflow, presets and limitations](AUTOMIX.md).

Not implemented: live audio or device transport, networking, web/TUI/controller,
recording, lighting integration, gates or true-peak limiting.
No live latency, acoustic safety, listening acceptance or Pi headroom is claimed.

Next: review the new manufacturer-reference SOURCE → OUR MIX comparisons and record
listener preference separately from policy compliance. Earlier rejected candidates
remain preserved as historical evidence. Hardware integration remains separate.

The [archived blueprint](archive/blueprints/blueprint-v2.md) retains the wider scope.

## Optional effects and automatic review pass

Implemented: separate plate/chamber/hall reverbs, vocal predelay validation, chorus,
filtered delay, oversampled excitation and modest master maximization. The Rust
`finish` command analyzes the preliminary mix, applies bounded spectral/return/dynamics
rules, rerenders and records its decisions and post-checks. Corrections are made by
code; no spectrogram interpretation or AI decision is in that loop.
[Commands, algorithms and limitations](FX_PASS.md). Listening acceptance remains pending.

## Unity-source revision

Implemented and offline-validated: DI-only bass, unity input trims and faders,
90 Hz channel HPFs except kick/bass, 40 Hz processed master HPF, measured compressor
thresholds and loss compensation, independent final PCM output leveling to −0.01 dBFS, preserved
float sums, and code-generated envelope/spectral reports. No loudness matching in
this workflow. [Commands and limits](UNITY_PASS.md). Listening acceptance remains open.

## Musical balance pass

Implemented and offline-validated: synchronized role-band and event measurements,
explicit tom-bleed uncertainty, microphone covariance, bounded static kit/group
fader search, held-out section regression checks, event-level compressor/master
reduction, and exact unmatched OLD/A/B excerpts. The local processing candidate
partially restores guitar low-mid cuts after rendering the fader-only candidate.
Initial policy targets remain hypotheses and some remain unmet; listener preference
is pending. No playback or hardware changes. [Workflow and limits](BALANCE_PASS.md).

## Manufacturer-reference experiment

Implemented and offline-validated: explicit low/high shelves, broad-Q bells,
traceable local Yamaha parameter mapping, unsupported-parameter audits and a bounded
training-only compression adaptation. Six local sessions were assessed; five received
full-song processing and separate fader experiments. Exact SOURCE → OUR MIX excerpts
and two independent produced-reference comparisons are ready locally. The normal
suite passes (41 Rust and four Python tests); listener preference remains unknown.

Only Complainiacs' fader proposal was accepted. Rainfall met its applicable balance
policy without a fader change; other band targets remain unmet, and Phoenix has no
applicable ensemble policy. Dark Ride's full-band render is deferred without a
verified bass DI. [Methods, reference-use limits and results](PRESET_EXPERIMENT.md).
Manufacturer collections, recordings, settings and detailed private evidence remain
local. No console emulation or hardware verification is claimed.

## Instrument tone preparation

Implemented offline: explicit known guitar input groups, intent-conditioned
body/presence measurement, a fixed-budget broad-EQ search, actual DSP validation and
held-out rejection. Original routing, faders and secondary paths are preserved.
The listener reported improved overall balance from the manufacturer-reference pass
but insufficient Complainiacs guitar body. The new pass addresses that automatically;
its full-song correction improved held-out body/presence by 1.6 dB while the
selected tone range remains unmet. Listening preference remains pending. Other fault
classes are future work; no ringing fault is asserted in this source. See [tone preparation](TONE_PASS.md).

## Source rules and source-first advice

Implemented offline: explicit numerical profiles with instrument/capture matching,
body/presence correction, sustained-compression relief, PCM full-scale-contact
abstention, combined-rule validation and separate remaining-target reports. Profile
names do not select processing. Large live-capture mismatches produce setup-specific
source-adjustment advice and retain current settings for a repeat soundcheck.
Historical recordings can receive a bounded improvement with limitations recorded.
Style catalogues, other instrument families, live musician UI and physical capture
control remain planned. See [source rules](SOURCE_RULES.md).

Offline validation: 68 Rust and four Python tests pass. Five local guitar pilots
and a full-song Complainiacs check preserve the existing accepted full-song settings;
a labelled injected compression fault demonstrates bounded partial relief. Reports
separate remaining target deviations from accepted changes. No new listening round
or playback was added. Detailed outcomes are in [source rules](SOURCE_RULES.md).

## Guitar decay revision

The prior ending veto treated a fading spectrum as steady tone. The new temporal
rule preserves dynamics checks on that fade while allowing a stronger static guitar
correction. Complainiacs now accepts +5 dB at 300 Hz / −3 dB at 2400 Hz; held-out
body/presence improves from −8.49 to −2.36 dB. One current → new mix pair is ready
locally. Listener preference is pending. The four other saved session pilots retain
their settings. 71 Rust and four Python tests pass. See [evidence, rules and
limitations](TONE_DECAY.md).

## Independent supplied-reference review

Implemented offline: bounded event-envelope alignment, confidence/ambiguity checks,
matched-section tone/dynamics/stereo measurements and exact native-gain excerpts.
Reference measurements never select mix settings. Complainiacs aligns at about
+0.10 s in eight main sections; its fading ending is ambiguous. A prepared two-clip
player now completes preflight before either clip starts. 76 Rust and eight Python
tests pass. This work and all supplied media remain local, without a push. See
[reference review](REFERENCE_REVIEW.md).

## Shared source library

267 loose WAVs and the original recording library now live in `../waves`, available
to GigPies and shr-lux. Existing GigPies recording paths remain valid through an
ignored symlink; the shared catalogue has canonical paths. No audio was rewritten
or pushed. See [layout and verification](LOCAL_MEDIA.md).

## Bass DI and kick review

Implemented and offline-validated: synchronized processing-stage measurements,
confidence-limited note estimates, bounded bass EQ, note-support/transient guards
and a separately measured bass-fader option. Complainiacs selects +4 dB at 700 Hz
and −2 dB at 180 Hz, Q 0.7, with all previous guitar/drum settings retained.
Held-out definition/body improves 3.89 dB; the provisional intent range remains
unmet. A localized PCM-contact safeguard was reconciled with the existing repeated
contact rule, with the unchanged candidate revalidated and the source limitation
retained. One verified 12-second current → bass-corrected pair is ready; no playback,
commit or push. 79 Rust and eight Python tests pass. See [bass/kick evidence and
limitations](BASS_KICK.md).

## Kick/snare rhythmic emphasis

Implemented and offline-validated: synchronized drum processing-stage evidence,
bounded static-EQ sensitivity probes, recovery-conditioned compressor relief and
explicit processed-output rhythmic offsets with frozen-candidate full-song checks.
Complainiacs retains existing drum EQ/compression and adds +2 dB at each kick/snare
fader, preserving the accepted guitar and latest bass processing. Full-song peaks
require 1.682 dB more export attenuation; the finalized mix is about 1.2 LU quieter.
One exact baseline → new 12-second pair is ready, without playback. Neutral-balance
perception, snare tone preference and listener acceptance remain unresolved.
86 Rust and eight Python tests pass. See [drum evidence and tradeoffs](DRUMS.md).

## Confirmed snare bleed and event-detector repair

The listener confirms snare bleed. A deeper review found that a weak precursor could
lock out a stronger hit and corrupt quiet-hit/body statistics; the detector now
retains competing rises, protects compound events and keeps independent crest guards.
Three bounded static snare EQ candidates fail protection/confidence checks and remain
unapplied. Frozen low-band waveform predictors explain about 22% (kick reference)
and 40% (overhead reference) of weak-event energy in 42 held-out events, but do not
establish safe source separation. No gate, expander or subtraction is enabled.
The existing rhythmic pair passes revalidation. 94 Rust and eight Python tests pass.
See [snare-bleed evidence and failure handling](SNARE_BLEED.md).

## Temporal bleed identifiability and recovery follow-up

The next bounded experiment compares frozen attack and four-phase temporal shapes.
The 24–36 s training passage supplies 28 attack references but no strong events with
isolated 240 ms support; temporal fitting abstains. Attack shapes still overlap
quiet kick-coincident hypotheses, so source removal remains unsupported. A separate
recovery defect now stops the preceding tail at the next cluster's weak precursor,
removing six kick and eight snare invalid recovery votes. Full-song production-DSP
revalidation retains the unchanged +2 dB rhythmic candidate and its export tradeoff.
95 Rust and 16 Python tests pass; three historical tests remain opt-in. No new audio,
playback, commit or push. See [temporal evidence and required soundcheck](SNARE_BLEED.md#temporal-follow-up-frozen-representation-audit).

## Joint-reference bleed diagnostics

Implemented and offline-validated: a fixed-delay joint kick/overhead predictor,
complete split-contained sample support, bounded fitting and frozen full-source
evaluation. On 42 held-out weak hypotheses, median low-band energy explained rises
to 47.75%, but hypothetical subtraction increases energy in 76/123 strong events
and has large protected-event/gap changes. Source identity remains insufficient;
no processing or listening export was selected. Known-source regressions protect
against contaminated references and correlated unison playing. 102 Rust and 16
Python tests pass; three historical tests remain opt-in. See
[joint-reference methods and failures](SNARE_BLEED.md#joint-kickoverhead-reference-experiment).

## Complete six-example listening checkpoint

All six established examples now have full SOURCE and FINAL MIX exports plus one
verified synchronized excerpt pair. Dark Ride receives its first complete render,
retaining both complementary bass parts without inventing DI/amp identity. Phoenix
accepts a full-song guitar correction at fixed faders; the other examples preserve
their latest defensible baselines. No snare cleanup or new static balance retry was
applied. Final files are independently peak-finalized and clips retain exact parent
PCM. 102 Rust and 20 Python tests pass; three historical tests remain opt-in.
Listener acceptance remains pending; nothing was played, committed or pushed.
See [selection, routing and validation](LISTENING_CHECKPOINT.md); the private listening
index is `artifacts/automix/listening-checkpoint-v1/LISTEN.md`.

## Complainiacs reassessment after checkpoint listening

The listener found SOURCE better in some passages and FINAL too forward in the
voice, less prominent in guitars and lacking bass/body. A local audit traced the
largest relationship changes to earlier faders, overlapping vocal EQ, the guitar
presence correction and drum-driven export attenuation. A bounded five-proposal
pass selects one new **fader-only** FINAL: kick/snare −2 dB, both guitar paths +2 dB,
bass +1 dB and lead vocal −3 dB relative to the checkpoint. Processing probes were
not selected; failed protections remain recorded. The old artistic balance policy
still fails and is not presented as listener preference.

Implemented: actual SOURCE bypass analysis and fixed-window paired ensemble reports
with section, quiet/strong, coherent-microphone and export-stage evidence. Validated:
103 normal Rust and 24 Python tests, unchanged prior measurements on Rainfall/Catbite,
and exact full exports/clips at 24–36 and 90–102 seconds. Five other examples retain
their settings. New listener acceptance remains pending; no playback or publication.
See [audit, settings, decisions and limits](COMPLAINIACS_REASSESSMENT.md). Private
listening index: `artifacts/automix/complainiacs-reassessment-v1/LISTEN.md`.
