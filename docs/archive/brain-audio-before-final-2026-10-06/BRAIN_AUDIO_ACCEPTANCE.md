# Brain audio acceptance

Status: **INCOMPLETE**. Production implementation and focused software evidence
exist. Historical trial16 passed simultaneous 16-input checks on an earlier
candidate. All seven final scenarios remain pending: simultaneous 16/32/48 inputs,
duplex stall, duplex restart, controller stall and Stagebox restart. Final artifact
review, publication/CI and exact source/ledger receipts also remain pending. This document does not authorize physical audio activation.

This record covers the Brain duplex audio integration requirements.
[Architecture and operation](BRAIN_AUDIO.md), [control](BRAIN_AUDIO_CONTROL.md),
[clock/device bounds](BRAIN_AUDIO_BRIDGE.md) and
[authenticated transport](REMOTE_TRANSPORT.md) own the implemented contracts.
Stagebox remains the processing/recording reference. Brain owns one local duplex
endpoint with a separate clock; both new audio crossings use Rubato 5.0.1 ASRC.
Source-clock FX and raw recording retain their original timeline.

## Requirement, implementation and evidence

| Requirement | Production owner/path | Evidence and remaining gate |
|---|---|---|
| One duplex endpoint, explicit mapping/capabilities, safe startup/reopen | `brain_audio/device.rs`, `host.rs`, `brain_runtime.rs`, `bin/brain.rs` | Fake adapter exercises the same host, mapping, format, partial-transfer, fault and readback boundaries; raw ALSA diagnostics implemented. Physical opening not exercised. |
| Physical Stagebox composition and source-clock FX | `host/duplex.rs`, `remote/runner.rs`, `device_epoch.rs` | Shared authority advances once per captured48 frames; authorization before I/O, durable fresh epoch, map/readback checks and late partial-playback refusal tested. Source-agnostic FX is explicit. Sustained physical deadlines remain unqualified. |
| Independent clocks and bounded conversion at both crossings | `brain_audio/bridge.rs`, remote host and Brain renderer | Independent numerical references, allocation guards, five optimized clock cases including combined actual FakeDuplex/BrainHost/both bridges passed. |
| Actual monitor destinations and protected FOH talkback | `local_audio.rs`, `mixer.rs`, `module_graph.rs` | Actual PA/FX/REC owner test at 16/32/48; every five configured test monitor destinations, unselected silence and independently assembled PA reference. Five is a test configuration, not a product limit. Network acceptance pending. |
| Main, performer mix, PFL/AFL; no audience mix modification or implicit TB loop | Mixer tap and pre-TB program/monitor capture | `brain_control`, `brain_owners` verify real taps, centered pre-mute/fader/pan PFL, stereo AFL, selection isolation and pre-TB operator feeds. |
| Held-action lifecycle and independent scopes | `brain_control.rs`, LocalAudio, remote policy/host | 50ms heartbeat, observation-bound 150ms deadman and 240-frame fade; tombstones, stale/replayed messages, pending mutation, lease/FOH permission and fault regressions. Packet admission rejects expiry without requiring another render. |
| Requested/applied/ready distinction; explicit rearm | GP15-brain/device, runtime ArmFence, host readiness | Actual device/bridge identities gate readiness; replacement requires acknowledgment, closed observation and later explicit arm. Source selection invalidates old queued playback. |
| Existing FX/analysis/REC preserved | Dedicated Brain media role/queues plus existing owner graph | Actual module samples and raw recording checked; remote coexistence regressions pass. All-channel two-Pi sample evidence pending. |
| Stable intent only; dynamic dimensions | Structural intent and GP15 contracts | Arm/holds/grants excluded from persistence; producer fixture corpus includes 16/17/32/33/48 inputs and independently varying monitor counts. |
| Actual Desk provider/client/frontend | SHR Desk Brain/device actions and observations | Producer fixtures, keyboard/controller/focus safety, device epoch/map draft/review/retry fences and CPU-headless presentation tested by Desk owner. Actual endpoint runs remain incomplete. |
| Finite authenticated capacity and isolation | Remote session/proxy/host | Sixteen-session deployment budget; actual TLS test admits16, refuses17 without eviction, and accepts fresh replacement. At least five simultaneous scoped controller/worker roles are required. Loaded timing is a separate gate. |
| Publication and synchronized recovery | Owning publication policies and private ledger | Pending final exact-source validation, provider-before-consumer publication, CI and twelve-repository/two-node receipts. |

## Declared thresholds and observed results

Numerical bridge tests use independently declared signals and projections, not
another instance of the same resampler as oracle. Normal regressions pass the
following thresholds; these are acceptance limits, not published measured extrema:

| Property | Acceptance limit |
|---|---|
| Gain through 18kHz | Within 0.1dB |
| 23–24kHz stopband | At least 80dB rejection |
| Inactive channel | Below −100dBFS |
| Unity-ratio 997Hz analytic waveform, amplitude0.5 | Absolute error at most0.001 |
| Independent impulse area, amplitude0.5 | Error below0.001 |
| Admitted steady oscillator skew | Both signs through1000ppm; hard controller limit1500ppm |
| Applied ratio slew | At most100ppm/s |
| Bridge failure fade | At most240 output samples, 5ms at48kHz |

The analytic coefficient-grid delay is 127−1/128 frames; reported128-frame delay
is a conservative budget checked against the independent impulse/phase references.
Production target occupancy is960 source frames (20ms nominal), plus approximately
128 output filter frames (2.67ms nominal); first ready48-frame packet boundary is
1056 queued frames. These are queue/filter components, not measured physical or
one-way network latency. Already submitted hardware PCM has a separate configured
buffer tail. Physical phase uncertainty is unknown and clock lock unverified.

The combined production-host virtual-clock test used480-frame target/1024-frame
capacity and fixed48-frame packets. Both directions passed with zero under/overruns,
actual returned/submitted reference samples and unmapped-slot silence:

| Clock skew | Virtual duration per sign | Observed occupancy across both crossings/signs | Largest settled mean correction error |
|---|---:|---:|---:|
| ±10ppm | 1250s | 431–528 frames | <0.46ppm |
| ±100ppm | 125s | 424–524 frames | <3.32ppm |
| ±1000ppm | 120s | 158–768 frames | <4.32ppm |

The predeclared mean-error limit was30ppm; instantaneous occupancy correction is
not an oscillator measurement. All five optimized clock cases passed in145.98s
wall time, including separate jitter, slow signed ramp and outside-envelope closure.
The durations exceed uncompensated buffer exhaustion. No physical clocks were used.

Focused actual-owner acceptance passed12 control and4 owner/render tests. It checked
294,912 exact recorded PCM samples across96 stems and73,728 protected PA output
samples against independently assembled input/reference processing, including the
configured −24dBFS ceiling. Host allocation **and deallocation** guards cover mixer,
actual talkback sample helper and module graph sections, including fade/nonfinite
failure. These guards do not intercept allocations inside independently loaded
foreign libraries; owner-library realtime evidence remains separately required.

## Validation state and reproduction

An earlier producer default normal suite passed371 tests, with0 failures and15 opt-in tests
ignored across53 targets. The hardware-host normal suite passed424 tests, with0
failures and30 opt-in tests ignored across56 targets. Formatting and warnings-denied
Clippy passed in both configurations. The final Brain and remote release executable
rebuild passed after the bounded monitor handover correction.
The focused physical-composition hardware library suite passed66 tests. Desk
at `0ee87d6` passed 184 default and 186 native normal tests, with 11 opt-ins ignored
in each configuration; 32 focused local-audio tests, 2 focused remote-parser tests,
formatting and both Clippy configurations passed.
The external release driver and default/native release executables passed.
Actual endpoint acceptance remains pending. One
CPU-headless test rendered1920×1080,960×540,540×960 and3840×2160 with actual device
health and capture-drop telemetry. An initial incorrect filter ran zero tests and
was corrected; that empty run is not counted as acceptance.
Independent source reviews retained exact hashes and identified/fixed device intent
pinning, stale route readiness, held capture tagging and connection-budget issues.

Use Rust1.97.1, locked dependencies, one job, incremental compilation disabled and
the shared parent-held nonblocking build lock. Normal commands include:

```sh
flock -xn /home/shome/p/.gigpies-build.lock env CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 \
  cargo +1.97.1 test --locked -j1 --all-targets
flock -xn /home/shome/p/.gigpies-build.lock env CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 \
  cargo +1.97.1 test --locked -j1 --all-targets --features hardware-host
```

Long historical/research/audition cases remain excluded from normal runs. The five
affected clock opt-ins were explicitly run; reproduce after relevant changes using
`cargo +1.97.1 test --locked -j1 --release --features hardware-host --test brain_bridge -- --ignored`
under the same lock/environment. Actual owner evidence uses `--test brain_owners
-- --include-ignored` with explicitly supplied accepted owner manifest and PA
fixtures; it never downloads media. Historical unrelated auditions and exhaustive
research were intentionally skipped. Actual network drivers are ignored tests and
require a finite mutually acknowledged reservation, reviewed exact binaries and
private configurations; they must not be run as ordinary CI or without that scope.

## Pending integrated evidence

Trial16 passed the16-input software profile, with all processes successful and
zero monitor underruns/overflows. Nineteen active PFL windows matched within1e-12;
two closed selection-boundary windows were exactly silent. Their initial active-wave
reference failure is retained: independent review corrected the oracle only for
complete closed state at the matching successful Desk command/frame/revision/selection,
with independently predicted pre-close peak. Active thresholds and coverage remain
unchanged. The1248 post-ASRC reference samples gave−0.03628dB gain error; selected
monitor-destination4 talkback gave−0.00729dB across5712 samples, with912 released
samples silent. Simultaneous talkback/monitor/FX/REC evidence covered2256 distinct
playback frames against the480-frame minimum. These are software observations.

Initial attempts exposed
missing host timing utility, incorrect test-only output editing, required explicit
read-only reconnect after output-map replacement, and insufficient legacy connection
capacity. A fifth attempt exposed runtime-only capture-queue telemetry absent from
the strict Desk schema; the shared producer fixtures and explicit consumer field
validation were corrected. A sixth attempt exposed reversed processing of queued
stateful pending/final replies; Desk now preserves their wire order while coalescing
raw observations. A seventh exposed a valid final arriving before its required raw
readback; bounded continuation and transport deadline propagation were corrected across
Unix, the page wrapper and QUIC. Focused and full Desk tests pass; actual endpoint
acceptance remains pending. An eighth trial applied sustained talkback before a
heartbeat was rejected as stale/future. It exposed receipt-age tracking that could
accept a queued observation as the answer to a newer query. Ordered query-probe
freshness was corrected without changing provider limits; full default/native
validation and releases passed; actual network acceptance remains pending. Those failed
attempts remain failed evidence. A ninth trial correctly refused a stale UI-pinned
revision before PTT: the external test driver had not waited for the monitor
client to observe another client’s confirmed FOH change. Test-only fresh raw/Brain
revision barriers were added without replaying mutations. A tenth trial reached
held talkback, then lost protected FOH authority; available evidence cannot
distinguish lease expiry from writer disconnection. Review found a superseded
Brain/raw observation pair that could stall passive-client refresh, and discarded
monitor packets on talkback renegotiation despite unchanged monitor identity.
Both corrections passed focused and full normal validation. An eleventh trial
retained simultaneous monitor/talkback/FX sample evidence but again lost FOH
authority. Review found that a full1900ms receive wait could suppress the existing
100/250/500ms identical-request retry schedule after backpressure. The new regression
failed with the old receive loop and passed with the independently reviewed correction;
26 focused local-audio tests and full default/native suites (178/180 tests) passed.
Both Clippy configurations and all release builds passed. A twelfth trial still
lost FOH authority; the new diagnostics showed a fresh client update with an expired
local lease. Review found cached-revision renewal admission and hidden terminal
renewal refusals. Exact trial renewal outcomes were not logged. A deterministic
regression reproduced the old implementation returning success without lease
extension. The reviewed GP15 correction requires its own matched Brain revision
and paired raw observation before allocating a new renewal request, and reports
terminal refusals without replay or lease extension. Focused and full validation,
formatting, both Clippy configurations and all release builds passed. A thirteenth trial sustained talkback and completed key-up release, then refused
the next press. The driver had not waited for fresh admission again. A
reviewed test-only barrier now requires current paired readback, a live lease and
no hold before each new press, within the original deadline and without regrant
or replay. Its targeted checks and release build passed. A fourteenth trial
correctly timed out at that barrier: Brain probe state remained invalid after
release while raw and device observations stayed fresh. Review found normal
release/context cancellation could permanently invalidate probe provenance; the
first triggering error had been overwritten, so exact trial causation remains
conditional. The reviewed lifecycle correction separates local cancellation from
terminal wire/provenance faults, retains the first fault reason, and requires a
new matched probe before a later explicit hold. Its negative-control test fails
with the old catch-all handler; focused and complete default/native validation,
both Clippy configurations and release builds passed. The same trial recorded two genuine monitor starvation events near
source transitions. Stable sampled windows showed no later increases, but sparse
observations cannot prove exact ordering. These lifetime counters still fail the
unchanged zero-underrun acceptance gate. A bounded authenticated silence handover
has passed independent source review, three focused production-host/bridge tests,
complete producer suites, both Clippy configurations and the release rebuild.
Integrated acceptance remains pending.
A fifteenth trial completed all three talkback press/release modes and the source
handovers, then timed out because the driver waited for audible readiness while
leaving monitor mute enabled. The test correction explicitly unmutes through the
same reviewed action and retains the original deadline. Final reconnect/PA actions
also require current paired observations and the existing live authority. A later
underrun occurred on the unchanged armed-but-muted PFL stream; sparse evidence
does not identify its packet/scheduling cause. That trial remains failed. The
zero-underrun/overflow checker now examines every playback window and final bridge
status, so disarming after a terminal fault cannot hide its lifetime counters.
Thresholds and freshness
were not relaxed. Corrected harnesses use actual frontend actions, production
PA/FX/REC, immutable configurations and independent numerical checkers.

Trial17 at32 inputs failed: the FOH frontend lost its paired-observation deadline
and authority, and the final cross-controller barrier correctly timed out. All
three release modes completed and monitor counters remained zero. Exact response
starvation, churn or freshness causation is not established; bounded first-failure
diagnostics were added without changing admission, timing or recovery.

Trial18 verified stall closure, but the replacement duplex process was refused
while the previous authenticated owner still awaited its2s QUIC idle timeout.
The reviewed harness now waits2.2s after either exited fault mode, within the
original trial budget; the failed trial remains failed. Trial19 stopped before
fault injection: stale review presentation did not mark pages, and confirmation
was correctly refused. The driver now waits for fresh unchanged review identity
and content before each page and confirmation, using the original deadline.

Trial20 at32 inputs lost the first hold after about1.1s without a release
command, revision change, FOH authority loss or media fault. Both bridges retained
zero underruns/overflows. The last heartbeat and closed observation are consistent
with deadman expiry; the specific blocking call is unrecorded. Independent source
review found that normal250ms polling and up to1900ms renewal could run while
held, exceeding the150ms deadman. A dedicated bounded held-service correction is
implemented and independently reviewed in Desk `c9a13fd`. Focused36, default188
and native190 tests passed, alongside parser, format, both Clippy checks and release
builds. Eleven opt-in cases were skipped in each normal suite. The deadman,
freshness and sample thresholds remain unchanged.

Trial21 at16 inputs kept the first hold for2.1s but failed release readback.
The held paired observation exceeded its30ms budget while matched Brain and raw
revisions agreed and the Brain receipt had been invalidated. Provider evidence
shows an explicit release boundary. Independent source review found that key-up
inside this held observation sends priority close but does not stop the wait;
a later observation timeout can incorrectly become a terminal provenance fault.
The exact close reply was not logged, so this runtime attribution is conditional.
A scoped typed-cancellation correction was implemented, preserving malformed/partial
reply faults and every timing limit. Trial21 remains failed.

Trial22 at16 inputs failed before the first held state was reported applied: a
valid matched Brain/raw pair took31ms against a remaining26ms probe budget. Both
observations were fresh, so this is distinct from trial21 cancellation. Source
review confirmed that the held path requested raw readback only after its Brain
reply, adding a dependent round trip. A held-only eager paired-query correction
was implemented within the unchanged absolute deadline and exact revision checks.
Trial22 remains failed; no numerical acceptance is inferred from other processes
exiting normally.

Desk `c039da1` passed all ten validation steps: 40 focused checks, two parser
checks, 192 default and 194 native tests, format, both warnings-denied Clippy
configurations and both release builds. Each normal suite skipped 11 opt-ins.
Trials 21–23 remain failed: key-up/readback cancellation, serial paired-query
latency, then eager paired-query latency respectively. Trial 23 observed a 38 ms
post-heartbeat step against 29 ms remaining; unchanged deadlines correctly
refused it.

An optimized opt-in offline benchmark v2 passed its one test. Median measured
post-I/O CPU stages were 8.693/18.066/26.797 ms for 16/32/48 inputs with five
configured monitor buses. These are descriptive CPU measurements, not end-to-end
network, held-action or audio latency. At that checkpoint, the following measured production optimization had not yet
been applied.

Required completion rows remain **PENDING**: simultaneous16/32/48-input sample
acceptance; independent talkback/monitor return-path references; controller/duplex
stall and restart recovery; raw REC/analysis and dry protected audio continuation;
final CPU/RSS/queue measurements; exact artifact hashes; publication/CI and both-Pi
source/ledger receipts. Sparse evidence windows cannot establish a5ms waveform fade
or a155ms physical-key/network wall-clock deadline; source-frame/unit bounds and
measured network observation brackets must be reported separately.

## Physical follow-up

Physical activation is a separate authorized session. Start with read-only actual
card/endpoint inventory; record socket/channel mappings, duplex clock compatibility
and hardware direct-monitoring state. Then use conservative configured levels and
explicit arm to check capture/playback, selected destinations and PA protection.
Measure independent-device drift, sustained deadline behavior, ASRC occupancy,
release/fault closure and monitor/roundtrip latency with method and uncertainty.
No acoustic quality, feedback immunity, hardware clock lock or physical qualification
is inferred from fake PCM, numerical samples or CPU-headless presentation.

## Historical checkpoint — authority-validation optimization and trial 24

Desk `3eca755` replaces repeated authority JSON round trips with direct semantic
validation and a bounded counting serializer, preserving strict wire checks and
typed byte limits. All eleven validation steps passed: 79 library and 41 focused
contract tests, an optimized benchmark, 197 default/199 native tests, format,
both Clippy configurations and release builds. Each normal suite skipped 12
opt-ins. The unchanged benchmark measured median post-I/O CPU totals of
4.585/9.145/12.785 ms for 16/32/48 inputs, reductions of 47.3/49.4/52.3%.
These are CPU observations, not network or physical latency qualification.

Trial 24 remained failed: the first held action later exceeded its paired
observation budget, 36 ms against 29 ms remaining. Brain and raw revision 58
matched and remained fresh; all other roles exited normally. The CPU improvement
did not resolve this deadline failure. At that checkpoint, bounded opt-in client/provider stage
diagnostics were being implemented to distinguish waiting, scheduling and parsing;
QUIC flow control is a hypothesis, not an established cause. Timing, freshness,
quality and safety limits remain unchanged. All seven final scenarios are pending.

## Historical checkpoint — strict-document reuse under validation

Provider timing diagnostics passed 34 focused tests, default 378/hardware-host
432 normal tests (15/30 opt-ins ignored), both Clippy configurations and release
gates. The final style/test-only correction was independently checked. Desk
`78fc22f` passed all nine trace-validation gates, including default 200/native 202
normal tests with 12 opt-ins ignored in each suite.

Trial 25 remains **FAILED**: Desk probe 74 at revision 50 took 31 ms against a
29 ms budget. Explicitly correlated provider request ordinals 149/150 took about
1.27/1.41 ms across the observed provider stages, with no observed write Pending.
Desk's repeated strict parsing dominated its recorded elapsed stages. These are
same-process elapsed observations, not thread CPU, one-way latency or proof of a
network cause. The provider trace overflowed later; coverage of the identified
pair does not establish complete whole-run coverage.

A reviewed six-file Desk candidate reuses sealed, strictly validated documents.
Focused 88 and default 206 tests passed, with 12 opt-ins ignored in the default
suite; native, lint and release gates remain pending. Parser and canonical-size
caps, session checks, page handling and deadlines remain unchanged. The offline
baseline medians are 4.59/9.20/12.82 ms for the same 16/32/48-input fixture. The
optimized benchmark has a known cache bias awaiting correction; no improvement
number or deadline qualification is claimed for this candidate.

All seven final scenarios remain pending: simultaneous 16/32/48 inputs, duplex
stall, duplex restart, controller stall and Stagebox restart. Trial 16 is historical
success from an earlier candidate. Publication, CI, source synchronization and
physical qualification remain separate outstanding gates.

## Current checkpoint — compact held readback in progress

Desk `ee95873` passed focused 88, default 206 and native 208 tests, with 12
opt-ins ignored in each normal suite, plus formatting, both Clippy configurations
and release gates. Its corrected strict-document benchmark measured median
elapsed totals of 2.045/4.221/8.359 ms for derived 16/32/48-input fixtures,
against 4.595/9.201/12.817 ms before reuse. These are offline post-I/O elapsed
measurements, not thread CPU or network deadline qualification. The derived
32-input fixture is 51,096 bytes and omits a duplicate authority present in the
actual response, which exceeds 88,283 bytes and is paged.

Trial 26 completed with all roles successful and passed independent sample,
talkback and overlap checks. Its original checker failure remains preserved:
a reviewed correction restricts the exact-zero internal-PFL exception to an
acknowledged selection boundary; ordinary internal taps retain the unchanged
analytic tolerance. Trial 27 at 32 inputs failed a held paired-readback deadline,
30 ms against 29 ms remaining. Neither result qualifies the subsequent protocol.
Historical trial 16 belongs to an earlier candidate.

The compact held-proof design is frozen and independently reviewed;
implementation is in progress and has not been validated. It binds a fresh
scoped authority observation to previously reviewed talkback configuration while
keeping full UI readback separate. All seven scenarios must pass on its final
source: simultaneous 16/32/48 inputs, duplex stall, duplex restart, controller
stall and Stagebox restart. Publication and synchronization gates remain open.
No physical PCM was activated; mapping, clock lock and acoustic qualification
remain separate.
