//! Opt-in scheduling for the current audio thread only. Workers already exist.
use std::{io, marker::PhantomData, rc::Rc};

pub struct AudioScheduling {
    state: SavedScheduling,
    // Restoration must happen on the thread that installed these settings.
    _thread: PhantomData<Rc<()>>,
}

struct SavedScheduling {
    policy: i32,
    parameter: libc::sched_param,
    affinity: libc::cpu_set_t,
    restore_policy: bool,
    restore_affinity: bool,
}

enum Change<'a> {
    Policy(i32, &'a libc::sched_param),
    Affinity(&'a libc::cpu_set_t),
}

fn apply_change(change: Change<'_>) -> io::Result<()> {
    // SAFETY: pid 0 addresses this calling thread. All pointers reference valid
    // local values, and the affinity size matches the supplied cpu_set_t.
    let result = unsafe {
        match change {
            Change::Policy(policy, parameter) => libc::sched_setscheduler(0, policy, parameter),
            Change::Affinity(affinity) => {
                libc::sched_setaffinity(0, size_of::<libc::cpu_set_t>(), affinity)
            }
        }
    };
    if result != 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

fn current_affinity() -> io::Result<libc::cpu_set_t> {
    // SAFETY: zero is a valid empty CPU mask, and the syscall fills this value.
    let mut affinity: libc::cpu_set_t = unsafe { std::mem::zeroed() };
    if unsafe { libc::sched_getaffinity(0, size_of::<libc::cpu_set_t>(), &mut affinity) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(affinity)
}

impl AudioScheduling {
    pub fn enter(priority: Option<i32>, cpu: Option<usize>) -> io::Result<Option<Self>> {
        if priority.is_none() && cpu.is_none() {
            return Ok(None);
        }
        if priority.is_some_and(|priority| priority != 20) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "only explicit FIFO priority 20 is supported",
            ));
        }
        if cpu.is_some_and(|cpu| cpu >= libc::CPU_SETSIZE as usize) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "audio CPU must fit CPU_SETSIZE",
            ));
        }
        let affinity = current_affinity()?;
        // SAFETY: the CPU index was bounded above and the mask is initialized.
        if cpu.is_some_and(|cpu| !unsafe { libc::CPU_ISSET(cpu, &affinity) }) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "audio CPU must belong to this thread's current allowed mask",
            ));
        }
        // SAFETY: pid 0 addresses this calling thread and the output pointer is
        // valid. All validation and snapshots precede either mutation.
        let policy = unsafe { libc::sched_getscheduler(0) };
        if policy < 0 {
            return Err(io::Error::last_os_error());
        }
        let mut parameter = libc::sched_param { sched_priority: 0 };
        if unsafe { libc::sched_getparam(0, &mut parameter) } != 0 {
            return Err(io::Error::last_os_error());
        }
        let mut guard = Self {
            state: SavedScheduling {
                policy,
                parameter,
                affinity,
                restore_policy: false,
                restore_affinity: false,
            },
            _thread: PhantomData,
        };
        // If installation and rollback both fail, dropping this guard retries
        // the remaining restoration before returning the installation error.
        guard.state.install(priority, cpu, apply_change)?;
        Ok(Some(guard))
    }

    pub fn restore(&mut self) -> io::Result<()> {
        self.state.restore_with(apply_change)
    }
}

impl SavedScheduling {
    fn install(
        &mut self,
        priority: Option<i32>,
        cpu: Option<usize>,
        mut change: impl FnMut(Change<'_>) -> io::Result<()>,
    ) -> io::Result<()> {
        if let Some(cpu) = cpu {
            // SAFETY: the caller validated the CPU index before any mutation.
            let mut requested: libc::cpu_set_t = unsafe { std::mem::zeroed() };
            unsafe { libc::CPU_SET(cpu, &mut requested) };
            change(Change::Affinity(&requested))?;
            self.restore_affinity = true;
        }
        if let Some(priority) = priority {
            let requested = libc::sched_param {
                sched_priority: priority,
            };
            if let Err(error) = change(Change::Policy(libc::SCHED_FIFO, &requested)) {
                return match self.restore_with(&mut change) {
                    Ok(()) => Err(error),
                    Err(rollback) => Err(io::Error::new(
                        error.kind(),
                        format!("{error}; restoring thread settings also failed: {rollback}"),
                    )),
                };
            }
            self.restore_policy = true;
        }
        Ok(())
    }

    fn restore_with(
        &mut self,
        mut change: impl FnMut(Change<'_>) -> io::Result<()>,
    ) -> io::Result<()> {
        let mut first_error = None;
        if self.restore_policy {
            match change(Change::Policy(self.policy, &self.parameter)) {
                Ok(()) => self.restore_policy = false,
                Err(error) => first_error = Some(error),
            }
        }
        // Always attempt both restorations, even if the first failed. Keep only
        // failed settings pending so explicit retries and Drop are idempotent.
        if self.restore_affinity {
            match change(Change::Affinity(&self.affinity)) {
                Ok(()) => self.restore_affinity = false,
                Err(error) if first_error.is_none() => first_error = Some(error),
                Err(_) => {}
            }
        }
        first_error.map_or(Ok(()), Err)
    }
}

impl Drop for AudioScheduling {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

/// Current audio thread's consumed CPU time, independent of wall-clock waits.
pub(crate) fn thread_cpu_ns() -> io::Result<u64> {
    let mut time = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: the output pointer is valid and this clock addresses the caller.
    if unsafe { libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, &mut time) } != 0 {
        return Err(io::Error::last_os_error());
    }
    let invalid = || io::Error::new(io::ErrorKind::InvalidData, "invalid thread CPU timestamp");
    let seconds = u64::try_from(time.tv_sec).map_err(|_| invalid())?;
    let nanos = u64::try_from(time.tv_nsec).map_err(|_| invalid())?;
    if nanos >= 1_000_000_000 {
        return Err(invalid());
    }
    seconds
        .checked_mul(1_000_000_000)
        .and_then(|seconds| seconds.checked_add(nanos))
        .ok_or_else(invalid)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn saved() -> SavedScheduling {
        let mut affinity = unsafe { std::mem::zeroed() };
        unsafe { libc::CPU_SET(1, &mut affinity) };
        unsafe { libc::CPU_SET(3, &mut affinity) };
        SavedScheduling {
            policy: libc::SCHED_OTHER,
            parameter: libc::sched_param { sched_priority: 0 },
            affinity,
            restore_policy: false,
            restore_affinity: false,
        }
    }

    fn describe(change: Change<'_>) -> String {
        match change {
            Change::Policy(policy, parameter) => {
                format!("policy:{policy}:{}", parameter.sched_priority)
            }
            Change::Affinity(mask) => {
                let cpus: Vec<_> = (0..libc::CPU_SETSIZE as usize)
                    .filter(|cpu| unsafe { libc::CPU_ISSET(*cpu, mask) })
                    .collect();
                format!("affinity:{cpus:?}")
            }
        }
    }

    #[test]
    fn absent_or_invalid_requests_leave_current_thread_unchanged() {
        let before = unsafe { libc::sched_getscheduler(0) };
        let affinity = current_affinity().unwrap();
        let mut parameter = libc::sched_param { sched_priority: 0 };
        assert_eq!(unsafe { libc::sched_getparam(0, &mut parameter) }, 0);
        assert!(AudioScheduling::enter(None, None).unwrap().is_none());
        for priority in [0, 1, 19, 21, 99] {
            assert_eq!(
                AudioScheduling::enter(Some(priority), Some(0))
                    .err()
                    .unwrap()
                    .kind(),
                io::ErrorKind::InvalidInput
            );
        }
        let unavailable = (0..libc::CPU_SETSIZE as usize)
            .find(|cpu| !unsafe { libc::CPU_ISSET(*cpu, &affinity) });
        for cpu in [
            Some(libc::CPU_SETSIZE as usize),
            Some(usize::MAX),
            unavailable,
        ]
        .into_iter()
        .flatten()
        {
            assert_eq!(
                AudioScheduling::enter(Some(20), Some(cpu))
                    .err()
                    .unwrap()
                    .kind(),
                io::ErrorKind::InvalidInput
            );
        }
        assert_eq!(unsafe { libc::sched_getscheduler(0) }, before);
        let mut after = libc::sched_param { sched_priority: 0 };
        assert_eq!(unsafe { libc::sched_getparam(0, &mut after) }, 0);
        assert_eq!(parameter.sched_priority, after.sched_priority);
        assert_eq!(
            describe(Change::Affinity(&affinity)),
            describe(Change::Affinity(&current_affinity().unwrap()))
        );
    }

    #[test]
    fn installation_restores_exact_settings_and_only_requested_changes() {
        for (priority, cpu) in [(Some(20), None), (None, Some(3)), (Some(20), Some(3))] {
            let mut state = saved();
            state.policy = libc::SCHED_RR;
            state.parameter.sched_priority = 7;
            let mut changes = Vec::new();
            state
                .install(priority, cpu, |change| {
                    changes.push(describe(change));
                    Ok(())
                })
                .unwrap();
            let mut expected = Vec::new();
            if cpu.is_some() {
                expected.push("affinity:[3]".to_owned());
            }
            if priority.is_some() {
                expected.push(format!("policy:{}:20", libc::SCHED_FIFO));
            }
            assert_eq!(changes, expected);
            changes.clear();
            state
                .restore_with(|change| {
                    changes.push(describe(change));
                    Ok(())
                })
                .unwrap();
            expected.clear();
            if priority.is_some() {
                expected.push(format!("policy:{}:7", libc::SCHED_RR));
            }
            if cpu.is_some() {
                expected.push("affinity:[1, 3]".to_owned());
            }
            assert_eq!(changes, expected);
            state.restore_with(|_| panic!("already restored")).unwrap();
        }
    }

    #[test]
    fn failed_first_mutation_never_changes_policy_or_requests_rollback() {
        let mut state = saved();
        let mut calls = 0;
        let error = state
            .install(Some(20), Some(3), |change| {
                calls += 1;
                assert!(matches!(change, Change::Affinity(_)));
                Err(io::Error::from_raw_os_error(libc::EINVAL))
            })
            .unwrap_err();
        assert_eq!(calls, 1);
        assert_eq!(error.raw_os_error(), Some(libc::EINVAL));
        assert!(!state.restore_policy && !state.restore_affinity);
        state.restore_with(|_| panic!("nothing changed")).unwrap();
    }

    #[test]
    fn failed_second_mutation_rolls_back_affinity_and_retains_failed_rollback() {
        for fail_rollback in [false, true] {
            let mut state = saved();
            let mut changes = Vec::new();
            let error = state
                .install(Some(20), Some(3), |change| {
                    changes.push(describe(change));
                    match changes.len() {
                        2 => Err(io::Error::from_raw_os_error(libc::EPERM)),
                        3 if fail_rollback => Err(io::Error::from_raw_os_error(libc::EINVAL)),
                        _ => Ok(()),
                    }
                })
                .unwrap_err();
            assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
            assert_eq!(
                changes,
                [
                    "affinity:[3]".to_owned(),
                    format!("policy:{}:20", libc::SCHED_FIFO),
                    "affinity:[1, 3]".to_owned(),
                ]
            );
            assert!(!state.restore_policy);
            assert_eq!(state.restore_affinity, fail_rollback);
            state
                .restore_with(|change| {
                    assert!(fail_rollback);
                    assert_eq!(describe(change), "affinity:[1, 3]");
                    Ok(())
                })
                .unwrap();
        }
    }

    #[test]
    fn restoration_attempts_both_returns_first_error_and_retries_failures() {
        let mut state = saved();
        state.restore_policy = true;
        state.restore_affinity = true;
        let mut calls = 0;
        let error = state
            .restore_with(|_| {
                calls += 1;
                Err(io::Error::from_raw_os_error(if calls == 1 {
                    libc::EPERM
                } else {
                    libc::EINVAL
                }))
            })
            .unwrap_err();
        assert_eq!(calls, 2);
        assert_eq!(error.raw_os_error(), Some(libc::EPERM));
        assert!(state.restore_policy && state.restore_affinity);
        state
            .restore_with(|change| match change {
                Change::Policy(..) => Ok(()),
                Change::Affinity(_) => Err(io::Error::from_raw_os_error(libc::EINVAL)),
            })
            .unwrap_err();
        assert!(!state.restore_policy && state.restore_affinity);
        state
            .restore_with(|change| {
                assert!(matches!(change, Change::Affinity(_)));
                Ok(())
            })
            .unwrap();
        state.restore_with(|_| panic!("already restored")).unwrap();
    }

    #[test]
    fn thread_cpu_clock_is_monotonic() {
        let first = thread_cpu_ns().unwrap();
        let second = thread_cpu_ns().unwrap();
        assert!(second >= first);
    }
}
