use gigpies::automix::bleed::{self, Correlation, Feature};
fn feature(t: f64, quiet: bool, spill: bool) -> Feature {
    let bands = if spill {
        [-25., -39., -55., -60., -60.]
    } else if quiet {
        [-50., -35., -50., -55., -60.]
    } else {
        [-35., -20., -35., -40., -45.]
    };
    let level = if spill {
        -35.
    } else if quiet {
        -40.
    } else {
        -18.
    };
    Feature {
        seconds: t,
        held_out: t >= 12.,
        ambiguous_onset: false,
        raw_attack_db: level,
        kick_distance_seconds: if spill { 0.01 } else { 0.2 },
        phase_bands: [[bands; 5]; 4],
        phase_broad: [[level; 5]; 4],
        low_band_kick_similarity: Correlation {
            absolute: 0.8,
            signed: 0.8,
            lag_ms: 3.,
            channel_pair: [0, 0],
        },
        low_band_overhead_similarity: Correlation {
            absolute: 0.1,
            signed: 0.1,
            lag_ms: 0.,
            channel_pair: [0, 0],
        },
        high_band_overhead_envelope_correlation: Some(0.5),
        spectrum_distance_db: None,
        classification: "unclassified_protected".into(),
    }
}
fn rows() -> Vec<Feature> {
    let mut v = vec![];
    for split in [0., 12.] {
        for i in 0..10 {
            let t = split + i as f64 + 0.1;
            v.push(feature(t, false, false));
            v.push(feature(t + 0.4, false, true));
            v.push(feature(t + 0.7, true, false));
        }
    }
    v
}
#[test]
fn quiet_snare_shapes_compounds_and_unconfirmed_spill_are_protected() {
    let d = bleed::classify(rows(), true);
    assert_eq!(
        d.events
            .iter()
            .filter(|e| e.classification == "kick_correlated_spill_candidate")
            .count(),
        20
    );
    assert!(
        d.events
            .iter()
            .filter(|e| e.raw_attack_db == -40.)
            .all(|e| e.classification == "snare_like_protected")
    );
    let mut r = rows();
    for e in &mut r {
        if e.raw_attack_db == -35. {
            e.ambiguous_onset = true;
        }
    }
    assert!(
        bleed::classify(r, true)
            .events
            .iter()
            .all(|e| e.classification != "kick_correlated_spill_candidate")
    );
    assert!(
        bleed::classify(rows(), false)
            .events
            .iter()
            .all(|e| e.classification != "kick_correlated_spill_candidate")
    );
    let d = bleed::classify(vec![feature(1., false, false)], true);
    assert!(d.direct_reference_shape.is_none());
}
#[test]
fn training_templates_ignore_held_out_tone_and_static_failures_identify_events() {
    let b = bleed::classify(rows(), true);
    let mut altered = rows();
    for e in &mut altered {
        if e.held_out {
            e.phase_bands = [[[0.; 5]; 5]; 4];
            e.raw_attack_db = 0.;
        }
    }
    let d = bleed::classify(altered, true);
    assert_eq!(b.direct_reference_shape, d.direct_reference_shape);
    assert_eq!(b.direct_reference_level, d.direct_reference_level);
    let mut a = b.events.clone();
    for e in &mut a {
        if e.classification == "kick_correlated_spill_candidate" {
            e.phase_broad[0][4] -= 1.2;
        }
    }
    assert!(bleed::evaluate("synthetic selective change", &b, &a).accepted);
    for e in &mut a {
        if e.held_out && e.classification == "kick_correlated_spill_candidate" {
            e.phase_broad[0][4] += 1.2;
        }
    }
    let v = bleed::evaluate("held-out failure", &b, &a);
    assert!(v.training_eligible);
    assert!(!v.accepted);
    a[0].phase_bands[0][4][1] -= 1.;
    let v = bleed::evaluate("body loss", &b, &a);
    assert!(!v.training_eligible);
    assert_eq!(v.split[0]["worst_body_seconds"], serde_json::json!(0.1));
}
#[test]
fn lagged_correlation_handles_delay_polarity_silence_and_stereo_without_cancellation() {
    let mut seed = 73_u64;
    let a = (0..512)
        .map(|_| {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let v = (seed >> 32) as f64 / u32::MAX as f64 - 0.5;
            [v, -v]
        })
        .collect::<Vec<_>>();
    let mut b = vec![[0.; 2]; 512];
    for i in 12..512 {
        b[i] = a[i - 12].map(|v| v * -0.2);
    }
    let c = bleed::correlate(&a, &b, 6000.);
    assert!(c.absolute > 0.999);
    assert!((c.lag_ms - 2.).abs() < 0.01);
    let c = bleed::correlate(&a, &vec![[0.; 2]; 512], 6000.);
    assert_eq!(c.absolute, 0.);
}

#[test]
fn native_measurement_keeps_history_and_measures_real_injected_spill() {
    use gigpies::automix::{
        config::{self, Role},
        drums, unity,
    };
    let root = std::env::temp_dir().join(format!("gigpies-spill-{}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    let mut s = config::example();
    s.channels.truncate(4);
    s.sample_rate = 32000;
    for (i, c) in s.channels.iter_mut().enumerate() {
        c.role = match i {
            0 => Role::Kick,
            1 => Role::Snare,
            2 => Role::Overheads,
            _ => Role::BassDi,
        };
        c.file = format!("{i}.wav").into();
        c.eq.clear();
        c.compressor.ratio = 1.;
        c.compressor.makeup_db = 3.;
        let channels = if i == 2 { 2 } else { 1 };
        let mut w = hound::WavWriter::create(
            root.join(&c.file),
            hound::WavSpec {
                channels,
                sample_rate: 32000,
                bits_per_sample: 24,
                sample_format: hound::SampleFormat::Int,
            },
        )
        .unwrap();
        for j in 0..64000 {
            let t = j as f64 / 32000.;
            let kick = 0.3 * (-40. * (t % 0.5)).exp() * (std::f64::consts::TAU * 70. * t).sin();
            let snare = if t >= 1. {
                0.3 * (-30. * (t - 1.)).exp() * (std::f64::consts::TAU * 180. * t).sin()
            } else {
                0.
            };
            let x = match i {
                0 => kick,
                1 => snare + 0.15 * kick,
                2 => 0.1 * snare,
                _ => 0.1 * (std::f64::consts::TAU * 55. * t).sin(),
            };
            w.write_sample((x * 8388608.) as i32).unwrap();
            if channels == 2 {
                w.write_sample((-x * 8388608.) as i32).unwrap();
            }
        }
        w.finalize().unwrap();
    }
    unity::prepare(&mut s).unwrap();
    let p = bleed::Policy {
        drums: drums::Policy {
            kick: 0,
            kick_file: "0.wav".into(),
            snare: 1,
            snare_file: "1.wav".into(),
            bass: 3,
            bass_file: "3.wav".into(),
            neutral_basis: "synthetic".into(),
            current_emphasis_db: Some([0.; 2]),
            desired_emphasis_db: [2.; 2],
        },
        overhead: 2,
        overhead_file: "2.wav".into(),
        listener_confirmed_snare_bleed: true,
    };
    let m = bleed::measure(&s, &root, &p, 0., 2.).unwrap();
    assert_eq!(m.frames.len(), 200);
    let wave = &m.low_wave[100..1000];
    let a = wave.iter().map(|x| x[0]).collect::<Vec<_>>();
    let b = wave.iter().map(|x| x[1]).collect::<Vec<_>>();
    assert!(bleed::correlate(&a, &b, m.waveform_analysis_rate).absolute > 0.99);
    let tail = bleed::measure(&s, &root, &p, 0.5, 2.).unwrap();
    assert_eq!(tail.frames[0].bands, m.frames[50].bands);
    for f in &m.frames {
        if f.broad[3] > -120. {
            assert!((f.broad[4] - f.broad[3] - 3.).abs() < 1e-8);
        }
    }
    assert!(m.frames.iter().any(|f| f.broad[2] > -50.)); // Opposite-polarity stereo does not disappear.
    let mut bad = p.clone();
    bad.overhead_file = "wrong.wav".into();
    assert!(bad.validate(&s).is_err());
    assert!(bleed::run(s, &root, &root, p, 0., 2.).is_err());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn waveform_predictor_is_frozen_before_held_out_evaluation_and_changes_no_samples() {
    let d = bleed::classify(rows(), true);
    let mut seed = 19_u64;
    let mut random = || {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        (seed >> 32) as f64 / u32::MAX as f64 - 0.5
    };
    let x = (0..24000).map(|_| random()).collect::<Vec<_>>();
    let mut wave = vec![];
    for i in 0..24000 {
        let t = i as f64 / 1000.;
        let kick = x[i];
        let spill = 0.2 * x[i.saturating_sub(3)];
        let snare = if (0.4..0.6).contains(&(t % 1.)) {
            spill
        } else {
            spill + random()
        };
        wave.push([[kick, -kick], [snare, -snare], [0.; 2]]);
    }
    let mut m = bleed::Measurement {
        stages: ["kick", "snare", "overheads", "eq", "post"],
        bands_hz: bleed::BANDS,
        frames: vec![],
        waveform_analysis_rate: 1000.,
        low_wave: wave,
    };
    let original = m.low_wave.clone();
    let p = bleed::prediction_probe(&m, &d, 0);
    assert_eq!(m.low_wave, original);
    assert!((p.coefficient.unwrap() - 0.2).abs() < 1e-8);
    assert_eq!(p.lag_ms, Some(3.));
    assert!(
        p.evaluation[3]["median_explained_energy_fraction"]
            .as_f64()
            .unwrap()
            > 0.999
    );
    for v in &mut m.low_wave[12000..] {
        v[1] = [random(), random()];
    }
    let q = bleed::prediction_probe(&m, &d, 0);
    assert_eq!(p.coefficient, q.coefficient);
    assert_eq!(p.lag_ms, q.lag_ms);
    assert!(
        q.evaluation[3]["median_explained_energy_fraction"]
            .as_f64()
            .unwrap()
            < 0.1
    );
}

#[test]
fn confirmed_bleed_with_a_snare_like_shape_cannot_be_relabelled_to_pass_a_correction() {
    let mut r = rows();
    let template = feature(0., false, false).phase_bands;
    for e in &mut r {
        if e.raw_attack_db == -35. {
            e.phase_bands = template.map(|stages| stages.map(|bands| bands.map(|x| x - 17.)));
        }
    }
    let d = bleed::classify(r, true);
    assert!(d.listener_confirmed_bleed);
    assert!(
        d.events
            .iter()
            .all(|e| e.classification != "kick_correlated_spill_candidate")
    );
    let v = bleed::evaluate("no identifiable spill class", &d, &d.events);
    assert!(!v.accepted);
    assert!(
        v.reasons
            .iter()
            .any(|x| x.contains("insufficient_spill_events"))
    );
}

#[test]
fn excessive_static_corrections_and_missing_bands_abstain_without_partial_dsp() {
    use gigpies::automix::{
        config::{self, EqBand, EqKind},
        drums, unity,
    };
    let mut s = config::example();
    unity::prepare(&mut s).unwrap();
    let p = bleed::Policy {
        drums: drums::Policy {
            kick: 0,
            kick_file: s.channels[0].file.clone(),
            snare: 1,
            snare_file: s.channels[1].file.clone(),
            bass: 7,
            bass_file: s.channels[7].file.clone(),
            neutral_basis: "test".into(),
            current_emphasis_db: Some([0.; 2]),
            desired_emphasis_db: [2.; 2],
        },
        overhead: 2,
        overhead_file: s.channels[2].file.clone(),
        listener_confirmed_snare_bleed: true,
    };
    s.channels[1].eq = vec![
        EqBand {
            kind: EqKind::Bell,
            hz: 132.,
            q: 1.2,
            db: -11.,
        },
        EqBand {
            kind: EqKind::HighShelf,
            hz: 5000.,
            q: std::f64::consts::FRAC_1_SQRT_2,
            db: -11.,
        },
    ];
    let proposals = bleed::static_proposals(&s, &p).unwrap();
    assert_eq!(proposals.len(), 3);
    assert!(
        proposals
            .iter()
            .all(|p| p.settings.is_none() && p.abstention.as_ref().unwrap().contains("bounds"))
    );
    s.channels[1].eq.clear();
    assert!(
        bleed::static_proposals(&s, &p)
            .unwrap()
            .iter()
            .all(|p| p.settings.is_none())
    );
}
