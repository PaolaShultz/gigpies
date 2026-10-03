//! Explicit opt-in hardware bench; ordinary GigPies never opens audio devices.
use gigpies::host::{
    adapters::Result,
    device::{self, PaConfig},
    network::{self, BrainConfig},
};
use serde::Deserialize;
use std::{io::Write, path::PathBuf};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Brain {
    bind: String,
    peer: String,
    control_bind: String,
    control_peer: String,
    epoch: u64,
    return_delay_frames: u32,
    seconds: u32,
    lease: u64,
    faults: bool,
    library: PathBuf,
    report: PathBuf,
}
fn run() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 1 || args.iter().any(|s| s == "--help") {
        println!(
            "gigpies-hardware pa|brain CONFIG.json\nExplicit selected-device bench. Requires independently built owner libraries, explicit endpoints, a fresh epoch and private output paths. Opens hardware only in pa mode. No default device or microphone monitor route."
        );
        return Ok(());
    }
    if args.len() != 3 {
        return Err("usage: gigpies-hardware pa|brain CONFIG.json".into());
    }
    let bytes = std::fs::read(&args[2])?;
    if bytes.len() > 16384 {
        return Err("configuration too large".into());
    }
    let (report, result) = match args[1].as_str() {
        "pa" => {
            let c: PaConfig = serde_json::from_slice(&bytes)?;
            let report = c.report.clone();
            let file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(report)?;
            (file, device::run(c)?)
        }
        "brain" => {
            let c: Brain = serde_json::from_slice(&bytes)?;
            if !(1..=660).contains(&c.seconds) {
                return Err("duration must be 1..660 seconds".into());
            }
            network::validate_addresses(&c.bind, &c.peer)?;
            network::validate_addresses(&c.control_bind, &c.control_peer)?;
            let file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(c.report)?;
            let result = network::brain(BrainConfig {
                bind: &c.bind,
                peer: &c.peer,
                control_bind: &c.control_bind,
                control_peer: &c.control_peer,
                epoch: c.epoch,
                return_delay_frames: c.return_delay_frames,
                seconds: f64::from(c.seconds),
                lease: c.lease,
                faults: c.faults,
                library: &c.library,
            })?;
            (file, result)
        }
        _ => return Err("unknown role".into()),
    };
    let failed = result.get("fault").is_some_and(|v| !v.is_null())
        || result
            .get("record_finalization")
            .is_some_and(|v| v.as_i64() != Some(0));
    let mut report = report;
    serde_json::to_writer_pretty(&mut report, &result)?;
    report.write_all(b"\n")?;
    report.sync_all()?;
    if failed {
        return Err("run retained a fault; inspect private report and take".into());
    }
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
