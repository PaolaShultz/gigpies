# Development

## Layout

```text
src/                    Rust library and command entry point
tests/                  fast synthetic integration and CLI tests
examples/               instructions for runnable examples as they are added
sessions/               public session-format guidance; local/ is ignored
docs/                   maintained design, status and development guides
docs/assets/            supplied concept artwork
docs/archive/           preserved draft documents
recordings/             ignored downloads and original audio
artifacts/              ignored renders, reports and scratch evidence
archive/local/          ignored retired local experiments
.github/workflows/      hardware-free Rust CI
```

Keep current decisions in maintained docs. Preserve historically useful drafts under
`docs/archive/` with an index; do not treat them as active implementation requirements.
Use `archive/local/` for private retired experiments and `artifacts/` for disposable
outputs. Never move private recordings into the public documentation archive.

## Validation

```sh
rustc -vV
cargo fmt --all -- --check
cargo check --locked
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
python3 -m unittest discover -s scripts -p 'test_*.py'
cargo build --locked --release
```

Normal tests use generated synthetic WAVs and temporary directories. They protect
metadata interpretation, malformed input handling and CLI contracts. They never
start audio, MIDI, DMX, network services or access third-party music.

The agent selects tests based on changed behavior. Run focused regressions while
implementing; run the normal production suite for engine/rendering, shared models,
routing/persistence, concurrency, safety and broadly reused changes, and releases.
Historical auditions, exhaustive matrices, long benchmarks and private-media checks
belong behind explicit opt-in commands once added. Keep required current production
regressions in the default suite. Report run and intentionally skipped test classes.

## Scope and hardware

Sibling checkouts are references. Keep this project independently buildable; no local
path dependencies or assumptions that another repository has been installed.
Preserve source licences when adapting code. The skeleton starts no hardware.
Hardware sessions and audible playback require a specific authorized task.

Before publication inspect staged files, confirm music/private data are absent,
run `git diff --cached --check`, and run the normal checks above. Version 0.1.0 is
the first foundation release; tag it `v0.1.0`. Later release notes must distinguish
implemented, offline-validated and physically verified behavior.

## Offline automixer

See [AUTOMIX.md](AUTOMIX.md) for commands and explicit private-media test opt-ins.
`src/automix/config.rs` owns editable settings and validation, `dsp.rs` owns signal
processing, and `render.rs` owns offline files/timeline/reports. Production tests
remain hardware-free. Failed render directories are partial and must not be published.

The optional FX/review extension is documented in [FX_PASS.md](FX_PASS.md).
`effects.rs` owns configuration/routing; attributed static engines are in
`fx_engines.rs` and `exciter.rs`; `analysis.rs` owns deterministic measurements and
bounded review rules. `scripts/analyze_mix.py` is optional plotting only and cannot
change settings. All correction decisions remain in Rust and are covered by the
normal synthetic suite.

The [musical balance pass](BALANCE_PASS.md) is owned by `automix/balance.rs`. It
reuses native source reading/routing and DSP, buffers synchronized energy covariance,
and searches musical faders without touching input trims. `scripts/balance_excerpts.py`
is an optional PCM excerpt/export verification tool; it never changes mix settings.

The [manufacturer-reference experiment](PRESET_EXPERIMENT.md) adds explicit shelf
shapes and local parameter mapping. Numerical collections and media stay ignored.

The [tone pass](TONE_PASS.md) is owned by `automix/tone.rs`. It reuses the existing
FFT and production DSP, accepts explicit instrument-path identity, and has focused
normal regressions in `tests/tone.rs`. Its spectral targets are editable intent
choices in a separate policy; it does not change the session persistence schema.

The [source-rule coordinator](SOURCE_RULES.md), `automix/expert.rs`, owns explicit
profile applicability, sustained-compression plans, source advice and joint
validation. `tone.rs` owns guitar spectral measurements and EQ search. New rules
must separate detection, proposed changes, actual validation and remaining defects;
never assign processing from display names. `tests/expert.rs` uses synthetic media
and is part of the normal suite. Public profile/rule examples are our parameters;
manufacturer collections and private media stay local.
