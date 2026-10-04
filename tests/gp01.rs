use gigpies::{
    roles::{Binding, Display, Registry, Role, unambiguous_displays},
    show::{Availability, Counter, Manifest},
};
const E01: &[u8] = include_bytes!("fixtures/gp01/e01.json");
const E02: &[u8] = include_bytes!("fixtures/gp01/e02.json");
fn lighting() -> Binding {
    Binding {
        role: Role::LightingDesk,
        display_connector: "test-hdmi-b".into(),
        display_edid: "test-edid-b".into(),
        controller: "test-controller-b".into(),
        profile: "test-profile-b".into(),
        operator_label: None,
    }
}
#[test]
fn e01_compatibility_has_no_engine_side_effects() {
    let m = Manifest::decode(E01).unwrap();
    let before = m.clone();
    let a = m
        .attach(&[("audio", "C-AUDIO", 1), ("lighting", "C-LIGHT", 2)])
        .unwrap();
    assert!(a.read_only);
    assert_eq!(a.modules[1].1, Availability::Version);
    assert_eq!(m, before);
    assert!(
        !m.attach(&[("audio", "C-AUDIO", 1), ("lighting", "C-LIGHT", 1)])
            .unwrap()
            .read_only
    );
    assert!(m.attach(&[("audio", "C-AUDIO", 1)]).is_err());
}
#[test]
fn e02_conflicts_and_stale_release_preserve_exact_state() {
    let mut r = Registry::decode(E02).unwrap();
    let before = r.clone();
    let mut b = lighting();
    b.controller = "test-controller-a".into();
    assert_eq!(r.claim(Counter(7), b).unwrap_err(), "conflict");
    assert_eq!(r, before);
    assert!(
        r.release(Counter(6), Role::AudioDesk, "test-controller-a")
            .is_err()
    );
    assert_eq!(r, before);
    assert_eq!(r.claim(Counter(7), lighting()).unwrap(), Counter(8));
    assert_eq!(
        r.release(Counter(8), Role::LightingDesk, "test-controller-b")
            .unwrap(),
        Counter(9)
    );
    let d = vec![
        Display {
            connector: "a".into(),
            edid: "same".into(),
        },
        Display {
            connector: "b".into(),
            edid: "same".into(),
        },
    ];
    assert!(unambiguous_displays(&d).is_empty());
}
#[test]
fn strict_decoding_and_bounds() {
    let s = std::str::from_utf8(E01).unwrap();
    for bad in [
        s.replace("\"version\": 1,", "\"version\": 1,\"version\":1,"),
        s.replace("\"version\": 1,", "\"version\": 2,"),
        s.replace("\"manifest_revision\": \"1\"", "\"manifest_revision\": 1"),
        s.replace("\"format\":", "\"unknown\":0,\"format\":"),
        format!("{s} null"),
        s.replace("\"id\":\"lighting\"", "\"id\":\"audio\""),
    ] {
        assert!(Manifest::decode(bad.as_bytes()).is_err(), "{bad}");
    }
    assert!(Manifest::decode(&[0xff]).is_err());
    assert!(Manifest::decode(&vec![b' '; 65537]).is_err());
    assert!(Manifest::decode(format!("{}0{}", "[".repeat(13), "]".repeat(13)).as_bytes()).is_err());
    let mut m = Manifest::decode(E01).unwrap();
    m.modules[0].version = 2;
    assert!(m.validate().is_err());
    let mut r = Registry::decode(E02).unwrap();
    r.generation = Counter(u64::MAX);
    let before = r.clone();
    assert!(r.claim(Counter(u64::MAX), lighting()).is_err());
    assert_eq!(r, before);
    let mut b = lighting();
    b.display_edid = "test-edid-a".into();
    assert!(Registry::decode(E02).unwrap().claim(Counter(7), b).is_err());
}
#[test]
fn private_atomic_checkpoints_preserve_valid_previous_state() {
    use std::{fs, os::unix::fs::PermissionsExt};
    let dir = std::env::temp_dir().join(format!("gigpies-gp01-{}", std::process::id()));
    fs::create_dir(&dir).unwrap();
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o700)).unwrap();
    let mut m = Manifest::decode(E01).unwrap();
    m.save(&dir, "show.json").unwrap();
    let old = fs::read(dir.join("show.json")).unwrap();
    assert_eq!(
        fs::metadata(dir.join("show.json"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    m.version = 2;
    assert!(m.save(&dir, "show.json").is_err());
    assert_eq!(fs::read(dir.join("show.json")).unwrap(), old);
    m.version = 1;
    m.manifest_revision = Counter(2);
    m.save(&dir, "show.json").unwrap();
    assert_eq!(
        Manifest::decode(&fs::read(dir.join("show.json")).unwrap()).unwrap(),
        m
    );
    Registry::decode(E02)
        .unwrap()
        .save(&dir, "roles.json")
        .unwrap();
    assert!(m.save(&dir, "../escape").is_err());
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(m.save(&dir, "show.json").is_err());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn optional_unknown_modules_remain_unavailable_and_duplicate_edids_need_choice() {
    let mut m = Manifest::decode(E01).unwrap();
    m.modules[1].required = false;
    m.modules[1].contract = "C-FUTURE".into();
    let result = m
        .attach(&[("audio", "C-AUDIO", 1), ("lighting", "C-FUTURE", 1)])
        .unwrap();
    assert_eq!(result.modules[1].1, Availability::Unavailable);
    assert!(!result.read_only);
    let mut r = Registry::decode(E02).unwrap();
    r.bindings[0].operator_label = Some("left".into());
    let mut b = lighting();
    b.display_edid = "test-edid-a".into();
    b.operator_label = Some("right".into());
    r.claim(Counter(7), b).unwrap();
    r.bindings[1].operator_label = Some("left".into());
    assert!(r.validate().is_err());
}

#[test]
fn optional_missing_or_incompatible_does_not_disable_required_audio() {
    let mut m = Manifest::decode(E01).unwrap();
    m.modules[1].required = false;
    for providers in [
        vec![("audio", "C-AUDIO", 1)],
        vec![("audio", "C-AUDIO", 1), ("lighting", "C-LIGHT", 2)],
    ] {
        let a = m.attach(&providers).unwrap();
        assert!(!a.read_only);
        assert_eq!(a.modules[0].1, Availability::Compatible);
        assert_ne!(a.modules[1].1, Availability::Compatible);
    }
    assert!(m.attach(&[("audio", "C-AUDIO", 2)]).unwrap().read_only);
    assert!(m.attach(&[]).is_err());
}
