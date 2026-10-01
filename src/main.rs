use std::{ffi::OsStr, path::Path, process::ExitCode};

fn run() -> gigpies::inventory::Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    match args.as_slice() {
        [] => println!(
            "GigPies {} — experimental offline foundation\nRun gigpies --help for commands.",
            env!("CARGO_PKG_VERSION")
        ),
        [arg] if arg == "--help" || arg == "-h" => println!(
            "GigPies — experimental offline foundation\n\nUsage:\n  gigpies inspect <WAV-or-directory>\n  gigpies --version\n\ninspect prints JSON WAV header metadata; directories are nonrecursive.\nIt does not analyze samples, mix, play audio, or open hardware."
        ),
        [arg] if arg == "--version" || arg == "-V" => {
            println!("gigpies {}", env!("CARGO_PKG_VERSION"))
        }
        [command, path] if command == OsStr::new("inspect") => {
            let report = gigpies::inventory::inspect(Path::new(path))?;
            println!("{}", serde_json::to_string_pretty(&report)?);
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
