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
        training: vec![[0., 2.], [2., 4.]],
        held_out: vec![[4., 6.], [6., 8.]],
    };
    Fixture { root, s, p }
}
#[test]
fn expert_plan_uses_actual_fx_and_preserves_direct_chain() {
    let f = fixture();
    let out = f.root.join("plan");
    ambience::run(f.s.clone(), &f.root, &out, f.p.clone()).unwrap();
    let verified = ambience::verify(&out, &f.root).unwrap();
    assert!(verified.technically_eligible);
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
    let audit = ambience::audit_saved(&out, &f.root.join("zero-audit")).unwrap();
    assert_eq!(audit.recorded_selection, "baseline");
    assert!(audit.buses.is_empty());
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
    samples[4 * 8000 * 2..6 * 8000 * 2].fill(0);
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
    for case in 0..6 {
        let mut p = f.p.clone();
        match case {
            0 => p.groups[0].family = Family::SuppliedFx,
            1 => p.groups[1].inputs[0].channel = 0,
            2 => p.held_out[0] = [1., 3.],
            3 => p.groups[0].inputs[0].file = "wrong.wav".into(),
            4 => p.amount = f64::NAN,
            _ => {
                p.training = vec![[0., 2.], [4., 6.]];
                p.held_out = vec![[2., 4.], [6., 8.]];
            }
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

#[test]
fn individual_decay_limits_remain_visible_when_ensemble_checks_pass() {
    let mut f = fixture();
    f.s.channels[0].role = Role::Other;
    f.p.groups[0].family = Family::Percussion;
    f.p.groups[0].existing_space_reported = true;
    f.p.style = Style::Metal;
    let out = f.root.join("limited-decay");
    let review = ambience::run(f.s.clone(), &f.root, &out, f.p.clone()).unwrap();
    assert!(review.technically_eligible);
    assert_eq!(review.decay_range_limits(), 1);
    let decay = review.decisions[0].decay_calibration.as_ref().unwrap();
    assert_eq!(
        decay.target_range,
        ambience::DecayRange::BelowMeasuredMinimum
    );
    assert_eq!(decay.selected_control, 0.);
    assert!(decay.residual_seconds > 0.01);
    assert_eq!(decay.measured_seconds, decay.minimum_control_seconds);
    assert_eq!(review.listener_accepted, None);
    assert_eq!(review.full_export_verified, None);
    assert_eq!(review.export_gain_db, None);
    let measured = &review.bus_observations[0];
    assert_eq!(measured.split, "training_pooled");
    assert!(measured.residual_db.unwrap().abs() < 1e-8);
    assert!(review.summary().contains("outside measured range: 1"));
    let text = std::fs::read_to_string(out.join("REVIEW.md")).unwrap();
    assert!(text.contains("Requested decay below engine minimum"));
    assert!(text.contains("Listener acceptance: **pending**"));
    assert!(text.contains("Full export verification and export gain: **unmeasured**"));
    let selected: serde_json::Value =
        serde_json::from_reader(std::fs::File::open(out.join("selection.json")).unwrap()).unwrap();
    assert_eq!(selected["technically_eligible"], true);
    assert_eq!(selected["decay_targets_outside_measured_range"], 1);
}

#[test]
fn wet_targets_follow_amount_and_expose_the_schema_return_floor() {
    let mut f = fixture();
    f.p.amount = 0.25;
    let quiet = ambience::run(f.s.clone(), &f.root, &f.root.join("quarter"), f.p.clone()).unwrap();
    let c = &quiet.calibration[0];
    assert!((c.amount_db - 20. * 0.25_f64.log10()).abs() < 1e-10);
    assert!((c.effective_target_db - c.artistic_profile_target_db - c.amount_db).abs() < 1e-10);
    assert!(!c.return_floor_applied);
    assert!(quiet.bus_observations[0].residual_db.unwrap().abs() < 1e-8);
    f.p.amount = 1e-30;
    let tiny = ambience::run(f.s.clone(), &f.root, &f.root.join("tiny"), f.p.clone()).unwrap();
    let floor = &tiny.calibration[0];
    assert_eq!(floor.bounded_return_db, -60.);
    assert!(floor.return_floor_applied);
    assert_eq!(tiny.return_limits(), 1);
    assert_eq!(floor.amount_db, -600.);
    assert!(tiny.markdown().contains("1.000e-30"));
    assert!(tiny.markdown().contains("−60 dB return floor"));
    assert!(!tiny.markdown().contains("None; −60"));
    assert!(floor.predicted_residual_db > 30.);
    assert!(
        (tiny.bus_observations[0].residual_db.unwrap() - floor.predicted_residual_db).abs() < 1e-8
    );
    assert_eq!(
        serde_json::to_value(&quiet.decisions).unwrap(),
        serde_json::to_value(&tiny.decisions).unwrap()
    );
}

#[test]
fn absent_held_out_source_is_unmeasured_and_never_recalibrates_returns() {
    let f = fixture();
    let before = ambience::run(f.s.clone(), &f.root, &f.root.join("before"), f.p.clone()).unwrap();
    let path = f.root.join("0.wav");
    let mut reader = hound::WavReader::open(&path).unwrap();
    let spec = reader.spec();
    let mut samples = reader
        .samples::<i32>()
        .map(Result::unwrap)
        .collect::<Vec<_>>();
    drop(reader);
    samples[6 * 8000 * 2..].fill(0);
    let mut writer = hound::WavWriter::create(&path, spec).unwrap();
    for sample in samples {
        writer.write_sample(sample).unwrap();
    }
    writer.finalize().unwrap();
    let after = ambience::run(f.s.clone(), &f.root, &f.root.join("after"), f.p.clone()).unwrap();
    assert_eq!(
        serde_json::to_value(&before.calibration).unwrap(),
        serde_json::to_value(&after.calibration).unwrap()
    );
    assert_eq!(
        serde_json::to_value(&before.training_checks).unwrap(),
        serde_json::to_value(&after.training_checks).unwrap()
    );
    let held = after.bus_observations.last().unwrap();
    assert_eq!(held.split, "held_out");
    assert_eq!(held.active_windows, 0);
    assert_eq!(held.measured_wet_source_db, None);
    assert_eq!(held.residual_db, None);
    assert_eq!(held.evidence, "insufficient_active_source");
}

#[test]
fn master_action_cannot_hide_behind_passed_relative_ensemble_guards() {
    let f = fixture();
    // Floating sources may exceed unity. This deliberately overdrives the
    // otherwise inactive +24 dBFS FX master ceiling without changing any policy.
    let path = f.root.join("0.wav");
    let mut reader = hound::WavReader::open(&path).unwrap();
    let samples = reader
        .samples::<i32>()
        .map(Result::unwrap)
        .collect::<Vec<_>>();
    drop(reader);
    let mut writer = hound::WavWriter::create(
        &path,
        hound::WavSpec {
            channels: 2,
            sample_rate: 8000,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        },
    )
    .unwrap();
    for sample in samples {
        writer
            .write_sample(sample as f32 / 8388608. * 400.)
            .unwrap();
    }
    writer.finalize().unwrap();
    let obs = ambience::measure(&f.s, &f.root, &f.p, None, &f.p.training).unwrap();
    let (mut fx, _, _) = ambience::propose(&f.s, &f.p, &obs).unwrap();
    fx.buses[0].return_db = -60.;
    let measured = ambience::measure(&f.s, &f.root, &f.p, Some(&fx), &f.p.training).unwrap();
    assert!(
        ambience::checks(&measured, &f.p.training)
            .iter()
            .any(|c| !c.passed),
        "A master-stage reduction must make this spatial-only candidate ineligible"
    );
    assert!(measured.iter().any(|frame| frame.master_reduction_db > 6.));
    let mut candidate = f.s.clone();
    candidate.effects = Some(fx);
    let rendered = f.root.join("overload-render");
    automix::run(candidate, &f.root, &rendered, None).unwrap();
    let mut reader = hound::WavReader::open(rendered.join("processed-unity-float.wav")).unwrap();
    let samples = reader
        .samples::<f32>()
        .map(Result::unwrap)
        .collect::<Vec<_>>();
    for frame in measured {
        let start = (frame.start * 8000.).round() as usize * 2;
        let end = (frame.end * 8000.).round() as usize * 2;
        let power = samples[start..end]
            .iter()
            .map(|&x| f64::from(x).powi(2))
            .sum::<f64>()
            / (end - start) as f64;
        assert!((frame.mixed_power - power).abs() <= (power * 1e-7).max(1e-8));
    }
    let out = f.root.join("rejected");
    assert!(ambience::run(f.s.clone(), &f.root, &out, f.p.clone()).is_err());
    assert!(!out.join("settings.json").exists());
    assert!(!out.join("ready.json").exists());
    assert!(out.join("failure.json").is_file());
    assert!(ambience::verify(&out, &f.root).is_err());
    let report: ambience::Review =
        serde_json::from_reader(std::fs::File::open(out.join("review.json")).unwrap()).unwrap();
    assert!(!report.technically_eligible);
    assert_eq!(report.selected, "baseline");
    assert!(report.held_out_checks.is_empty());
    assert!(
        report
            .training_checks
            .iter()
            .any(|c| c.max_master_reduction_db.unwrap() > 6.)
    );
    let audit = ambience::audit_saved(&out, &f.root.join("rejected-audit")).unwrap();
    assert_eq!(audit.recorded_ensemble_checks_passed, Some(false));
    assert_eq!(audit.recorded_selection, "baseline");
    assert_eq!(audit.recorded_listener_acceptance, None);
}

#[test]
fn readiness_binds_settings_reports_and_source_bytes_without_writing() {
    let f = fixture();
    let out = f.root.join("pinned");
    automix::write_json(&f.root.join("baseline.json"), &f.s).unwrap();
    automix::write_json(&f.root.join("policy.json"), &f.p).unwrap();
    let cli = |command: &str| {
        let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_gigpies"));
        cmd.arg(command);
        if command == "ambience-plan" {
            cmd.arg(f.root.join("baseline.json"))
                .arg(&f.root)
                .arg(&out)
                .arg(f.root.join("policy.json"));
        } else {
            cmd.arg(&out).arg(&f.root);
        }
        cmd.output().unwrap()
    };
    let result = cli("ambience-plan");
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(String::from_utf8_lossy(&result.stdout).contains("Listener acceptance"));
    let snapshot = std::fs::read_dir(&out)
        .unwrap()
        .map(|entry| {
            let path = entry.unwrap().path();
            let bytes = std::fs::read(&path).unwrap();
            (path, bytes)
        })
        .collect::<Vec<_>>();
    let result = cli("ambience-check");
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    for (path, bytes) in &snapshot {
        assert_eq!(*bytes, std::fs::read(path).unwrap());
    }
    for name in [
        "settings.json",
        "policy.json",
        "decisions.json",
        "REVIEW.md",
        "held-out-checks.json",
    ] {
        let path = out.join(name);
        let original = std::fs::read(&path).unwrap();
        let mut changed = original.clone();
        changed.push(b' ');
        std::fs::write(&path, changed).unwrap();
        let error = ambience::verify(&out, &f.root).unwrap_err().to_string();
        assert!(error.contains(name), "{error}");
        std::fs::write(path, original).unwrap();
    }
    let source = f.root.join("0.wav");
    let original = std::fs::read(&source).unwrap();
    let mut changed = original.clone();
    *changed.last_mut().unwrap() ^= 1;
    std::fs::write(&source, changed).unwrap();
    assert!(
        ambience::verify(&out, &f.root)
            .unwrap_err()
            .to_string()
            .contains("source recordings changed")
    );
    std::fs::write(&source, original).unwrap();
    ambience::verify(&out, &f.root).unwrap();
    let ready = std::fs::read(out.join("ready.json")).unwrap();
    std::fs::remove_file(out.join("ready.json")).unwrap();
    assert!(
        ambience::verify(&out, &f.root)
            .unwrap_err()
            .to_string()
            .contains("incomplete or legacy")
    );
    assert!(out.join("settings.json").exists());
    std::fs::write(out.join("ready.json"), ready).unwrap();
    ambience::verify(&out, &f.root).unwrap();
}

#[test]
fn interrupted_preparation_retains_inputs_and_requires_a_fresh_retry() {
    let f = fixture();
    let out = f.root.join("interrupted");
    automix::write_json(&f.root.join("baseline.json"), &f.s).unwrap();
    automix::write_json(&f.root.join("policy.json"), &f.p).unwrap();
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_gigpies"))
        .arg("ambience-plan")
        .arg(f.root.join("baseline.json"))
        .arg(&f.root)
        .arg(&out)
        .arg(f.root.join("policy.json"))
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    while !out.join("seed-fx.json").exists() {
        assert!(
            child.try_wait().unwrap().is_none(),
            "child exited before interruption point"
        );
        if std::time::Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("preparation did not reach the interruption point");
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    child.kill().unwrap();
    assert!(!child.wait().unwrap().success());
    assert!(!out.join("ready.json").exists());
    assert!(ambience::verify(&out, &f.root).is_err());
    let baseline = std::fs::read(out.join("before-settings.json")).unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&baseline).unwrap();
    assert_eq!(parsed, serde_json::to_value(&f.s).unwrap());
    assert!(ambience::run(f.s.clone(), &f.root, &out, f.p.clone()).is_err());
    assert_eq!(
        baseline,
        std::fs::read(out.join("before-settings.json")).unwrap()
    );
    let retry = f.root.join("retry");
    ambience::run(f.s.clone(), &f.root, &retry, f.p.clone()).unwrap();
    assert!(
        ambience::verify(&retry, &f.root)
            .unwrap()
            .technically_eligible
    );
}

#[test]
fn failed_measurement_records_recovery_without_ready_settings() {
    let f = fixture();
    let path = f.root.join("0.wav");
    let bytes = std::fs::read(&path).unwrap();
    // Retain a valid WAV header but truncate its declared sample payload.
    std::fs::write(&path, &bytes[..100]).unwrap();
    let out = f.root.join("failed");
    assert!(ambience::run(f.s.clone(), &f.root, &out, f.p.clone()).is_err());
    assert!(out.join("before-settings.json").is_file());
    assert!(out.join("input-identity.json").is_file());
    assert!(!out.join("settings.json").exists());
    assert!(!out.join("ready.json").exists());
    let failure: serde_json::Value =
        serde_json::from_reader(std::fs::File::open(out.join("failure.json")).unwrap()).unwrap();
    assert_eq!(failure["ready"], false);
    assert_eq!(failure["baseline_retained"], true);
    std::fs::write(&path, bytes).unwrap();
    assert!(ambience::verify(&out, &f.root).is_err());
}

#[test]
fn source_change_during_held_out_measurement_cannot_publish_readiness() {
    let f = fixture();
    let out = f.root.join("changing-input");
    automix::write_json(&f.root.join("baseline.json"), &f.s).unwrap();
    automix::write_json(&f.root.join("policy.json"), &f.p).unwrap();
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_gigpies"))
        .arg("ambience-plan")
        .arg(f.root.join("baseline.json"))
        .arg(&f.root)
        .arg(&out)
        .arg(f.root.join("policy.json"))
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    while !out.join("frozen-before-held-out.json").exists() {
        assert!(child.try_wait().unwrap().is_none());
        if std::time::Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("preparation did not freeze its candidate");
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    // Change one valid sample while retaining the path and size. The completed
    // source identity check must catch it even if a reader already buffered it.
    let path = f.root.join("0.wav");
    let original = std::fs::read(&path).unwrap();
    {
        use std::io::{Seek, SeekFrom, Write};
        let mut file = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
        file.seek(SeekFrom::End(-3)).unwrap();
        file.write_all(&[original[original.len() - 3] ^ 1]).unwrap();
    }
    let result = child.wait_with_output().unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("source recordings changed"));
    assert!(out.join("held-out-checks.json").is_file());
    assert!(out.join("failure.json").is_file());
    assert!(!out.join("settings.json").exists());
    assert!(!out.join("ready.json").exists());
    std::fs::write(&path, original).unwrap();
    assert!(ambience::verify(&out, &f.root).is_err());
}

#[test]
fn measurement_support_uses_complete_windows_and_rejects_broken_evidence() {
    let f = fixture();
    let spans = [[0., 1.001]];
    let frames = ambience::measure(&f.s, &f.root, &f.p, None, &spans).unwrap();
    assert_eq!(frames.len(), 50);
    assert_eq!(frames.last().unwrap().end, 1.);
    assert!(frames.iter().all(|v| (v.end - v.start - 0.02).abs() < 1e-9));
    let complete = ambience::checks(&frames, &spans);
    assert!(complete[0].passed);
    assert_eq!(complete[0].observed_windows, Some(50));
    let mut gap = frames.clone();
    gap.remove(25);
    let check = &ambience::checks(&gap, &spans)[0];
    assert!(!check.passed);
    assert_eq!(check.mixed_rms_change_db, None);
    assert!(check.failure_reasons.iter().any(|v| v.contains("coverage")));
    for case in 0..5 {
        let mut invalid = frames.clone();
        match case {
            0 => invalid[10].mixed_power = f64::NAN,
            1 => invalid[10].mixed_peak = f64::INFINITY,
            2 => invalid[10].dry_power = -1.,
            3 => invalid[10].master_reduction_db = -1.,
            _ => invalid[10].end = invalid[10].start,
        }
        let check = &ambience::checks(&invalid, &spans)[0];
        assert!(!check.passed);
        assert_eq!(check.mixed_rms_change_db, None);
        assert_eq!(check.max_master_reduction_db, None);
        let json = serde_json::to_value(check).unwrap();
        assert!(json["mixed_rms_change_db"].is_null());
        assert!(!check.failure_reasons.is_empty());
    }
    let missing = &ambience::checks(&[], &spans)[0];
    assert!(!missing.passed);
    assert_eq!(missing.dry_rms_dbfs, None);
}

#[test]
fn saved_audit_exposes_bounds_without_sources_or_new_eligibility() {
    let mut f = fixture();
    f.p.amount = 1e-30;
    let plan = f.root.join("plan");
    let review = ambience::run(f.s.clone(), &f.root, &plan, f.p.clone()).unwrap();
    let snapshot = std::fs::read_dir(&plan)
        .unwrap()
        .map(|entry| {
            let path = entry.unwrap().path();
            let bytes = std::fs::read(&path).unwrap();
            (path, bytes)
        })
        .collect::<Vec<_>>();
    // The saved audit deliberately has no dependency on current audio.
    std::fs::remove_file(f.root.join("0.wav")).unwrap();
    let out = f.root.join("audit");
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_gigpies"))
        .arg("ambience-audit")
        .arg(&plan)
        .arg(&out)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(String::from_utf8_lossy(&result.stdout).contains("1 bounded returns"));
    let audit: ambience::SavedAudit =
        serde_json::from_slice(&std::fs::read(out.join("audit.json")).unwrap()).unwrap();
    assert!(audit.training_precedes_held_out);
    assert!(audit.recorded_master_stage_observations);
    assert!(audit.recorded_coverage_observations);
    assert!(!audit.current_source_identity_verified);
    assert_eq!(audit.recorded_listener_acceptance, None);
    assert!(audit.buses[0].return_floor_applied);
    assert_eq!(
        audit.buses[0].predicted_residual_db,
        review.calibration[0].predicted_residual_db
    );
    assert_eq!(std::fs::read_dir(&out).unwrap().count(), 2);
    assert!(!out.join("settings.json").exists());
    assert!(!out.join("ready.json").exists());
    assert!(ambience::audit_saved(&plan, &out).is_err());
    assert!(ambience::verify(&plan, &f.root).is_err());
    for (path, bytes) in snapshot {
        assert_eq!(bytes, std::fs::read(path).unwrap());
    }
}

#[test]
fn saved_audit_preserves_legacy_unknowns_and_refuses_inconsistent_evidence() {
    let f = fixture();
    let plan = f.root.join("plan");
    ambience::run(f.s.clone(), &f.root, &plan, f.p.clone()).unwrap();
    // Build a legacy-shaped saved-report fixture. These artificial timestamps
    // test parsing and scope labels; they are not new audio measurements.
    let mut legacy = f.p.clone();
    legacy.training = vec![[0., 2.], [4., 6.]];
    legacy.held_out = vec![[2., 4.], [6., 8.]];
    std::fs::write(
        plan.join("policy.json"),
        serde_json::to_vec_pretty(&legacy).unwrap(),
    )
    .unwrap();
    for (name, spans) in [
        ("training-checks.json", &legacy.training),
        ("held-out-checks.json", &legacy.held_out),
    ] {
        let path = plan.join(name);
        let mut checks: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        for (check, span) in checks.as_array_mut().unwrap().iter_mut().zip(spans) {
            check["span"] = serde_json::json!(span);
            for key in [
                "max_master_reduction_db",
                "observed_windows",
                "observed_seconds",
                "failure_reasons",
            ] {
                check.as_object_mut().unwrap().remove(key);
            }
        }
        std::fs::write(&path, serde_json::to_vec_pretty(&checks).unwrap()).unwrap();
    }
    let audit = ambience::audit_saved(&plan, &f.root.join("legacy-audit")).unwrap();
    assert!(!audit.training_precedes_held_out);
    assert!(!audit.recorded_master_stage_observations);
    assert!(!audit.recorded_coverage_observations);
    assert!(audit.markdown().contains("leave passage coverage unknown"));
    assert_eq!(audit.recorded_ensemble_checks_passed, Some(true));
    assert_eq!(audit.recorded_listener_acceptance, None);
    assert!(
        audit
            .markdown()
            .contains("Missing legacy master-stage observations remain unknown")
    );

    for case in 0..6 {
        let name = match case {
            0 => "calibration.json",
            1 => "decisions.json",
            2 => "seed-fx.json",
            3 => "training-checks.json",
            4 => "settings.json",
            _ => "selection.json",
        };
        let path = plan.join(name);
        let original = std::fs::read(&path).unwrap();
        let mut value: serde_json::Value = serde_json::from_slice(&original).unwrap();
        match case {
            0 => value[0]["bounded_return_db"] = serde_json::json!(17.),
            1 => value[0]["bus_names"]
                .as_array_mut()
                .unwrap()
                .push(serde_json::json!("missing_bus")),
            2 => value["exciter_amount"] = serde_json::json!(0.1),
            3 => value[0]["mixed_rms_change_db"] = serde_json::json!(3.),
            4 => value["channels"][0]["fader_db"] = serde_json::json!(-4.),
            _ => value["listener_accepted"] = serde_json::json!("yes"),
        }
        std::fs::write(&path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
        let out = f.root.join(format!("inconsistent-{case}"));
        assert!(ambience::audit_saved(&plan, &out).is_err(), "case {case}");
        assert!(!out.exists());
        std::fs::write(&path, original).unwrap();
    }
}
