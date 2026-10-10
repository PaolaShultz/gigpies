> Historical snapshot, retired as an active tracker. Relative links were
> adjusted for relocation; source text and dated evidence retain their original scope.

# Parallel software work and launch cards

Checkpoint **2026-10-04 / GP-2026-10-04.1**. The user starts the implementation
sessions after this planning pass. The inventory/review helper is a bounded
read-only worker, not an implementation lane. Never run two helper receivers,
bypass its lock, resume an existing UI session or dispatch recursively.

## Verified development resources

| Host / class | Read-only finding | Initial limit and evidence boundary |
|---|---|---|
| `rpi5` / NVMe, 2 GB Pi 5 | Four cores, about 1.3 GiB available RAM and 30 GiB free; exact 1.97.1 installed. Existing GigPies target 7.6 GiB, DAW 5.2 GiB. | One Cargo build/test/Clippy/linker at once, jobs=1. Review large existing targets before new builds; retain them while idle/ownership is unconfirmed. No new performance benchmark. |
| `rpi4` / microSD, 4 GB Pi 4 | Four cores, about 3.05 GiB available RAM and 35.79 GiB free; exact 1.97.1 installed; default stable is newer. Existing GigPies target about 0.96 GiB; no desk checkouts or other module targets at inventory. | Two independent editing/reasoning sessions, one build at a time, jobs=1. Explicit +1.97.1 avoids the newer default. Storage is useful for software work; no NVMe-throughput claim. |

These are resource observations, not proved compilation peaks or combined-show
budgets. No cargo/rustc job was observed during inventory. Interactive Codex
sessions remain active/preserved on both hosts; process cwd cannot establish
exclusive module ownership. Never borrow their files, reservations or target
without an exact scope check. Development roles leave runtime Brain/Stagebox open.

Exact CPU/storage/device/process facts and peer revisions stay in the private
task-0006 inventory record (private ledger receipt retained locally).
Public plans contain resource classes and reproducible rules, not device setup.
At inventory Pi4's GigPies/PA/FX/REC revisions were older than the inspected Pi5
providers. Never substitute those old clones for the manifest required below.

## Four disjoint lanes

| Lane / host | Checkout and owned work area | First task / dependencies | Budget, output and review |
|---|---|---|---|
| A / rpi5 | `/home/shome/p/gigpies`; new src/show.rs, src/roles.rs, lib export, focused tests and own plan progress. Central contracts owned here exclusively. | GP-01 READY, C-SHOW:1/E01 and C-ROLE:1/E02; HEAD 99eb0f08050a9a86039af9f1ba2a3541649ca7b6 plus preserved pre-existing docs. | Provisional ≤1.2 GiB compiler RSS, ≤1 GiB target growth; slot first. Show/role fixtures → coordinator review, then Desk/Lux consumers. Fallback GP-02 source/fixture design, no concurrent writer. |
| B / rpi5 | `/home/shome/p/shr-lux`; new fixture.rs, lighting_contract.rs, lib export, focused tests and note 0027. | LX-01 READY, C-LIGHT:1/E04; HEAD ba4ccd92656e6d2a3cbc6424cdc2a017d3e4f14d. No GigPies provider needed for this pure model. | Provisional ≤768 MiB compiler RSS, ≤512 MiB growth; waits for A's build slot. Capabilities/corpus → GigPies + Lightdesk review. Fallback further patch rejection cases, LX-02 design only. |
| P4-A / rpi4 | `/home/shome/p/gigpies-module-planning-0006/shr-desk`; actions.rs, model.rs, midi.rs, main.rs, tests and own plan progress only. | DS-01 READY; no engine dependency. Exact no-HEAD snapshot manifest, not a public revision. | Provisional <512 MiB compiler RSS, <256 MiB target growth; slot first. Action/confirmation tests → audio integration reviewer. Fallback remaining DS-01 recovery/docs without compile. |
| P4-B / rpi4 | `/home/shome/p/gigpies-module-planning-0006/shr-lightdesk`; actions.rs, surface.rs, controller.rs, main.rs, tests and own plan progress only. | LD-01 READY; no Lux dependency. Snapshot based on 94402b42aae434a158657b7de9a73f013e27e352 plus plans. | Provisional <512 MiB compiler RSS, <256 MiB target growth; waits for P4-A's slot. Editors/recovery tests → Lux/lighting integration reviewer. Fallback remaining LD-01 interaction cases/docs. |

No two lanes write a worktree, contract definition or physical resource together.
Pi5 original Desk/Lightdesk become read-only references while their peer lanes own
implementation. New local work there requires explicit handback and reviewed
patch/commit; do not independently implement the same UI slice. A/B future tasks
in shared lib.rs remain serial within that lane. The pure first tasks intentionally
avoid new GPU dependencies, media, long renders and device discovery.

## Source and plan delivery

The private task-0006 delivery record names the exact archive SHA-256 and per-file
manifest. The bounded snapshot contains only reviewed source/lockfiles/licensed
font and documentation for Desk/Lightdesk plus GigPies planning/reference files.
It excludes .git, target, media, user settings, credentials and session logs.
An existing active checkout is never updated, reset or overlaid. The isolated root
is `/home/shome/p/gigpies-module-planning-0006`; each surface gets its own Git
metadata and normal target directory. The reference GigPies tree in that root is
read-only and is not a third peer implementation lane.

The source-and-plan archives are prepared in Pi5 ignored `user/module-planning/`.
Delivery/review status and all exact hashes are in the task-0006 private record,
not inferred from successful scp. The receiver checks member paths/types, the
whole archive hash, every manifest file and exact base provenance, then posts
its own immutable acknowledgement before implementation. An uncommitted snapshot
is a source baseline, not an upstream commit. Lightdesk retains its parent SHA in
the manifest; Desk truthfully has no parent. Workers may make a scoped local
baseline commit after inspecting the complete snapshot, without public push.

If a directory already exists, verify its manifest and ownership; never extract
over it or reset it. Source differences require a new named snapshot or reviewed
patch, not silent reuse. Plan-only updates likewise get new hashes and receiving
review. Normal later transfers prefer reviewed exact commits over snapshots.
A patch needs repository/base SHA, SHA-256 and `git apply --check` in an isolated
checkout before acceptance. Only named files/patches travel; no build caches or
home/workspace mirrors. Keep explicit parent instructions beside the isolated
root so nested work reads the same two-node protocol.

If a peer is later unreachable, retain the bounded SSH error and finish local
work. Keep both peer lane cards; mark delivery unverified until checked. Local A/B
still provide two active independent sessions. Do not invent peer inventory or
ask the user to debug routine transfer failures.

## One build slot per host

This is a manual reservation with one OS lock, not a new scheduler/service.
`flock` is available on both hosts. Pi4 has no `/usr/bin/time`; keep the same
lock and use the shell `time` fallback below, recording peak RSS unavailable.
Do not install a profiler or claim measured RSS from that fallback. Check tool
availability again at launch. If flock itself is missing, use one explicitly
acknowledged manual slot; no second build is allowed.

1. At startup inspect live processes, free space, own target size and the latest
   clean private ledger (`git pull --ff-only`). Dirty exchange work is preserved.
   State file ownership and exact source/task IDs in a queued node record.
2. The host slot starts assigned to A on Pi5 and P4-A on Pi4. Each owner appends
   an immutable accepted/running record with node, checkout, task, exact revision
   or manifest hash, command batch and expected ≤20-minute reservation window.
   Other sessions see that owner and continue source review/editing. No peer
   hardware reservation is implied by a compile slot.
3. Before compiling take the same host-local nonblocking lock. Keep the lock in
   the command's parent process for Cargo, tests, linker and child processes.
   A second session that cannot take it exits 75 and does independent work.
4. Use jobs=1 and CARGO_INCREMENTAL=0, normal target and consistent flags. Do not
   change debug info, test coverage, swap, CPU affinity or priority. No per-task
   target copies. The first build logs wall time and disk delta, and peak RSS where the verified
   collector is available (otherwise explicitly unavailable);
   exceeding estimates triggers review/serialization, not automatic concurrency.
5. On completion release only your lock, record result, then hand slot to B/P4-B.
   Failed/interrupted batch records retained artifacts and commands. A wall-clock
   expiry alone does not release a running build: inspect lock/process and contact
   its owner. Never kill another build, delete its lock file or clean its target.

Example command block **after** the ledger claim, from the owning checkout:

```sh
(
  exec 9>/home/shome/p/.gigpies-build.lock
  flock -n 9 || exit 75
  export CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0
  if test -x /usr/bin/time; then
    /usr/bin/time -v cargo +1.97.1 test --locked --all-targets -j 1
  else
    time cargo +1.97.1 test --locked --all-targets -j 1
  fi
)
```

This example is not a substitute for the task's focused/complete validation
selection. All build-producing commands share the slot, including Clippy/check,
release and supposedly small unit tests; two target directories do not provide
resource isolation. Small source/link/docs checks need no Cargo slot. DAW's
stricter explicit build restriction remains in force if it is ever activated.

Peer first-wave source ≤a few MiB and combined target-growth estimate ≤512 MiB;
local first-wave growth estimate ≤1.5 GiB. Stop/review on unexpected growth or
sustained swapping. Keep <32 MiB per synthetic evidence batch; native GPU, full
show, storage-heavy recording and long DSP matrices need a new budget. No cleanup
is needed to author these plans. At task end remove only owned disposable evidence,
keeping required binaries, concise results and reproducible commands.

## User-started implementation cards

Open **two separate interactive sessions** on Pi4, one in each cwd below, after
source acceptance. These are not two invocations of gigpies-peer. Open the local
sessions independently; current planning and unrelated sessions relinquish no
files by implication. Pasting a card authorizes just its named task and fallback.
The one-slot procedure applies even while all four sessions are active.

### A: rpi5, GP-01

Cwd: `/home/shome/p/gigpies`. Plan: `/home/shome/p/gigpies/docs/MODULE_IMPLEMENTATION_PLAN.md`.

```text
Work only in the current gigpies checkout. Read AGENTS.md (if present), the
owning docs and docs/MODULE_IMPLEMENTATION_PLAN.md, then the referenced GP-2026-10-04.1 contracts.
Implement only GP-01; keep progress and evidence in this plan. Check hostname,
HEAD/source manifest, live Git state, active ownership and current contract hashes
before edits; preserve other sessions and unrelated work. If this is a delivered
snapshot, verify its handoff manifest and separate receiving acknowledgement first.
Reserve this host's one build slot as described in GigPies PARALLEL_WORK_PLAN.md;
use Rust 1.97.1, Cargo.lock, CARGO_INCREMENTAL=0 and cargo -j 1 in normal target/.
Run focused checks during work and the required normal suite for changed behavior.
No sibling writes, sibling path dependencies or unilateral contract changes.
No audio/MIDI/DMX/playback/device/display/service changes or shared load tests.
When blocked report exact provider/task/version mismatch and continue only the
independent fallback GP-02 source review/tests design while waiting for the build slot within this repository; do not fake acceptance.
Stop after the scoped task and reviewable handoff, before later milestones,
physical operations, publication or deployment. Do not stage unrelated files or
claim mock, planned or incomplete behavior is a finished engine.
```

### B: rpi5, LX-01

Cwd: `/home/shome/p/shr-lux`. Plan: `/home/shome/p/shr-lux/docs/notes/0027-gigpies-implementation.md`.

```text
Work only in the current shr-lux checkout. Read AGENTS.md (if present), the
owning docs and docs/notes/0027-gigpies-implementation.md, then the referenced GP-2026-10-04.1 contracts.
Implement only LX-01; keep progress and evidence in this plan. Check hostname,
HEAD/source manifest, live Git state, active ownership and current contract hashes
before edits; preserve other sessions and unrelated work. If this is a delivered
snapshot, verify its handoff manifest and separate receiving acknowledgement first.
Reserve this host's one build slot as described in GigPies PARALLEL_WORK_PLAN.md;
use Rust 1.97.1, Cargo.lock, CARGO_INCREMENTAL=0 and cargo -j 1 in normal target/.
Run focused checks during work and the required normal suite for changed behavior.
No sibling writes, sibling path dependencies or unilateral contract changes.
No audio/MIDI/DMX/playback/device/display/service changes or shared load tests.
When blocked report exact provider/task/version mismatch and continue only the
independent fallback LX-01 additional patch rejection vectors and documentation; outline LX-02 tests without implementing before review within this repository; do not fake acceptance.
Stop after the scoped task and reviewable handoff, before later milestones,
physical operations, publication or deployment. Do not stage unrelated files or
claim mock, planned or incomplete behavior is a finished engine.
```

### P4-A: rpi4, DS-01

Cwd: `/home/shome/p/gigpies-module-planning-0006/shr-desk`. Plan: `/home/shome/p/gigpies-module-planning-0006/shr-desk/docs/GIGPIES_IMPLEMENTATION.md`.

```text
Work only in the current shr-desk checkout. Read AGENTS.md (if present), the
owning docs and docs/GIGPIES_IMPLEMENTATION.md, then the referenced GP-2026-10-04.1 contracts.
Implement only DS-01; keep progress and evidence in this plan. Check hostname,
HEAD/source manifest, live Git state, active ownership and current contract hashes
before edits; preserve other sessions and unrelated work. If this is a delivered
snapshot, verify its handoff manifest and separate receiving acknowledgement first.
Reserve this host's one build slot as described in GigPies PARALLEL_WORK_PLAN.md;
use Rust 1.97.1, Cargo.lock, CARGO_INCREMENTAL=0 and cargo -j 1 in normal target/.
Run focused checks during work and the required normal suite for changed behavior.
No sibling writes, sibling path dependencies or unilateral contract changes.
No audio/MIDI/DMX/playback/device/display/service changes or shared load tests.
When blocked report exact provider/task/version mismatch and continue only the
independent fallback DS-01 remaining action-table/recovery cases and documentation without compiling; DS-02 headless event-design only after DS-01 review within this repository; do not fake acceptance.
Stop after the scoped task and reviewable handoff, before later milestones,
physical operations, publication or deployment. Do not stage unrelated files or
claim mock, planned or incomplete behavior is a finished engine.
```

### P4-B: rpi4, LD-01

Cwd: `/home/shome/p/gigpies-module-planning-0006/shr-lightdesk`. Plan: `/home/shome/p/gigpies-module-planning-0006/shr-lightdesk/docs/GIGPIES_IMPLEMENTATION.md`.

```text
Work only in the current shr-lightdesk checkout. Read AGENTS.md (if present), the
owning docs and docs/GIGPIES_IMPLEMENTATION.md, then the referenced GP-2026-10-04.1 contracts.
Implement only LD-01; keep progress and evidence in this plan. Check hostname,
HEAD/source manifest, live Git state, active ownership and current contract hashes
before edits; preserve other sessions and unrelated work. If this is a delivered
snapshot, verify its handoff manifest and separate receiving acknowledgement first.
Reserve this host's one build slot as described in GigPies PARALLEL_WORK_PLAN.md;
use Rust 1.97.1, Cargo.lock, CARGO_INCREMENTAL=0 and cargo -j 1 in normal target/.
Run focused checks during work and the required normal suite for changed behavior.
No sibling writes, sibling path dependencies or unilateral contract changes.
No audio/MIDI/DMX/playback/device/display/service changes or shared load tests.
When blocked report exact provider/task/version mismatch and continue only the
independent fallback LD-01 remaining focus/confirmation/recovery cases and docs while build slot is occupied within this repository; do not fake acceptance.
Stop after the scoped task and reviewable handoff, before later milestones,
physical operations, publication or deployment. Do not stage unrelated files or
claim mock, planned or incomplete behavior is a finished engine.
```

## Later waves and physical gates

| Wave | Local progression | Peer progression | Acceptance to unlock next work |
|---|---|---|---|
| 1 | A GP-01; B LX-01 | P4-A DS-01; P4-B LD-01 | Four small independent source/test handoffs; full normal suites for shared state, no devices |
| 2 | A GP-02 then GP-03, GP-09 role adapter; B LX-02 then LX-03 | DS-02/LD-02 headless frontend/recovery work; or review new provider fixtures | Native dependency compile only after resource review; actual windows/displays separate. No adapter claims before provider acceptance |
| 3 | Freed local owner lane REC-01/02, PA-01 and FX-01 serially; PA-02/FX-02 design reviews | DS-03/LD-03 read-only adapters when exact fixtures/endpoint exist | Recorder operation/progress, truthful fixed-v1 capabilities and fresh snapshot recovery |
| 4 | GP-04/05/06 and LX-04/05 in dependency order | DS-04/05 and LD-04 | Real software control and synthetic recorder/FX/PA/null-light integration; production remote gate separately reviewed |
| 5 | GP-07 scoped channel/monitor/scene expansion | Surface capability workflows | Full normal production checks; independent provider revisions and no simulated completion |
| Physical | GP-H1/H2/H3, LX-06, PA-H1, FX-H1, REC-H1 | Explicit reviewer/reservation roles | Verified interfaces/clock, two displays/controllers, known fixture/loss policies, storage/acoustics, then complete combined show workload |

Each physical gate requires a fresh session scope and exact shared-resource
acknowledgement. None is authorized by this planning prompt, source delivery or
software implementation card. Preserve historical failures and report implemented,
offline-validated, integrated and hardware-verified separately.


## Active execution — 2026-10-04

The user subsequently authorized coordinator-launched workers and sustained
implementation without intervention between software milestones. The cards above
remain the original first-wave handoff. All four first-wave sessions completed;
the coordinator completed corrections and continued the dependency order in
new bounded sessions through the accepted local engine/client integration. Review gates are handled internally, not returned to the
user as a request to restart each small task.

GigPies was implemented in `/home/shome/p/gigpies-gp01-0007`; its reviewed
source has now returned to the original owning checkout after hash and baseline
checks. Lux retains its owning checkout. Both Pi4 surface lanes completed final review and returned hash-verified source
to the original Desk and Lightdesk owning repositories. All four final workers
exited successfully; their isolated source and private evidence remain available.
Every delegated session uses the requested Sol 6.1 model with low reasoning.
One build per host, jobs=1 and explicit release still apply. Review waits release
the build turn; completed workers provide exact source/fixture hashes before the
coordinator starts a dependent pass. Native helper receivers remain single-owner.

The current concrete outcome and dependency progress are in the implementation
map's execution checkpoint. No later physical gate is implied by continuation.
