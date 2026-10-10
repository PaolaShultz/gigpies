# Coordinating prompt: module plans and parallel implementation

Run the prompt below in a new planning session rooted at `/home/shome/p/gigpies`.
It authorizes analysis and documentation across the relevant module repositories,
not implementation or physical tests. The result must be usable by independent
implementation sessions, including two sessions on the Pi without NVMe.

```text
Work from /home/shome/p/gigpies as the GigPies planning and integration coordinator.

Analyze the actual system and its module repositories, then WRITE one actionable
GigPies implementation plan in each relevant owning repository. Also write the
central dependency/interface map and a concrete two-Pi parallel work schedule.
Finish the plans and launch instructions; do not stop at a proposal in chat.

I will subsequently open separate Codex implementation sessions in the individual
projects, using less expensive models. Make the plans specific enough that those
sessions can execute bounded tasks without independently redesigning the system.
The other Pi has no NVMe but is available for substantial software work. Plan for
at least TWO independent implementation sessions on that Pi as well as useful
local work. Verify resources and separate concurrent sessions from concurrent
heavy builds; do not promise a workload the hardware has not demonstrated.

1. SCOPE AND WORKING AGREEMENTS

Read /home/shome/p/AGENTS.md, the current repository's AGENTS.md, README.md,
docs/STATUS.md, docs/architecture/ARCHITECTURE.md, docs/architecture/COMPONENTS.md, docs/architecture/BRAIN_CONSOLES.md,
docs/archive/plans/NEXT_SESSION.md, docs/development/NODE_LAB.md and docs/PUBLICATION.md. Then read each
relevant sibling's AGENTS.md, README, current status, owning blueprint/roadmap,
control contracts and implementation/test evidence. Inspect code where needed
to establish whether a documented capability actually exists.

This prompt explicitly authorizes documentation-only writes in GigPies and the
relevant ../shr-* module repositories: their implementation plans and the minimal
index/status links needed to make those plans discoverable. Preserve unrelated
roadmaps and uncommitted work. Archive superseded owning drafts before replacement.
Do not implement runtime code, create speculative crates, change dependency
versions, configure devices/services/displays or run physical/load experiments.
Do not install tools or download private media merely to write a plan.

Check the actual hostname, live Git status, source revision and active ownership
before writing. Do not assume this session is on a particular Pi. Preserve all
interactive sessions. Use the established pinned SSH aliases and private exchange
ledger; refresh clean clones with --ff-only and never overwrite dirty clones.
Do not reset, stash away, clean or rebase another session's work.

Read-only peer inventory and bounded read-only analysis/review tasks are authorized
through the installed gigpies-peer helper. Read its current --check/help and obey
the parent protocol, including its installed execution policy. Assign exact scope,
revision and timeout. Do not launch implementation workers during this planning
session or send tasks into existing interactive sessions. Coordinate ownership
before documentation writes in any actively edited checkout.

No public push, release, version bump or broad source commit is requested by this
planning prompt. Leave requested plan files in their owning workspaces and report
them. Authorized private coordination records can follow the ledger protocol.
Keep machine inventory and peer prompts/logs private; public plans need resource
classes and reproducible interfaces, not private device configuration.

2. PRESERVE THE PRODUCT AND ENGINE BOUNDARIES

GigPies is the COMPLETE system and integration owner. It includes human-operated
audio AND lighting consoles. Automation is a mode, not the whole product.

The intended Brain has two 1920x1080 monitors and two separately assigned MIDI
keyboard controllers: primarily one audio pairing and one lighting pairing.
SHR Desk owns audio UI. SHR Lightdesk owns lighting UI. SHR Lux owns lighting
analysis, fixture evaluation, programmer/cue/effect authority and physical output.
Lightdesk must not become a second lighting engine.

Stagebox owns physical audio I/O, the local mixer/monitor path, PA protection and
local recording. SHR PA owns speaker processing/measurement/alignment, SHR FX
owns effects, and SHR REC owns recording capabilities. The complete band mixer
is distinct from SHR PA's speaker processor; use the actual GigPies ownership
map rather than assigning the whole mixer to PA or DAW by name.

Keep live callbacks bounded and free of allocation, locks and I/O. Lighting/UI
failure must not block essential audio, recording, protection or audio control.
Keep MANUAL/ASSIST/AUTO, deliberate takeover/release, actual versus proposed state,
separate controller/LED ownership and reconnect recovery explicit. Lighting
attributes need lighting arbitration, not copied audio fader semantics.

The qualified stereo bench, surface mocks and current packet/control/host ABIs
are useful existing work. Reuse them where appropriate; do not relabel a test
scalar as a complete control API, a mock as a finished engine, or a preview as
physical output. Preserve recorded failures and limits. Three-way PA, monitors,
phone controls and richer show features remain subject to actual owner capabilities.

3. INVENTORY MODULES AND THEIR REAL GAPS

Start with GigPies, shr-desk, shr-lightdesk, shr-lux, shr-pa, shr-fx and shr-rec.
Inspect the other projects named in COMPONENTS.md, including shr-daw and relevant
instrument modules. Decide from actual consumers whether each is a core runtime
module, supporting dependency, optional later integration or reference-only project.
Do not turn every sibling directory into a new required runtime component.
The media library and development-skill repositories are not product engines.

For every actual module, produce one owning plan. For optional/reference modules,
make the immediate scope explicitly small or deferred with an activation condition;
do not invent work just to keep a session busy. List excluded non-module projects
and the reason in the central inventory so none is silently forgotten.

Record per module: repository/path, current HEAD, relevant dirty-state caveats,
source files/symbols inspected, current owner docs, working capabilities, mock-only
behavior, missing contracts, hardware assumptions and existing test commands.
Distinguish planned, implemented, offline-validated, integrated and hardware-verified.
Documentation alone is not evidence of implementation or fresh physical acceptance.

Read and preserve existing plans before adding another. Place the GigPies-specific
implementation plan in the repository's existing plan/doc convention and link it
from the owning index/status. If there is no convention, use
docs/plans/GIGPIES_IMPLEMENTATION.md. For a numbered-note repository, use its next
available note and index rather than imposing a second documentation system.
If a current plan already owns exactly this scope, update it coherently instead
of creating a competing plan. Record the exact entry-point path centrally.

4. RESOLVE CROSS-MODULE CONTRACTS BEFORE ASSIGNING DEPENDENT WORK

Create docs/reference/MODULE_CONTRACTS.md in GigPies, or extend an existing exact owner if
one exists. Give each required interface a stable ID, version/status, authoritative
owner, producers/consumers, normative document and corresponding module tasks.
Reuse current transport and versioned owner ABIs. Avoid a large generic framework.

For each near-term interface, state concrete data/command/event examples, units,
identities, capabilities, bounds, timing basis, revision/epoch rules, acknowledgement
and rejection behavior, retries, ownership, reconnect and failure semantics.
Specify persistence/version compatibility where relevant. Document how consumers
will test against the same agreed examples and how provider acceptance is proven.
Do not implement schemas or test fixtures during this documentation-only pass.

Cover the actual first consumers: audio controls and observations, lighting
capabilities/programmer/playback/source attribution, recorder control/status,
FX parameter/lifecycle behavior, shared show identity, controller/display roles,
and named audio-analysis subscriptions. Keep optional cross-system cues explicit.
Do not freeze a giant speculative protocol for features with no immediate consumer.

Resolve decisions supported by the code and existing requirements now. Mark each
contract as ready-for-implementation or unresolved, with an exact blocking question,
decision owner and affected task IDs. Do not send two workers off to invent opposite
ends of an interface. A documentation decision is not an implemented engine contract.
Mocks must be clearly separate from production adapters and have replacement gates.

5. MAKE EACH MODULE PLAN EXECUTABLE

Each plan must contain:

- Its objective in GigPies, owning boundary and explicit exclusions.
- Current implementation/evidence and source baseline, including uncommitted work.
- The first useful end-to-end milestone and subsequent ordered small milestones.
- Concrete tasks with stable IDs, priority, one owner, likely files/symbols to
  change, required inputs, expected outputs and measurable completion criteria.
- Exact dependencies as provider/task/contract IDs plus required artifact/version
  and acceptance condition. Distinguish READY, WAITING and DEFERRED tasks.
- Meaningful normal test commands and expected checks, separate optional research,
  hardware and combined-load gates. Use the pinned toolchain/lockfile, independent
  builds without sibling path dependencies, and CARGO_INCREMENTAL=0.
- Failure/refusal, stale-state/reconnect and recovery cases appropriate to that
  module. Identify engine-owned safety rules and the tests that protect them.
- A node/resource assignment, build/storage budget and fallback independent work
  if a provider is unavailable. No unbounded render/research workload.
- Review/handoff artifacts: changes, exact revision or source manifest, contract
  compatibility, checks and evidence limits, plus the next owner/action.
- A short implementation launch prompt that names this plan and the first READY
  task, authorizes only this repository's scoped implementation, and instructs
  the worker to check current state before edits and keep progress in the plan.

Break difficult architecture into decisions/examples rather than vague instructions
to a cheaper model to "design a robust system." Identify tasks needing stronger
design review, but do not assign model names/prices or change any session's model.
Make each initial task small enough to inspect, test, commit independently and
recover after interruption. Subsequent tasks may continue only within agreed scope.

Every launch prompt must prohibit opportunistic sibling writes and unilateral
contract changes, preserve other sessions, and clearly stop before unapproved
physical operations. It should continue useful independent work when blocked,
report exact dependency mismatches, and never mark mocked/incomplete work done.

6. PLAN REAL PARALLEL WORK ON BOTH PIS

Create docs/archive/plans/PARALLEL_WORK_PLAN.md and a central entry point
docs/MODULE_IMPLEMENTATION_MAP.md. Use a small dependency graph and concrete work
waves, with at least two simultaneously READY implementation tasks where the
inspected system permits. Do not force independent UI/model/test work to wait
for a future device driver; do not pretend dependent integration can start early.

Inventory both hosts read-only: RAM, free space/storage type, CPU, toolchain,
available checkouts/revisions, running builds and existing ownership. The other
Pi has no NVMe; verify what storage it actually has rather than calling it unusable.
Keep exact host inventory in the private ledger and link the evidence record.

The peer must have TWO distinct implementation lanes, for example two separately
opened Codex sessions in different module repositories or explicitly isolated
checkouts. Choose the actual pair from dependency/resource evidence, not a fixed
guess. Pair them with independent local work and give all lanes exact first tasks.
Plan at least two active sessions overall even if the peer is temporarily unreachable;
record its two-lane assignment as unverified until its resource check succeeds.

The installed gigpies-peer helper permits only ONE bounded receiver per node.
Do not remove/bypass its lock, run two competing helper receivers, or claim that
one helper call creates two sessions. Two user-started interactive peer sessions
are separate from that helper and must have disjoint file/resource ownership.
Give me exact host, cwd, plan path and pasteable prompt for each session. The helper
can do bounded reviews/handoffs without taking over those sessions' owned files.

Concurrent reasoning/editing is different from simultaneous heavy compilation.
Define a lightweight shared build-slot/reservation procedure using the existing
ledger, based on available memory and storage. Start with one heavy build per
resource-constrained host unless evidence supports more; set sensible per-command
Cargo job limits without reducing test coverage/debug information. Workers should
edit/review another READY task while waiting instead of both exhausting RAM/disk.
Do not build a new scheduler, daemon or orchestration framework for this.

Good peer candidates include bounded Rust implementation, controller/state logic,
pure contract and recovery tests, UI interaction models and focused synthetic DSP
checks. Assign based on actual dependencies. Lack of NVMe rules out claiming NVMe
recording throughput acceptance, not implementing or unit-testing recorder logic.
Keep large media, full-show renders, storage-heavy artifacts and NVMe-specific
acceptance on appropriate hardware. Native GPU/display and actual-device tests
remain separately reserved gates; do not infer them from a headless build.

For every lane record: host/resource class, repository and exact work area,
source baseline, task IDs, owned files, dependencies, estimated RAM/disk class,
build/test slot needs, outputs, handoff/reviewer and the next READY fallback task.
No two sessions may own the same worktree files or contract definition concurrently.
Use normal target directories consistently and avoid copying build caches/media.

Specify how a peer obtains the exact source and plan. Prefer reviewed commits;
where needed source is uncommitted or a repo has no HEAD, use a bounded isolated
source snapshot/patch plus file hashes, preserve the live checkout and document
that limitation. Never silently substitute an older public revision or mirror a
whole home/workspace. A delivered patch is not accepted until the receiver reviews
it and posts the separate acknowledgement required by the ledger.

If the peer is unreachable, retain the bounded connection error and continue all
local planning. Produce the peer launch cards with exact prerequisites and honest
unverified status; do not ask me to diagnose it or invent inventory results.

7. VALIDATE AND FINISH THE PLANNING DELIVERABLES

Cross-check all plans together: each task has one owner, dependencies resolve to
real tasks/contracts, provider and consumer examples agree, no circular first-wave
dependency exists, no engine is duplicated, and the proposed concurrent lanes
have disjoint writes and viable resource reservations.

Verify each plan is actually written in the owning repository and linked from
its current docs. Check paths, anchors, whitespace and publication boundaries
where applicable. Do documentation-appropriate validation; do not rerun all
production suites, historical studies or hardware tests just to author plans.
Record source evidence reviewed separately from checks actually run.

Leave module status honest: planned work is not implemented by writing it down.
If some real decision requires my answer, finish independent plans and identify
the exact question and blocked task IDs. Routine design/test selection belongs
to you; do not front-load a questionnaire or ask me to choose test classes.

Finish with:
1. An index of every module and its actual plan path/status.
2. The agreed ownership/contracts and unresolved blockers.
3. The first parallel wave: exact local and TWO peer session launch cards,
   independent tasks, source availability and build-slot arrangement.
4. The later dependency/integration order and separate physical acceptance gates.
5. Concise validation, changed files and preserved working-tree caveats.

Do the analysis and WRITE all plans now. Stop after the planning/handoff deliverables;
I will start the implementation sessions separately.
```
