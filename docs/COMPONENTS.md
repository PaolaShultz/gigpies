# Existing components and dependencies

Sibling projects were inspected on 2026-10-01. All remain in development.
Their own source and status documents own current details; no sibling code was copied
or linked into this release.

| Project | Useful existing work | Integration limit |
|---|---|---|
| [shr-pa](https://github.com/PaolaShultz/shr-pa) | Rust 2-input/6-output PA DSP, EQ, crossovers, delay, compression, limiting, presets and offline rendering | Fixed routing; full PA feature plan and physical acceptance remain incomplete; direct ALSA transport |
| [shr-fx](https://github.com/PaolaShultz/shr-fx) | Two wet-only engines, up to eight parallel slots each; reverb, delay, chorus, exciter | Send/return semantics; JACK host; listening and live acceptance remain |
| [shr-daw](https://github.com/PaolaShultz/shr-daw) | Channel processing, EQ, dynamics, audio graph, controller support and recording | Coupled to workstation models; adapt narrowly and preserve provenance |
| shr-lux | Offline multitrack replay, source activity/kick analysis, lighting simulation and pad previews | Private reference; actual DMX/live capture remain pending |
| shr-rec | Recorder/player application shell | Audio recording/playback not wired yet |
| shr-drums, shr-synth, shr-sampler | Separate instrument engines and offline render/host patterns | Useful references, not automixing prerequisites |
| shr-tone-over-9000 | NAM live processor, prepared chain changes | Guitar processing; no need to integrate for the first mix experiment |
| shr-skills | Development workflow material | Not an audio runtime component |

## Libraries

Active dependencies are intentionally small:

| Library | Current use |
|---|---|
| hound 3.5.1 | WAV metadata reading; already used by SHR projects |
| serde 1.0.229 | Typed report serialization |
| serde_json 1.0.151 | JSON command output |

Rust 1.97.1 and edition 2024 match the current related projects.
`Cargo.lock` owns the complete resolution.

Candidate libraries when their features are built: `alsa` for direct Linux device
access, `jack` for JACK integration, `rtrb` for bounded single-producer/single-consumer
queues, `ratatui`/`crossterm` for terminal UI and `signal-hook` for shutdown handling.
These already appear in the related projects but are not dependencies of this skeleton.
Select one audio-device owner; do not independently attach PA and FX device transports
and assume they form one low-latency mixer. Pure DSP and host transport are separate.
FFT, resampling, networking and web libraries will be chosen when requirements exist.
