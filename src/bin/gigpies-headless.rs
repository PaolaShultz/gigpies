//! Explicit Linux-only synthetic engine. Opens only the requested private Unix socket.
#[cfg(target_os = "linux")]
fn main() {
    if let Err(error) = run() {
        eprintln!("gigpies-headless: {error}");
        std::process::exit(1);
    }
}
#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("gigpies-headless requires Linux Unix peer credentials");
    std::process::exit(1);
}
#[cfg(target_os = "linux")]
static STOP: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
#[cfg(target_os = "linux")]
extern "C" fn stop(_: libc::c_int) {
    STOP.store(true, std::sync::atomic::Ordering::Relaxed);
}
#[cfg(target_os = "linux")]
fn run() -> Result<(), String> {
    use gigpies::{local_audio::LocalAudio, show::Counter};
    let mut directory = None;
    let mut show = None;
    let mut epoch = None;
    let mut ticks = None;
    let mut analysis = false;
    let mut fouraux = false;
    let mut modules: Option<String> = None;
    let mut topology = None;
    let mut arm = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--help" {
            println!(
                "Usage: gigpies-headless --directory ABS_PRIVATE_0700_DIR --show UUID --epoch POSITIVE_INTEGER [--ticks COUNT] [--synthetic-source fouraux] [--analysis] [--modules ABS_HASH_MANIFEST] [--topology ABS_JSON] [--arm-synthetic]\nSynthetic configurable engine; defaults to explicit legacy eight-input/two-monitor compatibility. Expanded topology starts disarmed. Same-UID audio.sock only; no physical devices. New show/epoch identity must be supplied explicitly."
            );
            return Ok(());
        }
        if arg == "--arm-synthetic" {
            arm = true;
            continue;
        }
        if arg == "--analysis" {
            analysis = true;
            continue;
        }
        let value = args.next().ok_or("missing option value")?;
        match arg.as_str() {
            "--topology" => topology = Some(value),
            "--modules" => modules = Some(value),
            "--directory" => directory = Some(value),
            "--show" => show = Some(value),
            "--epoch" => epoch = Some(value.parse::<u64>().map_err(|e| e.to_string())?),
            "--synthetic-source" => {
                if value != "fouraux" {
                    return Err("unknown synthetic source".into());
                }
                fouraux = true;
            }
            "--ticks" => ticks = Some(value.parse::<u64>().map_err(|e| e.to_string())?),
            _ => return Err("unknown option".into()),
        }
    }
    let directory = directory.ok_or("explicit directory required")?;
    let show = show.ok_or("explicit show required")?;
    let epoch = epoch.ok_or("explicit new epoch required")?;
    if ticks.is_some_and(|n| n > 1_000_000) {
        return Err("ticks limit".into());
    }
    let topology = if let Some(path) = topology {
        serde_json::from_slice::<gigpies::topology::EngineTopology>(
            &std::fs::read(path).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?
    } else {
        gigpies::topology::EngineTopology::legacy()
    };
    let capability_version = if topology.is_legacy() { 1 } else { 2 };
    let mut server = LocalAudio::bind_configured(
        std::path::Path::new(&directory),
        "audio.sock",
        &show,
        Counter(epoch),
        topology,
    )?;
    if arm {
        server.rearm()?;
    }
    if fouraux {
        server.enable_synthetic_fouraux()?;
    }
    if analysis {
        server.enable_analysis(std::path::Path::new(&directory))?;
    }
    let module_enabled = modules.is_some();
    if let Some(manifest) = modules {
        #[cfg(feature = "hardware-host")]
        server.enable_modules(std::path::Path::new(&manifest))?;
        #[cfg(not(feature = "hardware-host"))]
        {
            let _ = manifest;
            return Err("--modules requires hardware-host build; no devices are opened".into());
        }
    }
    // SAFETY: handlers perform only an atomic store; installed in this explicit process.
    unsafe {
        libc::signal(libc::SIGINT, stop as *const () as libc::sighandler_t);
        libc::signal(libc::SIGTERM, stop as *const () as libc::sighandler_t);
    }
    let path = if module_enabled {
        "GP05-modules:1 fixed owner graph; query module_status; physical unverified"
    } else {
        "offline-unprotected; modules unavailable"
    };
    eprintln!(
        "GP03-rendered:{capability_version} synthetic show={show} epoch={epoch} socket={directory}/audio.sock {path}; no physical output arm"
    );
    let started = std::time::Instant::now();
    let mut count = 0_u64;
    while !STOP.load(std::sync::atomic::Ordering::Relaxed)
        && ticks.is_none_or(|limit| count < limit)
    {
        let now = u64::try_from(started.elapsed().as_millis()).map_err(|_| "elapsed exhausted")?;
        server.tick(now)?;
        count = count.checked_add(1).ok_or("tick exhausted")?;
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    Ok(())
}
