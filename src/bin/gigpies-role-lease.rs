//! Per-surface role lock holder. Does not enumerate or open hardware.
use gigpies::{
    roles::{
        Binding,
        native::{Lease, decode_acquire},
    },
    show::Counter,
};
use serde::Deserialize;
use std::io::{self, Write};
use std::os::fd::AsRawFd;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Command {
    format: String,
    version: u32,
    operation: String,
    generation: Counter,
    binding: Binding,
}
fn line(reader: &impl AsRawFd) -> Result<Option<Vec<u8>>, String> {
    let mut bytes = Vec::new();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    loop {
        let ms = deadline
            .saturating_duration_since(std::time::Instant::now())
            .as_millis();
        if ms == 0 {
            return Err("read_deadline".into());
        }
        let mut p = libc::pollfd {
            fd: reader.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        // SAFETY: live stdin fd and initialized one-byte buffer.
        if unsafe { libc::poll(&mut p, 1, ms as i32) } <= 0 {
            return Err("read_deadline".into());
        }
        let mut byte = 0u8;
        // SAFETY: buffer valid for one byte; poll established readable fd.
        let n = unsafe { libc::read(reader.as_raw_fd(), (&mut byte as *mut u8).cast(), 1) };
        if n == 0 {
            return if bytes.is_empty() {
                Ok(None)
            } else {
                Err("incomplete_frame".into())
            };
        }
        if n < 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        bytes.push(byte);
        if bytes.len() > 65536 {
            return Err("frame_capacity".into());
        }
        if byte == b'\n' {
            return Ok(Some(bytes));
        }
    }
}
fn emit(value: &impl serde::Serialize) -> Result<(), String> {
    let mut out = io::stdout().lock();
    let mut p = libc::pollfd {
        fd: out.as_raw_fd(),
        events: libc::POLLOUT,
        revents: 0,
    };
    // SAFETY: live stdout pollfd; replies are below PIPE_BUF.
    if unsafe { libc::poll(&mut p, 1, 2000) } <= 0 {
        return Err("write_deadline".into());
    }
    serde_json::to_writer(&mut out, value).map_err(|e| e.to_string())?;
    out.write_all(b"\n").map_err(|e| e.to_string())?;
    out.flush().map_err(|e| e.to_string())
}
fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() != 2 {
        return Err("usage: gigpies-role-lease ABSOLUTE_PRIVATE_DIRECTORY".into());
    }
    #[cfg(target_os = "linux")]
    {
        // SAFETY: process-local death signal; protects authority even if a descendant inherits a pipe.
        let parent = unsafe { libc::getppid() };
        if unsafe { libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM) } != 0
            || unsafe { libc::getppid() } != parent
            || parent == 1
        {
            return Err("parent_lifetime".into());
        }
    }
    // SAFETY: stdout is an owned process descriptor. Nonblocking writes fail closed instead of hanging.
    let flags = unsafe { libc::fcntl(libc::STDOUT_FILENO, libc::F_GETFL) };
    if flags < 0
        || unsafe { libc::fcntl(libc::STDOUT_FILENO, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0
    {
        return Err("stdout_flags".into());
    }
    let input = io::stdin().lock();
    let request = decode_acquire(&line(&input)?.ok_or("missing_acquire")?)?;
    let lease = Lease::acquire(std::path::Path::new(&args[1]), &request)?;
    emit(lease.grant())?;
    while let Some(bytes) = line(&input)? {
        let command: Command =
            gigpies::roles::native::decode_protocol(&bytes).map_err(|e| e.to_string())?;
        if command.format != "gigpies-role-command" || command.version != 1 {
            return Err("version".into());
        }
        match command.operation.as_str() {
            "verify" => {
                lease.verify(command.generation, &command.binding)?;
                emit(
                    &serde_json::json!({"format":"gigpies-role-verified","version":1,"lease":lease.grant(),"registry_generation":lease.registry_generation()?}),
                )?;
            }
            "release" => {
                lease.release(command.generation, &command.binding)?;
                emit(
                    &serde_json::json!({"format":"gigpies-role-released","version":1,"generation":command.generation}),
                )?;
                return Ok(());
            }
            "forget" => {
                if command.binding != lease.grant().binding {
                    return Err("identity".into());
                }
                let generation = lease.forget(command.generation)?;
                emit(
                    &serde_json::json!({"format":"gigpies-role-released","version":1,"generation":generation}),
                )?;
                return Ok(());
            }
            _ => return Err("operation".into()),
        }
    }
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
