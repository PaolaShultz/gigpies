# Execute Brain sound-card talkback and monitoring integration

Prepared 2026-10-05 after modular-engine task0014. This file is an execution
brief, not an instruction to begin work merely by creating or reading it for
review. Execute it when the user explicitly invokes it.

## Product correction

Brain has **one local duplex sound card**: microphone input for operator talkback
and audio output for operator monitoring. Stagebox has its own sound card and
ADAT expansion. “Brain follows Stagebox frames without an audio device” describes
the earlier FX worker, not the complete Brain product. Preserve that historical
acceptance, but correct current architecture and implement the missing paths.

```text
Work from /home/shome/p/gigpies. Execute this increment through owner implementation,
independent review, software acceptance, reviewed publication/CI and exact source/
ledger synchronization on both Pis. Do not stop at a design, mock, synthetic-only
application, transport loopback, new UI controls or an unconnected device adapter.

PRODUCT AND CLOCK CONTRACT

Brain uses one actual duplex sound-card endpoint for local talkback capture and
operator monitor playback. One process/device owner provisions both directions;
Desk controls it through the established authority/client layers. Do not open a
second competing PCM endpoint from Desk or the FX worker. Do not assume a card
model, socket order, independent headphone DAC, channel count or hardware mixer
setting that has not been observed. Minimum software reference: one microphone
input and stereo monitor output, with configurable validated physical mapping.
An explicitly configured mono output may be supported without silently collapsing
a stereo request. Extra advertised inputs/outputs are not a new product ceiling.

There are two device clock domains unless an actual external synchronization
arrangement is proven: Stagebox sound card plus its ADAT expansion, and Brain's
single duplex sound card. Equal nominal 48 kHz rates do not establish equal clocks.
Within each node require the configured duplex endpoint to satisfy the declared
shared-clock contract. Never combine unrelated capture/playback devices implicitly.
Keep Stagebox capture, mixer, PA, local recording and source-frame FX on the
Stagebox reference timeline. Brain local I/O owns a distinct epoch/frame timeline.
Do not clock Brain FX processing from its sound card, resample raw Stagebox REC,
or disturb existing source-indexed analysis and FX-return deadlines.

Implement bounded asynchronous sample-rate conversion and clock-domain bridging
at BOTH new crossings: Brain talkback into Stagebox rendering, and Stagebox monitor
feed into Brain playback. Select a maintained suitable implementation after review
of official documentation, license, realtime behavior, variable-ratio support and
quality; record the choice and reproducible dependency version. Reuse an existing
suitable owner if available. Do not build a second unrelated audio engine or silently
replace ASRC with routine sample duplication/drop or unbounded buffering.
An alternative is acceptable only with equivalent demonstrated independent-clock
correctness, bounded latency and failure semantics; network timestamps, NTP or
nominal-rate equality alone are not that demonstration.

Separate oscillator drift from network jitter and scheduling delay. Declare signed
ratio conventions, estimation/filtering, acquisition, prefill, target occupancy,
ratio and slew limits, quality/delay, maximum acceptable skew and recovery behavior.
Use explicit integer frame identities and epochs; document the mapping and its
uncertainty. Do not promise exact raw-sample preservation across resampling. Neither
a Brain device fault nor bridge reacquisition may reset the Stagebox clock.
No general clock-source picker or host clock-daemon tuning is required.

RECOVERY, OWNERSHIP AND EXECUTION

Read /home/shome/p/AGENTS.md and each activated repository's AGENTS.md. Inspect both
nodes' live Git state, remotes, worktrees, processes, build locks and disk. Preserve
unrelated work and interactive sessions. Reserve the next unused private task ID;
do not reopen task0014. Use its final private completion.json, final-revisions.json,
final-ledger.json and software acceptance as recovery evidence, not reset targets.
Expected prior published heads (verify live state): GigPies ddc39ef, SHR PA d06aec1,
SHR Desk f75f7e9. Never reset newer work to these checkpoints.

Read README, STATUS, ARCHITECTURE, COMPONENTS, BRAIN_CONSOLE_PLAN, MODULAR_PROCESSING,
MODULAR_ENGINE_ACCEPTANCE, REMOTE_PROCESSING/REMOTE_TRANSPORT, AUDIO_TRANSPORT,
AUDIO_HARDWARE and their owning plans; control/render/local-service/module contracts,
DEVELOPMENT and PUBLICATION. Read Desk BLUEPRINT, CONTROL_CONTRACT, NATIVE_FRONTEND,
GIGPIES_IMPLEMENTATION and relevant source. Trace actual LocalAudio/mixer/authority,
clock, remote/media/session/host/runner, host/device/brain_fx and hardware executables.
Inspect existing PFL/solo/talkback plans and implement one coherent production path.

GigPies owns audio-device hosting, routing, bridging, transport and authority;
SHR Desk owns operator controls and presentation. PA, FX and REC algorithms remain
in their owners. Authorized sibling changes are Desk and demonstrated necessary
PA/FX/REC interface adaptations; unrelated modules remain read-only. No sibling
path dependencies or copied owner algorithms. Keep Rust1.97.1, edition2024 and
committed lockfiles unless repository instructions explicitly supersede them.

Create a requirement-to-code-to-test matrix and a dependency-ordered queue with
exact file ownership, base revisions, frozen contracts and resource slots. Use an
independent architecture/clock/safety reviewer before implementation and independent
final artifact review. Keep the coordinator's selected model; EVERY delegated
worker must explicitly use gpt-6-astra with reasoning low, including peer/review/test
workers. Record actual launch overrides and available runtime metadata, not only
prompt prose. No recursive delegation. Follow the private ledger, pinned SSH,
--yolo peer runner, bounded timeouts, one peer receiver per host and durable handoffs.
Use both Pis for disjoint useful work. Root alone publishes upstream source.

A. PRODUCTION DEVICE HOST AND PREPARED CONFIGURATION

Implement the Brain duplex adapter and executable/configuration path that can
actually open the selected physical device when separately authorized. Reuse the
existing device abstractions where appropriate; use the same production bridge,
authority, queues and render path for fake-device and physical adapters. Software
acceptance must not require activating a real PCM. Validate actual capabilities,
formats, rates, period/buffer bounds, duplex compatibility and socket/channel maps;
report unsupported requests precisely, without fallback to an unrelated default.

Expose stable device/port/route IDs, configured and supported resources, direction,
stream width, format conversion, physical-verification state and device health.
Persist configuration intent, never active talkback, grants or permission to play.
Prepare allocation, coefficients and stream resources off the realtime path. Swap
at declared boundaries and retire old state off that path. No allocation/free of
last owners, locks, socket/file I/O, parsing, coefficient design or unbounded work
inside callbacks/rendering. Declare finite bounds with reasons and expansion paths.
Provide safe startup, explicit arm, mute, stop, unplug/reopen and shutdown behavior.
Do not install or auto-start a service.

B. TALKBACK: ACTUAL MICROPHONE-TO-DESTINATION AUDIO

Route Brain microphone capture through explicit gain/mute, clock bridge and
negotiated authenticated talkback media to selected Stagebox destinations. Expose
actual microphone, outgoing and destination state/meters. Distinguish requested,
authorized, applied and audible-path-ready state. Talkback is a separate role and
route, not counterfeit raw analysis, an FX return or an automatic extra band input.

Implement momentary push-to-talk with explicit destination selection for admitted
performer monitor buses and an explicit separately protected FOH route if selected.
Default closed, no destinations selected, FOH excluded. Do not infer monitor bus
count from fixture sizes or physical socket count. Mix at a documented point with
declared gain/headroom behavior. FOH talkback must traverse configured PA protection;
monitor talkback must obey destination mute, gain and output safety. Never bypass
output patch admission or a global safety mute. No implicit FX send/feedback loop,
raw-band recording, analysis subscription or talkback-to-local-monitor loop.
Optional sidetone, ducking or latching is not required; any implemented version
needs explicit routing, safe defaults, bounded gain and separate acceptance.

Use scoped authority and a bounded held-action heartbeat/deadman. Release must
close within a documented maximum even if key-up is lost. Focus loss, controller
removal, lease/session loss, revocation, application failure, device fault and
shutdown close talkback. Reject late/reordered/duplicate/stale media and held-action
messages after release or epoch replacement. Reconnect never resumes a held button
or persisted talkback state. A read-only client cannot transmit authorized talkback.
Test multiple clients and enforce one explicit owner where required.

C. MONITORING: ACTUAL STAGEBOX-TO-BRAIN AUDIO

Implement an operator listen bus with explicit source selection, stereo main/program
monitoring, selected performer-mix monitoring and channel PFL/AFL. Define the exact
pre/post EQ, dynamics, mute, fader and pan tap semantics; use consistent documented
labels. If established product semantics differ, resolve and document the choice
before code. No solo-in-place or change to audience/performer mixes from selecting
operator monitoring. Implement deterministic selection/clear behavior and explicit
multi-selection mixing semantics if offered; do not silently sum unrelated sources.

Send actual selected samples through a distinct authenticated monitor stream,
bounded jitter/ASRC bridge and Brain playback. Monitor gain, mute and dim are local
to the operator output; bound transitions and gain/headroom safely. Startup and
recovery remain muted until explicit arm. On starvation fade promptly to silence;
never repeat stale audio indefinitely or build an ever-growing latency backlog.
Show source, tap, mapping, readiness, meters, mute/dim, underruns, bridge occupancy,
ratio/drift estimate and latency components with honest units/uncertainty. Do not
label network RTT as one-way latency or inferred drift as physical clock lock.

A route/selection change must invalidate queued old-source audio at an explicit
boundary. Device reopen or reconnect requires fresh identities, prefill, readback
and explicit rearm. Brain local monitoring must coexist with source-clock FX and
analysis. Losing local monitoring must not stop Stagebox dry audio, PA protection,
raw recording or unrelated authorized FX/control work.

D. AUTHENTICATED PROTOCOL, CLOCK FAULTS AND PERSISTENCE

Extend the production mutual-TLS QUIC control/datagram path and versioned descriptors,
not the historical unauthenticated UDP prototype. Distinguish talkback and monitor
roles, permissions, channel/tap identities, both device epochs, map generations,
bridge state, sample rates, packet grouping, deadlines and supported versions.
Do not accept arbitrary media solely because its peer has an unrelated FX grant.
Do not let a new role monopolize the existing media owner or evict valid FX/analysis.
Design and test simultaneous negotiation, bounded queues and slow-consumer isolation.

Apply control changes through existing revision/freshness/grant rules with real
boundary acknowledgments. Freeze producer-generated success/failure fixtures for
Desk; reject unsupported versions explicitly, without truncation. Preserve existing
clients' supported behavior. Persist stable routes/settings with mismatch validation,
never session IDs, active permissions, PTT holds or automatic audible recovery.

Exercise Brain capture/playback stalls, xruns, lock/clock discontinuity, unplug/reopen,
queue saturation, out-of-envelope drift, network loss and both process restarts.
Define which failures mute one path versus invalidate the whole duplex epoch.
Maintain Stagebox safety behavior for Stagebox faults; invalidate old bridge data
and grants as required. Recovery must not replay talkback, old monitor selections
or output-unmute commands. Expose failures promptly and preserve diagnosis counters.

E. REAL DESK INTEGRATION

Add device/mapping setup, talkback destination review, held-action controls,
monitor source/PFL/AFL selection, gain/mute/dim and bridge/device health to the actual
Desk provider/client/frontend. Use existing semantic actions for keyboard and
injected-controller parity. Handle focus/release/disconnect safety through the
provider, not just UI state. Keep local operator monitor authority distinct from
Stagebox mix-edit and talkback-destination authority. Show unavailable/failed/stale
states truthfully. Test banks containing inputs16/17/32/33/48, dynamic monitor buses,
resize and CPU-headless native presentation. No lighting-console redesign or real
MIDI/display activation is needed.

F. INDEPENDENT SOFTWARE ACCEPTANCE

Write acceptance thresholds and references before implementation. Include:
- Independently paced fake Stagebox and Brain duplex clocks using the production
  host/bridge paths. Both drift signs, rate ramps, jitter without drift, startup,
  near/outside admitted skew and discontinuities. Include at least ±10/±100ppm and
  ±1000ppm stress: the latter must either meet the declared admitted envelope or
  refuse/fail closed explicitly. Simulate long enough for an uncompensated bridge
  to exhaust its buffer; accelerate virtual time rather than require long sleeps.
- Independent numerical ASRC references and declared passband/alias rejection,
  gain/phase, delay and transition tolerances. Test DC, impulses, tones, chirps,
  silence, nonfinite input and multichannel isolation. Steady admitted drift must
  not cause repeated drop/dup corrections, unbounded occupancy or latency growth.
- Exact samples before conversion and appropriate numerical comparison after it.
  Verify every talkback destination, unselected-route silence, all monitor selections,
  high channels and actual FOH protection. Prove operator PFL/AFL and local gain/dim
  do not change audience/performer outputs or raw REC/analysis. Include arbitrary
  channel/socket permutations, invalid/duplicate maps and atomic failed changes.
- Actual Desk actions on one Pi controlling real production endpoints on the other,
  with actual SHR PA/FX/REC libraries and independently checked sample artifacts.
  Run simultaneous talkback, monitor feed, FX/analysis and recording at16/32/48 input
  profiles under finite mutually acknowledged network reservations. Use fake PCM
  endpoints through the production adapter boundary; do not replace DSP or routing
  with test-only implementations. Verify return-path audio, not only descriptors.
- Lost key-up/deadman, stale/revoked peer, lease loss, replay, wrong role/channel/map,
  packet loss/reorder/duplicate/late delivery, independent worker stalls, Brain
  sound-card failure, process restart and Stagebox clock replacement. Assert safe
  closure/rearm and continued local dry/protected/recorded audio where specified.
- Allocation AND deallocation guards, bounded queue/memory/work, prepared-state
  retirement, partition equivalence and measured CPU/RSS/latency/occupancy. Report
  observed synthetic capacity separately from physical deadlines and sound quality.

Run focused regressions, then full normal suites for every changed engine/render/
protocol/schema/authority owner and relevant consumers. Run fmt, warnings-denied
Clippy, relevant feature combinations, release executables, C ABI/link/lifetime
where changed, script/docs checks and publication guards. Historical auditions,
exhaustive research and long benchmarks remain opt-in unless directly affected;
record skipped classes and reproducible commands. Never weaken safety/freshness or
quality thresholds to turn a failed run green. Preserve failures and corrections.

Every agent-run Cargo/check/test/Clippy/build/C-link command uses the host's
parent-held NONBLOCKING /home/shome/p/.gigpies-build.lock, Rust+1.97.1, --locked,
-j1 where applicable, CARGO_BUILD_JOBS=1 and CARGO_INCREMENTAL=0. Reuse normal targets.
Review free space below20GiB or target above5GiB; preserve shared caches and active
sessions. Clean only known task-owned disposable output after checking users/locks;
retain final executable identities, concise evidence and reproducible commands.

AUTHORIZATION, PUBLICATION AND COMPLETION

Executing this brief authorizes the scoped software/owner work, independent workers,
synthetic temporary audio, reserved bounded two-Pi checks, CPU-headless rendering,
reviewed source commits/pushes, CI repairs and twelve-repository source/ledger sync.
It does NOT authorize opening physical PCM, microphones/headphones/speakers, changing
hardware mixers/phantom power/clock selectors, MIDI/DMX/displays, host audio/network/
time tuning, deployment, public listeners, tags or binary releases. Implement the
real hardware adapter fully and prove it with fake-device boundary tests; physical
activation and hardware qualification require a separate session authorization.
Do not ask for that authorization while independent software work remains undone.

Correct current “Brain has no audio device” and global-single-clock statements in
owning docs/README/status/architecture/diagrams. State clearly that Stagebox remains
the processing reference while Brain local I/O crosses a separate clock domain.
Preserve original drafts and dated evidence; do not rewrite past measurements.
Update contracts, scope instructions, roadmap and requirement-to-test matrix.

Independently review final integrated source and artifact hashes. Follow each
owner's PUBLICATION policy, enable versioned hooks, review new reusable scripts,
inspect live/staged state and complete-index/outgoing-history guards. Stage only
owned files. Root publishes providers before consumers to verified upstreams and
checks CI to completion. Never force-push or publish private audio/state/keys.
Safely fast-forward all twelve canonical repositories on both Pis to exact final
published revisions, preserving Pi4's Pi5-backed origins and unrelated work.
Synchronize the private ledger with independent receiving acknowledgment and final
manifests. Source synchronization is not deployment.

Prepare the exact bounded physical followup for the actual Brain card and Stagebox:
read-only inventory first; observed channel/socket and hardware-monitoring mapping;
separately authorized duplex activation with conservative output level; safe test
signal/microphone routing; measured independent-device drift/ASRC stability, roundtrip
and monitor latency with method/uncertainty; talkback release/failure tests; sustained
configured deadlines and output-path safety. Do not invent card identity or claim
hardware clock lock, acoustic quality, feedback immunity or physical qualification
from synthetic evidence. State the remaining physical gates without blocking the
complete software implementation on missing hardware.

Finish with a concise factual report and durable acceptance record: architecture
correction, real talkback/monitor paths and ASRC choice, all test results/skips,
actual operator/node/module/sample evidence, declared/measured bounds, physical
gates, worker model/effort, per-owner commits/push/CI, both-Pi source/ledger receipt,
disk cleanup and preserved work. Do not describe the full software path as complete
if any required software acceptance is still failing.
```
