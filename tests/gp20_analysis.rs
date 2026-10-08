use gigpies::{
    analysis_stream::{Descriptor, Mapping, PACKET_BYTES, Tap},
    topology::EngineTopology,
    transport::Packet,
};

fn mapping() -> Mapping {
    Mapping {
        version: 1,
        inputs: ["input-17", "input-03", "input-09", "input-01"].map(String::from),
    }
}

#[test]
fn configured_mapping_is_admitted_against_real_inventory_and_preserves_v1() {
    let topology = EngineTopology::software(17, 3, 0).unwrap();
    assert_eq!(
        serde_json::to_value(&topology).unwrap(),
        serde_json::from_str::<serde_json::Value>(include_str!("fixtures/gp04/v2/topology.json"))
            .unwrap()
    );
    let d = mapping().descriptor(&topology, 9, 0).unwrap();
    assert_eq!(d.version, 2);
    assert_eq!(d.map_revision.0, topology.map_revision);
    assert_eq!(
        serde_json::to_value(&d).unwrap(),
        serde_json::from_str::<serde_json::Value>(include_str!(
            "fixtures/gp04/v2/configured-descriptor.json"
        ))
        .unwrap()
    );
    assert_ne!(d.attach_request(), Descriptor::new(9, 0).attach_request());
    assert!(
        mapping()
            .descriptor(&EngineTopology::legacy(), 9, 0)
            .is_err()
    );
    let mut legacy = d.clone();
    legacy.version = 1;
    legacy.subscription = "lux.aux.v1".into();
    assert!(legacy.validate().is_err());
    for invalid in [
        "input-001",
        "input-0",
        "input-65536",
        "input-+1",
        "input-1 ",
        "input-18",
        "input-03",
    ] {
        let mut m = mapping();
        m.inputs[0] = invalid.into();
        assert!(m.descriptor(&topology, 9, 0).is_err(), "{invalid}");
    }
    assert!(
        serde_json::from_str::<Mapping>(
            r#"{"version":1,"version":1,"inputs":["input-01","input-02","input-03","input-04"]}"#
        )
        .is_err()
    );
    assert!(
        serde_json::from_str::<Mapping>(
            r#"{"version":1,"inputs":["input-01","input-02","input-03","input-04"],"other":true}"#
        )
        .is_err()
    );
}

#[test]
fn configured_tap_proves_every_selected_raw_sample_across_supported_sizes() {
    for count in [17, 32, 48] {
        let topology = EngineTopology::software(count, 3, 0).unwrap();
        let d = mapping().descriptor(&topology, 9, 0).unwrap();
        let (mut tap, mut receiver) = Tap::new(&d).unwrap();
        for block in 0..10 {
            let raw: Vec<i32> = (0..count * 48)
                .map(|i| ((block * 48 + i / count) * 100 + i % count) as i32 - 30000)
                .collect();
            tap.offer_interleaved(block as u64 * 48, &raw, count, block as u64)
                .unwrap();
        }
        let window = receiver.pop().unwrap();
        window.validate(&d, 0, 0).unwrap();
        for bytes in window.packets.chunks_exact(PACKET_BYTES) {
            let packet = Packet::parse(bytes).unwrap();
            for frame in 0..48 {
                for (channel, index) in [16, 2, 8, 0].iter().enumerate() {
                    let expected =
                        ((packet.source_frame() as usize + frame) * 100 + index) as i32 - 30000;
                    assert_eq!(packet.pcm(frame * 4 + channel).unwrap(), expected);
                }
            }
        }
    }
}
