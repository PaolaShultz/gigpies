//! C-ANALYSIS:1 named raw input tap. Construction/control are outside callbacks.
use crate::transport::{Encoding, Packet, Role, StreamSpec};
use rtrb::{Consumer, Producer, RingBuffer};
use serde::{Deserialize, Serialize};
pub const PACKET_BYTES: usize = 624;
pub const WINDOW_BYTES: usize = PACKET_BYTES * 10;
pub const MAX_SUBSCRIBERS: usize = 8;
pub const WIRE_BYTES: usize = WINDOW_BYTES + 40;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Descriptor {
    pub contract: String,
    pub version: u32,
    pub subscription: String,
    pub source_epoch: crate::show::Counter,
    pub stream: u32,
    pub sample_rate: u32,
    pub first_frame: crate::show::Counter,
    pub sources: [String; 4],
    pub inputs: [String; 4],
    pub tap: String,
    pub clock: String,
    pub map_revision: crate::show::Counter,
    pub calibration_revision: crate::show::Counter,
}
impl Descriptor {
    pub fn new(epoch: u64, frame: u64) -> Self {
        Self {
            contract: "C-ANALYSIS".into(),
            version: 1,
            subscription: "lux.aux.v1".into(),
            source_epoch: crate::show::Counter(epoch),
            stream: 3,
            sample_rate: 48000,
            first_frame: crate::show::Counter(frame),
            sources: ["kick", "bass", "guitar-1", "guitar-2"].map(String::from),
            inputs: ["input-01", "input-02", "input-03", "input-04"].map(String::from),
            tap: "raw-pre-fader".into(),
            clock: "linux-clock-monotonic-ms".into(),
            map_revision: crate::show::Counter(1),
            calibration_revision: crate::show::Counter(2),
        }
    }
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.contract != "C-ANALYSIS"
            || self.version != 1
            || self.subscription != "lux.aux.v1"
            || self.source_epoch.0 == 0
            || self.stream != 3
            || self.sample_rate != 48000
            || !self.first_frame.0.is_multiple_of(48)
            || self.sources != ["kick", "bass", "guitar-1", "guitar-2"]
            || self.clock != "linux-clock-monotonic-ms"
            || self.tap != "raw-pre-fader"
            || self.map_revision.0 == 0
            || self.calibration_revision.0 == 0
        {
            return Err("descriptor identity/rate");
        }
        for (i, input) in self.inputs.iter().enumerate() {
            if !(1..=8).any(|n| input == &format!("input-{n:02}"))
                || self.inputs[..i].contains(input)
            {
                return Err("explicit input mapping");
            }
        }
        Ok(())
    }
    pub fn spec(&self) -> StreamSpec {
        StreamSpec {
            session: self.source_epoch.0,
            stream: 3,
            first_channel: 0,
            channels: 4,
            frames: 48,
            encoding: Encoding::Pcm24,
            role: Role::Analysis,
            delay_frames: 0,
        }
    }
}
#[derive(Clone, Copy)]
pub struct Window {
    pub packets: [u8; WINDOW_BYTES],
    pub produced_mono_ms: u64,
}
impl Window {
    pub fn validate(
        &self,
        descriptor: &Descriptor,
        expected_frame: u64,
        expected_sequence: u32,
    ) -> Result<(), &'static str> {
        descriptor.validate()?;
        for (i, bytes) in self.packets.chunks_exact(PACKET_BYTES).enumerate() {
            let p = Packet::parse(bytes).map_err(|_| "packet")?;
            if p.spec() != descriptor.spec()
                || p.source_frame()
                    != expected_frame
                        .checked_add(i as u64 * 48)
                        .ok_or("frame overflow")?
                || p.sequence()
                    != expected_sequence
                        .checked_add(i as u32)
                        .ok_or("sequence overflow")?
            {
                return Err("window identity/timeline");
            }
        }
        Ok(())
    }
    pub fn wire(&self, losses: u64, map: u64, calibration: u64) -> [u8; WIRE_BYTES] {
        let mut out = [0; WIRE_BYTES];
        out[..4].copy_from_slice(b"GAW1");
        out[4..12].copy_from_slice(&losses.to_be_bytes());
        out[12..20].copy_from_slice(&map.to_be_bytes());
        out[20..28].copy_from_slice(&calibration.to_be_bytes());
        out[28..36].copy_from_slice(&self.produced_mono_ms.to_be_bytes());
        out[40..].copy_from_slice(&self.packets);
        out
    }
}
/// Fixed-storage raw tap; offer does no allocation, locks, I/O or retries.
pub struct Tap {
    spec: StreamSpec,
    mapping: [usize; 4],
    frame: u64,
    sequence: u32,
    count: usize,
    last_mono_ms: Option<u64>,
    window: Window,
    output: Producer<Window>,
    pub dropped_windows: u64,
}
impl Tap {
    pub fn new(d: &Descriptor) -> Result<(Self, Consumer<Window>), &'static str> {
        d.validate()?;
        let mapping = std::array::from_fn(|i| d.inputs[i][6..].parse::<usize>().unwrap() - 1);
        let (output, consumer) = RingBuffer::new(2);
        Ok((
            Self {
                spec: d.spec(),
                mapping,
                frame: d.first_frame.0,
                sequence: 0,
                count: 0,
                last_mono_ms: None,
                window: Window {
                    packets: [0; WINDOW_BYTES],
                    produced_mono_ms: 0,
                },
                output,
                dropped_windows: 0,
            },
            consumer,
        ))
    }
    pub fn offer(
        &mut self,
        frame: u64,
        raw: &[[i32; 8]; 48],
        produced_mono_ms: u64,
    ) -> Result<(), &'static str> {
        if frame != self.frame {
            self.count = 0;
            return Err("tap timeline");
        }
        if self
            .last_mono_ms
            .is_some_and(|previous| produced_mono_ms < previous)
        {
            self.count = 0;
            return Err("tap clock regression");
        }
        let next_frame = frame.checked_add(48).ok_or("frame exhausted")?;
        let next_sequence = self.sequence.checked_add(1).ok_or("sequence exhausted")?;
        let mut samples = [0; 192];
        for (f, inputs) in raw.iter().enumerate() {
            for (c, &input) in self.mapping.iter().enumerate() {
                samples[f * 4 + c] = inputs[input];
            }
        }
        self.spec
            .encode_pcm(
                frame,
                self.sequence,
                &samples,
                &mut self.window.packets
                    [self.count * PACKET_BYTES..(self.count + 1) * PACKET_BYTES],
            )
            .map_err(|_| "PCM24")?;
        if self.count == 0 {
            self.window.produced_mono_ms = produced_mono_ms;
        }
        self.last_mono_ms = Some(produced_mono_ms);
        self.frame = next_frame;
        self.sequence = next_sequence;
        self.count += 1;
        if self.count == 10 {
            if self.output.push(self.window).is_err() {
                self.dropped_windows = self.dropped_windows.saturating_add(1);
            }
            self.count = 0;
        }
        Ok(())
    }
}
/// Invented exact signed PCM24 inputs, shared by synthetic mixer and raw tap.
pub fn synthetic_inputs(frame: u64) -> [[i32; 8]; 48] {
    std::array::from_fn(|f| {
        std::array::from_fn(|c| {
            let n = ((frame % 4096 + f as u64) % 4096) as i32;
            (n - 2048) * (c as i32 + 1) * 32
        })
    })
}
#[cfg(target_os = "linux")]
pub mod local;

/// Validates an attachment's windows without interpreting missing audio as silence.
/// On any refusal discard this cursor and reattach/recalibrate before use.
pub struct WindowCursor {
    descriptor: Descriptor,
    next: Option<(u64, u32)>,
    losses: u64,
    invalid: bool,
    last_now: Option<u64>,
    last_produced: Option<u64>,
}
impl WindowCursor {
    pub fn new(descriptor: Descriptor) -> Result<Self, &'static str> {
        descriptor.validate()?;
        Ok(Self {
            descriptor,
            next: None,
            losses: 0,
            invalid: false,
            last_now: None,
            last_produced: None,
        })
    }
    pub fn accept(&mut self, bytes: &[u8], now_mono_ms: u64) -> Result<Window, &'static str> {
        let result = self.check(bytes, now_mono_ms);
        if result.is_err() {
            self.invalid = true;
        }
        result
    }
    fn check(&mut self, bytes: &[u8], now: u64) -> Result<Window, &'static str> {
        if self.invalid
            || bytes.len() != WIRE_BYTES
            || &bytes[..4] != b"GAW1"
            || bytes[36..40] != [0; 4]
        {
            return Err("window framing/invalid cursor");
        }
        let n = |i| u64::from_be_bytes(bytes[i..i + 8].try_into().unwrap());
        if self.last_now.is_some_and(|previous| now < previous) {
            return Err("receipt clock regression");
        }
        let losses = n(4);
        let produced = n(28);
        if self
            .last_produced
            .is_some_and(|previous| produced < previous)
        {
            return Err("source clock regression");
        }
        if n(12) != self.descriptor.map_revision.0
            || n(20) != self.descriptor.calibration_revision.0
            || produced > now
            || now - produced > 100
            || losses < self.losses
        {
            return Err("window map/calibration/age/loss");
        }
        let mut w = Window {
            packets: [0; WINDOW_BYTES],
            produced_mono_ms: produced,
        };
        w.packets.copy_from_slice(&bytes[40..]);
        let first = Packet::parse(&w.packets[..PACKET_BYTES]).map_err(|_| "packet")?;
        let frame = first.source_frame();
        let sequence = first.sequence();
        let offset = frame
            .checked_sub(self.descriptor.first_frame.0)
            .ok_or("stale origin")?;
        if offset / 48 != u64::from(sequence) || !offset.is_multiple_of(480) {
            return Err("source origin/sequence");
        }
        if let Some((f, s)) = self.next
            && (frame != f || sequence != s || losses != self.losses)
        {
            return Err("gap/overlap/loss");
        }
        w.validate(&self.descriptor, frame, sequence)?;
        self.next = Some((
            frame.checked_add(480).ok_or("frame exhausted")?,
            sequence.checked_add(10).ok_or("sequence exhausted")?,
        ));
        self.losses = losses;
        self.last_now = Some(now);
        self.last_produced = Some(produced);
        Ok(w)
    }
}
