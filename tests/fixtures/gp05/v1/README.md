# GP05 module integration corpus

`providers.json` pins accepted independent provider commits and header/library
hashes. It contains identities only; no executable libraries are distributed.
`status-request.json` is the strict separately queried `GP05-modules:1` envelope.
`status-finalized.json` records an actual synthetic owner-library run, including
terminal observer progress after consuming finish. Counters are canonical decimal
strings; null durable frames means unknown. The eight-input raw mapping is shared
with GP04, and physical acceptance remains unverified.

See [standalone commands and bounds](../../../../docs/reference/MODULE_GRAPH.md). Generate
hashes with `sha256sum tests/fixtures/gp05/v1/*.json`. Default normal tests protect
codec/authority/absence behavior; explicit actual-library tests additionally prove
PCM24 stems, fixed wet+dry through PA, observer lifetime, cancellation, faults and
reset/recreation. No devices or third-party media are discovered.
