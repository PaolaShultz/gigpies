# Next GigPies integration: planning brief

Use this prompt in a new planning session. It authorizes a source-grounded plan
and its documentation checks, not runtime implementation or device activation.
Model choice belongs to the user.

```text
Work in /home/shome/p/gigpies. Plan one substantial end-to-end integration slice:
GP-METER, actual audio metering through the production GigPies graph and
transport into the real SHR Desk. Make one executable plan; do not implement it.

Read AGENTS.md, docs/MODULE_IMPLEMENTATION_PLAN.md (especially GP-METER),
docs/COMPONENTS.md and docs/MODULE_CONTRACTS.md. Use zk to load the GigPies index.
Read the relevant modular/Brain acceptance limits and /home/shome/p/shr-desk
instructions. Inspect only the named render/observation/transport and Desk client/
frontend seams plus directly needed tests; follow dependencies when necessary.
Verify live Git state. Avoid a fresh audit of every repository or old research.

Update the GP-METER card in docs/MODULE_IMPLEMENTATION_PLAN.md itself. Keep its
plan, implementation checklist, state, evidence and exact next action together.
Do not create a second plan/status file or duplicate the task in Desk. Other docs
may receive precise contract/discovery links only. Preserve unrelated work.

Outcome: measured channel/main/configured-monitor levels and clipping, alongside
existing dynamics feedback, usable in the real Desk. Determine existing support
first. Resolve tap identities and units, peak/RMS/clip semantics, windows/decimation,
source-frame/epoch/map/age identity, bounded publication and compatible schemas.
Design authenticated delivery, stale/lost-source presentation and recovery without
renewing control authority, fabricating silence or replaying mutations. Preserve
raw REC/analysis, dry/protected output, PFL/AFL/talkback and existing timing owners.
No callback allocations, locks, I/O, unbounded work or telemetry backpressure.

Make the plan executable by another coding agent without rediscovering design:
- exact outcome/non-goals and source-backed decisions, with real uncertainties;
- dependency-ordered phases and named files/functions/contracts by repository;
- small acceptance checklists and fields for implementation/evidence/next action;
- provider-before-consumer delivery, compatibility, faults and recovery;
- deterministic independent signal checks at 16/17/32/48, actual provider-to-Desk
  integration, native offscreen checks and exact raw sample preservation;
- scoped normal tests and required whole suites under each owner's policy,
  serialized shared build lock/jobs=1; separate opt-in physical/load/listening gates;
- one short standalone execution prompt covering the full agreed slice, included
  in the same card, with stop conditions and repository publication boundaries.

Exclude FFT/spectrogram projects, live automix, writable FX, scenes and new device
owners. DAW/Player, Drums, Synth, Sampler, Tone and Skills are references/code sources,
not GigPies modules or tasks to integrate. Reuse needed code with provenance only.
No broad refactors, dependency upgrades, fresh hardware/audio/MIDI/DMX/service work,
private media inspection, full builds or tests for this planning-only pass. Use
source inspection and documentation checks. Do not select or recommend models,
spawn agents or expand the slice to fill time. Keep the plan concise but complete.

Validate changed links/anchors and whitespace; update source-linked knowledge
routing only if needed and run its validator. Finish with the chosen phases,
remaining uncertainties and the execution prompt. Follow owning Git/publication
rules; do not publish from repositories whose instructions require explicit scope.
```
