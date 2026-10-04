# Review and repair the 2026-10-04 commits on Pi5

Open a new session in `/home/shome/p/gigpies` on **rpi5**, select the stronger
model you want to use, and give it this instruction:

> Read `docs/DAILY_REVIEW_EXECUTION_PROMPT.md` and execute the review-and-fix
> brief below in full, using the currently selected stronger model. Review all
> of the pinned 2026-10-04 commits across the twelve repositories, fix confirmed
> defects in place on this Pi, validate, commit and push the fixes, and update
> both Pis to the final published source revisions. Continue autonomously.

The brief is for a future review session. Preparing this file does not constitute
that review. [The frozen scope](reviews/2026-10-04-scope.json) lists **20 commits**
published before this prompt, with full SHAs and each repository's base and tip.
The prompt's own commit and later same-day follow-ups are added at review start.

## Execution brief

Perform a thorough source review and repair pass over the work committed on
**2026-10-04 in Europe/Zagreb**. Execute locally on **rpi5**, starting from
`/home/shome/p/gigpies`. Fix confirmed problems in the original owning
repositories under `/home/shome/p`; do not stop at findings, a plan, one small
fix, or successful tests. Continue through review, repair, validation and final
publication without asking me to approve routine steps between repositories.

Use the stronger model already selected for this session for the substantive
review and fixes. The old Sol 6.1 low implementation-worker requirement does not
apply to this review. Do not downgrade to those earlier workers. Do the review
yourself on this Pi; do not dispatch review or implementation to Pi4. SSH to Pi4
is for state inspection, ledger synchronization and the final source update.
Do not change model configuration, install tools or purchase a service merely
to satisfy the phrase "stronger model".

### Recover the actual source and freeze the review inventory

1. Verify `hostname` is `rpi5`. Read `/home/shome/p/AGENTS.md`, the local
   `AGENTS.md`, `README.md`, `docs/STATUS.md`, `docs/DEVELOPMENT.md`,
   `docs/PUBLICATION.md`, `docs/COMPONENTS.md`, `docs/MODULE_IMPLEMENTATION_MAP.md`,
   `docs/MODULE_CONTRACTS.md`, `docs/HEADLESS_INTEGRATION.md` and the relevant
   architecture/implementation documents. Read each activated owner's own
   instructions, development/publication rules and current implementation.
2. Inspect live status, branches, remotes, active processes, build locks and
   uncommitted changes in every owner before writes. Fetch verified origins and
   inspect divergence. Preserve later user edits and unrelated work; never reset,
   clean, overwrite, or force-push it. Use the original canonical repositories,
   not historical worker worktrees. Resolve routine integration without handing
   the task back to me.
3. Read `docs/reviews/2026-10-04-scope.json`. Its date is fixed, even if execution
   happens on a later date. The window is `[2026-10-04T00:00:00+02:00,
   2026-10-05T00:00:00+02:00)`, selected by **committer timestamp**. Do not use
   only UTC dates, `HEAD~1`, the final commit of each repository, or a moving
   `--since=today` filter. Early-morning H8 documentation and the initial
   Lightdesk root commit are explicitly included.
4. Verify every pinned SHA exists and is reachable from the intended published
   branch. Read every listed commit and its full diff. Build the union of paths
   touched by the individual commits, including deleted/reverted paths and merge
   changes; a final net diff alone can omit work. A null base means the root
   commit is included: review the entire Desk and Lightdesk source trees.
   Read changed production code and tests in full, following relevant unchanged
   callers, dependencies and contracts across repositories.
5. Inspect descendants after each pinned tip. Add any additional reachable
   commits made on 2026-10-04, including this prompt/manifest commit, to the
   initial review inventory. Treat later-date edits as integration context and
   preserve them. Freeze and record the starting heads and covered commit/path
   inventory before making fixes, so new review-fix commits cannot create a
   moving review target.

All twelve owners are in scope: `gigpies`, `shr-desk`, `shr-lightdesk`, `shr-lux`,
`shr-rec`, `shr-fx`, `shr-pa`, `shr-daw`, `shr-drums`, `shr-synth`, `shr-sampler`,
and `shr-tone-over-9000`. The last five changed plans/discovery links only in
this milestone; review those changes and their claims without starting optional
instrument implementation. Algorithms stay in their owners. No sibling path
dependencies or duplicate PA/FX/REC implementations.

Read concise existing evidence from the private
`/home/shome/p/gigpies/user/module-workers/0009/` directory: `completion.json`,
`final-owning-source-manifests.json`, `final-ledger.json`, `joint-results.json`
and final owner checkpoints as needed. Task0008 and task0009 are completed
historical work. Their successful tests and previous reviews are useful evidence,
not grounds to skip a fresh code review. Do not ingest large event logs merely
to recover context or mutate their immutable evidence.

Use a fresh private review directory and the next unused private ledger task ID.
Read the ledger under its lock and keep progress, per-file coverage, findings,
fixes and test commands durable there. Preserve sufficient state to resume
after a context reset without starting over or quietly dropping repositories.

### Review the implementation, not only the happy-path tests

Prioritize concrete correctness, data loss, memory/ABI safety, realtime bounds,
authority, persistence and recovery. For each issue, identify its trigger,
affected path, consequence and supporting source or reproduction. Distinguish
confirmed defects from hypotheses and intentional deferred capabilities.

Work through these connected areas, checking both ends of each contract:

- **GigPies audio and modules:** mixer routing, FOH/monitor independence, ramps,
  frame accounting, sample formats/rates, finite-value validation and numeric
  overflow; actual REC/FX/PA calls and fixed wet/dry order; invalid-input and
  fault behavior; preservation of the qualified host path. Inspect audio callback
  reachability for allocations, locks, I/O, unbounded work and destruction costs.
- **Role broker and local control:** descriptor identity and ambiguity,
  process-held exclusion, generations/epochs, file/socket permissions, symlink
  and path handling, restart fencing, EOF/death cleanup, idle and queue bounds;
  lease authority, shared request deduplication, stale revisions, request ID
  reuse, delayed replies and no replay after uncertain completion.
- **Analysis and lighting:** raw-source identity/map/epoch consistency,
  acquisition age rather than receive-time freshness, partial/dropped windows,
  queue overflow and independent failure; calibration, bounded AUTO grants,
  human Hold precedence, release timing and optional capabilities; persistence,
  atomic replacement, restart disarming and old-source contribution provenance.
  Check LX05 with and without timing/durability instead of assuming all optional
  fields or capabilities are present together.
- **Owner APIs and recorder lifecycle:** C layout/size/version contracts,
  pointer validation, handle/observer lifetime, concurrent finish/query/drop,
  cancellation during preparation, retained terminal state, worker spawn/error
  paths, sample/gap/overflow accounting and durability claims. Preserve existing
  v1 ABI behavior. Check truthful FX/PA capabilities and fault recovery; do not
  confuse logical sample limiting with verified physical or true-peak protection.
- **Both native frontends:** actual provider-backed state/actions, bounded
  separate I/O and input queues, event-loop responsiveness, queue saturation,
  fresh confirmations, stale/unavailable state, disconnect/reconnect, role loss,
  resize/device-loss recovery, frame/state coherence and feedback. Trace operator
  actions through real clients to authorities and back to displayed state.
  A simulator gallery or successful native compilation is insufficient evidence.
- **Publication and documentation:** all added scripts, exact font/licence
  policy, complete-index and outgoing-history guards, root-commit handling,
  ignored/private boundaries, lockfiles, capability/provenance hashes and current
  status claims. Preserve dated historical evidence and distinguish it from
  current behavior. Inspect today's artwork/licensing changes as assets; do not
  regenerate them as a review exercise.
- **Tests and integration:** look for tests that reproduce the implementation's
  assumption, swallow errors, skip the actual provider, weaken timing/authority
  checks, or pass despite a broken path. Review actual process/library integration
  and owned-child cleanup, including negative/error/cancel/restart cases.

Maintain coverage for every in-scope changed path. Record a concrete result for
each area, even if no defect is found. Do not invent findings to meet a quota,
perform broad aesthetic refactors, expand deferred product scope, or replace
working designs without a demonstrated defect or clear maintainability need
within the reviewed change.

### Fix and validate in place

Repair confirmed issues directly in the owning repositories on Pi5. Keep changes
focused, add meaningful regression coverage for the failure mode, and update
affected contracts/documentation with the fix. Re-review the resulting diff and
dependent consumers. Do not disable assertions, soften safety checks, weaken
publication rules, or relabel a failure as expected just to make tests pass.

The agent owns test selection. Run focused regressions during implementation.
For engine/rendering, shared schemas, routing/persistence, concurrency, safety
or broadly reused changes, run the complete normal production suite in each
affected owner. Run matching device-free `hardware-host` and `native` feature
checks where their behavior is affected. Follow owner-required formatting,
Clippy, ABI, publication and release-build checks. Documentation-only changes
do not require repeated full Rust builds.

Build and test only on Pi5. Every Cargo/check/test/Clippy/release/link command
must hold the same **parent-held nonblocking** `/home/shome/p/.gigpies-build.lock`;
one build at a time, `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, Rust `+1.97.1`,
`--locked`, `-j1` where supported, normal target directories and consistent
profiles. Inspect live children before a new build; an elapsed reservation does
not cancel an active process. Reuse the inspected private build helper if useful.
Check free space and target sizes first; review below20GiB free or above5GiB
target. Native dependencies can be substantial. Do not weaken debug/validation
profiles or remove another session's output to save space.

For changed cross-module behavior, validate with actual freshly built owner
libraries/executables and real consumers, recording exact artifact hashes.
Re-run the bounded synthetic joint demonstration when fixes affect its contracts
or behavior; preserve and inspect the prior private harness before adapting it.
Check exact PCM24 samples, recorder lifecycle, analysis delivery, console actions,
lighting loss/release/restart and independent audio operation as applicable.
Join owned children before cleaning temporary endpoints/files. Retain concise
results and reproducible commands rather than generated audio in Git.

Historical media/audition generation, exhaustive matrices, long benchmarks and
one-time evidence renderers remain opt-in unless directly affected. Report which
classes ran and which were intentionally skipped. Synthetic/offscreen functional
checks do not establish physical safety, hardware latency or combined-load limits.

### Authorization and boundaries

I authorize this source review, in-place fixes, focused documentation, software
tests, synthetic PCM/temp-socket and CPU/offscreen checks, and scoped commits and
pushes for the twelve named owners. Review findings may require coordinated
provider/consumer changes; handle those autonomously in dependency order.

Do not open physical audio/MIDI/DMX devices, play audio, open windows on operator
displays, change Bluetooth/audio/display/network/service settings, install
services, or run hardware/shared-load experiments. Kill or interrupt only test
children created and owned by this review. Production remote authentication,
wider channel/scene expansion, writable FX/PA and physical acceptance remain
separate work unless a narrow existing-behavior defect requires a scoped repair.

Do not edit or copy private media, credentials, user sessions, machine state,
downloaded research or old worker snapshots into source commits. Do not change
repository visibility; Desk/Lightdesk are public and REC's existing visibility
is private. No tags, version bump, binary release, installer or manual deployment
follows from this source-review authorization.

### Finish the entire review and publish the repairs

Before declaring completion, finish coverage of every frozen commit/path and
resolve all confirmed in-scope findings that can be repaired without an external
decision. If a real blocker remains, retain evidence, finish independent work,
and report the exact unresolved issue. Do not call an unreviewed area clean.

Inspect live Git state and each complete staged index. Follow each owner's
publication rules and existing hooks; stage named files only, inspect actual
diffs, run guards and whitespace checks, and review outgoing history. Commit
reviewed fixes by coherent owner scope and push to verified origins, providers
before consumers that reference them. Preserve published history and lockfiles;
no force-push. Verify remote branch SHAs and relevant CI, fixing task-related
failures. Do not make empty commits in repositories with no changes.

Then synchronize the **canonical** `/home/shome/p/<repository>` checkouts on both
Pis to the final published revisions. On Pi4, use the pinned `gigpies-pi4` alias
from Pi5, inspect clean status and active ownership, and fast-forward safely.
Its existing origins may point to Pi5's canonical repositories; preserve that
verified arrangement. Never push through a Pi4 origin into Pi5's checked-out
branch. Preserve older task worktrees and isolated worker directories as
historical snapshots; they are not canonical active checkouts. Do not overwrite
dirty work to achieve matching SHAs. Verify all twelve canonical heads and clean
statuses on both hosts against the intended published branches. This is source
synchronization, not a service deployment or a claim that old binaries rebuilt
themselves.

Record final revisions, coverage, findings/fixes, validation, CI, synchronization
and remaining limits in the private ledger under its lock and ownership rules.
Keep a concise durable review report with file/line references, severity, trigger,
fix and regression evidence; separate fixed findings from unresolved risks and
areas with no findings. Clean only owned disposable artifacts after work is idle.

End with a compact per-repository report: covered commits/areas, findings fixed,
tests and intentionally skipped classes, resulting SHAs, push/CI status, both-Pi
source synchronization, preserved unrelated work and any genuine blocker.
