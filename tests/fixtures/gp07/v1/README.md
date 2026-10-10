# GP07-processing:1 producer corpus

These eight synthetic JSON payloads come from the real OfflineEngine, not a
handwritten provider simulation. [The contract](../../../../docs/reference/CHANNEL_PROCESSING.md)
owns units, authority, timing and signal taps. `providers.json` records the exact
producer source snapshot and fixture SHA-256 identities. No executable or media
is included. Consumer copies retain exact bytes and this provenance.

`producer_fixture_v1` in `tests/gp07.rs` runs normally and compares the real
producer results byte-for-byte. The generator is explicitly ignored; only run it
when changing the owning contract or its evidence, under the shared build lock:

```sh
flock -xn /home/shome/p/.gigpies-build.lock \
  env CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=1 \
  cargo +1.97.1 test --locked -j1 --test gp07 write_producer_fixture_v1 -- --ignored --exact
```

Regeneration requires reviewing the data, refreshing hashes and accepting the
consumer amendment. Pending frame48/ticket1 is separate from final frame49;
ready at frame288 ends the crossfade. Readiness feedback does not claim physical
output, metering, listening quality or hardware acceptance.
