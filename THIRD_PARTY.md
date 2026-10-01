# Source, artwork and media

The Rust foundation and the code-authored SVG banner/architecture diagram are original project work under MIT. No code from sibling SHR
repositories is copied into v0.1.0. Dependency versions are locked in Cargo.lock;
upstream dependency licences continue to apply.

- hound: Apache-2.0 WAV library.
- serde and serde_json: MIT OR Apache-2.0 serialization libraries.
- Transitive dependencies: see each package's licence metadata in the locked graph.

`docs/assets/gigpies-concept.png` is AI-generated concept artwork supplied by the
project owner. It is preserved as a concept reference, with limitations documented in
`docs/CONCEPT_REVIEW.md`. Third-party names and marks belong to their owners; no
endorsement or ownership of those marks is claimed.

The Complainiacs / Dark Ride recordings are local educational experiment material.
Their source notices remain with local files. They are not covered by this project's
MIT licence and are not included in Git, tests, CI, packages or releases.

## Offline automixer adaptations

`src/automix/dsp.rs` adapts normalized transposed-direct-form biquads, HPF/bell
coefficient equations and soft-knee compression from:

- [SHR PA](https://github.com/PaolaShultz/shr-pa), `src/dsp.rs`, inspected commit
  `ccc816a97f1957549a2a515d94a672dc78c59012`.
  [Preserved MIT licence](licenses/shr-pa-MIT.txt).
- [SHR DAW](https://github.com/PaolaShultz/shr-daw), `src/effects/compressor.rs`,
  `eq.rs`, `channel.rs`, inspected commit `c3d2e6517cc60c7a75a319655246a54130437875`.
  Stereo maximum detection and attack/release smoothing inform our linked compressor.
  [Preserved MIT licence](licenses/shr-daw-MIT.txt).

SHR DAW gate/mixing and SHR PA metering/offline rendering were reviewed as references.
SHR FX `src/dsp.rs` at `9b9a2a94f8fe79389fba03d065924c0dd335599f` was inspected
for effects/send-return structure during the first pass.
The subsequent effects pass adapts its `src/dsp.rs` Ring/Tap/allpass/comb reverb and
chorus implementations into `src/automix/fx_engines.rs`, and its four-times-oversampled
`src/exciter.rs` into `src/automix/exciter.rs`, including harmonic/alias tests. Static
configuration and local type paths replace host controls; unused reset paths are omitted.
[Preserved SHR FX MIT licence](licenses/shr-fx-MIT.txt).
The new delay, routing, wet-return calibration and deterministic spectral review are
GigPies implementations; no sibling checkout is changed or linked.
The gain estimator, routing, file workflow and preset choices are GigPies work.
Manufacturer guidance and the loudness standard are linked in [AUTOMIX](docs/AUTOMIX.md).

The shelf coefficient equations follow Robert Bristow-Johnson's mathematical
[Audio EQ Cookbook, published by W3C](https://www.w3.org/TR/audio-eq-cookbook/).
Our implementation fixes shelf slope to S=1. No manufacturer DSP code, firmware,
manual or preset collection is included. See [reference-use boundaries](docs/PRESET_EXPERIMENT.md).
