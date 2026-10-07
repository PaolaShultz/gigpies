# GP18-sends:1 / GP07-processing:4 actual producer corpus

Produced by OfflineEngine from committed source
`49798f8ff70185e3789b335ba1665724abacd041`, with bounded synthetic software
samples. `providers.json` identifies the exact source blobs and fixture SHA256;
`SHA256SUMS` covers every generated JSON fixture except the manifest itself.
No audio, hardware acceptance, private session or consumer implementation is included.
The corpus remains a candidate until coordinator review and freeze.

The actual producer executable was the normal Rust test binary compiled from
those identical source blobs before the scoped commit; generation ran after
commit, verified its exact HEAD and emitted source hashes. Its private executable
SHA256 and commands are in the handoff. The verifier checks blobs at the producing
revision, so subsequent producer source changes do not alter historical provenance.

Includes exact-dimension 16/17/32/48 input examples, raw paired baseline,
committed-only intent2, pending admission/readback, boundary final, mid-transition,
settled readback, backpressure, stale revision, reused ID, lease expiry and disarmed
fresh-epoch recovery. GP07v4 includes baseline, real high-input EQ/compressor
mutation, pending/boundary final/settled readback and an actual v3 refusal.
All counters and identities are actual provider output, not hand-authored responses.

Consumer instructions: [send contract](../../../../docs/MONITOR_SENDS.md).
Use GP07:4 for dynamic processing; retain GP07:2 for explicit legacy8/2.
Pair GP03/GP18 revisions, preserve current/target/remaining, and separate tap and
level authority/review. No action replay on reconnect or identity/focus loss.

Generation and verification are explicit opt-ins (`write_sends_producer_corpus`,
`verify_sends_producer_corpus` in `tests/sends.rs`). Generation requires
`GP18_SOURCE_REVISION` matching HEAD and explicit `GP18_FIXTURES`; old GP07
and other historical corpora were preserved byte-for-byte.
