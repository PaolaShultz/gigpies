//! Explicit, bounded device-free executables for authenticated functional runs.
//! Configuration, credentials and reports remain task-private. This is not a
//! service installer, device host, load benchmark or hardware qualification.
use super::*;
use crate::{
    control_model::{Command, Request as AudioRequest},
    local_audio::LocalAudio,
    show::Counter,
    topology::EngineTopology,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    io::Write,
    net::SocketAddr,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

#[derive(Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum RunConfig {
    Provider {
        bind: SocketAddr,
        certificate: PathBuf,
        key: PathBuf,
        ca: PathBuf,
        peers: Vec<Peer>,
        private_directory: PathBuf,
        manifest: PathBuf,
        pa_configuration: Option<PathBuf>,
        record_take: Option<String>,
        show_id: String,
        source_epoch: u64,
        inputs: usize,
        monitors: usize,
        pa_outputs: usize,
        reference_16_18: bool,
        duration_ms: u64,
        period_ms: u64,
        report: PathBuf,
        ready: Option<PathBuf>,
    },
    Brain {
        bind: SocketAddr,
        peer: SocketAddr,
        server_name: String,
        certificate: PathBuf,
        key: PathBuf,
        ca: PathBuf,
        peers: Vec<Peer>,
        show_id: String,
        fx_library: PathBuf,
        duration_ms: u64,
        return_delay_frames: u32,
        report: PathBuf,
    },
}
#[derive(Serialize)]
struct RevisionWindow {
    revision: Counter,
    first_source_frame: u64,
    last_source_frame: u64,
    blocks: u64,
    energy_per_playback_slot: Vec<f64>,
    first_block_samples: Vec<f64>,
}
fn save(path: &Path, value: &Value) -> Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    file.write_all(&serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())
}
pub async fn run_config(config: RunConfig) -> Result<Value> {
    match config {
        RunConfig::Provider {
            bind,
            certificate,
            key,
            ca,
            peers,
            private_directory,
            manifest,
            pa_configuration,
            record_take,
            show_id,
            source_epoch,
            inputs,
            monitors,
            pa_outputs,
            reference_16_18,
            duration_ms,
            period_ms,
            report,
            ready,
        } => {
            if !(1..=60_000).contains(&duration_ms) || !(1..=100).contains(&period_ms) {
                return Err("bounded synthetic duration/period required".into());
            }
            let credentials = Credentials::load_der(&certificate, &key, &ca)?;
            let policy = PolicyStore::new(peers)?;
            let server = RemoteServer::bind(bind, &credentials, policy)?;
            let address = server.local_addr()?;
            let topology = if reference_16_18 {
                if inputs != 16 {
                    return Err("reference analog profile requires16 inputs".into());
                }
                EngineTopology::reference_16_18(pa_outputs, monitors)?
            } else {
                EngineTopology::software(inputs, monitors, pa_outputs)?
            };
            let mut provider = LocalAudio::bind_configured(
                &private_directory,
                "remote-provider.sock",
                &show_id,
                Counter(source_epoch),
                topology,
            )?;
            provider.enable_modules(&manifest)?;
            if let Some(path) = pa_configuration {
                let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
                if bytes.len() > 48 * 1024 {
                    return Err("PA configuration file capacity".into());
                }
                provider.configure_pa(&bytes)?;
            }
            if let Some(take) = &record_take {
                start_recording(&mut provider, &show_id, take, monotonic_ms())?;
            }
            let mut host = HostAuthority::new(provider, 1)?;
            let capture_channels = host.provider().topology().capture_channels;
            let playback_channels = host.provider().topology().playback_channels;
            let (accepted_tx, mut accepted_rx) = tokio::sync::mpsc::channel(4);
            let acceptor = tokio::spawn(async move {
                loop {
                    match server.accept().await {
                        Ok(session) => {
                            if accepted_tx.send(session).await.is_err() {
                                break;
                            }
                        }
                        Err(_) => tokio::task::yield_now().await,
                    }
                }
            });
            if let Some(path) = ready {
                save(
                    &path,
                    &json!({"state":"ready","bind":address.to_string(),"source_epoch":source_epoch,"inputs":inputs,"capture_channels":capture_channels,"playback_channels":playback_channels,"outputs_initially_muted":true}),
                )?;
            }
            let mut mailboxes = Vec::new();
            let mut networks = Vec::new();
            let mut timer = tokio::time::interval(Duration::from_millis(period_ms));
            timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            let deadline = tokio::time::sleep(Duration::from_millis(duration_ms));
            tokio::pin!(deadline);
            let started = Instant::now();
            let mut capture = vec![0.; 48 * capture_channels];
            let mut playback = vec![0.; 48 * playback_channels];
            let mut peaks = vec![0f64; playback_channels];
            let mut sums = vec![0f64; playback_channels];
            let mut revision_windows: Vec<RevisionWindow> = Vec::new();
            let mut revision_windows_overflow = 0u64;
            let mut blocks = 0u64;
            let mut sessions = 0u64;
            let mut control_faults = 0u64;
            let mut fault = None;
            loop {
                tokio::select! {
                    _=&mut deadline=>break,
                    accepted=accepted_rx.recv()=>{
                        let Some(session)=accepted else{fault=Some("accept worker stopped".to_string());break;};
                        let (mut proxy,mailbox)=authority_channel(session.context(),host.identity())?;
                        mailboxes.push(mailbox);sessions+=1;
                        networks.retain(|task: &tokio::task::JoinHandle<_>| !task.is_finished());
                        networks.push(tokio::spawn(async move{session.run(&mut proxy).await}));
                    }
                    _=timer.tick()=>{
                        let now=monotonic_ms();
                        for mailbox in &mut mailboxes {if mailbox.service(&mut host,now).is_err(){control_faults+=1;}}
                        mailboxes.retain(|m|!m.retired());
                        let frame=host.provider().frame();
                        capture.fill(0.);
                        for f in 0..48 {for (index,input) in host.provider().topology().inputs.iter().enumerate(){
                            // Distinct exact PCM24 source values and periods; no
                            // external media, acoustic stimulus or file output.
                            let polarity=if ((frame+f as u64)/(24+index as u64)).is_multiple_of(2) {1.}else{-1.};
                            capture[f*capture_channels+input.capture_slot]=polarity*((index+1)*128) as f64/8_388_608.;
                        }}
                        if let Err(error)=host.process_source(now,source_epoch,frame,&capture,&mut playback){fault=Some(error);break;}
                        for row in playback.chunks_exact(playback_channels){for(channel,sample)in row.iter().enumerate(){peaks[channel]=peaks[channel].max(sample.abs());sums[channel]+=sample*sample;}}
                        let revision=host.provider_mut().engine_mut().revision();
                        if revision_windows.last().is_none_or(|w|w.revision!=revision) {
                            if revision_windows.len()<64 {
                                revision_windows.push(RevisionWindow{revision,first_source_frame:frame,last_source_frame:frame+47,blocks:0,energy_per_playback_slot:vec![0.;playback_channels],first_block_samples:playback.clone()});
                            } else {revision_windows_overflow+=1;}
                        }
                        if let Some(window)=revision_windows.last_mut().filter(|w|w.revision==revision) {
                            window.last_source_frame=frame+47;window.blocks+=1;
                            for row in playback.chunks_exact(playback_channels){for(channel,sample)in row.iter().enumerate(){window.energy_per_playback_slot[channel]+=sample*sample;}}
                        }
                        blocks+=1;
                    }
                }
            }
            acceptor.abort();
            let _ = acceptor.await;
            for network in &networks {
                network.abort();
            }
            for network in networks {
                let _ = network.await;
            }
            for mailbox in &mut mailboxes {
                let _ = mailbox.service(&mut host, monotonic_ms());
            }
            if record_take.is_some() {
                host.provider_mut().quiesce_source("bounded_source_end")?;
                for _ in 0..200 {
                    let status = host.provider_mut().module_status(monotonic_ms())?;
                    if !matches!(
                        status["recording"]["state"].as_str(),
                        Some("starting" | "recording" | "finalizing")
                    ) {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(5)).await;
                }
            }
            let snapshot = host.provider_mut().snapshot()?;
            let structure = host.provider_mut().structural_snapshot()?;
            let modules = host.provider_mut().module_status(monotonic_ms())?;
            let evidence = json!({"mode":"provider","software_only":true,"real_time_qualified":false,"inputs":inputs,"capture_channels":capture_channels,"playback_channels":playback_channels,"source_epoch":source_epoch,"processed_frames":blocks*48,"blocks":blocks,"elapsed_ms":started.elapsed().as_millis(),"synthetic_period_ms":period_ms,"sessions":sessions,"control_faults":control_faults,"revision_windows":revision_windows,"revision_windows_overflow":revision_windows_overflow,"media":host.media_stats(),"accepted_wet_arrival_sha256":host.accepted_wet_sha256(),"fault":fault,"peak_per_playback_slot":peaks,"energy_per_playback_slot":sums,"final_snapshot":snapshot,"final_structure":structure,"modules":modules,"record_take":record_take,"recording_root":private_directory.join("takes")});
            save(&report, &evidence)?;
            Ok(evidence)
        }
        RunConfig::Brain {
            bind,
            peer,
            server_name,
            certificate,
            key,
            ca,
            peers,
            show_id,
            fx_library,
            duration_ms,
            return_delay_frames,
            report,
        } => {
            if !(1..=60_000).contains(&duration_ms)
                || !(48..=1536).contains(&return_delay_frames)
                || !return_delay_frames.is_multiple_of(48)
            {
                return Err("bounded Brain duration/deadline required".into());
            }
            let credentials = Credentials::load_der(&certificate, &key, &ca)?;
            let endpoint = client_endpoint(bind, &credentials)?;
            let policy = PolicyStore::new(peers)?;
            let mut client = RemoteClient::connect(endpoint, peer, &server_name, policy).await?;
            let request = AudioRequest {
                contract: "C-AUDIO".into(),
                version: 2,
                show_id,
                module: "audio".into(),
                epoch: client.identity().source_epoch,
                writer: None,
                lease: None,
                request_id: None,
                expected_revision: None,
                command: Command::Snapshot {},
            };
            client
                .send_command(serde_json::to_value(request).map_err(|e| e.to_string())?)
                .await?;
            let Response::Reply { payload, .. } = client.receive().await? else {
                return Err("Brain requires actual engine snapshot".into());
            };
            let ids: Vec<String> =
                serde_json::from_value(payload["snapshot"]["authority"]["inputs"].clone())
                    .map_err(|e| e.to_string())?;
            let max = client.max_datagram();
            let mut streams = grouped_streams(MediaRole::Analysis, &ids, 48, 0, 1, max)?;
            let first = streams.len() as u32 + 1;
            let fx_ids = vec!["foh-left".into(), "foh-right".into()];
            streams.extend(grouped_streams(
                MediaRole::FxSend,
                &fx_ids,
                48,
                0,
                first,
                max,
            )?);
            streams.extend(grouped_streams(
                MediaRole::WetReturn,
                &fx_ids,
                48,
                return_delay_frames,
                first + 1,
                max,
            )?);
            let identity = client.identity().clone();
            let descriptor = MediaDescriptor {
                session: Counter(client.session()),
                source_epoch: identity.source_epoch,
                capability_generation: identity.capability_generation,
                map_generation: identity.map_generation,
                sample_rate: 48000,
                streams,
            };
            client.negotiate(descriptor.clone()).await?;
            if !matches!(client.receive().await?, Response::MediaAccepted { .. }) {
                return Err("Brain media negotiation refused".into());
            }
            let mut worker = BrainWorker::prepare(client.media_channel()?, &fx_library)?;
            let started = Instant::now();
            let deadline = tokio::time::sleep(Duration::from_millis(duration_ms));
            tokio::pin!(deadline);
            let mut first_source = None;
            let mut last_source = None;
            let mut analysis_hash = Sha256::new();
            let mut mismatches = 0u64;
            let mut channel_samples: std::collections::BTreeMap<String, u64> =
                ids.iter().cloned().map(|id| (id, 0)).collect();
            let mut fault = None;
            loop {
                tokio::select! {
                    _=&mut deadline=>break,
                    result=worker.step()=>match result {
                        Ok(Some(bytes))=>{
                            let packet=crate::transport::Packet::parse(&bytes).map_err(|e|format!("analysis evidence {e:?}"))?;
                            first_source.get_or_insert(packet.source_frame());last_source=Some(packet.source_frame());
                            let stream=descriptor.streams.iter().find(|s|s.stream==packet.spec().stream).ok_or("analysis evidence stream")?;
                            for (channel,id) in stream.channel_ids.iter().enumerate() {
                                let index:usize=id.strip_prefix("input-").ok_or("analysis source ID")?.parse::<usize>().map_err(|e|e.to_string())?.checked_sub(1).ok_or("analysis source index")?;
                                for f in 0..usize::from(packet.spec().frames) {
                                    let polarity=if ((packet.source_frame()+f as u64)/(24+index as u64)).is_multiple_of(2) {1}else{-1};
                                    let expected=polarity*((index+1)*128) as i32;
                                    if packet.pcm(f*usize::from(packet.spec().channels)+channel).map_err(|e|format!("analysis PCM {e:?}"))?!=expected {mismatches+=1;}
                                    *channel_samples.get_mut(id).ok_or("analysis channel evidence")?+=1;
                                }
                            }
                            analysis_hash.update(&bytes);
                        },
                        Ok(None)=>(),Err(error)=>{fault=Some(error);break;}
                    }
                }
            }
            client.close();
            if mismatches != 0 {
                fault = Some("raw analysis sample mismatch".into());
            }
            if channel_samples.values().any(|count| *count == 0) {
                fault = Some("not every negotiated analysis channel was observed".into());
            }
            if worker.stats.processed_blocks == 0 {
                fault = Some("no actual owner FX blocks observed".into());
            }
            if worker.nonzero_return_samples() == 0 {
                fault = Some("functional acceptance observed no nonzero owner wet result; source may remain muted".into());
            }
            let evidence = json!({"mode":"brain","software_only":true,"descriptor":descriptor,"elapsed_ms":started.elapsed().as_millis(),"first_analysis_source_frame":first_source,"last_analysis_source_frame":last_source,"analysis_packets":worker.stats.analysis_packets,"analysis_sample_mismatches":mismatches,"analysis_samples_per_channel":channel_samples,"analysis_arrival_sha256":format!("{:x}",analysis_hash.finalize()),"returned_wet_sha256":worker.returned_sha256(),"nonzero_returned_wet_samples":worker.nonzero_return_samples(),"fx_send_packets":worker.stats.send_packets,"actual_owner_fx_blocks":worker.stats.processed_blocks,"wet_packets":worker.stats.returned_packets,"incomplete_blocks":worker.stats.incomplete_blocks,"stale_groups":worker.stats.stale_groups,"fx_owner_resets":worker.owner_resets(),"fx_intentional_delay_frames":worker.intentional_delay_frames(),"fault":fault});
            save(&report, &evidence)?;
            Ok(evidence)
        }
    }
}

/// Explicit synthetic-run recorder request through the same live authority and
/// owner lifecycle. Permission ends immediately; the take continues independently.
pub(crate) fn start_recording(
    provider: &mut LocalAudio,
    show_id: &str,
    take: &str,
    now: u64,
) -> Result<()> {
    let epoch = Counter(provider.source_epoch());
    let version = provider.engine_mut().wire_version();
    let revision = provider.engine_mut().revision();
    let mut grant = AudioRequest {
        contract: "C-AUDIO".into(),
        version,
        show_id: show_id.into(),
        module: "audio".into(),
        epoch,
        writer: Some("bounded-recorder".into()),
        lease: None,
        request_id: Some(Counter(1)),
        expected_revision: Some(revision),
        command: Command::Grant {
            scope: crate::control_model::Scope::Foh,
        },
    };
    let reply = provider.engine_mut().handle(&grant, now)?;
    let lease = reply
        .outcome
        .and_then(|r| r.body.granted_lease)
        .ok_or("recorder grant refused")?;
    let request = crate::module_wire::ModuleRequest {
        contract: "GP05-modules".into(),
        version,
        show_id: show_id.into(),
        module: "audio".into(),
        epoch,
        writer: grant.writer.clone(),
        lease: Some(lease),
        request_id: Some(Counter(2)),
        expected_revision: Some(revision),
        command: crate::module_wire::ModuleCommand::RecordStart {
            take_id: take.into(),
            operation_id: Counter(1),
        },
    };
    let accepted = provider.dispatch_module(request, now, None)?;
    if accepted["state"].as_str() == Some("rejected") {
        return Err(format!("recorder start refused: {}", accepted["reason"]));
    }
    grant.lease = Some(lease);
    grant.request_id = Some(Counter(3));
    grant.command = Command::Release {};
    let release = provider.engine_mut().handle(&grant, now)?;
    if release.outcome.is_none_or(|r| r.kind != "applied") {
        return Err("recorder release refused".into());
    }
    Ok(())
}
