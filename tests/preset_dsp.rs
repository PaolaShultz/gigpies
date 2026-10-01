use gigpies::automix::{
    config::{self, Channel, EqBand, EqKind, Role},
    dsp::{Biquad, Strip, db},
};
use std::f64::consts::{FRAC_1_SQRT_2, PI};

fn response(e: &EqBand, hz: f64, rate: u32) -> f64 {
    let mut f = Biquad::equalizer(e, rate);
    let mut power = 0.;
    for i in 0..rate {
        let y = f.tick((2. * PI * hz * i as f64 / rate as f64).sin());
        assert!(y.is_finite());
        if i >= rate / 2 {
            power += y * y;
        }
    }
    db((power / (rate / 2) as f64).sqrt()) + 3.01029995664
}
#[test]
fn shelves_have_correct_endpoints_midpoint_and_inverse_at_native_rates() {
    for rate in [44100, 48000, 96000] {
        for kind in [EqKind::LowShelf, EqKind::HighShelf] {
            let mut e = EqBand {
                kind,
                hz: 1000.,
                q: FRAC_1_SQRT_2,
                db: 6.,
            };
            assert!((response(&e, 1000., rate) - 3.).abs() < 0.01);
            let (low, high) = (response(&e, 20., rate), response(&e, 18000., rate));
            if kind == EqKind::LowShelf {
                assert!((low - 6.).abs() < 0.01 && high.abs() < 0.01);
            } else {
                assert!(low.abs() < 0.01 && (high - 6.).abs() < 0.01);
            }
            e.db = -6.;
            assert!((response(&e, 1000., rate) + 3.).abs() < 0.01);
            e.db = 0.;
            assert!(response(&e, 1000., rate).abs() < 0.01);
        }
    }
}
#[test]
fn broad_bells_legacy_json_and_unsupported_shapes_are_explicit() {
    let e: EqBand = serde_json::from_str(r#"{"hz":1500,"q":0.1,"db":3}"#).unwrap();
    assert_eq!(e.kind, EqKind::Bell);
    assert!((response(&e, 1500., 48000) - 3.).abs() < 0.01);
    let mut s = config::example();
    s.channels[0].eq = vec![e];
    s.validate().unwrap();
    s.channels[0].eq[0].kind = EqKind::LowShelf;
    assert!(s.validate().is_err());
    assert!(
        serde_json::from_str::<EqBand>(r#"{"kind":"console_shelf","hz":100,"q":1,"db":2}"#)
            .is_err()
    );
}
#[test]
fn slow_attack_preserves_transients_and_links_stereo_without_makeup_twice() {
    let mut ch = Channel::preset("synthetic.wav", Role::Other, "x", 0., 0.);
    ch.hpf_hz = 0.;
    ch.compressor.threshold_db = -24.;
    ch.compressor.ratio = 4.;
    ch.compressor.knee_db = 0.;
    ch.compressor.attack_ms = 30.;
    ch.compressor.makeup_db = 0.;
    let mut strip = Strip::new(&ch, 48000);
    let first = strip.tick([1., 0.25]);
    assert!(first[0] > 0.99);
    for _ in 0..4800 {
        let y = strip.tick([1., 0.25]);
        assert!((y[0] * 0.25 - y[1]).abs() < 1e-12);
    }
    assert!(strip.reduction_db() > 17.);
    for _ in 0..48000 {
        assert_eq!(strip.tick([0., 0.]), [0., 0.]);
    }
    ch.compressor.ratio = 1.;
    ch.compressor.makeup_db = 6.;
    let y = Strip::new(&ch, 48000).tick([0.1, 0.025]);
    assert!((db(y[0] / 0.1) - 6.).abs() < 1e-9);
}
