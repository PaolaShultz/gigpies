use super::diagnostics::{self, Handle, Stage, Token};
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
    read_frame_diagnostic(stream, &None, Token::default()).await
}
pub(crate) async fn read_frame_diagnostic<T: DeserializeOwned>(
    stream: &mut quinn::RecvStream,
    trace: &Handle,
    token: Token,
) -> Result<T> {
    let mut length = [0; 4];
    stream
        .read_exact(&mut length[..1])
        .await
        .map_err(|e| e.to_string())?;
    diagnostics::record(trace, token, Stage::FirstByte, 0, 0);
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
        diagnostics::record(trace, token, Stage::Framed, bytes.len() as u64, 0);
        let result = decode(&bytes);
        diagnostics::record(trace, token, Stage::Decoded, u64::from(result.is_err()), 0);
        result
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
    write_response_diagnostic(stream, response, &None, Token::default()).await
}
pub(crate) async fn write_response_diagnostic(
    stream: &mut quinn::SendStream,
    response: &Response,
    trace: &Handle,
    token: Token,
) -> Result<()> {
    let pages = prepare_response(response, trace, token)?;
    let operation = async {
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
    };
    let budget = if matches!(response, Response::Reply {payload,..} if payload.get("contract").and_then(Value::as_str)==Some(crate::meter_wire::CONTRACT))
    {
        100
    } else {
        IO_TIMEOUT_MS
    };
    let result = tokio::time::timeout(
        Duration::from_millis(budget),
        observe_write(operation, trace, token),
    )
    .await
    .map_err(|_| "remote snapshot write timeout".to_string());
    if result.is_err() {
        diagnostics::record(trace, token, Stage::WriteEnd, 2, 0);
    }
    result?
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

fn prepare_response(response: &Response, trace: &Handle, token: Token) -> Result<Vec<Vec<u8>>> {
    diagnostics::record(trace, token, Stage::SerializeBegin, 0, 0);
    let bytes = serde_json::to_vec(response).map_err(|e| e.to_string())?;
    diagnostics::record(trace, token, Stage::SerializeEnd, bytes.len() as u64, 0);
    let pages = crate::snapshot_pages::encode(bytes)?;
    if trace.is_some() {
        diagnostics::record(
            trace,
            token,
            Stage::PagesEnd,
            pages.len() as u64,
            pages.iter().map(|p| p.len() as u64 + 4).sum(),
        );
    }
    Ok(pages)
}

async fn observe_write<F: std::future::Future<Output = Result<()>>>(
    future: F,
    trace: &Handle,
    token: Token,
) -> Result<()> {
    if trace.is_none() {
        return future.await;
    }
    tokio::pin!(future);
    let mut polls = 0u64;
    let mut first = true;
    std::future::poll_fn(|cx| {
        if first {
            diagnostics::record(trace, token, Stage::WritePoll, 0, 0);
            first = false;
        }
        match future.as_mut().poll(cx) {
            std::task::Poll::Pending => {
                if polls == 0 {
                    diagnostics::record(trace, token, Stage::WritePending, 0, 0);
                }
                polls += 1;
                std::task::Poll::Pending
            }
            std::task::Poll::Ready(result) => {
                diagnostics::record(
                    trace,
                    token,
                    Stage::WriteEnd,
                    u64::from(result.is_err()),
                    polls,
                );
                std::task::Poll::Ready(result)
            }
        }
    })
    .await
}

#[cfg(test)]
mod diagnostic_tests {
    use super::*;
    #[test]
    fn disabled_and_enabled_response_wire_bytes_are_identical() {
        let response = Response::Refused {
            session: Counter(1),
            reason: "test".into(),
        };
        let plain = prepare_response(&response, &None, Token::default()).unwrap();
        let traced = prepare_response(
            &response,
            &Some(super::super::diagnostics::Trace::test_trace()),
            Token::default(),
        )
        .unwrap();
        assert_eq!(plain, traced);
        assert_eq!(
            plain,
            vec![br#"{"kind":"refused","session":"1","reason":"test"}"#.to_vec()]
        );
    }

    #[tokio::test]
    async fn write_observer_preserves_success_error_and_pending() {
        let storage = super::super::diagnostics::Trace::test_trace();
        let trace = Some(storage.clone());
        let token = Token {
            session: 1,
            ordinal: 2,
            ..Token::default()
        };
        let mut first = true;
        let operation = std::future::poll_fn(move |cx| {
            if first {
                first = false;
                cx.waker().wake_by_ref();
                std::task::Poll::Pending
            } else {
                std::task::Poll::Ready(Ok(()))
            }
        });
        assert!(observe_write(operation, &trace, token).await.is_ok());
        assert_eq!(
            observe_write(async { Err("original".into()) }, &trace, token)
                .await
                .unwrap_err(),
            "original"
        );
        assert_eq!(
            observe_write(async { Err("original".into()) }, &None, token)
                .await
                .unwrap_err(),
            "original"
        );
        let rows = storage.report().records;
        assert!(rows.iter().any(|r| r[0] == Stage::WritePending as u64));
        assert!(
            rows.iter()
                .any(|r| r[0] == Stage::WriteEnd as u64 && r[6] == 0 && r[7] == 1)
        );
        assert!(
            rows.iter()
                .any(|r| r[0] == Stage::WriteEnd as u64 && r[6] == 1)
        );
    }
}
