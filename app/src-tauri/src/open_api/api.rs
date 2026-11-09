use super::dto::{
    Accepted, CoverAsset, Health, LyricsAsset, Player, PlayerCommand, Progress, Queue, State,
};
use crate::{model::Track, playback::publication::PlayerBootstrap};
use axum::{
    extract::{
        rejection::{JsonRejection, QueryRejection},
        ws::{CloseFrame, Message, WebSocket},
        DefaultBodyLimit, Query, State as AxumState, WebSocketUpgrade,
    },
    http::{
        header::{CACHE_CONTROL, CONTENT_TYPE, ETAG, HOST, IF_NONE_MATCH, X_CONTENT_TYPE_OPTIONS},
        HeaderMap, HeaderName, HeaderValue, Method, Request, StatusCode,
    },
    middleware::{from_fn_with_state, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use futures_util::{stream::SplitSink, SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    future::Future,
    pin::Pin,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tokio::sync::{broadcast, watch};
use tower_http::cors::{Any, CorsLayer};

pub const MAX_BODY_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_WS_BYTES: usize = 16 * 1024;
pub const MAX_CONNECTIONS: usize = 16;
pub const MAX_COVER_BYTES: usize = 16 * 1024 * 1024;
pub const EVENT_CAPACITY: usize = 64;
const HEARTBEAT: Duration = Duration::from_secs(30);
const SEND_TIMEOUT: Duration = Duration::from_secs(5);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

pub type ProviderFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, ApiError>> + Send + 'a>>;

pub trait Provider: Send + Sync {
    fn bootstrap(&self) -> Result<PlayerBootstrap, ApiError>;
    fn track(&self, track_id: &str) -> Result<Track, ApiError>;
    fn lyrics(&self, track: Track) -> ProviderFuture<'_, Option<LyricsAsset>>;
    fn cover(&self, track: Track) -> ProviderFuture<'_, Option<CoverAsset>>;
    fn command(&self, command: PlayerCommand) -> ProviderFuture<'_, ()>;
}

#[derive(Clone)]
pub struct Context {
    pub provider: Arc<dyn Provider>,
    pub events: broadcast::Sender<String>,
    allowed_hosts: Arc<HashSet<String>>,
    permits: Arc<tokio::sync::Semaphore>,
    http_permits: Arc<tokio::sync::Semaphore>,
    cover_permits: Arc<tokio::sync::Semaphore>,
    connections: Arc<AtomicUsize>,
    shutdown: watch::Receiver<bool>,
}

impl Context {
    pub fn new(
        provider: Arc<dyn Provider>,
        allowed_hosts: HashSet<String>,
        event_capacity: usize,
        shutdown: watch::Receiver<bool>,
    ) -> Self {
        let (events, _) = broadcast::channel(event_capacity);
        Self {
            provider,
            events,
            allowed_hosts: Arc::new(allowed_hosts),
            permits: Arc::new(tokio::sync::Semaphore::new(MAX_CONNECTIONS)),
            http_permits: Arc::new(tokio::sync::Semaphore::new(32)),
            cover_permits: Arc::new(tokio::sync::Semaphore::new(8)),
            connections: Arc::new(AtomicUsize::new(0)),
            shutdown,
        }
    }

    pub fn connections(&self) -> usize {
        self.connections.load(Ordering::Relaxed)
    }

    pub fn publish<T: Serialize>(&self, event_type: &str, value: &T) {
        match event(event_type, value) {
            Ok(payload) => {
                let _ = self.events.send(payload);
            }
            Err(error) => {
                crate::infrastructure::diagnostics::warn("open_api.event.serialize", error);
            }
        }
    }
}

#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub code: &'static str,
    pub message: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ErrorBody {
    code: &'static str,
    message: &'static str,
}

impl ApiError {
    pub fn invalid(message: &'static str) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code: "invalid_request",
            message,
        }
    }

    pub fn not_found(message: &'static str) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            code: "not_found",
            message,
        }
    }

    pub fn conflict(message: &'static str) -> Self {
        Self {
            status: StatusCode::CONFLICT,
            code: "stale_queue",
            message,
        }
    }

    pub fn busy() -> Self {
        Self {
            status: StatusCode::TOO_MANY_REQUESTS,
            code: "busy",
            message: "播放器暂时忙碌",
        }
    }

    pub fn unavailable() -> Self {
        Self {
            status: StatusCode::SERVICE_UNAVAILABLE,
            code: "unavailable",
            message: "播放状态暂时不可用",
        }
    }

    pub fn internal() -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "internal",
            message: "服务内部错误",
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        crate::infrastructure::diagnostics::warn(
            "open_api.http",
            format!("status={} code={}", self.status, self.code),
        );
        let body = ErrorBody {
            code: self.code,
            message: self.message,
        };
        (self.status, Json(body)).into_response()
    }
}

pub fn router(context: Arc<Context>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_headers(Any)
        .expose_headers([
            ETAG,
            HeaderName::from_static("x-nons-profile-version"),
            HeaderName::from_static("x-nons-effective-offset-ms"),
            HeaderName::from_static("x-nons-lyric-source"),
        ])
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS]);
    Router::new()
        .route("/v1/health", get(health))
        .route("/v1/player", get(player))
        .route("/v1/queue", get(queue))
        .route("/v1/lyrics", get(lyrics))
        .route("/v1/cover", get(cover))
        .route("/v1/docs", get(docs))
        .route("/v1/player/commands", post(command))
        .route("/v1/events", get(events))
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .layer(cors)
        .route_layer(from_fn_with_state(context.clone(), validate_host))
        .route_layer(from_fn_with_state(context.clone(), bound_http))
        .with_state(context)
}

pub fn event<T: Serialize>(event_type: &str, value: &T) -> Result<String, serde_json::Error> {
    #[derive(Serialize)]
    struct Envelope<'a, T> {
        r#type: &'a str,
        #[serde(flatten)]
        value: &'a T,
    }
    serde_json::to_string(&Envelope {
        r#type: event_type,
        value,
    })
}

async fn validate_host(
    AxumState(context): AxumState<Arc<Context>>,
    request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    let Some(authority) = request
        .headers()
        .get(HOST)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<http::uri::Authority>().ok())
    else {
        return ApiError::invalid("缺少有效的 Host 请求头").into_response();
    };
    if !context.allowed_hosts.contains(authority.host()) {
        return ApiError::not_found("Host 不受信任").into_response();
    }
    next.run(request).await
}

async fn bound_http(
    AxumState(context): AxumState<Arc<Context>>,
    request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    let Ok(permit) = context.http_permits.clone().try_acquire_owned() else {
        return ApiError::busy().into_response();
    };
    let result = tokio::time::timeout(REQUEST_TIMEOUT, next.run(request)).await;
    drop(permit);
    match result {
        Ok(response) => response,
        Err(_) => ApiError {
            status: StatusCode::GATEWAY_TIMEOUT,
            code: "timeout",
            message: "请求处理超时",
        }
        .into_response(),
    }
}

async fn health() -> Json<Health> {
    Json(Health {
        version: env!("CARGO_PKG_VERSION"),
        protocol_version: 1,
        capabilities: [
            "player.read",
            "player.control",
            "queue.read",
            "lyrics.ttml",
            "cover.read",
            "events",
            "docs",
        ],
    })
}

async fn docs() -> Response {
    (
        StatusCode::OK,
        [(CONTENT_TYPE, "text/plain; charset=utf-8")],
        include_str!("../../../../docs/open-api.md"),
    )
        .into_response()
}

async fn player(AxumState(context): AxumState<Arc<Context>>) -> Result<Json<Player>, ApiError> {
    let value = context.provider.bootstrap()?;
    Ok(Json(Player::from_bootstrap(&value)))
}

async fn queue(AxumState(context): AxumState<Arc<Context>>) -> Result<Json<Queue>, ApiError> {
    let value = context.provider.bootstrap()?;
    Ok(Json(Queue::from_public(&value.queue)))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TrackQuery {
    track_id: String,
}

async fn lyrics(
    AxumState(context): AxumState<Arc<Context>>,
    query: Result<Query<TrackQuery>, QueryRejection>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let Query(query) = track_query(query)?;
    let track = context.provider.track(&query.track_id)?;
    let Some(asset) = context.provider.lyrics(track).await? else {
        return Err(ApiError::not_found("当前曲目没有可用歌词"));
    };
    let etag = format!("\"{}\"", asset.etag);
    let mut response = if headers
        .get(IF_NONE_MATCH)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.split(',').any(|value| value.trim() == etag))
    {
        StatusCode::NOT_MODIFIED.into_response()
    } else {
        (StatusCode::OK, asset.body).into_response()
    };
    response.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/ttml+xml; charset=utf-8"),
    );
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("private, no-cache"));
    response.headers_mut().insert(
        ETAG,
        HeaderValue::from_str(&etag).map_err(|_| ApiError::internal())?,
    );
    response.headers_mut().insert(
        "x-nons-profile-version",
        HeaderValue::from_str(&asset.profile).map_err(|_| ApiError::internal())?,
    );
    response.headers_mut().insert(
        "x-nons-effective-offset-ms",
        HeaderValue::from_str(&asset.effective_offset_ms.to_string())
            .map_err(|_| ApiError::internal())?,
    );
    response.headers_mut().insert(
        "x-nons-lyric-source",
        HeaderValue::from_str(&asset.source).map_err(|_| ApiError::internal())?,
    );
    Ok(response)
}

async fn cover(
    AxumState(context): AxumState<Arc<Context>>,
    query: Result<Query<TrackQuery>, QueryRejection>,
) -> Result<Response, ApiError> {
    let Query(query) = track_query(query)?;
    let permit = tokio::time::timeout(
        Duration::from_secs(3),
        context.cover_permits.clone().acquire_owned(),
    )
    .await
    .map_err(|_| ApiError::busy())?
    .map_err(|_| ApiError::unavailable())?;
    let track = context.provider.track(&query.track_id)?;
    let Some(asset) = context.provider.cover(track).await? else {
        return Err(ApiError::not_found("当前曲目没有封面"));
    };
    drop(permit);
    let mut response = (StatusCode::OK, asset.bytes).into_response();
    response.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_str(&asset.content_type).map_err(|_| ApiError::internal())?,
    );
    response.headers_mut().insert(
        CACHE_CONTROL,
        HeaderValue::from_static("private, max-age=3600"),
    );
    response
        .headers_mut()
        .insert(X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    Ok(response)
}

fn track_query(
    value: Result<Query<TrackQuery>, QueryRejection>,
) -> Result<Query<TrackQuery>, ApiError> {
    value.map_err(|error| {
        crate::infrastructure::diagnostics::warn("open_api.query.track", error);
        ApiError::invalid("trackId 查询参数无效")
    })
}

async fn command(
    AxumState(context): AxumState<Arc<Context>>,
    value: Result<Json<PlayerCommand>, JsonRejection>,
) -> Result<(StatusCode, Json<Accepted>), ApiError> {
    let Json(value) = value.map_err(|error| {
        crate::infrastructure::diagnostics::warn("open_api.command.json", error);
        ApiError::invalid("请求正文不是有效的命令 JSON")
    })?;
    context.provider.command(value).await?;
    Ok((StatusCode::ACCEPTED, Json(Accepted { accepted: true })))
}

async fn events(
    AxumState(context): AxumState<Arc<Context>>,
    upgrade: WebSocketUpgrade,
) -> Result<Response, ApiError> {
    let Ok(permit) = context.permits.clone().try_acquire_owned() else {
        return Err(ApiError {
            status: StatusCode::SERVICE_UNAVAILABLE,
            code: "too_many_connections",
            message: "事件连接数已达上限",
        });
    };
    let guard = WsGuard::new(permit, context.connections.clone());
    let provider = context.provider.clone();
    let receiver = context.events.subscribe();
    let shutdown = context.shutdown();
    Ok(upgrade
        .max_message_size(MAX_WS_BYTES)
        .max_frame_size(MAX_WS_BYTES)
        .on_upgrade(move |socket| async move {
            let _guard = guard;
            socket_loop(socket, provider, receiver, shutdown).await;
        })
        .into_response())
}

struct WsGuard {
    _permit: tokio::sync::OwnedSemaphorePermit,
    connections: Arc<AtomicUsize>,
}

impl WsGuard {
    fn new(permit: tokio::sync::OwnedSemaphorePermit, connections: Arc<AtomicUsize>) -> Self {
        connections.fetch_add(1, Ordering::Relaxed);
        Self {
            _permit: permit,
            connections,
        }
    }
}

impl Drop for WsGuard {
    fn drop(&mut self) {
        self.connections.fetch_sub(1, Ordering::Relaxed);
    }
}

impl Context {
    fn shutdown(&self) -> watch::Receiver<bool> {
        self.shutdown.clone()
    }
}

fn event_sequence(value: &str) -> Option<u64> {
    serde_json::from_str::<serde_json::Value>(value)
        .ok()?
        .get("sequence")?
        .as_u64()
}

async fn send_bounded(sender: &mut SplitSink<WebSocket, Message>, message: Message) -> bool {
    match tokio::time::timeout(SEND_TIMEOUT, sender.send(message)).await {
        Ok(Ok(())) => true,
        Ok(Err(error)) => {
            crate::infrastructure::diagnostics::warn("open_api.ws.send", error);
            false
        }
        Err(error) => {
            crate::infrastructure::diagnostics::warn("open_api.ws.send_timeout", error);
            false
        }
    }
}

async fn send_or_stop(
    sender: &mut SplitSink<WebSocket, Message>,
    message: Message,
    shutdown: &mut watch::Receiver<bool>,
) -> bool {
    if *shutdown.borrow() {
        return false;
    }
    loop {
        tokio::select! {
            changed = shutdown.changed() => {
                if changed.is_err() || *shutdown.borrow() {
                    return false;
                }
            }
            result = tokio::time::timeout(SEND_TIMEOUT, sender.send(message.clone())) => {
                return match result {
                    Ok(Ok(())) => true,
                    Ok(Err(error)) => {
                        crate::infrastructure::diagnostics::warn("open_api.ws.send", error);
                        false
                    }
                    Err(error) => {
                        crate::infrastructure::diagnostics::warn("open_api.ws.send_timeout", error);
                        false
                    }
                };
            }
        }
    }
}

async fn close(sender: &mut SplitSink<WebSocket, Message>, code: u16, reason: &'static str) {
    match tokio::time::timeout(
        SEND_TIMEOUT,
        sender.send(Message::Close(Some(CloseFrame {
            code,
            reason: reason.into(),
        }))),
    )
    .await
    {
        Ok(Ok(())) => {}
        Ok(Err(error)) => {
            crate::infrastructure::diagnostics::warn("open_api.ws.close", error);
        }
        Err(error) => {
            crate::infrastructure::diagnostics::warn("open_api.ws.close_timeout", error);
        }
    }
}

async fn socket_loop(
    socket: WebSocket,
    provider: Arc<dyn Provider>,
    mut events: broadcast::Receiver<String>,
    mut shutdown: watch::Receiver<bool>,
) {
    if *shutdown.borrow() {
        return;
    }
    let (mut sender, mut receiver) = socket.split();
    let (snapshot_sequence, initial) = match (|| -> Result<(u64, Vec<String>), ApiError> {
        let bootstrap = provider.bootstrap()?;
        Ok((
            bootstrap.state.sequence,
            vec![
                event(
                    "ready",
                    &serde_json::json!({
                        "serverTime": (time::OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000) as i64
                    }),
                )
                .map_err(|_| ApiError::internal())?,
                event("player.queue", &Queue::from_public(&bootstrap.queue))
                    .map_err(|_| ApiError::internal())?,
                event("player.state", &State::from_public(&bootstrap.state))
                    .map_err(|_| ApiError::internal())?,
                event("player.progress", &Progress::from_public(&bootstrap.progress))
                    .map_err(|_| ApiError::internal())?,
            ],
        ))
    })() {
        Ok(initial) => initial,
        Err(error) => {
            crate::infrastructure::diagnostics::warn(
                "open_api.ws.snapshot",
                format!("{}: {}", error.code, error.message),
            );
            let body = serde_json::to_string(&ErrorBody {
                code: error.code,
                message: error.message,
            })
            .unwrap_or_else(|_| "{\"code\":\"internal\"}".into());
            let _ = send_bounded(&mut sender, Message::Text(body.into())).await;
            close(&mut sender, 1011, "snapshot unavailable").await;
            return;
        }
    };
    for item in initial {
        if !send_or_stop(&mut sender, Message::Text(item.into()), &mut shutdown).await {
            return;
        }
    }

    let mut heartbeat = tokio::time::interval(HEARTBEAT);
    let mut last_pong = Instant::now();
    loop {
        tokio::select! {
            changed = shutdown.changed() => {
                if changed.is_err() || *shutdown.borrow() {
                    close(&mut sender, 1001, "service stopped").await;
                    return;
                }
            }
            item = events.recv() => {
                match item {
                    Ok(item) => {
                        if event_sequence(&item)
                            .is_some_and(|sequence| sequence <= snapshot_sequence)
                        {
                            continue;
                        }
                        if !send_or_stop(
                            &mut sender,
                            Message::Text(item.into()),
                            &mut shutdown,
                        )
                        .await
                        {
                            return;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        let lagged = event("events.lagged", &serde_json::json!({"resync": true}))
                            .unwrap_or_else(|_| "{\"type\":\"events.lagged\"}".into());
                        let _ = send_bounded(&mut sender, Message::Text(lagged.into())).await;
                        close(&mut sender, 1013, "slow consumer").await;
                        return;
                    }
                    Err(broadcast::error::RecvError::Closed) => return,
                }
            }
            incoming = receiver.next() => {
                match incoming {
                    Some(Ok(Message::Ping(value))) => {
                        if !send_or_stop(&mut sender, Message::Pong(value), &mut shutdown).await {
                            return;
                        }
                    }
                    Some(Ok(Message::Pong(_))) => last_pong = Instant::now(),
                    Some(Ok(Message::Close(_))) | None => {
                        let _ = send_bounded(&mut sender, Message::Close(None)).await;
                        return;
                    }
                    Some(Ok(_)) => {}
                    Some(Err(error)) => {
                        crate::infrastructure::diagnostics::warn("open_api.ws.receive", error);
                        return;
                    }
                }
            }
            _ = heartbeat.tick() => {
                if last_pong.elapsed() > HEARTBEAT * 2 {
                    close(&mut sender, 1001, "heartbeat timeout").await;
                    return;
                }
                if !send_or_stop(
                    &mut sender,
                    Message::Ping(Vec::new().into()),
                    &mut shutdown,
                )
                .await
                {
                    return;
                }
            }
        }
    }
}

pub fn check_queue_revision(bootstrap: &PlayerBootstrap, expected: u64) -> Result<(), ApiError> {
    if bootstrap.queue.queue_revision == expected {
        Ok(())
    } else {
        Err(ApiError::conflict("播放队列已变化，请重新同步"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        PlaybackOptions, PlaybackStatus, Progress as ModelProgress, RepeatMode, TrackSource,
    };
    use crate::open_api::dto::public_track_id;
    use crate::playback::publication::{PlayerProgress, PlayerQueue, PlayerState};
    use std::net::{Ipv4Addr, SocketAddr};
    use std::sync::Mutex;
    use tokio_tungstenite::{connect_async, tungstenite::Message as ClientMessage};

    struct MockProvider {
        bootstrap: Mutex<PlayerBootstrap>,
        track: Track,
        commands: Mutex<Vec<PlayerCommand>>,
    }

    fn track(path: &str) -> Track {
        Track {
            key: "internal-track-key".into(),
            title: "Song".into(),
            aliases: vec![],
            artist: "Artist".into(),
            artists: vec![],
            album: "Album".into(),
            duration_ms: 100_000,
            cover: "/private/cache/cover.png".into(),
            source: TrackSource::Local {
                path: path.into(),
                netease_id: None,
                cue: None,
            },
        }
    }

    fn bootstrap(sequence: u64, queue_revision: u64) -> PlayerBootstrap {
        let track = track("C:/private/music/song.flac");
        PlayerBootstrap {
            state: PlayerState {
                sequence,
                queue_revision,
                revision: 1,
                playback_options: PlaybackOptions::default(),
                private_fm_session: None,
                shuffle: true,
                repeat_mode: RepeatMode::Off,
                index: Some(0),
                status: PlaybackStatus::Paused,
                volume: 0.7,
                device_id: Some("private-device".into()),
                actual_quality: Some("lossless".into()),
                playback_source: None,
                playback_mode: None,
                local_playback_available: false,
                local_playback_quality: None,
                error: Some("C:/private/failure".into()),
                media_error: None,
            },
            queue: PlayerQueue {
                sequence,
                queue_revision,
                queue: Arc::new(vec![track]),
                shuffle_order: Arc::new(vec![0]),
            },
            progress: PlayerProgress {
                sequence,
                value: ModelProgress {
                    revision: 1,
                    position_ms: 25,
                    duration_ms: 100_000,
                    status: PlaybackStatus::Paused,
                },
            },
        }
    }

    impl MockProvider {
        fn new() -> Self {
            Self {
                bootstrap: Mutex::new(bootstrap(1, 7)),
                track: track("C:/private/music/song.flac"),
                commands: Mutex::new(Vec::new()),
            }
        }
    }

    impl Provider for MockProvider {
        fn bootstrap(&self) -> Result<PlayerBootstrap, ApiError> {
            Ok(self.bootstrap.lock().unwrap().clone())
        }

        fn track(&self, track_id: &str) -> Result<Track, ApiError> {
            if track_id == public_track_id(&self.track.key) {
                Ok(self.track.clone())
            } else {
                Err(ApiError::not_found("曲目不在播放队列中"))
            }
        }

        fn lyrics(&self, _track: Track) -> ProviderFuture<'_, Option<LyricsAsset>> {
            Box::pin(async {
                Ok(Some(LyricsAsset {
                    body: "<tt><body/></tt>".into(),
                    source: "test".into(),
                    profile: "test-v1".into(),
                    effective_offset_ms: -20_000,
                    etag: "stable".into(),
                }))
            })
        }

        fn cover(&self, _track: Track) -> ProviderFuture<'_, Option<CoverAsset>> {
            Box::pin(async {
                Ok(Some(CoverAsset {
                    bytes: b"\x89PNG\r\n\x1a\n".to_vec(),
                    content_type: "image/png".into(),
                }))
            })
        }

        fn command(&self, command: PlayerCommand) -> ProviderFuture<'_, ()> {
            Box::pin(async move {
                if let PlayerCommand::Shuffle { queue_revision, .. }
                | PlayerCommand::Repeat { queue_revision, .. }
                | PlayerCommand::Jump { queue_revision, .. } = &command
                {
                    let current = self.bootstrap.lock().unwrap().queue.queue_revision;
                    if *queue_revision != current {
                        return Err(ApiError::conflict("播放队列已变化，请重新同步"));
                    }
                }
                self.commands.lock().unwrap().push(command);
                Ok(())
            })
        }
    }

    fn context(
        provider: Arc<MockProvider>,
        capacity: usize,
    ) -> (Arc<Context>, watch::Sender<bool>) {
        let (shutdown, receiver) = watch::channel(false);
        (
            Arc::new(Context::new(
                provider,
                HashSet::from(["127.0.0.1".into(), "localhost".into()]),
                capacity,
                receiver,
            )),
            shutdown,
        )
    }

    async fn serve(
        context: Arc<Context>,
        shutdown: watch::Sender<bool>,
    ) -> (SocketAddr, tokio::task::JoinHandle<()>, watch::Sender<bool>) {
        let listener = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .await
            .unwrap();
        let address = listener.local_addr().unwrap();
        let mut receiver = context.shutdown();
        let task = tokio::spawn(async move {
            axum::serve(listener, router(context))
                .with_graceful_shutdown(async move {
                    let _ = receiver.changed().await;
                })
                .await
                .unwrap();
        });
        (address, task, shutdown)
    }

    #[test]
    fn command_json_uses_camel_case_and_rejects_field_style_mismatch() {
        let command: PlayerCommand =
            serde_json::from_str(r#"{"action":"jump","queueRevision":7,"trackId":"abc"}"#).unwrap();
        assert!(matches!(
            command,
            PlayerCommand::Jump {
                queue_revision: 7,
                ..
            }
        ));
        assert!(serde_json::from_str::<PlayerCommand>(
            r#"{"action":"jump","queue_revision":7,"trackId":"abc"}"#
        )
        .is_err());
    }

    #[tokio::test]
    async fn real_loopback_routes_hide_paths_and_return_stable_contracts() {
        let provider = Arc::new(MockProvider::new());
        let (context, shutdown) = context(provider.clone(), 64);
        let (address, server, shutdown) = serve(context.clone(), shutdown).await;
        let client = reqwest::Client::new();
        let player = client
            .get(format!("http://{address}/v1/player"))
            .send()
            .await
            .unwrap();
        assert_eq!(player.status(), StatusCode::OK);
        let body = player.text().await.unwrap();
        assert!(!body.contains("C:/private"));
        assert!(body.contains("internal-track-key") == false);
        assert!(body.contains("\"error\":\"playback_error\""));

        let invalid = client
            .post(format!("http://{address}/v1/player/commands"))
            .header(CONTENT_TYPE, "application/json")
            .body("{")
            .send()
            .await
            .unwrap();
        assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            invalid.json::<serde_json::Value>().await.unwrap()["code"],
            "invalid_request"
        );
        let invalid_query = client
            .get(format!("http://{address}/v1/lyrics"))
            .send()
            .await
            .unwrap();
        assert_eq!(invalid_query.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            invalid_query.json::<serde_json::Value>().await.unwrap()["code"],
            "invalid_request"
        );

        let command = PlayerCommand::Shuffle {
            queue_revision: 7,
            shuffle: false,
        };
        let accepted = client
            .post(format!("http://{address}/v1/player/commands"))
            .json(&command)
            .send()
            .await
            .unwrap();
        assert_eq!(accepted.status(), StatusCode::ACCEPTED);
        assert_eq!(provider.commands.lock().unwrap().len(), 1);

        let cors = client
            .get(format!(
                "http://{address}/v1/lyrics?trackId={}",
                public_track_id(&provider.track.key)
            ))
            .header("origin", "http://localhost:3000")
            .send()
            .await
            .unwrap();
        assert!(cors
            .headers()
            .get("access-control-expose-headers")
            .unwrap()
            .to_str()
            .unwrap()
            .contains("x-nons-effective-offset-ms"));

        let host = client
            .get(format!("http://{address}/v1/health"))
            .header(HOST, "attacker.example")
            .send()
            .await
            .unwrap();
        assert_eq!(host.status(), StatusCode::NOT_FOUND);
        let _ = shutdown.send(true);
        server.await.unwrap();
    }

    #[tokio::test]
    async fn lyric_304_keeps_latest_offset_and_etag() {
        let provider = Arc::new(MockProvider::new());
        let (context, shutdown) = context(provider.clone(), 64);
        let (address, server, shutdown) = serve(context, shutdown).await;
        let id = public_track_id(&provider.track.key);
        let response = reqwest::Client::new()
            .get(format!("http://{address}/v1/lyrics?trackId={id}"))
            .header(IF_NONE_MATCH, "\"stable\"")
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_MODIFIED);
        assert_eq!(response.headers()["x-nons-effective-offset-ms"], "-20000");
        assert_eq!(response.headers()[ETAG], "\"stable\"");
        let _ = shutdown.send(true);
        server.await.unwrap();
    }

    #[tokio::test]
    async fn websocket_snapshot_filters_old_sequences_and_stop_closes_connections() {
        let provider = Arc::new(MockProvider::new());
        let (context, shutdown) = context(provider, 16);
        let (address, server, shutdown) = serve(context.clone(), shutdown).await;
        let (mut socket, _) = connect_async(format!("ws://{address}/v1/events"))
            .await
            .unwrap();
        context.publish(
            "player.progress",
            &serde_json::json!({"sequence": 0, "positionMs": 1}),
        );
        context.publish(
            "player.progress",
            &serde_json::json!({"sequence": 2, "positionMs": 2}),
        );
        for expected in ["ready", "player.queue", "player.state", "player.progress"] {
            let text = socket
                .next()
                .await
                .unwrap()
                .unwrap()
                .to_text()
                .unwrap()
                .to_owned();
            assert!(text.contains(expected), "{text}");
        }
        let next = socket.next().await.unwrap().unwrap();
        let text = next.to_text().unwrap();
        assert!(text.contains("\"positionMs\":2"), "{text}");
        assert!(!text.contains("\"positionMs\":1"));
        assert_eq!(context.connections(), 1);

        let _ = shutdown.send(true);
        server.await.unwrap();
        let closed = tokio::time::timeout(Duration::from_secs(2), socket.next())
            .await
            .unwrap();
        assert!(matches!(
            closed,
            None | Some(Ok(ClientMessage::Close(_))) | Some(Err(_))
        ));
        assert_eq!(context.connections(), 0);
    }

    #[tokio::test]
    async fn slow_websocket_is_lagged_and_disconnected() {
        let provider = Arc::new(MockProvider::new());
        let (context, shutdown) = context(provider, 2);
        let (address, server, shutdown) = serve(context.clone(), shutdown).await;
        let (mut socket, _) = connect_async(format!("ws://{address}/v1/events"))
            .await
            .unwrap();
        for _ in 0..12 {
            context.publish(
                "blob",
                &serde_json::json!({"payload": "x".repeat(64 * 1024)}),
            );
        }
        let mut lagged = false;
        for _ in 0..20 {
            let Some(message) = tokio::time::timeout(Duration::from_secs(2), socket.next())
                .await
                .ok()
                .flatten()
            else {
                break;
            };
            let Ok(message) = message else {
                break;
            };
            if matches!(message, ClientMessage::Close(_)) {
                break;
            }
            if message
                .to_text()
                .is_ok_and(|text| text.contains("events.lagged"))
            {
                lagged = true;
            }
        }
        assert!(lagged);
        let _ = shutdown.send(true);
        server.await.unwrap();
    }

    #[tokio::test]
    async fn port_conflict_is_a_bind_error() {
        let occupied = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .await
            .unwrap();
        let address = occupied.local_addr().unwrap();
        assert!(tokio::net::TcpListener::bind(address).await.is_err());
    }
}
