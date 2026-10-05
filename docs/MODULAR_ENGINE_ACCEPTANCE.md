# Modular engine acceptance

Task0014 implementation is in progress. This document records acceptance criteria
before implementation results; no row is accepted merely by being listed. The
original eight-input milestone, PA v1 ABI and historical evidence remain intact.

## Reviewed architecture

An independent source review selected evolution of the existing Mixer/Authority,
an owner-native configurable SHR PA successor, and mutually authenticated QUIC
for both control streams and GPA1 datagrams. Shared prepared topology translates
stable physical/source/strip/bus/module/output IDs into bounded render indices.
Preparation, JSON, network, credential work and state retirement stay off render.
Structural changes quiesce and retain old state on refusal; successful replacement
and recovery remain muted until explicit rearm. Brain follows device source frames.

## Requirement and evidence matrix

| Requirement | Owner/code boundary | Required acceptance | State |
|---|---|---|---|
| Configurable strips and monitor buses | GigPies mixer, authority, processing schemas | Every input at16/32/48 and a nonfixture count; independent EQ/dynamics, isolation, neutral exactness, ramps and partition equivalence | Focused pass: `modular_engine` covers16/17/32/48; complete default suite passes |
| Resource admission and prepared ownership | GigPies topology/render, SHR PA prepared ABI | Checked dimensions/budgets; precise refusal; no render alloc/free; saturated pending/retirement slots | Focused pass: `gp07_alloc`, `modular_engine`, actual PA/module graph; hardware aggregate pending |
|16/18 analog patch | GigPies topology/device adapters | Unique channel patterns through explicit configurable USB/socket permutations; manufacturer18/20 reference separately labelled unverified; duplicate writers refused | Software permutation/reference tests pass; attached UMC/ADA mapping unverified |
| Single synthetic/device production graph | GigPies module graph/host | Same code path, raw REC/analysis, dry+wet before PA, silent unmapped outputs | Actual PA/REC/FX host and composed producer tests pass; physical adapter unopened |
| Six/eight PA outputs and4x8 matrix | SHR PA config/graph/DSP | Independent complex crossover branch/sum references; weights/headroom, gain/EQ/delay/polarity and final protection | Owner normal79 tests and actual loaded graph pass; PA source published with green CI |
| PA successor ABI | SHR PA ffi/header; GigPies adapter | Real release-linked C caller and actual loaded library; exact sizes/version/lifetimes; old v1 bytes/behavior | Actual C v1/v2 callers and loaded host pass; exact owner hashes retained privately |
| Scoped PA and output routing control | GigPies authority/Desk | Dedicated grants; atomic prepared commit/readback and actual boundary samples; retained muted rollback | Actual local producer16/32/48 passes; Desk cross-node acceptance pending |
| Authenticated commands/media | GigPies remote/Desk transport | Actual paired peers, wrong/revoked peer refusal, bounded framing, capability/descriptor exchange, no reconnect replay | Remote focused security/authority tests pass; Desk cross-node integration under correction |
| Real Brain FX and analysis | GigPies remote/host, unchanged SHR FX | Grouped source-indexed media on both Pis, exact identity, actual wet samples; loss/reorder/stalls/restart preserve dry/REC | Actual loaded FX/REC and loss/recovery regressions pass; cross-node media acceptance pending |
| One clock domain | GigPies clock/provider/host, Desk health | Fake-device faults exercise production mute, incomplete recording, new epoch/map, rejection of old intents/audio and explicit rearm | Synthetic production fault/recovery tests pass; physical clock/ADAT lock remains unknown |
| Dynamic Desk and protected review | SHR Desk client/frontend | Inputs16/17/32/33/48, bank navigation, PA/patch/clock controls, keyboard/controller parity and CPU-headless resize | Consumer implementation and focused tests pass; independent review fixes and actual cross-node acceptance pending |
| Persistence and compatibility | GigPies schemas/restore, Desk | Safe restore of intended mappings/config; mismatch refusal; no unmute/grant replay; old fixture preservation | Focused safe restore, mismatch, legacy corpus and unsupported-version refusal tests pass |
| Publication and synchronization | Each owner/coordinator | Normal production suites, Clippy/fmt/release/C/docs/guards, independent final review, exact upstream/CI and twelve repos on both Pis | PA provider published/CI green; GigPies/Desk final review, publication and both-node sync pending |

## Capacity reporting

Dimensions represent separate resources. Mixer strips do not define USB ports,
monitor count, PA outputs, recorder tracks, network group size or UI bank size.
Each implementation bound must report its origin, units, admission reason and
expansion path. SHR REC v1 currently accepts1–64 tracks,8192 frames per push and
64MiB queued samples; an optional recorder refusal must never silently trim a
show. GPA1 protocol channel indices and datagram MTU are transport bounds.
Software correctness at48 inputs is required; hardware deadline capacity remains
unqualified until measured on the actual configured graph.

## Validation classes

Focused production regressions run during implementation. Complete normal suites
run for every changed engine/schema/authority/concurrency owner, including affected
consumers. Actual trusted-library, producer-fixture, CPU-headless and reserved
bounded two-Pi functional checks run explicitly. Historical private music, auditions,
exhaustive research and long benchmarks remain opt-in unless directly affected.
Physical device/output, speakers, MIDI/DMX, clock switches, operator windows, host
tuning, deployment and full-show combined-load qualification are not authorized
by this software increment.

## Physical mapping evidence

Physical sockets, USB transport slots, contiguous logical analog channels and
processing destinations have independent identities. Neither a manufacturer's
channel table nor a historical stereo fixture supplies an observed Linux patch.
USB indices and signal/socket assignments require independent validation.
The implementation supports reviewed explicit patches and arbitrary permutation
tests without constraining DSP or Desk to the expected device ordering.
No PA/monitor output allocation is mandatory.

Read-only Pi5 inventory on2026-10-05 found a PreSonus AudioBox USB96 with two
capture/playback channels; no UMC1820/ADA8200 was attached. No PCM was opened.
The UMC reference therefore remains unverified. The manufacturer's
[UMC1820 guide](https://www.bhphotovideo.com/lit_files/155647.pdf), controls13/23
and Digital I/O Routing, documents ADAT capture11–18 and playback13–20 at48kHz,
with S/PDIF occupying separate slots. This does not establish the actual attached
Linux device order. A reference mapping must never silently become an accepted
physical map. Logical input09 means the ninth admitted source, not USB slot9.

## Current integrated validation

The first three reserved synthetic two-Pi operator runs are retained failures,
not acceptance. The first run exposed large-response reassembly; the second
exposed structural snapshot/final envelope mismatches; the third reached attach
and grant before exposing an unsatisfied paired-freshness request.
For the first run:
Desk timed out before completing a structural transaction. The provider completed
its bounded source run without a control fault. Independent review identified
missing reassembly of large authenticated response envelopes in Desk; correction
and renewed operator acceptance remain required. All48 raw recorder tracks in
that run were checked byte for byte against the independent PCM24 source formula
and source-frame timeline (2,861,088 frames per track, no gaps/drops). Intentional
source shutdown finalized the take as incomplete. Disposable WAVs were removed
after retaining hashes, timeline, results and failure logs privately.

The complete default production suite passed334 tests with9 opt-in tests ignored.
The hardware-feature aggregate passed361 tests with23 opt-in tests ignored;
actual-owner/core checks passed25, and focused final physical-admission/envelope
checks passed8. Both warnings-denied Clippy configurations and formatting pass.
Final release, Desk and cross-node validation remains in progress. These observations do not imply physical synchronization or
realtime deadline qualification.
