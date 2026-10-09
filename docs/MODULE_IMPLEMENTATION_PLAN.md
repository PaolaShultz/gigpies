# GigPies integration: plan and implementation

Updated 2026-10-09. This document owns cross-module integration tasks from planning
through implementation and acceptance. Module-only work stays in the
[owning module plan](MODULE_IMPLEMENTATION_MAP.md). The same task must not have a
second status table in an index, handoff, capability matrix or knowledge note.

## Scope

GigPies is the live-band sound and lighting system. Runtime owners are GigPies,
SHR Desk, SHR Lightdesk, SHR PA, SHR FX, SHR REC and SHR Lux.
SHR-DAW (including Player), Drums, Synth, Sampler, Tone Over 9000 and Skills are
reference/code sources, not GigPies runtime modules or unfinished integration
requirements. Reuse only a needed implementation with provenance; do not add
instrument hosting, a DAW process or their standalone backlogs to GigPies.
[Components](COMPONENTS.md) owns the precise boundaries.

## How work is tracked

1. Give each task one stable ID, one owning document and one accountable owner.
   A cross-module task here names contributing repositories and their boundaries;
   do not mirror it as new tasks in each contributor's plan. A distinct provider
   algorithm/ABI task may live in its module plan and be a named dependency.
2. Keep outcome, scope, acceptance checklist, current state, evidence and exact next
   action in the same task card. Planning adds detail to that card; implementation
   updates it in place. Never append a contradictory second execution checkpoint.
3. Use PLANNED, IN PROGRESS, BLOCKED (with a concrete dependency), IMPLEMENTED,
   or ACCEPTED. Acceptance must name software, physical or musical scope. Passing
   software tests never closes an unperformed physical gate.
4. Keep only open work in the active queue. On completion, record the final result
   and evidence in its existing card, then archive that closed card once. Link the
   archive from the relevant technical contract; do not maintain a running done list.
5. STATUS is dated evidence, NEXT_SESSION is a pointer, and the module map is
   routing. Contracts describe behavior; acceptance reports contain measurements
   and retained failures. They do not maintain another backlog. Knowledge routes
   to these sources. Update the owning card with the behavior change, not later.

## Baseline evidence

The configurable Stagebox graph, actual PA v2/raw REC/source-clock FX, authenticated
transport and Desk controls have [modular software acceptance](MODULAR_ENGINE_ACCEPTANCE.md).
Brain duplex/ASRC, PFL/AFL and talkback have [integrated software acceptance](BRAIN_AUDIO_ACCEPTANCE.md).
Both consoles have provider-backed native software; Lux has real null-output
lighting authority and named analysis. Physical multichannel, controller/display,
fixture, acoustic and complete-show acceptance remain separate.

The embedded FX contract is fixed stereo delay, not SHR FX's standalone rack.
Current rendered mixer snapshots leave meters unavailable; schematic graphs are
not audio measurements. Offline automix results are not live automatic mixing.
See [dated evidence](STATUS.md), [module contracts](MODULE_CONTRACTS.md) and source.
The [previous plan](archive/tracking-before-2026-10-09/MODULE_IMPLEMENTATION_PLAN.md) preserves legacy GP-01..09/task0009 launch cards;
it is historical, not a queue to execute. Later accepted work supersedes its limits.

## Active work and dependencies

| Task | Owner / contributors | State and next action |
|---|---|---|
| [GP-METER](#gp-meter--measured-audio-on-the-real-desk) | GigPies lead; SHR Desk consumer | IMPLEMENTED / software-validated; owner-artifact P4 BLOCKED: supply reviewed manifest |
| [GP-FX](#gp-fx--writable-owner-effects-in-the-console) | GigPies lead; SHR FX provider, SHR Desk consumer | PLANNED: depends on owner FX-02/03 prepared writable contract |
| [GP-SHOW](#gp-show--audio-scenes-and-operator-recovery) | GigPies lead; SHR Desk consumer | PLANNED: define scene contents, safes and reviewed recall before implementation |
| [GP-AUTO](#gp-auto--live-soundcheck-and-bounded-assistance) | GigPies lead; SHR Desk consumer | PLANNED: needs trustworthy measurements and an explicit musical objective |
| [GP-REC](#gp-rec--complete-integrated-recording-workflow) | GigPies lead; SHR REC provider, SHR Desk consumer | PLANNED: audit existing host lifecycle before defining missing operator actions |
| [GP-H1](#gp-h1--physical-audio-qualification) | GigPies lead; PA/REC/FX owners as needed | PLANNED: separate physical session and observed rig required |
| [GP-H2](#gp-h2--physical-dual-console-operation) | GigPies lead; Desk/Lightdesk contributors | PLANNED: actual displays/controllers and separate physical session required |
| [GP-H3](#gp-h3--whole-show-qualification) | GigPies lead; all runtime owners | PLANNED: follows audio/console qualification and Lux LX-06 output |

## GP-METER — measured audio on the real Desk

State: **IMPLEMENTED / development-machine software-validated; owner-artifact P4 acceptance BLOCKED**.
Owner: GigPies integration. Contributor: SHR Desk client/presentation. This card
is the sole plan, checklist and evidence record. The user authorized scoped software
execution in GigPies and Desk on 2026-10-09. The subsequent user instruction
authorizes source commits and pushes; hardware activation remains excluded.

Outcome: actual channel, stereo main and every configured monitor level/clip
observation reaches the provider-backed native Desk alongside existing dynamics
feedback. Measured silence, unavailable, stale, invalid and clipped are distinct.
Software acceptance covers the production graph and authenticated delivery;
physical sound, device mapping and sustained deadlines remain separate.
Execution is on the development machine, not an RPi. P1–P4 require deterministic
software correctness checks only: no performance measurements, latency/CPU/RSS
benchmarks, two-Pi trials or hardware qualification. Synthetic signal assertions
verify meter arithmetic; they are not device measurements.

### Source findings and fixed design

- [Mixer::process_interleaved](../src/mixer.rs) already has raw samples, the
  actual crossfaded `strip.tick` result, mute/fader/pan sums and raw post-mute
  monitor sends. [EngineAuthority48::snapshot](../src/mixer_control.rs) sets
  `meters: None`; `RenderedSnapshot::validate` rejects populated meters. Keep
  C-AUDIO/rendered1/2 unchanged; filling that field would break compatibility.
- [LocalAudio::tick_raw_with_brain](../src/local_audio.rs) sequences raw analysis,
  mixer, pre-talkback operator capture, module composition and monitor talkback.
  [ModuleGraph::process_interleaved_brain](../src/module_graph.rs) adds actual wet
  return and FOH talkback before PA. Main meter must observe that sum, not the
  dry mixer output or `program_before_talkback`. No new PA/FX/REC ABI is needed.
- [ProcessingSnapshot](../src/processing_wire.rs) and
  [channel processing observation](../src/channel_processing.rs) already report
  detector gain reduction in nonnegative milli-dB, unavailable during transitions,
  bypass or fault. Preserve it and its independent freshness; it is not a level.
- [Paired readback](../src/paired_readback.rs) is a strict committed raw/Brain
  authority snapshot. Meter reads must not modify it, its read-admission latches,
  processing freshness, lease maintenance, held proofs or mutation retry history.
- Desk's [provider decoder](../../shr-desk/src/provider.rs),
  [remote connection](../../shr-desk/src/remote.rs) and
  [frontend worker/Update](../../shr-desk/src/frontend.rs) separate observations
  from commands. [render.rs](../../shr-desk/src/render.rs) fixture meters are
  simulator-only. [modules::Worker](../../shr-desk/src/modules.rs) supplies a
  bounded separate-worker precedent, but receipt-based freshness there is not
  sufficient for meters. Use a dedicated meter observer, not the control poll loop.

The following are the implemented GP-METER v1 boundaries. Acceptance evidence
and the remaining owner-artifact gate are recorded below:

| Tap ID / UI label | Exact sample boundary |
|---|---|
| `input ID + raw` / Input raw | Logical mapped f64 input before EQ, mute, fader; same source as raw REC/analysis, without modifying either |
| `input ID + processed` / Channel pre-fader | Actual `strip.tick` result including transition blend, before shared mute/fader/pan; never target coefficients |
| `main-l`, `main-r` / Main pre-PA | Actual dry + accepted wet + authorized FOH talkback immediately before PA input composition; without modules, actual dry main (do not invent wet processing) |
| `monitor ID` / Monitor bus | Each configured mono bus after shared mute/send/output safety gain and authorized monitor talkback, before physical patch/optional PA routing |

Main is explicitly **pre-PA**, not a physical output/protection meter; neither
meter nor clip proves ADC/DAC clipping, true peak, SPL or acoustic safety. PA output
metering is outside this slice. PFL/AFL/operator playback are unchanged and are
not substituted for these taps. Monitor IDs come from admitted topology/authority,
not screen banks or a fixed count. Carry physical/USB mapping through existing
structural readback rather than copying it into every observation.

Aggregate nonoverlapping 20 ms source windows (960 samples at 48 kHz; derive
`sample_rate / 50`, refusing nonintegral unsupported rates for telemetry only).
All samples contribute: peak=max(abs(x)), RMS=sqrt(mean(x*x)), clip count=number
with abs(x)>=1.0, including exactly full scale. Use overflow-safe scaled sum of
squares for finite over-range inputs. A nonfinite sample invalidates its tap for
that window; report invalid count and null levels, never measured silence. A graph
fault/quiescence invalidates affected processed/bus taps, even if safety emits
zeros. Valid muted output is measured zero. Raw evidence can remain valid when
only downstream processing faults. Publish only complete windows; discard partial
windows on gap, epoch/map change, reset or source recovery. Do not average across
identities or fill gaps. Existing DSP fault policy is unchanged.

Wire levels: integer milli-dBFS `round(20000*log10(amplitude))`, floor -120000;
include explicit `silent` (peak exactly zero), `below_floor`, and `over_range`
flags. Positive dBFS is valid. Peak/RMS null only for invalid/unavailable taps;
counts and sequence/frame/epoch/generation use decimal-string u64 counters.
Clip count is per window; Desk holds the red indication for one local second
only while data is fresh, with no provider latch/reset mutation. Missing windows
are visible loss, not a promise to capture every transient. No UI smoothing may
hide stale/invalid state or change numeric measurements.

New independent strict JSON contract `GP-METER:1`, `kind: meter_snapshot`:
read request names show/module/source epoch, query ID and expected map; writer,
lease, request_id and expected_revision are null. Reply echoes query and carries
show/module, source epoch, capability/map generation, topology identity/rate,
sequence, half-open `[first_frame,end_frame)`, acquisition age of the **first**
sample, publication loss count, validity/reason and ordered named tap records.
Remote reply remains inside the authenticated session envelope. No arbitrary
client-selected taps or new grants. A valid current authenticated reader may
observe the same inventory as existing snapshots; revoked/unpaired sessions fail.
Local binding retains private same-UID access. Unknown contract/version is a
bounded unsupported result; old providers stay usable with meters unavailable.
Old consumers never receive unsolicited new fields/messages.

Poll at most 25 Hz per observer, one outstanding query, total query/assembly
budget 100 ms; provider windows are 50 Hz. Provider computes acquisition age from
its own monotonic acquisition timestamp, never from latest request/serialization.
Desk conservatively adds the entire local query elapsed time, then elapsed time
since receipt: no comparison of unsynchronized host clocks. Freshness limit is
250 ms from window start. Keep original deadline across partial frames/pages;
repeated sequence may increase age but cannot reset its prior expiry. Reject
future/regressing frames, malformed windows, map/epoch/session mismatch, duplicate
IDs, unknown fields/keys, invalid numeric relationships and oversize inventories.
After identity change clear cached meters and await matching current topology plus
a complete new window. Reconnect/unsupported/stall cannot grant, renew, rearm,
recall or replay any mutation. A stopped source with responsive control becomes
stale; receipt alone never makes it live.

### Dependency-ordered delivery and acceptance

Each phase remains unchecked until its evidence is entered below. New files named
here are planned boundaries; existing links identify the inspected implementation.

**P1 — Provider contract and bounded acquisition (GigPies).** Add `src/metering.rs`
for prepared per-tap accumulators/window identity and `src/meter_wire.rs` for strict
request/reply validation; register in `src/lib.rs`. In `src/topology.rs` admission,
account for accumulators, two preallocated SPSC window slots and serialization
capacity using checked arithmetic and admitted counts. Use the existing
[analysis handoff](../src/analysis_stream.rs) pattern, not its PCM protocol or Lux
subscriber limits. A full queue drops a whole meter window and increments loss;
never waits, allocates, locks, serializes, does I/O or retries on the render path.
Worker drains bounded slots and retains only the latest complete observation.
Strings, storage and tap maps are prepared off render; no per-sample strings/logs.

Instrument `Mixer::process_interleaved` for raw/processed taps and
`ModuleGraph::process_interleaved_brain` at its final main sum; finish monitor taps
in `LocalAudio::tick_raw_with_brain` after talkback. Add only bounded observation
hooks, with meter lifetime/reset owned by LocalAudio and source/map changes in
`quiesce_source`, `recover_source` and structural commit. Preserve arithmetic/order
and owner handles. Existing LocalAudio tick also performs control I/O; do not
misdescribe it as a realtime-safe callback or refactor the whole host. The new
sample hooks and handoff must be independently realtime-safe.

- [x] Analytic silence/DC/impulse/coherent sine and independent summation checks
  verify peak/RMS/clip/window boundaries, invalid/over-range behavior and reset.
  Amplitude .5 DC gives -6021 milli-dBFS peak/RMS; a whole-cycle .5 sine gives
  -9031 milli-dBFS RMS (rounding tolerance 1 milli-dB, floor checked separately).
- [x] Callback allocation guard plus bounded-capacity/drop tests prove no new
  allocation/lock/I/O; meter disabled, saturated and disconnected runs have exactly
  identical audio samples. Reject meter capacity independently of audio operation.
- [x] Add provider-produced synthetic `tests/fixtures/gp-meter/v1/` corpus with
  schema, actual source revision and SHA-256 manifest, including all refusal and
  identity/age cases. Additive contract documentation belongs in existing
  `docs/MODULE_CONTRACTS.md`; no second tracker.

**P2 — Provider delivery (depends on P1).** Extend LocalAudio's `Incoming` decode
and bounded query dispatch for the new contract; JSON/cache publication is outside
sample processing. Extend `src/remote/policy.rs::check_writer`,
`src/remote/host.rs::HostAuthority::dispatch` and existing remote framing in
`src/remote/wire.rs`/`src/snapshot_pages.rs` only where required. Reads must bypass
control admission/lease state changes in `src/remote/authority.rs`.
Keep 64 KiB frames and existing <=1 MiB/16-page immutable document bound, including
outer remote wrapper; meter assembly has the stricter 100 ms deadline. Admit only
inventories fitting the bound; oversized telemetry reports unavailable, never
silently truncates channels or rejects an otherwise valid audio topology.
Use existing finite connection/session capacities, no new per-client queues in
audio; one reply in flight, discard on timeout, immutable page identity throughout.

- [x] Actual private Unix and mutually authenticated QUIC requests decode the
  real provider corpus; unauthorized/revoked/wrong-session queries fail closed.
- [ ] Slow reader, partial page, queue saturation, duplicate query, missing window,
  source stall, epoch/map change and provider restart cannot refresh old data,
  splice pages, block audio or change lease expiry/readiness/revision/held proof.
- [x] Existing rendered/processing/paired fixtures and old client behavior remain
  accepted byte/schema-compatible. Freeze provider fixtures before Desk delivery.

**P3 — Desk observation and presentation (depends on accepted P2 fixtures).**
In `/home/shome/p/shr-desk`, add `src/metering.rs` codec/cache/observer worker,
registered in `src/lib.rs`; reuse `provider.rs` strict bounded document parsing and
`remote.rs::Connection` authenticated framing. Extend `src/local_audio.rs` transport
adapters only as needed for a separate read-only connection with no grant/renew.
A dedicated worker and latest-only UI mailbox prevent 100 ms meter timeouts from
stalling the existing control worker/50 ms held-action service. Respect the
existing 16-session deployment budget, including this observer; no automatic
capacity increase. Probe once per attach; unsupported stays unavailable until an
explicit new attach, while transient read failures recover without commands.

Extend `frontend.rs::Update`, `Frontend` scene construction and `render.rs` shared
meter primitives for actual banked input raw/processed selection, stereo main and
configured monitor buses. `native.rs::offscreen_at` consumes the same scene; leave
the simulator explicitly fixture-backed. Show tap label, peak/RMS, clip and age;
stale values may remain dimmed with STALE and age, invalid/unavailable use distinct
text, and silence uses an empty bar plus measured-silence indication. Keep GR
separate and independently aged. Meter recovery must not dismiss pending reviews,
change selection, synthesize input or restore held talkback.

- [x] Copy exact producer corpus/hashes into Desk `tests/fixtures/gp-meter/v1/`;
  test codec/freshness/recovery and legacy fallback before UI acceptance.
- [ ] Extend `tests/frontend.rs`, `tests/frontend_real.rs`, `tests/gp07_frontend.rs`
  and `tests/gp15_frontend.rs` as appropriate: all banks/buses, GR coexistence,
  clipping/silence/stale/invalid, context loss and held-action isolation.
- [x] CPU-native offscreen checks inspect actual provider-fed scenes at normal and
  resized viewports: readable labels, no clipping/overlap, no fabricated graphs.

**P4 — Integrated software acceptance (depends on P1–P3).** Extend existing
GigPies `tests/modular_engine.rs`, `tests/modular_owners.rs`, `tests/brain_owners.rs`
and `src/remote/tests.rs`; add focused `tests/metering.rs` in each changed owner.
Use finite synthetic signals with independently calculated expected values, not
another meter instance or decoder round trips as oracle. Exercise 16/17/32/48
inputs with independently varied monitor counts and nontrivial capture/output
maps, including last input/bus and bank boundaries. Check mute/fader/pan, EQ and
compressor transitions, actual FX return, FOH/monitor talkback and PA protection.

- [ ] Actual LocalAudio + owner PA/FX/REC artifacts → authenticated provider →
  independently built Desk client → native scene agrees with independent sample
  windows (1 milli-dB quantization tolerance; exact frame/clip counts). Run both
  local and remote delivery, not solely copied fixtures or a mock server.
- [ ] Compare every raw recorded PCM24 sample and analysis sample against original
  source-frame/input references exactly; check dry/protected output, PFL/AFL and
  no implicit talkback sidetone with meters attached, stalled and recovering.
- [ ] Source stop with control alive, Desk loss, worker loss, revoked session,
  source/provider restart, map change and delayed old replies become visibly
  stale/unavailable and resume only with fresh identity. Control leases still
  expire; no automatic grant/rearm/mutation replay. Preserve existing 150 ms
  held deadman, 50 ms heartbeat, 240-frame fade and 250 ms control freshness gates.

### Verification, evidence and exact next action

Before execution (2026-10-09), planning inspected the named seams and updated
only this card. The authorized execution preserved that existing planning diff
and updated this same record; no conflicting documentation edits were found.
During execution run focused normal meter tests first, then full normal default
and `hardware-host` suites in GigPies and default/`native` suites in Desk because
render/schema/shared state changes require them. For each configuration run
`cargo +1.97.1 test --locked -j1 --all-targets`, warnings-denied Clippy and release
build, plus `cargo +1.97.1 fmt --check` and each owner's publication Python checks.
Prefix every Cargo invocation with
`flock -xn /home/shome/p/.gigpies-build.lock env CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0`;
serialize owners/configurations, keep normal targets, inspect disk per Desk policy.
Do not weaken deadlines or omit failed tests; record failures and skipped classes.

Actual owner/provider drivers are explicit finite development-host opt-ins using
reviewed artifact hashes and isolated synthetic loopback/private Unix endpoints.
No remote RPi endpoint or lab reservation is needed for this slice. Follow
[modular acceptance](MODULAR_ENGINE_ACCEPTANCE.md),
[Brain acceptance](BRAIN_AUDIO_ACCEPTANCE.md) and
[Desk development](../../shr-desk/docs/DEVELOPMENT.md). Run required owner cases via
`--test modular_owners -- --include-ignored` and
`--test brain_owners -- --include-ignored` with their documented artifact inputs.
Native rendering uses inspected `/usr/share/vulkan/icd.d/lvp_icd.json` via
`VK_ICD_FILENAMES`, DISPLAY/WAYLAND_DISPLAY unset, and `native::offscreen_at`.
No physical PCM/MIDI/DMX/service activation. Full-show/load, real displays/controllers,
ADAT/socket/acoustic and listening gates require separate authorization; historical
music/research/exhaustive matrices remain opt-in and outside this slice.

| Phase | Implementation | Evidence / next action |
|---|---|---|
| P1 | IMPLEMENTED / software-validated | Analytic windows, allocation/drop guard, exact audio comparison, independent meter admission and frozen provider corpus |
| P2 | IMPLEMENTED / software-validated | Actual private Unix and mutual-TLS QUIC reads, revocation/read-only admission; strict versioned corpus and page/deadline bounds |
| P3 | IMPLEMENTED / software-validated | Independent codec/cache/observer; 20 fresh actual provider-fed scenes, 40 exact CPU-native RGBA comparisons; FOH topology attachment corrected |
| P4 | Independent dry software checks validated; owner-artifact acceptance BLOCKED | Actual 16/17/32/48 LocalAudio and independently built Desk/native driver pass; reviewed GP05 PA/FX/REC manifest absent |

Execution results (2026-10-09), local development machine:

| Owner/configuration | Complete normal all-target suite | Warnings-denied Clippy | Release build |
|---|---|---|---|
| GigPies default | 399 passed, 17 explicitly ignored | PASS | PASS |
| GigPies `hardware-host` | 476 passed, 32 explicitly ignored | PASS | PASS |
| Desk default | 249 passed, 13 explicitly ignored | PASS | PASS |
| Desk `native` | 251 passed, 13 explicitly ignored | PASS | PASS |

All used the exact locked serial command prefix above. Feature rows add
`--features hardware-host` or `--features native` to test/Clippy/build; Clippy
uses `--all-targets -- -D warnings`. Both `fmt --check` commands passed.
Publication Python regressions passed (GigPies 6, Desk 9); real complete indexes
and disposable candidate indexes including every named new file passed the guard.
Real indexes remain untouched. Local document targets and `git diff --check`
passed; source-linked knowledge routing and validation passed. Normal native
suites had display/ICD discovery unset; the explicit driver below selected CPU.

Provider evidence: analytic silence/DC/sine/impulse/floor/full-scale/over-range,
nonfinite and extreme-finite RMS; allocation/deallocation guard, two-slot drops,
partial/gap/reset discard, independent meter admission and exact unchanged audio
with capture disabled/saturated/disconnected. Actual mapped LocalAudio references
cover 16/3, 17/1, 32/5 and 48/7 input/monitor configurations. Actual private Unix
and mutual-TLS QUIC queries pass without control admission/revision changes;
revocation fails closed. The focused authenticated stopped-source test also
passed after adding a second read with rendering stopped: sequence/frames/taps
stay identical and first-sample age exceeds 250 ms while control remains alive.
Acquisition is stamped at block entry, before control servicing.

Consumer evidence: exact copied producer corpus; strict codec, query/identity,
regression/overlap, repeated-window expiry, 100 ms assembly deadline, page identity,
16-page refusal, one-second clip hold and independent input/monitor banks.
Meter state uses `Frontend::meters` and its own latest-only mailbox; the control
`Update` and held-action worker retain their existing ownership. Admitted
rendered-v2 topology supplies FOH inventory; structural readback is a fallback.
F11 opens meters, F12 switches raw/pre-fader, PageUp/Down selects monitor banks;
GR retains its independent processing age. Legacy rendered/processing/paired
fixtures and their null meter fields are unchanged.

Explicit opt-ins passed: GigPies `--test metering -- --ignored` with
`GP_METER_FIXTURE_OUT=tests/fixtures/gp-meter/v1` and
`GP_METER_TOPOLOGY_DIR=/home/shome/p/gigpies/artifacts/gp-meter/topologies`
(two exporters). Desk `--features native --test metering
actual_local_provider_native_scene -- --ignored --nocapture` used
`GP_METER_PROVIDER=/home/shome/p/gigpies/target/release/gigpies-headless`, that
same topology directory, `GP_METER_CPU=1`, and
`GP_METER_EVIDENCE_DIR=/home/shome/p/shr-desk/artifacts/gp-meter/screens`.
DISPLAY/WAYLAND_DISPLAY/VK_ADD_DRIVER_FILES were unset; VK_DRIVER_FILES and
VK_ICD_FILENAMES both named the inspected `/usr/share/vulkan/icd.d/lvp_icd.json`.
The independently built client consumed five actual private providers: 16/3,
17/1, 32/5, 48/7 and 17/13, with reversed capture/output maps. Independent
DC/dry-main/each-monitor peak/RMS, exact 960-frame windows and zero clip/invalid
counts passed. First/last inputs in both tap modes yielded 20 fresh scenes and
40 exact RGBA matches at 1920x1080 and 1280x720. Source-child stop made meters
stale. Retained normal/last-input/second-monitor-bank images were visually checked
for readable labels and distinct measured silence; scene bounds checks passed.

Provenance: `tests/fixtures/gp-meter/v1/manifest.json` in both repositories is
byte-identical, SHA-256
`5c72ac8bcb6de1710a3c01acc07fc4655a4ea2915c06df27d60e06b44732f0b7`.
It records the actual producer base revision plus every changed source witness
and fixture hash, explicitly marking this uncommitted slice. Final provider
executable SHA-256 is
`f77f91a006e18f87d24bd4dcde2b0ea594e17225d55a1c470880250525e78cd8`;
final native Desk release SHA-256 is
`6cb95e6bcc6fb9f8c8dc6580fddad9bcc30a9b36523f42d4d00c5e17ff2833ba`.
Each owner's ignored `artifacts/gp-meter/evidence.json` retains source/lock,
executable, corpus and log hashes; Desk also retains the actual native test
executable hash and 20 scene hashes. Commands/results and available failed logs
are under each owner's `artifacts/gp-meter/logs/`.

Failed attempts were resolved without changing production deadlines: a test
used nonexistent `Mixer::clone`; two constructed test fixtures had inconsistent
level/flag relationships or skipped monitor IDs; Clippy found nested conditionals and an
explicit eight-argument snapshot boundary. Tests/style were corrected. An
exporter invocation omitted its required output paths and was rerun explicitly.
48/13 failed existing control-snapshot admission; admitted 17/13 proves the second
monitor bank. The actual native run caught FOH waiting for structural metadata;
it now uses admitted rendered topology. CPU inspection paused the UI pump, so
the driver now awaits a fresh observation before each frozen scene. Final runs
passed. Disk review found ample free space; shared caches/evidence were preserved.

**BLOCKED artifact-dependent P4:** no reviewed `GP05_MANIFEST` (PA/FX/REC owner
artifacts) was supplied/found; `GP_PA_V2_FIXTURES` and `GP14_PA_FIXTURES` are not
configured for this session. Required `modular_owners`/`brain_owners
-- --include-ignored` drivers, actual owner wet/protected/REC/analysis sample
comparisons and owner-backed Desk local + remote acceptance were not run.
Mock-owner normal suites and dry-provider/native evidence do not replace them.
The corresponding P4 and comprehensive fault/recovery checkboxes remain open.
Other historical/exhaustive, hardware, listening, full-show/load and measurement
opt-ins were intentionally skipped under this card's scope. No PA/FX/REC ABI,
owner algorithm, device/service, dependency, commit, push or deployment changes.

Remaining uncertainties are verification gates, not open design alternatives:
inventory serialization bounds must pass deterministic size assertions in P1/P2;
freshness and timeout behavior use controlled clocks/fault injection, not loaded
latency measurements. Accepted development-host owner artifacts must be available
for P4. If absent, mark that artifact-dependent acceptance BLOCKED here and finish
independent software work. RPi performance and hardware qualification are deferred
to separately authorized sessions and do not gate development completion. No owner algorithm
change is anticipated; a required PA/FX/REC ABI change stops this slice for review.

Exact next action: supply the reviewed PA/FX/REC manifest and owner fixture paths,
then execute the remaining P4 owner drivers and owner-backed local/remote sample
acceptance. Independent development-machine software work is complete. Preserve live dirty
work, stage only named files if committing, and follow both owners' publication
guards. Deliver provider before consumer; Desk coordinator owns upstream pushes
under its continuation policy. The subsequent user instruction authorizes source commits and pushes for this
validated slice. Deployment and hardware activation remain excluded.
Knowledge already routes to this card; maintain that route without copying state.

### Standalone execution prompt

```text
Implement GP-METER P1–P4 in /home/shome/p/gigpies and /home/shome/p/shr-desk,
using the GP-METER card in gigpies/docs/MODULE_IMPLEMENTATION_PLAN.md as the
complete design and sole progress/evidence record. This instruction authorizes
scoped runtime changes in those two repositories and finite synthetic local/
authenticated loopback software acceptance on the development machine, not RPi.
No performance/device measurements, benchmarks, two-Pi trials, physical devices
or service deployment; use deterministic signal, capacity and clock assertions.
Read each AGENTS.md, the card and linked acceptance/contract sources; load the
GigPies zk index and inspect live Git before editing. Preserve other workers' work.
Deliver provider contract/acquisition, bounded authenticated observation, actual
Desk presentation, then independent 16/17/32/48 source-to-Desk/native/raw-sample
acceptance in that order. Preserve all authority, timing, raw REC/analysis,
PA/FX, PFL/AFL and talkback behavior; never refresh old audio from receipt time.
Run focused then required whole suites serially under the shared build lock,
Rust 1.97.1 --locked -j1 and CARGO_INCREMENTAL=0. Record exact evidence and opt-ins
in the card. Stop dependent work on incompatible source drift, required new owner
ABI, missing development-host artifact or a failed safety/identity gate; finish
independent checks without weakening limits. No new tracker, broad refactor,
dependency upgrade, FFT, automix, scenes, writable FX or new device owner.
Keep other siblings read-only; no model selection or agents. Local changes only:
report reviewed diffs/evidence and remaining gates; do not push, deploy or activate
hardware. Provider must be accepted before consumer publication in a later
explicitly authorized publication session. Update source-linked knowledge only
when needed and run /home/shome/Documents/knowledge/.zk/validate.sh.
```

## GP-FX — writable owner effects in the console

State: PLANNED. Outcome: real prepared wet-effects edits and readback from Desk
through GigPies into SHR FX, preserving independent dry/protected/recorded paths.
Provider design and DSP belong to [SHR FX's plan](https://github.com/PaolaShultz/shr-fx/blob/main/docs/GIGPIES_IMPLEMENTATION.md).
Next: resolve FX-02/03 units, f64 preparation/publication/retirement, bounds and
fault/tail semantics before specifying the host and Desk adapter. Acceptance must
check actual wet samples and failed edits, not just ACKs or standalone rack tests.

## GP-SHOW — audio scenes and operator recovery

State: PLANNED. Outcome: browse/preview/cancel/recall a bounded audio scene with
explicit safes, revision checks and truthful readback. Next: define saved versus
machine-only state and retained human holds. Exclude automatic unmute, resumed
recording/talkback and implicit cross-system lighting actions. Test atomic refusal,
interrupted recall, reconnect and recovery against actual providers.

## GP-AUTO — live soundcheck and bounded assistance

State: PLANNED. Outcome: use real measured sources to propose useful bounded mix
changes while preserving human control. Next: after GP-METER, select one defined
soundcheck/assistance behavior and connect existing offline logic where applicable.
Scope authority machinery already exists; do not recreate it. Require held-out
engineering evidence and separate human listening before musical acceptance.

## GP-REC — complete integrated recording workflow

State: PLANNED. Outcome: safe operator start/stop, readiness, written-versus-queued
progress, integrity and finalized outcome for actual raw REC. Next: inspect existing
host actions and Desk read-only health, then plan only missing operator/storage
feedback. REC owns writer/recovery; standalone player and shell completion do not
gate this task. Unknown durability must stay unknown.

## GP-H1 — physical audio qualification

State: PLANNED; requires a separate authorized physical session.
Next: observed Stagebox/Brain inventory and explicit socket/device maps, UMC/ADAT
clock relationship if that rig is present, duplex drift, electrical latency and
safe fault/rearm. Follow [modular physical procedure](MODULAR_PROCESSING.md#mapping-and-physical-acceptance)
and [Brain procedure](BRAIN_AUDIO.md#bounded-physical-followup).
Keep H8's unresolved timing/right-return evidence. Qualify sustained configured
hardware deadlines and acoustic protection; software profiles are not hardware proof.

## GP-H2 — physical dual-console operation

State: PLANNED; requires a separate authorized physical session.
Next: verify the two actual display/controller identities, separate input/LED
ownership, focus/held-release, restart/reconnect and operator usability. Follow
[roles](ROLE_BINDING.md) and each console's controller/native contract. Injected
role descriptors and offscreen rendering do not qualify physical operation.

## GP-H3 — whole-show qualification

State: PLANNED; requires qualified audio/console paths and a reserved session.
Next: measure the actual combined Stagebox recording/protected audio and Brain
FX/Lux/analysis/two-console workload, deadlines, control responsiveness, memory,
thermal behavior and failures. Physical lighting requires Lux LX-06 first.
Use [Brain responsibilities](BRAIN_CONSOLE_PLAN.md) and owner acceptance limits.

## Verification and handoff

Follow each changed owner's AGENTS.md and current test policy. Use focused normal
regressions during work and required complete normal suites for shared render,
schema, transport, routing or safety changes. Actual-owner/provider/consumer checks
must use the production graph. Keep historical auditions, exhaustive research,
physical and long-load campaigns explicit. Serialize builds under the shared lock;
use pinned Rust, locked dependencies, jobs=1 and CARGO_INCREMENTAL=0.

Before completion, update this task's state/checklist/evidence/next action, preserve
other workers' edits, inspect named diffs and publication guards, and maintain only
source-linked knowledge routing. Follow the owner's publication authorization;
source acceptance does not deploy services or authorize device activation.
