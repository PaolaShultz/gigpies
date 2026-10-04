# Visual map and historical concept review

The current README [hero](assets/gigpies-banner.svg) and
[system diagram](assets/architecture.svg) are original, code-authored SVGs updated
for the dual-console direction in 0.2.3. They show the **intended complete system**,
not a photograph of working hardware or proof of live integration.

- One Brain hosts the audio surface **SHR Desk** and lighting surface **SHR Lightdesk**.
  Each gets its own 1920×1080 screen and independently assigned MIDI controller.
  The stylized controllers describe the intended vocabulary, not verified device models.
- Human operation comes first. MANUAL, ASSIST and bounded AUTO are console modes.
- GigPies owns integration and the band mixer. Stagebox owns local audio I/O,
  monitor paths, PA protection and recording to NVMe; its small display is local PA UI.
- **SHR Lux** owns fixture evaluation, cue/effect execution, lighting arbitration
  and physical output. Lightdesk owns the operator surface, not another engine.
- Audio FX/source transport and lighting control have distinct contracts. Lighting
  redraw/failure must not block audio control, protection or recording.

Both surfaces currently have offline foundations. Native desks, full Lux/manual
fixture control and combined display/controller/load acceptance remain planned.
The stereo audio bench has its own qualified evidence in [AUDIO_HARDWARE.md](AUDIO_HARDWARE.md).

## Preserved original artwork

The supplied AI-generated [original concept image](assets/gigpies-concept.png)
remains unchanged as historical artwork, linked here instead of promoted in the
README. The [earlier review](archive/concept_review-before-0.2.3-2026-10-04.md)
and [older SVGs](archive/README.md) preserve the previous design context.

Its generated device panels, labels and some mappings are inaccurate. In particular:

- The Brain calculates prepared mix state; Stagebox validates/applies it. The new
  design has two human-operated consoles rather than an automation-only Brain.
- Live behavior uses deterministic rules. External AI is optional outside the
  essential live path; a generic “AI-assisted” label does not describe the authority model.
- The old RTP/PTP labels are not the selected implementation. The prototype uses
  GPA1 UDP and the PA source-frame clock; no PTP or audio ASRC was installed.
- ADAT direction, channel availability and clock master require actual device
  documentation and acceptance; the drawing is not a wiring guide.
- “4×8” PA routing was an ambition. Existing SHR PA's 2-input/6-output speaker
  processor does not establish a complete GigPies band mixer or that routing.
- Primary recording is now Stagebox-local NVMe. Recording taps, physical outputs
  and fault policies come from explicit contracts, not decorative arrows.
- Muted source analysis does not establish acoustic response, speaker protection,
  feedback behavior, lighting coverage or dependable unattended operation.

Third-party names in the original artwork imply no endorsement. Read current
[architecture](ARCHITECTURE.md), [owners](COMPONENTS.md) and
[status](STATUS.md) for implementation and acceptance boundaries.
