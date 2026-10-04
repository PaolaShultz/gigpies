# Named analysis provider — GP-04 / C-ANALYSIS:1

Opt-in `gigpies-headless --directory ABS_PRIVATE_0700_DIR --show UUID --epoch N
--synthetic-source fouraux --analysis` serves same-effective-UID clients on `analysis.sock` mode0600. No
physical endpoints, resampling, playback or production network endpoint are opened.
The default audio service remains unchanged. The explicit fouraux source flag selects invented signed
PCM24 eight-input blocks that feed both the real mixer and its raw input tap; inputs
01..04 map explicitly to kick,bass,guitar-1,guitar-2. Inputs05..08 still feed the
mixer and are never substituted into this subscription. This is offline-unprotected.

## Exact LOCAL wire proposal GP04-local:1

Every frame has a four-byte unsigned big-endian payload length. Client sends the
exact UTF-8 payload `{"subscription":"lux.aux.v1","version":1}` (41bytes).
Only this bounded request is accepted; malformed/extra bytes close the connection.
Provider replies with a JSON descriptor (see E09 corpus), then binary whole-window
frames. No control request or source algorithm executes in the tap callback.
Descriptor clock is `linux-clock-monotonic-ms`. Descriptor counters are canonical decimal strings. It declares C-ANALYSIS version1,
subscription, source_epoch, stream3, sample_rate48000, first_frame, ordered sources,
explicit input IDs, raw-pre-fader tap, map_revision and calibration_revision.
Only 48kHz is supported. Descriptor identity must be validated before admission.
Source/map/calibration changes require disconnect and a new validated descriptor.
A live attachment never reinterprets a changed source map.

Binary payload is6280bytes: offsets0..4 `GAW1`,4..12 cumulative lost windows u64BE,
12..20 map_revision u64BE,20..28 calibration_revision u64BE,28..36 oldest (first-packet acquisition) CLOCK_MONOTONIC milliseconds u64BE,36..40 reserved zeros,
40..6280 ten unchanged624byte GPA1 analysis PCM24 packets. Each packet is48frames
of four frame-interleaved signed big-endian24bit samples. GPA1 epoch,stream,frame,
sequence,48kHz and reserved bytes retain their original meaning. Initial sequence
is0; frame comes from the descriptor. Windows span480frames/10ms at48kHz.
A new subscriber can join later: the first delivered GPA1 frame/sequence establishes
its observed start; descriptor first_frame is the source origin, not attachment time.
Consumers reject stale/reordered/overlap/gap packets, epoch/map/calibration changes,
nonzero reserved bytes or invalid lengths; missing data never becomes musical silence.
Consumers MUST compare oldest (first-packet acquisition) CLOCK_MONOTONIC milliseconds against their own
same-host clock, reject future timestamps and age above100ms; new receipt alone
never makes kernel-buffered old audio fresh. Loss increase or skipped frame
invalidates temporal interpretation. Lux owns fresh
calibration/confidence and preserving human Holds; this provider makes no features.

The allocation-free tap uses a preallocated two-whole-window SPSC queue. A separate owned transport thread
consumes at most two windows per tick and fans out to at most8subscribers,
each with at most two pending frames (a partial frame counts toward capacity).
An occupied subscriber queue drops the new whole window and increments its own
loss count; the tap has independent global loss accounting. Sent loss is the sum.
The JSON handshake temporarily occupies one pending slot. Unix kernel socket
buffering is bounded by the OS separately; no partial frame is discarded/interleaved.
Each tick accepts at most8clients and writes at most8192bytes/client. Incomplete
handshake and stalled writes expire after2000ms. Expired pending windows disconnect the reader if partly sent; write progress
cannot extend the100ms total frame lifetime. Transport failure marks analysis
unavailable while mixer frame/control processing continues. Actual socket parsing and all
allocation/I/O stay on the transport thread outside the raw tap and mixer. Audio/REC never wait for Lux.

## Validation and limits

`tests/gp04*` protect exact PCM/window identity, explicit mapping, malformed and
44.1kHz refusal, gaps/reordering and independent bounded loss; synthetic process
checks open only owned temporary Unix endpoints. `tests/fixtures/gp04/v1` contains
invented data/hex provenance, never media recordings. Hardware, combined-load
throughput, latency and lighting algorithm acceptance are separate owner gates.

Default constant GP03 source cannot attach fouraux analysis. Select fouraux explicitly
before processing; analysis attachment itself never changes mixer source samples.

Reproducible device-free checks (serialize Cargo work under the owning workspace's
build policy, using the normal target and profiles):

```sh
CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=1 cargo +1.97.1 test --locked -j1 --test gp04 --test gp04_alloc --test gp04_local
CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=1 cargo +1.97.1 test --locked -j1 --all-targets --features hardware-host
cargo +1.97.1 fmt --all -- --check
CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=1 cargo +1.97.1 clippy --locked -j1 --all-targets --features hardware-host -- -D warnings
CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=1 cargo +1.97.1 build --locked -j1 --release --features hardware-host
python3 -m unittest discover -s scripts -p 'test_*.py'
```

Historical media/audition/exhaustive, UDP/load and hardware tests are intentionally
skipped for this software scope.
