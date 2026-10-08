# Configured named-source corpus

The existing four semantic names bind to explicit distinct raw logical strips.
`configured-descriptor.json` is checked against the actual producer serializer by
`cargo test --locked --test gp20_analysis`. `mapping.json` is the bounded startup
configuration for a topology containing at least seventeen inputs. Neither file
asserts physical socket mapping or acoustic verification.

The descriptor and request explicitly select C-ANALYSIS:2 / lux.aux.v2. Existing
v1 fixtures are unchanged. Packet and GAW1 framing, age, source-frame progression,
loss and calibration checks remain unchanged. See the producer contract in
`docs/CONFIGURED_ANALYSIS.md` for admission and recovery.

`topology.json` is the exact serialized `EngineTopology::software(17, 3, 0)`
software-only configuration, asserted against the owner constructor by that test.
Consumers can pass it directly to `gigpies-headless --topology`.
