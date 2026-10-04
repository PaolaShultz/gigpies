# Coordinated continuation and publication prompt

Paste the prompt below into a new coordinator thread rooted at
`/home/shome/p/gigpies`. It continues the accepted 2026-10-04 software work and
explicitly authorizes the final source commits and pushes. It does not authorize
physical operations. The earlier planning-only prompt and first-task launch cards
are historical; use their contracts and ownership, not their old stopping points.

```text
Work from /home/shome/p/gigpies as the continuing GigPies implementation,
integration and publication coordinator. Execute the work; do not merely produce
another plan. I want substantial integrated progress, using the synchronized
two-Pi workflow from the previous thread, without my intervention between tasks.
At the end, all reviewed project work from the preceding planning/implementation
threads and this continuation must be committed and pushed in its owning Git
repository. Keep unrelated work and private data intact.

AUTHORIZATION AND SCOPE

I authorize implementation, focused documentation, tests and integration in
GigPies, shr-desk, shr-lightdesk, shr-lux, shr-rec, shr-fx and shr-pa, within the
owning module plans and concrete next milestone below. I also authorize reviewing,
committing and pushing the existing GigPies planning documents and discovery
links in the other repositories listed by MODULE_IMPLEMENTATION_MAP.md:
shr-daw, shr-drums, shr-synth, shr-sampler and shr-tone-over-9000. Those optional
or reference modules do not become new runtime implementation projects.

I explicitly authorize delegated implementation and independent review workers.
Use gpt-6.1-sol with model_reasoning_effort="low" for EVERY delegated worker and
reviewer. Preserve the coordinator's current model. Launch workers from here,
including TWO INDEPENDENT SESSIONS on the other Pi; do not ask me to paste their
prompts or start each subsequent pass. Perform dependency acceptance yourself.
Continue after reviews and bounded worker exits until the next substantial
milestone is complete and publication is verified, or a real external blocker
prevents it. A worker reaching its time limit is not a reason to return routine
coordination to me.

Source-only builds, native frontend implementation, synthetic role enumeration,
offscreen/software-rendering checks, bounded synthetic PCM/recording fixtures,
and temporary owned private Unix-socket integration tests are authorized. Do not
open physical audio/MIDI/DMX devices, launch windows on the actual operator
displays, change display/audio/network/service settings, install services, play
audio, run physical/shared-load experiments or deploy remote production control.
Missing physical acceptance does not block useful software implementation or
honest source publication. Do not relabel untested hardware behavior as complete.

RECOVER THE ACTUAL STATE FIRST

Read /home/shome/p/AGENTS.md, this checkout's AGENTS.md, README.md and:
- docs/STATUS.md
- docs/NEXT_SESSION.md
- docs/HEADLESS_INTEGRATION.md
- docs/MODULE_IMPLEMENTATION_MAP.md
- docs/MODULE_IMPLEMENTATION_PLAN.md
- docs/MODULE_CONTRACTS.md
- docs/PARALLEL_WORK_PLAN.md
- docs/COMPONENTS.md
- docs/BRAIN_CONSOLE_PLAN.md
- docs/PUBLICATION.md
Then read each activated repository's AGENTS.md, owning plan, current status,
development/publication rules and relevant implementation before assigning edits.

Read the PRIVATE completed handoff under:
  /home/shome/p/gigpies/user/module-workers/0008/
Start with completion.json, final-owning-source-manifests.json, final-ledger.json,
joint-show-results.json and handback/*/result.json. CONTINUATION.md has a long
history: its FINAL/COMPLETE records supersede earlier in-progress entries. Read
the accepted final pass3 handoffs/checkpoints only as needed; do not ingest huge
event logs or repeat completed experiments merely to recover context.
This continuation prompt and its index links were added after the final task0008
source manifest; their doc-only hashes are in continuation-prompt-handoff.json
under that private run root. Reconcile later changes rather than resetting them.

Refresh the clean private ledger using its lock/protocol, and read the final
task0008 acceptance record and any newer records. Task0008 is COMPLETE; create
the next unused task/run namespace for this continuation. Never overwrite its
immutable records, runner metadata, original delivery manifests or acceptance.

The accepted baseline, to verify against live source rather than blindly assume:
- GigPies: GP-01 show/roles, GP-02 authority/codec, GP-03 real eight-input offline
  mixer (stereo FOH + two monitor sends; 48-frame boundary, 240-frame ramps), and
  the PRIVATE LOCAL subset of GP-06. Durable epoch reservation prevents old
  uncertain commands from becoming fresh commands after restart. Meters remain
  unavailable and the new offline graph is not yet PA protected.
- Lux: LX-01/02/03/04 real null-output fixture/programmer/Hold/cue/playback
  authority, bounded service, 500 ms release and durable disarmed restart.
- Desk: DS-01, headless portions of DS-02, DS-03/04 actual provider client.
- Lightdesk: LD-01, headless portions of LD-02, LD-03/04 actual Lux client.
- Accepted normal suites: GigPies243 + Lux89 + Desk82 + Lightdesk81 = 495 Rust
  tests; GigPies37 Python tests; Lux8 terminal checks. All four fmt, Clippy and
  release builds passed. These are historical evidence counts, not new results.
- The real four-process synthetic show passed. Audio remained controllable while
  the owned Lux test child crashed/restarted; Lux recovered the saved look,
  master and blackout under a new epoch, disarmed and without replay.
- All four final workers exited successfully. Both build slots were released.
  Recheck current processes/locks; do not assume they are still unused.
- All reviewed source returned to the original owning repositories on Pi5.
  Product work remains UNCOMMITTED. Desk is an existing unborn repository with
  substantial source; NEVER recreate it. Desk and Lightdesk had no Git remote.
  Existing source and licensed fonts must be preserved.

The planning tables contain dated READY/WAITING entries that predate implementation.
Reconcile current status from code, final handoffs and acceptance before assigning
work. Do not reimplement completed GP-01..03, LX-01..04 or the console clients.
Give me a short concrete done/partial/remaining summary early; do not invent a
percentage for the entire product.

NEXT SUBSTANTIAL MILESTONE

Complete a useful integrated software extension in the following dependency order,
with internal review between producer contracts and consumer implementation:

1. GP-09: implement the planned descriptor-based native role adapter, private
   process-held exclusive ownership and generation-checked handoff. Test with
   synthetic descriptors, owned locks and crash/recovery. Publish the accepted
   versioned role-binding artifact for both surfaces. Real device enumeration,
   opening and dual-controller/display acceptance remain GP-H2.

2. Complete the SOFTWARE portions of DS-02 and LD-02: native frontend entrypoints
   over the existing scenes/font/action models, resize/device-loss recovery and
   bounded separate input/output queues. Wire the REAL accepted provider clients
   into displayed state and operator actions, including stale/unavailable state,
   reviewed confirmations and reconnect/no-replay behavior. Consume GP-09 rather
   than inventing a competing role system. Keep headless/simulator modes explicit.
   Compile the native backend and test rendering/input/provider integration using
   headless or offscreen facilities where available. Review native graphics build
   dependencies and resource budgets before compilation. A simulator gallery alone
   does not complete this task; physical HDMI/MIDI/LED acceptance is still separate.

3. GP-04 then LX-05: provide real bounded named audio analysis subscriptions using
   existing transport/handoff primitives; review E09/artifact before Lux consumes
   it. Exercise real synthetic PCM windows, source identity/map/epoch changes and
   dropped windows. Connect Lux's owned analysis path with honest stale/lost-input
   handling and bounded automation authority. Audio and recording must never wait
   for analysis or lighting, and human holds must remain authoritative.

4. Implement REC-01 then REC-02, FX-01 and PA-01 IN THEIR OWNING REPOSITORIES.
   This includes operation-correlated recorder lifecycle and bounded non-owning
   progress queries, and truthful versioned FX/PA capability/status descriptors.
   Review ABI lifetime/bounds/compatibility before consumers use new artifacts.
   Preserve existing v1 signatures and behavior. Publish exact headers, corpora,
   source revisions and private library hashes for integration.

5. GP-05 plus the justified read-only portion of DS-05: integrate real owner
   libraries into a bounded synthetic graph and expose real recorder/PA/FX health.
   Prove recorder start/stop/finalization and exact recorded samples, fixed wet/dry
   processing into the actual PA path, and truthful unavailable capabilities.
   Do not block this useful fixed-v1 integration on speculative writable PA/FX
   redesign. Do not copy their algorithms into GigPies or the desks. Preserve the
   already qualified host path and run its device-free feature tests when changed.

6. Run an updated bounded integration demonstration with actual accepted provider
   and consumer executables/libraries: real console actions, analysis delivery,
   recorder lifecycle and state, PA/FX capabilities, lighting release/recovery and
   independent failure handling. Keep synthetic functional acceptance distinct
   from physical safety, throughput, scheduler and combined-load acceptance.

Resolve routine software design decisions yourself using owner requirements and
independent review; update agreed contracts centrally before dependent work.
Keep B-NET production remote control/authentication, wider GP-07 channel/scene
expansion, writable FX/PA extensions, physical gates and optional instruments as
separate subsequent increments unless an actual dependency requires a narrow
reviewed change. Do not grow the milestone indefinitely. Conversely, do not stop
after one small schema, a plan, a mock or the first successful worker exit.

COORDINATED EXECUTION

Use four disjoint implementation lanes, adapting their assignments as dependencies
finish rather than keeping idle workers alive:
- Pi5 A: GigPies GP-09/GP-04, then GP-05 integration, with central contracts owned
  by the coordinator and non-overlapping source ownership explicitly assigned.
- Pi5 B: serial owner work in REC/FX/PA and Lux LX-05 when its provider is accepted.
  Change the assigned repository only after its previous scope is handed back.
- Pi4 A: Desk native frontend/role/provider integration, then real module status.
- Pi4 B: Lightdesk native frontend/role/provider integration.
Independent reviewers use the same requested model/effort. Do not oversubscribe
native compilation to make all sessions appear busy. Useful source work and
reviews can run concurrently while heavy commands remain serialized.

Use the pinned SSH aliases and existing direct bounded native-worker workflow.
The previous private runner is:
  Pi5 /home/shome/p/gigpies/user/module-workers/0008/run_worker.py
  Pi4 /home/shome/.local/state/gigpies/module-workers/0008/run_worker.py
Inspect it before reusing it in NEW run directories. It invokes independent
codex exec --yolo -m gpt-6.1-sol -c 'model_reasoning_effort="low"' sessions, records
start/events/result/exit and bounds each pass to 3600 seconds. Verify installed
executable paths/model settings on each host instead of guessing. Do not use
resume --last or inject instructions into an existing interactive session. Do not
bypass gigpies-peer receiver locks or create recursive peer dispatch.

Every assignment needs exact repository/base or manifest, owned files/resources,
accepted inputs, substantial deliverables, required validation, deadline and
handoff. When a pass finishes or times out, inspect children, source, partial
results and ledger before a continuation; never blindly repeat a mutation. Start
the next bounded pass autonomously if required. Keep concise progress updates.

The original Pi5 repositories now own the integrated baseline. The old isolated
GigPies worktree and Pi4 reference tree are not automatically current. Verify the
last handback and any later edits before choosing/reusing isolated checkouts.
Deliver only named reviewed source/artifacts with hashes and receiving acceptance;
do not overlay dirty trees, mirror the workspace or use sibling path dependencies.
Preserve both independently runnable Pi4 sessions and existing interactive users.

On EACH host, every Cargo/check/test/Clippy/release/link command holds the same
parent-held nonblocking /home/shome/p/.gigpies-build.lock. Only ONE build per host,
CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0, Rust +1.97.1, --locked, -j1, normal target
and consistent profiles. Publish fresh build-turn grants/releases; old markers
are not permission. Inspect live children; elapsed reservations do not cancel a
build. Serialize ledger Git transactions with /home/shome/p/.gigpies-ledger.lock.

Recheck free space, memory and output sizes before substantial builds. The last
review retained about10.2GiB in GigPies target with27GiB free on Pi5; Pi4 had36GiB
free. Those are dated observations. Review <20GiB free or >5GiB target, and budget
native GPU dependencies explicitly. No blanket cache/target cleanup, profile
weakening, swap/tuning changes or another session's process termination. Preserve
useful caches/evidence; remove only owned disposable artifacts when idle.

VALIDATION AND REVIEW

Select tests yourself from changed behavior and repository policy. Focused tests
during edits; complete normal production suites for engine/render/schema/routing/
persistence/concurrency/safety/shared-component changes and before publication
where required. Run matching feature checks for native or hardware-host software
without opening physical devices. Keep historical media, auditions, exhaustive
matrices and long benchmarks opt-in unless directly affected. Add meaningful
regressions for actual review findings. Do not keep repeating a passed full suite
for documentation-only changes or byte-identical source handback.

Require independent source review, meaningful tests and real provider/consumer
interoperability before accepting a milestone. Record source/artifact hashes,
contract versions, commands, results and explicit limits. Compiler success alone
does not establish a working UI, engine contract, safety property or integration.
Correct stale owner status/test/dependency descriptions encountered in this scope.

FINAL COMMITS AND PUSHES — EXPLICITLY AUTHORIZED

Finish all implementation, review, handback and required publication checks before
declaring completion. I authorize scoped source commits and pushes for ALL work
belonging to these coordinated planning/implementation threads, including the
already uncommitted task0006/0007/0008 work and this continuation. Do not ask again
whether to commit or push. This supersedes old session-specific documentation
saying no source commit/publication was requested; safety and content boundaries
still apply. Checkpoint commits after review are allowed when useful; at the end
every completed owned change must be on its intended remote.

Publication owner is the coordinator: freeze workers, inspect live Git state,
integrate reviewed peer source into owning repositories without losing local
edits, and review the complete staged content in each changed repository. Include
the twelve owning plans and their scoped index links, even in optional/reference
repositories where no runtime work is due. Do not commit arbitrary unrelated
dirty files under the phrase "all work". Do not delete private files to achieve a
clean status. Retain and identify any unrelated changes left outside the commits.

Follow each repository's publication rules. Enable/version the applicable hooks
without replacing unrelated hooks. In Desk/Lightdesk adopt a reviewed independent
publication policy/guard as their development docs require before first push;
review the bundled font and licence explicitly. Review new scripts and allowlists.
Check complete indexes and outgoing histories, not only changed text. Never publish
user/, node-lab/exchange contents, worker prompts/logs, machine-private state,
credentials, recordings, generated audio, build outputs, copied provider binaries,
downloaded research or one-off runners. No weakened guard or blanket git add .
to make a check pass. Keep reusable synthetic fixtures and reviewed commands only.

Use existing verified remotes and repository branch policies. Fetch first and
inspect divergence; resolve routine integration yourself without force-push,
resetting another person's branch or rewriting published history. Publish
providers before consumers that reference their commits. Preserve original commit
history and lockfiles. No version bump, release tag, binary release, installer or
deployment is requested merely by source commit/push authorization.

Desk and Lightdesk previously had NO remotes. Verify current state and the
authenticated project owner from existing origins/authentication. Use an existing
matching PUBLIC repository under that verified owner when appropriate. If absent,
I explicitly authorize creating PUBLIC shr-desk/shr-lightdesk repositories under
that same owner and configuring their origins so their reviewed source can be
pushed. New product repositories must be public, not private. Do not treat a
private remote as completed publication. Verify any existing namesake belongs to
this exact project; preserve its history and do not overwrite unrelated contents
or change an unrelated repository's visibility. Private coordination records,
worker logs and user data remain excluded from public source. Follow branch protections;
if they require a PR, push the reviewed branch and create the PR, reporting its
actual status without claiming it merged. If authentication/ownership policy blocks
one push, retain its local commits, finish other authorized repositories and
report the exact remaining blocker; do not repeatedly ask for routine approval.

Verify remote branch SHAs after pushing and check relevant CI when present. Fix
task-related failures. Update the private ledger with final revisions, review
acceptance and remaining limits. End with a compact per-repository summary of
implemented/partial/deferred work, tests, commit SHAs, push/PR status, any unrelated
work preserved and any genuine blocker. Clearly distinguish software validation,
integration and hardware verification. I should not have to reconstruct what is
done from worker logs or restart you after each small task.
```
