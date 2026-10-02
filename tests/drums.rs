use gigpies::automix::{
    balance::rhythmic_emphasis_delta,
    drums::{self, Frame},
};
fn frames(quiet: bool, rapid: bool) -> Vec<Frame> {
    (0..2400)
        .map(|i| {
            let phase = i % if rapid { 12 } else { 50 };
            let level = if phase < 3 {
                -15.
            } else if phase < 8 {
                -30.
            } else {
                -80.
            };
            let level = level - if quiet { 30. } else { 0. };
            Frame {
                seconds: i as f64 / 100.,
                duration: 0.01,
                rms: [level; 15],
                peak: [level + 6.; 15],
                mean_gr: [0.; 2],
                max_gr: [0.; 2],
                role_band: [level; 2],
            }
        })
        .collect()
}
#[test]
fn explicit_emphasis_is_relative_and_unknown_neutral_abstains() {
    assert_eq!(
        rhythmic_emphasis_delta(Some([0., 0.]), [2., 2.]).unwrap(),
        Some([2., 2.])
    );
    assert_eq!(
        rhythmic_emphasis_delta(Some([1.5, 2.]), [2., 2.]).unwrap(),
        Some([0.5, 0.])
    );
    assert_eq!(rhythmic_emphasis_delta(None, [2., 2.]).unwrap(), None);
    assert!(rhythmic_emphasis_delta(Some([-3., 0.]), [2., 2.]).is_err());
    assert!(rhythmic_emphasis_delta(None, [f64::NAN, 2.]).is_err());
}

#[test]
fn unknown_balance_allows_fixed_fader_verification_but_never_a_fader_move() {
    use gigpies::automix::{
        config::{self, Role},
        unity,
    };
    let root = std::env::temp_dir().join(format!("gigpies-unknown-balance-{}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    let mut s = config::example();
    s.channels.truncate(3);
    s.sample_rate = 8000;
    for (i, c) in s.channels.iter_mut().enumerate() {
        c.file = format!("{i}.wav").into();
        c.role = [Role::Kick, Role::Snare, Role::BassDi][i];
        c.eq.clear();
        c.compressor.ratio = 1.;
        c.compressor.makeup_db = 1.;
        let mut writer = hound::WavWriter::create(
            root.join(&c.file),
            hound::WavSpec {
                channels: 1,
                sample_rate: 8000,
                bits_per_sample: 24,
                sample_format: hound::SampleFormat::Int,
            },
        )
        .unwrap();
        for k in 0..24 * 8000 {
            let t = k as f64 / 8000.;
            let x = 0.2
                * (-35. * (t % 0.5)).exp()
                * (std::f64::consts::TAU * [70., 180., 55.][i] * t).sin();
            writer.write_sample((x * 8388608.) as i32).unwrap();
        }
        writer.finalize().unwrap();
    }
    unity::prepare(&mut s).unwrap();
    let p = drums::Policy {
        kick: 0,
        kick_file: "0.wav".into(),
        snare: 1,
        snare_file: "1.wav".into(),
        bass: 2,
        bass_file: "2.wav".into(),
        neutral_basis: "unknown".into(),
        current_emphasis_db: None,
        desired_emphasis_db: [2.; 2],
    };
    let mut candidate = s.clone();
    candidate.channels[1].compressor.makeup_db -= 1.;
    let out = root.join("verify");
    drums::verify(
        s.clone(),
        candidate.clone(),
        &root,
        &out,
        p.clone(),
        0.,
        24.,
    )
    .unwrap();
    let read = |file: &str| -> serde_json::Value {
        serde_json::from_slice(&std::fs::read(out.join(file)).unwrap()).unwrap()
    };
    assert_eq!(read("outcome.json")["accepted"], true);
    assert_eq!(
        read("balance-intent.json")["unknown_neutral_basis_abstention"],
        true
    );
    assert_eq!(
        read("settings.json"),
        serde_json::to_value(&candidate).unwrap()
    );
    candidate.channels[1].fader_db -= 1.;
    assert!(
        drums::verify(
            s.clone(),
            candidate,
            &root,
            &root.join("bad"),
            p.clone(),
            0.,
            24.
        )
        .is_err()
    );
    assert!(!root.join("bad").exists());
    let corrected = root.join("correct");
    drums::correct(s.clone(), &root, &corrected, p, 0., 24.).unwrap();
    let applied: config::Session =
        serde_json::from_slice(&std::fs::read(corrected.join("settings.json")).unwrap()).unwrap();
    assert_eq!(
        serde_json::to_value(applied).unwrap(),
        serde_json::to_value(s).unwrap()
    );
    // Unknown intent does not hide a technical protection failure.
    let b = measurement(frames(false, false));
    let mut bad = measurement(b.frames.clone());
    for f in &mut bad.frames {
        f.max_gr[1] += 2.;
    }
    assert!(!drums::validate_change(&b, &bad, &drums::diagnose(&b), [0.; 2]).accepted);
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn silence_steady_bleed_quiet_hits_and_rapid_recovery() {
    for quiet in [false, true] {
        let f = frames(quiet, false);
        let e = drums::events(&f, 0);
        assert!(e.len() > 35);
        assert!(e.iter().all(|e| e.recovery_gr_db.is_some()));
    }
    let e = drums::events(&frames(false, true), 1);
    assert!(e.len() > 150);
    assert!(e.iter().filter(|e| e.recovery_gr_db.is_none()).count() > 140);
    let mut f = frames(false, false);
    for x in &mut f {
        x.role_band = [-240.; 2];
    }
    assert!(drums::events(&f, 0).is_empty());
    for x in &mut f {
        x.role_band = [-45.; 2];
    }
    assert!(drums::events(&f, 1).is_empty());
}
fn measurement(f: Vec<Frame>) -> drums::Measurement {
    drums::Measurement {
        stages: drums::STAGES,
        frames: f,
        spectra: vec![],
        pcm_contacts: [0; 2],
        rate: 44100,
    }
}
#[test]
fn fixed_masks_guard_processing_but_do_not_veto_artistic_level() {
    let b = measurement(frames(false, false));
    let d = drums::diagnose(&b);
    let mut a = measurement(b.frames.clone());
    for f in &mut a.frames {
        for k in [3, 4, 8, 9] {
            f.rms[k] += 2.;
            f.peak[k] += 2.;
        }
        f.peak[14] += 1.;
    }
    let v = drums::validate_change(&b, &a, &d, [2.; 2]);
    assert!(v.accepted, "{:?}", v.failures);
    for f in &mut a.frames {
        f.max_gr[0] += 2.;
    }
    assert!(!drums::validate_change(&b, &a, &d, [2.; 2]).accepted);
    let mut a = measurement(b.frames.clone());
    for f in &mut a.frames {
        f.rms[4] += 4.;
    }
    assert!(!drums::validate_change(&b, &a, &d, [0.; 2]).accepted);
    let mut a = measurement(b.frames.clone());
    for e in &d.events[1] {
        a.frames[e.frame].rms[7] -= 12.;
        a.frames[e.frame + 1].rms[7] -= 12.;
        a.frames[e.frame + 2].rms[7] -= 12.;
    }
    assert!(!drums::validate_change(&b, &a, &d, [0.; 2]).accepted);
    let a = measurement(vec![]);
    assert!(!drums::validate_change(&b, &a, &d, [0.; 2]).accepted);
}
#[test]
fn maxima_alone_never_trigger_relief_and_held_out_never_selects() {
    let b = measurement(frames(false, false));
    let mut d = drums::diagnose(&b);
    assert_eq!(drums::processing_plan(&d).threshold_relief_db, [0.; 2]);
    for e in &mut d.events[0] {
        if e.held_out {
            e.attack_body_db -= 5.;
            e.recovery_gr_db = Some(6.);
        }
    }
    assert_eq!(drums::processing_plan(&d).threshold_relief_db, [0.; 2]);
    for e in &mut d.events[0] {
        if !e.held_out {
            e.attack_body_db -= 5.;
            e.recovery_gr_db = Some(6.);
        }
    }
    assert_eq!(drums::processing_plan(&d).threshold_relief_db, [2., 0.]);
    d.events[0].truncate(2);
    assert_eq!(drums::processing_plan(&d).threshold_relief_db, [0.; 2]);
}
#[test]
fn production_strip_measurement_preserves_history_and_separates_makeup_faders() {
    use gigpies::automix::{
        config::{self, Role},
        unity,
    };
    let root = std::env::temp_dir().join(format!("gigpies-drums-{}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    let mut s = config::example();
    s.channels.truncate(3);
    s.sample_rate = 8000;
    for (i, c) in s.channels.iter_mut().enumerate() {
        c.file = format!("{i}.wav").into();
        c.role = match i {
            0 => Role::Kick,
            1 => Role::Snare,
            _ => Role::BassDi,
        };
        c.eq.clear();
        c.compressor.threshold_db = -24.;
        c.compressor.ratio = 3.;
        c.compressor.attack_ms = 9.;
        c.compressor.release_ms = 58.;
        c.compressor.makeup_db = 3.;
        let mut w = hound::WavWriter::create(
            root.join(&c.file),
            hound::WavSpec {
                channels: 1,
                sample_rate: 8000,
                bits_per_sample: 24,
                sample_format: hound::SampleFormat::Int,
            },
        )
        .unwrap();
        for j in 0..8000 * 2 {
            let t = j as f64 / 8000.;
            let x = if i == 2 {
                0.1 * (std::f64::consts::TAU * 55. * t).sin()
            } else {
                0.5 * (-35. * (t % 0.5)).exp()
                    * (std::f64::consts::TAU * if i == 0 { 70. } else { 180. } * t).sin()
            };
            w.write_sample((x * 8388608.) as i32).unwrap();
        }
        w.finalize().unwrap();
    }
    unity::prepare(&mut s).unwrap();
    let p = drums::Policy {
        kick: 0,
        kick_file: "0.wav".into(),
        snare: 1,
        snare_file: "1.wav".into(),
        bass: 2,
        bass_file: "2.wav".into(),
        neutral_basis: "synthetic controlled balance".into(),
        current_emphasis_db: Some([0.; 2]),
        desired_emphasis_db: [2.; 2],
    };
    let b = drums::measure(&s, &root, &p, 0., 2.).unwrap();
    assert!(b.frames.iter().any(|f| f.max_gr[0] > 3.));
    assert_eq!(b.pcm_contacts, [0; 2]);
    let tail = drums::measure(&s, &root, &p, 1., 2.).unwrap();
    assert_eq!(tail.frames[0].rms, b.frames[100].rms);
    s.channels[0].fader_db += 2.;
    s.channels[1].fader_db += 2.;
    let a = drums::measure(&s, &root, &p, 0., 2.).unwrap();
    for (b, a) in b.frames.iter().zip(&a.frames) {
        assert_eq!(b.mean_gr, a.mean_gr);
        assert_eq!(b.rms[2], a.rms[2]);
        if b.rms[4] > -160. {
            assert!(
                (a.rms[4] - b.rms[4] - 2.).abs() < 1e-6,
                "at {}: {} -> {}",
                b.seconds,
                b.rms[4],
                a.rms[4]
            );
        }
        assert_eq!(b.rms[10], a.rms[10]);
    }
    let out = root.join("analysis");
    drums::analyze(s.clone(), &root, &out, p.clone(), 0., 2.).unwrap();
    assert!(drums::analyze(s.clone(), &root, &out, p.clone(), 0., 2.).is_err());
    let mut bad = p.clone();
    bad.snare_file = "wrong.wav".into();
    assert!(bad.validate(&s).is_err());
    let out = root.join("correction");
    drums::correct(s, &root, &out, p, 0., 2.).unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn tone_probes_are_bounded_existing_eq_changes_and_relief_requires_measured_benefit() {
    use gigpies::automix::config::{self, EqBand, EqKind};
    let mut s = config::example();
    s.channels[0].eq = vec![EqBand {
        kind: EqKind::Bell,
        hz: 100.,
        q: 1.2,
        db: 3.5,
    }];
    s.channels[1].eq = vec![EqBand {
        kind: EqKind::HighShelf,
        hz: 5000.,
        q: std::f64::consts::FRAC_1_SQRT_2,
        db: 4.5,
    }];
    let p = drums::Policy {
        kick: 0,
        kick_file: s.channels[0].file.clone(),
        snare: 1,
        snare_file: s.channels[1].file.clone(),
        bass: 7,
        bass_file: s.channels[7].file.clone(),
        neutral_basis: "test".into(),
        current_emphasis_db: Some([0.; 2]),
        desired_emphasis_db: [2.; 2],
    };
    let probes = drums::tone_probes(&s, &p);
    assert_eq!(probes.len(), 2);
    assert_eq!(probes[0].settings.channels[0].eq[0].db, 2.);
    assert_eq!(probes[1].settings.channels[1].eq[0].db, 2.5);
    for (i, probe) in probes.iter().enumerate() {
        let mut changed = probe.settings.clone();
        changed.channels[i].eq = s.channels[i].eq.clone();
        assert_eq!(
            serde_json::to_value(changed).unwrap(),
            serde_json::to_value(&s).unwrap()
        );
    }
    let b = measurement(frames(false, false));
    let d = drums::diagnose(&b);
    assert!(!drums::relief_failures(&b, &b, &d, [true, false]).is_empty());
    let mut a = measurement(b.frames.clone());
    for f in &mut a.frames {
        f.mean_gr[0] -= 0.5;
    }
    assert!(drums::relief_failures(&b, &a, &d, [true, false]).is_empty());
    for f in &mut a.frames {
        if f.seconds >= 12. {
            f.rms[2] += 1.;
        }
    }
    assert!(!drums::relief_failures(&b, &a, &d, [true, false]).is_empty());
}

#[test]
fn isolated_pcm_contact_is_reported_while_repeated_contact_blocks() {
    let mut b = measurement(frames(false, false));
    b.pcm_contacts = [100, 0];
    let d = drums::diagnose(&b);
    assert!(drums::validate_change(&b, &b, &d, [0.; 2]).accepted);
    for i in 0..3 {
        b.spectra.push(drums::Spectral {
            seconds: i as f64,
            bands: vec![],
            bass_note: None,
            pcm_contact_fraction: [0.001, 0.],
        });
    }
    assert!(!drums::validate_change(&b, &b, &d, [0.; 2]).accepted);
    b.spectra.truncate(1);
    assert!(drums::validate_change(&b, &b, &d, [0.; 2]).accepted);
}

#[test]
fn a_weak_precursor_cannot_lock_out_a_strong_hit_and_flams_stay_ambiguous() {
    let mut f = frames(false, false);
    for x in &mut f {
        x.role_band = [-90.; 2];
        x.rms = [-90.; 15];
    }
    for i in [40, 1240] {
        f[i].role_band = [-35.; 2];
        f[i].rms = [-35.; 15];
        f[i + 6].role_band = [-10.; 2];
        f[i + 6].rms = [-10.; 15];
    }
    let e = drums::events(&f, 1);
    assert_eq!(e.len(), 2);
    assert_eq!(e[0].frame, 46);
    assert!(e[0].ambiguous_onset);
    assert!(e[0].rise_candidates_seconds.contains(&0.4));
    assert!(e[0].recovery_gr_db.is_none());
    assert_eq!(e[1].frame, 1246);
    // No out-of-bounds read from an anchor near the incomplete trailing region.
    let n = f.len();
    f[n - 25].role_band = [-5.; 2];
    let _ = drums::events(&f, 0);
}

#[test]
fn recovery_ends_at_next_precursor_even_when_next_peak_is_later() {
    let mut f = frames(false, false);
    for x in &mut f {
        x.role_band = [-90.; 2];
        x.rms = [-90.; 15];
    }
    for i in [40, 1240] {
        f[i].role_band = [-10.; 2];
        f[i + 20].role_band = [-35.; 2];
        f[i + 26].role_band = [-10.; 2];
    }
    let e = drums::events(&f, 1);
    assert_eq!(e[0].frame, 40);
    assert_eq!(e[1].frame, 66);
    assert!((e[0].next_seconds.unwrap() - 0.20).abs() < 1e-8);
    assert!(e[0].recovery_gr_db.is_none());
    assert!(e[0].tail_attack_db.is_none());
    assert!(e[1].ambiguous_onset);
}
