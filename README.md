<div align="center">

![GigPies — two audio and lighting console screens with MIDI keyboards and one Brain Pi, facing a lit band stage with PA, monitors, a separate processing stagebox and phone control](docs/assets/gigpies-hero.png)

**Audio console · Lighting console · Performer monitors · Multitrack recording**

[![CI](https://github.com/PaolaShultz/gigpies/actions/workflows/ci.yml/badge.svg)](https://github.com/PaolaShultz/gigpies/actions/workflows/ci.yml)
[![Version](https://img.shields.io/badge/version-0.2.3-44d8e5)](CHANGELOG.md)
[![Rust](https://img.shields.io/badge/Rust-1.97.1-f0b77b?logo=rust&logoColor=white)](rust-toolchain.toml)
[![License](https://img.shields.io/badge/license-MIT-b6a0ec)](LICENSE)
[![Status](https://img.shields.io/badge/status-experimental-e7a85d)](docs/STATUS.md)

[**Start here**](#run-the-cli) · [**Architecture**](docs/ARCHITECTURE.md) · [**Roadmap**](docs/STATUS.md) · [**Documentation**](docs/README.md)

</div>

## Audio and light. Two consoles. One system.

GigPies is an experimental live-band sound and lighting system built in **Rust for Raspberry Pi**.
Version **0.2.3** brings together offline processing, transport and qualified stereo
USB bench work, with the dual-console integration plan. The complete live system
is still in development. [The integration plan](docs/MODULE_IMPLEMENTATION_PLAN.md)
owns active tasks and their implementation progress. Its intended live core needs no Internet or external AI service.

GigPies is the complete system, with human-operated audio and lighting consoles.
Automation is one operating mode within those consoles. The vision combines
mixing, PA management, independent performer monitors, lighting and recording.
In MANUAL, the operator runs the show; ASSIST proposes changes; AUTO acts only
within explicitly granted scopes. Soundcheck can prepare a baseline, while
human control remains available independently of automation.

The hero is an AI-generated, photorealistic visualization of the intended operator
setup and stage, with both digital desks, the separate processing node and phone control.
The [original concept artwork](docs/CONCEPT_REVIEW.md) is retained as a historical reference.

## Two nodes. Clear responsibilities.

![Intended Stagebox and Brain architecture](docs/assets/architecture.svg)

| Stagebox / Mixer | Brain / Show |
|---|---|
| Owns Stagebox audio I/O and the processing reference clock | Owns one local duplex card for talkback and operator listening |
| Runs channel DSP, monitors, PA protection and local NVMe recording | Hosts audio/lighting consoles, richer FX and analysis |
| Applies validated settings and local protection | Sends bounded parameter updates |

Brain local capture/playback has its own device clock. Talkback and operator
monitoring cross through independent asynchronous sample-rate bridges; FX and
raw analysis retain Stagebox source-frame identities. See the
[Brain audio integration](docs/BRAIN_AUDIO.md) for implementation and acceptance status.

These boundaries passed integrated software acceptance at 16/32/48 inputs and
under the declared restart/stall scenarios. Physical qualification is separate. The qualified stereo
bench exercises a narrower audio path. See [architecture](docs/ARCHITECTURE.md).
The [dual-console Brain](docs/BRAIN_CONSOLE_PLAN.md) pairs **two 1920×1080 monitors
and two MIDI keyboard controllers**: SHR Desk for audio and SHR Lightdesk for
lighting, both on one Brain. Both surfaces now have optional native frontends over
real local provider clients, checked with offscreen CPU rendering and injected
role descriptors. Actual dual displays, controllers and physical outputs remain
unverified. SHR Lux owns lighting authority and output; the small display serves
the PA unit.

## Working today

The [configurable modular engine](docs/MODULAR_PROCESSING.md) combines validated
input and monitor dimensions, four-band channel EQ and compression, actual SHR PA
v2, source-clock SHR FX and raw SHR REC. Task0014 software acceptance covers
16/32/48 inputs and a 17-input regression; these profiles are not product caps.
Desk supports authenticated control, reviewed PA configuration and output mapping.
The [acceptance matrix](docs/MODULAR_ENGINE_ACCEPTANCE.md) records the scope.

Brain duplex talkback and operator monitoring, independent ASRC crossings and
Desk controls passed independent review, complete offline gates and all seven
reserved two-Pi scenarios on one frozen candidate. Atomic scoped lease maintenance,
paired raw/Brain readback and held-action safety retain their original limits.
See [Brain audio acceptance](docs/BRAIN_AUDIO_ACCEPTANCE.md) for independent sample
checks, retained failed trials and measured software resource ranges.
Physical mapping, clock lock, sustained hardware deadlines and acoustic protection
remain unqualified.

Desk and Lightdesk native software uses real provider actions with CPU-headless
rendering evidence. Physical displays and controllers remain separate gates.
Lux consumes named analysis with explicit calibration and bounded intensity
automation while preserving human holds and disarmed recovery. The earlier
[eight-input integrated milestone](docs/MODULE_IMPLEMENTATION_MAP.md#execution-checkpoint--2026-10-04)
and [channel-processing increment](docs/CHANNEL_PROCESSING.md) retain their dated
fixed-graph scope and evidence. The offline workflows below remain available.

The first offline automixer now provides causal soundcheck, editable instrument
presets, frozen settings and full-song stereo rendering. A new unity-source workflow
keeps source gains intact and exports comparisons without loudness matching.
The offline balance pass adds measured kit/group fader search and held-out checks.
Manufacturer-reference experiments separate published settings, explicit DSP mappings
and evidence-based adaptations. Instrument tone preparation now applies bounded
guitar body/presence corrections against explicit musician intent. The source-rule
coordinator adds profile-based compressor relief and source-adjustment advice.
Offline review now preserves sources when musical intent is unspecified, admits
filter bypass, and can select the unchanged source sum as FINAL.
[Decision model and current contract →](docs/SOURCE_PRESERVATION.md)
The explicit [artistic FX pass](docs/ARTISTIC_FX.md) adds instrument/style-based
spatial choices with production-engine calibration and separate ensemble checks.
Plan reviews expose individual target residuals and bounds; saved plans can be
audited without audio, and new plans bind their completion record to source hashes.
It builds independently and opens no audio hardware. Mix quality awaits listening.

Version 0.2.3 includes [summing observations and delivery controls](docs/SUMMING_DELIVERY.md):
independent neutral-routing checks, production stage and peak attribution, an explicit
delivery sidecar and validated true-peak finalization. Comparison gain and final
limiting are separate controls. The legacy renderer preserves frozen session and
audio identities; new deliveries retain the selected GigPies effects.

[**Source rules →**](docs/SOURCE_RULES.md) · [**Automatic guitar tone →**](docs/TONE_PASS.md) · [**Manufacturer-reference experiment →**](docs/PRESET_EXPERIMENT.md) · [**Musical balance experiment →**](docs/BALANCE_PASS.md) · [**Unity-source comparison →**](docs/UNITY_PASS.md) · [**Run the offline automixer →**](docs/AUTOMIX.md) · [**Effects and automatic review →**](docs/FX_PASS.md)

The [frozen EQ matcher](docs/EQ_MATCHING.md) adds versioned reference maps, broad
production-EQ fitting, an exact 0–100% amount/reset contract and a local HTML review
page. Artistic directions remain provisional; unchanged tone stays eligible.
An offline comparison of identical recordings reports EQ changes separately from phrase
variation, without changing matching tolerances or selecting another mix.
Readable summaries can also be created from saved diagnostic reports. New FX and
EQ plans require training before held out to protect against continuous DSP history.

The [audio transport prototype](docs/AUDIO_TRANSPORT.md) implements
bounded UDP audio/control and a PA-owned sample timeline, with two-Pi synthetic
validation. The [USB hardware bench](docs/AUDIO_HARDWARE.md) now connects selected stereo
capture/output to real SHR PA, FX and recording, with explicit timing/fault evidence.
Wider live routing, physical fixture output and acoustic acceptance remain
separate. The audio and lighting surfaces are independently built native and
headless clients; this checkout documents their integration and does not bundle
or launch operator windows.
[The component map](docs/COMPONENTS.md) records the related developing SHR projects.
The system is modular: PA processing and measurement are developed in SHR PA,
with the finished module intended for [integration here](docs/COMPONENTS.md#pa-module-and-planned-integration).

## Run the CLI

Install Rust through rustup. This checkout selects **Rust 1.97.1**.

```sh
git clone https://github.com/PaolaShultz/gigpies.git
cd gigpies
cargo build --locked
cargo run --locked -- --help
cargo run --locked -- inspect /path/to/multitrack-folder
```

Inspection prints JSON channel counts, sample rates, bit depths, frame counts and
durations for one WAV or a flat directory. It reads headers only and leaves source
files unchanged. It opens no audio hardware. Invalid inputs return a nonzero status.

## From recordings to a mix

Our first local session is **The Complainiacs — Etc**: 13 original WAV tracks with
drums, bass DI/amp, guitars and vocals. **Dark Ride — Hammer Down** provides a larger
40-track session for later experiments. Both are local educational material; no music
is distributed here. [Source notes and local layout →](docs/MULTITRACKS.md)

The first loop is simple: **inspect → measure → balance → listen → refine**.
Each step should produce something we can assess before adding the next layer.

## Explore the project

| | |
|---|---|
| [**Architecture**](docs/ARCHITECTURE.md) | Audio ownership, node responsibilities and timing |
| [**Brain consoles**](docs/BRAIN_CONSOLE_PLAN.md) | Audio Desk, lighting Lightdesk, two displays/controllers and engine boundaries |
| [**Status & next steps**](docs/STATUS.md) | Implemented behavior and the gradual build sequence |
| [**Existing components**](docs/COMPONENTS.md) | SHR PA, FX, DAW, lighting and library choices |
| [**Development**](docs/DEVELOPMENT.md) | Build, validation, directories and contribution workflow |
| [**Original blueprint**](docs/archive/blueprints/blueprint-v2.md) | Preserved draft with the broader venue vision |
| [**Release notes**](CHANGELOG.md) | What each public version actually delivers |

---

<div align="center">

**Small venues. Sound and light. People in control.**

[MIT code](LICENSE) · [Artwork & third-party notes](THIRD_PARTY.md) · [Report an issue](https://github.com/PaolaShultz/gigpies/issues)

</div>

The configurable modular processing increment is documented in
[Modular processing](docs/MODULAR_PROCESSING.md), including independent strip,
USB/socket and output-patch maps, owner-native configurable PA, shared clock
recovery and [authenticated Brain endpoints](docs/REMOTE_PROCESSING.md).
[Acceptance](docs/MODULAR_ENGINE_ACCEPTANCE.md) distinguishes software results from
physical UMC1820/ADA8200 mapping, lock and deadline qualification. The reference
16-input/18-output patch starts unassigned; 32/48-input software profiles are not
product limits.
