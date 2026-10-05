//! Brain follows source-indexed complete send blocks. It has no audio device or
//! free-running render clock, and calls the actual SHR FX library through its
//! existing host adapter. All allocation/network operations are worker-side.
use super::*;
use crate::transport::{Packet, Role};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
struct Block {
    samples: Vec<f64>,
    groups: BTreeSet<u32>,
}
pub struct GroupedReceiver {
    descriptor: MediaDescriptor,
    role: MediaRole,
    blocks: BTreeMap<u64, Block>,
    next: Option<u64>,
    pub incomplete_dropped: u64,
}
impl GroupedReceiver {
    pub fn new(descriptor: MediaDescriptor, role: MediaRole) -> Result<Self> {
        let specs = descriptor
            .streams
            .iter()
            .map(|s| s.spec(descriptor.session.0))
            .collect::<Result<Vec<_>>>()?;
        crate::transport::validate_session(&specs)
            .map_err(|e| format!("group descriptor {e:?}"))?;
        let channels = descriptor.channels(role);
        if channels == 0 || channels > 256 || descriptor.streams.len() > 64 {
            return Err("group receiver capacity".into());
        }
        Ok(Self {
            descriptor,
            role,
            blocks: BTreeMap::new(),
            next: None,
            incomplete_dropped: 0,
        })
    }
    pub fn push(&mut self, packet: Packet<'_>) -> Result<Option<(u64, Vec<f64>)>> {
        let stream = self
            .descriptor
            .streams
            .iter()
            .find(|s| s.stream == packet.spec().stream && s.role == self.role)
            .ok_or("group stream")?;
        if stream.spec(self.descriptor.session.0)? != packet.spec()
            || self.next.is_some_and(|n| packet.source_frame() < n)
        {
            return Err("stale or mismatched group".into());
        }
        if self.blocks.len() == 32 && !self.blocks.contains_key(&packet.source_frame()) {
            self.blocks.pop_first();
            self.incomplete_dropped = self.incomplete_dropped.saturating_add(1);
        }
        let channels = self.descriptor.channels(self.role);
        let block = self
            .blocks
            .entry(packet.source_frame())
            .or_insert_with(|| Block {
                samples: vec![0.; channels * usize::from(packet.spec().frames)],
                groups: BTreeSet::new(),
            });
        if !block.groups.insert(packet.spec().stream) {
            return Err("duplicate group".into());
        }
        for frame in 0..usize::from(packet.spec().frames) {
            for channel in 0..usize::from(packet.spec().channels) {
                block.samples
                    [frame * channels + usize::from(packet.spec().first_channel) + channel] =
                    packet
                        .sample(frame * usize::from(packet.spec().channels) + channel)
                        .map_err(|e| format!("group sample {e:?}"))?;
            }
        }
        let expected = self
            .descriptor
            .streams
            .iter()
            .filter(|s| s.role == self.role)
            .count();
        let complete = self
            .blocks
            .iter()
            .find(|(_, block)| block.groups.len() == expected)
            .map(|(frame, _)| *frame);
        let Some(frame) = complete else {
            return Ok(None);
        };
        let block = self.blocks.remove(&frame).ok_or("group completion")?;
        while self
            .blocks
            .first_key_value()
            .is_some_and(|(old, _)| *old < frame)
        {
            self.blocks.pop_first();
            self.incomplete_dropped = self.incomplete_dropped.saturating_add(1);
        }
        self.next = Some(
            frame
                .checked_add(u64::from(packet.spec().frames))
                .ok_or("source frame exhausted")?,
        );
        Ok(Some((frame, block.samples)))
    }
}
#[derive(Debug, Default, Clone)]
pub struct BrainStats {
    pub analysis_packets: u64,
    pub send_packets: u64,
    pub processed_blocks: u64,
    pub returned_packets: u64,
    pub stale_groups: u64,
    pub incomplete_blocks: u64,
}
/// A caller may consume analysis packets through `step` while FX remains backed
/// by the independent owner. Analysis consumers cannot mutate the source cursor.
pub struct BrainWorker {
    media: MediaChannel,
    receiver: GroupedReceiver,
    fx: crate::host::brain_fx::BrainFx,
    wet: Vec<f64>,
    pub stats: BrainStats,
    returned_hash: Sha256,
    nonzero_return_samples: u64,
}
impl BrainWorker {
    pub fn prepare(media: MediaChannel, fx_library: &Path) -> Result<Self> {
        let descriptor = media.descriptor().clone();
        let channels = descriptor.channels(MediaRole::FxSend);
        let frames = descriptor
            .streams
            .iter()
            .find(|s| s.role == MediaRole::FxSend)
            .ok_or("FX send not negotiated")?
            .frames;
        if descriptor.channels(MediaRole::WetReturn) != channels {
            return Err("FX return dimensions".into());
        }
        let fx = crate::host::brain_fx::BrainFx::prepare(
            fx_library,
            channels,
            usize::from(frames),
            descriptor.source_epoch.0,
        )
        .map_err(|e| e.to_string())?;
        Ok(Self {
            media,
            receiver: GroupedReceiver::new(descriptor, MediaRole::FxSend)?,
            fx,
            wet: vec![0.; usize::from(frames) * channels],
            stats: BrainStats::default(),
            returned_hash: Sha256::new(),
            nonzero_return_samples: 0,
        })
    }
    pub fn returned_sha256(&self) -> String {
        format!("{:x}", self.returned_hash.clone().finalize())
    }
    pub fn nonzero_return_samples(&self) -> u64 {
        self.nonzero_return_samples
    }
    pub fn intentional_delay_frames(&self) -> u32 {
        self.fx.intentional_delay_frames()
    }
    pub fn owner_resets(&self) -> u64 {
        self.fx.resets
    }
    pub async fn step(&mut self) -> Result<Option<Vec<u8>>> {
        let bytes = self.media.receive().await?;
        let packet = Packet::parse(&bytes).map_err(|e| format!("Brain packet {e:?}"))?;
        if packet.spec().role == Role::Analysis {
            self.stats.analysis_packets = self.stats.analysis_packets.saturating_add(1);
            return Ok(Some(bytes));
        }
        self.stats.send_packets = self.stats.send_packets.saturating_add(1);
        let assembled = match self.receiver.push(packet) {
            Ok(block) => block,
            Err(_) => {
                self.stats.stale_groups = self.stats.stale_groups.saturating_add(1);
                return Ok(None);
            }
        };
        if let Some((frame, input)) = assembled {
            let descriptor = self.media.descriptor();
            self.fx
                .process(descriptor.source_epoch.0, frame, &input, &mut self.wet)
                .map_err(|e| format!("Brain FX {e:?}"))?;
            let packets = encode_grouped(descriptor, MediaRole::WetReturn, frame, &self.wet)?;
            for packet in packets {
                let parsed =
                    Packet::parse(&packet).map_err(|e| format!("return evidence {e:?}"))?;
                for index in 0..parsed.spec().samples() {
                    self.nonzero_return_samples +=
                        u64::from(parsed.sample(index).is_ok_and(|sample| sample != 0.));
                }
                self.returned_hash.update(&packet);
                self.media.send(packet)?;
                self.stats.returned_packets = self.stats.returned_packets.saturating_add(1);
            }
            self.stats.processed_blocks = self.stats.processed_blocks.saturating_add(1);
            self.stats.incomplete_blocks = self.receiver.incomplete_dropped;
        }
        Ok(None)
    }
}
