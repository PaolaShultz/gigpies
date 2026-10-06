# Execute the modular processing-engine, capacity and Brain integration increment

Prepared 2026-10-05 after inspecting the original product blueprint, current
GigPies/SHR PA/SHR Desk contracts and implementation, and task0013 acceptance.
Creating this file does not execute it, start workers or authorize physical I/O.
The execution brief below takes effect when the user explicitly invokes it.

## What the inspection established

The original requirement was not an eight-input product. The preserved
[blueprint, sections 3–4](archive/blueprints/blueprint-v2.md) specifies UMC1820 +
ADA8200, **16 analog inputs / 18 analog outputs at 48 kHz**, flexible output
allocation, no hard-coded monitor count, and roughly 4×8-style PA routing.
The latest user direction makes 16 inputs the minimum, with 32 and 48 important
growth targets, and includes PA in the modular processing engine.

The eight-input graph entered the published implementation in `b3c9f64`
(2026-10-04), under the smaller GP-03 milestone in
[the implementation plan](MODULE_IMPLEMENTATION_PLAN.md). It subsequently spread
into strict schemas, recorder maps and Desk decoders. Task0013's execution prompt
explicitly retained eight inputs; that scope explains the last increment but does
not replace the original product requirement. Existing work is still present;
its separate prototypes and restricted adapters have not been fully integrated.

| Area | Actual current state | Missing integration |
| --- | --- | --- |
| Mixer and channel controls | `src/mixer.rs` fixes eight mono inputs; four parametric bands/compressor per input, stereo FOH and two monitor sends. `control_model.rs`, `processing_wire.rs`, `module_graph.rs` and Desk contain related count assumptions. | Configurable channel/bus/output capacity, physical patch map and consistent larger schemas/UI. |
| Modular PA | `shr-pa` has real standalone 2×6 processing and controls. Its embedded v1 API fixes full-range L/R on outputs 0/1 and silences 2–5; GigPies consumes it and exposes health read-only. | Configurable prepared embedding, real crossover/output routing and operator control, scalable PA topology. |
| Network audio | GPA1 implements grouped PCM24 analysis and float32 FX sends/returns, deadlines, epochs, bounded queues and recovery. A prior two-Pi synthetic design run covered 64 analysis + 16 send/return channels. | Negotiation and integration with the current larger mixer/authority and actual Brain FX path. Network capacity is not mixer or physical-channel acceptance. |
| Remote commands | GPH1/GPC1/GPK1 prototype controls one test scalar. Actual mixer/processing commands use the private same-UID Unix provider. | Authenticated Brain-to-processing-node commands, current capabilities/readback and actual boundary completion over the network. |
| Physical host | Separate qualified stereo USB bench with actual PA/FX/REC libraries. | One multichannel engine shared by synthetic and device adapters; 16/18 physical profile remains unqualified. |
| Clocks | Processing-node audio device is the sample-clock reference; Brain follows source frames without a second audio device. No ASRC or device clock-control UI is implemented. | Enforce and expose one soundcard/ADAT clock domain and shared frame timeline throughout the integrated graph; test lock-loss/restart handling. The user accepts soundcard clocking; a new source selector is not required. |

Sources: [architecture](ARCHITECTURE.md), [transport](AUDIO_TRANSPORT.md),
[local provider](AUDIO_LOCAL_SERVICE.md), [module contracts](MODULE_CONTRACTS.md),
[PA embedding](../../shr-pa/docs/EMBEDDING.md),
[PA integration plan](../../shr-pa/docs/GIGPIES_IMPLEMENTATION.md),
[Desk control contract](../../shr-desk/docs/CONTROL_CONTRACT.md).

The manufacturer's [UMC1820 guide](https://www.bhphotovideo.com/lit_files/155647.pdf)
also exposes an important mapping detail: at 48 kHz in ADAT mode, analog capture
occupies 1–8, S/PDIF 9–10 and ADAT 11–18; analog playback occupies 1–10,
S/PDIF 11–12 and ADAT 13–20. Therefore the intended **16/18 analog profile is
not simply the first 16 capture and first 18 playback slots** of that transport.
Verify the actual Linux device mapping before physical use. The guide also notes
that changing optical mode resets the interface. ADA8200 clock selection includes
a physical selector; consult its [manufacturer guide](https://mediadl.musictribe.com/media/PLM/data/docs/P0ATL/QSG_BE_0800-AAB_ADA8200_WW.pdf).
Neither document proves the attached device’s observed clock lock. The user’s
clarification is to keep soundcard clocking and ensure everything shares it, not
to add selectable clock sources.

## Execution brief

```text
Work from /home/shome/p/gigpies on rpi5. Execute this increment through owner
implementation, independent review, software validation, scoped publication/CI
and canonical source/ledger synchronization on both Pis. Do not stop at an audit,
new constants, a contract, mocks, compilation or a smaller milestone that omits PA.

PRODUCT DIRECTION AND PRECEDENCE

The processing engine is the modular Stagebox/processing-node system: capture,
channel processing, mixing/buses, PA processing/protection, local recording and
network endpoints. SHR PA is an active module in that system. Repository ownership
determines where code lives; it is not a reason to omit required PA work.
GigPies owns composition/host/authority, SHR PA owns speaker DSP/measurement,
SHR Desk owns operator presentation, SHR FX owns effects and SHR REC owns recording.
Reuse them. Do not copy their algorithms or create another parallel mixer/PA engine.
Do not move the whole band mixer into shr-pa merely because its node is called PA.

Minimum reference setup: UMC1820 plus ADA8200 over bidirectional ADAT, 48 kHz,
16 analog inputs and 18 analog outputs. The first-class software profiles must
also exercise 32 and 48 inputs. Neither 8, 16, 32 nor 48 is a universal product cap.
Physical socket count, USB stream width, mixer strips, bus count, PA inputs/outputs,
network packet groups, recorder tracks and UI bank size are separate quantities.

No arbitrary hard limits: do not replace INPUTS=8 with INPUTS=48 and call it modular.
Use validated configuration and advertised capabilities, finite preallocated runtime
storage, checked arithmetic and explicit resource admission. Document each remaining
bound: origin, layer, units, reason, observed/configured capacity and expansion path.
Packet MTUs, protocol integer widths and memory/deadline budgets are legitimate
bounds; a convenient fixture or screen size is not a product requirement.
No silent truncation, hidden fixed monitor count, channel aliasing or automatic
downmix. Do not promise unlimited work or allocate dynamically inside rendering.
Forty-eight-channel software correctness is required; real-time hardware throughput
at that count must be measured before claiming it. If capacity admission fails,
report the precise resource reason without silently reducing the show configuration.

This instruction supersedes earlier task-specific restrictions that held the mixer
at eight channels or deferred all configurable PA, 4-way/topology, remote-control
and common-clock integration work. In particular, read shr-pa/AGENTS.md's current fixed-2x6 scope and
its dated decisions as the previous milestone, not a permanent ban on this increment.
Update relevant owner scope/roadmap instructions so future sessions do not restore
obsolete product ceilings. Preserve original drafts, old ABI semantics and dated
evidence. Do not erase safety, realtime or physical-operation requirements.

RECOVER AND ARRANGE THE WORK

Read /home/shome/p/AGENTS.md and each activated owner's AGENTS.md. Reinspect live
Git state, branches/remotes, existing worktrees/owners/processes, build locks and
disk on both hosts. Preserve unrelated edits and interactive sessions. Fetch verified
origins without resetting anything. Reserve the next unused private-ledger task ID;
task0013 is complete and must not be reopened.

Recovery references, not reset targets:
- GigPies 96e7595813d79c97081b04ae37180687b755d34b.
- Desk e4f7983917c0443cabfd2a413a393fe980dd24f3; implementation 2cb269b.
- SHR PA 44a111f0e2e2d640d3fff1378a7f6131b3df3151.
- Final task0013 ledger 2a765537df6208985a8e6a8d33cfdb7c6f8fc7cc.
- GigPies user/four-band-eq/0013/REVIEW.md, final-revisions.json, final-ledger.json,
  desk-accepted.json, joint-final.json, workers.json and owner artifact manifests.

Read GigPies README, STATUS, ARCHITECTURE, original blueprint-v2 sections on
hardware/output allocation, COMPONENTS, BRAIN_CONSOLE_PLAN, MODULE_CONTRACTS,
MODULE_IMPLEMENTATION_MAP/PLAN, CHANNEL_PROCESSING, AUDIO_CONTROL_WIRE,
AUDIO_RENDERED_WIRE, AUDIO_LOCAL_SERVICE, MODULE_GRAPH, ANALYSIS_STREAM,
AUDIO_TRANSPORT, AUDIO_HARDWARE and their owning plans, DEVELOPMENT/PUBLICATION.
Read SHR PA README, STATUS, ARCHITECTURE, DSP, config/control/FFI source,
EMBEDDING, GIGPIES_IMPLEMENTATION, FUTURE, relevant decisions, VALIDATION and
PUBLICATION. Read Desk BLUEPRINT, GIGPIES_IMPLEMENTATION, CONTROL_CONTRACT,
NATIVE_FRONTEND, STATUS and development/publication rules. Read REC/FX/Lux owner
contracts only where their consumed interfaces are affected.

Trace actual callers through GigPies mixer/control_model/mixer_control,
processing_wire/local_audio/module_graph, host/device/adapters/network and
transport packet/control/handoff/buffer; Desk audio/processing/local_audio/frontend;
SHR PA Engine/Prepared/Handoff/config/ffi/header. Search hidden shape assumptions,
fixed arrays/masks, schema counts, channel-name parsers, two-monitor enums,
recorder maps, analysis subscriptions, host stereo buffers, transport grouping,
snapshot/frame-byte limits and UI banking. Do not infer capability from a document
when the implementation disagrees. Separate current status from historical records.

Create a concise current-state matrix and dependency-ordered execution queue with
one owner, exact files/base, contract/artifact dependencies, acceptance and resource
slot per assignment. Resolve the architecture/ABI/network/common-clock decisions with a
separate independent reviewer, then implement them. Ordinary decisions and the
existing B-PA/B-NET review gates are engineering work, not reasons to ask the user
to design the system or leave the feature perpetually deferred.

Keep the coordinator's selected model. Every delegated worker, including peer,
code, review and test workers, MUST explicitly use gpt-6-astra, reasoning low.
For built-in workers use a context mode permitting explicit overrides. Inspect
CLI/runner support and pass real per-invocation overrides; prompt prose alone is
insufficient. Preserve pinned SSH, one receiver per host, --yolo, bounded timeouts,
owned-process cleanup and private logs. Use/review a task-private runner adaptation
if needed; never change global defaults/authentication or existing sessions.
Record launch argv and available runtime model metadata. No recursive dispatch.

Use both Pis for disjoint useful lanes, integrate provider before consumer and
freeze new producer fixtures from actual execution before consumer acceptance.
One writer per file/worktree; coordinator owns shared contract/status integration.
Never push into Pi4's Pi5-backed checked-out origins. Workers may make scoped local
handoff commits; root alone publishes verified upstreams. Save commits, hashes,
remaining commands and resource state well before worker deadlines. Connection
loss/timeouts do not roll back work: inspect durable records before continuing.

A. CAPABILITY-DRIVEN ENGINE AND I/O TOPOLOGY

Implement one production graph used by both synthetic/headless and physical-device
adapters. Eliminate the architectural split between the eight-input local authority
and the stereo device/network bench; preserve qualified legacy behavior through
explicit compatibility, not a second new implementation.

Describe stable physical-port, source/strip, bus, module-port and output IDs and
their mappings. Advertise supported/configured counts, sample format/rate,
block bounds, module versions and resource admission. Provision off the realtime
path; apply a fully validated prepared configuration atomically at an explicit
boundary and retire old state off that path. Never free the last heap owner,
design coefficients, parse, lock, wait or perform I/O inside render/callback work.
Keep the four real parametric bands, individual/global bypass and existing
compressor on every admitted input, with neutral exactness, smoothing and truthful
current/target/readiness/GR. Preserve original authority/revision/freshness fences.

Deliver a tested 16-input/18-output reference profile and 32/48-input software
profiles through the same engine. Extend recorder/analysis/module integration
and Desk dynamically; do not leave high channels present only in a descriptor.
Demonstrate an additional non-fixture dimension where useful to expose a hidden
shape ceiling. UI banks paginate capabilities; they never define engine capacity.

Provide a reviewed reference physical map separating the 16/18 analog routes from
the UMC1820's wider USB stream and digital S/PDIF slots. Preserve explicit indices
and directions; do not map the first16/18 slots blindly or count headphone mirrors
as independent DACs. Device-free fixtures must model that noncontiguous map.
The attached unit's actual negotiated shape and socket/clock mapping remain
hardware acceptance; a reference profile is not an observation of attached hardware.

Monitor and PA allocations derive from topology. Support more than two monitors
and leave unassigned outputs explicitly silent. No mandatory consumption of every
output and no hard-wired PA/monitor socket split. Show examples using six PA outputs
plus remaining monitor/output capacity, and eight PA outputs plus the remainder.
Measurement inputs are separate and never implicitly routed into program/monitors.

B. REAL MODULAR PA IMPLEMENTATION, INCLUDING ITS OWNER

Activate SHR PA as a code owner now. Implement the reviewed successor configurable
embedding contract to fixed-v1; preserve v1 bytes/semantics and standalone operation.
Build on existing Engine/Prepared/Handoff and verified DSP. Specify exact C ABI
version/size, port dimensions, prepare/apply/status/retire ownership and lifetimes,
bounded queue/retirement behavior, fault/reset semantics and sample-boundary result.
Host retains libraries while handles/state live. Freeze actual library/header/
fixture hashes; test real C callers and GigPies' actual loaded library.

Expose real PA configuration, not only health: routing/layout, crossover,
existing input/output EQ, gain/mute, delay/polarity and protection controls where
supported by the owner. Advertise exact availability and ranges; missing measurement
or true-peak/acoustic capabilities remain honestly unavailable. Keep musical channel
EQ and speaker-system EQ distinct while both belong to the composed engine.

Keep existing 2x6 layouts working. Deliver real six-output stereo3-way and
eight-output stereo4-way software configurations. Reuse crossover primitives;
review whole-system phase/summation, compensation, slopes, polarity, gain and delay
for the 4-way extension instead of just appending another filter/pair. Do not
silently approximate a4-way preset using the fixed-v1 full-range adapter.

Establish the next usable matrix foundation: descriptor-driven program inputs,
mono sum/routing nodes and independently processed outputs. A 4-input/8-output
software reference demonstrates explicit weighted sums and port mapping; 4x8 is
an example, not a new universal cap. Review whether composition of reusable mono
branches/instances or an owner-native topology extension best preserves current DSP.
Document sums/headroom without hidden normalization; refuse cycles/invalid ports
and accidental duplicate physical-output writers. Basic reviewed route editing and
readback are required; an elaborate arbitrary-graph visual editor is not.

Protection follows every summed signal requiring it. No added wet/matrix path may
bypass final output protection. Preserve sample-limiter versus true-peak/calibrated
speaker-protection distinctions. Structural/routing changes need explicit reviewed
mute/quiesce/reprepare/commit/rearm semantics, rollback and retained-state behavior;
never click-switch live crossovers or silently unmute after recovery. Newly available
channels/outputs start in the reviewed safe state, not accidentally live.

C. PROCESSING NODE ↔ BRAIN AUDIO AND COMMANDS

Complete the useful remote path through the same real authority. GPC1's scalar
test authority is not the remote mixer. Do not invent a second weaker authority,
translate unsupported schemas silently or use an SSH tunnel as proof that the
application's remote protocol/authentication is implemented.

Resolve B-NET with independent review: peer identity/pairing, standard maintained
authenticated transport, credentials and revocation, bounded framing, capability
exchange, local/remote endpoint selection and failure behavior. No home-grown
cryptography, hard-coded credentials, public default listeners or unauthenticated
production writes. Retain the private Unix endpoint and explicit legacy refusal.
Choose practical reviewed defaults from existing work; do not leave the implementation
blocked on routine protocol/library decisions.

Remote commands must retain scope, writer/lease, request IDs, retry history,
expected revision, atomic configuration, pending/final application frame and
reconnect-without-replay. PA configuration and routes have their own advertised permissions; a channel
grant does not silently grant them. Clock status is observational in this increment.
Snapshots/telemetry must scale and be coherent. Review bounded pagination/chunking
with revision identity if necessary, rather than independently combining stale
pages or arbitrarily raising message limits. Slow clients never stall audio/REC.
Integrate authenticated state with restart/rekey/capability/map changes and reject
old packets, descriptors and queued intents. Label unsupported data explicitly.

Reuse GPA1 media grouping/deadlines/fault handling where sufficient. Implement
actual stream-descriptor negotiation/session binding for selected raw analysis,
Brain FX sends and wet returns at the configured dimensions. Specify peer/session
validation and the media trust boundary; a control TLS connection does not by
itself authenticate arbitrary UDP audio. Preserve fixed packet/queue bounds, use
multiple groups, and keep media arrival from advancing the source render clock.
Keep SSH/Git development traffic conceptually separate from product protocols.

Exercise real SHR FX on Brain and return its actual wet result into the processing
node's dry+wet sum before PA. Preserve local dry mains/monitors/protection and
recording through Brain restart/loss. Missing/late wet returns follow bounded
fades and fresh-session recovery. Recorder and analysis queues are independent.
Do not broaden FX algorithms/racks unnecessarily; change its owner only where a
demonstrated interface adaptation is needed. Preserve current Lux subscriptions
through an adapter or reviewed compatible extension if analysis identity changes.

D. ONE SOUNDCARD CLOCK DOMAIN AND SHARED SAMPLE TIMELINE

The user accepts the soundcard clock. Keep it as the system reference and ensure
all audio components use the same domain; do not add a clock-source selector or
make clock switching a prerequisite. Distinguish the converter clock from host
source-frame counters, network deadlines and authority-local lease clocks.

The processing node's interface supplies the audio timeline. Its ADAT expansion
must be synchronized to that same reference, with an explicit verified master/
follower relationship and required ADAT connections. Do not infer synchronization
from equal nominal 48kHz settings, packet arrival, wall clocks or USB enumeration.
No clock loops or two independently free-running audio masters in the reference
profile. The existing hardware mapping/lock remains unqualified until checked on
that actual rig; prepare its exact acceptance procedure without fabricating lock.

Drive capture, channel/bus/PA processing and output from the authoritative device
frame progression. REC and analysis retain the original source epoch/frame/tap
identity. Brain without an audio interface follows those frames: its FX consumes
indexed sends and returns samples tagged for the declared output deadline. Brain's
UI timer or CPU scheduling must not become a second audio clock. Network arrival
must never advance or reset the render cursor. Algorithmic/network delay is explicit
and distinct from rate synchronization. Do not add NTP/PTP/ASRC by name and claim
that it synchronizes the converters.

Expose truthful clock-domain/rate/epoch/continuity status in existing provider
health and Desk, including ADAT lock evidence when actually readable. Unavailable
physical lock information is unknown, not an invented healthy value or a checkbox
claim. A physical/manual setup requirement can be documented without pretending
that software performed it. No new clock-configuration authority or picker is
needed. Reuse existing status/telemetry seams with explicit versions where required.

Review and implement discontinuity/lock-loss/device-reopen behavior: controlled
mute/quiesce, recording continuity or explicit finalized/incomplete take outcome,
fresh epoch/maps after valid recovery, rejection of old wet audio and commands,
and explicit recovery/rearm with bounded ramp. No silent sample slips, old-frame
reuse, unmute or resurrection of write grants. Synthetic device/clock fault
injection must exercise these actual production state transitions. Independent
capture/playback devices are outside this single-clock profile unless a separately
supported synchronization boundary is proven; do not silently combine them.

E. DESK, PERSISTENCE AND COMPATIBILITY

Expose capability-derived strips/buses/outputs, labels and physical mapping;
edit inputs 16, 17, 32, 33 and 48 as present without off-by-one aliases or hidden controls.
Preserve all four-band controls and complete protected review at every bank.
Add real PA/output routing controls and common-clock health using existing
semantic action and provider layers, not a second simulator. Show module boundaries, unavailable
controls, current/pending/failed/unknown state and honest units. Test keyboard
and injected-controller parity and CPU-headless resize/layout presentation.

Version schemas whose fixed-eight shapes or new authority/topology semantics
change. Preserve historical fixture corpora and old supported behavior, or explicitly
refuse unsupported versions. Never present a 48-channel provider to an eight-channel
client by silently dropping 40channels. No mandatory sibling source-path dependency.
Define snapshot/restore/migration of the configuration actually introduced here,
including stable mapping/topology identities and capability mismatch handling.
Persistent show/route intent is not permission to unmute or replay on attach.
Do not expand this into unrelated scene automation, musical features or lighting UI.

F. SOFTWARE ACCEPTANCE THROUGH THE REAL MODULES AND BOTH NODES

Write acceptance criteria before implementation and keep independent references.
Complete the 16-in/18-out path first, then 32/48 configurations through the same
production components. Do not call the whole task complete at the first slice.
Required evidence includes:
- Unique per-input impulses/tones/PCM, channel isolation and four-band EQ/dynamics
  beyond input 8. Exact neutral/bypass and raw recording/analysis preservation.
  Check every admitted input and mapped output, not only descriptor length.
- Multiple monitor mixes, actual six/eight-way PA output paths,4x8 weighted mono
  sums and noncontiguous16/18 reference USB mapping. Verify every crossover branch,
  independently computed summed response/phase, protection order and silent unused
  outputs. Include invalid/cyclic/duplicate maps and atomic failed reconfiguration.
- Actual loaded new PA library and unchanged-compatible REC/FX owners, not copies.
  Apply from the real Desk through authenticated network authority on the other Pi,
  observe the boundary, changed samples, final readback and visible state. Include
  high-numbered input controls, PA controls, output patch changes and shared-clock
  status/recovery.
- Real grouped audio/analysis/FX across the two Pis with source-frame evidence,
  stream negotiation and exact source identity. Tests cover packet loss/reorder,
  duplicates/stale/late data, slow client, wrong/revoked peer, reconnect/role/lease
  loss, incompatible schema, map/epoch change and independently stalled workers.
  Kill/restart only owned test children; never disrupt SSH or unrelated sessions.
- Brain loss/restart preserves local dry audio/protection/recording; wet returns
  fail/recover as specified; no replay/unmute. Clock discontinuity/lock loss uses a fake
  device boundary to prove actual production transition logic and honest UI status.
- Realtime allocation/deallocation guards, bounded resource use, prepared-state
  retirement, queue saturation and partition equivalence at 16, 32 and 48 inputs. Record declared
  CPU/memory/throughput profiles and synthetic timing separately from hardware
  deadline qualification. Do not infer 48-channel real-time success from offline
  rendering or from the old 64-channel identity network transport measurement.

Use finite synthetic functional network runs under explicit reservations on both
hosts, with declared ports/durations/load and stop conditions. No uncontrolled
stress or full-show combined-load campaign is implicit. Negotiate the reservation
autonomously through the private ledger; preserve existing interactive sessions.
Use CPU-headless native rendering on an inspected permitted backend; never open
operator windows or silently use physical GPUs. Hardware unavailable does not block
software implementation, but it does block hardware claims.

Run focused regressions during implementation, then complete normal production
suites for every changed engine/render/schema/authority/concurrency owner. Run
format, warnings-denied Clippy, relevant features, release, C ABI/link/lifetime,
script/docs and publication checks. Test dependent consumers whose contracts change;
do not rebuild unrelated instruments by habit. Historical media/auditions/exhaustive
research/long benchmarks remain opt-in unless directly affected. Document skipped
classes and commands. Preserve failures; do not relax thresholds to manufacture success.

Every Cargo/check/test/Clippy/release/C-link command uses the host's parent-held
NONBLOCKING /home/shome/p/.gigpies-build.lock, Rust +1.97.1, committed Cargo.lock,
--locked and -j1 where applicable, CARGO_BUILD_JOBS=1 and CARGO_INCREMENTAL=0.
Reuse normal targets and flags. Review disk below 20 GiB free or target above 5 GiB;
do not weaken diagnostics or erase another owner's cache. A busy slot means useful
independent work then retry, not hidden blocking builds. Clean only task-owned
disposable outputs after checking users/locks; retain final artifacts and concise
reproducible evidence. Preserve all original prompts, media and unrelated work.

AUTHORIZATION WHEN THIS BRIEF IS EXECUTED

Authorized: coordinated software implementation in GigPies, SHR PA and SHR Desk;
necessary demonstrated interface adaptations in SHR REC/FX and affected analysis
consumers; independent Astra-low review/workers on both Pis; synthetic temporary
PCM/UDS and reserved bounded two-Pi functional network checks; CPU-headless UI;
reviewed source commits/pushes, CI repair and final source/ledger synchronization.
Read-only work elsewhere stays read-only. This is not an invitation to redesign
unrelated instruments, lighting, business services or every PA measurement feature.

Not authorized by merely invoking this brief: physical PCM/device activation,
playback, speakers/amps, MIDI/DMX, display windows, physical clock/ADAT switching,
host audio/network/time/display tuning, service deployment, full combined-load
qualification, public exposure, tags/version bumps or binary releases. Prepare
the exact bounded hardware acceptance plan for the real 16/18 rig after software
work; obtain session-specific authorization before running those operations.
If physical access is genuinely needed, finish independent software work first
and state the exact remaining check and why it needs that access. Do not claim
the requested complete software path if a required software acceptance still fails.

COMPLETION, PUBLICATION AND CONTINUITY

Update focused owner contracts, architecture, scopes/AGENTS where obsolete,
status and roadmap next steps alongside behavior. Preserve original drafts and
qualified failures. Maintain a requirement-to-code-to-test matrix so hardware,
module, protocol and UI work cannot disappear behind a smaller fixture milestone.
Every deferral must identify the unmet requirement, reason, owner and next concrete
acceptance; routine difficulty is not a reason to silently shrink authorized scope.
Future-facing instructions must distinguish product capacity from current profile.

Independently review integrated final artifacts, exact source and staged content.
Enable existing versioned hooks, review new reusable scripts under publication
policy, run complete-index and outgoing-history guards, and stage only named files.
Publish owner providers before dependent consumers to verified upstreams, inspect
remote SHAs/CI and fix task-related failures. No force push, empty owner commits,
private data publication or changes to unrelated commits.

Safely fast-forward twelve canonical source checkouts on both Pis to exact final
published revisions, preserving Pi4's Pi5-backed origins and unrelated work.
Do not push through those origins into checked-out Pi5 branches. Synchronize the
private ledger with independent receiving acknowledgment and exact manifests.
Source synchronization does not deploy the system.

Finish with a concise factual report and durable acceptance record: modular owner
changes, 16/18 reference mapping,16/32/48 software results and measured capacity
limits, six/eight-output and matrix PA behavior, audio/control protocol status,
single-clock enforcement/status and remaining physical lock verification, actual
operator/network/library/sample evidence,
tests run/skipped, remaining physical gates, every worker's model/effort, per-owner
commits/push/CI, both-Pi source/ledger sync and disk/work preserved. Do not describe
all protocols, all PA functions or live hardware as complete without the corresponding
evidence. Do not stop at a proposal or hand routine coordination decisions back to me.
```
