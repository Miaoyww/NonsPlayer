use crate::{model::AppResult, music::manager::ResourceTicket};
use axum::{
    body::Body,
    extract::{Request, State},
    http::{header, HeaderValue, StatusCode},
    response::Response,
    Router,
};
use futures_util::StreamExt;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

struct Entry {
    ticket: Arc<ResourceTicket>,
    generation: u64,
    expires: Instant,
}
#[derive(Clone)]
struct StateData {
    entries: Arc<Mutex<BTreeMap<String, Entry>>>,
    client: reqwest::Client,
    requests: Arc<tokio::sync::Semaphore>,
    authority: String,
}

pub struct MvProxy {
    data: StateData,
    server: tauri::async_runtime::JoinHandle<()>,
}
impl MvProxy {
    pub fn new() -> AppResult<Self> {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
        listener.set_nonblocking(true).map_err(|e| e.to_string())?;
        let authority = listener
            .local_addr()
            .map_err(|e| e.to_string())?
            .to_string();
        let data = StateData {
            entries: Default::default(),
            client: reqwest::Client::builder()
                .connect_timeout(Duration::from_secs(12))
                .read_timeout(Duration::from_secs(20))
                .pool_max_idle_per_host(1)
                .pool_idle_timeout(Duration::from_secs(30))
                .build()
                .map_err(|_| "无法初始化 MV 传输")?,
            requests: Arc::new(tokio::sync::Semaphore::new(4)),
            authority,
        };
        let router = Router::new().fallback(stream).with_state(data.clone());
        let server = tauri::async_runtime::spawn(async move {
            match tokio::net::TcpListener::from_std(listener) {
                Ok(listener) => {
                    if let Err(error) = axum::serve(listener, router).await {
                        log::warn!(target: "nons::mv", "MV 流服务停止：{error}");
                    }
                }
                Err(error) => log::warn!(target: "nons::mv", "MV 流服务不可用：{error}"),
            }
        });
        Ok(Self { data, server })
    }

    pub fn issue(&self, ticket: Arc<ResourceTicket>, generation: u64) -> AppResult<String> {
        let mut entries = self.data.entries.lock().map_err(|_| "MV 资源状态不可用")?;
        let now = Instant::now();
        entries.retain(|_, entry| entry.expires > now);
        if entries.len() >= 8 {
            return Err("MV 资源过多，请稍后重试".into());
        }
        let token = uuid::Uuid::new_v4().to_string();
        entries.insert(
            token.clone(),
            Entry {
                ticket,
                generation,
                expires: now + Duration::from_secs(2 * 60 * 60),
            },
        );
        Ok(format!("http://{}/{token}", self.data.authority))
    }
}
impl Drop for MvProxy {
    fn drop(&mut self) {
        self.server.abort();
    }
}

fn can_retry(status: StatusCode) -> bool {
    matches!(status.as_u16(), 403 | 404 | 408 | 410 | 429 | 500..=599)
}

fn range_start(range: Option<&HeaderValue>) -> Option<u64> {
    range?
        .to_str()
        .ok()?
        .strip_prefix("bytes=")?
        .split('-')
        .next()?
        .parse()
        .ok()
}

async fn open(
    state: &StateData,
    entry: &Entry,
    url: &str,
    method: &axum::http::Method,
    range: Option<&HeaderValue>,
    if_range: Option<&HeaderValue>,
    offset: u64,
) -> Result<reqwest::Response, StatusCode> {
    let mut upstream = state
        .client
        .request(method.clone(), url)
        .header(header::ACCEPT_ENCODING, "identity");
    for (name, value) in entry.ticket.headers() {
        if let (Ok(name), Ok(value)) = (
            header::HeaderName::try_from(name),
            header::HeaderValue::try_from(value),
        ) {
            upstream = upstream.header(name, value);
        }
    }
    if offset > 0 {
        let end = range
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("bytes="))
            .and_then(|value| value.split_once('-'))
            .and_then(|(_, end)| (!end.is_empty()).then_some(end))
            .map(|end| format!("bytes={offset}-{end}"))
            .unwrap_or_else(|| format!("bytes={offset}-"));
        upstream = upstream.header(header::RANGE, end);
    } else if let Some(value) = range {
        upstream = upstream.header(header::RANGE, value);
    }
    if let Some(value) = if_range {
        upstream = upstream.header(header::IF_RANGE, value);
    }
    upstream.send().await.map_err(|_| StatusCode::BAD_GATEWAY)
}

async fn stream(State(state): State<StateData>, request: Request) -> Result<Response, StatusCode> {
    if request
        .headers()
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        != Some(state.authority.as_str())
    {
        return Err(StatusCode::FORBIDDEN);
    }
    if !matches!(
        *request.method(),
        axum::http::Method::GET | axum::http::Method::HEAD
    ) {
        return Err(StatusCode::METHOD_NOT_ALLOWED);
    }
    let token = request
        .uri()
        .path()
        .strip_prefix('/')
        .ok_or(StatusCode::NOT_FOUND)?;
    let entry = {
        let entries = state
            .entries
            .lock()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let entry = entries.get(token).ok_or(StatusCode::NOT_FOUND)?;
        if entry.expires <= Instant::now() {
            return Err(StatusCode::GONE);
        }
        Entry {
            ticket: entry.ticket.clone(),
            generation: entry.generation,
            expires: entry.expires,
        }
    };
    entry
        .ticket
        .check(entry.generation)
        .map_err(|_| StatusCode::GONE)?;
    let permit = state
        .requests
        .clone()
        .try_acquire_owned()
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let urls = entry
        .ticket
        .urls()
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let method = request.method().clone();
    let range = request.headers().get(header::RANGE).cloned();
    let if_range = request.headers().get(header::IF_RANGE).cloned();
    let start = range_start(range.as_ref()).unwrap_or(0);
    let mut index = 0;
    let upstream = loop {
        let response = open(
            &state,
            &entry,
            &urls[index],
            &method,
            range.as_ref(),
            if_range.as_ref(),
            0,
        )
        .await?;
        if can_retry(response.status()) && index + 1 < urls.len() {
            index += 1;
            continue;
        }
        break response;
    };
    let status = upstream.status();
    let mut response = Response::builder().status(status);
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
    response = response
        .header(header::CACHE_CONTROL, "no-store")
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*");
    if method == axum::http::Method::HEAD {
        return response
            .body(Body::empty())
            .map_err(|_| StatusCode::BAD_GATEWAY);
    }
    let body = futures_util::stream::unfold(
        (
            upstream.bytes_stream(),
            entry,
            index,
            start,
            0_u64,
            state,
            method,
            range,
            if_range,
            permit,
        ),
        |(mut body, entry, mut index, start, mut read, state, method, range, if_range, permit)| async move {
            loop {
                match body.next().await {
                    Some(Ok(chunk)) => {
                        read += chunk.len() as u64;
                        return Some((
                            Ok::<_, std::io::Error>(chunk),
                            (
                                body, entry, index, start, read, state, method, range, if_range,
                                permit,
                            ),
                        ));
                    }
                    Some(Err(_)) if index + 1 < entry.ticket.urls().len() => {
                        index += 1;
                        let url = entry.ticket.urls()[index].to_owned();
                        match open(
                            &state,
                            &entry,
                            &url,
                            &method,
                            range.as_ref(),
                            if_range.as_ref(),
                            start + read,
                        )
                        .await
                        {
                            Ok(next) => body = next.bytes_stream(),
                            Err(_) if index + 1 < entry.ticket.urls().len() => continue,
                            Err(_) => return None,
                        }
                    }
                    Some(Err(error)) => {
                        return Some((
                            Err(std::io::Error::other(error)),
                            (
                                body, entry, index, start, read, state, method, range, if_range,
                                permit,
                            ),
                        ))
                    }
                    None => return None,
                }
            }
        },
    );
    response
        .body(Body::from_stream(body))
        .map_err(|_| StatusCode::BAD_GATEWAY)
}

#[allow(dead_code)]
fn _header(value: &str) -> HeaderValue {
    HeaderValue::from_str(value).unwrap_or(HeaderValue::from_static(""))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_statuses_match_cdn_transport_failures() {
        for code in [403, 404, 408, 410, 429, 500, 599] {
            assert!(can_retry(StatusCode::from_u16(code).unwrap()));
        }
        for code in [200, 206, 301, 400] {
            assert!(!can_retry(StatusCode::from_u16(code).unwrap()));
        }
    }

    #[tokio::test]
    async fn proxy_falls_back_before_body_and_keeps_the_requested_range() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tauri::async_runtime::spawn(async move {
            axum::serve(
                listener,
                Router::new().fallback(|request: Request| async move {
                    if request.uri().path() == "/bad" {
                        return Response::builder()
                            .status(StatusCode::FORBIDDEN)
                            .body(Body::empty())
                            .unwrap();
                    }
                    assert_eq!(request.headers()[header::RANGE], "bytes=2-5");
                    Response::builder()
                        .status(StatusCode::PARTIAL_CONTENT)
                        .header(header::CONTENT_RANGE, "bytes 2-5/10")
                        .header(header::CONTENT_TYPE, "video/mp4")
                        .body(Body::from("2345"))
                        .unwrap()
                }),
            )
            .await
            .unwrap();
        });
        let proxy = MvProxy::new().unwrap();
        let ticket = ResourceTicket::for_audio_test_fallback(
            format!("http://{address}/bad"),
            vec![format!("http://{address}/good")],
        );
        let url = proxy.issue(ticket, 0).unwrap();
        let response = reqwest::Client::new()
            .get(url)
            .header(header::RANGE, "bytes=2-5")
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(response.bytes().await.unwrap().as_ref(), b"2345");
        server.abort();
    }
}
