//! Prepared, bounded sample observer. offer performs no allocation, lock, I/O,
//! strings, serialization or retries. Quantization is exclusively control-side.
use crate::{
    meter_wire::{self, Snapshot, Tap},
    show::Counter,
};
use rtrb::{Consumer, Producer, RingBuffer};
#[derive(Clone, Copy, Debug, Default)]
pub struct Accumulator {
    peak: f64,
    squares: f64,
    clipped: u64,
    invalid: u64,
    fault: bool,
}
impl Accumulator {
    pub fn add(&mut self, x: f64, valid: bool) {
        self.fault |= !valid;
        if !x.is_finite() {
            self.invalid += 1;
            return;
        }
        let a = x.abs();
        self.clipped += u64::from(a >= 1.);
        if a > self.peak {
            let ratio = self.peak / a;
            self.squares = self.squares * ratio * ratio + 1.;
            self.peak = a;
        } else if self.peak > 0. {
            let ratio = a / self.peak;
            self.squares += ratio * ratio;
        }
    }
    pub fn record(&self, id: String, frames: u64) -> Tap {
        let valid = self.invalid == 0 && !self.fault;
        let level = |a: f64| {
            if a == 0. {
                -120000
            } else {
                (20000. * a.log10()).round().max(-120000.) as i32
            }
        };
        Tap {
            id,
            valid,
            reason: if self.invalid > 0 {
                Some("nonfinite".into())
            } else if self.fault {
                Some("graph_fault".into())
            } else {
                None
            },
            peak_millidbfs: valid.then(|| level(self.peak)),
            rms_millidbfs: valid.then(|| level(self.peak * (self.squares / frames as f64).sqrt())),
            silent: valid && self.peak == 0.,
            below_floor: valid && self.peak < 1e-6,
            over_range: valid && self.peak > 1.,
            clip_count: Counter(self.clipped),
            invalid_count: Counter(self.invalid),
        }
    }
}
struct Window {
    data: Box<[Accumulator]>,
    first: u64,
    end: u64,
    mono: u64,
    sequence: u64,
    loss: u64,
}
pub struct Acquisition {
    inputs: usize,
    monitors: usize,
    window_frames: u64,
    expected: Option<u64>,
    first: u64,
    count: u64,
    mono: u64,
    sequence: u64,
    accumulators: Box<[Accumulator]>,
    free: Consumer<Window>,
    output: Producer<Window>,
    pub losses: u64,
}
pub struct Latest {
    input: Consumer<Window>,
    recycle: Producer<Window>,
    latest: Option<Window>,
}
pub fn capacity(inputs: usize, monitors: usize) -> Option<usize> {
    let taps = inputs
        .checked_mul(2)?
        .checked_add(monitors)?
        .checked_add(2)?;
    let wire = taps.checked_mul(512)?.checked_add(4096)?;
    (inputs > 0 && taps <= meter_wire::MAX_TAPS && wire <= meter_wire::MAX_BYTES - 4096)
        .then_some(taps)
}
pub fn prepare(inputs: usize, monitors: usize, rate: u32) -> Option<(Acquisition, Latest)> {
    let taps = capacity(inputs, monitors)?;
    if rate == 0 || !rate.is_multiple_of(50) {
        return None;
    }
    let (output, input) = RingBuffer::new(2);
    let (mut recycle, free) = RingBuffer::new(2);
    for _ in 0..2 {
        recycle
            .push(Window {
                data: vec![Accumulator::default(); taps].into_boxed_slice(),
                first: 0,
                end: 0,
                mono: 0,
                sequence: 0,
                loss: 0,
            })
            .ok()?;
    }
    Some((
        Acquisition {
            inputs,
            monitors,
            window_frames: u64::from(rate / 50),
            expected: None,
            first: 0,
            count: 0,
            mono: 0,
            sequence: 0,
            accumulators: vec![Accumulator::default(); taps].into_boxed_slice(),
            free,
            output,
            losses: 0,
        },
        Latest {
            input,
            recycle,
            latest: None,
        },
    ))
}
impl Acquisition {
    pub fn reset(&mut self) {
        self.count = 0;
        self.expected = None;
        self.accumulators.fill(Accumulator::default());
    }
    #[allow(clippy::too_many_arguments)]
    pub fn offer(
        &mut self,
        frame: u64,
        mono: u64,
        raw: &[f64],
        processed: &[f64],
        processed_valid: &[bool],
        main: &[f64],
        buses: &[f64],
        valid: bool,
    ) {
        let frames = raw.len() / self.inputs;
        let stride = self.monitors + 2;
        if raw.len() != frames * self.inputs
            || processed.len() != raw.len()
            || processed_valid.len() != frames
            || main.len() != frames * 2
            || buses.len() != frames * stride
            || frame.checked_add(frames as u64).is_none()
        {
            self.reset();
            return;
        }
        if self.expected.is_some_and(|n| n != frame) || (self.count > 0 && mono < self.mono) {
            self.reset();
        }
        for f in 0..frames {
            if self.count == 0 {
                self.first = frame + f as u64;
                self.mono = mono;
            }
            for c in 0..self.inputs {
                self.accumulators[c * 2].add(raw[f * self.inputs + c], true);
                self.accumulators[c * 2 + 1]
                    .add(processed[f * self.inputs + c], processed_valid[f]);
            }
            let offset = self.inputs * 2;
            for c in 0..2 {
                self.accumulators[offset + c].add(main[f * 2 + c], valid && processed_valid[f]);
            }
            for c in 0..self.monitors {
                self.accumulators[offset + 2 + c]
                    .add(buses[f * stride + 2 + c], valid && processed_valid[f]);
            }
            self.count += 1;
            if self.count == self.window_frames {
                self.sequence = self.sequence.saturating_add(1);
                if let Ok(mut slot) = self.free.pop() {
                    slot.data.copy_from_slice(&self.accumulators);
                    slot.first = self.first;
                    slot.end = frame + f as u64 + 1;
                    slot.mono = self.mono;
                    slot.sequence = self.sequence;
                    slot.loss = self.losses;
                    // At most two slots exist; taking a free slot guarantees output capacity.
                    assert!(self.output.push(slot).is_ok());
                } else {
                    self.losses = self.losses.saturating_add(1);
                }
                self.count = 0;
                self.accumulators.fill(Accumulator::default());
            }
        }
        self.expected = Some(frame + frames as u64);
    }
}
impl Latest {
    /// Control-side bounded drain and ownership recycling.
    pub fn drain(&mut self) {
        for _ in 0..2 {
            if let Ok(window) = self.input.pop()
                && let Some(old) = self.latest.replace(window)
            {
                assert!(self.recycle.push(old).is_ok());
            }
        }
    }
    pub fn clear(&mut self) {
        self.drain();
        if let Some(old) = self.latest.take() {
            assert!(self.recycle.push(old).is_ok());
        }
    }
    #[allow(clippy::too_many_arguments)]
    pub fn snapshot(
        &mut self,
        request: &meter_wire::Request,
        topology: &crate::topology::EngineTopology,
        epoch: u64,
        generation: u64,
        now: u64,
        losses: u64,
        quiesced: bool,
    ) -> Snapshot {
        self.drain();
        let reason = if request.version != 1 {
            Some("unsupported")
        } else if request.source_epoch.0 != epoch || request.expected_map.0 != topology.map_revision
        {
            Some("identity")
        } else if quiesced {
            Some("quiesced")
        } else if self.latest.is_none() {
            Some("missing_window")
        } else {
            None
        };
        let mut s = Snapshot {
            contract: meter_wire::CONTRACT.into(),
            version: 1,
            kind: "meter_snapshot".into(),
            show_id: request.show_id.clone(),
            module: "audio".into(),
            source_epoch: Counter(epoch),
            query_id: request.query_id,
            capability_generation: Counter(generation),
            map_generation: Counter(topology.map_revision),
            topology: topology.identity.clone(),
            sample_rate: topology.sample_rate,
            inputs: topology.inputs.len(),
            monitors: topology.monitors,
            sequence: Counter(0),
            first_frame: None,
            end_frame: None,
            acquisition_age_ms: None,
            publication_loss: Counter(losses),
            valid: reason.is_none(),
            reason: reason.map(str::to_owned),
            taps: Vec::new(),
        };
        if reason.is_none() {
            let w = self.latest.as_ref().unwrap();
            s.sequence = Counter(w.sequence);
            s.first_frame = Some(Counter(w.first));
            s.end_frame = Some(Counter(w.end));
            s.acquisition_age_ms = Some(Counter(now.saturating_sub(w.mono)));
            s.publication_loss = Counter(losses.max(w.loss));
            for (i, a) in w.data.iter().enumerate() {
                let id = if i < s.inputs * 2 {
                    format!(
                        "{}:{}",
                        topology.inputs[i / 2].id,
                        if i % 2 == 0 { "raw" } else { "processed" }
                    )
                } else if i == s.inputs * 2 {
                    "main-l".into()
                } else if i == s.inputs * 2 + 1 {
                    "main-r".into()
                } else {
                    format!("monitor-{}", i - s.inputs * 2 - 1)
                };
                s.taps.push(a.record(id, u64::from(s.sample_rate / 50)));
            }
        }
        s
    }
}
