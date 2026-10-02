use gigpies::automix::{
    bleed::{self, Correlation, Diagnosis, Feature, Measurement},
    bleed_reference::{self as reference, Model, Reference},
};
fn event(t: f64, strong: bool) -> Feature {
    let correlation = Correlation {
        absolute: 0.,
        signed: 0.,
        lag_ms: 0.,
        channel_pair: [0, 0],
    };
    Feature {
        seconds: t,
        held_out: (t / 12.).floor() as u64 % 2 == 1,
        ambiguous_onset: false,
        raw_attack_db: if strong { -15. } else { -35. },
        kick_distance_seconds: 0.,
        phase_bands: [[[-40.; 5]; 5]; 4],
        phase_broad: [[-40.; 5]; 4],
        low_band_kick_similarity: correlation.clone(),
        low_band_overhead_similarity: correlation,
        high_band_overhead_envelope_correlation: None,
        spectrum_distance_db: None,
        classification: "unclassified_protected".into(),
    }
}
fn fixture() -> (Measurement, Diagnosis, Vec<Model>) {
    let mut state = 17_u64;
    let mut random = || {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        (state >> 32) as f64 / u32::MAX as f64 - 0.5
    };
    let wave = (0..48000)
        .map(|_| {
            let (x, z) = (random(), random());
            let y = 0.2 * x - 0.1 * z;
            [[x, -x], [y, -y], [z, -z]]
        })
        .collect();
    let m = Measurement {
        stages: ["kick_raw", "snare_raw", "overheads_raw", "eq", "post"],
        bands_hz: bleed::BANDS,
        frames: vec![],
        waveform_analysis_rate: 1000.,
        low_wave: wave,
    };
    let d = Diagnosis {
        direct_reference_level: Some(-15.),
        direct_reference_shape: None,
        events: (0..28)
            .map(|i| event(24.2 + i as f64 * 0.4, false))
            .chain((0..20).map(|i| event(36.2 + i as f64 * 0.4, i % 2 == 0)))
            .collect(),
        counts: serde_json::json!({}),
        interpretation: vec![],
        listener_confirmed_bleed: true,
    };
    let models = [0, 2]
        .into_iter()
        .map(|stage| Model {
            name: format!("seed_{stage}"),
            analysis_rate: 1000.,
            target_channel: 0,
            references: vec![Reference {
                stage,
                channel: 0,
                lag_ms: 0.,
            }],
            coefficients: Some(vec![if stage == 0 { 0.2 } else { -0.1 }]),
            reference_level: Some(-15.),
            training_events: vec![],
            training_samples: 0,
            normalized_condition: None,
            abstention: None,
        })
        .collect();
    (m, d, models)
}
#[test]
fn joint_fit_recovers_known_spill_without_changing_samples_or_claiming_source_identity() {
    let (m, d, seeds) = fixture();
    let before = m.low_wave.clone();
    let model = reference::fit_joint(&m, &d, &seeds, [24., 36.]).unwrap();
    let cs = model.coefficients.as_ref().unwrap();
    assert!((cs[0] - 0.2).abs() < 0.001 && (cs[1] + 0.1).abs() < 0.001);
    let r = reference::evaluate(&m, &d, &model).unwrap();
    assert!(
        r.events
            .iter()
            .all(|x| x.explained_energy_fraction.unwrap() > 0.999)
    );
    assert_eq!(m.low_wave, before);
    assert!(!r.production_dsp_validated);
    assert_eq!(r.decision, "abstain_missing_independent_source_labels");
}
#[test]
fn held_out_transfer_change_never_refits_or_passes_as_good_prediction() {
    let (mut m, d, seeds) = fixture();
    let before = reference::fit_joint(&m, &d, &seeds, [24., 36.]).unwrap();
    for x in &mut m.low_wave[36000..] {
        x[1] = x[1].map(|v| -v);
    }
    let after = reference::fit_joint(&m, &d, &seeds, [24., 36.]).unwrap();
    assert_eq!(before.coefficients, after.coefficients);
    let r = reference::evaluate(&m, &d, &before).unwrap();
    assert!(
        r.events
            .iter()
            .filter(|x| x.held_out)
            .all(|x| x.explained_energy_fraction.unwrap() < -2.)
    );
}
#[test]
fn collinear_and_silent_references_abstain_instead_of_inventing_coefficients() {
    let (mut m, d, seeds) = fixture();
    for x in &mut m.low_wave {
        x[2] = x[0];
    }
    let c = reference::fit_joint(&m, &d, &seeds, [24., 36.]).unwrap();
    assert_eq!(c.abstention.as_deref(), Some("collinear_references"));
    assert!(c.coefficients.is_none());
    for x in &mut m.low_wave {
        x[2] = [0.; 2];
    }
    assert_eq!(
        reference::fit_joint(&m, &d, &seeds, [24., 36.])
            .unwrap()
            .abstention
            .as_deref(),
        Some("silent_reference")
    );
}
#[test]
fn contaminated_reference_can_cancel_quiet_wanted_unison_despite_a_perfect_fit() {
    let (mut m, mut d, seeds) = fixture();
    let mut model = seeds[0].clone();
    model.coefficients = Some(vec![1.]);
    for x in &mut m.low_wave {
        // During a wanted-only take both snare and reference contain the quiet snare.
        // This has the same observations as an all-spill take with unit transfer.
        let wanted = 0.01 * x[0][0];
        x[0] = [wanted, -wanted];
        x[1] = [wanted, -wanted];
        x[2] = [0.; 2];
    }
    for e in &mut d.events {
        e.raw_attack_db = -45.;
    }
    let r = reference::evaluate(&m, &d, &model).unwrap();
    assert!(
        r.events
            .iter()
            .all(|x| x.explained_energy_fraction == Some(1.))
    );
    assert!(
        r.events
            .iter()
            .all(|x| x.residual_change_db.unwrap() < -100.)
    );
    assert!(!r.production_dsp_validated);
    assert_eq!(r.decision, "abstain_missing_independent_source_labels");
    // Selecting the other target/reference channel preserves, rather than cancels, stereo evidence.
    model.target_channel = 1;
    model.references[0].channel = 1;
    let stereo = reference::evaluate(&m, &d, &model).unwrap();
    assert_eq!(
        r.events[0].explained_energy_fraction,
        stereo.events[0].explained_energy_fraction
    );
}
#[test]
fn complete_lagged_support_prevents_split_leakage_and_duplicate_sample_weight() {
    let (m, mut d, mut seeds) = fixture();
    d.events = vec![];
    for i in 0..8 {
        d.events.push(event(25. + i as f64 * 0.01, false));
    }
    d.events.push(event(35.92, false));
    seeds[1].references[0].lag_ms = -15.;
    let model = reference::fit_joint(&m, &d, &seeds, [24., 36.]).unwrap();
    assert_eq!(model.training_events.len(), 8);
    assert_eq!(model.training_samples, 170); // 8 overlapping 100 ms windows, each sample once.
    let r = reference::evaluate(&m, &d, &model).unwrap();
    assert_eq!(
        r.events.last().unwrap().exclusion.as_deref(),
        Some("incomplete_or_split_crossing_support")
    );
    d.events.push(event(35.93, false));
    assert_eq!(
        reference::fit_joint(&m, &d, &seeds, [24., 36.])
            .unwrap()
            .coefficients,
        model.coefficients
    );
}
#[test]
fn silence_reference_injection_compounds_and_long_ending_remain_visible() {
    let (mut m, mut d, seeds) = fixture();
    for x in &mut m.low_wave[..1000] {
        *x = [[0.; 2]; 3];
    }
    for x in &mut m.low_wave[45000..] {
        x[1] = [0.; 2];
    }
    d.events[0].ambiguous_onset = true;
    let r = reference::evaluate(&m, &d, &seeds[0]).unwrap();
    assert_eq!(r.events[0].group, "compound");
    assert!(
        r.intervals
            .iter()
            .any(|x| x.exclusion.as_deref() == Some("silent_target"))
    );
    assert!(
        r.intervals
            .iter()
            .any(|x| x.exclusion.as_deref() == Some("silent_target_reference_injects_energy"))
    );
    assert_eq!(r.intervals.len(), 480);
}
#[test]
fn incompatible_models_invalid_measurements_and_output_reuse_fail_closed() {
    let (mut m, mut d, mut seeds) = fixture();
    seeds[0].analysis_rate = 2000.;
    assert!(reference::evaluate(&m, &d, &seeds[0]).is_err());
    seeds[0].analysis_rate = 1000.;
    seeds[1].target_channel = 1;
    assert!(reference::fit_joint(&m, &d, &seeds, [24., 36.]).is_err());
    d.events[0].held_out = true;
    assert!(reference::evaluate(&m, &d, &seeds[0]).is_err());
    d.events[0].held_out = false;
    m.low_wave[0][0][0] = f64::NAN;
    assert!(reference::evaluate(&m, &d, &seeds[0]).is_err());
    let existing = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let missing = std::path::Path::new("no-such-source");
    assert!(
        reference::fit_saved(missing, missing, &existing, [24., 36.])
            .unwrap_err()
            .to_string()
            .contains("exists")
    );
    assert!(
        reference::evaluate_saved(missing, missing, missing, &existing)
            .unwrap_err()
            .to_string()
            .contains("exists")
    );
}
