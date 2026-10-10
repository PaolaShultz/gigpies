#![cfg(feature = "hardware-host")]
use gigpies::{
    fx_wire::{Channel, Configuration},
    host::{brain_fx::BrainFx, fx_v2::Owner},
};
fn library() -> std::path::PathBuf {
    std::env::var_os("GP21_FX_LIBRARY")
        .expect("explicit hash-pinned owner artifact required")
        .into()
}
fn config() -> Configuration {
    Configuration {
        channels: [
            Channel {
                delay_ms: 1.,
                feedback: 0.,
                damping: 0.,
                wet_gain: 0.25,
                bypass: false,
            },
            Channel {
                delay_ms: 20.,
                feedback: 0.25,
                damping: 0.35,
                wet_gain: 0.5,
                bypass: false,
            },
        ],
    }
}
#[test]
#[ignore = "requires explicitly pinned actual SHR FX delay v2 library"]
fn gp21_actual_owner_independent_channel_transition_and_exact_frames() {
    let mut owner = Owner::load(&library(), 48)
        .unwrap()
        .expect("source-timeline delay v2 ABI");
    let initial = owner.status().unwrap();
    assert_eq!(initial.applied_generation, 0);
    owner.prepare(&config(), 0).unwrap();
    assert_eq!(owner.commit(48), -6);
    assert_eq!(owner.status().unwrap().applied_generation, 0);
    assert_eq!(owner.commit(0), 0);
    owner.retire();
    let input = [0.1; 96];
    let mut output = [0.; 96];
    for frame in (0..960).step_by(48) {
        assert_eq!(owner.process(&input, &mut output, frame), 0);
    }
    let s = owner.status().unwrap();
    assert_eq!(s.applied_source_frame, 0);
    assert_eq!(s.settled_source_frame, 960);
    assert_eq!(s.applied_generation, 1);
    assert_eq!(s.settled_generation, 1);
    assert_eq!(s.remaining_frames, [0, 0]);
    assert!(output.chunks_exact(2).all(|p| p[0] > 0. && p[1] == 0.));
    assert_eq!(owner.panic(1), 0);
    assert_eq!(owner.status().unwrap().applied_generation, 1);
    assert_eq!(owner.process(&[0.; 96], &mut output, 960), 0);
    assert!(output.chunks_exact(2).all(|p| p[0] == 0. && p[1] > 0.));
    let before = owner.status().unwrap();
    assert_eq!(owner.process(&input, &mut output, 960), -6);
    assert_eq!(
        owner.status().unwrap().next_source_frame,
        before.next_source_frame
    );
}
#[test]
#[ignore = "requires explicitly pinned actual SHR FX delay v2 library"]
fn gp21_actual_brain_opt_in_preserves_v1_and_source_order() {
    let mut fixed = BrainFx::prepare(&library(), 2, 48, 9).unwrap();
    assert_eq!(fixed.intentional_delay_frames(), 960);
    let mut fx = BrainFx::prepare(&library(), 2, 48, 9).unwrap();
    assert!(fx.enable_configured(&library()).unwrap());
    let input = [0.125; 96];
    let mut a = [0.; 96];
    let mut b = [0.; 96];
    for frame in (0..4800).step_by(48) {
        fixed.process(9, frame, &input, &mut a).unwrap();
        fx.process(9, frame, &input, &mut b).unwrap();
        assert_eq!(a, b);
    }
    assert!(fx.process(9, 0, &input, &mut b).is_err());
    assert!(fx.process(10, 4800, &input, &mut b).is_err());
    let resets = fx.resets;
    fx.process(9, 9600, &[0.; 96], &mut b).unwrap();
    assert_eq!(fx.resets, resets + 1);
    assert_eq!(b, [0.; 96]);
}
#[test]
#[ignore = "requires explicitly pinned old SHR FX v1 library"]
fn gp21_old_library_remains_read_only_media() {
    assert!(Owner::load(&library(), 48).unwrap().is_none());
    let mut old = BrainFx::prepare(&library(), 2, 48, 9).unwrap();
    assert!(!old.enable_configured(&library()).unwrap());
    let mut out = [0.; 96];
    old.process(9, 0, &[0.1; 96], &mut out).unwrap();
    assert_eq!(old.intentional_delay_frames(), 960);
}

#[test]
#[ignore = "requires explicitly pinned generic SHR FX v2 library without the delay extension"]
fn gp21_generic_v2_library_is_not_called_as_source_timeline_delay() {
    let path = library();
    let generic = unsafe { libloading::Library::new(&path) }.unwrap();
    // This symbol uniquely identifies the generic prepared-palette API.
    // Resolve only, never invoke a function with a foreign signature.
    assert!(unsafe { generic.get::<unsafe extern "C" fn()>(b"shr_fx_v2_publish\0") }.is_ok());
    // A generic v2 owner uses incompatible function signatures. The loader must
    // refuse its optional controls before calling any of those symbols, while
    // retaining the independently supported fixed v1 media path.
    assert!(Owner::load(&library(), 48).unwrap().is_none());
    let mut media = BrainFx::prepare(&library(), 2, 48, 9).unwrap();
    assert!(!media.enable_configured(&library()).unwrap());
    let mut output = [0.; 96];
    media.process(9, 0, &[0.1; 96], &mut output).unwrap();
    assert!(output.iter().all(|sample| sample.is_finite()));
}
