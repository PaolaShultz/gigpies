# Execute the mixing-engine plan

Copy the prompt below into the next working session.

```text
Work in /home/shome/p/gigpies. Execute docs/SUMMING_PLAN.md through its required
engineering phases, implementation, verification and final review deliverables.
Do the work, not another planning-only pass. Continue beyond the first improvement.

Read AGENTS.md, README.md, docs/STATUS.md, docs/NEXT_SESSION.md, the complete summing
plan and its owning documents. Inspect actual code, tests, saved settings and
reports. Establish the live Git baseline; version 0.2.2 contains the EQ comparison,
FX target reporting, source/readiness checks and chronological-plan admission.
Preserve unrelated edits and do not rebuild those completed features.

First check disk space, ext4 counters, available SMART/kernel evidence and active
processes. The prior Pi crash remains unexplained; boot recovery is not proof of
a full offline filesystem scan. Use Rust 1.97.1, edition 2024, committed Cargo.lock
and CARGO_INCREMENTAL=0. Keep all sibling repositories read-only; PA measurement
and alignment algorithms belong in ../shr-pa.

All old generated renders were explicitly retired at the user's request, including
SOURCE sums, six FINALs, float buses, EQ diagnostics and excerpts. Do not expect the
old listening queue to work or recreate it before doing the engineering work.
Original recordings, source notices/archives, frozen SOURCE/FINAL settings and
scalar evidence remain. Read the local retirement manifest at
artifacts/automix/render-retirement-2026-10-03/manifest.json and completion.json.
Preserve every original and all retained evidence.

Make a short evidence-based task list, then implement the plan in dependency order:
1. Verify neutral summing, pan/stereo interpretation, timing and FX routing against
   an independent reference. Fix demonstrated defects; retain f64 summing if sound.
2. Add the necessary stage, coherent-group, peak-contribution, dynamics and export
   observations. Explain each pursued concern, including no-change findings.
3. Implement a versioned delivery-policy sidecar, a validated true-peak estimator
   and independent comparison/delivery controls. Preserve legacy session identities
   and replay. Comparison settings must not toggle DSP. Validate final quantized
   PCM with an independent meter and the plan's declared error/ceiling budget.
4. Investigate only supported musical hypotheses. Before any real-audio evaluation,
   freeze a small candidate budget, fresh chronological training/held-out passages,
   selection criteria and stopping conditions. Use at most three musical candidates
   per pilot in total. Select on training, freeze once, then test held out. Failure
   keeps the baseline; no retries against held-out failures or widened tolerances.
5. After implementation and technical checks pass, produce new complete mixes with
   our GigPies FX and verify all six examples. Reconstruct only the historical
   baselines needed for comparison, using saved settings rather than rerunning old
   interleaved planners; verify them against retained hashes. Reuse buses and avoid
   redundant audio. Label reconstructed baselines and new deliveries clearly.

During engineering, synthetic test WAVs and temporary test renders are allowed.
Measure real sources without writing listening audio; create the new complete
listening renders in phase 5. New true-peak deliveries use
independent static finalization to the declared -1 dBTP ceiling after meter validation.
Legacy compatibility renders retain their recorded -0.01 dBFS sample-peak contract.
Do not create equal-LUFS listening copies. Exact excerpts must inherit parent samples
and gain. A higher numerical score or compliant peak does not establish better sound.

Preserve source gains, timing, stereo/coherent groups, verified DI-only routing and
existing musical choices unless a declared supported experiment justifies a change.
Dark Ride's two bass capture identities remain unknown. No imported FX returns,
source normalization, hidden makeup/fader moves or stronger EQ merely to force a pass.
Separate technical eligibility, remaining mismatch, compressor/FX interactions,
export gain and listener preference. Retain an unchanged option.

Choose and run focused tests during implementation, then the full normal suite for
shared engine/render/schema/persistence/routing changes. Run applicable opt-in
studies only when their protected behavior changes. Verify formatting, Clippy and
the locked build. Keep recordings, private settings and one-off runners out of Git.

Update focused documentation and the handoff. Keep concise reproducible evidence,
complete new exports and a small verified listening index. Clean your disposable
intermediates after checking dependencies/open files. Report implemented behavior,
validation, unresolved/deferred ownership issues, disk impact and the exact next
step. Listener acceptance remains pending until feedback is recorded.

Proceed autonomously; ask only for a concrete missing fact and continue independent
work meanwhile. No playback without a fresh user go. No hardware, host-audio, MIDI,
DMX, service changes, sibling edits, commit, push or publication in this execution.
If an execution limit intervenes, leave a precise resumable handoff and report what
remains; do not label unfinished phases complete.
```
