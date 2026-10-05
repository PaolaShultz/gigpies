//! Explicit raw ALSA adapter for the common processing graph.
//! Construction opens hardware only when deliberately invoked by the host.
//! Driver I/O and provider control pumping are outside the bounded DSP section.
use super::{adapters::Result, capture_msb24};
use crate::{
    local_audio::LocalAudio,
    topology::{EngineTopology, ResourceBudget},
};
use alsa::{
    Direction, ValueOr,
    pcm::{Access, Format, HwParams, PCM, TstampType},
};
use std::time::{Duration, Instant};

/// A retained manual acceptance record binds an observed device and exact patch.
/// This is evidence supplied by the physical acceptance procedure, not an ADAT
/// lock measurement or a software-created claim of converter synchronization.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceAcceptance {
    pub device: String,
    pub topology_sha256: String,
    pub native_significant_bits: u32,
    pub socket_mapping_record: String,
    pub single_clock_setup_record: String,
}
impl DeviceAcceptance {
    pub fn validate(&self, device: &str, topology: &EngineTopology) -> Result<()> {
        use sha2::{Digest, Sha256};
        let hash = format!("{:x}", Sha256::digest(serde_json::to_vec(topology)?));
        validate_main_routes(topology)?;
        if topology.mapping_evidence != "operator-verified"
            || self.device != device
            || self.topology_sha256 != hash
            || self.native_significant_bits != 24
            || self.socket_mapping_record.trim().is_empty()
            || self.socket_mapping_record.len() > 4096
            || self.single_clock_setup_record.trim().is_empty()
            || self.single_clock_setup_record.len() > 4096
        {
            return Err(
                "observed native PCM, exact socket map and single-clock acceptance record required"
                    .into(),
            );
        }
        Ok(())
    }
}
/// Two directions of the same raw hardware endpoint. No separate capture/playback
/// device names are accepted: this profile has one device clock domain.
pub struct Duplex {
    capture: PCM,
    playback: PCM,
    topology: EngineTopology,
    raw: Vec<i32>,
    dac: Vec<i32>,
    input: Vec<f64>,
    output: Vec<f64>,
    frames: usize,
    started_playback: bool,
    faulted: bool,
}
fn open_direction(
    device: &str,
    direction: Direction,
    channels: usize,
    frames: usize,
    buffer: usize,
) -> Result<PCM> {
    let channels = u32::try_from(channels).map_err(|_| "ALSA channel representation")?;
    let pcm = PCM::new(device, direction, true)?;
    let p = HwParams::any(&pcm)?;
    p.set_access(Access::RWInterleaved)?;
    p.set_rate_resample(false)?;
    p.set_format(Format::S32LE)?;
    p.set_channels(channels)?;
    p.set_rate(48000, ValueOr::Nearest)?;
    p.set_period_size(frames as i64, ValueOr::Nearest)?;
    p.set_buffer_size(buffer as i64)?;
    pcm.hw_params(&p)?;
    let actual = pcm.hw_params_current()?;
    if actual.get_channels()? != channels
        || actual.get_rate()? != 48000
        || actual.get_period_size()? != frames as i64
        || actual.get_buffer_size()? != buffer as i64
        || actual.get_format()? != Format::S32LE
    {
        return Err("ALSA negotiated shape differs from explicit configuration".into());
    }
    let sw = pcm.sw_params_current()?;
    sw.set_start_threshold(sw.get_boundary()?)?;
    sw.set_avail_min(frames as i64)?;
    sw.set_tstamp_mode(true)?;
    sw.set_tstamp_type(TstampType::Monotonic)?;
    pcm.sw_params(&sw)?;
    drop(sw);
    drop(actual);
    drop(p);
    pcm.prepare()?;
    Ok(pcm)
}
fn transfer(pcm: &PCM, data: &mut [i32], channels: usize, capture: bool) -> Result<()> {
    if channels == 0 || data.is_empty() || !data.len().is_multiple_of(channels) {
        return Err("invalid PCM transfer shape".into());
    }
    let io = pcm.io_i32()?;
    let begin = Instant::now();
    let result = super::transfer::transfer_frames(
        data.len() / channels,
        || begin.elapsed() >= Duration::from_secs(2),
        || match pcm.avail_update() {
            Ok(n) => Ok(n as usize),
            Err(e) if matches!(e.errno(), libc::EINTR | libc::EAGAIN) => Ok(0),
            Err(e) => Err(e),
        },
        |offset| {
            let result = if capture {
                io.readi(&mut data[offset * channels..])
            } else {
                io.writei(&data[offset * channels..])
            };
            match result {
                Ok(n) => Ok(Some(n)),
                Err(e) if matches!(e.errno(), libc::EINTR | libc::EAGAIN) => Ok(None),
                Err(e) => Err(e),
            }
        },
        |ms| match pcm.wait(Some(ms)) {
            Ok(_) => Ok(()),
            Err(e) if matches!(e.errno(), libc::EINTR | libc::EAGAIN) => Ok(()),
            Err(e) => Err(e),
        },
    );
    match result {
        Ok(()) => Ok(()),
        Err(super::transfer::TransferError::Device(e)) => Err(e.into()),
        Err(super::transfer::TransferError::Deadline) => Err("PCM transfer deadline".into()),
        Err(super::transfer::TransferError::InvalidProgress) => {
            Err("PCM invalid transfer progress".into())
        }
    }
}
impl Duplex {
    /// Caller supplies an explicitly verified native S32LE upper-24-bit profile.
    /// A reference factory is not such verification. This method never discovers,
    /// selects, resamples, remaps or opens a second independent audio device.
    pub fn open(
        device: &str,
        topology: EngineTopology,
        acceptance: &DeviceAcceptance,
        period_frames: usize,
        buffer_frames: usize,
    ) -> Result<Self> {
        topology
            .validate(ResourceBudget::default())
            .map_err(|e| e.to_string())?;
        acceptance.validate(device, &topology)?;
        if !device.starts_with("hw:")
            || device.len() <= 3
            || period_frames == 0
            || period_frames > topology.max_block_frames
            || !period_frames.is_multiple_of(48)
            || buffer_frames < period_frames.checked_mul(2).ok_or("buffer overflow")?
            || buffer_frames > i32::MAX as usize
        {
            return Err("explicit raw same-device PCM and valid buffer dimensions required".into());
        }
        let capture_samples = period_frames
            .checked_mul(topology.capture_channels)
            .ok_or("capture buffer overflow")?;
        let playback_samples = period_frames
            .checked_mul(topology.playback_channels)
            .ok_or("playback buffer overflow")?;
        let capture = open_direction(
            device,
            Direction::Capture,
            topology.capture_channels,
            period_frames,
            buffer_frames,
        )?;
        let playback = open_direction(
            device,
            Direction::Playback,
            topology.playback_channels,
            period_frames,
            buffer_frames,
        )?;
        let ci = capture.info()?;
        let pi = playback.info()?;
        if ci.get_card() != pi.get_card() || ci.get_device() != pi.get_device() {
            return Err("capture/playback device identity mismatch".into());
        }
        Ok(Self {
            capture,
            playback,
            topology,
            raw: vec![0; capture_samples],
            dac: vec![0; playback_samples],
            input: vec![0.; capture_samples],
            output: vec![0.; playback_samples],
            frames: period_frames,
            started_playback: false,
            faulted: false,
        })
    }
    /// Deliberate start after configuration review. No signal generator is added.
    pub fn start(&mut self) -> Result<()> {
        if self.faulted {
            return Err("device fault requires new adapter and fresh source epoch".into());
        }
        self.capture.start()?;
        Ok(())
    }
    /// Capture drives frame progression; provider performs the exact same graph
    /// used by synthetic input. Network arrival cannot call or advance this pump.
    pub fn pump(&mut self, provider: &mut LocalAudio, now_ms: u64) -> Result<()> {
        if self.faulted {
            return Err("device adapter faulted".into());
        }
        if let Err(error) = validate_provider_map(provider.topology(), &self.topology) {
            self.faulted = true;
            let _ = provider.quiesce_source("device_mapping_changed");
            let _ = self.capture.drop();
            let _ = self.playback.drop();
            return Err(error);
        }
        provider
            .engine_mut()
            .mark_verified_mapping()
            .map_err(|e| e.to_string())?;
        let control_started = Instant::now();
        let result = (|| -> Result<()> {
            transfer(
                &self.capture,
                &mut self.raw,
                self.topology.capture_channels,
                true,
            )?;
            for (input, raw) in self.input.iter_mut().zip(&self.raw) {
                *input = capture_msb24(*raw);
            }
            let epoch = provider.source_epoch();
            let first = provider.frame();
            for (capture, playback) in self
                .input
                .chunks_exact(48 * self.topology.capture_channels)
                .zip(
                    self.output
                        .chunks_exact_mut(48 * self.topology.playback_channels),
                )
            {
                let elapsed = u64::try_from(control_started.elapsed().as_millis())
                    .map_err(|_| "lease clock elapsed overflow")?;
                let fresh_now = now_ms.checked_add(elapsed).ok_or("lease clock overflow")?;
                provider
                    .tick_with_capture(fresh_now, epoch, provider.frame(), capture, playback)
                    .map_err(|e| e.to_string())?;
            }
            for (dac, sample) in self.dac.iter_mut().zip(&self.output) {
                if !sample.is_finite() {
                    return Err("nonfinite shared graph playback".into());
                }
                *dac = (sample * 8388608.).round().clamp(-8388608., 8388607.) as i32 * 256;
            }
            transfer(
                &self.playback,
                &mut self.dac,
                self.topology.playback_channels,
                false,
            )?;
            if !self.started_playback {
                self.playback.start()?;
                self.started_playback = true;
            }
            if provider.frame()
                != first
                    .checked_add(self.frames as u64)
                    .ok_or("source cursor overflow")?
            {
                return Err("shared graph source progression mismatch".into());
            }
            Ok(())
        })();
        if result.is_err() {
            self.faulted = true;
            let _ = provider.quiesce_source("device_transfer_discontinuity");
            let _ = self.capture.drop();
            let _ = self.playback.drop();
        }
        result
    }
}
impl Drop for Duplex {
    fn drop(&mut self) {
        let _ = self.capture.drop();
        let _ = self.playback.drop();
    }
}

/// Logical signal assignments and fresh map generations may change while the
/// observed physical transport map remains exactly the accepted device map.
pub fn same_device_map(a: &EngineTopology, b: &EngineTopology) -> bool {
    a.sample_rate == b.sample_rate
        && a.capture_channels == b.capture_channels
        && a.playback_channels == b.playback_channels
        && a.inputs == b.inputs
        && a.measurement_slots == b.measurement_slots
        && a.mapping_evidence == b.mapping_evidence
        && a.outputs.len() == b.outputs.len()
        && a.outputs.iter().zip(&b.outputs).all(|(a, b)| {
            a.id == b.id && a.playback_slot == b.playback_slot && a.physical_port == b.physical_port
        })
}

/// Controller-side pump admission also checks the active provider's signal routes.
/// An accepted device map alone does not authorize a separately constructed
/// provider to bypass its configured PA through direct main outputs.
pub fn validate_provider_map(provider: &EngineTopology, accepted: &EngineTopology) -> Result<()> {
    if !same_device_map(provider, accepted) {
        return Err("device/provider physical map mismatch; verified reopen required".into());
    }
    if provider.pa_outputs != accepted.pa_outputs {
        return Err("device/provider PA inventory mismatch; verified reopen required".into());
    }
    validate_main_routes(provider)
}
fn validate_main_routes(topology: &EngineTopology) -> Result<()> {
    if topology.pa_outputs > 0
        && topology
            .outputs
            .iter()
            .any(|p| matches!(p.source, Some(crate::topology::OutputSource::Main { .. })))
    {
        return Err("physical main route must pass through configured PA protection".into());
    }
    Ok(())
}
