# Next session: physical gates and subsequent software increments

Current handoff after the integrated software milestone, 2026-10-04. The
[previous local-client handoff](archive/next-session-before-modules-2026-10-04.md)
is preserved. The [continuation prompt](MODULE_CONTINUATION_EXECUTION_PROMPT.md)
records this completed software/publication scope; recover the current source
and acceptance before using any historical launch card.

The requested next software action is the [October 4 review and repair pass](DAILY_REVIEW_EXECUTION_PROMPT.md)
using the stronger model selected for a new session on Pi5. Its exact daily commit
inventory includes all twelve owners; it authorizes in-place fixes, validation,
publication and final source synchronization. This prompt has been prepared;
the stronger-model review itself has not yet run.

Read the [local integration instructions](HEADLESS_INTEGRATION.md),
[implementation map](MODULE_IMPLEMENTATION_MAP.md), [Brain plan](BRAIN_CONSOLE_PLAN.md),
[architecture](ARCHITECTURE.md) and [ownership map](COMPONENTS.md). The intended
Brain hosts two 1920×1080 monitors and two independently assigned MIDI controllers:
Desk for audio and Lightdesk for lighting. Manual operation remains independent;
ASSIST proposes, and AUTO requires explicit bounded authority.

| Owner | Accepted software foundation | Next separate increment |
|---|---|---|
| GigPies | GP-01..05, GP-09 and private local GP-06 subset; real owner graph, named analysis and role broker | Reviewed production remote authentication (B-NET), wider GP-07 scenes/channels; separately authorized physical/combined-load gates |
| SHR Desk | DS-01..04 including native software frontend; read-only DS-05 actual module health | Physical dual-display/controller/LED acceptance (GP-H2); later writable controls only after owner contracts |
| SHR Lightdesk | LD-01..04 including native software frontend and read-only LX05 compatibility | Physical role/display/controller acceptance; later analysis controls only as an explicit increment |
| SHR Lux | LX-01..05 null-output authority, release/recovery and named source automation | LX-06 fixture output only with known patch, explicit arming and hardware reservation |
| SHR REC / FX / PA | REC-01/02 and fixed-v1 FX-01/PA-01 implemented in their owners and consumed by GP-05 | Writable extensions/acoustic acceptance remain owner work; no duplicate algorithms in GigPies |

Read each owner's instructions and current acceptance before changes. Console
software rendering and injected descriptors do not verify actual display/controller
identity, MIDI or LED ownership. Real enumeration and dual-device acceptance remain
GP-H2. Missing or ambiguous identities must remain unbound.

Physical fixture output, acoustic protection and combined CPU/GPU/memory/scheduler
acceptance require their own authorized sessions and reservations. Preserve human
holds and current looks through loss/reconnect until explicit release. Lighting
failure must not block audio control or recording. No physical operation follows
from the completed software publication.

## Qualified stereo hardware handoff

Read [AUDIO_HARDWARE.md](AUDIO_HARDWARE.md) and [its owning plan](AUDIO_HARDWARE_PLAN.md).
The selected USB host uses 48-frame / 1 ms processing, independent device capacity
and zero silent prefill, with real PA/FX/REC modules and sample-verified recovery.
The final ten-minute working-channel H8 run measured **5.19–5.23 ms**, with exact
software output and zero xruns/late wet returns. Two one-frame physical offset
changes remain unresolved. The historical 56 ms prefill/57 ms loopback is not the
current live-latency result.

The next useful physical check is a narrowly scoped USB transfer/feedback trace
around those changes under a fresh reservation. Existing one-second driver
snapshots are too coarse to identify the cause. Keep direct audio buffers fixed.
Read final H7/H8 conditions, retained xruns and the locked-page migration diagnosis
before extending the tested scope. Temporary settings were restored; no persistent
low-latency host profile was installed.

The prior session authorized the working channel. That does not grant a new
hardware session. The right return was about 69 dB weaker; verify a working route
before stereo physical acceptance. The USB microphone was absent. Acoustic PA,
clock, multichannel mixer, UI and complete-show acceptance remain separate.

## Offline listening handoff

The [summing plan](SUMMING_PLAN.md) and [delivery contract](SUMMING_DELIVERY.md)
are implemented and offline-validated, now included in 0.2.3. The f64 summer,
channel choices and selected GigPies effects remain unchanged; no supported new
musical candidate was selected.

Use the retained private listening index
`artifacts/automix/summing-study/2026-10-03-engine/LISTEN.md`. It identifies six
complete mixes, seven established excerpts and exact historical comparisons.
Independent measurements range from −1.400441 to −1.395178 dBTP against the −1 dBTP
ceiling; the largest production/independent difference is 0.004922 dB. Deliveries
use static gain on the selected buses, with 0.4001 dB margin and no loudness target.

Obtain a fresh playback go, then record preference by passage and concern. Do not
rerender just to reopen this queue, or repeat historical planners without a new
hypothesis and stopping rules. Preserve source recordings, frozen settings, hashes
and useful evidence. Latest publication checks live in [VALIDATION.md](VALIDATION.md).
The unresolved earlier Pi crash still needs separate diagnosis; boot recovery and
zero current filesystem counters do not establish a complete offline scan.

## Peer work and resource ownership

Read `/home/shome/p/AGENTS.md` and [NODE_LAB.md](NODE_LAB.md). Check the actual
hostname and live working trees; development coordinator/worker roles do not fix
the runtime Stagebox/Brain Pi assignment. The private ledger owns reservations,
exact source revisions and immutable review/acceptance records.

Use bounded `gigpies-peer` workers only within the current authorized scope; never
resume or take over an existing interactive session. Keep one owner per file and
resource. Shared load, hardware and restart experiments require a new explicit
reservation. Completed synthetic transport and stereo integration are recorded in
[transport](AUDIO_TRANSPORT_PLAN.md) and [hardware](AUDIO_HARDWARE_PLAN.md); do not
repeat them merely because an old execution prompt says to start them.
