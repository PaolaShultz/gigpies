//! Explicit bounded authenticated provider/Brain; physical PCM requires a CLI gate.
#[tokio::main(flavor = "current_thread")]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("gigpies-remote: {error}");
        std::process::exit(1);
    }
}
async fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let physical_authorized = args.len() == 3 && args[2] == "--activate-physical";
    if (args.len() != 2 && !physical_authorized) || args[0] != "--config" {
        return Err(
            "usage: gigpies-remote --config PRIVATE.json [--activate-physical] (explicit provider or brain mode)".into(),
        );
    }
    let path = std::path::Path::new(&args[1]);
    let metadata = std::fs::metadata(path).map_err(|e| e.to_string())?;
    if !metadata.is_file() || metadata.len() > 65_536 {
        return Err("config file capacity/type".into());
    }
    let config = serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let report = gigpies::remote::run_config_authorized(config, physical_authorized).await?;
    println!(
        "{}",
        serde_json::json!({"mode":report["mode"],"fault":report["fault"],"software_only":report["software_only"]})
    );
    if !report["fault"].is_null() {
        return Err("bounded run reported a fault; retain its report".into());
    }
    Ok(())
}
