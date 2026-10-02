use gigpies::automix::{
    self,
    ambience::{self, Family, Group, Input, Policy, Style},
    config::{self, Role},
    effects::{Effect, ReverbKind},
    preservation, unity,
};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture {
    root: PathBuf,
    s: config::Session,
    p: Policy,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
fn fixture() -> Fixture {
    let root = std::env::temp_dir().join(format!(
        "gigpies-ambience-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&root).unwrap();
    let mut s = config::example();
    s.channels.truncate(2);
    s.sample_rate = 8000;
    s.channels[0].role = Role::LeadVocal;
    s.channels[1].role = Role::Other;
    for (i, c) in s.channels.iter_mut().enumerate() {
        c.file = format!("{i}.wav").into();
        let mut w = hound::WavWriter::create(
            root.join(&c.file),
            hound::WavSpec {
                channels: 2,
                sample_rate: 8000,
                bits_per_sample: 24,
                sample_format: hound::SampleFormat::Int,
            },
        )
        .unwrap();
        for k in 0..8000 * 8 {
            let t = k as f64 / 8000.;
            let env = if t % 0.8 < 0.45 { 1. } else { 0. };
            let x = 0.08
                * env
                * ((std::f64::consts::TAU * (220. + i as f64 * 113.) * t).sin()
                    + 0.4 * (std::f64::consts::TAU * 771. * t).sin());
            for v in [x, x * if i == 0 { 0.65 } else { -0.3 }] {
                w.write_sample((v * 8388608.) as i32).unwrap();
            }
        }
        w.finalize().unwrap();
    }
    unity::prepare(&mut s).unwrap();
    s = preservation::source_settings(&s).unwrap();
    s.channels[1].fader_db = -4.;
    s.channels[1].pan = -0.3;
    let p = Policy {
        style: Style::Acoustic,
        style_basis: "Explicit test musical intent".into(),
        tempo_bpm: None,
        amount: 1.,
        groups: vec![
            Group {
                name: "vocal".into(),
                family: Family::LeadVocal,
                inputs: vec![Input {
                    channel: 0,
                    file: "0.wav".into(),
                }],
                identity_basis: "Fixture vocal".into(),
                existing_space_reported: false,
            },
            Group {
                name: "unknown".into(),
                family: Family::Unknown,
                inputs: vec![Input {
                    channel: 1,
                    file: "1.wav".into(),
                }],
                identity_basis: "Fixture unknown source; no classification".into(),
                existing_space_reported: false,
            },
        ],
        training: vec![[0., 2.], [4., 6.]],
        held_out: vec![[2., 4.], [6., 8.]],
    };
    Fixture { root, s, p }
}
#[test]
fn expert_plan_uses_actual_fx_and_preserves_direct_chain() {
    let f = fixture();
    let out = f.root.join("plan");
    ambience::run(f.s.clone(), &f.root, &out, f.p.clone()).unwrap();
    let selected: config::Session =
        serde_json::from_reader(std::fs::File::open(out.join("settings.json")).unwrap()).unwrap();
    assert_eq!(
        serde_json::to_value(&selected.channels).unwrap(),
        serde_json::to_value(&f.s.channels).unwrap()
    );
    assert_eq!(selected.master_hpf_hz, f.s.master_hpf_hz);
    let fx = selected.effects.as_ref().unwrap();
    assert_eq!(fx.exciter_amount, 0.);
    assert!(fx.master_eq.is_empty());
    assert_eq!(fx.maximizer_drive_db, 0.);
    assert_eq!(fx.buses.len(), 1);
    assert!(
        matches!(fx.buses[0].effect,Effect::Reverb(p) if p.kind==ReverbKind::Hall && p.predelay_ms>=30.)
    );
    assert_eq!(fx.buses[0].sends[0].channel, 0);
    let evidence = ambience::measure(&f.s, &f.root, &f.p, Some(fx), &[[0., 8.]]).unwrap();
    let render = f.root.join("render");
    automix::run(selected, &f.root, &render, None).unwrap();
    let mut reader = hound::WavReader::open(render.join("processed-unity-float.wav")).unwrap();
    let samples = reader
        .samples::<f32>()
        .map(Result::unwrap)
        .collect::<Vec<_>>();
    for frame in &evidence {
        let a = (frame.start * 8000.).round() as usize * 2;
        let b = (frame.end * 8000.).round() as usize * 2;
        let power = samples[a..b]
            .iter()
            .map(|&x| f64::from(x).powi(2))
            .sum::<f64>()
            / (b - a) as f64;
        assert!((power - frame.mixed_power).abs() < 1e-9);
    }
    let report: serde_json::Value =
        serde_json::from_reader(std::fs::File::open(render.join("measurements.json")).unwrap())
            .unwrap();
    assert_eq!(report["effects"]["maximizer_affected_frames"], 0);
    assert!(
        report["effects"]["return_meters_before_master"][0]["peak_dbfs"]
            .as_f64()
            .unwrap()
            > -80.
    );
    assert!((report["processed_export_peak_dbfs"].as_f64().unwrap() + 0.01).abs() < 1e-8);
    assert!(ambience::run(f.s.clone(), &f.root, &out, f.p.clone()).is_err());
}
#[test]
fn zero_amount_and_unknown_identity_can_preserve_every_setting() {
    let mut f = fixture();
    f.p.amount = 0.;
    let out = f.root.join("zero");
    ambience::run(f.s.clone(), &f.root, &out, f.p.clone()).unwrap();
    let after: serde_json::Value =
        serde_json::from_reader(std::fs::File::open(out.join("settings.json")).unwrap()).unwrap();
    assert_eq!(after, serde_json::to_value(&f.s).unwrap());
    f.p.amount = 1.;
    for g in &mut f.p.groups {
        g.family = Family::Unknown;
    }
    let obs = ambience::measure(&f.s, &f.root, &f.p, None, &f.p.training).unwrap();
    let (fx, _, decisions) = ambience::propose(&f.s, &f.p, &obs).unwrap();
    assert!(fx.buses.is_empty());
    assert!(decisions.iter().all(|d| d.bus_names.is_empty()));
}
#[test]
fn style_changes_artistic_type_without_using_held_out_audio() {
    let mut f = fixture();
    f.p.style = Style::Metal;
    f.p.tempo_bpm = Some(100.);
    let obs = ambience::measure(&f.s, &f.root, &f.p, None, &f.p.training).unwrap();
    let (metal, _, d) = ambience::propose(&f.s, &f.p, &obs).unwrap();
    assert!(matches!(metal.buses[0].effect,Effect::Reverb(p) if p.kind==ReverbKind::Plate));
    assert!(
        matches!(metal.buses[1].effect,Effect::Delay(p) if (p.left_ms-300.).abs()<1e-5 && (p.right_ms-450.).abs()<1e-5)
    );
    assert!(
        (d[0].desired_decay_seconds.unwrap() - d[0].measured_decay_seconds.unwrap()).abs() < 0.15
    );
    // A radically different held-out payload cannot enter the frozen source observations.
    let path = f.root.join("0.wav");
    let mut r = hound::WavReader::open(&path).unwrap();
    let spec = r.spec();
    let mut samples = r.samples::<i32>().map(Result::unwrap).collect::<Vec<_>>();
    drop(r);
    samples[2 * 8000 * 2..4 * 8000 * 2].fill(0);
    let mut w = hound::WavWriter::create(&path, spec).unwrap();
    for v in samples {
        w.write_sample(v).unwrap();
    }
    w.finalize().unwrap();
    // Training-only selection must ignore the changed held-out source interval.
    let changed = ambience::measure(&f.s, &f.root, &f.p, None, &f.p.training).unwrap();
    assert_eq!(
        serde_json::to_value(&obs).unwrap(),
        serde_json::to_value(&changed).unwrap()
    );
    let (again, _, _) = ambience::propose(&f.s, &f.p, &changed).unwrap();
    assert_eq!(
        serde_json::to_value(metal).unwrap(),
        serde_json::to_value(again).unwrap()
    );
}
#[test]
fn routing_fx_returns_and_invalid_spans_are_rejected_before_outputs() {
    let f = fixture();
    for case in 0..5 {
        let mut p = f.p.clone();
        match case {
            0 => p.groups[0].family = Family::SuppliedFx,
            1 => p.groups[1].inputs[0].channel = 0,
            2 => p.held_out[0] = [1., 3.],
            3 => p.groups[0].inputs[0].file = "wrong.wav".into(),
            _ => p.amount = f64::NAN,
        };
        let out = f.root.join(format!("bad-{case}"));
        assert!(ambience::run(f.s.clone(), &f.root, &out, p).is_err());
        assert!(!out.exists());
    }
}
#[test]
fn overwhelming_returns_fail_ensemble_protection() {
    let f = fixture();
    let obs = ambience::measure(&f.s, &f.root, &f.p, None, &f.p.training).unwrap();
    let (mut fx, _, _) = ambience::propose(&f.s, &f.p, &obs).unwrap();
    fx.buses[0].return_db = 18.;
    let measured = ambience::measure(&f.s, &f.root, &f.p, Some(&fx), &f.p.held_out).unwrap();
    assert!(
        ambience::checks(&measured, &f.p.held_out)
            .iter()
            .any(|x| !x.passed)
    );
}
