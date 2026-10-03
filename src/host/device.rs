//! Explicit ALSA bench host. Driver waits and status queries are outside render.
//! Negotiation/priming follows SHR PA's standalone transport (MIT), retaining its
//! raw-hardware, no-resampling and stop-on-xrun boundaries.
use super::{
    adapters::{Dsp, Recorder, Result},
    network::{self, ClientConfig},
    *,
};
use alsa::{
    Direction, ValueOr,
    pcm::{Access, Format, HwParams, PCM, TstampType},
};
use rtrb::RingBuffer;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaConfig {
    pub device: String,
    pub expected_usb_id: String,
    pub card_stream_file: PathBuf,
    pub capture_with_probe: bool,
    pub stall_device_after_frames: Option<u64>,
    pub seconds: u32,
    pub period: usize,
    pub buffer_periods: u32,
    pub epoch: u64,
    pub return_delay_frames: u32,
    pub pa_library: PathBuf,
    pub recorder_library: PathBuf,
    pub take: PathBuf,
    pub report: PathBuf,
    pub audio_bind: String,
    pub audio_peer: String,
    pub control_bind: String,
    pub control_peer: String,
    pub faults: bool,
}
#[derive(Serialize)]
struct Negotiated {
    rate: u32,
    channels: u32,
    period: usize,
    buffer: usize,
    format: String,
    period_min: i64,
    period_max: i64,
    buffer_min: i64,
    buffer_max: i64,
}
fn open(
    device: &str,
    direction: Direction,
    block: usize,
    buffer: usize,
) -> Result<(PCM, Negotiated)> {
    let pcm = PCM::new(device, direction, true)?;
    let p = HwParams::any(&pcm)?;
    p.set_access(Access::RWInterleaved)?;
    p.set_rate_resample(false)?;
    p.set_format(Format::S32LE)?;
    p.set_channels(2)?;
    p.set_rate(48000, ValueOr::Nearest)?;
    let bounds = (
        p.get_period_size_min()?,
        p.get_period_size_max()?,
        p.get_buffer_size_min()?,
        p.get_buffer_size_max()?,
    );
    p.set_period_size(block as i64, ValueOr::Nearest)?;
    p.set_buffer_size(buffer as i64)?;
    pcm.hw_params(&p)?;
    let actual = pcm.hw_params_current()?;
    let n = Negotiated {
        rate: actual.get_rate()?,
        channels: actual.get_channels()?,
        period: actual.get_period_size()? as usize,
        buffer: actual.get_buffer_size()? as usize,
        format: format!("{:?}", actual.get_format()?),
        period_min: bounds.0,
        period_max: bounds.1,
        buffer_min: bounds.2,
        buffer_max: bounds.3,
    };
    if n.rate != 48000 || n.channels != 2 || n.period != block || n.buffer != buffer {
        return Err("negotiated configuration differs from requested bounds".into());
    }
    let sw = pcm.sw_params_current()?;
    sw.set_start_threshold(sw.get_boundary()?)?;
    sw.set_avail_min(block as i64)?;
    sw.set_tstamp_mode(true)?;
    sw.set_tstamp_type(TstampType::Monotonic)?;
    pcm.sw_params(&sw)?;
    drop(sw);
    drop(actual);
    drop(p);
    pcm.prepare()?;
    Ok((pcm, n))
}
fn transfer(pcm: &PCM, data: &mut [i32], capture: bool) -> Result<()> {
    let io = pcm.io_i32()?;
    let mut offset = 0;
    let begin = Instant::now();
    while offset < data.len() {
        match if capture {
            io.readi(&mut data[offset..])
        } else {
            io.writei(&data[offset..])
        } {
            Ok(0) => {
                pcm.wait(Some(20))?;
            }
            Ok(n) => {
                if n * 2 > data.len() - offset {
                    return Err("invalid ALSA transfer length".into());
                }
                offset += n * 2;
            }
            Err(e) if matches!(e.errno(), 4 | 11) => {
                pcm.wait(Some(20))?;
            }
            Err(e) => return Err(e.into()),
        }
        if begin.elapsed() > Duration::from_secs(2) {
            return Err("ALSA transfer exceeded bounded two-second deadline".into());
        }
    }
    Ok(())
}
pub fn run(c: PaConfig) -> Result<serde_json::Value> {
    let buffer = device_buffer_frames(c.period, c.buffer_periods)
        .map_err(|_| "supported period and explicit four/eight-period buffer required")?;
    if !c.device.starts_with("hw:CARD=")
        || !c.device.ends_with(",DEV=0")
        || !(1..=600).contains(&c.seconds)
        || c.epoch == 0
        || ![384, 768].contains(&c.return_delay_frames)
    {
        return Err("explicit raw card, bounded duration/period and fresh epoch required".into());
    }
    network::validate_addresses(&c.audio_bind, &c.audio_peer)?;
    network::validate_addresses(&c.control_bind, &c.control_peer)?;
    // A caller-supplied descriptor is insufficient: match the selected card ID to
    // the kernel USB descriptor itself before opening either direction.
    let card_id = c
        .device
        .trim_start_matches("hw:CARD=")
        .trim_end_matches(",DEV=0");
    if card_id.is_empty()
        || !card_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        return Err("invalid stable card ID".into());
    }
    let card_path = std::fs::canonicalize(format!("/proc/asound/{card_id}"))?;
    if std::fs::read_to_string(card_path.join("usbid"))?.trim() != c.expected_usb_id {
        return Err("USB identity mismatch".into());
    }
    let stream_path = std::fs::canonicalize(&c.card_stream_file)?;
    if stream_path != card_path.join("stream0") {
        return Err("stream descriptor does not belong to selected device".into());
    }
    let descriptor = std::fs::read_to_string(&stream_path)?;
    if !native_stereo_24(&descriptor) {
        return Err("this host requires native 24-valid-bit S32 USB capture".into());
    }
    let (capture, cn) = open(&c.device, Direction::Capture, c.period, buffer)?;
    let (playback, pn) = open(&c.device, Direction::Playback, c.period, buffer)?;
    let mut pa = Dsp::load(&c.pa_library, "pa", c.period as u32)?;
    let mut final_pa = Dsp::load(&c.pa_library, "pa", c.period as u32)?;
    let mut recorder = Recorder::create(&c.recorder_library, &c.take, c.period as u32, c.epoch)?;
    let (mut send_producer, send_consumer) = RingBuffer::new(256);
    let (return_producer, mut return_consumer) = RingBuffer::new(256);
    let stop = Arc::new(AtomicBool::new(false));
    let ready = Arc::new(AtomicBool::new(false));
    let worker = network::client(
        ClientConfig {
            audio_bind: c.audio_bind,
            audio_peer: c.audio_peer,
            control_bind: c.control_bind,
            control_peer: c.control_peer,
            epoch: c.epoch,
            return_delay_frames: c.return_delay_frames,
            faults: c.faults,
        },
        send_consumer,
        return_producer,
        Arc::clone(&stop),
        Arc::clone(&ready),
    )?;
    let worker = thread::spawn(worker);
    let n = c.period;
    let mut raw = vec![0i32; n * 2];
    let mut dac = vec![0i32; n * 2];
    let mut source = vec![0.; n * 2];
    let mut dry = vec![0.; n * 6];
    let mut wet = vec![0.; n * 2];
    let mut mixed = vec![0.; n * 2];
    let mut protected = vec![0.; n * 6];
    let mut record = vec![0.; n * 8];
    let mut prime = vec![0; (pn.buffer - pn.period) * 2];
    let mut wet_render =
        WetRender::with_delay(c.epoch, c.return_delay_frames).map_err(|_| "wet setup")?;
    let mut hashes: [Sha256; 8] = std::array::from_fn(|_| Sha256::new());
    let mut adc_hashes: [Sha256; 2] = std::array::from_fn(|_| Sha256::new());
    let mut low_byte_histograms = [vec![0u64; 256], vec![0u64; 256]];
    let mut dsp_time = Histogram::default();
    let mut cycle_time = Histogram::default();
    let mut read_time = Histogram::default();
    let mut write_time = Histogram::default();
    let mut interval = Histogram::default();
    let mut frames = 0u64;
    let mut captured_frames = 0u64;
    let mut record_accepted_frames = 0u64;
    let mut network_drops = 0;
    let mut recorder_drops = 0;
    let mut low_byte_nonzero = 0u64;
    let mut clipped = [0u64; 2];
    let mut peaks = [0.0f64; 8];
    let mut occupancy = 0;
    let mut capture_avail_max = 0;
    let mut playback_delay_min = i64::MAX;
    let mut playback_delay_max = 0;
    let mut timestamps = Vec::with_capacity(c.seconds as usize + 4);
    let start = Instant::now();
    let mut previous = None;
    let mut xruns = 0;
    let mut failed_playback_cycle_us = None;
    eprintln!(
        "ready pid={} device={} period={} buffer={} epoch={}",
        std::process::id(),
        c.device,
        n,
        pn.buffer,
        c.epoch
    );
    let outcome: Result<()> = (|| {
        let ready_deadline = Instant::now();
        while !ready.load(Ordering::Acquire) {
            if ready_deadline.elapsed() > Duration::from_secs(2) {
                return Err("Brain fresh control snapshot not ready".into());
            }
            thread::sleep(Duration::from_millis(2));
        }
        transfer(&playback, &mut prime, false)?;
        playback.start()?;
        capture.start()?;
        while frames < u64::from(c.seconds) * 48000 {
            let t = Instant::now();
            transfer(&capture, &mut raw, true)?;
            captured_frames += n as u64;
            read_time.observe(t.elapsed().as_secs_f64() * 1e6);
            let cycle = Instant::now();
            if let Some(p) = previous {
                interval.observe(cycle.duration_since(p).as_secs_f64() * 1e6);
            }
            previous = Some(cycle);
            for _ in 0..64 {
                let Ok(d) = return_consumer.pop() else { break };
                let _ = wet_render.admit(&d.bytes[..d.len]);
            }
            occupancy = occupancy.max(wet_render.occupancy());
            for (f, pair) in source.chunks_exact_mut(2).enumerate() {
                for ch in 0..2 {
                    let captured = if c.capture_with_probe {
                        capture_msb24(raw[f * 2 + ch]) * 0.125
                    } else {
                        0.
                    };
                    pair[ch] = ((stimulus(frames + f as u64, ch) + captured) * 8388608.).round()
                        / 8388608.;
                }
            }
            // Render section: bounded owner calls, ring operations and arithmetic only.
            let t = Instant::now();
            let render_result = (|| -> std::result::Result<(), &'static str> {
                if !pa.process(&source, &mut dry, 6) {
                    return Err("PA fault");
                }
                wet_render
                    .render(frames, &mut wet)
                    .map_err(|_| "wet timeline fault")?;
                for (index, pair) in source.chunks_exact(96).enumerate() {
                    let mut d = Datagram::default();
                    let mut wire = [0f32; 96];
                    for (x, s) in wire.iter_mut().zip(pair) {
                        *x = *s as f32;
                    }
                    let f = frames + (index * 48) as u64;
                    d.len = spec(c.epoch, crate::transport::Role::FxSend)
                        .encode_float(f, (f / 48) as u32, &wire, &mut d.bytes)
                        .map_err(|_| "send format")?;
                    if send_producer.push(d).is_err() {
                        network_drops += 1;
                    }
                }
                for i in 0..n * 2 {
                    mixed[i] = source[i] + wet[i] * 0.25;
                }
                if !final_pa.process(&mixed, &mut protected, 6) {
                    return Err("final PA protection fault");
                }
                for f in 0..n {
                    for ch in 0..2 {
                        let adc = raw[f * 2 + ch];
                        low_byte_nonzero += u64::from(adc & 255 != 0);
                        clipped[ch] += u64::from(capture_fullscale(adc));
                        let output = protected[f * 6 + ch].clamp(-0.25, 0.25);
                        dac[f * 2 + ch] =
                            (output * 8388608.).round().clamp(-8388608., 8388607.) as i32 * 256;
                        record[f * 8 + ch] = capture_msb24(adc);
                        record[f * 8 + 2 + ch] = source[f * 2 + ch];
                        record[f * 8 + 4 + ch] = dry[f * 6 + ch];
                        record[f * 8 + 6 + ch] = f64::from(dac[f * 2 + ch]) / 2147483648.;
                    }
                }
                let pushed = recorder.push(frames, &record);
                if pushed != 0 {
                    recorder_drops += 1;
                    if pushed < 0 {
                        return Err("recorder worker fault");
                    }
                }
                if pushed == 0 {
                    record_accepted_frames += n as u64;
                }
                Ok(())
            })();
            dsp_time.observe(t.elapsed().as_secs_f64() * 1e6);
            render_result?;
            // Audit bytes are exact PCM24 submitted to the recorder; file verification is
            // performed independently after finalization, never inferred from counters.
            for row in raw.chunks_exact(2) {
                for ch in 0..2 {
                    let bytes = row[ch].to_le_bytes();
                    adc_hashes[ch].update(&bytes[1..]);
                    low_byte_histograms[ch][usize::from(bytes[0])] += 1;
                }
            }
            for row in record.chunks_exact(8) {
                for (ch, x) in row.iter().enumerate() {
                    peaks[ch] = peaks[ch].max(x.abs());
                    let pcm = (*x * 8388608.).round().clamp(-8388608., 8388607.) as i32;
                    hashes[ch].update(&pcm.to_le_bytes()[..3]);
                }
            }
            let t = Instant::now();
            let write_result = transfer(&playback, &mut dac, false);
            write_time.observe(t.elapsed().as_secs_f64() * 1e6);
            if let Err(error) = write_result {
                let elapsed = cycle.elapsed().as_secs_f64() * 1e6;
                cycle_time.observe(elapsed);
                failed_playback_cycle_us = Some(elapsed);
                return Err(error);
            }
            frames += n as u64;
            let cs = capture.status()?;
            let ps = playback.status()?;
            capture_avail_max = capture_avail_max.max(cs.get_avail_max());
            playback_delay_min = playback_delay_min.min(ps.get_delay());
            playback_delay_max = playback_delay_max.max(ps.get_delay());
            if frames % 48000 < (n as u64) {
                let stamp = cs.get_htstamp();
                let audio = cs.get_audio_htstamp();
                timestamps.push(json!({"frame":frames,"monotonic_elapsed_s":start.elapsed().as_secs_f64(),"capture_status_sec":stamp.tv_sec,"capture_status_ns":stamp.tv_nsec,"audio_sec":audio.tv_sec,"audio_ns":audio.tv_nsec,"capture_avail":cs.get_avail(),"playback_delay":ps.get_delay()}));
            }
            cycle_time.observe(cycle.elapsed().as_secs_f64() * 1e6);
            if c.stall_device_after_frames == Some(frames) {
                thread::sleep(Duration::from_millis(100));
            }
        }
        // Bounded quiet shutdown: continue clocked transfers until queued audio clears.
        dac.fill(0);
        for _ in 0..c.buffer_periods + 1 {
            transfer(&capture, &mut raw, true)?;
            transfer(&playback, &mut dac, false)?;
        }
        Ok(())
    })();
    if let Err(ref e) = outcome {
        if e.downcast_ref::<alsa::Error>()
            .is_some_and(|e| matches!(e.errno(), 32 | 86))
        {
            xruns += 1;
        }
        recorder.fault(2);
    }
    let _ = capture.drop();
    let _ = playback.drop();
    stop.store(true, Ordering::Release);
    let net = worker.join().map_err(|_| "network worker panic")?;
    let finalization = recorder.finish();
    let mut report = json!({"capture_with_probe":c.capture_with_probe,"epoch":c.epoch,"device":c.device,"capture":cn,"playback":pn,"source_valid_bits":24,"usb_descriptor_bits":24,"alsa_container_bits":32,"adc_native_msb24_sha256":adc_hashes.map(|h|format!("{:x}",h.finalize())),"low_byte_histograms":low_byte_histograms,"return_delay_frames":c.return_delay_frames,"frames":frames,"captured_frames":captured_frames,"record_accepted_frames":record_accepted_frames,"playback_complete_block_frames":frames,"elapsed_s":start.elapsed().as_secs_f64(),"fault":outcome.err().map(|e|e.to_string()),"xruns":xruns,"record_finalization":finalization,"record_queue_drops":recorder_drops,"network_queue_drops":network_drops,"low_byte_nonzero":low_byte_nonzero,"adc_fullscale_samples":clipped,"peaks":peaks,"record_pcm_sha256":hashes.map(|h|format!("{:x}",h.finalize())),"record_channels":["adc_left","adc_right","pa_source_left","pa_source_right","pa_dry_left","pa_dry_right","dac_submitted_left","dac_submitted_right"],"wet_present_packets":wet_render.present,"wet_missing_packets":wet_render.missing,"wet_expired":wet_render.expired,"wet_rejected":wet_render.rejected,"wet_peak_occupancy":occupancy,"render":dsp_time.summary(),"cycle":cycle_time.summary(),"read_wait":read_time.summary(),"write_wait":write_time.summary(),"capture_interval":interval.summary(),"capture_avail_max":capture_avail_max,"playback_delay_min":playback_delay_min,"playback_delay_max":playback_delay_max,"timestamps":timestamps,"network":net});
    report["failed_playback_cycle_us"] = json!(failed_playback_cycle_us);
    Ok(report)
}
