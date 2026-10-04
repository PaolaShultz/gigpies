#![cfg(unix)]
use gigpies::{
    roles::{
        Binding, Role,
        native::{Acquire, ControllerDescriptor, DisplayDescriptor, Inventory, Lease},
    },
    show::Counter,
};
use std::{
    fs,
    io::{BufRead, BufReader, Write},
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::{Command, Stdio},
};
static TEST_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "gp09-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&p).unwrap();
        fs::set_permissions(&p, fs::Permissions::from_mode(0o700)).unwrap();
        Self(p)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn request(role: Role, generation: u64) -> Acquire {
    let suffix = if role == Role::AudioDesk { "a" } else { "b" };
    Acquire {
        format: "gigpies-role-acquire".into(),
        version: 1,
        expected_generation: Counter(generation),
        binding: Binding {
            role,
            display_connector: format!("hdmi-{suffix}"),
            display_edid: format!("edid-{suffix}"),
            controller: format!("controller-{suffix}"),
            profile: "test-profile".into(),
            operator_label: None,
        },
        inventory: Inventory {
            displays: vec![DisplayDescriptor {
                connector: format!("hdmi-{suffix}"),
                edid: format!("edid-{suffix}"),
                operator_label: None,
            }],
            controllers: vec![ControllerDescriptor {
                identity: format!("controller-{suffix}"),
                serial: Some(format!("serial-{suffix}")),
                profile: "test-profile".into(),
                operator_label: None,
            }],
        },
    }
}
#[test]
fn concurrent_roles_crash_reclaim_and_cas() {
    let _guard = TEST_MUTEX.lock().unwrap();
    let t = Temp::new();
    let a = request(Role::AudioDesk, 0);
    let lease = Lease::acquire(&t.0, &a).unwrap();
    let b = request(Role::LightingDesk, 1);
    let lighting = Lease::acquire(&t.0, &b).unwrap();
    lease.verify(Counter(1), &a.binding).unwrap();
    assert!(Lease::acquire(&t.0, &request(Role::AudioDesk, 2)).is_err());
    let before = fs::read(t.0.join("registry.json")).unwrap();
    assert!(lighting.forget(Counter(1)).is_err());
    assert_eq!(before, fs::read(t.0.join("registry.json")).unwrap());
    drop(lease);
    let next = Lease::acquire(&t.0, &request(Role::AudioDesk, 2)).unwrap();
    assert_eq!(next.grant().generation, Counter(3));
    assert!(next.verify(Counter(1), &a.binding).is_err());
    assert_eq!(next.forget(Counter(3)).unwrap(), Counter(4));
}
#[test]
fn ambiguity_and_profile_refuse_exact() {
    let _guard = TEST_MUTEX.lock().unwrap();
    let t = Temp::new();
    let mut a = request(Role::AudioDesk, 0);
    a.inventory.controllers[0].serial = None;
    assert!(Lease::acquire(&t.0, &a).is_err());
    a.binding.operator_label = Some("Left".into());
    a.inventory.controllers[0].operator_label = Some("Left".into());
    let lease = Lease::acquire(&t.0, &a).unwrap();
    drop(lease);
    let before = fs::read(t.0.join("registry.json")).unwrap();
    a.expected_generation = Counter(1);
    a.binding.profile = "wrong".into();
    a.inventory.controllers[0].profile = "wrong".into();
    assert!(Lease::acquire(&t.0, &a).is_err());
    assert_eq!(before, fs::read(t.0.join("registry.json")).unwrap());
}
#[test]
fn private_symlink_corruption_and_lost_registry() {
    let _guard = TEST_MUTEX.lock().unwrap();
    let t = Temp::new();
    fs::set_permissions(&t.0, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(Lease::acquire(&t.0, &request(Role::AudioDesk, 0)).is_err());
    fs::set_permissions(&t.0, fs::Permissions::from_mode(0o700)).unwrap();
    std::os::unix::fs::symlink("/dev/null", t.0.join("registry.lock")).unwrap();
    assert!(Lease::acquire(&t.0, &request(Role::AudioDesk, 0)).is_err());
    fs::remove_file(t.0.join("registry.lock")).unwrap();
    let a = request(Role::AudioDesk, 0);
    let lease = Lease::acquire(&t.0, &a).unwrap();
    fs::remove_file(t.0.join("registry.json")).unwrap();
    assert!(lease.verify(Counter(1), &a.binding).is_err());
    drop(lease);
    assert!(Lease::acquire(&t.0, &request(Role::AudioDesk, 0)).is_err());
}
fn spawn(t: &Temp, a: &Acquire) -> (std::process::Child, BufReader<std::process::ChildStdout>) {
    let mut c = Command::new(env!("CARGO_BIN_EXE_gigpies-role-lease"))
        .arg(&t.0)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    writeln!(
        c.stdin.as_mut().unwrap(),
        "{}",
        serde_json::to_string(a).unwrap()
    )
    .unwrap();
    let mut out = BufReader::new(c.stdout.take().unwrap());
    let mut line = String::new();
    out.read_line(&mut line).unwrap();
    assert!(!line.is_empty());
    (c, out)
}
#[test]
fn independent_process_lock_eof_kill_and_handoff() {
    let _guard = TEST_MUTEX.lock().unwrap();
    let t = Temp::new();
    let a = request(Role::AudioDesk, 0);
    let (mut child, _) = spawn(&t, &a);
    assert!(Lease::acquire(&t.0, &request(Role::AudioDesk, 1)).is_err());
    let (mut light, _) = spawn(&t, &request(Role::LightingDesk, 1));
    child.kill().unwrap();
    child.wait().unwrap();
    let (mut replacement, mut output) = spawn(&t, &request(Role::AudioDesk, 2));
    writeln!(replacement.stdin.as_mut().unwrap(),"{}",serde_json::json!({"format":"gigpies-role-command","version":1,"operation":"verify","generation":"3","binding":a.binding})).unwrap();
    let mut line = String::new();
    output.read_line(&mut line).unwrap();
    assert!(line.contains("gigpies-role-verified"));
    drop(light.stdin.take());
    assert!(light.wait().unwrap().success());
    drop(replacement.stdin.take());
    assert!(replacement.wait().unwrap().success());
    let reclaimed = Lease::acquire(&t.0, &request(Role::AudioDesk, 3)).unwrap();
    assert_eq!(reclaimed.grant().generation, Counter(4));
}
#[test]
fn published_request_is_exact() {
    let _guard = TEST_MUTEX.lock().unwrap();
    let a: gigpies::roles::native::Acquire =
        gigpies::roles::native::decode_acquire(include_bytes!("fixtures/gp09/v1/acquire-a.json"))
            .unwrap();
    assert_eq!(a.binding, request(Role::AudioDesk, 0).binding);
    let t = Temp::new();
    assert_eq!(
        serde_json::to_value(Lease::acquire(&t.0, &a).unwrap().grant()).unwrap(),
        serde_json::from_slice::<serde_json::Value>(include_bytes!(
            "fixtures/gp09/v1/grant-a.json"
        ))
        .unwrap()
    );
}

#[test]
fn replaced_lock_inodes_fail_closed() {
    let _guard = TEST_MUTEX.lock().unwrap();
    for registry_lock in [true, false] {
        let t = Temp::new();
        let a = request(Role::AudioDesk, 0);
        let lease = Lease::acquire(&t.0, &a).unwrap();
        let name = if registry_lock {
            "registry.lock".to_string()
        } else {
            fs::read_dir(&t.0)
                .unwrap()
                .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                .find(|n| n.starts_with("device-"))
                .unwrap()
        };
        fs::rename(t.0.join(&name), t.0.join("retained-old-lock")).unwrap();
        let f = fs::File::create(t.0.join(&name)).unwrap();
        f.set_permissions(fs::Permissions::from_mode(0o600))
            .unwrap();
        assert!(lease.verify(Counter(1), &a.binding).is_err());
        assert!(Lease::acquire(&t.0, &request(Role::AudioDesk, 1)).is_err());
    }
}
#[test]
fn hardlinks_bounds_and_ancestor_symlinks_refuse() {
    let _guard = TEST_MUTEX.lock().unwrap();
    let t = Temp::new();
    let a = request(Role::AudioDesk, 0);
    let lease = Lease::acquire(&t.0, &a).unwrap();
    drop(lease);
    fs::hard_link(t.0.join("registry.json"), t.0.join("alias")).unwrap();
    assert!(Lease::acquire(&t.0, &request(Role::AudioDesk, 1)).is_err());
    fs::remove_file(t.0.join("alias")).unwrap();
    fs::write(t.0.join("registry.json"), vec![b' '; 65537]).unwrap();
    assert!(Lease::acquire(&t.0, &request(Role::AudioDesk, 1)).is_err());
    let link = t.0.join("linked");
    std::os::unix::fs::symlink(&t.0, &link).unwrap();
    assert!(Lease::acquire(&link, &a).is_err());
}
#[test]
fn duplicate_edids_require_unique_explicit_labels() {
    let _guard = TEST_MUTEX.lock().unwrap();
    let mut a = request(Role::AudioDesk, 0);
    let mut other = a.inventory.displays[0].clone();
    other.connector = "hdmi-b".into();
    a.inventory.displays.push(other);
    assert!(a.inventory.verify(&a.binding).is_err());
    a.binding.operator_label = Some("Left".into());
    a.inventory.displays[0].operator_label = Some("Left".into());
    assert!(a.inventory.verify(&a.binding).is_ok());
    a.inventory.displays[1].operator_label = Some("Left".into());
    assert!(a.inventory.verify(&a.binding).is_err());
}
#[test]
fn protocol_release_retains_intent_and_stale_forget_does_nothing() {
    let _guard = TEST_MUTEX.lock().unwrap();
    let t = Temp::new();
    let a = request(Role::AudioDesk, 0);
    let (mut c, mut out) = spawn(&t, &a);
    let before = fs::read(t.0.join("registry.json")).unwrap();
    writeln!(c.stdin.as_mut().unwrap(),"{}",serde_json::json!({"format":"gigpies-role-command","version":1,"operation":"release","generation":"1","binding":a.binding})).unwrap();
    let mut reply = String::new();
    out.read_line(&mut reply).unwrap();
    assert!(c.wait().unwrap().success());
    assert_eq!(before, fs::read(t.0.join("registry.json")).unwrap());
    let (mut c, _) = spawn(&t, &request(Role::AudioDesk, 1));
    let before = fs::read(t.0.join("registry.json")).unwrap();
    writeln!(c.stdin.as_mut().unwrap(),"{}",serde_json::json!({"format":"gigpies-role-command","version":1,"operation":"forget","generation":"1","binding":a.binding})).unwrap();
    assert!(!c.wait().unwrap().success());
    assert_eq!(before, fs::read(t.0.join("registry.json")).unwrap());
}
#[test]
fn malformed_protocol_and_idle_deadline_drop_live_authority() {
    let _guard = TEST_MUTEX.lock().unwrap();
    assert!(gigpies::roles::native::decode_acquire(br#"{"format":"x","format":"y"}"#).is_err());
    assert!(gigpies::roles::native::decode_acquire(&vec![b' '; 65537]).is_err());
    let t = Temp::new();
    let (mut c, _) = spawn(&t, &request(Role::AudioDesk, 0));
    let _held_stdin = c.stdin.take().unwrap();
    assert!(!c.wait().unwrap().success()); // 2 s bounded inactivity expiry, no endpoint opened.
    let lease = Lease::acquire(&t.0, &request(Role::AudioDesk, 1)).unwrap();
    assert_eq!(lease.grant().generation, Counter(2));
}
#[test]
fn changed_serial_cannot_reclaim_saved_identity() {
    let _guard = TEST_MUTEX.lock().unwrap();
    let t = Temp::new();
    let a = request(Role::AudioDesk, 0);
    drop(Lease::acquire(&t.0, &a).unwrap());
    let before = fs::read(t.0.join("registry.json")).unwrap();
    let mut a = request(Role::AudioDesk, 1);
    a.inventory.controllers[0].serial = Some("replacement-serial".into());
    assert!(Lease::acquire(&t.0, &a).is_err());
    assert_eq!(before, fs::read(t.0.join("registry.json")).unwrap());
}
#[test]
fn lock_history_capacity_refuses_without_poisoning_existing_lease() {
    let _guard = TEST_MUTEX.lock().unwrap();
    let t = Temp::new();
    let mut generation = 0;
    fn unique(n: u32, g: u64) -> Acquire {
        let mut a = request(Role::AudioDesk, g);
        a.binding.display_connector = format!("hdmi-{n}");
        a.binding.controller = format!("controller-{n}");
        a.inventory.displays[0].connector = a.binding.display_connector.clone();
        a.inventory.controllers[0].identity = a.binding.controller.clone();
        a
    }
    for n in 0..31 {
        let lease = Lease::acquire(&t.0, &unique(n, generation)).unwrap();
        generation = lease.forget(Counter(generation + 1)).unwrap().0;
    }
    let a = unique(31, generation);
    let lease = Lease::acquire(&t.0, &a).unwrap();
    generation += 1;
    let mut next = unique(32, generation);
    next.binding.role = Role::LightingDesk;
    let registry = fs::read(t.0.join("registry.json")).unwrap();
    let map = fs::read(t.0.join("initialized")).unwrap();
    assert!(Lease::acquire(&t.0, &next).is_err());
    assert_eq!(registry, fs::read(t.0.join("registry.json")).unwrap());
    assert_eq!(map, fs::read(t.0.join("initialized")).unwrap());
    lease.verify(Counter(generation), &a.binding).unwrap();
    drop(lease);
    let reclaimed = Lease::acquire(&t.0, &unique(31, generation)).unwrap();
    assert_eq!(reclaimed.grant().generation, Counter(generation + 1));
}
#[test]
fn accepted_e02_saved_intent_adopts_without_steal() {
    let _guard = TEST_MUTEX.lock().unwrap();
    let t = Temp::new();
    let a = request(Role::AudioDesk, 7);
    let registry = gigpies::roles::Registry {
        format: "gigpies-roles".into(),
        version: 1,
        generation: Counter(7),
        bindings: vec![a.binding.clone()],
    };
    fs::write(
        t.0.join("registry.json"),
        serde_json::to_vec(&registry).unwrap(),
    )
    .unwrap();
    fs::set_permissions(t.0.join("registry.json"), fs::Permissions::from_mode(0o600)).unwrap();
    let before = fs::read(t.0.join("registry.json")).unwrap();
    let mut b = a.clone();
    b.binding.role = Role::LightingDesk;
    assert!(Lease::acquire(&t.0, &b).is_err());
    assert_eq!(before, fs::read(t.0.join("registry.json")).unwrap());
    let lease = Lease::acquire(&t.0, &a).unwrap();
    assert_eq!(lease.grant().generation, Counter(8));
}
#[test]
fn exact_independent_process_corpus() {
    let _guard = TEST_MUTEX.lock().unwrap();
    let t = Temp::new();
    let corpus: serde_json::Value =
        serde_json::from_slice(include_bytes!("fixtures/gp09/v1/process-corpus.json")).unwrap();
    let mut children = std::collections::BTreeMap::new();
    for step in corpus["steps"].as_array().unwrap() {
        match step["operation"].as_str().unwrap() {
            "acquire" => {
                let bytes = match step["request"].as_str().unwrap() {
                    "acquire-a.json" => {
                        include_bytes!("fixtures/gp09/v1/acquire-a.json").as_slice()
                    }
                    "acquire-b.json" => {
                        include_bytes!("fixtures/gp09/v1/acquire-b.json").as_slice()
                    }
                    "reclaim-a.json" => {
                        include_bytes!("fixtures/gp09/v1/reclaim-a.json").as_slice()
                    }
                    _ => panic!(),
                };
                let a = gigpies::roles::native::decode_acquire(bytes).unwrap();
                let (c, mut out) = {
                    let mut c = Command::new(env!("CARGO_BIN_EXE_gigpies-role-lease"))
                        .arg(&t.0)
                        .stdin(Stdio::piped())
                        .stdout(Stdio::piped())
                        .stderr(Stdio::null())
                        .spawn()
                        .unwrap();
                    c.stdin.as_mut().unwrap().write_all(bytes).unwrap();
                    let out = BufReader::new(c.stdout.take().unwrap());
                    (c, out)
                };
                let mut line = String::new();
                out.read_line(&mut line).unwrap();
                let grant: serde_json::Value = serde_json::from_str(&line).unwrap();
                assert_eq!(grant["generation"], step["generation"]);
                let role = serde_json::to_value(a.binding.role)
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .to_string();
                children.insert(role, (c, out, a.binding));
            }
            "verify" => {
                let (c, out, b) = children.get_mut(step["role"].as_str().unwrap()).unwrap();
                writeln!(c.stdin.as_mut().unwrap(),"{}",serde_json::json!({"format":"gigpies-role-command","version":1,"operation":"verify","generation":step["generation"],"binding":b})).unwrap();
                let mut line = String::new();
                out.read_line(&mut line).unwrap();
                let reply: serde_json::Value = serde_json::from_str(&line).unwrap();
                assert_eq!(reply["lease"]["generation"], step["generation"]);
                assert_eq!(reply["registry_generation"], step["registry_generation"]);
            }
            "kill" | "eof" => {
                let (mut c, _, _) = children.remove(step["role"].as_str().unwrap()).unwrap();
                if step["operation"] == "kill" {
                    c.kill().unwrap();
                } else {
                    drop(c.stdin.take());
                }
                c.wait().unwrap();
            }
            _ => panic!(),
        }
    }
    assert!(children.is_empty());
}
#[cfg(target_os = "linux")]
#[test]
fn spawning_surface_death_revokes_child_even_with_inherited_pipe() {
    let _guard = TEST_MUTEX.lock().unwrap();
    let t = Temp::new();
    let a = request(Role::AudioDesk, 0);
    let script = "import subprocess,sys,time,json\np=subprocess.Popen([sys.argv[1],sys.argv[2]],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.DEVNULL)\np.stdin.write(sys.stdin.buffer.readline());p.stdin.flush()\nsys.stdout.buffer.write(p.stdout.readline());sys.stdout.buffer.flush()\ntime.sleep(30)\n";
    let mut parent = Command::new("python3")
        .args(["-c", script, env!("CARGO_BIN_EXE_gigpies-role-lease")])
        .arg(&t.0)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    writeln!(
        parent.stdin.as_mut().unwrap(),
        "{}",
        serde_json::to_string(&a).unwrap()
    )
    .unwrap();
    let mut out = BufReader::new(parent.stdout.take().unwrap());
    let mut grant = String::new();
    out.read_line(&mut grant).unwrap();
    assert!(grant.contains("gigpies-role-lease"));
    parent.kill().unwrap();
    parent.wait().unwrap();
    // SIGTERM delivery/reaping is asynchronous; bounded recovery retries only own temporary locks.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    loop {
        match Lease::acquire(&t.0, &request(Role::AudioDesk, 1)) {
            Ok(lease) => {
                assert_eq!(lease.grant().generation, Counter(2));
                break;
            }
            Err(e) if e == "occupied" && std::time::Instant::now() < deadline => {
                std::thread::sleep(std::time::Duration::from_millis(10))
            }
            Err(e) => panic!("{e}"),
        }
    }
}
#[test]
fn partial_device_lock_failure_never_grants_or_updates_intent() {
    let _guard = TEST_MUTEX.lock().unwrap();
    let t = Temp::new();
    let a = request(Role::AudioDesk, 0);
    let lease = Lease::acquire(&t.0, &a).unwrap();
    let before = fs::read(t.0.join("registry.json")).unwrap();
    let mut b = request(Role::LightingDesk, 1);
    b.binding.controller = a.binding.controller.clone();
    b.inventory.controllers[0].identity = a.binding.controller.clone();
    assert!(Lease::acquire(&t.0, &b).is_err());
    assert_eq!(before, fs::read(t.0.join("registry.json")).unwrap());
    drop(lease);
    // Conflicting saved intent fails before device acquisition; the disjoint pairing remains available.
    let lighting = Lease::acquire(&t.0, &request(Role::LightingDesk, 1)).unwrap();
    assert_eq!(lighting.grant().generation, Counter(2));
}
#[test]
fn lost_native_metadata_cannot_reset_descriptor_or_inode_history() {
    let _guard = TEST_MUTEX.lock().unwrap();
    let t = Temp::new();
    let a = request(Role::AudioDesk, 0);
    let lease = Lease::acquire(&t.0, &a).unwrap();
    let before = fs::read(t.0.join("registry.json")).unwrap();
    fs::remove_file(t.0.join("initialized")).unwrap();
    assert!(lease.verify(Counter(1), &a.binding).is_err());
    drop(lease);
    let mut altered = request(Role::AudioDesk, 1);
    altered.inventory.controllers[0].serial = Some("changed-serial".into());
    assert!(Lease::acquire(&t.0, &altered).is_err());
    assert_eq!(before, fs::read(t.0.join("registry.json")).unwrap());
    assert!(Lease::acquire(&t.0, &request(Role::LightingDesk, 1)).is_err());
}
#[test]
fn interrupted_first_native_creation_and_lost_marker_fail_closed() {
    let _guard = TEST_MUTEX.lock().unwrap();
    let t = Temp::new();
    fs::write(t.0.join("native.created"), b"gigpies-role-native:1\n").unwrap();
    fs::set_permissions(
        t.0.join("native.created"),
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    assert!(Lease::acquire(&t.0, &request(Role::AudioDesk, 0)).is_err());
    assert!(!t.0.join("registry.json").exists());
    let t = Temp::new();
    let a = request(Role::AudioDesk, 0);
    let lease = Lease::acquire(&t.0, &a).unwrap();
    fs::remove_file(t.0.join("native.created")).unwrap();
    assert!(lease.verify(Counter(1), &a.binding).is_err());
    drop(lease);
    assert!(Lease::acquire(&t.0, &request(Role::AudioDesk, 1)).is_err());
}
