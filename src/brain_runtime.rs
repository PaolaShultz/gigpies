//! Explicit Brain duplex process. Network/control work surrounds the same bounded
//! device callbacks for fake and physical adapters; FX has a separate source owner.
use crate::{
    brain_audio::{
        bridge::{Bridge, BridgeConfig, BridgeEpochs},
        device::{AlsaDuplex, DeviceConfig, DeviceError, DuplexDevice, FakeDuplex},
        host::{BrainHost, BrainRender, LocalLevels},
    },
    brain_control::{Command, Request, Snapshot},
    device_epoch::Epoch,
    remote::*,
    show::Counter,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs::OpenOptions,
    io::Write,
    net::SocketAddr,
    path::PathBuf,
    time::{Duration, Instant},
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub bind: SocketAddr,
    pub peer: SocketAddr,
    pub server_name: String,
    pub certificate: PathBuf,
    pub key: PathBuf,
    pub ca: PathBuf,
    pub peers: Vec<Peer>,
    pub show_id: String,
    pub device: DeviceConfig,
    #[serde(default)]
    pub restore_intent: bool,
    pub physical: bool,
    pub epoch_file: PathBuf,
    pub duration_ms: u64,
    pub fake_skew_ppm: i32,
    pub report: PathBuf,
}
enum Device {
    Fake(FakeDuplex),
    Physical(AlsaDuplex),
    Closed(crate::brain_audio::device::DeviceCapabilities),
}
impl DuplexDevice for Device {
    fn capabilities(&self) -> &crate::brain_audio::device::DeviceCapabilities {
        match self {
            Self::Fake(d) => d.capabilities(),
            Self::Physical(d) => d.capabilities(),
            Self::Closed(c) => c,
        }
    }
    fn start(&mut self) -> std::result::Result<(), DeviceError> {
        match self {
            Self::Fake(d) => d.start(),
            Self::Physical(d) => d.start(),
            Self::Closed(_) => Err(DeviceError::NotArmed),
        }
    }
    fn read_capture(&mut self, out: &mut [f64]) -> std::result::Result<usize, DeviceError> {
        match self {
            Self::Fake(d) => d.read_capture(out),
            Self::Physical(d) => d.read_capture(out),
            Self::Closed(_) => Err(DeviceError::NotArmed),
        }
    }
    fn write_playback(&mut self, input: &[f64]) -> std::result::Result<usize, DeviceError> {
        match self {
            Self::Fake(d) => d.write_playback(input),
            Self::Physical(d) => d.write_playback(input),
            Self::Closed(_) => Err(DeviceError::NotArmed),
        }
    }
    fn stop(&mut self) {
        match self {
            Self::Fake(d) => d.stop(),
            Self::Physical(d) => d.stop(),
            Self::Closed(_) => (),
        }
    }
}
fn device(config: &DeviceConfig, physical: bool) -> Result<Device> {
    if physical {
        AlsaDuplex::open_detailed(config)
            .map(Device::Physical)
            .map_err(|e| e.to_string())
    } else {
        FakeDuplex::prepare(config)
            .map(Device::Fake)
            .map_err(|e| e.to_string())
    }
}
#[derive(Clone, Copy)]
struct Captured {
    epoch: u64,
    frame: u64,
    hold_generation: u64,
    samples: [f64; 48],
}
struct Renderer {
    captured: rtrb::Producer<Captured>,
    monitor: Bridge,
    epochs: BridgeEpochs,
    fault: Option<DeviceError>,
    dropped: u64,
    hold_generation: u64,
}
impl Renderer {
    /// Only a held-action change retires queued microphone samples. Monitor
    /// negotiation must not introduce a hole in the independent talkback stream.
    fn observe_hold(&mut self, hold: u64, captured: &mut rtrb::Consumer<Captured>) {
        if self.hold_generation != hold {
            self.hold_generation = 0;
            while captured.pop().is_ok() {}
        }
    }
}
fn monitor_selection_current(selection: Option<u64>, descriptor: &BrainMediaDescriptor) -> bool {
    selection == Some(descriptor.selection_generation)
}
fn accepted_capture_hold(hold: Option<u64>, descriptor: &BrainMediaDescriptor) -> u64 {
    if hold == Some(descriptor.hold_generation) {
        descriptor.hold_generation
    } else {
        0
    }
}
impl BrainRender for Renderer {
    fn capture(&mut self, epoch: u64, first: u64, mono: &[f64]) {
        if self.hold_generation == 0 {
            return;
        }
        for (n, chunk) in mono.chunks_exact(48).enumerate() {
            let mut samples = [0.; 48];
            samples.copy_from_slice(chunk);
            if self
                .captured
                .push(Captured {
                    epoch,
                    frame: first + n as u64 * 48,
                    hold_generation: self.hold_generation,
                    samples,
                })
                .is_err()
            {
                self.dropped += 1;
            }
        }
    }
    fn playback(&mut self, epoch: u64, first: u64, stereo: &mut [f64]) {
        if epoch != self.epochs.destination {
            stereo.fill(0.);
            return;
        }
        let _ = self.monitor.render(self.epochs, first, stereo);
    }
    fn fault(&mut self, _: u64, error: DeviceError) {
        self.fault = Some(error);
        self.monitor.invalidate();
    }
}
fn query(show: &str, epoch: Counter) -> Result<Value> {
    serde_json::to_value(Request {
        contract: "GP15-brain".into(),
        version: 1,
        show_id: show.into(),
        module: "audio".into(),
        epoch,
        writer: None,
        lease: None,
        request_id: None,
        expected_revision: None,
        command: Command::BrainSnapshot {},
    })
    .map_err(|e| e.to_string())
}
fn observation(
    host: &BrainHost<Device>,
    renderer: &Renderer,
    ticket: Option<u64>,
    success: Option<bool>,
    error: Option<String>,
) -> BrainDeviceObservation {
    let status = host.status();
    let telemetry = brain_device_telemetry(&status, &renderer.monitor.status(), renderer.dropped);
    BrainDeviceObservation {
        brain_epoch: status.readback.epoch,
        brain_map: status.readback.configuration_generation,
        frame: status.capture_frame,
        config: host.config().clone(),
        status: telemetry,
        ticket,
        success,
        error,
    }
}
/// A device epoch must first be acknowledged by the authority, then observed
/// closed, before a later explicit control revision may make it audible.
#[derive(Default)]
struct ArmFence {
    acknowledged: bool,
    closed_revision: Option<u64>,
}
impl ArmFence {
    fn observe(&mut self, revision: u64, active: bool) -> bool {
        if !self.acknowledged {
            return false;
        }
        if !active {
            self.closed_revision = Some(revision);
            return true;
        }
        self.closed_revision.is_some_and(|closed| revision > closed)
    }
}
#[derive(Default, Serialize)]
struct Evidence {
    captured_packets: u64,
    monitor_packets: u64,
    rejected_monitor_packets: u64,
    playback_frames: u64,
    playback_energy: f64,
    playback_peak: f64,
    first_playback: Vec<f64>,
    negotiations: u64,
    configuration_changes: u64,
    playback_windows: Vec<Value>,
    playback_windows_overflow: u64,
}

fn validate_run_bounds(
    duration_ms: u64,
    fake_skew_ppm: i32,
    physical: bool,
    physical_authorized: bool,
) -> Result<()> {
    let maximum_ms = if physical { 86_400_000 } else { 60_000 };
    if !(1..=maximum_ms).contains(&duration_ms) || !(-2000..=2000).contains(&fake_skew_ppm) {
        return Err("bounded duration/skew required".into());
    }
    if physical && !physical_authorized {
        return Err("physical duplex requires explicit --activate-physical".into());
    }
    Ok(())
}

/// `physical_authorized` comes only from the explicit command-line activation
/// switch. The default fake mode cannot turn into physical mode through Desk.
pub async fn run(mut config: Config, physical_authorized: bool) -> Result<Value> {
    validate_run_bounds(
        config.duration_ms,
        config.fake_skew_ppm,
        config.physical,
        physical_authorized,
    )?;
    if config.restore_intent {
        let path = config
            .epoch_file
            .parent()
            .ok_or("intent parent")?
            .join("brain-device-intent.json");
        let metadata = std::fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
        if !metadata.is_file() || metadata.len() > 65_536 {
            return Err("intent type/capacity".into());
        }
        let saved: DeviceConfig =
            serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        if saved.device_id != config.device.device_id || saved.endpoint != config.device.endpoint {
            return Err("saved intent device mismatch; explicit remap required".into());
        }
        config.device = saved;
    }
    config.device.validate().map_err(|e| e.to_string())?;
    let mut epoch = Epoch::open(&config.epoch_file)?;
    let mut host = BrainHost::prepare(
        device(&config.device, config.physical)?,
        config.device.clone(),
        epoch.value,
        epoch.value,
    )
    .map_err(|e| e.to_string())?;
    let credentials = Credentials::load_der(&config.certificate, &config.key, &config.ca)?;
    let mut client = RemoteClient::connect(
        client_endpoint(config.bind, &credentials)?,
        config.peer,
        &config.server_name,
        PolicyStore::new(config.peers)?,
    )
    .await?;
    let mut channel = client.take_brain_media()?;
    let (tx, mut rx) = rtrb::RingBuffer::new(64);
    let epochs = BridgeEpochs {
        source: client.identity().source_epoch.0,
        destination: epoch.value,
        route: u64::MAX,
    };
    let mut renderer = Renderer {
        captured: tx,
        monitor: Bridge::prepare(BridgeConfig::voice(2), epochs).map_err(|e| e.to_string())?,
        epochs,
        fault: None,
        dropped: 0,
        hold_generation: 0,
    };
    let mut descriptor: Option<BrainMediaDescriptor> = None;
    let mut proposed: Option<BrainMediaDescriptor> = None;
    let mut snapshot: Option<Snapshot> = None;
    let mut arm_fence = ArmFence::default();
    let mut generation = 1u64;

    let mut monitor_arm_revision = None;
    let mut evidence = Evidence::default();
    let mut playback_hash = Sha256::new();
    let mut capture_hash = Sha256::new();
    let mut monitor_hash = Sha256::new();
    let mut last_window: Option<(u64, u64, u64)> = None;
    let mut fault = None;
    let started = Instant::now();
    let mut timer = tokio::time::interval(Duration::from_micros(250));
    timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Burst);
    let mut controls = tokio::time::interval(Duration::from_millis(25));
    controls.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut fake_frame = 0u64;
    let mut fake_origin = None;
    let mut fake_capture = vec![0.; config.device.period_frames * config.device.capture_channels];
    let mut fake_output = vec![0.; config.device.period_frames * host.config().playback_channels];
    client
        .report_brain_device(observation(&host, &renderer, None, None, None))
        .await?;
    client
        .send_command(query(&config.show_id, client.identity().source_epoch)?)
        .await?;
    let execution: Result<()> = async {
    loop {
        if started.elapsed() >= Duration::from_millis(config.duration_ms) {
            break;
        }
        tokio::select! {
            _=timer.tick()=>{
                let now=monotonic_ms();
                if host.is_armed() {
                    let period=host.config().period_frames;
                    let capture_width=host.config().capture_channels;
                    let microphone_slot=host.config().microphone.slot;
                    if let Device::Fake(fake)=host.device_mut(){
                        let origin=*fake_origin.get_or_insert_with(Instant::now);
                        let expected=(origin.elapsed().as_secs_f64()*48_000.*(1.+f64::from(config.fake_skew_ppm)*1e-6)) as u64;
                        if expected.saturating_sub(fake_frame)>4800 {fault=Some("fake duplex scheduling debt exceeded100ms".into());break;}
                        if fake_frame+period as u64<=expected{
                            for (n,row) in fake_capture.chunks_exact_mut(capture_width).enumerate(){row.fill(0.);row[microphone_slot]=if ((fake_frame+n as u64)/24).is_multiple_of(2){0.0625}else{-0.0625};}
                            if let Err(e)=fake.advance(&fake_capture,period){fault=Some(e.to_string());break;}
                            fake_frame+=period as u64;
                        }
                        let frames=fake.drain_playback(&mut fake_output);
                        let first=evidence.playback_frames;evidence.playback_frames+=frames as u64;
                        for sample in &fake_output[..frames*host.config().playback_channels]{playback_hash.update(sample.to_le_bytes());}
                        if frames>0{
                            let state=snapshot.as_ref().map(|s|(s.selection_generation.0,s.revision.0,first/4800)).unwrap_or((0,0,first/4800));
                            if last_window!=Some(state){
                                if evidence.playback_windows.len()<2048{evidence.playback_windows.push(json!({"elapsed_ms":started.elapsed().as_millis() as u64,"epoch":host.readback().epoch,"first_frame":first,"snapshot":snapshot,"bridge":renderer.monitor.status(),"samples":&fake_output[..frames*host.config().playback_channels]}));}else{evidence.playback_windows_overflow+=1;}last_window=Some(state);
                            }
                        }
                        for sample in &fake_output[..frames*host.config().playback_channels]{evidence.playback_energy+=sample*sample;evidence.playback_peak=evidence.playback_peak.max(sample.abs());}
                        if evidence.first_playback.is_empty()&&fake_output.iter().any(|v|*v!=0.){evidence.first_playback=fake_output.clone();}
                    }
                    if let Err(e)=host.service(now,&mut renderer){fault=Some(e.to_string());break;}
                }
                for _ in 0..32 {
                    let Ok(block)=rx.pop() else{break;};
                    if let Some(d)=&descriptor && d.talkback && d.hold_generation!=0 && block.epoch==d.brain_epoch && block.hold_generation==d.hold_generation {
                        channel.send(&BrainPacket{descriptor:d.clone(),role:BrainMediaRole::Talkback,frame:block.frame,samples:block.samples.to_vec()})?;
                        for sample in block.samples{capture_hash.update(sample.to_le_bytes());}evidence.captured_packets+=1;
                    }
                }
            }
            _=controls.tick()=>{
                client.send_command(query(&config.show_id,client.identity().source_epoch)?).await?;
                client.report_brain_device(observation(&host,&renderer,None,None,None)).await?;
            }
            packet=async { if let Some(d)=&descriptor { channel.receive(d).await } else { std::future::pending().await } }=>{
                match packet{
                    Ok(packet)=>{
                        for sample in &packet.samples{monitor_hash.update(sample.to_le_bytes());}
                        if snapshot.as_ref().is_some_and(|s|s.monitor_armed&&monitor_selection_current(Some(s.selection_generation.0),&packet.descriptor)) {if renderer.monitor.push(renderer.epochs,packet.frame,&packet.samples).is_ok(){evidence.monitor_packets+=1;}else{evidence.rejected_monitor_packets+=1;}}
                    }
                    _=>evidence.rejected_monitor_packets+=1,
                }
                if let Some(s)=&snapshot && s.monitor_armed && !renderer.monitor.status().armed && monitor_arm_revision!=Some(s.revision.0) && renderer.monitor.status().ready {
                    renderer.monitor.arm(renderer.epochs).map_err(|e|e.to_string())?;monitor_arm_revision=Some(s.revision.0);
                }
            }
            reply=client.receive()=>{
                let payload=match reply{Ok(Response::Reply{payload,..})=>payload,Ok(Response::Refused{reason,..})=>{fault=Some(reason);break;},Ok(_)=>continue,Err(e)=>{fault=Some(e);break;}};
                match payload.get("contract").and_then(Value::as_str){
                    Some("GP15-brain")=>{
                        let reply:crate::brain_control::Reply=serde_json::from_value(payload).map_err(|e|e.to_string())?;
                        if let Some(s)=reply.snapshot{
                            if !arm_fence.observe(s.revision.0,s.monitor_armed||s.held_generation.is_some()){continue;}
                            let hold=s.held_generation.map_or(0,|v|v.0);
                            let changed=descriptor.as_ref().is_none_or(|d|d.selection_generation!=s.selection_generation.0||d.hold_generation!=hold||d.brain_epoch!=host.readback().epoch);
                            renderer.observe_hold(hold,&mut rx);
                            if renderer.epochs.source!=client.identity().source_epoch.0||renderer.epochs.destination!=host.readback().epoch||renderer.epochs.route!=s.selection_generation.0 {
                                host.invalidate_monitor_output();renderer.monitor.invalidate();
                            }
                            if changed&&proposed.is_none(){
                                generation=generation.checked_add(1).ok_or("media generation exhausted")?;
                                let next=BrainMediaDescriptor{contract:"GP15-media".into(),version:1,session:client.session(),stagebox_epoch:client.identity().source_epoch.0,brain_epoch:host.readback().epoch,stagebox_map:client.identity().map_generation.0,brain_map:host.readback().configuration_generation,generation,selection_generation:s.selection_generation.0,hold_generation:hold,sample_rate:48_000,frames:48,talkback:true,monitor:true};
                                client.negotiate_brain_media(next.clone()).await?;proposed=Some(next);monitor_arm_revision=None;
                            }
                            host.set_levels(LocalLevels{microphone_gain_db:0.,microphone_muted:hold==0||s.talkback_mute,monitor_gain_db:f64::from(s.monitor_gain_cdb)/100.,monitor_muted:s.monitor_mute||!s.monitor_armed,monitor_dim:s.monitor_dim}).map_err(|e|e.to_string())?;
                            if !host.is_armed() && host.fault_reason().is_none() && (hold!=0||s.monitor_armed){host.arm(host.readback(),monotonic_ms()).map_err(|e|e.to_string())?;}
                            snapshot=Some(s);
                        }
                    }
                    Some("GP15-media") if payload["state"]=="accepted"=>{
                        let d:BrainMediaDescriptor=serde_json::from_value(payload["descriptor"].clone()).map_err(|e|e.to_string())?;
                        if proposed.as_ref()!=Some(&d){fault=Some("uncorrelated media acceptance".into());break;}
                        proposed=None;
                        if d.brain_epoch!=host.readback().epoch {descriptor=None;continue;}
                        if !monitor_selection_current(snapshot.as_ref().map(|s|s.selection_generation.0),&d){
                            renderer.hold_generation=accepted_capture_hold(snapshot.as_ref().map(|s|s.held_generation.map_or(0,|g|g.0)),&d);
                            descriptor=Some(d);continue;
                        }
                        let next_epochs=BridgeEpochs{source:d.stagebox_epoch,destination:d.brain_epoch,route:d.selection_generation};
                        if renderer.epochs!=next_epochs {renderer.monitor.reset(next_epochs).map_err(|e|e.to_string())?;renderer.epochs=next_epochs;}renderer.hold_generation=accepted_capture_hold(snapshot.as_ref().map(|s|s.held_generation.map_or(0,|g|g.0)),&d);descriptor=Some(d);evidence.negotiations+=1;
                    }
                    Some("GP15-device") if payload["state"]=="observed"=>{
                        if payload["brain_epoch"].as_u64()==Some(host.readback().epoch)&&payload["brain_map"].as_u64()==Some(host.readback().configuration_generation){arm_fence.acknowledged=true;}
                    }
                    Some("GP15-device") if payload["kind"]=="apply_configuration"=>{
                        let ticket=payload["ticket"].as_u64().ok_or("device ticket")?;
                        let next:DeviceConfig=serde_json::from_value(payload["config"].clone()).map_err(|e|e.to_string())?;
                        host.stop();renderer.monitor.invalidate();renderer.hold_generation=0;
                        arm_fence=ArmFence::default();snapshot=None;
                                descriptor=None;proposed=None;
                        let old_capabilities=host.status().capabilities;let retired=std::mem::replace(host.device_mut(),Device::Closed(old_capabilities));drop(retired);
                        let fresh=epoch.advance()?;
                        let replacement=device(&next,config.physical).and_then(|d|BrainHost::prepare(d,next.clone(),fresh,fresh).map_err(|e|e.to_string()));
                        match replacement{
                            Ok(next_host)=>{host=next_host;
                                crate::show::persist(config.epoch_file.parent().ok_or("intent parent")?,"brain-device-intent.json",host.config())?;fake_capture.resize(host.config().period_frames*host.config().capture_channels,0.);fake_output.resize(host.config().period_frames*host.config().playback_channels,0.);fake_frame=0;fake_origin=None;evidence.configuration_changes+=1;client.report_brain_device(observation(&host,&renderer,Some(ticket),Some(true),None)).await?;}
                            Err(e)=>{client.report_brain_device(observation(&host,&renderer,Some(ticket),Some(false),Some(e))).await?;}
                        }
                    }
                    _=>(),
                }
            }
        }
    }
        Ok(())
    }.await;
    if let Err(error) = execution {
        fault.get_or_insert(error);
    }
    host.stop();
    renderer.monitor.invalidate();
    let final_observation = observation(&host, &renderer, None, None, fault.clone());
    let _ = client.report_brain_device(final_observation.clone()).await;
    client.close();
    let report = json!({"mode":"brain_duplex","software_only":!config.physical,"physical_qualified":false,"fault":fault,"evidence":evidence,"captured_pre_wire_sha256":format!("{:x}",capture_hash.finalize()),"received_monitor_sha256":format!("{:x}",monitor_hash.finalize()),"submitted_playback_sha256":format!("{:x}",playback_hash.finalize()),"final_observation":final_observation,"elapsed_ms":started.elapsed().as_millis(),"fake_skew_ppm":config.fake_skew_ppm});
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&config.report)
        .map_err(|e| e.to_string())?;
    file.write_all(&serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn finite_run_limits_require_separate_physical_activation() {
        assert!(validate_run_bounds(60_000, 1000, false, false).is_ok());
        assert!(validate_run_bounds(60_001, 0, false, true).is_err());
        assert!(validate_run_bounds(86_400_000, 0, true, true).is_ok());
        assert!(validate_run_bounds(86_400_001, 0, true, true).is_err());
        assert!(validate_run_bounds(60_001, 0, true, false).is_err());
        assert!(validate_run_bounds(1, 0, true, false).is_err());
        assert!(validate_run_bounds(0, 0, false, false).is_err());
        assert!(validate_run_bounds(0, 0, true, true).is_err());
        assert!(validate_run_bounds(1, 2001, false, false).is_err());
    }

    #[test]
    fn durable_epochs_exclude_competing_owner_and_refuse_corrupt_recovery() {
        let directory = std::env::temp_dir().join(format!("gp15-epoch-{}", std::process::id()));
        std::fs::create_dir(&directory).unwrap();
        std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700)).unwrap();
        let path = directory.join("epoch.json");
        let mut first = Epoch::open(&path).unwrap();
        assert_eq!(first.value, 1);
        assert!(Epoch::open(&path).is_err());
        assert_eq!(first.advance().unwrap(), 2);
        drop(first);
        assert_eq!(Epoch::open(&path).unwrap().value, 3);
        std::fs::write(&path, []).unwrap();
        assert!(Epoch::open(&path).is_err());
        std::fs::write(&path, b"broken").unwrap();
        assert!(Epoch::open(&path).is_err());
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn replacement_requires_ack_closed_readback_and_new_arm_revision() {
        let mut fence = ArmFence::default();
        assert!(!fence.observe(10, true));
        assert!(!fence.observe(11, false));
        fence.acknowledged = true;
        assert!(!fence.observe(12, true));
        assert!(fence.observe(13, false));
        assert!(!fence.observe(12, true));
        assert!(!fence.observe(13, true));
        assert!(fence.observe(14, true));
        fence = ArmFence::default();
        assert!(!fence.observe(14, true));
    }

    #[test]
    fn captured_blocks_keep_the_hold_that_owned_their_samples() {
        let epochs = BridgeEpochs {
            source: 1,
            destination: 2,
            route: 3,
        };
        let (tx, mut rx) = rtrb::RingBuffer::new(4);
        let mut renderer = Renderer {
            captured: tx,
            monitor: Bridge::prepare(BridgeConfig::voice(2), epochs).unwrap(),
            epochs,
            fault: None,
            dropped: 0,
            hold_generation: 11,
        };
        renderer.capture(2, 0, &[0.25; 48]);
        renderer.hold_generation = 0;
        renderer.capture(2, 48, &[0.5; 48]);
        renderer.hold_generation = 12;
        renderer.capture(2, 96, &[0.75; 48]);
        let old = rx.pop().unwrap();
        let new = rx.pop().unwrap();
        assert_eq!(
            (old.hold_generation, old.frame, old.samples[0]),
            (11, 0, 0.25)
        );
        assert_eq!(
            (new.hold_generation, new.frame, new.samples[0]),
            (12, 96, 0.75)
        );
        assert!(rx.pop().is_err());
    }
    fn route_descriptor(selection: u64, hold: u64) -> BrainMediaDescriptor {
        BrainMediaDescriptor {
            contract: "GP15-media".into(),
            version: 1,
            session: 1,
            stagebox_epoch: 1,
            brain_epoch: 2,
            stagebox_map: 1,
            brain_map: 1,
            generation: 9,
            selection_generation: selection,
            hold_generation: hold,
            sample_rate: 48_000,
            frames: 48,
            talkback: true,
            monitor: true,
        }
    }
    #[test]
    fn pending_hold_proposal_cannot_restore_a_superseded_monitor_selection() {
        let pending = route_descriptor(3, 12);
        // The selection changed while this hold-only proposal was in flight.
        assert!(!monitor_selection_current(Some(4), &pending));
        assert_eq!(accepted_capture_hold(Some(12), &pending), 12);
        // A subsequent release also closes capture, regardless of stale ACK.
        assert_eq!(accepted_capture_hold(Some(0), &pending), 0);
        assert!(!monitor_selection_current(None, &pending));
        let current = route_descriptor(4, 12);
        assert!(monitor_selection_current(Some(4), &current));
    }
    #[test]
    fn monitor_selection_change_preserves_held_capture_queue_and_packet_frames() {
        let epochs = BridgeEpochs {
            source: 1,
            destination: 2,
            route: 3,
        };
        let (tx, mut rx) = rtrb::RingBuffer::new(4);
        let mut renderer = Renderer {
            captured: tx,
            monitor: Bridge::prepare(BridgeConfig::voice(2), epochs).unwrap(),
            epochs,
            fault: None,
            dropped: 0,
            hold_generation: 11,
        };
        renderer.capture(2, 0, &[0.25; 48]);
        // Same production hold barrier is used even while media negotiation is pending.
        renderer.observe_hold(11, &mut rx);
        renderer.monitor.invalidate();
        renderer.capture(2, 48, &[0.5; 48]);
        let old = route_descriptor(3, 11);
        let next = route_descriptor(4, 11);
        assert!(!monitor_selection_current(Some(4), &old));
        for (frame, sample) in [(0, 0.25), (48, 0.5)] {
            let block = rx.pop().unwrap();
            assert_eq!((block.frame, block.hold_generation), (frame, 11));
            let bytes = BrainPacket {
                descriptor: old.clone(),
                role: BrainMediaRole::Talkback,
                frame: block.frame,
                samples: block.samples.to_vec(),
            }
            .encode()
            .unwrap();
            let decoded = BrainPacket::decode(&bytes, &next, BrainMediaRole::Talkback).unwrap();
            assert_eq!(decoded.samples, vec![sample; 48]);
        }
        renderer.capture(2, 96, &[0.75; 48]);
        renderer.observe_hold(0, &mut rx);
        assert_eq!(renderer.hold_generation, 0);
        assert!(rx.pop().is_err());
    }
}
