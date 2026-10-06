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
fn transfer(
    pcm: &PCM,
    data: &mut [i32],
    channels: usize,
    capture: bool,
    deadline: Instant,
) -> Result<()> {
    if channels == 0 || data.is_empty() || !data.len().is_multiple_of(channels) {
        return Err("invalid PCM transfer shape".into());
    }
    let io = pcm.io_i32()?;
    let result = transfer_before_deadline(
        data.len() / channels,
        deadline,
        Instant::now,
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
/// Bound the age of unsubmitted samples, distinct from already queued hardware
/// latency. Recheck after availability queries and before every partial write.
/// A driver wait may detect expiry late (up to the existing20ms wait quantum),
/// but no prepared samples are admitted after this deadline check fails.
fn transfer_before_deadline<E>(
    frames: usize,
    deadline: Instant,
    now: impl Fn() -> Instant,
    mut available: impl FnMut() -> std::result::Result<usize, E>,
    mut transfer: impl FnMut(usize) -> std::result::Result<Option<usize>, E>,
    mut wait: impl FnMut(u32) -> std::result::Result<(), E>,
) -> std::result::Result<(), super::transfer::TransferError<E>> {
    use super::transfer::TransferError;
    let result = super::transfer::transfer_frames(
        frames,
        || now() >= deadline,
        || available().map_err(TransferError::Device),
        |offset| {
            if now() >= deadline {
                return Err(TransferError::Deadline);
            }
            transfer(offset).map_err(TransferError::Device)
        },
        |ms| wait(ms).map_err(TransferError::Device),
    );
    match result {
        Ok(()) => Ok(()),
        Err(TransferError::Device(error)) => Err(error),
        Err(TransferError::Deadline) => Err(TransferError::Deadline),
        Err(TransferError::InvalidProgress) => Err(TransferError::InvalidProgress),
    }
}
fn playback_deadline(captured: Instant, frames: usize) -> Result<Instant> {
    let nanos = u64::try_from(frames)?
        .checked_mul(1_000_000_000)
        .ok_or("playback deadline overflow")?
        / 48_000;
    captured
        .checked_add(Duration::from_nanos(nanos))
        .ok_or_else(|| "playback deadline overflow".into())
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
        if ci.get_card() != pi.get_card()
            || ci.get_device() != pi.get_device()
            || ci.get_subdevice() != pi.get_subdevice()
        {
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
    /// Most recently converted capture period, in physical interleaved slot order.
    pub fn last_capture(&self) -> &[f64] {
        &self.input
    }
    /// Most recently processed playback period; silenced after a pump failure.
    pub fn last_playback(&self) -> &[f64] {
        &self.output
    }
    /// Capture drives frame progression; provider performs the same graph used by
    /// synthetic input. Network arrival cannot call or advance this pump.
    pub fn pump(&mut self, provider: &mut LocalAudio, now_ms: u64) -> Result<()> {
        self.pump_source(provider, now_ms, |_, _| Ok(()))
    }
    /// Authenticated Brain/FX media is consumed and monitor packets are emitted at
    /// the same captured source boundary as physical audio. This owns no new PCM.
    pub fn pump_authority(
        &mut self,
        authority: &mut crate::remote::HostAuthority,
        now_ms: u64,
    ) -> Result<()> {
        self.pump_authority_with(authority, now_ms, |_, _| Ok(()))
    }
    /// Service authenticated controller mailboxes after capture and immediately
    /// before each48-frame source boundary, using its freshly sampled lease time.
    pub fn pump_authority_with(
        &mut self,
        authority: &mut crate::remote::HostAuthority,
        now_ms: u64,
        before_render: impl FnMut(&mut crate::remote::HostAuthority, u64) -> Result<()>,
    ) -> Result<()> {
        self.pump_source(authority, now_ms, before_render)
    }
    fn pump_source<S: SourceBoundary>(
        &mut self,
        source: &mut S,
        now_ms: u64,
        before_render: impl FnMut(&mut S, u64) -> Result<()>,
    ) -> Result<()> {
        if self.faulted {
            return Err("device adapter faulted".into());
        }
        let control_started = Instant::now();
        let result = (|| -> Result<()> {
            validate_provider_map(source.provider().topology(), &self.topology)?;
            transfer(
                &self.capture,
                &mut self.raw,
                self.topology.capture_channels,
                true,
                Instant::now() + Duration::from_secs(2),
            )?;
            // Timestamp the prepared period before conversion, control and DSP.
            // A delayed partial submission must never resume stale talkback.
            let playback_deadline = playback_deadline(Instant::now(), self.frames)?;
            for (input, raw) in self.input.iter_mut().zip(&self.raw) {
                *input = capture_msb24(*raw);
            }
            process_captured(
                source,
                &self.topology,
                &self.input,
                &mut self.output,
                self.frames,
                before_render,
                || source_now(now_ms, control_started),
            )?;
            for (dac, sample) in self.dac.iter_mut().zip(&self.output) {
                *dac = (sample * 8388608.).round().clamp(-8388608., 8388607.) as i32 * 256;
            }
            transfer(
                &self.playback,
                &mut self.dac,
                self.topology.playback_channels,
                false,
                playback_deadline,
            )?;
            if !self.started_playback {
                self.playback.start()?;
                self.started_playback = true;
            }
            Ok(())
        })();
        if result.is_err() {
            fault_source(
                source,
                &mut self.faulted,
                &mut self.output,
                &mut self.dac,
                || {
                    let _ = self.capture.drop();
                    let _ = self.playback.drop();
                },
            );
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

fn fault_source<S: SourceBoundary>(
    source: &mut S,
    faulted: &mut bool,
    output: &mut [f64],
    dac: &mut [i32],
    stop: impl FnOnce(),
) {
    *faulted = true;
    output.fill(0.);
    dac.fill(0);
    let _ = source
        .provider_mut()
        .quiesce_source("device_transfer_discontinuity");
    stop();
}

fn source_now(now_ms: u64, control_started: Instant) -> Result<u64> {
    let elapsed = u64::try_from(control_started.elapsed().as_millis())
        .map_err(|_| "lease clock elapsed overflow")?;
    now_ms
        .checked_add(elapsed)
        .ok_or_else(|| "lease clock overflow".into())
}

// The only difference between legacy and authenticated hosting is the existing
// source entry point. Conversion, maps, frame accounting and fault handling stay
// shared. This controller-side boundary is outside the allocation-free DSP core.
trait SourceBoundary {
    fn provider(&self) -> &LocalAudio;
    fn provider_mut(&mut self) -> &mut LocalAudio;
    fn process(
        &mut self,
        now: u64,
        epoch: u64,
        frame: u64,
        capture: &[f64],
        playback: &mut [f64],
    ) -> Result<()>;
}
impl SourceBoundary for LocalAudio {
    fn provider(&self) -> &LocalAudio {
        self
    }
    fn provider_mut(&mut self) -> &mut LocalAudio {
        self
    }
    fn process(
        &mut self,
        now: u64,
        epoch: u64,
        frame: u64,
        capture: &[f64],
        playback: &mut [f64],
    ) -> Result<()> {
        self.tick_with_capture(now, epoch, frame, capture, playback)
            .map_err(Into::into)
    }
}
impl SourceBoundary for crate::remote::HostAuthority {
    fn provider(&self) -> &LocalAudio {
        self.provider()
    }
    fn provider_mut(&mut self) -> &mut LocalAudio {
        self.provider_mut()
    }
    fn process(
        &mut self,
        now: u64,
        epoch: u64,
        frame: u64,
        capture: &[f64],
        playback: &mut [f64],
    ) -> Result<()> {
        self.process_source(now, epoch, frame, capture, playback)
            .map_err(Into::into)
    }
}
/// Shared device-independent source boundary: tests supply captured samples to the
/// same implementation used after a successful raw PCM read, never another mixer.
fn process_captured<S: SourceBoundary>(
    source: &mut S,
    accepted: &EngineTopology,
    capture: &[f64],
    playback: &mut [f64],
    frames: usize,
    mut before_render: impl FnMut(&mut S, u64) -> Result<()>,
    mut clock: impl FnMut() -> Result<u64>,
) -> Result<()> {
    playback.fill(0.);
    let result = (|| -> Result<()> {
        if frames == 0
            || !frames.is_multiple_of(48)
            || frames > accepted.max_block_frames
            || frames.checked_mul(accepted.capture_channels) != Some(capture.len())
            || frames.checked_mul(accepted.playback_channels) != Some(playback.len())
        {
            return Err("captured source period shape".into());
        }
        validate_provider_map(source.provider().topology(), accepted)?;
        source
            .provider_mut()
            .engine_mut()
            .mark_verified_mapping()
            .map_err(|e| e.to_string())?;
        let epoch = source.provider().source_epoch();
        let first = source.provider().frame();
        let expected_end = first
            .checked_add(frames as u64)
            .ok_or("source cursor overflow")?;
        for (input, output) in capture
            .chunks_exact(48 * accepted.capture_channels)
            .zip(playback.chunks_exact_mut(48 * accepted.playback_channels))
        {
            validate_provider_map(source.provider().topology(), accepted)?;
            let frame = source.provider().frame();
            let now = clock()?;
            before_render(source, now)?;
            validate_provider_map(source.provider().topology(), accepted)?;
            if source.provider().frame() != frame || source.provider().source_epoch() != epoch {
                return Err("controller advanced captured source timeline".into());
            }
            source.process(clock()?, epoch, frame, input, output)?;
            // A source-boundary control commit must not change the physical map
            // between admission and submission to the configured device.
            validate_provider_map(source.provider().topology(), accepted)?;
            if source.provider().source_epoch() != epoch
                || source.provider().frame()
                    != frame.checked_add(48).ok_or("source cursor overflow")?
            {
                return Err("shared graph source progression mismatch".into());
            }
            if output.iter().any(|sample| !sample.is_finite()) {
                return Err("nonfinite shared graph playback".into());
            }
        }
        if source.provider().frame() != expected_end {
            return Err("shared graph source progression mismatch".into());
        }
        Ok(())
    })();
    if result.is_err() {
        playback.fill(0.);
        let _ = source
            .provider_mut()
            .quiesce_source("device_source_discontinuity");
    }
    result
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

#[cfg(test)]
mod source_boundary_tests {
    use super::*;
    use crate::{remote::*, show::Counter};
    use std::{
        os::unix::fs::PermissionsExt,
        path::PathBuf,
        sync::atomic::{AtomicUsize, Ordering},
    };

    static NEXT: AtomicUsize = AtomicUsize::new(0);
    struct Fixture {
        host: HostAuthority,
        accepted: EngineTopology,
        path: PathBuf,
    }
    impl Fixture {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "gp-device-source-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir(&path).unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
            let mut accepted = EngineTopology::reference_16_18(0, 4).unwrap();
            // Test-only accepted map; this fixture never opens hardware or claims
            // real socket/clock qualification.
            accepted.mapping_evidence = "operator-verified".into();
            let provider = LocalAudio::bind_configured(
                &path,
                "host",
                "11111111-1111-4111-8111-111111111111",
                Counter(9),
                accepted.clone(),
            )
            .unwrap();
            Self {
                host: HostAuthority::new(provider, 1).unwrap(),
                accepted,
                path,
            }
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
    #[test]
    fn captured_period_uses_authenticated_monitor_path_once_per_48_frames() {
        let mut f = Fixture::new();
        let policy = PolicyStore::new(vec![Peer {
            id: "duplex".into(),
            certificate_sha256: fingerprint(b"device-boundary"),
            permissions: [Permission::LocalOperatorMonitor].into_iter().collect(),
        }])
        .unwrap();
        let context = policy.authenticate(b"device-boundary", 12).unwrap();
        let descriptor = BrainMediaDescriptor {
            contract: "GP15-media".into(),
            version: 1,
            session: 12,
            stagebox_epoch: 9,
            brain_epoch: 77,
            stagebox_map: f.accepted.map_revision,
            brain_map: 1,
            generation: 1,
            selection_generation: f.host.provider().brain_snapshot().selection_generation.0,
            hold_generation: 0,
            sample_rate: 48000,
            frames: 48,
            talkback: false,
            monitor: true,
        };
        f.host.dispatch(&context, serde_json::json!({"contract":"GP15-media","version":1,"kind":"negotiate","writer":context.writer(),"descriptor":descriptor}), 0).unwrap();
        let capture = vec![0.125; 96 * f.accepted.capture_channels];
        let mut playback = vec![f64::NAN; 96 * f.accepted.playback_channels];
        let mut boundaries = Vec::new();
        let mut clock = 1_999;
        process_captured(
            &mut f.host,
            &f.accepted,
            &capture,
            &mut playback,
            96,
            |host, now| {
                boundaries.push((host.provider().frame(), now));
                Ok(())
            },
            || {
                clock += 1;
                Ok(clock)
            },
        )
        .unwrap();
        assert_eq!(boundaries, [(0, 2_000), (48, 2_002)]);
        assert_eq!(f.host.provider().frame(), 96);
        assert_eq!(f.host.media_stats().emitted_monitor_packets, 2);
        for frame in [0, 48] {
            let bytes = f.host.poll_brain_media(&context).unwrap().unwrap();
            let packet = BrainPacket::decode(&bytes, &descriptor, BrainMediaRole::Monitor).unwrap();
            assert_eq!(packet.frame, frame);
            assert!(
                packet.samples.iter().all(|x| *x == 0.),
                "unarmed monitor remains silent"
            );
        }
        assert!(f.host.poll_brain_media(&context).unwrap().is_none());
        assert!(playback.iter().all(|x| x.is_finite()));
        assert!(f.host.provider().brain_snapshot().held_generation.is_none());
    }
    #[test]
    fn rejected_capture_boundary_quiesces_without_advancing_or_leaking_output() {
        for failure in 0..3 {
            let mut f = Fixture::new();
            f.host.provider_mut().rearm().unwrap();
            let capture = vec![0.25; 48 * f.accepted.capture_channels];
            let mut playback = vec![1.; 48 * f.accepted.playback_channels];
            if failure == 0 {
                f.accepted.inputs[0].physical_port.push_str("-changed");
            }
            let result = process_captured(
                &mut f.host,
                &f.accepted,
                &capture,
                &mut playback,
                48,
                |_, _| {
                    if failure == 1 {
                        Err("mailbox fault".into())
                    } else {
                        Ok(())
                    }
                },
                || {
                    if failure == 2 {
                        Err("clock overflow".into())
                    } else {
                        Ok(2_000)
                    }
                },
            );
            assert!(result.is_err());
            assert_eq!(f.host.provider().frame(), 0);
            assert!(f.host.provider_mut().engine_mut().outputs_quiesced());
            let brain = f.host.provider().brain_snapshot();
            assert!(brain.held_generation.is_none() && !brain.monitor_armed);
            assert!(playback.iter().all(|x| *x == 0.));
        }
    }
    #[test]
    fn blocked_capture_or_mailbox_expires_held_talkback_before_render() {
        use crate::{
            brain_control::{Command as BrainCommand, Request as BrainRequest},
            control_model::{Command, Request as AudioRequest, Scope},
        };
        use std::cell::Cell;
        for delay_in_mailbox in [false, true] {
            let mut f = Fixture::new();
            f.host.provider_mut().rearm().unwrap();
            let show = "11111111-1111-4111-8111-111111111111";
            let policy = PolicyStore::new(vec![Peer {
                id: "duplex".into(),
                certificate_sha256: fingerprint(b"held-device-boundary"),
                permissions: [Permission::TalkbackDestinations].into_iter().collect(),
            }])
            .unwrap();
            let context = policy.authenticate(b"held-device-boundary", 12).unwrap();
            let grant = AudioRequest {
                contract: "C-AUDIO".into(),
                version: 2,
                show_id: show.into(),
                module: "audio".into(),
                epoch: Counter(9),
                writer: Some(context.writer().into()),
                lease: None,
                request_id: Some(Counter(1)),
                expected_revision: Some(Counter(0)),
                command: Command::Grant {
                    scope: Scope::TalkbackDestinations,
                },
            };
            let lease = f
                .host
                .provider_mut()
                .engine_mut()
                .handle(&grant, 0)
                .unwrap()
                .outcome
                .unwrap()
                .body
                .granted_lease
                .unwrap();
            let capture = vec![0.; 96 * f.accepted.capture_channels];
            let mut playback = vec![0.; 96 * f.accepted.playback_channels];
            for (id, revision, now, command) in [
                (
                    2,
                    0,
                    1,
                    BrainCommand::TalkbackSet {
                        monitors: vec![0],
                        gain_cdb: 0,
                        mute: false,
                    },
                ),
                (
                    3,
                    1,
                    3,
                    BrainCommand::Hold {
                        generation: Counter(1),
                    },
                ),
            ] {
                let request = BrainRequest {
                    contract: "GP15-brain".into(),
                    version: 1,
                    show_id: show.into(),
                    module: "audio".into(),
                    epoch: Counter(9),
                    writer: Some(context.writer().into()),
                    lease: Some(lease),
                    request_id: Some(Counter(id)),
                    expected_revision: Some(Counter(revision)),
                    command,
                };
                f.host
                    .provider_mut()
                    .brain_request(request, now, true, None)
                    .unwrap();
                process_captured(
                    &mut f.host,
                    &f.accepted,
                    &capture,
                    &mut playback,
                    96,
                    |_, _| Ok(()),
                    || Ok(now),
                )
                .unwrap();
            }
            assert_eq!(
                f.host.provider().brain_snapshot().held_generation,
                Some(Counter(1))
            );
            let descriptor = BrainMediaDescriptor {
                contract: "GP15-media".into(),
                version: 1,
                session: 12,
                stagebox_epoch: 9,
                brain_epoch: 77,
                stagebox_map: f.accepted.map_revision,
                brain_map: 1,
                generation: 1,
                selection_generation: f.host.provider().brain_snapshot().selection_generation.0,
                hold_generation: 1,
                sample_rate: 48000,
                frames: 48,
                talkback: true,
                monitor: false,
            };
            f.host.dispatch(&context, serde_json::json!({"contract":"GP15-media","version":1,"kind":"negotiate","writer":context.writer(),"descriptor":descriptor}), 5).unwrap();
            // Valid microphone packets are ready, so stale time would actually
            // allow the held path to render, not merely pass an empty bridge.
            for block in 0..22 {
                let packet = BrainPacket {
                    descriptor: descriptor.clone(),
                    role: BrainMediaRole::Talkback,
                    frame: 480_000 + block * 48,
                    samples: vec![0.125; 48],
                }
                .encode()
                .unwrap();
                f.host.brain_media(&context, &packet, 5).unwrap();
            }
            assert!(f.host.brain_bridge_status().unwrap().ready);
            let baseline = Cell::new(if delay_in_mailbox {
                Instant::now()
            } else {
                Instant::now() - Duration::from_millis(200)
            });
            let first = f.host.provider().frame();
            process_captured(
                &mut f.host,
                &f.accepted,
                &capture,
                &mut playback,
                96,
                |_, _| {
                    if delay_in_mailbox {
                        baseline.set(Instant::now() - Duration::from_millis(200));
                    }
                    Ok(())
                },
                || source_now(5, baseline.get()),
            )
            .unwrap();
            assert_eq!(f.host.provider().frame(), first + 96);
            assert!(f.host.provider().brain_snapshot().held_generation.is_none());
            assert_eq!(f.host.provider().brain_snapshot().outgoing_peak_nano, 0);
            assert!(playback.iter().all(|sample| *sample == 0.));
        }
    }
    #[test]
    fn prepared_playback_deadline_rejects_stale_and_partial_writes_and_quiesces() {
        use super::super::transfer::TransferError;
        use std::cell::Cell;
        for scenario in 0..3 {
            let captured = Instant::now();
            let deadline = playback_deadline(captured, 48).unwrap();
            assert_eq!(deadline.duration_since(captured), Duration::from_millis(1));
            // Case0 already expired; case1 availability itself crosses deadline;
            // case2 firstpartialwrite succeeds, then its remainder becomes stale.
            let now = Cell::new(if scenario == 0 {
                captured + Duration::from_secs(2)
            } else {
                captured
            });
            let writes = Cell::new(0);
            let result = transfer_before_deadline(
                48,
                deadline,
                || now.get(),
                || {
                    if scenario == 1 {
                        now.set(captured + Duration::from_secs(2));
                    }
                    Ok::<_, ()>(48)
                },
                |offset| {
                    assert_eq!(offset, 0);
                    writes.set(writes.get() + 1);
                    now.set(captured + Duration::from_secs(2));
                    Ok(Some(24))
                },
                |_| panic!("no wait expected for available fake frames"),
            );
            assert_eq!(result, Err(TransferError::Deadline));
            assert_eq!(writes.get(), usize::from(scenario == 2));
            let mut f = Fixture::new();
            f.host.provider_mut().rearm().unwrap();
            let mut faulted = false;
            let mut output = [0.125; 96];
            let mut dac = [1; 96];
            let drops = Cell::new(0);
            // Exactly the cleanup invoked by pump_source on a transfer error.
            fault_source(&mut f.host, &mut faulted, &mut output, &mut dac, || {
                drops.set(drops.get() + 2)
            });
            assert!(faulted);
            assert_eq!(drops.get(), 2);
            assert!(output.iter().all(|x| *x == 0.) && dac.iter().all(|x| *x == 0));
            assert!(f.host.provider_mut().engine_mut().outputs_quiesced());
            assert!(f.host.provider().brain_snapshot().held_generation.is_none());
            assert!(!f.host.provider().brain_snapshot().monitor_armed);
        }
        let captured = Instant::now();
        assert_eq!(
            playback_deadline(captured, 96)
                .unwrap()
                .duration_since(captured),
            Duration::from_millis(2)
        );
    }
}
