# Brain audio acceptance

Status: **SOFTWARE VALIDATION PASSED — 2026-10-06**. The independently reviewed
frozen candidate passed complete production/consumer gates and all seven reserved
two-Pi scenarios, trials 37–43. Earlier passes and failed trials are retained
separately. These results do not authorize physical sound-card activation or
establish hardware/acoustic qualification. Publication, hosted CI and exact source/
ledger receiving receipts are tracked separately by the coordinator.

This record covers the Brain duplex audio integration requirements.
[Architecture and operation](../reference/BRAIN_AUDIO.md), [control](../reference/BRAIN_AUDIO_CONTROL.md),
[clock/device bounds](../reference/BRAIN_AUDIO_BRIDGE.md) and
[authenticated transport](../reference/REMOTE_TRANSPORT.md) own the implemented contracts.
Stagebox remains the processing/recording reference. Brain owns one local duplex
endpoint with a separate clock; both new audio crossings use Rubato 5.0.1 ASRC.
Source-clock FX and raw recording retain their original timeline.

## Requirement, implementation and evidence

| Requirement | Production owner/path | Evidence and remaining gate |
|---|---|---|
| One duplex endpoint, explicit mapping/capabilities, safe startup/reopen | `brain_audio/device.rs`, `host.rs`, `brain_runtime.rs`, `bin/brain.rs` | Fake adapter exercises the same host, mapping, format, partial-transfer, fault and readback boundaries; raw ALSA diagnostics implemented. Physical opening not exercised. |
| Physical Stagebox composition and source-clock FX | `host/duplex.rs`, `remote/runner.rs`, `device_epoch.rs` | Shared authority advances once per captured48 frames; authorization before I/O, durable fresh epoch, map/readback checks and late partial-playback refusal tested. Source-agnostic FX is explicit. Sustained physical deadlines remain unqualified. |
| Independent clocks and bounded conversion at both crossings | `brain_audio/bridge.rs`, remote host and Brain renderer | Independent numerical references, allocation guards, five optimized clock cases including combined actual FakeDuplex/BrainHost/both bridges passed. |
| Actual monitor destinations and protected FOH talkback | `local_audio.rs`, `mixer.rs`, `module_graph.rs` | Actual PA/FX/REC owner test at 16/32/48; every five configured test monitor destinations, unselected silence and independently assembled PA reference. Five is a test configuration, not a product limit. Final 16/32/48 network checks passed. |
| Main, performer mix, PFL/AFL; no audience mix modification or implicit TB loop | Mixer tap and pre-TB program/monitor capture | `brain_control`, `brain_owners` verify real taps, centered pre-mute/fader/pan PFL, stereo AFL, selection isolation and pre-TB operator feeds. |
| Held-action lifecycle and independent scopes | `brain_control.rs`, LocalAudio, remote policy/host | 50ms heartbeat, observation-bound 150ms deadman and 240-frame fade; tombstones, stale/replayed messages, pending mutation, lease/FOH permission and fault regressions. Packet admission rejects expiry without requiring another render. |
| Requested/applied/ready distinction; explicit rearm | GP15-brain/device, runtime ArmFence, host readiness | Actual device/bridge identities gate readiness; replacement requires acknowledgment, closed observation and later explicit arm. Source selection invalidates old queued playback. |
| Existing FX/analysis/REC preserved | Dedicated Brain media role/queues plus existing owner graph | Actual module samples and raw recording checked; remote coexistence regressions pass. All-channel two-Pi sample evidence passed. |
| Stable intent only; dynamic dimensions | Structural intent and GP15 contracts | Arm/holds/grants excluded from persistence; producer fixture corpus includes 16/17/32/33/48 inputs and independently varying monitor counts. |
| Actual Desk provider/client/frontend | SHR Desk Brain/device actions and observations | Producer fixtures, keyboard/controller/focus safety, device epoch/map draft/review/retry fences and CPU-headless presentation tested by Desk owner. All seven final actual-endpoint scenarios passed. |
| Finite authenticated capacity and isolation | Remote session/proxy/host | Sixteen-session deployment budget; actual TLS test admits16, refuses17 without eviction, and accepts fresh replacement. At least five simultaneous scoped controller/worker roles are required. Loaded timing is a separate gate. |
| Publication and synchronized recovery | Owning publication policies and private ledger | Exact-source reviews and seven scenarios passed; provider-before-consumer publication, CI and twelve-repository receipts are tracked by the private ledger. |

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

## Validation commands and classification

The current campaign results are recorded below; earlier candidate counts remain
in the archived acceptance draft. Failed trials never qualify a later candidate.

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

## Integrated candidate and retained failures

The atomic-control candidate adds authenticated maintenance of the same live scoped
lease and a full raw/Brain pair from one committed source boundary. Maintenance
has an independent bounded request namespace, exact replay outcomes and conservative
client expiry; it cannot grant authority, revive an expired lease or alter audio.
Staged authority copies cannot overwrite successful maintenance. Both client views
refresh only after the entire pair passes validation. Compact held readback retains
its original freshness and deadman bounds.

The final candidate passed independent combined source and artifact reviews. Producer
normal suites passed 390 default and 465 hardware-host tests, four actual-owner
tests, both warnings-denied Clippy configurations, release builds and 39 Python tests.
Desk passed 240 default and 242 native normal tests, one explicit CPU-headless
presentation check, both Clippy configurations, release driver/default/native builds
and nine Python tests. Producer suites intentionally skipped 15/30 opt-ins; Desk
suites skipped 12 each. Unchanged long clock evidence above remains applicable.
A parallel test-directory collision was reproduced and corrected in one hardware-only
test; it did not change production behavior.

Trials 31 and 32 passed simultaneous 16/32-input independent sample checks on an
earlier candidate. Trial 32's original checker failure remains retained: an independently
reviewed reference correction matches the producer's nearest-integer peak encoding
instead of truncating it. Exact equality and all audio thresholds remain unchanged;
the same captured evidence passed without another network launch.

Trial 33 at 48 inputs failed before talkback: the monitor-arm command was refused
because actual device readback was stale. Provider, duplex and FX processes exited
normally. Source inspection found that confirmation refreshes raw/Brain state but
can leave device readback older than the unchanged 250 ms limit. A deterministic regression reproduced the exact refusal. The reviewed correction
pins device epoch/map/full configuration before queueing, refreshes the same device
after topology reads, and fails closed on replacement, staleness or cancellation.
It passed focused and complete consumer validation without changing the limit.

Trial 34 passed that point and completed key-up, focus-loss and controller-removal
hold/release loops, then failed at the final PA authority barrier. The passive PA
client reported a stale-revision renewal refusal. Atomic maintenance still covered
only the three Brain scopes, leaving PA and ordinary mixer scopes on the legacy
read/revision renewal path. The independently reviewed scope-complete correction admits every canonical existing
control scope. Desk uses it for ordinary mixer/PA scopes only with explicit GP15
opt-in; legacy clients retain their existing behavior. Focused regressions cover
90 permission mismatches, 60 staged-authority combinations, heartbeat churn and the
67-fixture producer/consumer corpus. Full offline gates passed for this correction.
Neither trial qualifies the whole scenario or a later candidate.

Trial35 completed every Desk action at48 inputs, including the final PA barrier,
but the provider timed out during recording publication. All60,001 source blocks
were processed;47 of48 retained partial WAV headers had been updated, and the
report still showed recording finalization. File sizes/header updates do not prove
durable completion. Independent review found that the runner could save pending
recording evidence without a top-level fault. The focused correction observes an
actual terminal lifecycle state under a finite five-second budget and reports
pending timeout or terminal error as failure, preserving real counters and the
established incomplete-on-source-stop semantics. Recorder durability operations
and ownership are unchanged. Independent source review and the complete producer campaign passed: 390 default
and 465 hardware-host tests, four actual-owner tests, both Clippy configurations,
release builds and 39 Python tests. Trial35 remains failed.

The reviewed private scenario configuration uses55 seconds of source processing
and unchanged45-second clients within the existing60-second active/65-second total
ceilings. Every exact sample, contiguous recording, minimum overlap/continuation
frame count and safety assertion remains unchanged. The workload reserves shutdown
time; it does not guarantee filesystem latency or waive any required event window.

Trial36 used the reviewed 55-second provider workload. The provider exited normally,
processed 2,640,000 source frames and reported finalized recording with the expected
incomplete-on-source-stop outcome. Accepted and written recording counts both equal
2,632,464 frames, with no reported drops, gaps or overflows. Desk failed its first
held-loop status assertion at an observation age of 50.205767 ms. The status had
arrived about 18.493 ms after its original query send. These observations establish
a freshness failure, not its sole cause or the ordering of subsequent closure.
Source inspection found redundant compact queries, maintenance after the final
status query, and an idle wait before publication. A narrow consumer correction
passed independent contract/source/artifact reviews, focused regressions, complete
Desk gates and the final scenarios below. It reuses an unused eligible proof only
with original-send freshness and current guards, orders maintenance before the
final proof, and publishes before idle waiting. The passing candidate does not
retroactively establish the sole cause of trial36.
No freshness, heartbeat, deadman or numerical threshold is changed.

All failures and historical passes remain private evidence. The prior detailed
chronology is preserved in the [dated acceptance draft](../archive/brain-audio-before-final-2026-10-06/BRAIN_AUDIO_ACCEPTANCE.md).
Measured facts, source-established defects and unproven runtime explanations remain
distinct. No provider closure is attributed to a preceding held-status failure
without evidence that excludes panic/disconnect as its cause.

## Final two-Pi scenarios and measured limits

All rows use the same provider/Brain binaries and Desk runtime source, with actual
PA/FX/REC owner libraries. The frozen producer source digest is
`ab4ff2da106ee202cf3e31b6c44201e5d414d817d4c13a929b43ef5fd20d6a2d`;
Desk runtime source is `70ba6b99d10e315778f1b00edddd5e2cee417014`, digest
`f2cdac711d119e2b1e295e13aa8b19bae7d0c46e3ce0f76e3201fb405d052c48`.
Publication documentation commits do not change these runtime inputs. Exact
artifact/checker/configuration hashes and immutable per-run receipts remain in the
private task ledger. No private recordings or test configurations are published.

| Scenario | Trial | Independently checked result |
|---|---:|---|
| 16-input simultaneous talkback, monitoring, FX/analysis and REC | 38 | PASS; 2,637,168 exact raw frames/channel; PFL plateau error −0.04273 dB; 1,536 qualifying overlap playback frames |
| 32-input simultaneous paths | 39 | PASS; 2,634,480 exact raw frames/channel; PFL plateau error −0.02854 dB; 1,104 overlap frames |
| 48-input simultaneous paths | 37 | PASS; 2,632,128 exact raw frames/channel; PFL plateau error −0.01361 dB; 1,632 overlap frames |
| Duplex stall and fresh unarmed replacement | 40 | PASS; expected failed owner, no held replay, continuing dry/PA/FX/REC |
| Duplex process termination/restart | 41 | PASS; fresh epoch, unarmed recovery, no held replay, continuing dry/PA/FX/REC |
| Controller process stall/resume | 42 | PASS; held closure without replay, independent media and recording continue |
| Stagebox process restart | 43 | PASS; source/Brain epochs advance, read-only frontend has no writer grant, fresh playback is silent; new take has 572,976 exact raw frames/channel |

The unperturbed duplex/FX clients actually ran at least45 seconds in each profile.
Normal source processing was55 seconds, within the unchanged60-second active and
65-second total reservation ceilings. The driver deliberately confirms output mute
at the end: post-FX tails of223,776–293,904 source frames remain running and exactly
silent at the outputs while raw REC continues to the last processed source frame.
These muted tails are not claimed as nonzero PA continuation.

Fault runs instead leave band output running. Independent checks verify at least
two seconds of post-closure dry/PA/FX/REC continuation; the analytic dry reference's
largest error was2.17e−19 against the unchanged1e−12 tolerance. Additional post-FX
running/nonzero PA tails span295,728–297,984 source frames, with contiguous raw REC
to source end. FX ran its full45 seconds. Deliberately interrupted duplex owners
need not reach45 seconds; replacements ran their configured15 seconds. Controller
stall retained the full45-second duplex client.

Stagebox restart intentionally interrupts the first processes. Recording cannot
continue while its owner is dead: the first take is preserved without a finalization
or crash-recovery claim. The new provider/duplex ran their full12/10 seconds, and
fresh raw recording reached the new source end. Finalized scenario takes retain
the established incomplete-on-source-stop outcome (host fault6), with accepted and
written counts equal and no reported drops, gaps, overflows, invalid blocks,
clipped samples or writer fault. Finalization is not relabeled as a complete take.

| Simultaneous profile | Mean provider CPU, % of one core | Mean Desk CPU, % of one core |
|---|---:|---:|
| 16 | 31.7 | 91.4 |
| 32 | 44.0 | 126.1 |
| 48 | 56.9 | 146.7 |

Across these profiles, observed maximum process RSS was108,736 KiB for the provider
and25,156 KiB for Desk. Mean process CPU includes wrapper accounting and can exceed
one core; neither means nor RSS establish a hardware deadline guarantee. Armed/ready
sampled bridge occupancy ranged607–1,286 frames across both crossings, nominal queue
components12.65–26.80 ms and filter components2.65–2.67 ms. Sampled underrun/overflow/
rejection counters were zero. These are sparse observations, not continuous maxima,
physical oscillator estimates, one-way latency or acoustic delay.

Source and artifact reviews, normal suites, numerical/clock/owner evidence and all
seven independent scenario receipts jointly establish the declared software scope.
Sparse network evidence cannot establish a5 ms waveform fade or155 ms physical-key/
network wall-clock deadline; source-frame bounds and observed brackets are separate.

The development-PC Desk12/Lightdesk16 screen-reference collections were merged,
checked and retained as design inputs. Eight missing CI workflows were added to
REC, FX, Lightdesk, DAW, Drums, Synth, Sampler and Tone Over 9000; all eight completed
successfully. Their changes are workflow/documentation only. A fresh-checkout Synth
workspace-directory failure was preserved and corrected without changing tests or
runtime code. DAW retains its repository-required focused CI scope.

## Physical follow-up

Physical activation is a separate authorized session. Start with read-only actual
card/endpoint inventory; record socket/channel mappings, duplex clock compatibility
and hardware direct-monitoring state. Then use conservative configured levels and
explicit arm to check capture/playback, selected destinations and PA protection.
Measure independent-device drift, sustained deadline behavior, ASRC occupancy,
release/fault closure and monitor/roundtrip latency with method and uncertainty.
No acoustic quality, feedback immunity, hardware clock lock or physical qualification
is inferred from fake PCM, numerical samples or CPU-headless presentation.
