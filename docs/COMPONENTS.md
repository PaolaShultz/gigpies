# Existing components and dependencies

Sibling projects were inspected on 2026-10-01. The local inventory and README scope
were refreshed on 2026-10-02, with a source/test inspection of SHR PA's generator,
delays and polarity controls. This is not a fresh implementation audit of every
sibling. All remain in development. Their own source and status documents own
current details. The offline automixer adapts
narrow biquad/dynamics equations from SHR PA/DAW with preserved MIT notices; see
[attribution](../THIRD_PARTY.md). No sibling is linked as a dependency. The PA
development task below was added to SHR PA with explicit user authorization.

## Module ownership

The intended system is modular across these projects. Develop each module in its
owning repository, then integrate the finished component into GigPies through a
defined interface. In particular, PA processing and measurement belong in SHR PA;
GigPies should not grow a second PA implementation. Standalone tools can remain
usable alongside the integrated system.

Record new module tasks in the owner's roadmap/status documents so development
continues there in future threads. Keep GigPies' integration intent and contracts
here. Changes to another repository still require authorization; local paths below
are relative to the GigPies root and do not introduce build dependencies.

| Owning checkout | Area and project documents |
|---|---|
| `../shr-pa` | PA DSP, crossover, generator, alignment, protection and measurement: `docs/STATUS.md`, `docs/DSP.md`, `docs/DRIVERACK_MAP.md` |
| `../shr-fx` | Send/return effects: `docs/PLAN.md`, `docs/ACCEPTANCE.md` |
| `../shr-daw` | Workstation DSP, graph, controllers and existing recording code: `docs/WORKSPACE_HANDOFF.md` |
| `../shr-lux` | Lighting and source-activity analysis: `idea.md`, `docs/index.md` |
| `../shr-rec` | Standalone multichannel recording/playback destination: `docs/STATUS.md`, `docs/PLAN.md` |
| `../shr-drums` | Drum engine, kit building and source provenance: `README.md`, `FORMAT.md`, `SOURCES.md` |
| `../shr-synth` | Synth engines and presets: `docs/HANDOFF.md`, `docs/PRESET_SCHEMA.md` |
| `../shr-sampler` | Sample instruments, SFZ import and hosting: `docs/NATIVE_PACKAGE_FORMAT.md`, `docs/LIVE_PROCESS_CONTRACT.md` |
| `../shr-tone-over-9000` | NAM/cabinet processing and prepared chains: `README.md` |
| `../shr-skills` | Development workflow skills: `README.md`, `AGENTS.md`; not an audio engine |

The shared media library is `../waves`; follow [local media ownership](LOCAL_MEDIA.md).
These local paths are ownership references, not build or CI requirements.

## Existing work and integration limits

| Project | Useful existing work | Integration limit |
|---|---|---|
| [shr-pa](https://github.com/PaolaShultz/shr-pa) | Rust 2-input/6-output PA DSP, EQ, crossovers, pink/white generators, pair delay/polarity, compression, limiting, presets and offline rendering | Fixed routing and linked L/R pair settings; setup-mic/RTA/automatic alignment and physical acoustic acceptance remain pending; direct ALSA transport |
| [shr-fx](https://github.com/PaolaShultz/shr-fx) | Two wet-only engines, up to eight parallel slots each; reverb, delay, chorus, exciter | Send/return semantics; JACK host; listening and live acceptance remain |
| [shr-daw](https://github.com/PaolaShultz/shr-daw) | Channel processing, EQ, dynamics, audio graph, controller support and recording | Coupled to workstation models; adapt narrowly and preserve provenance |
| shr-lux | Offline multitrack replay, source activity/kick analysis, lighting simulation and pad previews | Private reference; actual DMX/live capture remain pending |
| shr-rec | Recorder/player application shell | Audio recording/playback not wired yet |
| shr-drums, shr-synth, shr-sampler | Separate instrument engines and offline render/host patterns | Useful references, not automixing prerequisites |
| shr-tone-over-9000 | NAM live processor, prepared chain changes | Guitar processing; no need to integrate for the first mix experiment |
| shr-skills | Development workflow material | Not an audio runtime component |

## PA module and planned integration

The user requested phase measurement and small-delay correction as a **SHR PA
development task**, with the finished PA module intended for GigPies integration.
The canonical task is [SHR PA's phase-alignment plan](https://github.com/PaolaShultz/shr-pa/blob/main/docs/PHASE_ALIGNMENT.md),
also linked from its `AGENTS.md`, status, P4/P5 roadmap and function map.
Locally, the same task is `../shr-pa/docs/PHASE_ALIGNMENT.md` from this repository.

SHR PA already supplies pink noise, pair polarity and 0–10 ms pair delays at
nearest-sample resolution. Its task adds synchronized reference/mic measurement,
phase/confidence analysis and a verified delay/polarity proposal stage. L/R settings
are currently paired; independent-output changes require an explicit contract update.

GigPies will integrate the finished module rather than duplicate those algorithms.
Packaging and the host/control interface remain undecided. Preserve one audio-device
owner, explicit timing/protection/state contracts and standalone PA operation.
Module implementation, GigPies integration and acoustic hardware acceptance are
separate milestones; none is completed by recording this task.

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
