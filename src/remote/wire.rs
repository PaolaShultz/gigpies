use super::{IO_TIMEOUT_MS, MAX_FRAME, MediaDescriptor, Permission, Result};
use crate::show::Counter;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::collections::BTreeSet;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineIdentity {
    pub source_epoch: Counter,
    pub capability_generation: Counter,
    pub map_generation: Counter,
}
impl EngineIdentity {
    pub fn validate(&self) -> Result<()> {
        if self.source_epoch.0 == 0
            || self.capability_generation.0 == 0
            || self.map_generation.0 == 0
        {
            return Err("zero engine identity".into());
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Request {
    Open {
        contract: String,
        version: u32,
    },
    Command {
        session: Counter,
        capability_generation: Counter,
        payload: Value,
    },
    Negotiate {
        session: Counter,
        capability_generation: Counter,
        descriptor: MediaDescriptor,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Response {
    Hello {
        contract: String,
        version: u32,
        session: Counter,
        writer: String,
        peer_id: String,
        policy_generation: Counter,
        identity: EngineIdentity,
        permissions: BTreeSet<Permission>,
        max_datagram: u32,
    },
    Reply {
        session: Counter,
        payload: Value,
    },
    MediaAccepted {
        session: Counter,
        descriptor: MediaDescriptor,
    },
    Refused {
        session: Counter,
        reason: String,
    },
}

pub fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    let bytes = serde_json::to_vec(value).map_err(|e| e.to_string())?;
    if bytes.is_empty() || bytes.len() > MAX_FRAME {
        return Err("remote frame capacity".into());
    }
    Ok(bytes)
}
pub fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    crate::show::decode(bytes)
}
/// One complete bounded frame, including a deadline for partial/trickled data.
pub async fn read_frame<T: DeserializeOwned>(stream: &mut quinn::RecvStream) -> Result<T> {
    let mut length = [0; 4];
    stream
        .read_exact(&mut length[..1])
        .await
        .map_err(|e| e.to_string())?;
    tokio::time::timeout(Duration::from_millis(IO_TIMEOUT_MS), async {
        stream
            .read_exact(&mut length[1..])
            .await
            .map_err(|e| e.to_string())?;
        let length = u32::from_be_bytes(length) as usize;
        if length == 0 || length > MAX_FRAME {
            return Err("remote frame capacity".into());
        }
        let mut bytes = vec![0; length];
        stream
            .read_exact(&mut bytes)
            .await
            .map_err(|e| e.to_string())?;
        decode(&bytes)
    })
    .await
    .map_err(|_| "remote frame timeout".to_string())?
}
pub async fn write_frame<T: Serialize>(stream: &mut quinn::SendStream, value: &T) -> Result<()> {
    let bytes = encode(value)?;
    tokio::time::timeout(Duration::from_millis(IO_TIMEOUT_MS), async {
        stream
            .write_all(&(bytes.len() as u32).to_be_bytes())
            .await
            .map_err(|e| e.to_string())?;
        stream.write_all(&bytes).await.map_err(|e| e.to_string())
    })
    .await
    .map_err(|_| "remote write timeout".to_string())?
}

/// The same GP14 immutable snapshot pages as the Unix provider, with one MiB
/// admission and a whole-assembly deadline. Commands remain single-frame only.
pub async fn write_response(stream: &mut quinn::SendStream, response: &Response) -> Result<()> {
    let bytes = serde_json::to_vec(response).map_err(|e| e.to_string())?;
    let pages = crate::snapshot_pages::encode(bytes)?;
    tokio::time::timeout(Duration::from_millis(IO_TIMEOUT_MS), async {
        for page in pages {
            if page.is_empty() || page.len() > MAX_FRAME {
                return Err("remote page capacity".into());
            }
            stream
                .write_all(&(page.len() as u32).to_be_bytes())
                .await
                .map_err(|e| e.to_string())?;
            stream.write_all(&page).await.map_err(|e| e.to_string())?;
        }
        Ok(())
    })
    .await
    .map_err(|_| "remote snapshot write timeout".to_string())?
}
pub async fn read_response(stream: &mut quinn::RecvStream) -> Result<Response> {
    let first: Value = read_frame(stream).await?;
    if first.get("contract").and_then(Value::as_str) != Some("GP14-snapshot-pages") {
        return serde_json::from_value(first).map_err(|e| e.to_string());
    }
    tokio::time::timeout(Duration::from_millis(IO_TIMEOUT_MS), async {
        let started = std::time::Instant::now();
        let mut assembly = crate::snapshot_pages::Assembly::default();
        let mut value = first;
        loop {
            let page = serde_json::from_value(value).map_err(|e| e.to_string())?;
            if let Some(bytes) = assembly.offer(page, started.elapsed().as_millis() as u64)? {
                return crate::show::decode_bounded(&bytes, crate::snapshot_pages::ASSEMBLY_BYTES);
            }
            value = read_frame(stream).await?;
        }
    })
    .await
    .map_err(|_| "remote snapshot assembly timeout".to_string())?
}
