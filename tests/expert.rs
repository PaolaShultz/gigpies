use gigpies::automix::{
    config::{self, Role},
    expert::{self, Capture, DynamicsRule, Family, Profile, State},
    tone::{self, Instrument, Intent, Policy},
    unity,
};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static ID: AtomicU64 = AtomicU64::new(0);
struct Case {
    root: PathBuf,
    s: config::Session,
    p: Policy,
}
impl Drop for Case {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
fn case(scale: f64, clip: bool, stereo: bool) -> Case {
    let root = std::env::temp_dir().join(format!(
        "gigpies-expert-{}-{}",
        std::process::id(),
        ID.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&root).unwrap();
    let mut s = config::example();
    s.channels.truncate(1);
    s.channels[0].file = "guitar.wav".into();
    s.channels[0].role = Role::RhythmGuitar;
    s.sample_rate = 8000;
    s.channels[0].eq.clear();
    unity::prepare(&mut s).unwrap();
    s.channels[0].compressor.threshold_db = -36.;
    s.channels[0].compressor.ratio = 8.;
    s.channels[0].compressor.makeup_db = 2.;
    let profile = Profile {
        name: "editable experiment".into(),
        family: Family::ElectricGuitar,
        capture: Capture::RecordedTrack,
        body_presence_db: None,
        source_first: Default::default(),
        dynamics: Some(DynamicsRule {
            max_mean_reduction_db: 3.,
            max_p95_reduction_db: 6.,
            max_threshold_raise_db: 12.,
            max_output_rise_db: 3.,
        }),
    };
    let g = Instrument {
        name: "known guitar".into(),
        primary: 0,
        primary_file: "guitar.wav".into(),
        secondary: vec![],
        secondary_files: vec![],
        intent: Intent::Balanced,
        capture: Some(Capture::RecordedTrack),
        profile: Some(profile),
    };
    let p = Policy {
        instruments: vec![g],
        section_seconds: 4.,
        minimum_active_seconds: 1.,
        ..Default::default()
    };
    let mut w = hound::WavWriter::create(
        root.join("guitar.wav"),
        hound::WavSpec {
            channels: if stereo { 2 } else { 1 },
            sample_rate: 8000,
            bits_per_sample: 24,
            sample_format: hound::SampleFormat::Int,
        },
    )
    .unwrap();
    for k in 0..8000 * 16 {
        let t = k as f64 / 8000.;
        let raw = scale
            * (0.04 * (std::f64::consts::TAU * 220. * t).sin()
                + 0.1 * (std::f64::consts::TAU * 1800. * t).sin());
        let x = if clip { raw.clamp(-1., 1.) } else { raw };
        w.write_sample((x * 8388608.).clamp(-8388608., 8388607.) as i32)
            .unwrap();
        if stereo {
            w.write_sample((-x * 0.5 * 8388608.) as i32).unwrap();
        }
    }
    w.finalize().unwrap();
    Case { root, s, p }
}
fn measured(c: &Case) -> Vec<tone::Frame> {
    tone::measure(&c.s, &c.root, &c.p.instruments[0]).unwrap()
}
#[test]
fn measured_compression_relief_respects_level_budget_and_preserves_makeup_routing() {
    let c = case(1., false, true);
    let before = measured(&c);
    let q = expert::propose_dynamics(&c.s, &c.p.instruments[0], &before, &c.p);
    assert_eq!(q.finding.state, State::Deviation);
    assert!(q.threshold_proposed_db > q.threshold_before_db);
    let mut candidate = c.s.clone();
    candidate.channels[0].compressor.threshold_db = q.threshold_proposed_db;
    let after = tone::measure(&candidate, &c.root, &c.p.instruments[0]).unwrap();
    let r = expert::validate_dynamics(&before, &after, &q, &c.p.instruments[0], 8000, &c.p);
    assert!(r.accepted, "{}", r.reason);
    assert!(r.after[1].median_mean_reduction_db < q.before[1].median_mean_reduction_db - 1.);
    let original = std::fs::read(c.root.join("guitar.wav")).unwrap();
    expert::run(
        c.s.clone(),
        &c.root,
        &c.root.join("result"),
        c.p.clone(),
        true,
    )
    .unwrap();
    let s: config::Session =
        serde_json::from_reader(std::fs::File::open(c.root.join("result/settings.json")).unwrap())
            .unwrap();
    let mut expected = c.s.clone();
    expected.channels[0].compressor.threshold_db = q.threshold_proposed_db;
    assert_eq!(
        serde_json::to_value(s).unwrap(),
        serde_json::to_value(expected).unwrap()
    );
    assert_eq!(original, std::fs::read(c.root.join("guitar.wav")).unwrap());
    let info: serde_json::Value = serde_json::from_reader(
        std::fs::File::open(c.root.join("result/final/measurements.json")).unwrap(),
    )
    .unwrap();
    assert!((info["processed_export_peak_dbfs"].as_f64().unwrap() + 0.01).abs() < 1e-8);
    assert!(
        expert::run(
            c.s.clone(),
            &c.root,
            &c.root.join("result"),
            c.p.clone(),
            false
        )
        .is_err()
    );
}
#[test]
fn labels_do_not_choose_processing_and_disabled_rules_preserve_settings() {
    let mut c = case(1., false, false);
    let f = measured(&c);
    let a = expert::propose_dynamics(&c.s, &c.p.instruments[0], &f, &c.p);
    c.p.instruments[0].profile.as_mut().unwrap().name = "a different display name".into();
    let b = expert::propose_dynamics(&c.s, &c.p.instruments[0], &f, &c.p);
    assert_eq!(a.threshold_proposed_db, b.threshold_proposed_db);
    c.p.instruments[0].profile.as_mut().unwrap().dynamics = None;
    expert::run(
        c.s.clone(),
        &c.root,
        &c.root.join("disabled"),
        c.p.clone(),
        false,
    )
    .unwrap();
    let s: serde_json::Value = serde_json::from_reader(
        std::fs::File::open(c.root.join("disabled/settings.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(s, serde_json::to_value(&c.s).unwrap());
}
#[test]
fn profile_applicability_and_limits_are_checked_before_output() {
    let mut c = case(1., false, false);
    c.p.instruments[0].profile.as_mut().unwrap().capture = Capture::DirectInput;
    assert!(
        expert::run(
            c.s.clone(),
            &c.root,
            &c.root.join("invalid"),
            c.p.clone(),
            false
        )
        .is_err()
    );
    assert!(!c.root.join("invalid").exists());
    c.p.instruments[0].profile.as_mut().unwrap().capture = Capture::RecordedTrack;
    c.p.instruments[0].profile.as_mut().unwrap().family = Family::AcousticGuitar;
    assert!(c.p.validate(&c.s).is_err());
    c.p.instruments[0].profile.as_mut().unwrap().family = Family::ElectricGuitar;
    c.p.instruments[0]
        .profile
        .as_mut()
        .unwrap()
        .dynamics
        .as_mut()
        .unwrap()
        .max_output_rise_db = f64::NAN;
    assert!(c.p.validate(&c.s).is_err());
}
#[test]
fn repeated_full_scale_input_contact_blocks_a_spurious_repair() {
    let c = case(20., true, false);
    let f = measured(&c);
    let finding = expert::full_scale_finding(&f, 8000, &c.p);
    assert_eq!(finding.state, State::Blocked);
    expert::run(
        c.s.clone(),
        &c.root,
        &c.root.join("blocked"),
        c.p.clone(),
        false,
    )
    .unwrap();
    let s: serde_json::Value =
        serde_json::from_reader(std::fs::File::open(c.root.join("blocked/settings.json")).unwrap())
            .unwrap();
    assert_eq!(s, serde_json::to_value(&c.s).unwrap());
    let d: serde_json::Value = serde_json::from_reader(
        std::fs::File::open(c.root.join("blocked/decisions.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(d[0]["blocked_by_input_contact"], true);
    assert_eq!(d[0]["candidate_measured"], false);
}
#[test]
fn silence_and_isolated_peak_action_do_not_trigger_sustained_repair() {
    let c = case(0., false, false);
    let q = expert::propose_dynamics(&c.s, &c.p.instruments[0], &measured(&c), &c.p);
    assert_eq!(q.threshold_before_db, q.threshold_proposed_db);
    assert_eq!(q.finding.state, State::InsufficientEvidence);
    let c = case(1., false, false);
    let mut f = measured(&c);
    for x in &mut f {
        x.mean_reduction_db = 0.;
        x.max_reduction_db = 0.;
    }
    f[0].max_reduction_db = 14.;
    let q = expert::propose_dynamics(&c.s, &c.p.instruments[0], &f, &c.p);
    assert_eq!(q.threshold_before_db, q.threshold_proposed_db);
}
#[test]
fn held_out_data_never_selects_threshold_and_bad_output_rise_is_vetoed() {
    let c = case(1., false, false);
    let before = measured(&c);
    let q = expert::propose_dynamics(&c.s, &c.p.instruments[0], &before, &c.p);
    let mut alternate = before.clone();
    for f in &mut alternate {
        if (f.seconds / 4.) as u32 % 2 == 1 {
            f.mean_reduction_db = 0.;
            f.max_reduction_db = 0.;
        }
    }
    let again = expert::propose_dynamics(&c.s, &c.p.instruments[0], &alternate, &c.p);
    assert_eq!(q.threshold_proposed_db, again.threshold_proposed_db);
    let mut s = c.s.clone();
    s.channels[0].compressor.threshold_db = q.threshold_proposed_db;
    let mut after = tone::measure(&s, &c.root, &c.p.instruments[0]).unwrap();
    for f in &mut after {
        if (f.seconds / 4.) as u32 % 2 == 1 {
            f.primary_dbfs += 8.;
        }
    }
    assert!(
        !expert::validate_dynamics(&before, &after, &q, &c.p.instruments[0], 8000, &c.p).accepted
    );
}
#[test]
fn cross_rule_guard_checks_dynamics_even_without_a_threshold_proposal() {
    let c = case(1., false, false);
    let mut before = measured(&c);
    for f in &mut before {
        f.mean_reduction_db = 2.;
        f.max_reduction_db = 4.;
    }
    let q = expert::propose_dynamics(&c.s, &c.p.instruments[0], &before, &c.p);
    assert_eq!(q.threshold_proposed_db, q.threshold_before_db);
    let mut after = before.clone();
    for f in &mut after {
        f.mean_reduction_db = 5.;
        f.max_reduction_db = 7.;
    }
    assert!(
        !expert::validate_dynamics(&before, &after, &q, &c.p.instruments[0], 8000, &c.p).accepted
    );
}
#[test]
fn healthy_quiet_source_is_not_normalized_or_compressed_to_a_target() {
    let mut c = case(0.02, false, false);
    c.s.channels[0].compressor.threshold_db = -8.;
    let f = measured(&c);
    let q = expert::propose_dynamics(&c.s, &c.p.instruments[0], &f, &c.p);
    assert_eq!(q.finding.state, State::WithinTarget);
    assert_eq!(q.threshold_proposed_db, q.threshold_before_db);
}
#[test]
fn preserved_source_stays_unchanged_but_injected_compressor_fault_can_be_relieved() {
    let mut c = case(1., false, true);
    c.s = gigpies::automix::preservation::source_settings(&c.s).unwrap();
    expert::run(
        c.s.clone(),
        &c.root,
        &c.root.join("healthy"),
        c.p.clone(),
        false,
    )
    .unwrap();
    let selected: serde_json::Value =
        serde_json::from_reader(std::fs::File::open(c.root.join("healthy/settings.json")).unwrap())
            .unwrap();
    assert_eq!(selected, serde_json::to_value(&c.s).unwrap());
    // Known added processing fault relative to an explicit dynamics budget.
    // This is not a diagnosis of the signal's printed processing history.
    c.s.channels[0].compressor.ratio = 8.;
    c.s.channels[0].compressor.threshold_db = -36.;
    let before = measured(&c);
    let q = expert::propose_dynamics(&c.s, &c.p.instruments[0], &before, &c.p);
    assert_eq!(q.finding.state, State::Deviation);
    let mut candidate = c.s.clone();
    candidate.channels[0].compressor.threshold_db = q.threshold_proposed_db;
    let after = tone::measure(&candidate, &c.root, &c.p.instruments[0]).unwrap();
    let result = expert::validate_dynamics(&before, &after, &q, &c.p.instruments[0], 8000, &c.p);
    assert!(result.accepted, "{}", result.reason);
    assert!(result.after[1].median_mean_reduction_db < q.before[1].median_mean_reduction_db);
    assert_eq!(candidate.channels[0].compressor.makeup_db, 0.);
    assert_eq!(candidate.channels[0].fader_db, c.s.channels[0].fader_db);
}

#[test]
fn numerical_profile_overrides_legacy_demo_name_and_combined_repairs_are_validated() {
    let mut c = case(1., false, false);
    c.p.instruments[0].intent = Intent::Thin;
    c.p.instruments[0]
        .profile
        .as_mut()
        .unwrap()
        .body_presence_db = Some([-2., 4.]);
    let m = measured(&c);
    let q = tone::propose(&m, 8000, &c.p.instruments[0], &c.p);
    assert_eq!(q.range_db, Some([-2., 4.]));
    assert!(!q.proposed_eq.is_empty());
    expert::run(
        c.s.clone(),
        &c.root,
        &c.root.join("combined"),
        c.p.clone(),
        false,
    )
    .unwrap();
    let d: serde_json::Value = serde_json::from_reader(
        std::fs::File::open(c.root.join("combined/decisions.json")).unwrap(),
    )
    .unwrap();
    assert!(d[0]["body_validation"].is_object());
    assert!(d[0]["dynamics_validation"].is_object());
    // A bundle may be vetoed; every proposed change must share that final outcome.
    let applied: config::Session = serde_json::from_reader(
        std::fs::File::open(c.root.join("combined/settings.json")).unwrap(),
    )
    .unwrap();
    if d[0]["accepted"] == true {
        assert!(!applied.channels[0].eq.is_empty());
        assert!(
            applied.channels[0].compressor.threshold_db > c.s.channels[0].compressor.threshold_db
        );
    } else {
        assert_eq!(
            serde_json::to_value(applied).unwrap(),
            serde_json::to_value(&c.s).unwrap()
        );
    }
}
#[test]
fn compliant_body_rule_is_guarded_during_a_dynamics_only_repair() {
    let mut c = case(1., false, false);
    c.p.instruments[0]
        .profile
        .as_mut()
        .unwrap()
        .body_presence_db = Some([-24., 6.]);
    expert::run(
        c.s.clone(),
        &c.root,
        &c.root.join("guarded"),
        c.p.clone(),
        false,
    )
    .unwrap();
    let d: serde_json::Value = serde_json::from_reader(
        std::fs::File::open(c.root.join("guarded/decisions.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(d[0]["accepted"], true);
    assert_eq!(d[0]["body_plan"]["proposed_eq"], serde_json::json!([]));
    assert_eq!(d[0]["body_validation"]["accepted"], true);
}

#[test]
fn source_first_advice_is_setup_specific_and_never_infers_mic_position() {
    let mut c = case(1., false, false);
    c.p.instruments[0]
        .profile
        .as_mut()
        .unwrap()
        .body_presence_db = Some([2., 8.]);
    let m = measured(&c);
    let q = tone::propose(&m, 8000, &c.p.instruments[0], &c.p);
    for capture in [
        Capture::AmplifierMicrophone,
        Capture::DirectInput,
        Capture::AcousticMicrophone,
        Capture::RecordedTrack,
    ] {
        let g = &mut c.p.instruments[0];
        g.capture = Some(capture);
        g.profile.as_mut().unwrap().capture = capture;
        let a = expert::source_advice(&c.p.instruments[0], &q, &c.p, false);
        assert!(a.suspected_physical_cause.is_none());
        if capture == Capture::RecordedTrack {
            assert!(!a.blocks_automatic_changes);
            assert!(!a.repeat_soundcheck);
        } else {
            assert!(a.blocks_automatic_changes);
            assert!(a.repeat_soundcheck);
        }
        if capture == Capture::AmplifierMicrophone {
            assert!(a.requested_steps.join(" ").contains("add a little bass"));
        }
        if capture == Capture::DirectInput {
            assert!(!a.requested_steps.join(" ").contains("amp"));
        }
    }
}
#[test]
fn large_live_source_mismatch_requests_recapture_and_preserves_last_settings() {
    let mut c = case(1., false, false);
    let g = &mut c.p.instruments[0];
    g.capture = Some(Capture::AmplifierMicrophone);
    let profile = g.profile.as_mut().unwrap();
    profile.capture = Capture::AmplifierMicrophone;
    profile.body_presence_db = Some([2., 8.]);
    expert::run(
        c.s.clone(),
        &c.root,
        &c.root.join("source-first"),
        c.p.clone(),
        false,
    )
    .unwrap();
    let saved: serde_json::Value = serde_json::from_reader(
        std::fs::File::open(c.root.join("source-first/settings.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(saved, serde_json::to_value(&c.s).unwrap());
    let d: serde_json::Value = serde_json::from_reader(
        std::fs::File::open(c.root.join("source-first/decisions.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(d[0]["source_advice"]["repeat_soundcheck"], true);
    assert_eq!(d[0]["settings_changed"], false);
    assert_eq!(d[0]["candidate_measured"], false);
}

#[test]
fn exhausted_eq_capacity_keeps_diagnosis_and_preserves_previous_processing() {
    let mut c = case(1., false, false);
    c.s.channels[0].eq = vec![
        config::EqBand {
            kind: config::EqKind::Bell,
            hz: 400.,
            q: 1.,
            db: 0.
        };
        8
    ];
    c.p.instruments[0]
        .profile
        .as_mut()
        .unwrap()
        .body_presence_db = Some([-2., 4.]);
    expert::run(
        c.s.clone(),
        &c.root,
        &c.root.join("capacity"),
        c.p.clone(),
        false,
    )
    .unwrap();
    let d: serde_json::Value = serde_json::from_reader(
        std::fs::File::open(c.root.join("capacity/decisions.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(d[0]["eq_capacity_blocked"], true);
    assert_eq!(d[0]["candidate_measured"], false);
    assert!(d[0]["body_validation"].is_null());
    let s: serde_json::Value = serde_json::from_reader(
        std::fs::File::open(c.root.join("capacity/settings.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(s, serde_json::to_value(&c.s).unwrap());
}

#[test]
fn floating_point_headroom_is_not_misdiagnosed_as_clipped_pcm() {
    let c = case(1., false, false);
    let mut w = hound::WavWriter::create(
        c.root.join("guitar.wav"),
        hound::WavSpec {
            channels: 1,
            sample_rate: 8000,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        },
    )
    .unwrap();
    for k in 0..8000 * 16 {
        w.write_sample((2. * (std::f64::consts::TAU * 440. * k as f64 / 8000.).sin()) as f32)
            .unwrap();
    }
    w.finalize().unwrap();
    let m = measured(&c);
    assert!(m.iter().any(|f| f.input_full_scale_fraction > 0.1));
    assert_eq!(
        expert::full_scale_finding(&m, 8000, &c.p).state,
        State::NotApplicable
    );
}

#[test]
fn dynamics_activity_uses_raw_microphones_when_processing_makes_room_dominant() {
    let mut c = case(1., false, false);
    let reader = hound::WavReader::open(c.root.join("guitar.wav")).unwrap();
    let mut writer = hound::WavWriter::create(c.root.join("secondary.wav"), reader.spec()).unwrap();
    for x in reader.into_samples::<i32>() {
        writer.write_sample(x.unwrap() / 4).unwrap();
    }
    writer.finalize().unwrap();
    let mut secondary = c.s.channels[0].clone();
    secondary.file = "secondary.wav".into();
    secondary.compressor.ratio = 1.;
    secondary.compressor.makeup_db = 0.;
    c.s.channels.push(secondary);
    c.p.instruments[0].secondary = vec![1];
    c.p.instruments[0].secondary_files = vec!["secondary.wav".into()];
    c.s.channels[0].compressor.threshold_db = -50.;
    let f = measured(&c);
    assert!(f.iter().all(|x| x.raw_secondary_relative_db < -11.9));
    assert!(f.iter().any(|x| x.secondary_relative_db > 0.));
    let q = expert::propose_dynamics(&c.s, &c.p.instruments[0], &f, &c.p);
    assert_eq!(q.finding.state, State::Deviation);
    assert!(q.threshold_proposed_db > q.threshold_before_db);
    // Equal raw inputs remain insufficiently direct even with a louder primary fader.
    std::fs::copy(c.root.join("guitar.wav"), c.root.join("secondary.wav")).unwrap();
    c.s.channels[0].fader_db = 6.;
    let q = expert::propose_dynamics(&c.s, &c.p.instruments[0], &measured(&c), &c.p);
    assert_eq!(q.finding.state, State::InsufficientEvidence);
}
