# Execute four-band fully parametric channel EQ across both Pis

Paste this prompt into the coordinating session on Pi5. Creating this file does
not start workers, authorize physical operations in the current session, or imply
that the implementation below is already complete.

```text
Work from /home/shome/p/gigpies on rpi5. Continue the completed task0012 GP-07
processing implementation by replacing its three-band shelf/bell/shelf EQ with
FOUR FULLY PARAMETRIC EQ BANDS on every one of the eight current mono inputs.
Each band must independently expose frequency, gain and Q and support bell/peaking
operation across the full supported frequency range. The first and fourth bands
must not be restricted to shelves. Preserve the existing working compressor and
complete the real engine -> authority -> provider -> SHR Desk operator path.
Coordinate both Pis as useful and carry the work through
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
2. Recover completed task0012 from the private ledger and
   user/channel-processing/0012/REVIEW.md, final-revisions.json, final-ledger.json,
   desk-accepted.json, workers.json, joint-acceptance-final.json and validation
   summaries. Read docs/CHANNEL_PROCESSING.md and the actual implemented source:
   src/channel_processing.rs, processing_wire.rs, mixer.rs, mixer_control.rs and
   local_audio.rs; Desk's processing.rs, audio.rs, local_audio.rs and frontend.rs.
   The completed published checkpoint is GigPies
   874cff2dede9ae19bc0b19dc4431d412e490e6b4 and Desk
   1d6c81b8a79901b1b04e9d8d2e39b1b1fab61dde (implementation 84e40f9).
   Final synchronized ledger was 6bc7ecb7d83443df5881d7d61e0658b875d52d2e.
   These are recovery references, not permission to reset later work. Task0011
   and task0012 are complete. Reserve the next unused task ID; do not reopen them.
   Task0012 proved the actual chain with 1693 exact blocks, eight 80976-frame raw
   PCM24 stems, independent monitor/analysis/FX/PA comparisons, positive compressor
   GR and reconnect without replay. Its final default/native Desk suites were
   111/113; GigPies default/hardware-host were 297/320. Counts are historical
   evidence, not targets that permit dropping tests. Preserve original prompts.
   Recover the paired-read freshness and original-revision pinning repairs; do
   not regress them. Historical launch cards/worktrees are not current ownership.
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

The product correction is four independent parametric bands, not three fixed
bands plus an extra shelf or a UI-only fourth control. All four must have their own
frequency, gain and Q, stable band identity and independently testable DSP effect.
Keep global EQ bypass and add useful per-band bypass so individual corrections can
be auditioned through explicit operator actions. Zero gain/neutral and bypass must
preserve the established reference exactly. Existing ranges (20..20000 Hz,
+/-12 dB gain, Q0.1..10 with existing integer units/steps) are the starting point;
resolve ordinary details through owner/reviewer evidence, without asking routine
product questions. Bands may cross in frequency without silently sorting their
identities. Reuse the existing GigPies bell biquad implementation. Selectable shelf
shapes are optional future work, not a prerequisite for four fully parametric bands.
Do not expand compressor features, monitor taps, channel count, scenes or routing.

This changes an already published strict processing schema. Introduce an explicit
new processing capability/schema version, with a reviewed compatibility policy.
Never reinterpret GP07-processing:1 fields as four bands, fabricate old readback,
or silently approximate old shelves with bells. Preserve legacy processing exactly
only if the chosen compatibility path can represent it truthfully; otherwise give
an explicit safe unsupported-version refusal. Ordinary legacy GP03 clients must
continue working. Keep shared writer/lease/revision/request-ID and retry history
across supported versions, and test cross-version reuse/refusal. Freeze a NEW
producer fixture version from actual provider execution before consumer acceptance;
retain the original v1 corpus as compatibility evidence. Update tests/classification
according to which behavior remains supported; do not delete inconvenient evidence.

Before writing interdependent code, coordinator and a separate Astra-low reviewer
must accept a concise contract based on existing DSP and UI capabilities:
- Four independent bell-capable parametric EQ bands with frequency/gain/Q and
  bypass, stable band IDs, integer units, bounds and neutral defaults. Preserve
  existing compressor/dynamics parameters and explicit makeup gain semantics.
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
and immutable per-node records under the ledger lock. Plan bounded worker handoff
before the receiver deadline: save exact source/commit, accepted artifact hashes,
completed checks and remaining commands while time remains. Task0012's Pi4 worker
saved a valid handoff but reached its one-hour limit before the final response;
recover such artifacts explicitly, never assume a timeout rolled back work. Reuse
warm caches across hosts where appropriate instead of starting doomed cold builds. Dispatch useful independent
work rather than keeping workers idle solely to preserve a rigid wave structure.
Integrate provider before consumer and review the integrated result independently.

IMPLEMENT THE WHOLE PATH

A. Engine: integrate all four real parametric bands into the existing mixer with
   neutral defaults, prepared validated state, bounded transitions and truthful
   status. Audio callbacks must have bounded work and no allocation/deallocation,
   locks, I/O or potentially blocking destruction. Reuse the existing ownership
   and command-boundary patterns; retire prepared state off the audio path.
B. Authority/transport: extend the real local provider and versioned contract,
   preserving authorization, shared retry history and frame-accurate application.
   Invalid/nonfinite/out-of-range/oversized requests leave processing and revision
   unchanged. Reject incomplete atomic edits without partial application.
C. Desk: expose all four bands with usable frequency/gain/Q/bypass editing, retain
   the real channel selection and compressor controls, and provide explicit
   apply/cancel behavior appropriate to the established UI. Use existing semantic
   actions and controller abstraction, not a second special-purpose UI. Show
   provider-confirmed values, pending/failed state and gain-reduction/processing
   status with honest units. Keep stale state visibly stale. Bank/page/channel
   changes, role loss, disconnect and uncertain completion must not replay edits
   or confirm the wrong context. Keyboard and injected-controller actions must
   reach the same engine authority. Keep all four bands readable and editable in
   the existing native scene, including a complete protected review of the atomic
   edit. Group band controls sensibly; do not overflow the scene or hide unreviewed
   changes behind confirmation. Verify actual rendered layout and resize behavior.
   Do not open physical MIDI or displays.
D. Integration: exercise the actual Desk frontend against a freshly built actual
   GigPies provider and owner libraries. Verify operator action -> request ->
   authority -> boundary application -> changed samples -> confirmed readback ->
   visible state. Exercise frequency, gain and Q on EACH of the four bands, not
   merely a config echo or a fourth field in a fixture. Demonstrate band independence
   and nontrivial changed samples with independent references, single-band and
   combined settings, global/per-band bypass and neutral compatibility. Include a
   second channel to prove isolation, monitor independence,
   raw REC/analysis invariance, default compatibility and explicit reconnect with
   no unintended replay. Do not finish after only an independent DSP or UI test.

VALIDATION AND RESOURCE LIMITS

Own test selection. Use focused meaningful regressions during implementation,
then complete normal production suites for changed engine/render/schema/authority/
frontend owners. Run required format, Clippy, ABI, release and publication checks,
matching hardware-host/native device-free features. Run dependent-owner checks
where their behavior is affected; do not rebuild every optional project by habit.

Include independent frequency/impulse references for all four EQ bands and their
cascade, varying Q and frequency, crossed band frequencies, summed extreme gains,
per-band/global bypass, and unchanged compressor static/attack/
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
Task0012 used Pi4's lavapipe CPU ICD because Pi5 lacked it. Inspect both hosts
again rather than assuming availability. Prefer the approved CPU backend host.
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
milestones. This task completes four-band fully parametric channel EQ end to end while
preserving the accepted dynamics path; it does not complete all of GP-07.
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

Final report: all four bands and their independent controls; schema/legacy policy;
signal order; operator workflow; fixes and limitations; tests run/skipped; actual-provider/sample evidence; model/effort
used by every worker; per-owner commits/push/CI; both-Pi source/ledger sync; disk
impact and preserved work. If a real external blocker remains, finish independent
work and state precisely what prevents the remaining acceptance. Do not stop at a
proposal or return routine coordination decisions to me.
```
