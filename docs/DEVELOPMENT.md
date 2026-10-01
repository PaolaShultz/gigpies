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
