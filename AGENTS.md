# GigPies working agreements

Read README.md, docs/STATUS.md and the relevant owning document before changing code.
Build part by part, starting with offline band automixing. Ask questions when a concrete
next step needs an answer; do not front-load the whole project's unknowns.

The system is modular across GigPies and the related `../shr-*` projects. Follow
the ownership map in docs/COMPONENTS.md: PA processing, measurement and alignment
are developed in `../shr-pa`, with the finished PA module intended for integration
here. Keep module tasks and algorithms in their owning project; avoid parallel
implementations in GigPies. Track integration and hardware acceptance separately.

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
Follow docs/PUBLICATION.md. Enable the versioned hooks when absent and run the
publication guard against the complete index. New scripts need a reviewed entry in
scripts/publication-policy.json; private user data and one-off runners stay ignored.

Task0015 clock direction: Brain has one local duplex sound-card owner for operator
talkback capture and monitor playback. Stagebox plus its ADAT expansion remains
the processing reference; Brain local I/O has a separate device epoch/timeline.
Both new crossings require bounded ASRC. Keep FX, raw REC and source analysis on
Stagebox frames. Desk controls the owner without opening PCM. Physical activation,
clock lock, socket mapping and acoustic qualification remain separate acceptance.
