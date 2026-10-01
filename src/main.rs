use std::{ffi::OsStr, path::Path, process::ExitCode};

fn run() -> gigpies::inventory::Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    match args.as_slice() {
        [] => println!(
            "GigPies {} — experimental offline automixer\nRun gigpies --help for commands.",
            env!("CARGO_PKG_VERSION")
        ),
        [arg] if arg == "--help" || arg == "-h" => println!(
            "GigPies — experimental offline automixer\n\nUsage:\n  gigpies inspect <WAV-or-directory>\n  gigpies preset <new-settings.json>\n  gigpies soundcheck <settings.json> <source-dir> <new-output-dir> <finish-seconds>\n  gigpies render <prepared.json> <source-dir> <new-output-dir>\n  gigpies --version\n\ninspect prints JSON WAV header metadata; directories are nonrecursive.\nsoundcheck and render write local audio without playback or hardware access."
        ),
        [arg] if arg == "--version" || arg == "-V" => {
            println!("gigpies {}", env!("CARGO_PKG_VERSION"))
        }
        [command, path] if command == OsStr::new("inspect") => {
            let report = gigpies::inventory::inspect(Path::new(path))?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        [command, path] if command == "preset" => {
            gigpies::automix::write_json(Path::new(path), &gigpies::automix::config::example())?;
        }
        [command, config, root, out, seconds] if command == "soundcheck" => {
            let session = serde_json::from_reader(std::fs::File::open(config)?)?;
            let seconds = seconds.to_str().ok_or("invalid seconds")?.parse()?;
            gigpies::automix::run(session, Path::new(root), Path::new(out), Some(seconds))?;
        }
        [command, config, root, out] if command == "render" => {
            let session = serde_json::from_reader(std::fs::File::open(config)?)?;
            gigpies::automix::run(session, Path::new(root), Path::new(out), None)?;
        }
        _ => return Err("unknown command or arguments; run gigpies --help".into()),
    }
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("gigpies: {error}");
            ExitCode::FAILURE
        }
    }
}
