# E09 / GP04-local:1 invented corpus

`e09.json` is reusable synthetic contract data, not generated media. Its samples
are exact signed PCM24 invented from source frame and explicit input index using
the provenance formula recorded in the file. `packets_hex` contains unchanged GPA1
bytes; `wire_cases` wraps complete windows in the LOCAL GAW1 framing payload.
The Unix four-byte length prefix is separate from each window payload.

Descriptor E09: epoch9,stream3,48000Hz,frame48000,calibration2,map1, four ordered
kick/bass/guitar-1/guitar-2 mapped explicitly to input01/02/03/04,raw-pre-fader.
Wire oldest acquisition is1000ms in the same-host Linux CLOCK_MONOTONIC domain.
Consumer calls with now1101 refuse age;999 refuse future. Epoch/map/calibration,
gap,loss and continuous cases have exact hex bytes. `cases` also records44.1kHz,
reordering,overlap and stale-window expectations. `tests/gp04.rs` executes the
wire corpus and independently compares real tap PCM to the stated eight-input
source. Temporary Unix integration tests exercise the actual provider executable.

Wire layout/requirements: [ANALYSIS_STREAM.md](../../../../docs/ANALYSIS_STREAM.md).

`e09r-44100-descriptor.json` is the exact refused E09R descriptor.
`e09-map-change-descriptor.json` and `e09-epoch-change-descriptor.json` are explicit
new attachment identities; neither may reinterpret an old attachment's packets.
