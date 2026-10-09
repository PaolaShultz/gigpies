# Documentation

**GigPies 0.2.3: one system, two human-operated consoles.** Begin with the
[system map](ARCHITECTURE.md), [Brain console plan](BRAIN_CONSOLE_PLAN.md) and
[current handoff](NEXT_SESSION.md). SHR Desk owns audio UI, SHR Lightdesk lighting
UI, and SHR Lux the lighting engine. Native software and local engine integration
are implemented; physical qualification remains separate. Cross-module plans and progress live
only in [the integration plan](MODULE_IMPLEMENTATION_PLAN.md).

[Artistic FX pass](ARTISTIC_FX.md): six-example historical evidence, expert-selected
effects, preserved direct tone, calibration and verification. The executed
[summing-engine plan](SUMMING_PLAN.md) adds verified [delivery controls](SUMMING_DELIVERY.md).
Six new complete mixes retain those effects; the current local listening index is
`artifacts/automix/summing-study/2026-10-03-engine/LISTEN.md`.
The [source-preservation reassessment](SOURCE_PRESERVATION.md) supplies its channel
baseline and evidence for withholding unsupported correction.
The [Complainiacs review](COMPLAINIACS_WORKFLOW_REVIEW.md) retains its preceding
selection and evidence.

| Document | Owns |
|---|---|
| [Architecture](ARCHITECTURE.md) | Intended node responsibilities and processing boundaries |
| [USB hardware integration](AUDIO_HARDWARE.md) | Actual-device host, PA/FX/REC adapters, measured budgets, recovery and remaining physical limits |
| [Audio hardware plan](AUDIO_HARDWARE_PLAN.md) | Continuation targets, device ownership, failed targets and acceptance gates |
| [Audio transport](AUDIO_TRANSPORT.md) | GPA1 packet/control formats, PA clock, measured limits, recovery and physical acceptance gates |
| [Audio transport plan](AUDIO_TRANSPORT_PLAN.md) | Owning execution plan, targets, repairs, verification and task completion |
| [Brain console integration](BRAIN_CONSOLE_PLAN.md) | Dual audio/lighting Brain, SHR Desk/Lightdesk/Lux ownership, controller/display assignment and resource boundaries |
| [Local engine integration](HEADLESS_INTEGRATION.md) | Real offline audio and null lighting services, explicit console commands and acceptance limits |
| [Channel EQ and dynamics](CHANNEL_PROCESSING.md) | GP07 FOH-only strip DSP, strict processing control/readback and monitor/raw-tap preservation |
| [Owner-library graph](MODULE_GRAPH.md) | Actual REC/FX/PA activation, recorder lifecycle, fixed logical processing and health |
| [Named analysis stream](ANALYSIS_STREAM.md) | Bounded raw PCM windows, identity, acquisition age and independent failure |
| [Native role binding](ROLE_BINDING.md) | Descriptor identities, process-held ownership, generations and synthetic acceptance |
| [Integration plan and progress](MODULE_IMPLEMENTATION_PLAN.md) | One owning record for each shared task, acceptance checklist, evidence and next action |
| [Module plan routing](MODULE_IMPLEMENTATION_MAP.md) | Runtime owners and links; no copied task states |
| [Next integration planning brief](INTEGRATION_PLANNING_PROMPT.md) | Plan GP-METER end to end in its existing card, without implementing |
| [Module contracts](MODULE_CONTRACTS.md) | Coordinated versioned provider/consumer definitions and shared acceptance examples |
| [Parallel work](PARALLEL_WORK_PLAN.md) | One shared build slot per host; historical lane assignments archived |
| [Continuation and publication prompt](MODULE_CONTINUATION_EXECUTION_PROMPT.md) | Retired brief for the completed software milestone; no new launch authorization |
| [October 4 review and repair prompt](DAILY_REVIEW_EXECUTION_PROMPT.md) | Stronger-model review on Pi5 of the pinned daily commits across all twelve owners, in-place fixes and final source synchronization |
| [Module planning prompt](MODULE_PLANNING_EXECUTION_PROMPT.md) | Historical planning brief: owning module plans, shared contracts and independent implementation lanes |
| [Audio transport execution prompt](AUDIO_TRANSPORT_EXECUTION_PROMPT.md) | Historical brief for the completed transport phase; not a fresh test authorization |
| [Audio hardware execution prompt](AUDIO_HARDWARE_EXECUTION_PROMPT.md) | Historical brief for the qualified stereo host phase; follow the current hardware handoff |
| [Musician review](PERFORMER_REVIEW.md) | Planned QR station preferences, collective readiness and shared previews |
| [Dated evidence](STATUS.md) | Recorded milestones and failures; current tasks live in the owning plan |
| [Components](COMPONENTS.md) | SHR module ownership, PA integration intent, existing work and dependency choices |
| [Summing mixer plan](SUMMING_PLAN.md) | Execution results, experiment gates, true-peak delivery, source interactions and completion record |
| [Summing and delivery](SUMMING_DELIVERY.md) | Versioned delivery sidecar, true-peak measurement, production observations, legacy replay and verified checkpoints |
| [Multitracks](MULTITRACKS.md) | Local source inventory and experiment starting point |
| [Development](DEVELOPMENT.md) | Directory layout, validation and contribution workflow |
| [Publication](PUBLICATION.md) | Private directories, reviewed scripts, commit/push hooks and release boundaries |
| [Validation](VALIDATION.md) | Dated foundation checks and acceptance limits |
| [Visual map and concept review](CONCEPT_REVIEW.md) | Current hero/system mapping and limits of the preserved original artwork |
| [Archive](archive/README.md) | Preserved source drafts |

Historical research and validation entries keep their dated scope. Current status is explicit;
a feature described in a blueprint is not evidence that it is implemented or verified.

- [Offline automixer](AUTOMIX.md): CLI, soundcheck/freeze, presets, A/B and validation.

- [Effects and automatic review](FX_PASS.md): optional FX, deterministic corrections and comparison.

- [Offline musical balance](BALANCE_PASS.md): measurements, policy and OLD/A/B evidence.

- [Manufacturer-reference experiment](PRESET_EXPERIMENT.md): source traceability, DSP mappings and separate processing/fader comparisons.

- [Instrument tone preparation](TONE_PASS.md): known input groups, musician intent, automatic broad EQ and held-out DSP checks.

- [Source rules and soundcheck advice](SOURCE_RULES.md): explicit profiles, coordinated corrections, source-first suggestions and repeat-measurement boundaries.

- [Independent reference review](REFERENCE_REVIEW.md): content alignment, matched-section measurements and prepared two-clip playback.

- [Shared local media](LOCAL_MEDIA.md): common source library, compatibility paths and relocation verification.

- [DI bass and kick](BASS_KICK.md): processing-stage evidence, note-conditioned correction, safeguards and the local Complainiacs result.

- [Kick/snare evidence and rhythmic emphasis](DRUMS.md): processed-output intent,
  drum tone/dynamics investigation, bass interaction and final-export tradeoffs.

- [Confirmed snare bleed and failure handling](SNARE_BLEED.md): corrected event
  segmentation, conditional waveform evidence and rejected cleanup trials.

- [Complete listening checkpoint](LISTENING_CHECKPOINT.md): six complete SOURCE → FINAL MIX pairs, bounded selections, source routing and export verification.

- [Complainiacs reassessment](COMPLAINIACS_REASSESSMENT.md): decision provenance,
  ensemble/export causes, bounded rejected probes and the revised listening checkpoint.

- [Source preservation](SOURCE_PRESERVATION.md): evidence for intervention, optional offline HPFs, abstention defaults and bounded Dark Ride reassessment.

- [Expert artistic effects](ARTISTIC_FX.md): explicit instrument/style profiles,
  production-engine decay and return calibration, ensemble guards and finished mixes.

- [Frozen EQ matching](EQ_MATCHING.md): reference catalogue, provisional maps, amount/reset, CLI/local review and validation.
- [Next session](NEXT_SESSION.md): dual Full-HD consoles, engine contracts, qualified hardware limits and listening/peer handoffs.
- [Two-Pi development lab](NODE_LAB.md): fixed Ethernet addresses, SSH/Git handoffs and the first protocol experiments.

## Task ownership

Plan and track the same task in [the integration plan](MODULE_IMPLEMENTATION_PLAN.md)
or its [module owner](MODULE_IMPLEMENTATION_MAP.md). Other documents link there.
Closed milestone cards and obsolete launch instructions are archived, not active work.
