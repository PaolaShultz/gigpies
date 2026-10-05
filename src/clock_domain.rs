//! Single interface timeline. Physical lock is unknown until supplied by an
//! adapter that can actually read it; nominal rate never proves converter lock.
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LockEvidence {
    Unknown,
    Locked,
    Lost,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClockState {
    Disarmed,
    Running,
    Quiesced,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClockDomain {
    pub domain: String,
    pub sample_rate: u32,
    pub epoch: u64,
    pub next_frame: u64,
    pub state: ClockState,
    pub adat_lock: LockEvidence,
    pub mapping_verified: bool,
    pub fault: Option<String>,
}
impl ClockDomain {
    pub fn new(epoch: u64, frame: u64) -> Self {
        Self {
            domain: "single-interface".into(),
            sample_rate: 48000,
            epoch,
            next_frame: frame,
            state: ClockState::Disarmed,
            adat_lock: LockEvidence::Unknown,
            mapping_verified: false,
            fault: None,
        }
    }
    pub fn admit(&mut self, epoch: u64, frame: u64, frames: usize) -> Result<(), &'static str> {
        if self.state != ClockState::Running {
            return Err("clock disarmed");
        }
        if self.adat_lock == LockEvidence::Lost
            || epoch != self.epoch
            || frame != self.next_frame
            || frame.checked_add(frames as u64).is_none()
        {
            self.state = ClockState::Quiesced;
            self.fault = Some("source_discontinuity".into());
            return Err("source discontinuity");
        }
        self.next_frame += frames as u64;
        Ok(())
    }
    pub fn quiesce(&mut self, reason: &str) {
        self.state = ClockState::Quiesced;
        self.fault = Some(reason.into());
    }
    pub fn recover(&mut self, epoch: u64, frame: u64) -> Result<(), &'static str> {
        if self.state == ClockState::Running || epoch <= self.epoch || !frame.is_multiple_of(48) {
            return Err("fresh epoch while quiesced required");
        }
        self.epoch = epoch;
        self.next_frame = frame;
        self.state = ClockState::Disarmed;
        self.fault = None;
        self.adat_lock = LockEvidence::Unknown;
        Ok(())
    }
    pub fn rearm(&mut self) -> Result<(), &'static str> {
        if self.state != ClockState::Disarmed || self.adat_lock == LockEvidence::Lost {
            return Err("recovery required");
        }
        self.state = ClockState::Running;
        Ok(())
    }
}
