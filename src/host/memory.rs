//! Optional process-local memory locking for the fresh hardware CLI process.
//!
//! The caller must own all memory-lock API use in this process for the complete
//! guard lifetime. Already-started workers may allocate, but must not call
//! mlock/munlock, mlockall/munlockall or mmap(MAP_LOCKED). Linux locks do not
//! stack, so no guard can safely merge independent owners of these APIs.
//!
//! Enter only during setup, after allocation/worker creation; restore after PCM
//! stops and before worker joins. MCL_FUTURE may make later allocation or stack
//! growth fail at RLIMIT_MEMLOCK. This module changes no resource limit and
//! does not promise to eliminate scheduler, driver or other latency outliers.
//!
//! References: Linux mlock(2), https://man7.org/linux/man-pages/man2/mlock.2.html;
//! Linux mm/mlock.c (mlockall errors precede apply_mlockall_flags) and mm/mmap.c
//! (future VM_LOCKED mappings are charged to mm->locked_vm, including ONFAULT).
use std::{
    fs::File,
    io::{self, Read},
    marker::PhantomData,
    rc::Rc,
    sync::atomic::{AtomicBool, Ordering},
};

static OWNED: AtomicBool = AtomicBool::new(false);

pub struct AudioMemoryLock {
    state: LockState,
    // Keep setup, explicit restoration and Drop on the owning audio thread.
    _thread: PhantomData<Rc<()>>,
}

impl AudioMemoryLock {
    /// Requires exclusive memory-lock API ownership in this fresh CLI process.
    /// VmLck and an anonymous mapping probe reject existing current/future locks.
    pub fn enter(requested: bool) -> io::Result<Option<Self>> {
        if !requested {
            return Ok(None);
        }
        if OWNED
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "process memory-lock guard already owns or still needs to restore locks",
            ));
        }
        match LockState::enter(&mut LinuxBackend) {
            Ok(state) => Ok(Some(Self {
                state,
                _thread: PhantomData,
            })),
            Err(error) => {
                // mlockall failure leaves the existing lock state unchanged.
                // In particular, never call munlockall after failed activation.
                OWNED.store(false, Ordering::Release);
                Err(error)
            }
        }
    }

    pub fn restore(&mut self) -> io::Result<()> {
        if self.state.active {
            self.state.restore(&mut LinuxBackend)?;
            OWNED.store(false, Ordering::Release);
        }
        Ok(())
    }
}

impl Drop for AudioMemoryLock {
    fn drop(&mut self) {
        // Explicit restoration exposes errors to the report. Drop retries only
        // pending restoration; failure leaves OWNED set to refuse another owner.
        let _ = self.restore();
    }
}

trait Backend {
    fn locked_kib(&mut self) -> io::Result<u64>;
    fn reject_future_lock(&mut self) -> io::Result<()>;
    fn lock_all(&mut self) -> io::Result<()>;
    fn unlock_all(&mut self) -> io::Result<()>;
}

struct LockState {
    active: bool,
}

impl LockState {
    fn enter(backend: &mut impl Backend) -> io::Result<Self> {
        if backend.locked_kib()? != 0 {
            return Err(existing_lock_error());
        }
        backend.reject_future_lock()?;
        // Catch a changed observed baseline before mutation. Other independent
        // memory-lock users are forbidden by the caller ownership contract.
        if backend.locked_kib()? != 0 {
            return Err(existing_lock_error());
        }
        backend.lock_all()?;
        Ok(Self { active: true })
    }

    fn restore(&mut self, backend: &mut impl Backend) -> io::Result<()> {
        if self.active {
            backend.unlock_all()?;
            self.active = false;
        }
        Ok(())
    }
}

fn existing_lock_error() -> io::Error {
    io::Error::new(
        io::ErrorKind::AlreadyExists,
        "preexisting process memory locks or future-lock policy; activation refused",
    )
}

fn parse_locked_kib(status: &str) -> io::Result<u64> {
    let invalid = || io::Error::new(io::ErrorKind::InvalidData, "invalid or missing proc VmLck");
    let mut value = None;
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("VmLck:") {
            if value.is_some() {
                return Err(invalid());
            }
            let mut fields = rest.split_ascii_whitespace();
            let number = fields.next().ok_or_else(invalid)?;
            if number.is_empty() || !number.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err(invalid());
            }
            value = Some(number.parse().map_err(|_| invalid())?);
            if fields.next() != Some("kB") || fields.next().is_some() {
                return Err(invalid());
            }
        }
    }
    value.ok_or_else(invalid)
}

pub(crate) fn process_locked_kib() -> io::Result<u64> {
    // Fail closed on unexpectedly large proc output; no unbounded read/allocation.
    let mut bytes = [0; 16 * 1024];
    let mut used = 0;
    let mut file = File::open("/proc/self/status")?;
    loop {
        if used == bytes.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "proc status exceeds bound",
            ));
        }
        let read = file.read(&mut bytes[used..])?;
        if read == 0 {
            break;
        }
        used += read;
    }
    let text = std::str::from_utf8(&bytes[..used])
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "proc status is not UTF-8"))?;
    parse_locked_kib(text)
}

struct LinuxBackend;

impl Backend for LinuxBackend {
    fn locked_kib(&mut self) -> io::Result<u64> {
        process_locked_kib()
    }

    fn reject_future_lock(&mut self) -> io::Result<()> {
        // A future-only policy can coexist with VmLck=0. A fresh ordinary RW
        // mapping receives VM_LOCKED and counts toward VmLck even with ONFAULT.
        // Unmap only our page; do not clear any existing future-lock policy.
        let mut probe = ProbePage::new()?;
        let checked = process_locked_kib().and_then(|locked| {
            if locked == 0 {
                Ok(())
            } else {
                Err(existing_lock_error())
            }
        });
        let removed = probe.remove();
        match (checked, removed) {
            (Ok(()), result) | (result, Ok(())) => result,
            (Err(check), Err(remove)) => Err(io::Error::new(
                remove.kind(),
                format!("{check}; removing private probe page also failed: {remove}"),
            )),
        }
    }

    fn lock_all(&mut self) -> io::Result<()> {
        // SAFETY: no pointers; ownership is established before this process-wide
        // operation, and both flags are defined for the Linux target.
        if unsafe { libc::mlockall(libc::MCL_CURRENT | libc::MCL_FUTURE) } != 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    fn unlock_all(&mut self) -> io::Result<()> {
        // SAFETY: called only following our successful mlockall, under the
        // exclusive fresh-process ownership contract. No pointers are passed.
        if unsafe { libc::munlockall() } != 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }
}

struct ProbePage {
    address: *mut libc::c_void,
    length: usize,
    active: bool,
}

impl ProbePage {
    fn new() -> io::Result<Self> {
        // SAFETY: _SC_PAGESIZE takes no pointer argument.
        let length = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
        if !(4096..=65536).contains(&length) || !(length as usize).is_power_of_two() {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "unsupported probe page size",
            ));
        }
        let length = length as usize;
        // SAFETY: anonymous non-fixed mapping; no existing address is replaced.
        let address = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                length,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                -1,
                0,
            )
        };
        if address == libc::MAP_FAILED {
            return Err(io::Error::last_os_error());
        }
        let probe = Self {
            address,
            length,
            active: true,
        };
        if address.is_null() {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "null probe mapping",
            ));
        }
        // SAFETY: the new private mapping is writable for at least one byte.
        // Touch before observation, also covering a prior MCL_ONFAULT policy.
        unsafe { address.cast::<u8>().write_volatile(0) };
        Ok(probe)
    }

    fn remove(&mut self) -> io::Result<()> {
        if self.active {
            // SAFETY: this is exactly the mapping created above, still owned.
            if unsafe { libc::munmap(self.address, self.length) } != 0 {
                return Err(io::Error::last_os_error());
            }
            self.active = false;
        }
        Ok(())
    }
}

impl Drop for ProbePage {
    fn drop(&mut self) {
        let _ = self.remove();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    #[derive(Default)]
    struct Fake {
        locked: VecDeque<io::Result<u64>>,
        fail_probe: bool,
        fail_lock: bool,
        unlock_failures: usize,
        calls: Vec<&'static str>,
    }
    impl Backend for Fake {
        fn locked_kib(&mut self) -> io::Result<u64> {
            self.calls.push("status");
            self.locked.pop_front().unwrap_or(Ok(0))
        }
        fn reject_future_lock(&mut self) -> io::Result<()> {
            self.calls.push("probe");
            if self.fail_probe {
                Err(existing_lock_error())
            } else {
                Ok(())
            }
        }
        fn lock_all(&mut self) -> io::Result<()> {
            self.calls.push("lock");
            if self.fail_lock {
                Err(io::Error::from_raw_os_error(libc::ENOMEM))
            } else {
                Ok(())
            }
        }
        fn unlock_all(&mut self) -> io::Result<()> {
            self.calls.push("unlock");
            if self.unlock_failures > 0 {
                self.unlock_failures -= 1;
                Err(io::Error::from_raw_os_error(libc::EINTR))
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn disabled_guard_does_not_claim_or_change_memory_locks() {
        assert!(AudioMemoryLock::enter(false).unwrap().is_none());
    }

    #[test]
    fn vm_lck_parser_requires_one_valid_explicit_unit() {
        assert_eq!(parse_locked_kib("Name:\ttest\nVmLck:\t0 kB\n").unwrap(), 0);
        assert_eq!(parse_locked_kib("VmLck: 4096 kB").unwrap(), 4096);
        for text in [
            "",
            "VmLck: -1 kB",
            "VmLck: +1 kB",
            "VmLck: 4 B",
            "VmLck: 1 kB extra",
            "VmLck: 0 kB\nVmLck: 0 kB",
        ] {
            assert!(parse_locked_kib(text).is_err());
        }
    }

    #[test]
    fn preexisting_current_future_or_changed_locks_are_never_unlocked() {
        let mut existing = Fake::default();
        existing.locked.push_back(Ok(4));
        assert!(LockState::enter(&mut existing).is_err());
        assert_eq!(existing.calls, ["status"]);
        let mut future = Fake {
            fail_probe: true,
            ..Fake::default()
        };
        assert!(LockState::enter(&mut future).is_err());
        assert_eq!(future.calls, ["status", "probe"]);
        let mut changed = Fake::default();
        changed.locked.extend([Ok(0), Ok(4)]);
        assert!(LockState::enter(&mut changed).is_err());
        assert_eq!(changed.calls, ["status", "probe", "status"]);
    }

    #[test]
    fn activation_failure_never_attempts_process_wide_rollback() {
        let mut backend = Fake {
            fail_lock: true,
            ..Fake::default()
        };
        assert_eq!(
            LockState::enter(&mut backend).err().unwrap().raw_os_error(),
            Some(libc::ENOMEM)
        );
        assert_eq!(backend.calls, ["status", "probe", "status", "lock"]);
    }

    #[test]
    fn unreadable_lock_baseline_fails_without_mutation() {
        let mut backend = Fake::default();
        backend
            .locked
            .push_back(Err(io::Error::from_raw_os_error(libc::EACCES)));
        assert_eq!(
            LockState::enter(&mut backend).err().unwrap().raw_os_error(),
            Some(libc::EACCES)
        );
        assert_eq!(backend.calls, ["status"]);
    }

    #[test]
    fn restoration_failure_stays_pending_and_success_is_idempotent() {
        let mut backend = Fake {
            unlock_failures: 1,
            ..Fake::default()
        };
        let mut state = LockState::enter(&mut backend).unwrap();
        assert!(state.restore(&mut backend).is_err());
        assert!(state.active);
        state.restore(&mut backend).unwrap();
        assert!(!state.active);
        state.restore(&mut backend).unwrap();
        assert_eq!(
            backend.calls,
            ["status", "probe", "status", "lock", "unlock", "unlock"]
        );
    }
}
