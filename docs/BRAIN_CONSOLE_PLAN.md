# Brain consoles: SHR Desk and SHR Lightdesk

Updated 2026-10-06. **GigPies is the complete system with both an audio console
and a lighting console for human operation.** Automation is one operating mode
inside each console. The intended Brain has **two 1920×1080 monitors and two
independently assigned MIDI keyboard controllers**, normally one pairing for
audio and one for lighting. Both consoles run on the same Brain. The small
local display continues to serve the PA unit.

The preceding single-surface integration draft is preserved
[unchanged](archive/brain-console-before-lightdesk-2026-10-04.md); the earlier
[pre-Desk plan](archive/brain-console-before-shr-desk-2026-10-04.md) also remains.

## Brain sound-card ownership

Alongside the two consoles, Brain has one local duplex sound card: microphone
capture for talkback and playback for operator monitoring. GigPies owns both
directions in one device process; SHR Desk exposes setup, selection and safety
state through the existing authority/client layers. The card model, physical
channel map and hardware monitoring settings require actual observation.
Brain local I/O has a distinct device clock, bridged in both directions to the
Stagebox reference. Brain FX continues to follow Stagebox source frames.
See [Brain audio integration](BRAIN_AUDIO.md); the earlier console plan is
[preserved](archive/brain-audio-before-2026-10-05/BRAIN_CONSOLE_PLAN.md).

## Work ownership

Cross-module tasks and their progress live only in
[the integration plan](MODULE_IMPLEMENTATION_PLAN.md). Console-only tasks live in
[the owning plans](MODULE_IMPLEMENTATION_MAP.md). Native rendering and injected
roles have software evidence; actual display/controller/LED and fixture operation
need their named physical gates. This document describes responsibilities and
resource targets, not another task backlog.

## Retained screen references — 2026-10-06

The development-PC reference collections are integrated on both Pis. They add
manufacturer screen images, per-screen observations, source/version notes, local
HTML galleries and hash-pinned manifests. They are design evidence; they do not
change either console's implemented UI or establish physical operation.

| Owner | Reference collection | Questions it helps answer |
|---|---|---|
| SHR Desk | [12 audio-console screens](https://github.com/PaolaShultz/shr-desk/tree/main/docs/console-screen-reference) from Yamaha, Allen & Heath, Soundcraft and DiGiCo | Bank versus selected-channel detail, exact EQ/dynamics units, physical/logical patching and clear PFL/AFL destination context. |
| SHR Lightdesk | [16 lighting-console screens](https://github.com/PaolaShultz/shr-lightdesk/tree/main/docs/console-screen-reference) from MA, ChamSys, Avolites, Onyx and ETC | Stable fixture identity, capability-aware columns, programmer versus output state, colour/phaser controls and cue/playback context. |

Future layouts should preserve selection and focus as separate states, identify the
current destination/value layer, and distinguish a requested edit from applied,
held or measured state. Exact units belong beside precise controls; dense overview
screens need a readable selected-detail view. These are design hypotheses to test
against the owning screen plans and actual capabilities, not new implementation
requirements for task0015. Reference bank sizes never define product capacity.
Each library records image provenance and limits; old manual screenshots do not
establish a current console version or physical output behavior.

## Coexistence contract

Use separate surface processes, input/LED workers, focus and selection. Persistent
roles bind verified display/controller identities, not monitor order or ALSA port
numbers. Duplicate monitor identities or identical controllers without reliable
serials need explicit labelled assignment. A lighting reconnect/restart must not
steal the audio display, replay old commands or forward notes/LED traffic to the
audio desk/instruments. Keyboard operation remains complete without MIDI.

Lux must not compete for Lightdesk's assigned pad LEDs. Preserve Lux standalone
operation, but its integrated engine adapter must have no claim on those outputs.
No physical device identity, active preset or feedback protocol for the **second**
controller has been verified. Existing MiniLab mkII and MiniLab 3 evidence is scoped
to its recorded device/session, not automatically reusable as a fresh mapping.

Both desks share show UUID/module compatibility, theme/font/action vocabulary and
useful health summaries. Keep audio and lighting control domains distinct:
lighting programmer intensity can override a brighter cue even at zero, while
ordinary playback intensities can merge HTP; color/position require explicit
activation/priority semantics. Mode changes preserve the look; release is scoped
and deliberate. Lux owns timing/arbitration and fixture-safe output behavior.
A drawing, ACK or packet submission does not verify physical DMX/light.

Cross-system cues are opt-in named events with target scopes, epoch/expiry and
individual results. No light GO changes audio by default. Lux consumes named,
fresh source-analysis subscriptions from GigPies rather than Lightdesk opening
audio devices. Lighting redraw, failure and telemetry overload cannot block
Stagebox real-time audio, protection, recording or audio control input.

The [Lightdesk blueprint](../../shr-lightdesk/docs/BLUEPRINT.md#resource-and-responsiveness-targets--unverified)
sets the proposed combined acceptance profile and measurable budgets: two active
1080p/60 Hz surfaces, aggregate render CPU/GPU p99 ≤8 ms each per refresh,
combined surface PSS ≤384 MiB, local MIDI dispatch p99 ≤5 ms and visible feedback
p99 ≤33 ms. Whole-Brain FX/Lux/analysis/transport headroom must also pass. These
are **unmeasured targets**, not evidence that a particular Pi meets them.


[Historical console assessment and backlog](archive/tracking-before-2026-10-09/BRAIN_CONSOLE_PLAN.md); retired as a tracker.
