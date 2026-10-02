use std::{ffi::OsStr, path::Path, process::ExitCode};

fn run() -> gigpies::inventory::Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    match args.as_slice() {
        [] => println!(
            "GigPies {} — experimental offline automixer\nRun gigpies --help for commands.",
            env!("CARGO_PKG_VERSION")
        ),
        [arg] if arg == "--help" || arg == "-h" => println!(
            "GigPies — experimental offline automixer\n\nUsage:\n  gigpies inspect <WAV-or-directory>\n  gigpies preset <new-settings.json>\n  gigpies soundcheck <settings.json> <source-dir> <new-output-dir> <finish-seconds>\n  gigpies render <prepared.json> <source-dir> <new-output-dir>\n  gigpies fx-preset <prepared.json> <new-fx-settings.json>\n  gigpies finish <prepared-fx.json> <source-dir> <new-output-dir> [policy.json]\n  gigpies unity-pass <settings.json> <source-dir> <new-output-dir>\n  gigpies balance-analyze <settings.json> <source-dir> <new-output-dir> [policy.json]\n  gigpies balance-pass <settings.json> <source-dir> <new-output-dir> [policy.json]\n  gigpies tone-pass <settings.json> <source-dir> <new-output-dir> <tone-policy.json>\n  gigpies tone-analyze <settings.json> <source-dir> <new-output-dir> <tone-policy.json>\n  gigpies source-pass <settings.json> <source-dir> <new-output-dir> <source-policy.json>\n  gigpies source-analyze <settings.json> <source-dir> <new-output-dir> <source-policy.json>\n  gigpies compare <old.wav> <new.wav> <new-output-dir>\n  gigpies --version\n\ninspect prints JSON WAV header metadata; directories are nonrecursive.\nsoundcheck and render write local audio without playback or hardware access."
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
        [command, input, output] if command == "fx-preset" => {
            let mut session = serde_json::from_reader(std::fs::File::open(input)?)?;
            gigpies::automix::effects::add_pass(&mut session)?;
            gigpies::automix::write_json(Path::new(output), &session)?;
        }
        [command, config, root, out, seconds] if command == "soundcheck" => {
            let session = serde_json::from_reader(std::fs::File::open(config)?)?;
            let seconds = seconds.to_str().ok_or("invalid seconds")?.parse()?;
            gigpies::automix::run(session, Path::new(root), Path::new(out), Some(seconds))?;
        }
        [command, config, root, out, policy]
            if command == "source-pass" || command == "source-analyze" =>
        {
            let session = serde_json::from_reader(std::fs::File::open(config)?)?;
            let policy = serde_json::from_reader(std::fs::File::open(policy)?)?;
            gigpies::automix::expert::run(
                session,
                Path::new(root),
                Path::new(out),
                policy,
                command == "source-pass",
            )?;
        }
        [command, config, root, out, policy]
            if command == "tone-pass" || command == "tone-analyze" =>
        {
            let session = serde_json::from_reader(std::fs::File::open(config)?)?;
            let policy = serde_json::from_reader(std::fs::File::open(policy)?)?;
            gigpies::automix::tone::run(
                session,
                Path::new(root),
                Path::new(out),
                policy,
                command == "tone-pass",
            )?;
        }
        [command, config, root, out, policy] if command == "finish" => {
            let session = serde_json::from_reader(std::fs::File::open(config)?)?;
            let policy = serde_json::from_reader(std::fs::File::open(policy)?)?;
            gigpies::automix::analysis::finish(session, Path::new(root), Path::new(out), policy)?;
        }
        [command, config, root, out, policy]
            if command == "balance-pass" || command == "balance-analyze" =>
        {
            let session = serde_json::from_reader(std::fs::File::open(config)?)?;
            let policy = serde_json::from_reader(std::fs::File::open(policy)?)?;
            gigpies::automix::balance::run(
                session,
                Path::new(root),
                Path::new(out),
                Some(policy),
                command == "balance-pass",
            )?;
        }
        [command, config, root, out]
            if command == "balance-pass" || command == "balance-analyze" =>
        {
            let session = serde_json::from_reader(std::fs::File::open(config)?)?;
            gigpies::automix::balance::run(
                session,
                Path::new(root),
                Path::new(out),
                None,
                command == "balance-pass",
            )?;
        }
        [command, config, root, out] if command == "unity-pass" => {
            let session = serde_json::from_reader(std::fs::File::open(config)?)?;
            gigpies::automix::unity::run(session, Path::new(root), Path::new(out))?;
        }
        [command, config, root, out] if command == "finish" => {
            let session = serde_json::from_reader(std::fs::File::open(config)?)?;
            gigpies::automix::analysis::finish(
                session,
                Path::new(root),
                Path::new(out),
                Default::default(),
            )?;
        }
        [command, old, new, out] if command == "compare" => {
            gigpies::automix::compare(Path::new(old), Path::new(new), Path::new(out))?;
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
