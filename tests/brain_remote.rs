use gigpies::{
    remote::{BrainMediaDescriptor, BrainMediaRole, BrainPacket, EngineIdentity},
    show::Counter,
};
fn descriptor() -> BrainMediaDescriptor {
    BrainMediaDescriptor {
        contract: "GP15-media".into(),
        version: 1,
        session: 7,
        stagebox_epoch: 2,
        brain_epoch: 91,
        stagebox_map: 3,
        brain_map: 4,
        generation: 5,
        selection_generation: 6,
        hold_generation: 7,
        sample_rate: 48000,
        frames: 48,
        talkback: true,
        monitor: true,
    }
}
#[test]
fn independently_indexed_roles_roundtrip_and_refuse_stale() {
    let descriptor = descriptor();
    descriptor
        .validate(
            7,
            &EngineIdentity {
                source_epoch: Counter(2),
                capability_generation: Counter(1),
                map_generation: Counter(3),
            },
        )
        .unwrap();
    for role in [BrainMediaRole::Talkback, BrainMediaRole::Monitor] {
        let samples = (0..48 * role.channels())
            .map(|i| i as f64 / 128.)
            .collect::<Vec<_>>();
        let packet = BrainPacket {
            descriptor: descriptor.clone(),
            role,
            frame: 48 * 999_999,
            samples: samples.clone(),
        };
        let bytes = packet.encode().unwrap();
        let decoded = BrainPacket::decode(&bytes, &descriptor, role).unwrap();
        assert_eq!(decoded.frame, packet.frame);
        assert_eq!(decoded.samples, samples);
        for field in 0..8 {
            let mut stale = bytes.clone();
            stale[8 + field * 8 + 7] ^= 1;
            assert!(BrainPacket::decode(&stale, &descriptor, role).is_err());
        }
        let wrong = if role == BrainMediaRole::Talkback {
            BrainMediaRole::Monitor
        } else {
            BrainMediaRole::Talkback
        };
        assert!(BrainPacket::decode(&bytes, &descriptor, wrong).is_err());
        for length in [0, 3, 7, 79, bytes.len() - 1] {
            assert!(BrainPacket::decode(&bytes[..length], &descriptor, role).is_err());
        }
    }
}
#[test]
fn finite_exact_shape_and_version_required() {
    let mut descriptor = descriptor();
    let identity = EngineIdentity {
        source_epoch: Counter(2),
        capability_generation: Counter(1),
        map_generation: Counter(3),
    };
    descriptor.version = 2;
    assert!(descriptor.validate(7, &identity).is_err());
    descriptor.version = 1;
    let mut packet = BrainPacket {
        descriptor,
        role: BrainMediaRole::Talkback,
        frame: 0,
        samples: vec![0.; 48],
    };
    for value in [f64::NAN, f64::INFINITY, f64::MAX] {
        packet.samples[0] = value;
        assert!(packet.encode().is_err());
    }
    packet.samples[0] = 0.;
    packet.frame = 1;
    assert!(packet.encode().is_err());
    packet.frame = 0;
    let mut bytes = packet.encode().unwrap();
    bytes[80..84].copy_from_slice(&f32::NAN.to_be_bytes());
    assert!(BrainPacket::decode(&bytes, &packet.descriptor, packet.role).is_err());
}

#[test]
fn unrelated_role_changes_do_not_invalidate_continuous_media() {
    let original = descriptor();
    let monitor = BrainPacket {
        descriptor: original.clone(),
        role: BrainMediaRole::Monitor,
        frame: 480,
        samples: vec![0.125; 96],
    }
    .encode()
    .unwrap();
    let talkback = BrainPacket {
        descriptor: original.clone(),
        role: BrainMediaRole::Talkback,
        frame: 960,
        samples: vec![0.25; 48],
    }
    .encode()
    .unwrap();
    let mut held = original.clone();
    held.generation += 1;
    held.hold_generation += 1;
    assert!(BrainPacket::decode(&monitor, &held, BrainMediaRole::Monitor).is_ok());
    assert!(BrainPacket::decode(&talkback, &held, BrainMediaRole::Talkback).is_err());
    let mut selected = original;
    selected.generation += 1;
    selected.selection_generation += 1;
    assert!(BrainPacket::decode(&talkback, &selected, BrainMediaRole::Talkback).is_ok());
    assert!(BrainPacket::decode(&monitor, &selected, BrainMediaRole::Monitor).is_err());
}

#[test]
fn bridge_telemetry_uses_strict_integer_wire_units() {
    use gigpies::brain_audio::bridge::{Bridge, BridgeConfig, BridgeEpochs};
    let bridge = Bridge::prepare(
        BridgeConfig::voice(2),
        BridgeEpochs {
            source: 1,
            destination: 2,
            route: 3,
        },
    )
    .unwrap();
    let wire = gigpies::remote::brain_bridge_telemetry(&bridge.status());
    assert_eq!(wire["ratio_ppb"], 1_000_000_000i64);
    assert_eq!(wire["skew_ppb"], 0i64);
    assert!(wire["physical_mapping_uncertainty_milliframes"].is_null());
    assert_eq!(wire["physical_clock_lock_verified"], false);
    let encoded = gigpies::remote::encode(&wire).unwrap();
    let decoded: serde_json::Value = gigpies::remote::decode(&encoded).unwrap();
    assert_eq!(decoded, wire);
}
