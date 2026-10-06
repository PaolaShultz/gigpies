# Execute channel processing end to end across both Pis

Paste this prompt into the coordinating session on Pi5. Creating this file does
not start workers, authorize physical operations in the current session, or imply
that the implementation below is already complete.

```text
Work from /home/shome/p/gigpies on rpi5. Implement the complete first GP-07
channel-processing increment: real channel EQ and dynamics, authoritative engine
control and readback, and usable SHR Desk editing/confirmation/feedback through
the real provider. Coordinate both Pis as useful and carry the work through
implementation, independent review, validation, scoped commits/pushes, CI and
final canonical source synchronization. Do not stop at a plan, DSP-only library,
simulated UI, contract document or compilation success.

MODEL REQUIREMENT

Keep the coordinator's selected model. Every delegated worker, including code,
review, test and remote peer workers, MUST use GPT-6 Astra, model identifier
`gpt-6-astra`, reasoning effort `low`. This overrides historical Sol6.1 worker
instructions. Do not silently substitute models or inherit an unspecified model.
For built-in subagents pass both model and reasoning explicitly with a context
mode that permits overrides, and supply a complete bounded task brief.

For CLI workers inspect the installed executable/help and actual peer runner on
both hosts before dispatch. The installed gigpies-peer runner at preparation time
did not expose model/reasoning arguments. Merely putting the model name in a prompt
does not configure it. Use verified per-invocation model/reasoning overrides. If
needed, make a reviewed task-private adaptation of the existing runner, preserving
pinned SSH, receiver exclusion lock, timeout/owned-process cleanup, --yolo mode,
private logs and exact task scope. Do not change global model defaults, credentials,
existing interactive sessions or system services. Record launch arguments and
available runtime model metadata. If the requested model is unavailable, report
that exact problem; continue coordinator-owned independent work without silently
launching a different worker.

RECOVER CURRENT STATE

1. Read /home/shome/p/AGENTS.md and each activated owner's AGENTS.md. Read GigPies
   README.md, docs/STATUS.md, DEVELOPMENT.md, PUBLICATION.md, COMPONENTS.md,
   MODULE_IMPLEMENTATION_MAP.md, MODULE_IMPLEMENTATION_PLAN.md,
   MODULE_CONTRACTS.md, AUDIO_CONTROL_WIRE.md, AUDIO_RENDERED_WIRE.md,
   AUDIO_LOCAL_SERVICE.md, MODULE_GRAPH.md, ANALYSIS_STREAM.md and
   HEADLESS_INTEGRATION.md. Read Desk's current blueprint, implementation plan,
   native frontend, development and publication documents. Follow source callers
   and existing reusable GigPies EQ/dynamics implementations before designing.
2. Recover completed task0011 from the private ledger and
   user/daily-review/0011/REVIEW.md, final-revisions.json, final-ledger.json and
   validation summaries. The October4 review is complete; old prose saying it has
   not run is stale. Historical root launch cards and old worker worktrees are not
   current assignments. Do not restart completed GP01..05/GP09 or console work.
3. Inspect both hosts' identity, canonical heads/branches/remotes, dirty work,
   active ownership/processes, build locks, free disk and target sizes. Fetch
   verified origins and preserve any later edits. Reserve the next unused ledger
   task ID under its lock; record exact baseline heads and file/resource ownership.
   Use a fresh ignored private task directory. Correct narrowly relevant stale
   handoff/status prose as part of this increment.

PRODUCT SCOPE AND SIGNAL CONTRACT

Complete one coherent processing/control path applicable to all eight current
mono input strips, with one channel used as the first implementation/acceptance
slice. No channel-count expansion is required. Reuse existing GigPies-owned DSP;
PA algorithms remain in shr-pa and FX algorithms in shr-fx. No sibling path
dependencies, copied module algorithms or new parallel processing engine.

Before writing interdependent code, coordinator and a separate Astra-low reviewer
must accept a concise contract based on existing DSP and UI capabilities:
- A useful bounded channel EQ and compressor/dynamics parameter set, units,
  ranges, bypass, neutral defaults and explicit output/makeup gain semantics.
  Prefer existing proven algorithms. Do not expand into every gate/expander,
  multiband processor, sidechain route or complete console blueprint.
- Exact order of raw taps, EQ, dynamics, mute, FOH fader/pan and monitor sends.
  Preserve raw REC and named raw analysis samples byte-for-byte. Preserve current
  monitor behavior by default; new FOH processing must not silently alter monitor
  sound. Explicitly version and test any advertised processing tap. Do not add
  an unrequested monitor tap selector just to broaden scope.
- Sample-rate/block bounds, coefficient/state preparation, bounded transitions
  and reset behavior. Neutral/bypass must preserve the established reference;
  retargeting must be continuous and predictable. No hidden normalization, makeup
  gain or lookahead latency. Any unavoidable latency must be explicit and tested.
- Capability/version negotiation and strict schema; stable channel/parameter IDs;
  current/target/bypass/readiness and meaningful dynamics gain-reduction feedback.
  Unsupported or unavailable measurements must be labelled, not fabricated.
- Existing scoped lease, revision, shared deduplication, pending/final boundary,
  request-ID reuse, expiry, role-loss and reconnect semantics. Define processing
  authority scope explicitly. Successful transport delivery is not DSP application.
  Old clients/providers must remain compatible or refuse unsupported capabilities
  explicitly; never reinterpret existing messages silently.

Freeze the accepted contract and provider fixture version before consumer work.
Resolve ordinary design choices autonomously from current source and established
behavior. Ask only for a genuinely necessary product decision; finish independent
work while waiting. A review rejection requires a concrete revision, not weakened
requirements or an indefinitely expanding design phase.

COORDINATION AND OWNERSHIP

Use Pi5 for coordination, central contract/engine ownership and final integration.
Use Pi4 for a disjoint Desk implementation lane once the provider contract is
accepted, and for bounded software review/tests when useful. A reviewer can inspect
contracts/source while the provider lane implements; Desk can prepare UI behavior
against the frozen interface but cannot claim completion using mocks alone.

Create explicit bounded assignments: task ID, host, repo/base SHA, exact owned
files, accepted contract hashes, allowed operations, deliverable, validation,
resource slot and stopping conditions. Keep one writer per file/worktree. Only the
coordinator edits shared central contracts and acceptance records. Workers may
make scoped local commits for handoff; only the coordinator publishes to verified
upstreams after review. Workers must not recursively dispatch.

Use direct bounded peer workers through pinned gigpies-pi4/gigpies-pi5 SSH and the
inspected runner protocol. Preserve the one peer receiver per host rule and all
existing interactive sessions. Prefer a clean unowned canonical checkout; use
an isolated checkout when active ownership requires it. Never reset, overlay or
reuse old task worktrees as new active lanes. Do not duplicate builds/caches without
reviewing disk/resource impact. Deliver exact commits or named patches with SHA256;
receiver verifies base, content and applicability and acknowledges acceptance.
A pushed handoff is delivery, not acceptance. Timeouts retain logs and partial work;
inspect before resuming, and never blindly repeat a mutation.

Keep a dependency-ordered queue, durable progress/findings/commands/artifact hashes
and immutable per-node records under the ledger lock. Dispatch useful independent
work rather than keeping workers idle solely to preserve a rigid wave structure.
Integrate provider before consumer and review the integrated result independently.

IMPLEMENT THE WHOLE PATH

A. Engine: integrate actual per-channel processing into the existing mixer with
   neutral defaults, prepared validated state, bounded transitions and truthful
   status. Audio callbacks must have bounded work and no allocation/deallocation,
   locks, I/O or potentially blocking destruction. Reuse the existing ownership
   and command-boundary patterns; retire prepared state off the audio path.
B. Authority/transport: extend the real local provider and versioned contract,
   preserving authorization, shared retry history and frame-accurate application.
   Invalid/nonfinite/out-of-range/oversized requests leave processing and revision
   unchanged. Reject incomplete atomic edits without partial application.
C. Desk: provide real channel selection, EQ/dynamics editing, bypass and explicit
   apply/cancel behavior appropriate to the established UI. Use existing semantic
   actions and controller abstraction, not a second special-purpose UI. Show
   provider-confirmed values, pending/failed state and gain-reduction/processing
   status with honest units. Keep stale state visibly stale. Bank/page/channel
   changes, role loss, disconnect and uncertain completion must not replay edits
   or confirm the wrong context. Keyboard and injected-controller actions must
   reach the same engine authority. Do not open physical MIDI or displays.
D. Integration: exercise the actual Desk frontend against a freshly built actual
   GigPies provider and owner libraries. Verify operator action -> request ->
   authority -> boundary application -> changed samples -> confirmed readback ->
   visible state. Include a second channel to prove isolation, monitor independence,
   raw REC/analysis invariance, default compatibility and explicit reconnect with
   no unintended replay. Do not finish after only an independent DSP or UI test.

VALIDATION AND RESOURCE LIMITS

Own test selection. Use focused meaningful regressions during implementation,
then complete normal production suites for changed engine/render/schema/authority/
frontend owners. Run required format, Clippy, ABI, release and publication checks,
matching hardware-host/native device-free features. Run dependent-owner checks
where their behavior is affected; do not rebuild every optional project by habit.

Include independent EQ frequency/impulse references, compressor static/attack/
release behavior, finite/range/extreme-value handling, partition equivalence,
neutral/bypass behavior, transition/retarget/reset behavior and real-time allocation
checks. Verify authority failures, duplicate/stale/cross-contract IDs, late replies,
queue saturation, lease/role loss, confirmation freshness and recovery. Do not
write assertions that merely repeat implementation assumptions or normalize away
meaningful errors. Physical audio quality is not established by passing software.

Use synthetic PCM, private temporary UDS endpoints and owned test children only.
Adapt the previously reviewed private joint harness if useful; preserve its original
and verify actual artifact hashes. Confirm exact raw PCM24 stems, independent
monitor/analysis paths, REC lifecycle and existing FX/PA order alongside processing.
Join children before removing scratch. Record concise results and reproducible
commands rather than large generated audio or event-log collections.

Every Cargo/check/test/Clippy/release/C-link command on each host must hold that
host's parent-held NONBLOCKING /home/shome/p/.gigpies-build.lock. One build at a
time per host, CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0, Rust +1.97.1, committed
Cargo.lock, --locked and -j1 where supported. The Pis may build independently when
both have valid ownership/slots. A busy slot means independent work then retry;
never queue hidden blocking builds or treat elapsed reservations as cancellation.
Check disk and target sizes before substantial builds; review below20GiB free or
above5GiB target. Do not weaken debug information or validation to save space.

For native rendering use CPU/headless checks without opening operator windows.
Pi5 lacked the lavapipe CPU ICD during task0011; inspect both hosts rather than
assuming it now exists. Prefer the host with an available approved CPU backend.
Do not silently use a physical GPU, install/change host graphics configuration or
claim an unavailable check passed. Finish independent software checks and report
an exact environmental limit if no permitted backend exists.

Historical media/auditions, exhaustive research matrices, long benchmarks and
physical/shared-load experiments remain opt-in and out of scope. Report skipped
classes explicitly. Clean only this task's known disposable artifacts after
checking active users; preserve existing shared caches, recordings, private state,
unique evidence, required executables and unrelated work.

AUTHORIZATION WHEN THIS PROMPT IS EXECUTED

I authorize coordinated software implementation, delegated Astra-low workers on
both Pis, scoped fixes in GigPies and SHR Desk, focused dependent-owner changes
only if a demonstrated compatibility issue requires them, synthetic/temp-socket/
CPU-headless checks, reviewed commits and pushes, and final source synchronization.
Other modules remain read-only absent such a necessary scoped compatibility repair.

No physical audio/MIDI/DMX, playback, operator-display windows, hardware enumeration
that opens devices, service deployment, host audio/network/display tuning, shared
load experiment, tags, version bump, binary release or repository-visibility change.
Production remote authentication, writable PA/FX ABI expansion, scenes/PFL/routing
expansion, optional instruments and full standalone recorder/player remain separate
milestones. This task completes channel EQ/dynamics end to end, not all of GP-07.
Do not expand it opportunistically or require those separate milestones first.

FINISHING CRITERIA

Continue autonomously until the agreed processing path is implemented, independently
reviewed and tested through the actual operator/provider/audio chain. Fix confirmed
in-scope defects; retain failures and distinguish unresolved hypotheses. Update
focused owning contracts, status, implementation plan and handoff, preserving
original drafts and dated evidence. Do not label software acceptance as hardware,
listening or combined-load acceptance.

Inspect live Git state and complete staged indices; stage only named owned files.
Use existing versioned publication hooks/guards, review any new reusable scripts
and outgoing history, and exclude private data/binaries/one-off runners. Commit
coherent owner changes and push to verified origins in dependency order. Check
remote SHAs and relevant CI; repair task-related failures. Do not force-push,
change unrelated commits or make empty commits in unchanged owners.

Finally safely fast-forward canonical checkouts on both Pis to exact final
published revisions, preserving Pi4's verified Pi5-backed origins. Never push
through Pi4 origins into Pi5's checked-out branches. Verify clean matching heads
for the twelve canonical projects; preserve dirty unrelated work instead of
forcing a match. Synchronize the private ledger, record final acceptance and
artifact/test/source manifests. Source synchronization is not deployment.

Final report: implemented processing and signal order; operator workflow; fixes
and limitations; tests run/skipped; actual-provider/sample evidence; model/effort
used by every worker; per-owner commits/push/CI; both-Pi source/ledger sync; disk
impact and preserved work. If a real external blocker remains, finish independent
work and state precisely what prevents the remaining acceptance. Do not stop at a
proposal or return routine coordination decisions to me.
```
