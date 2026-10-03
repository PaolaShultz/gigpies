//! Independent neutral reference: no production routing or export-gain helpers.
use gigpies::automix::{config::*, run};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "gigpies-sum-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&p).unwrap();
        Self(p)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn reference(x: [f64; 2], stereo: bool, pan: f64, level: f64) -> [f64; 2] {
    let g = 10_f64.powf(level / 20.);
    if stereo {
        [
            x[0] * (1. - pan.max(0.)).sqrt() * g,
            x[1] * (1. + pan.min(0.)).sqrt() * g,
        ]
    } else {
        let a = std::f64::consts::PI * (pan + 1.) / 4.;
        [x[0] * a.cos() * g, x[0] * a.sin() * g]
    }
}
fn compensated(xs: impl Iterator<Item = f64>) -> f64 {
    let (mut sum, mut c) = (0_f64, 0_f64);
    for x in xs {
        let t = sum + x;
        c += if sum.abs() >= x.abs() {
            (sum - t) + x
        } else {
            (x - t) + sum
        };
        sum = t;
    }
    sum + c
}
fn fixture(
    rate: u32,
    count: usize,
    reverse: bool,
    zero: bool,
    block: usize,
) -> (Vec<f32>, Vec<i32>, Vec<[f64; 2]>) {
    let tmp = Scratch::new();
    let frames = 137;
    let mut s = example();
    s.sample_rate = rate;
    s.block_frames = block;
    s.prepared = true;
    s.master_db = 0.;
    s.master_hpf_hz = 0.;
    s.output_mode = OutputMode::Unmatched;
    s.ceiling_db = -0.01;
    s.channels.clear();
    s.groups.clear();
    s.effects = None;
    s.calibration.initial_trim_db = 0.;
    let mut terms = vec![Vec::new(); frames];
    for i in 0..count + usize::from(zero) {
        let stereo = i % 3 == 0;
        let pan = [-1., -0.37, 0., 0.62, 1.][i % 5];
        let fader = if i % 7 == 0 { 12. } else { 0. };
        let name = format!("{i}.wav");
        let mut w = hound::WavWriter::create(
            tmp.0.join(&name),
            hound::WavSpec {
                channels: if stereo { 2 } else { 1 },
                sample_rate: rate,
                bits_per_sample: 32,
                sample_format: hound::SampleFormat::Float,
            },
        )
        .unwrap();
        for (t, row) in terms.iter_mut().enumerate() {
            let amplitude = [0., 1e-12, 0.125, 2., 32., -32., 0.5][t % 7];
            let x = if i == count {
                [0.; 2]
            } else {
                [
                    ((t * 17 + i * 13) % 31) as f64 / 31. * amplitude,
                    -amplitude * 0.25,
                ]
            }
            .map(|v| v as f32 as f64);
            w.write_sample(x[0] as f32).unwrap();
            if stereo {
                w.write_sample(x[1] as f32).unwrap();
            }
            row.push(reference(x, stereo, pan, fader));
        }
        w.finalize().unwrap();
        let group = format!("g{i}");
        let mut ch = Channel::preset(&name, Role::Other, &group, fader, pan);
        ch.hpf_hz = 0.;
        ch.eq.clear();
        ch.compressor.ratio = 1.;
        ch.compressor.makeup_db = 0.;
        s.channels.push(ch);
        s.groups.push(Group {
            name: group,
            reference: i,
            trim_db: 0.,
        });
    }
    if reverse {
        s.channels.reverse();
        for g in &mut s.groups {
            g.reference = s.channels.iter().position(|c| c.group == g.name).unwrap();
        }
    }
    run(s, &tmp.0, &tmp.0.join("render"), None).unwrap();
    let bus: Vec<f32> = hound::WavReader::open(tmp.0.join("render/processed-unity-float.wav"))
        .unwrap()
        .samples()
        .map(Result::unwrap)
        .collect();
    let pcm: Vec<i32> = hound::WavReader::open(tmp.0.join("render/processed.wav"))
        .unwrap()
        .samples()
        .map(Result::unwrap)
        .collect();
    let expected: Vec<[f64; 2]> = terms
        .iter()
        .map(|v| std::array::from_fn(|c| compensated(v.iter().map(|x| x[c]))))
        .collect();
    for (t, row) in terms.iter().enumerate() {
        for c in 0..2 {
            let k = (8 * row.len() + 16) as f64;
            let gamma = k * f64::EPSILON / (1. - k * f64::EPSILON);
            let bound = gamma * row.iter().map(|x| x[c].abs()).sum::<f64>()
                + expected[t][c].abs() * f32::EPSILON as f64 * 0.5
                + f32::MIN_POSITIVE as f64;
            assert!(
                (bus[2 * t + c] as f64 - expected[t][c]).abs() <= bound,
                "rate={rate} n={count} frame={t} channel={c}"
            );
        }
    }
    // Independent peak gain uses the f64 bus, as required by the legacy contract.
    let peak = expected
        .iter()
        .flatten()
        .map(|x| x.abs())
        .fold(0_f64, f64::max);
    let scale = if peak == 0. {
        1.
    } else {
        10_f64.powf(-0.01 / 20.) / peak
    };
    for (x, y) in bus.iter().zip(&pcm) {
        assert_eq!((*x as f64 * scale * 8388608.).round() as i32, *y);
    }
    (bus, pcm, expected)
}
#[test]
fn neutral_multichannel_reference_and_boundaries() {
    for rate in [8000, 44100, 48000, 96000, 192000] {
        for n in [1, 2, 16, 40, 64] {
            fixture(rate, n, false, false, 32);
        }
    }
}
#[test]
fn block_sizes_zero_channel_and_reordering_preserve_neutral_sum() {
    let a = fixture(48000, 16, false, false, 32);
    let b = fixture(48000, 16, false, true, 8192);
    let c = fixture(48000, 16, true, false, 67);
    assert_eq!(a.0, b.0);
    assert_eq!(a.1, b.1);
    assert_eq!(a.0, c.0);
    assert_eq!(a.1, c.1);
}
#[test]
fn documented_pan_power_and_stereo_interpretation() {
    for pan in [-1., -0.5, 0., 0.5, 1.] {
        let x = reference([1.; 2], false, pan, 0.);
        assert!((x[0] * x[0] + x[1] * x[1] - 1.).abs() < 1e-15);
    }
    assert_eq!(reference([1., -1.], true, 0., 0.), [1., -1.]);
    assert!((reference([1.; 2], false, 0., 0.)[0] - 0.5_f64.sqrt()).abs() < 1e-15);
}

#[test]
fn cancellation_peak_cost_and_master_gain_cancellation_have_distinct_mechanisms() {
    let tmp = Scratch::new();
    let mut base = example();
    base.channels.truncate(2);
    base.groups.truncate(2);
    base.prepared = true;
    base.master_db = 0.;
    base.master_hpf_hz = 0.;
    base.calibration.initial_trim_db = 0.;
    base.output_mode = OutputMode::Unmatched;
    base.ceiling_db = -0.01;
    for c in &mut base.channels {
        c.hpf_hz = 0.;
        c.eq.clear();
        c.fader_db = 0.;
        c.pan = 0.;
        c.compressor.ratio = 1.;
        c.compressor.makeup_db = 0.;
    }
    let write = |name: &std::path::Path, signal: &dyn Fn(usize) -> f32| {
        let mut w = hound::WavWriter::create(
            name,
            hound::WavSpec {
                channels: 2,
                sample_rate: 44100,
                bits_per_sample: 32,
                sample_format: hound::SampleFormat::Float,
            },
        )
        .unwrap();
        for t in 0..512 {
            for _ in 0..2 {
                w.write_sample(signal(t)).unwrap();
            }
        }
        w.finalize().unwrap();
    };
    let read = |path: PathBuf| -> Vec<i32> {
        hound::WavReader::open(path)
            .unwrap()
            .samples()
            .map(Result::unwrap)
            .collect()
    };
    write(&tmp.0.join(&base.channels[0].file), &|t| {
        if t == 32 { 1. } else { 0. }
    });
    write(&tmp.0.join(&base.channels[1].file), &|_| 0.25);
    run(base.clone(), &tmp.0, &tmp.0.join("baseline"), None).unwrap();
    let original = read(tmp.0.join("baseline/processed.wav"));
    let mut boosted = base.clone();
    boosted.master_db = 20. * 2_f64.log10();
    run(boosted.clone(), &tmp.0, &tmp.0.join("master"), None).unwrap();
    assert_eq!(original, read(tmp.0.join("master/processed.wav")));
    // FX dynamics break the invariance: gain changes its attack/release envelope.
    let mut fx_session = example();
    gigpies::automix::effects::add_pass(&mut fx_session).unwrap();
    let mut fx = fx_session.effects.unwrap();
    fx.buses.clear();
    fx.exciter_amount = 0.;
    fx.master_eq.clear();
    fx.tail_seconds = 0.;
    fx.maximizer_drive_db = 0.;
    fx.maximizer_threshold_db = -6.;
    let mut limited = base.clone();
    limited.effects = Some(fx.clone());
    boosted.effects = Some(fx);
    run(limited, &tmp.0, &tmp.0.join("limited"), None).unwrap();
    run(boosted, &tmp.0, &tmp.0.join("limited-master"), None).unwrap();
    assert_ne!(
        read(tmp.0.join("limited/processed.wav")),
        read(tmp.0.join("limited-master/processed.wav"))
    );
    write(&tmp.0.join(&base.channels[0].file), &|t| {
        if t == 32 { 2. } else { 0. }
    });
    run(base.clone(), &tmp.0, &tmp.0.join("peak"), None).unwrap();
    let peak = read(tmp.0.join("peak/processed.wav"));
    assert!((peak[400] as f64 / original[400] as f64 - 1.25 / 2.25).abs() < 1e-6);
    // Identical/opposite input null and severe cancellation through actual renderer.
    write(&tmp.0.join(&base.channels[0].file), &|t| {
        if t % 2 == 0 { 16. } else { 1e-12 }
    });
    write(&tmp.0.join(&base.channels[1].file), &|t| {
        if t % 2 == 0 { -16. } else { -1e-12 }
    });
    run(base, &tmp.0, &tmp.0.join("null"), None).unwrap();
    assert!(
        read(tmp.0.join("null/processed.wav"))
            .iter()
            .all(|v| *v == 0)
    );
}
