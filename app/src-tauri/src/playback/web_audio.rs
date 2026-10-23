//! Browser execution endpoint. Only the current opaque URL is exposed to the main WebView.
//! Audio bodies stream with backpressure; no audio bytes enter IPC or a whole-file buffer.
use crate::model::{AppResult, ResolvedTrack};
use axum::{
    body::Body,
    extract::{Request, State},
    http::{header, StatusCode},
    response::Response,
    Router,
};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use tauri::Emitter;

#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Control {
    pub enabled: bool,
    pub serial: u64,
    pub source: Option<String>,
    pub seek_id: u64,
    pub position_ms: u64,
    pub start_ms: u64,
    pub end_ms: Option<u64>,
    pub playing: bool,
    pub volume: f64,
    pub fades: bool,
    pub crossfade_ms: u32,
    pub device_id: Option<String>,
    pub next: Option<NextSource>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NextSource {
    pub source: String,
    pub start_ms: u64,
    pub end_ms: Option<u64>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Feedback {
    pub serial: u64,
    pub sequence: u64,
    pub kind: FeedbackKind,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub error: Option<String>,
    pub next_source: Option<String>,
}

#[derive(Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum FeedbackKind {
    Playing,
    Paused,
    Buffering,
    Progress,
    Ended,
    Error,
    Stopped,
    Transition,
    TransitionEnd,
    PreloadError,
    DeviceDefault,
}

struct Source {
    token: String,
    resolved: ResolvedTrack,
    cancelled: tokio::sync::watch::Sender<bool>,
}
#[derive(Clone)]
struct StreamState {
    source: Arc<RwLock<Option<Arc<Source>>>>,
    secondary: Arc<RwLock<Option<Arc<Source>>>>,
    client: reqwest::Client,
    requests: Arc<tokio::sync::Semaphore>,
    authority: String,
}

pub struct WebAudio {
    control: Control,
    streams: StreamState,
    server: tauri::async_runtime::JoinHandle<()>,
    pub last_sequence: u64,
    retired_until: Option<std::time::Instant>,
}

impl WebAudio {
    pub fn new() -> AppResult<Self> {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
        listener.set_nonblocking(true).map_err(|e| e.to_string())?;
        let authority = listener
            .local_addr()
            .map_err(|e| e.to_string())?
            .to_string();
        let streams = StreamState {
            source: Default::default(),
            secondary: Default::default(),
            client: reqwest::Client::builder()
                .connect_timeout(std::time::Duration::from_secs(12))
                .read_timeout(std::time::Duration::from_secs(20))
                .pool_max_idle_per_host(1)
                .pool_idle_timeout(std::time::Duration::from_secs(30))
                .build()
                .map_err(|_| "无法初始化浏览器音频传输")?,
            requests: Arc::new(tokio::sync::Semaphore::new(4)),
            authority,
        };
        let router = Router::new().fallback(stream).with_state(streams.clone());
        let server = tauri::async_runtime::spawn(async move {
            match tokio::net::TcpListener::from_std(listener) {
                Ok(listener) => {
                    if let Err(error) = axum::serve(listener, router).await {
                        log::warn!(target: "nons::audio", "音频流服务停止：{error}");
                    }
                }
                Err(error) => log::warn!(target: "nons::audio", "音频流服务不可用：{error}"),
            }
        });
        Ok(Self {
            control: Default::default(),
            streams,
            server,
            last_sequence: 0,
            retired_until: None,
        })
    }

    pub fn control(&self) -> Control {
        self.control.clone()
    }

    pub fn emit(&mut self, app: &tauri::AppHandle) -> AppResult<()> {
        self.control.serial += 1;
        self.last_sequence = 0;
        app.emit_to("main", "web-audio-control", &self.control)
            .map_err(|e| e.to_string())
    }

    pub fn configure(&mut self, state: &crate::model::PlayerSnapshot) {
        self.control.enabled = state.playback_engine == crate::model::PlaybackEngine::WebAudio;
        self.control.fades = state.playback_options.fades;
        self.control.crossfade_ms = if state.playback_options.crossfade {
            state.playback_options.crossfade_duration_ms
        } else {
            0
        };
        self.control.device_id = state.device_id.clone();
    }

    fn clear_secondary(&mut self) -> AppResult<()> {
        if let Some(source) = self
            .streams
            .secondary
            .write()
            .map_err(|_| "音频资源状态不可用")?
            .take()
        {
            let _ = source.cancelled.send(true);
        }
        self.control.next = None;
        self.retired_until = None;
        Ok(())
    }

    pub fn cancel_preload(&mut self, app: &tauri::AppHandle) -> AppResult<()> {
        self.clear_secondary()?;
        self.emit(app)
    }

    pub fn prepare(&mut self, app: &tauri::AppHandle, resolved: ResolvedTrack) -> AppResult<()> {
        self.clear_secondary()?;
        let token = uuid::Uuid::new_v4().to_string();
        self.control.next = Some(NextSource {
            source: format!("http://{}/{}", self.streams.authority, token),
            start_ms: resolved.track.cue().map_or(0, |c| c.start_ms),
            end_ms: resolved.track.cue().map(|c| c.end_ms),
        });
        let (cancelled, _) = tokio::sync::watch::channel(false);
        *self
            .streams
            .secondary
            .write()
            .map_err(|_| "音频资源状态不可用")? = Some(Arc::new(Source {
            token,
            resolved,
            cancelled,
        }));
        self.emit(app)
    }

    pub fn promote(&mut self, app: &tauri::AppHandle, position_ms: u64) -> AppResult<()> {
        let next = self.control.next.take().ok_or("下一首音频尚未准备")?;
        let mut current = self
            .streams
            .source
            .write()
            .map_err(|_| "音频资源状态不可用")?;
        let mut secondary = self
            .streams
            .secondary
            .write()
            .map_err(|_| "音频资源状态不可用")?;
        std::mem::swap(&mut *current, &mut *secondary);
        drop(current);
        drop(secondary);
        self.control.source = Some(next.source);
        self.control.start_ms = next.start_ms;
        self.control.end_ms = next.end_ms;
        self.control.position_ms = position_ms;
        self.control.seek_id += 1;
        self.retired_until = Some(
            std::time::Instant::now()
                + std::time::Duration::from_millis(u64::from(self.control.crossfade_ms) + 1000),
        );
        self.emit(app)
    }

    pub fn retire(&mut self) -> AppResult<()> {
        if self.retired_until.is_some() {
            self.clear_secondary()?;
        }
        Ok(())
    }

    pub fn ready_to_prepare(&mut self) -> bool {
        if self
            .retired_until
            .is_some_and(|end| std::time::Instant::now() >= end)
        {
            let _ = self.clear_secondary();
        }
        self.retired_until.is_none() && self.control.next.is_none()
    }

    pub fn deactivate(&mut self, app: &tauri::AppHandle) -> AppResult<()> {
        self.control.enabled = false;
        self.stop(app)
    }

    pub fn stop(&mut self, app: &tauri::AppHandle) -> AppResult<()> {
        self.clear_secondary()?;
        if let Some(source) = self
            .streams
            .source
            .write()
            .map_err(|_| "音频资源状态不可用")?
            .take()
        {
            let _ = source.cancelled.send(true);
        }
        self.control.source = None;
        self.control.playing = false;
        self.emit(app)
    }

    pub fn load(
        &mut self,
        app: &tauri::AppHandle,
        resolved: ResolvedTrack,
        position_ms: u64,
        playing: bool,
        volume: f64,
    ) -> AppResult<()> {
        self.clear_secondary()?;
        if let Some(previous) = self
            .streams
            .source
            .write()
            .map_err(|_| "音频资源状态不可用")?
            .take()
        {
            let _ = previous.cancelled.send(true);
        }
        let token = uuid::Uuid::new_v4().to_string();
        let cue = resolved.track.cue();
        self.control.start_ms = cue.map_or(0, |c| c.start_ms);
        self.control.end_ms = cue.map(|c| c.end_ms);
        self.control.source = Some(format!("http://{}/{}", self.streams.authority, token));
        self.control.position_ms = position_ms;
        self.control.seek_id += 1;
        self.control.playing = playing;
        self.control.volume = volume;
        let (cancelled, _) = tokio::sync::watch::channel(false);
        *self
            .streams
            .source
            .write()
            .map_err(|_| "音频资源状态不可用")? = Some(Arc::new(Source {
            token,
            resolved,
            cancelled,
        }));
        self.emit(app)
    }

    pub fn intent(
        &mut self,
        app: &tauri::AppHandle,
        playing: bool,
        volume: f64,
        seek: Option<u64>,
    ) -> AppResult<()> {
        self.control.playing = playing;
        self.control.volume = volume;
        if let Some(position) = seek {
            self.control.position_ms = position;
            self.control.seek_id += 1;
        }
        self.emit(app)
    }

    pub fn accepts(&mut self, feedback: &Feedback) -> bool {
        if feedback.serial != self.control.serial || feedback.sequence <= self.last_sequence {
            return false;
        }
        self.last_sequence = feedback.sequence;
        true
    }
}

impl Drop for WebAudio {
    fn drop(&mut self) {
        let _ = self.clear_secondary();
        if let Ok(source) = self.streams.source.read() {
            if let Some(source) = source.as_ref() {
                let _ = source.cancelled.send(true);
            }
        }
        self.server.abort();
    }
}

async fn stream(
    State(state): State<StreamState>,
    request: Request,
) -> Result<Response, StatusCode> {
    // Exact Host prevents DNS rebinding. An unguessable per-track path grants only
    // the active audio resource, never a caller-supplied path, URL or account header.
    if request
        .headers()
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        != Some(&state.authority)
    {
        return Err(StatusCode::FORBIDDEN);
    }
    if !matches!(
        *request.method(),
        axum::http::Method::GET | axum::http::Method::HEAD
    ) {
        return Err(StatusCode::METHOD_NOT_ALLOWED);
    }
    let source = state
        .source
        .read()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .clone();
    let source = source
        .filter(|s| request.uri().path().strip_prefix('/') == Some(s.token.as_str()))
        .or_else(|| {
            state
                .secondary
                .read()
                .ok()?
                .clone()
                .filter(|s| request.uri().path().strip_prefix('/') == Some(s.token.as_str()))
        })
        .ok_or(StatusCode::NOT_FOUND)?;
    let permit = state
        .requests
        .clone()
        .try_acquire_owned()
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let mut cancelled = source.cancelled.subscribe();
    let uri = url::Url::parse(&source.resolved.uri).map_err(|_| StatusCode::BAD_GATEWAY)?;
    let mut response = if uri.scheme() == "file" {
        let path = uri.to_file_path().map_err(|_| StatusCode::BAD_GATEWAY)?;
        tower_http::services::ServeFile::new(path)
            .try_call(request)
            .await
            .map_err(|_| StatusCode::NOT_FOUND)?
            .map(Body::new)
    } else if matches!(uri.scheme(), "http" | "https") {
        let mut upstream = state
            .client
            .request(request.method().clone(), uri)
            .header(header::ACCEPT_ENCODING, "identity");
        for name in [header::RANGE, header::IF_RANGE] {
            if let Some(value) = request.headers().get(&name) {
                upstream = upstream.header(name, value);
            }
        }
        let upstream = tokio::select! {
            _ = cancelled.changed() => return Err(StatusCode::GONE),
            result = upstream.send() => result.map_err(|_| StatusCode::BAD_GATEWAY)?,
        };
        let mut response = Response::builder().status(upstream.status());
        for name in [
            header::CONTENT_TYPE,
            header::CONTENT_LENGTH,
            header::CONTENT_RANGE,
            header::ACCEPT_RANGES,
        ] {
            if let Some(value) = upstream.headers().get(&name) {
                response = response.header(name, value);
            }
        }
        response
            .body(Body::from_stream(upstream.bytes_stream()))
            .map_err(|_| StatusCode::BAD_GATEWAY)?
    } else {
        return Err(StatusCode::BAD_GATEWAY);
    };
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, "no-store".parse().unwrap());
    response
        .headers_mut()
        .insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*".parse().unwrap());
    let (parts, body) = response.into_parts();
    // Hold both the permit and decoded-file ownership for the body lifetime.
    let stream = futures_util::stream::unfold(
        (body.into_data_stream(), cancelled, permit, source),
        |(mut body, mut cancelled, permit, source)| async move {
            if *cancelled.borrow() {
                return None;
            }
            tokio::select! {
                _ = cancelled.changed() => None,
                item = body.next() => item.map(|item| (item, (body, cancelled, permit, source))),
            }
        },
    );
    Ok(Response::from_parts(parts, Body::from_stream(stream)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolved(uri: String, file: Option<Arc<tempfile::NamedTempFile>>) -> ResolvedTrack {
        ResolvedTrack {
            uri,
            decoded_audio: file,
            quality: None,
            track: crate::model::Track {
                key: "local:test".into(),
                title: "test".into(),
                aliases: vec![],
                artist: String::new(),
                artists: vec![],
                album: String::new(),
                album_id: None,
                duration_ms: 0,
                cover: String::new().into(),
                source: crate::model::TrackSource::Local {
                    path: "test".into(),
                    netease_id: None,
                    cue: None,
                },
            },
        }
    }

    #[tokio::test]
    async fn opaque_stream_serves_local_ranges_and_rejects_revoked_handles() {
        let audio = WebAudio::new().unwrap();
        let file = Arc::new(tempfile::NamedTempFile::new().unwrap());
        std::fs::write(file.path(), b"0123456789").unwrap();
        let (cancelled, _) = tokio::sync::watch::channel(false);
        *audio.streams.source.write().unwrap() = Some(Arc::new(Source {
            token: "only-current".into(),
            resolved: resolved(
                url::Url::from_file_path(file.path()).unwrap().into(),
                Some(file),
            ),
            cancelled,
        }));
        let client = reqwest::Client::new();
        let url = format!("http://{}/only-current", audio.streams.authority);
        let response = client
            .get(&url)
            .header("Range", "bytes=2-5")
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(response.headers()["content-range"], "bytes 2-5/10");
        assert_eq!(response.headers()["cache-control"], "no-store");
        assert_eq!(response.bytes().await.unwrap().as_ref(), b"2345");
        let response = client
            .get(&url)
            .header("Host", "evil.example")
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        *audio.streams.source.write().unwrap() = None;
        assert_eq!(
            client.get(&url).send().await.unwrap().status(),
            StatusCode::NOT_FOUND
        );
    }

    #[test]
    fn feedback_rejects_previous_intents_and_out_of_order_reports() {
        let mut audio = WebAudio::new().unwrap();
        audio.control.serial = 4;
        let mut feedback = Feedback {
            serial: 3,
            sequence: 100,
            kind: FeedbackKind::Ended,
            position_ms: 0,
            duration_ms: 0,
            error: None,
            next_source: None,
        };
        assert!(!audio.accepts(&feedback));
        feedback.serial = 4;
        feedback.sequence = 2;
        assert!(audio.accepts(&feedback));
        assert!(!audio.accepts(&feedback));
        feedback.sequence = 1;
        assert!(!audio.accepts(&feedback));
    }

    #[tokio::test]
    async fn cancelling_preload_revokes_only_the_secondary_resource() {
        let mut audio = WebAudio::new().unwrap();
        let file = Arc::new(tempfile::NamedTempFile::new().unwrap());
        std::fs::write(file.path(), b"audio").unwrap();
        let uri = url::Url::from_file_path(file.path()).unwrap().to_string();
        for (slot, token) in [
            (&audio.streams.source, "current"),
            (&audio.streams.secondary, "next"),
        ] {
            let (cancelled, _) = tokio::sync::watch::channel(false);
            *slot.write().unwrap() = Some(Arc::new(Source {
                token: token.into(),
                resolved: resolved(uri.clone(), Some(file.clone())),
                cancelled,
            }));
        }
        audio.clear_secondary().unwrap();
        for (token, available) in [("current", true), ("next", false)] {
            let request = Request::builder()
                .uri(format!("/{token}"))
                .header(header::HOST, &audio.streams.authority)
                .body(Body::empty())
                .unwrap();
            let response = stream(State(audio.streams.clone()), request).await;
            if available {
                let response = response.unwrap();
                assert_eq!(response.status(), StatusCode::OK);
                assert_eq!(response.headers()[header::ACCESS_CONTROL_ALLOW_ORIGIN], "*");
            } else {
                assert_eq!(response.err(), Some(StatusCode::NOT_FOUND));
            }
        }
    }

    #[tokio::test]
    async fn online_stream_forwards_ranges_without_disclosing_the_upstream_location() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let upstream = tauri::async_runtime::spawn(async move {
            axum::serve(
                listener,
                Router::new().fallback(|request: Request| async move {
                    assert_eq!(request.headers()["range"], "bytes=4-7");
                    assert_eq!(request.headers()["accept-encoding"], "identity");
                    Response::builder()
                        .status(206)
                        .header("content-range", "bytes 4-7/10")
                        .header("content-type", "audio/mpeg")
                        .header("content-length", "4")
                        .header("location", "https://private.example/resource?secret=hidden")
                        .body(Body::from("4567"))
                        .unwrap()
                }),
            )
            .await
            .unwrap();
        });
        let audio = WebAudio::new().unwrap();
        let (cancelled, _) = tokio::sync::watch::channel(false);
        *audio.streams.source.write().unwrap() = Some(Arc::new(Source {
            token: "online".into(),
            resolved: resolved(format!("http://{address}/private?secret=hidden"), None),
            cancelled,
        }));
        let response = reqwest::Client::new()
            .get(format!("http://{}/online", audio.streams.authority))
            .header("Range", "bytes=4-7")
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(response.headers()["content-range"], "bytes 4-7/10");
        assert!(response.headers().get("location").is_none());
        assert_eq!(response.bytes().await.unwrap().as_ref(), b"4567");
        upstream.abort();
    }
}
