//! Authorized, bounded WebSocket transport. Application protocols stay in guests.
use super::{
    http::{checked_url, resolve_addresses},
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
use tokio::sync::{mpsc, watch, Mutex};
use tokio_tungstenite::tungstenite::{
    client::IntoClientRequest, protocol::WebSocketConfig, Message as Frame,
};

fn ensure_crypto_provider() {
    static INSTALL: std::sync::Once = std::sync::Once::new();
    INSTALL.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

async fn wait_inactive(active: &AtomicBool) {
    while active.load(Ordering::Acquire) {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

const EVENT_CAPACITY: usize = 32;
const COMMAND_CAPACITY: usize = 16;
const CLOSE_GRACE: Duration = Duration::from_millis(500);
const EVENT_GRACE: Duration = Duration::from_millis(250);
const JOIN_GRACE: Duration = Duration::from_secs(1);

struct Connection {
    commands: mpsc::Sender<Vec<u8>>,
    cancel: watch::Sender<bool>,
    cancel_rx: watch::Receiver<bool>,
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
    connections: Arc<Mutex<HashMap<u32, Connection>>>,
    next: AtomicU32,
    failed: Arc<AtomicU32>,
    events: mpsc::Sender<Event>,
    receiver: Mutex<mpsc::Receiver<Event>>,
    active: Arc<AtomicBool>,
}
fn request_stop(cancel: &watch::Sender<bool>) {
    let _ = cancel.send(true);
}
async fn wait_cancel(cancel: &mut watch::Receiver<bool>) {
    while !*cancel.borrow() {
        if cancel.changed().await.is_err() {
            break;
        }
    }
}
async fn wait_stop(cancel: &mut watch::Receiver<bool>, active: &AtomicBool) {
    tokio::select! {
        _ = wait_cancel(cancel) => (),
        _ = wait_inactive(active) => (),
    }
}
impl Sockets {
    pub fn new(active: Arc<AtomicBool>) -> Self {
        let (events, receiver) = mpsc::channel(EVENT_CAPACITY);
        Self {
            connections: Arc::new(Mutex::new(HashMap::new())),
            next: AtomicU32::new(1),
            failed: Arc::new(AtomicU32::new(0)),
            events,
            receiver: Mutex::new(receiver),
            active,
        }
    }
    pub async fn open(&self, input: String, protocol: String, timeout_ms: u32) -> AppResult<u32> {
        if !self.active.load(Ordering::Acquire) {
            return Err("WebSocket 参数或上下文无效".into());
        }
        let mut url = url::Url::parse(&input)
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("websocket.url.parse", error);
            })
            .map_err(|_| "WebSocket 地址无效")?;
        let secure = match url.scheme() {
            "wss" => true,
            "ws" => false,
            _ => return Err("WebSocket 仅支持 WS 或 WSS".into()),
        };
        if secure {
            ensure_crypto_provider();
        }
        url.set_scheme(if secure { "https" } else { "http" })
            .inspect_err(|_| {
                super::diagnostics::warn("websocket.set_scheme", "invalid scheme transition");
            })
            .map_err(|_| "WebSocket 地址无效")?;
        let url = checked_url(url.as_str())?;
        let id = self
            .next
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |id| id.checked_add(1))
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("websocket.handle.allocate", error);
            })
            .map_err(|_| "WebSocket 句柄耗尽")?;
        let connect = async {
            let addresses = resolve_addresses(
                url.host_str().ok_or("WebSocket 目标无效")?,
                url.port_or_known_default().ok_or("WebSocket 目标无效")?,
            )
            .await?;
            let tcp = tokio::net::TcpStream::connect(addresses.as_slice())
                .await
                .inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn("websocket.tcp.connect", error);
                })
                .map_err(|_| "WebSocket 连接失败")?;
            let mut request = input
                .into_client_request()
                .inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn("websocket.handshake.request", error);
                })
                .map_err(|_| "WebSocket 握手参数无效")?;
            let requested: Vec<_> = protocol
                .split(',')
                .map(str::trim)
                .filter(|protocol| !protocol.is_empty())
                .collect();
            if !requested.is_empty() {
                request.headers_mut().insert(
                    "sec-websocket-protocol",
                    requested
                        .join(", ")
                        .parse()
                        .inspect_err(|error| {
                            crate::infrastructure::diagnostics::warn(
                                "websocket.protocol.parse",
                                error,
                            );
                        })
                        .map_err(|_| "WebSocket 子协议无效")?,
                );
            }
            let config = WebSocketConfig::default()
                .max_message_size(None)
                .max_frame_size(None);
            let (socket, response) =
                tokio_tungstenite::client_async_tls_with_config(request, tcp, Some(config), None)
                    .await
                    .inspect_err(|error| {
                        crate::infrastructure::diagnostics::warn("websocket.tls.handshake", error);
                    })
                    .map_err(|_| "WebSocket TLS 或握手失败")?;
            if !requested.is_empty() {
                let selected = response
                    .headers()
                    .get("sec-websocket-protocol")
                    .and_then(|value| {
                        value
                            .to_str()
                            .inspect_err(|error| {
                                crate::infrastructure::diagnostics::warn(
                                    "websocket.protocol.response",
                                    error,
                                );
                            })
                            .ok()
                    })
                    .unwrap_or_default();
                if !selected
                    .split(',')
                    .map(str::trim)
                    .any(|protocol| requested.contains(&protocol))
                {
                    return Err("WebSocket 子协议未获确认".into());
                }
            }
            Ok::<_, String>(socket)
        };
        let socket = if timeout_ms == 0 {
            tokio::select! {
                result = connect => result?,
                _ = wait_inactive(&self.active) => return Err("WebSocket 上下文已失效".into()),
            }
        } else {
            tokio::select! {
                result = tokio::time::timeout(Duration::from_millis(timeout_ms as u64), connect) => {
                    match result {
                        Ok(result) => result?,
                        Err(error) => {
                            crate::infrastructure::diagnostics::warn("websocket.connect.timeout", error);
                            return Err("WebSocket 连接超时".into());
                        }
                    }
                }
                _ = wait_inactive(&self.active) => return Err("WebSocket 上下文已失效".into()),
            }
        };
        if !self.active.load(Ordering::Acquire) {
            return Err("WebSocket 上下文已失效".into());
        }
        let (commands, mut command_rx) = mpsc::channel::<Vec<u8>>(COMMAND_CAPACITY);
        let (cancel, cancel_rx) = watch::channel(false);
        let events = self.events.clone();
        let active = self.active.clone();
        let failed = self.failed.clone();
        let connections = self.connections.clone();
        let mut task_cancel = cancel_rx.clone();
        let mut registered = self.connections.lock().await;
        let task = tokio::spawn(async move {
            let (mut writer, mut reader) = socket.split();
            loop {
                tokio::select! {
                    command = command_rx.recv() => match command {
                        Some(bytes) => {
                            let send = writer.send(Frame::Binary(bytes.into()));
                            let sent = tokio::select! {
                                result = send => result,
                                _ = wait_stop(&mut task_cancel, &active) => break,
                            };
                            if let Err(error) = sent {
                                super::diagnostics::warn("websocket.send", error);
                                break;
                            }
                        }
                        None => break,
                    },
                    frame = reader.next() => match frame {
                        Some(Ok(Frame::Binary(bytes))) => {
                            let publish = events.send(Event::Binary(Message { socket: id, data: bytes.to_vec() }));
                            let published = tokio::select! {
                                result = publish => result,
                                _ = wait_stop(&mut task_cancel, &active) => break,
                            };
                            if let Err(error) = published {
                                super::diagnostics::warn("websocket.publish_binary", error);
                                break;
                            }
                        }
                        Some(Ok(Frame::Ping(bytes))) => {
                            let send = writer.send(Frame::Pong(bytes));
                            let sent = tokio::select! {
                                result = send => result,
                                _ = wait_stop(&mut task_cancel, &active) => break,
                            };
                            if let Err(error) = sent {
                                super::diagnostics::warn("websocket.pong", error);
                                break;
                            }
                        }
                        Some(Ok(Frame::Pong(_))) => (),
                        Some(Err(error)) => { super::diagnostics::warn("websocket.receive", error); break; },
                        _ => break,
                    },
                    _ = wait_stop(&mut task_cancel, &active) => break,
                }
            }
            failed.store(id, Ordering::Release);
            match tokio::time::timeout(CLOSE_GRACE, writer.send(Frame::Close(None))).await {
                Ok(Ok(())) => {}
                Ok(Err(error)) => super::diagnostics::warn("websocket.close_frame", error),
                Err(error) => super::diagnostics::warn("websocket.close_frame_timeout", error),
            }
            match events.try_send(Event::Closed(id)) {
                Ok(()) => {}
                Err(mpsc::error::TrySendError::Full(event)) => {
                    match tokio::time::timeout(EVENT_GRACE, events.send(event)).await {
                        Ok(Ok(())) => {}
                        Ok(Err(error)) => {
                            super::diagnostics::warn("websocket.closed_send", error);
                        }
                        Err(error) => {
                            super::diagnostics::warn("websocket.closed_full", error);
                        }
                    }
                }
                Err(mpsc::error::TrySendError::Closed(_)) => {}
            }
            connections.lock().await.remove(&id);
        });
        registered.insert(
            id,
            Connection {
                commands,
                cancel,
                cancel_rx,
                task: Some(task),
            },
        );
        drop(registered);
        Ok(id)
    }
    pub async fn send(&self, socket: u32, bytes: Vec<u8>) -> AppResult<()> {
        if !self.active.load(Ordering::Acquire) {
            return Err("WebSocket 上下文已失效".into());
        }
        let (commands, mut cancel) = self
            .connections
            .lock()
            .await
            .get(&socket)
            .map(|connection| (connection.commands.clone(), connection.cancel_rx.clone()))
            .ok_or("WebSocket 句柄无效")?;
        tokio::select! {
            result = commands.send(bytes) => result
                .inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn("websocket.send.enqueue", error);
                })
                .map_err(|_| "WebSocket 已关闭".into()),
            _ = wait_cancel(&mut cancel) => Err("WebSocket 已关闭".into()),
            _ = wait_inactive(&self.active) => Err("WebSocket 上下文已失效".into()),
        }
    }
    pub async fn close(&self, socket: u32) -> AppResult<()> {
        let connection = { self.connections.lock().await.remove(&socket) };
        if let Some(mut connection) = connection {
            request_stop(&connection.cancel);
            if let Some(mut task) = connection.task.take() {
                match tokio::time::timeout(JOIN_GRACE, &mut task).await {
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
        if !self.active.load(Ordering::Acquire) {
            return Err("WebSocket 上下文已失效".into());
        }
        let failed = self.failed.swap(0, Ordering::AcqRel);
        if failed != 0 {
            return Ok(Some(Event::Closed(failed)));
        }
        let mut receiver = self.receiver.lock().await;
        let receive = async {
            if timeout_ms == 0 {
                return receiver
                    .recv()
                    .await
                    .map(Some)
                    .ok_or_else(|| "WebSocket 事件通道关闭".into());
            }
            match tokio::time::timeout(Duration::from_millis(timeout_ms as u64), receiver.recv())
                .await
            {
                Ok(Some(event)) => Ok(Some(event)),
                Ok(None) => Err("WebSocket 事件通道关闭".into()),
                Err(_) => Ok(None),
            }
        };
        tokio::select! {
            result = receive => result,
            _ = wait_inactive(&self.active) => Err("WebSocket 上下文已失效".into()),
        }
    }
    pub async fn clear(&self) {
        let mut connections = self.connections.lock().await;
        for (_, connection) in connections.drain() {
            request_stop(&connection.cancel);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;
    use tokio::net::TcpListener;
    use tokio_tungstenite::accept_async;

    async fn event_flood(frames: usize) -> (std::net::SocketAddr, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut socket = accept_async(stream).await.unwrap();
            for _ in 0..frames {
                socket
                    .send(Frame::Binary(vec![1; 32].into()))
                    .await
                    .unwrap();
            }
            tokio::time::sleep(Duration::from_secs(30)).await;
        });
        (address, server)
    }

    async fn silent_server() -> (std::net::SocketAddr, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let _socket = accept_async(stream).await.unwrap();
            tokio::time::sleep(Duration::from_secs(30)).await;
        });
        (address, server)
    }

    async fn echo_server(
        connections: usize,
    ) -> (std::net::SocketAddr, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            for _ in 0..connections {
                let (stream, _) = listener.accept().await.unwrap();
                tokio::spawn(async move {
                    let mut socket = accept_async(stream).await.unwrap();
                    while let Some(Ok(frame)) = socket.next().await {
                        match frame {
                            Frame::Binary(bytes) => {
                                socket.send(Frame::Binary(bytes)).await.unwrap();
                            }
                            Frame::Close(_) => break,
                            _ => {}
                        }
                    }
                });
            }
        });
        (address, server)
    }

    async fn stalled_handshake() -> (std::net::SocketAddr, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (_stream, _) = listener.accept().await.unwrap();
            tokio::time::sleep(Duration::from_secs(30)).await;
        });
        (address, server)
    }

    #[tokio::test]
    async fn accepts_arbitrary_urls_unlimited_connections_and_large_frames() {
        let active = Arc::new(AtomicBool::new(true));
        let sockets = Sockets::new(active.clone());
        for url in ["http://example.org/", "ws://", "wss://"] {
            assert!(sockets.open(url.into(), String::new(), 100).await.is_err());
        }
        assert!(sockets.send(1, vec![0]).await.is_err());
        assert!(sockets.next_event(1).await.unwrap().is_none());

        let (address, server) = echo_server(3).await;
        let first = sockets
            .open(format!("ws://{address}/first?query=1"), String::new(), 0)
            .await
            .unwrap();
        let second = sockets
            .open(
                format!("ws://user:secret@{address}/second"),
                String::new(),
                0,
            )
            .await
            .unwrap();
        let third = sockets
            .open(format!("ws://{address}/third"), String::new(), 0)
            .await
            .unwrap();

        let payload = vec![7u8; 256 * 1024];
        sockets.send(first, payload.clone()).await.unwrap();
        let echoed = tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                match sockets.next_event(1000).await.unwrap() {
                    Some(Event::Binary(message)) if message.socket == first => {
                        break message.data;
                    }
                    Some(_) | None => {}
                }
            }
        })
        .await
        .unwrap();
        assert_eq!(echoed, payload);

        for id in [first, second, third] {
            tokio::time::timeout(Duration::from_secs(2), sockets.close(id))
                .await
                .unwrap()
                .unwrap();
        }
        assert!(sockets.connections.lock().await.is_empty());
        server.abort();

        active.store(false, Ordering::Release);
        assert!(sockets.next_event(1).await.is_err());
        assert!(sockets
            .open("wss://example.org/".into(), String::new(), 100)
            .await
            .is_err());
        sockets.clear().await;
        assert!(sockets.connections.lock().await.is_empty());
    }

    #[tokio::test]
    async fn event_backpressure_yields_to_close_disable_clear_and_recovery() {
        let active = Arc::new(AtomicBool::new(true));
        let sockets = Arc::new(Sockets::new(active.clone()));
        let (address, server) = event_flood(128).await;
        sockets
            .open(format!("ws://{address}/"), String::new(), 0)
            .await
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            if sockets.receiver.lock().await.len() >= EVENT_CAPACITY {
                break;
            }
            assert!(Instant::now() < deadline, "event queue did not fill");
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        let id = *sockets.connections.lock().await.keys().next().unwrap();
        tokio::time::timeout(Duration::from_secs(2), sockets.close(id))
            .await
            .unwrap()
            .unwrap();
        assert!(sockets.connections.lock().await.is_empty());

        let (address, second) = event_flood(128).await;
        let id = sockets
            .open(format!("ws://{address}/"), String::new(), 0)
            .await
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(3);
        while sockets.receiver.lock().await.len() < EVENT_CAPACITY {
            assert!(Instant::now() < deadline, "event queue did not stay full");
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        active.store(false, Ordering::Release);
        assert!(
            tokio::time::timeout(Duration::from_millis(300), sockets.next_event(0))
                .await
                .unwrap()
                .is_err()
        );
        assert!(sockets.send(id, vec![1]).await.is_err());
        tokio::time::timeout(Duration::from_millis(300), sockets.clear())
            .await
            .unwrap();
        assert!(sockets.connections.lock().await.is_empty());
        active.store(true, Ordering::Release);
        while sockets.next_event(1).await.unwrap().is_some() {}
        server.abort();
        second.abort();
    }

    #[tokio::test]
    async fn close_interrupts_a_full_command_queue_without_hanging() {
        let active = Arc::new(AtomicBool::new(true));
        let sockets = Arc::new(Sockets::new(active));
        let (address, server) = silent_server().await;
        let id = sockets
            .open(format!("ws://{address}/"), String::new(), 0)
            .await
            .unwrap();
        let sender = tokio::spawn({
            let sockets = sockets.clone();
            async move {
                for _ in 0..128 {
                    if sockets.send(id, vec![7; 256 * 1024]).await.is_err() {
                        break;
                    }
                }
            }
        });
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            let full = sockets
                .connections
                .lock()
                .await
                .get(&id)
                .is_some_and(|connection| connection.commands.capacity() == 0);
            if full {
                break;
            }
            assert!(Instant::now() < deadline, "command queue did not fill");
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        sender.abort();
        let _ = sender.await;
        let blocked = tokio::spawn({
            let sockets = sockets.clone();
            async move { sockets.send(id, vec![9; 256 * 1024]).await }
        });
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(
            !blocked.is_finished(),
            "send should wait for command capacity"
        );

        let started = Instant::now();
        tokio::time::timeout(Duration::from_secs(3), sockets.close(id))
            .await
            .unwrap()
            .unwrap();
        assert!(
            started.elapsed() < Duration::from_secs(2),
            "close took {:?}",
            started.elapsed()
        );
        assert!(tokio::time::timeout(Duration::from_secs(1), blocked)
            .await
            .unwrap()
            .unwrap()
            .is_err());
        assert!(sockets.connections.lock().await.get(&id).is_none());
        server.abort();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn closing_during_handshake_leaves_no_task_and_finished_sockets_unregister() {
        let active = Arc::new(AtomicBool::new(true));
        let sockets = Arc::new(Sockets::new(active.clone()));
        let (address, server) = stalled_handshake().await;
        let pending = tokio::spawn({
            let sockets = sockets.clone();
            async move {
                sockets
                    .open(format!("ws://{address}/"), String::new(), 0)
                    .await
            }
        });
        tokio::time::sleep(Duration::from_millis(100)).await;
        active.store(false, Ordering::Release);
        assert!(tokio::time::timeout(Duration::from_secs(1), pending)
            .await
            .unwrap()
            .unwrap()
            .is_err());
        assert!(sockets.connections.lock().await.is_empty());
        server.abort();

        active.store(true, Ordering::Release);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let _socket = accept_async(stream).await.unwrap();
        });
        assert!(sockets
            .open(format!("ws://{address}/"), String::new(), 0)
            .await
            .is_ok());
        tokio::time::sleep(Duration::from_millis(1200)).await;
        assert!(sockets.connections.lock().await.is_empty());
        server.abort();
    }
}
