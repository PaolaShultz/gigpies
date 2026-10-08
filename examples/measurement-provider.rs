//! Explicit bounded device-free PA measurement/Desk interoperability witness.
#[cfg(feature = "hardware-host")]
fn main() {
    if let Err(e) = run() {
        eprintln!("measurement-provider: {e}");
        std::process::exit(1)
    }
}
#[cfg(not(feature = "hardware-host"))]
fn main() {
    eprintln!("measurement-provider requires hardware-host build; never opens hardware");
    std::process::exit(2)
}
#[cfg(feature = "hardware-host")]
static STOP: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
#[cfg(feature = "hardware-host")]
extern "C" fn stop(_: libc::c_int) {
    STOP.store(true, std::sync::atomic::Ordering::Relaxed);
}
#[cfg(feature = "hardware-host")]
fn run() -> Result<(), Box<dyn std::error::Error>> {
    use gigpies::{
        host::pa_v2::Pa, local_audio::LocalAudio, measurement_owner::Config,
        module_graph::Manifest, show::Counter, topology::EngineTopology,
    };
    use serde_json::json;
    use std::{
        io::Write,
        path::PathBuf,
        time::{Duration, Instant},
    };
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 6 {
        return Err("usage: measurement-provider DIR MODULE_MANIFEST OWNER_CONFIG GRAPH_JSON READY_JSON DURATION_MS".into());
    }
    let dir = PathBuf::from(&args[0]);
    let manifest_path = PathBuf::from(&args[1]);
    let owner: Config = serde_json::from_slice(&std::fs::read(&args[2])?)?;
    let mut graph = std::fs::read_to_string(&args[3])?;
    let ready = PathBuf::from(&args[4]);
    let duration = args[5].parse::<u64>()?;
    if !(1000..=120000).contains(&duration) || ready.parent() != Some(dir.as_path()) {
        return Err("bounded duration/private ready path required".into());
    }
    let manifest = Manifest::load(&manifest_path)?;
    let mut topology = EngineTopology::software(16, 2, 2)?;
    topology.capture_channels = 17;
    topology.measurement_slots = vec![16];
    let mut host = LocalAudio::bind_configured(
        &dir,
        "audio.sock",
        "11111111-1111-4111-8111-111111111111",
        Counter(1),
        topology,
    )?;
    host.enable_modules(&manifest_path)?;
    host.configure_pa(graph.as_bytes())?;
    host.enable_software_measurement(owner.clone())?;
    // Admit only the software clock; keep mixer and PA output delivery quiesced.
    host.engine_mut().rearm()?;
    host.engine_mut().mute_outputs()?;
    let mut virtual_pa = Pa::load(&manifest.pa.library, graph.as_bytes(), 1, 0)?;
    if virtual_pa.rearm(1, 0) != 0 {
        return Err("virtual PA rearm".into());
    }
    let mut ready_file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&ready)?;
    ready_file.write_all(serde_json::to_string(&json!({"ready":true,"software_only":true,"pid":std::process::id(),"socket":dir.join("audio.sock"),"show_id":"11111111-1111-4111-8111-111111111111","epoch":"1","owner_sha256":owner.owner_sha256,"basis":host.measurement_current_basis()?})).unwrap().as_bytes())?;
    // SAFETY: handlers only set the atomic stop flag for this explicitly owned witness.
    unsafe {
        libc::signal(libc::SIGTERM, stop as *const () as libc::sighandler_t);
        libc::signal(libc::SIGINT, stop as *const () as libc::sighandler_t);
    }
    let started = Instant::now();
    let mut noise = 17_u64;
    let mut input = [0.; 96];
    let mut mic = [0.; 96];
    let mut capture = vec![0.; 48 * 17];
    let mut playback = vec![0.; host.topology().playback_channels * 48];
    let mut changed_at = 0;
    let mut equal_error = 0_f64;
    let mut checked = 0_u64;
    let mut evidence = Vec::new();
    let mut selected = 0;
    while started.elapsed().as_millis() < duration as u128
        && !STOP.load(std::sync::atomic::Ordering::Relaxed)
    {
        let frame = host.frame();
        let basis = host.measurement_current_basis()?;
        if basis.configuration_json != graph {
            evidence.push(json!({"configuration_json":graph,"settled_pair_max_difference":equal_error,"compared_samples":checked}));
            graph = basis.configuration_json;
            virtual_pa = Pa::load(&manifest.pa.library, graph.as_bytes(), 1, frame)?;
            if virtual_pa.rearm(1, frame) != 0 {
                return Err("virtual replacement rearm".into());
            }
            changed_at = frame;
            equal_error = 0.;
            checked = 0;
        }
        if let Some(index) = host.measurement_requested_output() {
            if index >= 2 {
                return Err("witness only has two virtual outputs".into());
            }
            selected = index;
        }
        for f in 0..48 {
            noise ^= noise << 13;
            noise ^= noise >> 7;
            noise ^= noise << 17;
            let x = (noise as i64 as f64 / i64::MAX as f64 * 0.03 * 8388608.).round() / 8388608.;
            input[f * 2] = x;
            input[f * 2 + 1] = x;
            for channel in 0..16 {
                capture[f * 17 + channel] = x;
            }
        }
        if virtual_pa.process(&input, &mut mic, 1, frame) != 0 {
            return Err("virtual PA render".into());
        }
        for f in 0..48 {
            capture[f * 17 + 16] = mic[f * 2 + selected];
            if frame >= changed_at + 1024 {
                equal_error = equal_error.max((mic[f * 2] - mic[f * 2 + 1]).abs());
                checked += 1;
            }
        }
        host.tick_with_capture(
            started.elapsed().as_millis() as u64,
            1,
            frame,
            &capture,
            &mut playback,
        )?;
        // Returned buffers are never submitted to an audio device, even after explicit UI rearm.
        std::thread::sleep(Duration::from_millis(1));
    }
    evidence.push(json!({"configuration_json":graph,"settled_pair_max_difference":equal_error,"compared_samples":checked}));
    let final_path = ready.with_extension("final.json");
    let mut final_file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(final_path)?;
    final_file.write_all(serde_json::to_string_pretty(&json!({"software_only":true,"frame":host.frame().to_string(),"graphs":evidence,"snapshot":host.measurement_snapshot()}))?.as_bytes())?;
    Ok(())
}
