# Next session: compact console and two Raspberry Pis

Recorded from the user's ideas on 2026-10-03, for tomorrow's hardware session.
**Planned only.** No MIDI mapping, network service, host-audio change or hardware
measurement has been performed or authorized by this note.

## A small musician-facing console

Combine a 14-inch screen with the user's small Arturia MiniLab controller.
The proposed control vocabulary is:

- Each keyboard key selects a channel; 12 keys make one octave of channel choices.
- Octave switches select additional channel banks so a small keyboard can address
  many inputs. Confirm how the exact controller model reports those switches.
- Eight coloured, pressable pads show states and trigger named actions.
- Sixteen rotary controls operate the selected channel's parameters.
- The screen shows the selected channel, bank, parameter names/values and pad states.

This could serve as the physical control surface for the larger GigPies system.
The controller's exact model, available controls, feedback messages and encoder/pot
behaviour still need inspection. The counts above describe the user's proposal.
Do not assume MIDI keys or octave buttons expose particular messages until tested.

Design questions for that session: visible bank/channel feedback, protection from
changing the wrong channel, knob pickup or relative encoder behaviour after a bank
change, an obvious reset, and a usable screen/keyboard alternative. Colour must not
be the only state indication. Channel selection must retain the instrument's verified
input-group assignment. A controller action's scope (shared mix, monitor or offline
preview) needs to be visible. The offline EQ amount/reset contract is a useful first
control to exercise without controlling live audio.

## Second Pi and the cable connection

The user will wire the second Raspberry Pi and connect the two with a cable.
After the physical setup is ready, inventory both devices and their interfaces,
record clock/software versions, and agree node ownership before implementing a
minimal protocol. Follow the Stagebox/Brain boundaries in [architecture](ARCHITECTURE.md)
and the [component ownership map](COMPONENTS.md); sibling module code stays with its
owner. Each Pi should run its own assigned component independently.

First measurements should record the physical/link setup, address configuration,
round-trip time distribution, jitter, loss, throughput, reconnect behaviour and CPU
load under declared test conditions. Distinguish round-trip measurements from one-way
latency, which needs a validated clock relationship. Then test versioned messages,
sequence/revision identity, stale-message refusal, disconnect/reconnect, idempotent
retry and recovery from one node restarting. Define control traffic and any future
audio transport separately; no transport or clock scheme is selected by this note.

## Coordinating the two Codex sessions

Arrange a deliberate handoff workflow on the two Pis tomorrow. Start with a shared,
versioned task/interface document: one owner per file/module, exact repository and
revision, requested change, expected message/schema contract, reproducible commands,
results and unresolved questions. Exchange small patches or commits after reviewing
local work; do not let both sessions independently rewrite the same files or run
hardware actions concurrently. A task ledger can distinguish queued, running,
ready-for-review and accepted work, with explicit evidence attached to each handoff.

The connection mechanism between the Codex sessions is still to be chosen and tested.
This note does not assume that separate sessions automatically share context, tools,
credentials or approval. Establish the operator's command channel and hardware owner
before automating session-to-session communication. Network access, deployment and
hardware control belong to that future session's concrete scope.
