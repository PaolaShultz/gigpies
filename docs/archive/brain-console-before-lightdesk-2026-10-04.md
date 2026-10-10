# Brain console integration: SHR Desk

Updated 2026-10-04. **Surface ownership moved to `../shr-desk`.** GigPies remains
the complete product and owns module integration, runtime contracts and acceptance.
The original full-HD plan is preserved [unchanged in the archive](brain-console-before-shr-desk-2026-10-04.md).

## Owning project and plans

From the GigPies root, the project is `../shr-desk`. These local links require the
sibling checkout and are not published repository URLs or build dependencies:

- [SHR Desk README](../../../shr-desk/README.md)
- [Blueprint and module decomposition](../../../shr-desk/docs/BLUEPRINT.md)
- [Console screen study](../../../shr-desk/docs/CONSOLE_STUDY.md)
- [Screens and scope priorities](../../../shr-desk/docs/SCREENS.md)
- [MiniLab, octave selection and control map](../../../shr-desk/docs/CONTROLLER.md)
- [Proposed live control boundary](../../../shr-desk/docs/CONTROL_CONTRACT.md)
- [Implementation status and next steps](../../../shr-desk/docs/STATUS.md)

Desk owns full-HD TUI-style presentation, fonts, graphs, navigation, MIDI mapping,
keyboard input and the operator's view of automation/manual authority. Core mixing,
PA processing, effects, recording and lighting remain outside the surface.
The Stagebox/PA node's live graph is not the same module as the SHR PA speaker
processor. The [component map](../architecture/COMPONENTS.md) records those boundaries.

## Current checkpoint

Implemented in Desk: an independent Rust state/command simulator, explicit human
holds and mode transitions, pure MIDI translation/pickup/LED encoding, and three
1920×1080 SVG screen drafts with the existing Terminus font. Tests and a small
local gallery provide offline evidence. This does not implement a GPU window,
complete keyboard/controller workflow, a real mixer adapter or device output.

The intended controller is the previously documented Arturia MiniLab mkII:
sixteen rotations, eight physical pads with two banks, and octave-transposed
piano keys selecting successive groups of twelve channels. Actual learned
messages, both pad banks, LED behavior and HDMI readability remain acceptance work.

## GigPies integration work

1. Define the real multichannel control/capability schema and writer scopes.
   Existing [GPA1 and bounded control](../reference/AUDIO_TRANSPORT.md) are useful foundation;
   GPC1's test scalar is not a mixer API.
2. Expose authoritative channel/bus state and per-parameter automation grants,
   holds, pending/applied/rejected results and fresh-state reconnect.
3. Implement engine-owned smoothing and explicit scoped return to automation.
   Full manual operation preserves PA protection and does not require automixing.
4. Integrate PA/FX/REC/LUX through their owning adapters. UI loss must not stop
   Stagebox audio, local monitors or recording. Never auto-recall on UI restart.
5. Verify native display/controller behavior and combined workload separately
   from yesterday's [stereo hardware bench](../acceptance/AUDIO_HARDWARE.md). Use the private
   ledger for any shared two-node resource reservation.

No new audio/network/MIDI/service operation was performed for this checkpoint.
Local implementation ran on the shell identified as `rpi5`; pinned SSH confirmed
`rpi4` was reachable. This does not select final runtime Brain/Stagebox hardware.
