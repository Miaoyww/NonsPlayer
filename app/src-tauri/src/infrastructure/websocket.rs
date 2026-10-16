//! Authorized, bounded WebSocket transport. Application protocols stay in guests.
use super::{
    http::{checked_url, public_addresses, public_ip},
    wasm_runtime::host_io::{Event, Message},
};
use crate::model::AppResult;
use futures_util::{SinkExt, StreamExt};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::sync::{mpsc, Mutex};
use tokio_tungstenite::tungstenite::{
    client::IntoClientRequest, protocol::WebSocketConfig, Message as Frame,
};

enum Command {
    Send(Vec<u8>),
    Close,
}
struct Connection {
    commands: mpsc::Sender<Command>,
    task: Option<tokio::task::JoinHandle<()>>,
}
impl Drop for Connection {
    fn drop(&mut self) {
        if let Some(task) = &self.task {
            task.abort();
        }
    }
}
pub(crate) struct Sockets {
    connections: Mutex<HashMap<u32, Connection>>,
    next: AtomicU32,
    failed: Arc<AtomicU32>,
    events: mpsc::Sender<Event>,
    receiver: Mutex<mpsc::Receiver<Event>>,
    active: Arc<AtomicBool>,
}
impl Sockets {
    pub fn new(active: Arc<AtomicBool>) -> Self {
        let (events, receiver) = mpsc::channel(32);
        Self {
            connections: Mutex::new(HashMap::new()),
            next: AtomicU32::new(1),
            failed: Arc::new(AtomicU32::new(0)),
            events,
            receiver: Mutex::new(receiver),
            active,
        }
    }
    pub async fn open(
        &self,
        hosts: &[String],
        input: String,
        protocol: String,
        timeout_ms: u32,
    ) -> AppResult<u32> {
        if !self.active.load(Ordering::Acquire)
            || !(1..=15000).contains(&timeout_ms)
            || protocol.len() > 128
        {
            return Err("WebSocket 参数或上下文无效".into());
        }
        let mut url = url::Url::parse(&input)
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("translate_error", error);
            })
            .map_err(|_| "WebSocket 地址无效")?;
        if url.scheme() != "wss" {
            return Err("WebSocket 仅支持 WSS".into());
        }
        url.set_scheme("https")
            .inspect_err(|_| {
                super::diagnostics::warn("websocket.set_scheme", "invalid scheme transition");
            })
            .map_err(|_| "WebSocket 地址无效")?;
        let url = checked_url(hosts, url.as_str())?;
        let mut connections = self.connections.lock().await;
        if connections.len() >= 2 {
            return Err("WebSocket 连接数超过额度".into());
        }
        let id = self
            .next
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |id| id.checked_add(1))
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("translate_error", error);
            })
            .map_err(|_| "WebSocket 句柄耗尽")?;
        let socket = tokio::time::timeout(Duration::from_millis(timeout_ms as u64), async {
            let addresses =
                public_addresses(url.host_str().ok_or("WebSocket 目标无效")?, 443).await?;
            let tcp = tokio::net::TcpStream::connect(addresses.as_slice())
                .await
                .inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn("translate_error", error);
                })
                .map_err(|_| "WebSocket 连接失败")?;
            if !public_ip(
                tcp.peer_addr()
                    .inspect_err(|error| {
                        crate::infrastructure::diagnostics::warn("translate_error", error);
                    })
                    .map_err(|_| "WebSocket 目标不可验证")?
                    .ip(),
            ) {
                return Err("WebSocket 连接目标不是公网".into());
            }
            let mut request = input
                .into_client_request()
                .inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn("translate_error", error);
                })
                .map_err(|_| "WebSocket 握手参数无效")?;
            if !protocol.is_empty() {
                request.headers_mut().insert(
                    "sec-websocket-protocol",
                    protocol
                        .parse()
                        .inspect_err(|error| {
                            crate::infrastructure::diagnostics::warn("translate_error", error);
                        })
                        .map_err(|_| "WebSocket 子协议无效")?,
                );
            }
            let config = WebSocketConfig::default()
                .max_message_size(Some(16384))
                .max_frame_size(Some(16384))
                .max_write_buffer_size(32768)
                .write_buffer_size(0);
            let (socket, response) =
                tokio_tungstenite::client_async_tls_with_config(request, tcp, Some(config), None)
                    .await
                    .inspect_err(|error| {
                        crate::infrastructure::diagnostics::warn("translate_error", error);
                    })
                    .map_err(|_| "WebSocket TLS 或握手失败")?;
            if !protocol.is_empty()
                && response
                    .headers()
                    .get("sec-websocket-protocol")
                    .and_then(|v| {
                        v.to_str()
                            .inspect_err(|error| {
                                crate::infrastructure::diagnostics::warn("optional_result", error);
                            })
                            .ok()
                    })
                    != Some(protocol.as_str())
            {
                return Err("WebSocket 子协议未获确认".into());
            }
            Ok::<_, String>(socket)
        })
        .await
        .inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("translate_error", error);
        })
        .map_err(|_| "WebSocket 连接超时")??;
        let (commands, mut command_rx) = mpsc::channel(16);
        let events = self.events.clone();
        let active = self.active.clone();
        let failed = self.failed.clone();
        let task = tokio::spawn(async move {
            let (mut writer, mut reader) = socket.split();
            loop {
                tokio::select! {
                    command = command_rx.recv() => match command {
                        Some(Command::Send(bytes)) => if let Err(error) = writer.send(Frame::Binary(bytes.into())).await { super::diagnostics::warn("websocket.send", error); break; },
                        _ => { let _ = (writer.send(Frame::Close(None)).await).inspect_err(|error| { crate::infrastructure::diagnostics::warn("writer.send", error); }); break; }
                    },
                    frame = reader.next() => match frame {
                        Some(Ok(Frame::Binary(bytes))) => if let Err(error) = events.try_send(Event::Binary(Message { socket: id, data: bytes.to_vec() })) { super::diagnostics::warn("websocket.publish_binary", error); break; },
                        Some(Ok(Frame::Ping(bytes))) => if let Err(error) = writer.send(Frame::Pong(bytes)).await { super::diagnostics::warn("websocket.pong", error); break; },
                        Some(Ok(Frame::Pong(_))) => (),
                        Some(Err(error)) => { super::diagnostics::warn("websocket.receive", error); break; },
                        _ => break,
                    },
                    _ = tokio::time::sleep(Duration::from_millis(20)) => if !active.load(Ordering::Acquire) { break; },
                }
            }
            failed.store(id, Ordering::Release);
            let _ = (events.try_send(Event::Closed(id))).inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("events.try_send", error);
            });
        });
        connections.insert(
            id,
            Connection {
                commands,
                task: Some(task),
            },
        );
        Ok(id)
    }
    pub async fn send(&self, socket: u32, bytes: Vec<u8>) -> AppResult<()> {
        if !self.active.load(Ordering::Acquire) || bytes.len() > 16384 {
            return Err("WebSocket 帧或上下文超过额度".into());
        }
        let connections = self.connections.lock().await;
        let connection = connections.get(&socket).ok_or("WebSocket 句柄无效")?;
        connection
            .commands
            .try_send(Command::Send(bytes))
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("translate_error", error);
            })
            .map_err(|_| "WebSocket 已关闭或发送队列已满".into())
    }
    pub async fn close(&self, socket: u32) -> AppResult<()> {
        if let Some(mut connection) = self.connections.lock().await.remove(&socket) {
            let _ = (connection.commands.try_send(Command::Close)).inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("connection.commands.try_send", error);
            });
            if let Some(mut task) = connection.task.take() {
                match tokio::time::timeout(Duration::from_secs(1), &mut task).await {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) => super::diagnostics::warn("websocket.join", error),
                    Err(error) => {
                        super::diagnostics::warn("websocket.close_timeout", error);
                        task.abort();
                    }
                }
            }
        }
        Ok(())
    }
    pub async fn next_event(&self, timeout_ms: u32) -> AppResult<Option<Event>> {
        if !self.active.load(Ordering::Acquire) || timeout_ms > 1000 {
            return Err("WebSocket 事件等待超过额度或上下文失效".into());
        }
        let failed = self.failed.swap(0, Ordering::AcqRel);
        if failed != 0 {
            return Ok(Some(Event::Closed(failed)));
        }
        let mut receiver = self.receiver.lock().await;
        match tokio::time::timeout(Duration::from_millis(timeout_ms as u64), receiver.recv()).await
        {
            Ok(Some(event)) => Ok(Some(event)),
            Ok(None) => Err("WebSocket 事件通道关闭".into()),
            Err(_) => Ok(None),
        }
    }
    pub async fn clear(&self) {
        self.connections.lock().await.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn unauthorized_sockets_and_stale_handles_fail_without_network() {
        let active = Arc::new(AtomicBool::new(true));
        let sockets = Sockets::new(active.clone());
        let hosts = vec!["example.org".into()];
        for url in [
            "ws://example.org/",
            "wss://127.0.0.1/",
            "wss://example.org.evil.test/",
            "wss://user:secret@example.org/",
        ] {
            assert!(sockets
                .open(&hosts, url.into(), String::new(), 100)
                .await
                .is_err());
        }
        assert!(sockets.send(1, vec![0]).await.is_err());
        assert!(sockets.send(1, vec![0; 16385]).await.is_err());
        assert!(sockets.next_event(1001).await.is_err());
        assert!(sockets.next_event(1).await.unwrap().is_none());
        active.store(false, Ordering::Release);
        assert!(sockets.next_event(1).await.is_err());
        assert!(sockets
            .open(&hosts, "wss://example.org/".into(), String::new(), 100)
            .await
            .is_err());
        sockets.clear().await;
        assert!(sockets.connections.lock().await.is_empty());
    }
}
