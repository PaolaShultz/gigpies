# GigPies integration: plan and implementation

Updated 2026-10-09. This document owns cross-module integration tasks from planning
through implementation and acceptance. Module-only work stays in the
[owning module plan](MODULE_IMPLEMENTATION_MAP.md). The same task must not have a
second status table in an index, handoff, capability matrix or knowledge note.

## Scope

GigPies is the live-band sound and lighting system. Runtime owners are GigPies,
SHR Desk, SHR Lightdesk, SHR PA, SHR FX, SHR REC and SHR Lux.
SHR-DAW (including Player), Drums, Synth, Sampler, Tone Over 9000 and Skills are
reference/code sources, not GigPies runtime modules or unfinished integration
requirements. Reuse only a needed implementation with provenance; do not add
instrument hosting, a DAW process or their standalone backlogs to GigPies.
[Components](COMPONENTS.md) owns the precise boundaries.

## How work is tracked

1. Give each task one stable ID, one owning document and one accountable owner.
   A cross-module task here names contributing repositories and their boundaries;
   do not mirror it as new tasks in each contributor's plan. A distinct provider
   algorithm/ABI task may live in its module plan and be a named dependency.
2. Keep outcome, scope, acceptance checklist, current state, evidence and exact next
   action in the same task card. Planning adds detail to that card; implementation
   updates it in place. Never append a contradictory second execution checkpoint.
3. Use PLANNED, IN PROGRESS, BLOCKED (with a concrete dependency), IMPLEMENTED,
   or ACCEPTED. Acceptance must name software, physical or musical scope. Passing
   software tests never closes an unperformed physical gate.
4. Keep only open work in the active queue. On completion, record the final result
   and evidence in its existing card, then archive that closed card once. Link the
   archive from the relevant technical contract; do not maintain a running done list.
5. STATUS is dated evidence, NEXT_SESSION is a pointer, and the module map is
   routing. Contracts describe behavior; acceptance reports contain measurements
   and retained failures. They do not maintain another backlog. Knowledge routes
   to these sources. Update the owning card with the behavior change, not later.

## Baseline evidence

The configurable Stagebox graph, actual PA v2/raw REC/source-clock FX, authenticated
transport and Desk controls have [modular software acceptance](MODULAR_ENGINE_ACCEPTANCE.md).
Brain duplex/ASRC, PFL/AFL and talkback have [integrated software acceptance](BRAIN_AUDIO_ACCEPTANCE.md).
Both consoles have provider-backed native software; Lux has real null-output
lighting authority and named analysis. Physical multichannel, controller/display,
fixture, acoustic and complete-show acceptance remain separate.

The embedded FX contract is fixed stereo delay, not SHR FX's standalone rack.
Current rendered mixer snapshots leave meters unavailable; schematic graphs are
not audio measurements. Offline automix results are not live automatic mixing.
See [dated evidence](STATUS.md), [module contracts](MODULE_CONTRACTS.md) and source.
The [previous plan](archive/tracking-before-2026-10-09/MODULE_IMPLEMENTATION_PLAN.md) preserves legacy GP-01..09/task0009 launch cards;
it is historical, not a queue to execute. Later accepted work supersedes its limits.

## Active work and dependencies

| Task | Owner / contributors | State and next action |
|---|---|---|
| [GP-METER](#gp-meter--measured-audio-on-the-real-desk) | GigPies lead; SHR Desk consumer | PLANNED: produce one executable plan in the card below |
| [GP-FX](#gp-fx--writable-owner-effects-in-the-console) | GigPies lead; SHR FX provider, SHR Desk consumer | PLANNED: depends on owner FX-02/03 prepared writable contract |
| [GP-SHOW](#gp-show--audio-scenes-and-operator-recovery) | GigPies lead; SHR Desk consumer | PLANNED: define scene contents, safes and reviewed recall before implementation |
| [GP-AUTO](#gp-auto--live-soundcheck-and-bounded-assistance) | GigPies lead; SHR Desk consumer | PLANNED: needs trustworthy measurements and an explicit musical objective |
| [GP-REC](#gp-rec--complete-integrated-recording-workflow) | GigPies lead; SHR REC provider, SHR Desk consumer | PLANNED: audit existing host lifecycle before defining missing operator actions |
| [GP-H1](#gp-h1--physical-audio-qualification) | GigPies lead; PA/REC/FX owners as needed | PLANNED: separate physical session and observed rig required |
| [GP-H2](#gp-h2--physical-dual-console-operation) | GigPies lead; Desk/Lightdesk contributors | PLANNED: actual displays/controllers and separate physical session required |
| [GP-H3](#gp-h3--whole-show-qualification) | GigPies lead; all runtime owners | PLANNED: follows audio/console qualification and Lux LX-06 output |

## GP-METER — measured audio on the real Desk

State: **PLANNED; detailed planning requested next**. Owner: GigPies integration.
Contributors: GigPies render/observation/transport, SHR Desk client/presentation.

Outcome: the operator sees actual channel, main and configured monitor levels,
clipping and existing dynamics feedback through the real source graph, authenticated
transport and native Desk. Silence, unavailable data, stale data and clipping must
be distinguishable. Keep raw/processed/bus tap identities explicit.

Plan in this card, replacing the planning placeholder with phased implementation
and progress fields. Inspect `src/mixer.rs`, `src/mixer_control.rs`, `src/local_audio.rs`,
`src/remote/`, `src/paired_readback.rs`, `src/processing_wire.rs`, and Desk's
`src/provider.rs`, `src/remote.rs`, `src/frontend.rs`, `src/render.rs`, `src/native.rs`.
Determine what already exists before adding telemetry. Do not infer levels from
fader gain, commanded coefficients or illustrative simulator meters.

Required acceptance to refine into an executable checklist:

- Defined tap/units, aggregation window, peak/RMS and clip semantics, timing/age,
  reset and source/map/epoch identity; preserve existing gain-reduction meaning.
- Bounded observation/decimation with no callback allocation, locks, I/O or
  backpressure; meter failure cannot stop dry/PA/FX/REC or renew write authority.
- Versioned compatible provider/consumer contract; stalled publication cannot
  make old audio fresh; recovery cannot splice identities or replay controls.
- Real configured channels/buses at 16/17/32/48, independent signal references,
  silence/nonfinite/over-range behavior and exact raw recording preservation.
- Actual provider to Desk acceptance, native offscreen presentation and loss/restart
  behavior. Keep physical/audio/listening acceptance separate.

Exclude FFT/spectrogram/stereo-analysis projects, live automix, writable FX, scene
recall, new device owners and new runtime dependencies on reference projects.
No runtime implementation is authorized by the planning prompt itself.

Progress/evidence: documentation reconciliation only; runtime code and tests
unchanged. Next action: run [the planning brief](INTEGRATION_PLANNING_PROMPT.md).

## GP-FX — writable owner effects in the console

State: PLANNED. Outcome: real prepared wet-effects edits and readback from Desk
through GigPies into SHR FX, preserving independent dry/protected/recorded paths.
Provider design and DSP belong to [SHR FX's plan](https://github.com/PaolaShultz/shr-fx/blob/main/docs/GIGPIES_IMPLEMENTATION.md).
Next: resolve FX-02/03 units, f64 preparation/publication/retirement, bounds and
fault/tail semantics before specifying the host and Desk adapter. Acceptance must
check actual wet samples and failed edits, not just ACKs or standalone rack tests.

## GP-SHOW — audio scenes and operator recovery

State: PLANNED. Outcome: browse/preview/cancel/recall a bounded audio scene with
explicit safes, revision checks and truthful readback. Next: define saved versus
machine-only state and retained human holds. Exclude automatic unmute, resumed
recording/talkback and implicit cross-system lighting actions. Test atomic refusal,
interrupted recall, reconnect and recovery against actual providers.

## GP-AUTO — live soundcheck and bounded assistance

State: PLANNED. Outcome: use real measured sources to propose useful bounded mix
changes while preserving human control. Next: after GP-METER, select one defined
soundcheck/assistance behavior and connect existing offline logic where applicable.
Scope authority machinery already exists; do not recreate it. Require held-out
engineering evidence and separate human listening before musical acceptance.

## GP-REC — complete integrated recording workflow

State: PLANNED. Outcome: safe operator start/stop, readiness, written-versus-queued
progress, integrity and finalized outcome for actual raw REC. Next: inspect existing
host actions and Desk read-only health, then plan only missing operator/storage
feedback. REC owns writer/recovery; standalone player and shell completion do not
gate this task. Unknown durability must stay unknown.

## GP-H1 — physical audio qualification

State: PLANNED; requires a separate authorized physical session.
Next: observed Stagebox/Brain inventory and explicit socket/device maps, UMC/ADAT
clock relationship if that rig is present, duplex drift, electrical latency and
safe fault/rearm. Follow [modular physical procedure](MODULAR_PROCESSING.md#mapping-and-physical-acceptance)
and [Brain procedure](BRAIN_AUDIO.md#bounded-physical-followup).
Keep H8's unresolved timing/right-return evidence. Qualify sustained configured
hardware deadlines and acoustic protection; software profiles are not hardware proof.

## GP-H2 — physical dual-console operation

State: PLANNED; requires a separate authorized physical session.
Next: verify the two actual display/controller identities, separate input/LED
ownership, focus/held-release, restart/reconnect and operator usability. Follow
[roles](ROLE_BINDING.md) and each console's controller/native contract. Injected
role descriptors and offscreen rendering do not qualify physical operation.

## GP-H3 — whole-show qualification

State: PLANNED; requires qualified audio/console paths and a reserved session.
Next: measure the actual combined Stagebox recording/protected audio and Brain
FX/Lux/analysis/two-console workload, deadlines, control responsiveness, memory,
thermal behavior and failures. Physical lighting requires Lux LX-06 first.
Use [Brain responsibilities](BRAIN_CONSOLE_PLAN.md) and owner acceptance limits.

## Verification and handoff

Follow each changed owner's AGENTS.md and current test policy. Use focused normal
regressions during work and required complete normal suites for shared render,
schema, transport, routing or safety changes. Actual-owner/provider/consumer checks
must use the production graph. Keep historical auditions, exhaustive research,
physical and long-load campaigns explicit. Serialize builds under the shared lock;
use pinned Rust, locked dependencies, jobs=1 and CARGO_INCREMENTAL=0.

Before completion, update this task's state/checklist/evidence/next action, preserve
other workers' edits, inspect named diffs and publication guards, and maintain only
source-linked knowledge routing. Follow the owner's publication authorization;
source acceptance does not deploy services or authorize device activation.
