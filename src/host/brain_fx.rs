//! Source-indexed Brain effects through the actual independently loaded SHR FX.
//! One existing stereo owner instance per declared pair; no copied FX algorithm.
use super::adapters::{Dsp, Result};
use std::path::Path;

/// Prepared off processing. The final odd mono input uses a silent partner;
/// this mapping is explicit and never combines distinct source channels.
pub struct BrainFx {
    instances: Vec<Dsp>,
    channels: usize,
    max_block: usize,
    input_pair: Vec<f64>,
    output_pair: Vec<f64>,
    epoch: u64,
    next_frame: Option<u64>,
    pub resets: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FxError {
    Shape,
    Epoch,
    StaleFrame,
    FrameExhausted,
    Owner(i32),
}
impl BrainFx {
    /// Bounded memory admission is explicit and separate from mixer dimensions.
    /// Each owner instance reports its own fixed20ms intentional delay; network
    /// output deadline is a separate negotiated quantity.
    pub fn prepare(library: &Path, channels: usize, max_block: usize, epoch: u64) -> Result<Self> {
        if epoch == 0 || channels == 0 || channels > 256 || max_block == 0 || max_block > 8192 {
            return Err(
                "FX bank admission: nonzero epoch,1..256 media channels,1..8192 owner frames"
                    .into(),
            );
        }
        let instances = (0..channels.div_ceil(2))
            .map(|_| Dsp::load(library, "fx", max_block as u32))
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            instances,
            channels,
            max_block,
            input_pair: vec![0.; max_block * 2],
            output_pair: vec![0.; max_block * 2],
            epoch,
            next_frame: None,
            resets: 0,
        })
    }
    pub fn channels(&self) -> usize {
        self.channels
    }
    pub fn intentional_delay_frames(&self) -> u32 {
        self.instances[0].delay
    }
    /// Session replacement explicitly discards all old owner histories.
    pub fn reset_session(&mut self, epoch: u64) -> std::result::Result<(), FxError> {
        if epoch == 0 {
            return Err(FxError::Epoch);
        }
        for instance in &mut self.instances {
            instance.reset();
        }
        self.epoch = epoch;
        self.next_frame = None;
        self.resets = self.resets.saturating_add(1);
        Ok(())
    }
    /// Processes exactly the source-tagged block; no wall clock or arrival timer
    /// creates samples. Old/duplicate frames refuse before touching owner state.
    /// A forward gap resets tails before processing the new contiguous segment.
    pub fn process(
        &mut self,
        epoch: u64,
        frame: u64,
        input: &[f64],
        output: &mut [f64],
    ) -> std::result::Result<(), FxError> {
        if input.is_empty()
            || !input.len().is_multiple_of(self.channels)
            || input.len() != output.len()
            || input.len() / self.channels > self.max_block
        {
            return Err(FxError::Shape);
        }
        if epoch != self.epoch {
            return Err(FxError::Epoch);
        }
        if self.next_frame.is_some_and(|n| frame < n) {
            return Err(FxError::StaleFrame);
        }
        let frames = input.len() / self.channels;
        let end = frame
            .checked_add(frames as u64)
            .ok_or(FxError::FrameExhausted)?;
        if self.next_frame.is_some_and(|n| frame > n) {
            for instance in &mut self.instances {
                instance.reset();
            }
            self.resets = self.resets.saturating_add(1);
        }
        // Validate the complete bank before advancing any instance.
        if input.iter().any(|s| !s.is_finite() || s.abs() > 16.) {
            output.fill(0.);
            for instance in &mut self.instances {
                instance.reset();
            }
            self.next_frame = None;
            return Err(FxError::Owner(-3));
        }
        for (index, instance) in self.instances.iter_mut().enumerate() {
            let left = index * 2;
            let right = left + 1;
            for f in 0..frames {
                self.input_pair[f * 2] = input[f * self.channels + left];
                self.input_pair[f * 2 + 1] = if right < self.channels {
                    input[f * self.channels + right]
                } else {
                    0.
                };
            }
            let rc = instance.process_result(
                &self.input_pair[..frames * 2],
                &mut self.output_pair[..frames * 2],
                2,
            );
            if rc != 0 {
                output.fill(0.);
                for instance in &mut self.instances {
                    instance.reset();
                }
                self.next_frame = None;
                return Err(FxError::Owner(rc));
            }
            for f in 0..frames {
                output[f * self.channels + left] = self.output_pair[f * 2];
                if right < self.channels {
                    output[f * self.channels + right] = self.output_pair[f * 2 + 1];
                }
            }
        }
        self.next_frame = Some(end);
        Ok(())
    }
}
