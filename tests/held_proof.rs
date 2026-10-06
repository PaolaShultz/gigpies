use gigpies::{
    control_model::Scope,
    held_proof::{self, Configuration, Request},
    show::Counter,
};
const SHOW: &str = "11111111-1111-4111-8111-111111111111";
fn digest(inputs: u32, destinations: &[usize], gain: i32, mute: bool, foh: bool) -> String {
    held_proof::config_digest(Configuration {
        show_id: SHOW,
        epoch: 1,
        capability: 1,
        map: 1,
        dimensions: [inputs, 5, 0, inputs, 7, 48000],
        destinations: held_proof::destination_hash(destinations).unwrap(),
        gain_cdb: gain,
        mute,
        foh,
    })
}
#[test]
fn held_proof_canonical_membership_and_scalar_binding() {
    assert_eq!(
        digest(16, &[4, 0], -1200, false, false),
        digest(16, &[0, 4], -1200, false, false)
    );
    let base = digest(16, &[0, 4], -1200, false, false);
    // Independent Python struct.pack BE/hashlib golden encoding, no engine imports.
    assert_eq!(
        base,
        "ce543b561865a961906f67241d080ca858f6dc3d58ce58615b498ca210552cc6"
    );
    for other in [
        digest(16, &[0, 3], -1200, false, false),
        digest(16, &[0, 4], -1100, false, false),
        digest(16, &[0, 4], -1200, true, false),
        digest(16, &[0, 4], -1200, false, true),
        digest(32, &[0, 4], -1200, false, false),
    ] {
        assert_ne!(base, other);
    }
    assert!(held_proof::destination_hash(&[1, 1]).is_err());
}
fn request(writer: String, session: u64, lease: Counter, inputs: u32) -> Request {
    Request {
        contract: held_proof::CONTRACT.into(),
        version: 1,
        kind: "held_proof".into(),
        query_id: Counter(1),
        show_id: SHOW.into(),
        module: "audio".into(),
        epoch: Counter(1),
        authenticated_session: Counter(session),
        writer,
        capability_generation: Counter(1),
        map_generation: Counter(1),
        scope: Scope::TalkbackDestinations,
        lease,
        expected_config_digest: digest(inputs, &[], -1200, true, false),
    }
}
#[test]
fn held_proof_strict_envelope_and_byte_bound() {
    let r = request("desk".into(), 1, Counter(1), 16);
    let bytes = serde_json::to_vec(&r).unwrap();
    assert!(Request::decode(&bytes).is_ok());
    let mut value = serde_json::to_value(&r).unwrap();
    value["unknown"] = serde_json::json!(1);
    assert!(Request::decode(&serde_json::to_vec(&value).unwrap()).is_err());
    value = serde_json::to_value(&r).unwrap();
    value["query_id"] = serde_json::json!(1);
    assert!(Request::decode(&serde_json::to_vec(&value).unwrap()).is_err());
    assert!(Request::decode(&vec![b' '; held_proof::MAX_BYTES + 1]).is_err());
}

#[test]
fn held_proof_null_fields_are_mandatory_and_not_omittable() {
    let r = request("desk".into(), 1, Counter(1), 16);
    let witness = held_proof::Witness {
        revision: Counter(0),
        source_frame: Counter(0),
        config_digest: r.expected_config_digest.clone(),
        lease_remaining_ms: 1000,
        brain: held_proof::Brain {
            selection_generation: Counter(1),
            hold_generation_counter: Counter(0),
            held_generation: None,
            hold_deadline_ms: None,
            source: gigpies::brain_control::MonitorSource::None,
            monitor_armed: false,
            monitor_mute: true,
            monitor_dim: false,
            talkback_mute: true,
            talkback_foh: false,
            monitor_path_ready: false,
            talkback_path_ready: false,
        },
        foh_authorized: false,
        media_authorized: false,
        heartbeat_ms: 50,
        deadman_ms: 150,
        fade_frames: 240,
    };
    let value = serde_json::to_value(held_proof::Reply::new(r.clone(), Ok(witness))).unwrap();
    assert!(held_proof::Reply::decode(&serde_json::to_vec(&value).unwrap()).is_ok());
    for field in ["reason", "witness"] {
        let mut missing = value.clone();
        missing.as_object_mut().unwrap().remove(field);
        assert!(held_proof::Reply::decode(&serde_json::to_vec(&missing).unwrap()).is_err());
    }
    for field in ["held_generation", "hold_deadline_ms"] {
        let mut missing = value.clone();
        missing["witness"]["brain"]
            .as_object_mut()
            .unwrap()
            .remove(field);
        assert!(held_proof::Reply::decode(&serde_json::to_vec(&missing).unwrap()).is_err());
    }
    let mut refusal =
        serde_json::to_value(held_proof::Reply::new(r, Err(held_proof::Reason::Lease))).unwrap();
    assert!(held_proof::Reply::decode(&serde_json::to_vec(&refusal).unwrap()).is_ok());
    refusal.as_object_mut().unwrap().remove("witness");
    assert!(held_proof::Reply::decode(&serde_json::to_vec(&refusal).unwrap()).is_err());
}

mod render_guard {
    use std::{
        alloc::{GlobalAlloc, Layout, System},
        cell::Cell,
    };
    thread_local! {static WATCH:Cell<bool>=const{Cell::new(false)};static CHANGES:Cell<u64>=const{Cell::new(0)};}
    struct Guard;
    unsafe impl GlobalAlloc for Guard {
        unsafe fn alloc(&self, l: Layout) -> *mut u8 {
            WATCH.with(|w| {
                if w.get() {
                    CHANGES.with(|n| n.set(n.get() + 1))
                }
            });
            unsafe { System.alloc(l) }
        }
        unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
            WATCH.with(|w| {
                if w.get() {
                    CHANGES.with(|n| n.set(n.get() + 1))
                }
            });
            unsafe { System.dealloc(p, l) }
        }
        unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
            WATCH.with(|w| {
                if w.get() {
                    CHANGES.with(|v| v.set(v.get() + 1))
                }
            });
            unsafe { System.realloc(p, l, n) }
        }
    }
    #[global_allocator]
    static ALLOCATOR: Guard = Guard;
    #[test]
    fn held_proof_prepared_destinations_keep_borrowed_render_allocation_and_free_guard() {
        let destinations = [0, 4];
        let _prepared = gigpies::held_proof::destination_hash(&destinations).unwrap();
        let safety = [1.; 48];
        let samples = [0.25; 48];
        let mut output = [0.; 48 * 7];
        let mut envelope = 0.;
        let view = gigpies::brain_control::TalkbackRender {
            first_frame: 0,
            deadline_frame: Some(7200),
            monitors: &destinations,
            foh: false,
            gain: 1.,
            safety: &safety,
        };
        CHANGES.with(|n| n.set(0));
        WATCH.with(|w| w.set(true));
        let result = gigpies::brain_control::render_talkback_block(
            &view,
            &mut envelope,
            Some(&samples),
            &mut output,
            7,
        );
        WATCH.with(|w| w.set(false));
        assert!(result.is_ok());
        assert_eq!(CHANGES.with(Cell::get), 0);
    }
}
