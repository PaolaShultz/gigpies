//! Fixed-storage offline eight-mono graph. No PA protection or host integration.
use crate::control_model::{Edit, Target, Value};
use crate::show::Result;

pub const INPUTS: usize = 8;
pub const RAMP_FRAMES: u64 = 240;
pub const BOUNDARY_FRAMES: u64 = 48;
const COEFFICIENTS: usize = 6; // fader, pan L/R, mute, send 1/2
#[derive(Debug, Clone, Copy)]
struct Ramp {
    start: f64,
    target: f64,
    begin: u64,
    end: u64,
}
impl Ramp {
    fn fixed(value: f64) -> Self {
        Self {
            start: value,
            target: value,
            begin: 0,
            end: 0,
        }
    }
    fn at(self, frame: u64) -> f64 {
        if frame >= self.end {
            self.target
        } else if frame <= self.begin {
            self.start
        } else {
            self.start
                + (self.target - self.start)
                    * ((frame - self.begin) as f64 / (self.end - self.begin) as f64)
        }
    }
}
#[derive(Debug, Clone, Copy)]
enum Change {
    Keep,
    Ramp(f64),
    Freeze,
}
/// All transcendental arithmetic and target/string validation occurs here, outside process.
#[derive(Debug, Clone, Copy)]
pub struct Prepared {
    changes: [[Change; COEFFICIENTS]; INPUTS],
    processing: Option<(usize, crate::channel_processing::Prepared)>,
}
impl Prepared {
    pub fn processing(input: usize, prepared: crate::channel_processing::Prepared) -> Result<Self> {
        if input >= INPUTS {
            return Err("target".into());
        }
        Ok(Self {
            changes: [[Change::Keep; COEFFICIENTS]; INPUTS],
            processing: Some((input, prepared)),
        })
    }
    pub fn edits(edits: &[Edit]) -> Result<Self> {
        if edits.is_empty() || edits.len() > 64 {
            return Err("capacity".into());
        }
        let mut p = Self {
            changes: [[Change::Keep; COEFFICIENTS]; INPUTS],
            processing: None,
        };
        let mut seen = [[false; 5]; INPUTS];
        for e in edits {
            let (input, slot) = target_slot(&e.target)?;
            if seen[input][slot] {
                return Err("duplicate target".into());
            }
            seen[input][slot] = true;
            match (&e.target, &e.value) {
                (Target::Fader { .. }, Value::Integer(n))
                | (Target::Send { .. }, Value::Integer(n))
                    if (-60000..=12000).contains(n) && n % 100 == 0 =>
                {
                    let index = if slot == 0 { 0 } else { slot + 1 };
                    p.changes[input][index] = Change::Ramp(10_f64.powf(*n as f64 / 20000.0));
                }
                (Target::Pan { .. }, Value::Integer(n)) if (-100..=100).contains(n) => {
                    let angle = (*n as f64 + 100.0) * std::f64::consts::FRAC_PI_2 / 200.0;
                    let (l, r) = if *n == -100 {
                        (1.0, 0.0)
                    } else if *n == 100 {
                        (0.0, 1.0)
                    } else {
                        (angle.cos(), angle.sin())
                    };
                    p.changes[input][1] = Change::Ramp(l);
                    p.changes[input][2] = Change::Ramp(r);
                }
                (Target::Mute { .. }, Value::Boolean(mute)) => {
                    p.changes[input][3] = Change::Ramp(if *mute { 0.0 } else { 1.0 })
                }
                _ => return Err("range".into()),
            }
        }
        Ok(p)
    }
    pub fn freeze(scope: crate::control_model::Scope) -> Self {
        let mut p = Self {
            changes: [[Change::Keep; COEFFICIENTS]; INPUTS],
            processing: None,
        };
        for row in &mut p.changes {
            match scope {
                crate::control_model::Scope::Foh => row[..4].fill(Change::Freeze),
                crate::control_model::Scope::Monitor1 => row[4] = Change::Freeze,
                crate::control_model::Scope::Monitor2 => row[5] = Change::Freeze,
            }
        }
        p
    }
}
fn target_slot(target: &Target) -> Result<(usize, usize)> {
    let (input, slot) = match target {
        Target::Fader { input } => (input, 0),
        Target::Pan { input } => (input, 1),
        Target::Mute { input } => (input, 2),
        Target::Send { input, monitor } => (
            input,
            match monitor.as_str() {
                "monitor-1" => 3,
                "monitor-2" => 4,
                _ => return Err("target".into()),
            },
        ),
    };
    let index = (1..=8)
        .find(|i| input == &format!("input-{i:02}"))
        .ok_or("target")?
        - 1;
    Ok((index, slot))
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Completion {
    pub ticket: u64,
    pub frame: u64,
}
#[derive(Debug, Clone, Copy)]
struct Pending {
    prepared: Prepared,
    frame: u64,
    ticket: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessError {
    BufferLength,
    ClockExhausted,
}
#[derive(Debug)]
pub struct Mixer {
    clock: u64,
    ramps: [[Ramp; COEFFICIENTS]; INPUTS],
    pending: Option<Pending>,
    completion: Option<Completion>,
    fault: bool,
    processing: [crate::channel_processing::Strip; INPUTS],
}
impl Default for Mixer {
    fn default() -> Self {
        Self::new(0)
    }
}
impl Mixer {
    pub fn new(frame: u64) -> Self {
        let gain = 10_f64.powf(-6.0 / 20.0);
        Self {
            clock: frame,
            ramps: [[
                Ramp::fixed(gain),
                Ramp::fixed(std::f64::consts::FRAC_1_SQRT_2),
                Ramp::fixed(std::f64::consts::FRAC_1_SQRT_2),
                Ramp::fixed(1.0),
                Ramp::fixed(1.0),
                Ramp::fixed(0.001),
            ]; INPUTS],
            pending: None,
            completion: None,
            fault: false,
            processing: [crate::channel_processing::Strip::default(); INPUTS],
        }
    }
    pub fn frame(&self) -> u64 {
        self.clock
    }
    pub fn faulted(&self) -> bool {
        self.fault
    }
    pub fn processing_observations(
        &self,
    ) -> [(
        crate::channel_processing::Config,
        crate::channel_processing::Config,
        u32,
        Option<i32>,
    ); INPUTS] {
        self.processing
            .map(|s| s.observation(self.clock, self.fault))
    }
    pub fn processing_ready(&self) -> bool {
        self.processing_observations().iter().all(|v| v.2 == 0)
    }
    pub fn next_boundary(&self) -> Result<u64> {
        self.clock
            .checked_div(48)
            .and_then(|n| n.checked_add(1))
            .and_then(|n| n.checked_mul(48))
            .ok_or("clock exhausted".into())
    }
    pub fn schedule(&mut self, prepared: Prepared, ticket: u64) -> Result<u64> {
        if self.pending.is_some()
            || self.completion.is_some()
            || (prepared.processing.is_some() && !self.processing_ready())
        {
            return Err("capacity".into());
        }
        let frame = self.next_boundary()?;
        frame.checked_add(RAMP_FRAMES).ok_or("clock exhausted")?;
        if ticket == 0 {
            return Err("ticket".into());
        }
        self.pending = Some(Pending {
            prepared,
            frame,
            ticket,
        });
        Ok(frame)
    }
    pub fn cancel(&mut self, ticket: u64) -> bool {
        if self.pending.is_some_and(|p| p.ticket == ticket) {
            self.pending = None;
            true
        } else {
            false
        }
    }
    pub fn take_completion(&mut self) -> Option<Completion> {
        self.completion.take()
    }
    pub fn coefficients(&self) -> [[f64; COEFFICIENTS]; INPUTS] {
        self.ramps.map(|row| row.map(|r| r.at(self.clock)))
    }
    pub fn targets(&self) -> [[f64; COEFFICIENTS]; INPUTS] {
        self.ramps.map(|row| row.map(|r| r.target))
    }
    /// Frames are consumed in order. Boundary applies before its sample; the first
    /// sample uses the actual old coefficient, endpoint sample uses exact target.
    /// All storage is fixed and Copy; no heap values enter or leave this function.
    pub fn process(
        &mut self,
        input: &[[f64; INPUTS]],
        output: &mut [[f64; 4]],
    ) -> std::result::Result<(), ProcessError> {
        if input.len() != output.len() {
            return Err(ProcessError::BufferLength);
        }
        if self.clock.checked_add(input.len() as u64).is_none() {
            self.fault = true;
            output.fill([0.0; 4]);
            return Err(ProcessError::ClockExhausted);
        }
        for (sources, out) in input.iter().zip(output) {
            if let Some(p) = self.pending
                && p.frame == self.clock
            {
                for (row, changes) in self.ramps.iter_mut().zip(p.prepared.changes) {
                    for (r, c) in row.iter_mut().zip(changes) {
                        let current = r.at(self.clock);
                        match c {
                            Change::Keep => (),
                            Change::Freeze => *r = Ramp::fixed(current),
                            Change::Ramp(target) => {
                                *r = Ramp {
                                    start: current,
                                    target,
                                    begin: self.clock,
                                    end: self.clock + RAMP_FRAMES,
                                }
                            }
                        }
                    }
                }
                if let Some((input, prepared)) = p.prepared.processing {
                    self.processing[input].apply(prepared, self.clock);
                }
                self.pending = None;
                self.completion = Some(Completion {
                    ticket: p.ticket,
                    frame: p.frame,
                });
            }
            *out = [0.0; 4];
            if sources.iter().any(|s| !s.is_finite()) {
                self.fault = true;
            }
            if !self.fault {
                for ((sample, row), strip) in
                    sources.iter().zip(self.ramps).zip(&mut self.processing)
                {
                    let c = row.map(|r| r.at(self.clock));
                    if c.iter().any(|x| !x.is_finite()) {
                        self.fault = true;
                        break;
                    }
                    let shared = sample * c[3];
                    let processed = strip.tick(*sample, self.clock);
                    if !processed.is_finite() {
                        self.fault = true;
                        break;
                    }
                    let foh = processed * c[3];
                    out[0] += foh * c[0] * c[1];
                    out[1] += foh * c[0] * c[2];
                    out[2] += shared * c[4];
                    out[3] += shared * c[5];
                }
                if out.iter().any(|s| !s.is_finite()) {
                    self.fault = true;
                }
            }
            if self.fault {
                *out = [0.0; 4];
            }
            self.clock += 1;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn corrupted_coefficient_latches_finite_zero_output() {
        let mut mixer = Mixer::default();
        mixer.ramps[3][4] = Ramp::fixed(f64::INFINITY);
        let mut output = [[1.0; 4]; 2];
        mixer.process(&[[1.0; INPUTS]; 2], &mut output).unwrap();
        assert_eq!(output, [[0.0; 4]; 2]);
        assert!(mixer.faulted());
        assert_eq!(mixer.frame(), 2);
    }
}
