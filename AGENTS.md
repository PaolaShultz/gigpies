# GigPies working agreements

Read README.md, docs/STATUS.md and the relevant owning document before changing code.
Build part by part, starting with offline band automixing. Ask questions when a concrete
next step needs an answer; do not front-load the whole project's unknowns.

Use Rust 1.97.1, edition 2024, and committed Cargo.lock. Keep sibling repositories
read-only unless the user explicitly authorizes changes there. Avoid path dependencies.
Distinguish planned, implemented, offline-validated and hardware-verified behavior.
Update focused documentation alongside behavior. Preserve original drafts in docs/archive.

Keep third-party recordings, source archives, private sessions and generated audio out
of Git and releases. Preserve local source notices. No automatic media downloads in CI.
No audio, MIDI, DMX, service or host-audio changes without authorization for that session.
Live audio callbacks must have bounded work and no allocations, locks or I/O.

The agent owns test selection. Keep fast production unit, contract, schema, safety,
recovery and focused regression tests in the default suite. Run focused tests during
implementation; run the complete normal suite for engine, rendering, shared schemas,
routing/persistence, concurrency, safety or broadly reused changes and before releases.
Historical research, auditions, exhaustive matrices, long benchmarks and disposable
renderers are opt-in once their evidence is recorded. Run them only when directly
relevant or requested. Reclassify slow one-time tests found in scoped work and document
their on-demand command. Report run and intentionally skipped classes.

Before committing inspect live Git state and staged content; preserve unrelated edits.
Never publish private audio or state. Publication follows the user's authorized scope.
