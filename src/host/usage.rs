//! Calling-thread fault and scheduling counters, sampled outside DSP calls.
use serde::Serialize;
use std::{io, mem::MaybeUninit};

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct ThreadUsage {
    pub minor_faults: u64,
    pub major_faults: u64,
    pub voluntary_switches: u64,
    pub involuntary_switches: u64,
}

impl ThreadUsage {
    pub fn sample() -> io::Result<Self> {
        let mut raw = MaybeUninit::<libc::rusage>::uninit();
        // SAFETY: getrusage initializes the structure on success. RUSAGE_THREAD
        // observes only the caller and changes no scheduling or memory settings.
        if unsafe { libc::getrusage(libc::RUSAGE_THREAD, raw.as_mut_ptr()) } != 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: the successful call above initialized every rusage field.
        let raw = unsafe { raw.assume_init() };
        let convert =
            |value| u64::try_from(value).map_err(|_| io::Error::from_raw_os_error(libc::ERANGE));
        Ok(Self {
            minor_faults: convert(raw.ru_minflt)?,
            major_faults: convert(raw.ru_majflt)?,
            voluntary_switches: convert(raw.ru_nvcsw)?,
            involuntary_switches: convert(raw.ru_nivcsw)?,
        })
    }

    pub fn since(self, before: Self) -> Option<Self> {
        Some(Self {
            minor_faults: self.minor_faults.checked_sub(before.minor_faults)?,
            major_faults: self.major_faults.checked_sub(before.major_faults)?,
            voluntary_switches: self
                .voluntary_switches
                .checked_sub(before.voluntary_switches)?,
            involuntary_switches: self
                .involuntary_switches
                .checked_sub(before.involuntary_switches)?,
        })
    }

    pub fn changed(self) -> bool {
        self != Self::default()
    }

    pub fn accumulate(&mut self, delta: Self) {
        self.minor_faults = self.minor_faults.saturating_add(delta.minor_faults);
        self.major_faults = self.major_faults.saturating_add(delta.major_faults);
        self.voluntary_switches = self
            .voluntary_switches
            .saturating_add(delta.voluntary_switches);
        self.involuntary_switches = self
            .involuntary_switches
            .saturating_add(delta.involuntary_switches);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usage_deltas_keep_faults_and_switches_distinct_and_reject_backwards_counts() {
        let before = ThreadUsage {
            minor_faults: 10,
            major_faults: 2,
            voluntary_switches: 5,
            involuntary_switches: 8,
        };
        let after = ThreadUsage {
            minor_faults: 11,
            major_faults: 2,
            voluntary_switches: 5,
            involuntary_switches: 10,
        };
        let delta = after.since(before).unwrap();
        assert_eq!(
            delta,
            ThreadUsage {
                minor_faults: 1,
                major_faults: 0,
                voluntary_switches: 0,
                involuntary_switches: 2
            }
        );
        assert!(delta.changed());
        assert!(!before.since(before).unwrap().changed());
        assert!(before.since(after).is_none());
        let mut total = ThreadUsage::default();
        total.accumulate(delta);
        total.accumulate(delta);
        assert_eq!(total.minor_faults, 2);
        assert_eq!(total.involuntary_switches, 4);
    }

    #[test]
    fn calling_thread_usage_can_be_observed_without_mutation() {
        let before = ThreadUsage::sample().unwrap();
        let after = ThreadUsage::sample().unwrap();
        assert!(after.since(before).is_some());
    }
}
