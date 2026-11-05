use crate::model::AppResult;
use hyper_util::client::proxy::matcher::Matcher;
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    future::Future,
    net::SocketAddr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

pub struct Http {
    direct: reqwest::Client,
    direct_no_redirect: reqwest::Client,
    system: reqwest::Client,
    system_no_redirect: reqwest::Client,
    proxies: Matcher,
}

pub(crate) async fn resolve_addresses(host: &str, port: u16) -> AppResult<Vec<SocketAddr>> {
    let addresses: Vec<_> = tokio::net::lookup_host((host, port))
        .await
        .inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("http.dns.resolve", error);
        })
        .map_err(|_| "网络目标解析失败")?
        .collect();
    if addresses.is_empty() {
        return Err("网络目标地址数量无效".into());
    }
    Ok(addresses)
}
struct Resolver;
impl reqwest::dns::Resolve for Resolver {
    fn resolve(&self, name: reqwest::dns::Name) -> reqwest::dns::Resolving {
        Box::pin(async move {
            let addresses = resolve_addresses(name.as_str(), 0)
                .await
                .map_err(std::io::Error::other)?;
            Ok(Box::new(addresses.into_iter()) as reqwest::dns::Addrs)
        })
    }
}

#[derive(Clone, Debug)]
pub(crate) struct BinaryRequest {
    pub url: String,
    pub method: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub follow_redirects: bool,
    pub timeout_ms: u32,
}
#[derive(Debug)]
pub(crate) struct BinaryResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub url: String,
}

pub(crate) fn checked_url(input: &str) -> AppResult<url::Url> {
    let url = url::Url::parse(input)
        .inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("http.url.parse", error);
        })
        .map_err(|_| "HTTP URL 无效")?;
    if !matches!(url.scheme(), "http" | "https") || url.host().is_none() {
        return Err("HTTP 地址不是有效网络地址".into());
    }
    Ok(url)
}

fn checked_method(input: &str) -> AppResult<reqwest::Method> {
    reqwest::Method::from_bytes(input.as_bytes())
        .inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("http.method.parse", error);
        })
        .map_err(|_| "HTTP 方法无效".into())
}

fn apply_headers<N, V, I>(
    mut builder: reqwest::RequestBuilder,
    headers: I,
    operation: &str,
) -> AppResult<reqwest::RequestBuilder>
where
    N: AsRef<str>,
    V: AsRef<str>,
    I: IntoIterator<Item = (N, V)>,
{
    for (name, value) in headers {
        let name = reqwest::header::HeaderName::from_bytes(name.as_ref().as_bytes())
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn(
                    &format!("{operation}.header_name"),
                    error,
                );
            })
            .map_err(|_| "HTTP 请求头无效")?;
        let value = reqwest::header::HeaderValue::from_str(value.as_ref())
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn(
                    &format!("{operation}.header_value"),
                    error,
                );
            })
            .map_err(|_| "HTTP 请求头无效")?;
        builder = builder.header(name, value);
    }
    Ok(builder)
}

async fn wait_inactive(active: &AtomicBool) {
    while active.load(Ordering::Acquire) {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

async fn guard<F, T>(
    operation: F,
    timeout_ms: Option<u64>,
    active: &AtomicBool,
    cancelled: &'static str,
    timeout_operation: &'static str,
) -> AppResult<T>
where
    F: Future<Output = AppResult<T>>,
{
    let operation = async {
        match timeout_ms.filter(|timeout| *timeout > 0) {
            Some(timeout) => {
                match tokio::time::timeout(Duration::from_millis(timeout), operation).await {
                    Ok(result) => result,
                    Err(error) => {
                        crate::infrastructure::diagnostics::warn(timeout_operation, error);
                        Err("HTTP 请求超时".into())
                    }
                }
            }
            None => operation.await,
        }
    };
    tokio::select! {
        result = operation => result,
        _ = wait_inactive(active) => Err(cancelled.into()),
    }
}

async fn read_body(response: &mut reqwest::Response, max_bytes: Option<u64>) -> AppResult<Vec<u8>> {
    if max_bytes.is_some_and(|max| response.content_length().is_some_and(|length| length > max)) {
        crate::infrastructure::diagnostics::warn(
            "http.body_content_length",
            "response exceeds explicit maxBytes",
        );
        return Err("HTTP 响应超过 maxBytes".into());
    }
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .inspect_err(|error| {
            crate::infrastructure::diagnostics::warn_error("http.read", error);
        })
        .map_err(|_| "HTTP 读取失败")?
    {
        if max_bytes.is_some_and(|max| body.len() as u64 + chunk.len() as u64 > max) {
            crate::infrastructure::diagnostics::warn(
                "http.body_read",
                "response exceeds explicit maxBytes",
            );
            return Err("HTTP 响应超过 maxBytes".into());
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Request {
    url: String,
    #[serde(default)]
    method: Option<String>,
    #[serde(default)]
    timeout_ms: Option<u64>,
    #[serde(default)]
    max_bytes: Option<u64>,
    #[serde(default)]
    response_type: Option<String>,
    #[serde(default = "default_true")]
    follow_redirects: bool,
    #[serde(default)]
    headers: BTreeMap<String, String>,
    #[serde(default)]
    body: Option<String>,
}
fn default_true() -> bool {
    true
}

impl Http {
    pub fn new() -> AppResult<Self> {
        let system = Self::builder().build().map_err(|e| e.to_string())?;
        let system_no_redirect = Self::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|e| e.to_string())?;
        Self::with_system(system, system_no_redirect, Matcher::from_system())
    }
    fn builder() -> reqwest::ClientBuilder {
        reqwest::Client::builder().redirect(redirect_policy())
    }
    fn with_system(
        system: reqwest::Client,
        system_no_redirect: reqwest::Client,
        proxies: Matcher,
    ) -> AppResult<Self> {
        // Only system bypass rules use this client; all clients use the same loop policy.
        let direct = Self::builder()
            .no_proxy()
            .dns_resolver(Arc::new(Resolver))
            .build()
            .map_err(|e| e.to_string())?;
        let direct_no_redirect = Self::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .dns_resolver(Arc::new(Resolver))
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Self {
            direct,
            direct_no_redirect,
            system,
            system_no_redirect,
            proxies,
        })
    }
    fn uses_proxy(&self, url: &url::Url) -> bool {
        url[..url::Position::BeforePath]
            .parse::<http::Uri>()
            .ok()
            .is_some_and(|uri| self.proxies.intercept(&uri).is_some())
    }
    pub(crate) fn client(&self, url: &url::Url) -> &reqwest::Client {
        if self.uses_proxy(url) {
            &self.system
        } else {
            &self.direct
        }
    }
    pub(crate) fn client_with_redirects(
        &self,
        url: &url::Url,
        follow_redirects: bool,
    ) -> &reqwest::Client {
        match (self.uses_proxy(url), follow_redirects) {
            (true, true) => &self.system,
            (true, false) => &self.system_no_redirect,
            (false, true) => &self.direct,
            (false, false) => &self.direct_no_redirect,
        }
    }
    #[cfg(test)]
    pub(crate) fn for_test(client: reqwest::Client) -> Self {
        Self::for_test_with_redirect(client.clone(), client)
    }
    #[cfg(test)]
    pub(crate) fn for_test_with_redirect(
        client: reqwest::Client,
        no_redirect: reqwest::Client,
    ) -> Self {
        Self {
            system: client.clone(),
            system_no_redirect: no_redirect.clone(),
            direct: client,
            direct_no_redirect: no_redirect,
            proxies: Matcher::builder().build(),
        }
    }
    pub(crate) async fn binary_request(
        &self,
        request: BinaryRequest,
        active: &AtomicBool,
    ) -> AppResult<BinaryResponse> {
        if !active.load(Ordering::Acquire) {
            return Err("调用上下文已失效".into());
        }
        let url = checked_url(&request.url)?;
        let method = checked_method(&request.method)?;
        let builder = self
            .client_with_redirects(&url, request.follow_redirects)
            .request(method.clone(), url);
        let builder = apply_headers(builder, request.headers, "http.binary")?;
        let builder = if request.body.is_empty() {
            builder
        } else {
            builder.body(request.body)
        };
        let operation = async {
            let mut response = builder
                .send()
                .await
                .inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn_error("http.send", error);
                })
                .map_err(|_| "HTTP 请求失败")?;
            let status = response.status().as_u16();
            let url = response.url().to_string();
            let mut headers = Vec::new();
            for (name, value) in response.headers() {
                let value = value
                    .to_str()
                    .inspect_err(|error| {
                        crate::infrastructure::diagnostics::warn(
                            "http.binary.response_header",
                            error,
                        );
                    })
                    .map_err(|_| "HTTP 响应头不是文本")?;
                headers.push((name.to_string(), value.to_owned()));
            }
            let body = if method == reqwest::Method::HEAD {
                Vec::new()
            } else {
                read_body(&mut response, None).await?
            };
            Ok(BinaryResponse {
                status,
                headers,
                body,
                url,
            })
        };
        guard(
            operation,
            (request.timeout_ms > 0).then_some(request.timeout_ms as u64),
            active,
            "调用上下文已失效",
            "http.binary.timeout",
        )
        .await
    }
    pub async fn request(
        &self,
        args: &Value,
        active: &std::sync::atomic::AtomicBool,
    ) -> AppResult<Value> {
        let request: Request = serde_json::from_value(args.clone())
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("http.request.decode", error);
            })
            .map_err(|_| "HTTP 请求参数无效")?;
        let url = checked_url(&request.url)?;
        let method = checked_method(request.method.as_deref().unwrap_or("GET"))?;
        let read_text = match request.response_type.as_deref().unwrap_or("text") {
            "text" => true,
            "none" => false,
            _ => return Err("HTTP 响应类型无效".into()),
        };
        if request.max_bytes == Some(0) {
            return Err("maxBytes 必须为正整数".into());
        }
        let mut builder = self
            .client_with_redirects(&url, request.follow_redirects)
            .request(method.clone(), url);
        builder = apply_headers(
            builder,
            request.headers.iter().map(|(name, value)| (name, value)),
            "http.request",
        )?;
        if let Some(body) = request.body {
            builder = builder.body(body);
        }
        let operation = async {
            let mut response = builder
                .send()
                .await
                .inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn_error("http.send", error);
                })
                .map_err(|_| "HTTP 请求失败")?;
            let status = response.status().as_u16();
            let final_url = response.url().as_str().to_owned();
            let cookies: Vec<String> = response
                .headers()
                .get_all("set-cookie")
                .iter()
                .filter_map(|value| {
                    value
                        .to_str()
                        .inspect_err(|error| {
                            crate::infrastructure::diagnostics::warn(
                                "http.response.cookie_header",
                                error,
                            );
                        })
                        .ok()
                        .map(str::to_string)
                })
                .collect();
            let mut headers = BTreeMap::new();
            for (name, value) in response.headers() {
                let value = value
                    .to_str()
                    .inspect_err(|error| {
                        crate::infrastructure::diagnostics::warn(
                            "http.response.header_text",
                            error,
                        );
                    })
                    .map_err(|_| "HTTP 响应头不是文本")?;
                headers.insert(name.as_str().to_string(), value.to_string());
            }
            let bytes = if read_text && method != reqwest::Method::HEAD {
                read_body(&mut response, request.max_bytes).await?
            } else {
                Vec::new()
            };
            let body = String::from_utf8(bytes)
                .inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn("http.response.decode_utf8", error);
                })
                .map_err(|_| "HTTP 响应不是 UTF-8 文本")?;
            Ok(
                json!({"status":status,"headers":headers,"body":body,"cookies":cookies,"finalUrl":final_url}),
            )
        };
        guard(
            operation,
            request.timeout_ms,
            active,
            "插件已卸载或禁用",
            "http.request.timeout",
        )
        .await
    }
}

fn redirect_policy() -> reqwest::redirect::Policy {
    reqwest::redirect::Policy::custom(|attempt| {
        if attempt
            .previous()
            .iter()
            .any(|previous| previous == attempt.url())
        {
            let target = attempt.url();
            crate::infrastructure::diagnostics::warn(
                "http.redirect.loop",
                format!(
                    "redirect target repeated: {}:{}{}",
                    target.host_str().unwrap_or("-"),
                    target.port_or_known_default().unwrap_or_default(),
                    target.path()
                ),
            );
            attempt.error("HTTP redirect loop")
        } else {
            attempt.follow()
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc, Mutex,
        },
    };

    fn fixture(response: String, delay: Duration) -> (Http, std::thread::JoinHandle<()>) {
        let (http, server, _) = capture_fixture(response, delay);
        (http, server)
    }
    fn capture_fixture(
        response: String,
        delay: Duration,
    ) -> (Http, std::thread::JoinHandle<()>, Arc<Mutex<Vec<u8>>>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let captured = Arc::new(Mutex::new(Vec::new()));
        let request_bytes = captured.clone();
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut bytes = Vec::new();
            let header_end = loop {
                let mut chunk = [0; 1024];
                let read = socket.read(&mut chunk).unwrap();
                assert!(read > 0);
                bytes.extend_from_slice(&chunk[..read]);
                if let Some(end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
                    break end + 4;
                }
            };
            let length = String::from_utf8_lossy(&bytes[..header_end])
                .lines()
                .find_map(|line| {
                    line.to_ascii_lowercase()
                        .strip_prefix("content-length:")
                        .and_then(|value| value.trim().parse::<usize>().ok())
                })
                .unwrap_or(0);
            while bytes.len() < header_end + length {
                let mut chunk = [0; 1024];
                let read = socket.read(&mut chunk).unwrap();
                assert!(read > 0);
                bytes.extend_from_slice(&chunk[..read]);
            }
            *request_bytes.lock().unwrap() = bytes;
            std::thread::sleep(delay);
            let _ = socket.write_all(response.as_bytes());
        });
        let client = reqwest::Client::builder()
            .no_proxy()
            .resolve("example.org", address)
            .build()
            .unwrap();
        let no_redirect = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .resolve("example.org", address)
            .build()
            .unwrap();
        (
            Http::for_test_with_redirect(client, no_redirect),
            server,
            captured,
        )
    }
    fn read_request(socket: &mut std::net::TcpStream) -> Vec<u8> {
        let mut bytes = Vec::new();
        loop {
            let mut chunk = [0; 1024];
            let read = socket.read(&mut chunk).unwrap();
            assert!(read > 0);
            bytes.extend_from_slice(&chunk[..read]);
            if bytes.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }
        bytes
    }
    fn redirect_fixture(body: String) -> (Http, String, std::thread::JoinHandle<()>) {
        let start = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = start.local_addr().unwrap();
        let target = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let final_url = format!("http://{}/final", target.local_addr().unwrap());
        let redirect_url = final_url.clone();
        let server = std::thread::spawn(move || {
            let (mut socket, _) = start.accept().unwrap();
            read_request(&mut socket);
            let _ = write!(
                socket,
                "HTTP/1.1 302 Found\r\nLocation: {redirect_url}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            );
            let (mut socket, _) = target.accept().unwrap();
            read_request(&mut socket);
            let _ = write!(
                socket,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
        });
        let client = reqwest::Client::builder()
            .no_proxy()
            .resolve("example.org", address)
            .build()
            .unwrap();
        (Http::for_test(client), final_url, server)
    }
    fn redirect_client(address: std::net::SocketAddr) -> Http {
        let client = Http::builder()
            .no_proxy()
            .resolve("example.org", address)
            .resolve("other.org", address)
            .build()
            .unwrap();
        let no_redirect = Http::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .resolve("example.org", address)
            .resolve("other.org", address)
            .build()
            .unwrap();
        Http::for_test_with_redirect(client, no_redirect)
    }
    fn run(future: impl std::future::Future<Output = ()>) {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(future);
    }
    #[test]
    fn system_proxy_defaults_reach_local_proxy_without_resolving_origin() {
        const CHILD: &str = "NONS_SYSTEM_PROXY_TEST_CHILD";
        if std::env::var_os(CHILD).is_some() {
            run(async {
                let http = Http::new().unwrap();
                let url = url::Url::parse("http://unresolvable.invalid/index.json").unwrap();
                assert!(http.uses_proxy(&url));
                let response = http.client(&url).get(url).send().await.unwrap();
                assert_eq!(response.text().await.unwrap(), "proxied");
                let url = url::Url::parse("https://unresolvable.invalid/index.json").unwrap();
                assert!(http.uses_proxy(&url));
                // The fixture rejects CONNECT after observing it; no origin DNS is needed.
                assert!(http.client(&url).get(url).send().await.is_err());
                let bypass = url::Url::parse("http://bypass.invalid/").unwrap();
                assert!(!http.uses_proxy(&bypass));
                assert!(std::ptr::eq(http.client(&bypass), &http.direct));
                assert!(resolve_addresses("localhost", 80).await.is_ok());
            });
            return;
        }
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            for expected in [
                "GET http://unresolvable.invalid/index.json HTTP/1.1",
                "CONNECT unresolvable.invalid:443 HTTP/1.1",
            ] {
                let deadline = std::time::Instant::now() + Duration::from_secs(10);
                let mut socket = loop {
                    match listener.accept() {
                        Ok((socket, _)) => break socket,
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            assert!(std::time::Instant::now() < deadline, "proxy was not used");
                            std::thread::sleep(Duration::from_millis(10));
                        }
                        Err(error) => panic!("{error}"),
                    }
                };
                let mut bytes = Vec::new();
                while !bytes.ends_with(b"\r\n\r\n") {
                    let mut byte = [0];
                    assert_eq!(socket.read(&mut byte).unwrap(), 1);
                    bytes.push(byte[0]);
                    assert!(bytes.len() < 8192);
                }
                let request = String::from_utf8(bytes).unwrap();
                assert!(request.starts_with(expected), "{request}");
                assert!(request
                    .to_ascii_lowercase()
                    .contains("proxy-authorization: basic "));
                socket.write_all(if expected.starts_with("GET") {
                    b"HTTP/1.1 200 OK\r\nContent-Length: 7\r\nConnection: close\r\n\r\nproxied"
                } else {
                    b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                }).unwrap();
            }
        });
        // Isolate environment proxy settings from other tests and the user's configuration.
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "infrastructure::http::tests::system_proxy_defaults_reach_local_proxy_without_resolving_origin", "--nocapture"])
            .env(CHILD, "1")
            .env_remove("ALL_PROXY")
            .env_remove("all_proxy")
            .env_remove("http_proxy")
            .env_remove("https_proxy")
            .env_remove("no_proxy")
            .env_remove("REQUEST_METHOD")
            .env("HTTP_PROXY", format!("http://fixture:password@localhost:{port}"))
            .env("HTTPS_PROXY", format!("http://fixture:password@127.0.0.1:{port}"))
            .env("NO_PROXY", "bypass.invalid")
            .output().unwrap();
        let server_result = server.join();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        server_result.unwrap();
    }
    #[test]
    fn direct_connections_accept_private_resolved_addresses() {
        run(async {
            let (fixture, server) = fixture(
                "HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".into(),
                Duration::ZERO,
            );
            let url = url::Url::parse("http://example.org/").unwrap();
            assert!(fixture.client(&url).get(url).send().await.is_ok());
            server.join().unwrap();
        });
    }
    #[test]
    fn follows_redirects_and_keeps_large_text_and_empty_responses() {
        run(async {
            let active = AtomicBool::new(true);
            let (http, final_url, server) = redirect_fixture("ok".into());
            let result = http
                .request(&json!({"url":"http://example.org/start"}), &active)
                .await
                .unwrap();
            assert_eq!(result["status"], 200);
            assert_eq!(result["body"], "ok");
            assert_eq!(result["finalUrl"].as_str(), Some(final_url.as_str()));
            server.join().unwrap();

            let (http, server, _) = capture_fixture(
                "HTTP/1.1 302 Found\r\nLocation: /final\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    .into(),
                Duration::ZERO,
            );
            let result = http
                .request(
                    &json!({"url":"http://example.org/start","followRedirects":false}),
                    &active,
                )
                .await
                .unwrap();
            assert_eq!(result["status"], 302);
            assert_eq!(result["finalUrl"], "http://example.org/start");
            assert_eq!(result["headers"]["location"], "/final");
            server.join().unwrap();

            let (http, server, _) = capture_fixture(
                "HTTP/1.1 200 OK\r\nContent-Length: 7\r\nConnection: close\r\n\r\nignored".into(),
                Duration::ZERO,
            );
            let result = http
                .request(
                    &json!({"url":"http://example.org/","responseType":"none"}),
                    &active,
                )
                .await
                .unwrap();
            assert_eq!(result["body"], "");
            server.join().unwrap();

            let (http, server) = fixture(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    3 * 1024 * 1024,
                    "a".repeat(3 * 1024 * 1024)
                ),
                Duration::ZERO,
            );
            let result = http
                .request(&json!({"url":"http://example.org/"}), &active)
                .await
                .unwrap();
            assert_eq!(result["body"].as_str().unwrap().len(), 3 * 1024 * 1024);
            server.join().unwrap();
        });
    }
    #[test]
    fn follows_more_than_ten_cross_origin_redirects_and_strips_sensitive_headers() {
        const REDIRECTS: usize = 12;
        run(async {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            let captured = Arc::new(Mutex::new(Vec::new()));
            let final_request = captured.clone();
            let server = std::thread::spawn(move || {
                for index in 0..=REDIRECTS {
                    let (mut socket, _) = listener.accept().unwrap();
                    let request = read_request(&mut socket);
                    if index < REDIRECTS {
                        let host = if index % 2 == 0 {
                            "other.org"
                        } else {
                            "example.org"
                        };
                        let _ = write!(
                            socket,
                            "HTTP/1.1 302 Found\r\nLocation: http://{host}:{}/step-{}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                            address.port(),
                            index + 1
                        );
                    } else {
                        *final_request.lock().unwrap() = request;
                        let _ = socket.write_all(
                            b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok",
                        );
                    }
                }
            });
            let result = redirect_client(address)
                .request(
                    &json!({
                        "url":"http://example.org/start",
                        "headers":{
                            "authorization":"Bearer secret",
                            "cookie":"session=secret",
                            "x-public":"kept"
                        }
                    }),
                    &AtomicBool::new(true),
                )
                .await
                .unwrap();
            assert_eq!(result["status"], 200);
            assert_eq!(result["body"], "ok");
            assert!(result["finalUrl"].as_str().unwrap().ends_with("/step-12"));
            let request = String::from_utf8(captured.lock().unwrap().clone()).unwrap();
            assert!(request.to_ascii_lowercase().contains("x-public: kept"));
            assert!(!request.to_ascii_lowercase().contains("authorization:"));
            assert!(!request.to_ascii_lowercase().contains("cookie:"));
            server.join().unwrap();
        });
    }
    #[test]
    fn redirect_loops_stop_on_a_repeated_url() {
        run(async {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            let server = std::thread::spawn(move || {
                for _ in 0..2 {
                    let (mut socket, _) = listener.accept().unwrap();
                    read_request(&mut socket);
                    let _ = socket.write_all(
                        b"HTTP/1.1 302 Found\r\nLocation: /loop\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                    );
                }
            });
            let error = redirect_client(address)
                .request(
                    &json!({"url":"http://example.org/start"}),
                    &AtomicBool::new(true),
                )
                .await
                .unwrap_err();
            assert_eq!(error, "HTTP 请求失败");
            server.join().unwrap();
        });
    }
    #[test]
    fn arbitrary_methods_headers_and_bodies_reach_the_server() {
        run(async {
            for method in [
                "GET", "PUT", "PATCH", "DELETE", "OPTIONS", "PURGE", "M-SEARCH",
            ] {
                assert!(checked_method(method).is_ok(), "{method}");
            }
            assert!(checked_method("bad method").is_err());
            let body = "p".repeat(192 * 1024);

            let (http, server, captured) = capture_fixture(
                "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok".into(),
                Duration::ZERO,
            );
            let result = http
                .request(
                    &json!({
                        "url":"http://example.org/replace",
                        "method":"PUT",
                        "timeoutMs":0,
                        "headers":{"x-custom-header":"custom-value","content-length":body.len().to_string()},
                        "body":body.clone()
                    }),
                    &AtomicBool::new(true),
                )
                .await
                .unwrap();
            assert_eq!(result["status"], 200);
            let request = String::from_utf8(captured.lock().unwrap().clone()).unwrap();
            assert!(request.starts_with("PUT /replace HTTP/1.1"), "{request}");
            assert!(request
                .to_ascii_lowercase()
                .contains("x-custom-header: custom-value"));
            assert!(request.ends_with(&body));
            server.join().unwrap();
        });
    }
    #[test]
    fn zero_or_omitted_timeout_waits_past_the_old_three_second_limit() {
        run(async {
            let response = "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok";
            for timeout in [None, Some(0), Some(4000)] {
                let (http, server) = fixture(response.into(), Duration::from_millis(3200));
                let mut args = json!({"url":"http://example.org/slow"});
                if let Some(timeout) = timeout {
                    args["timeoutMs"] = json!(timeout);
                }
                let result = http.request(&args, &AtomicBool::new(true)).await.unwrap();
                assert_eq!(result["body"], "ok");
                server.join().unwrap();
            }
        });
    }
    #[test]
    fn max_bytes_is_optional_and_rejects_only_when_explicit() {
        run(async {
            let active = AtomicBool::new(true);
            for (limit, expected) in [
                (None, Ok(16)),
                (Some(16), Ok(16)),
                (Some(15), Err("maxBytes")),
            ] {
                eprintln!("case {limit:?}");
                let (http, server) = fixture(
                    "HTTP/1.1 200 OK\r\nContent-Length: 16\r\nConnection: close\r\n\r\n0123456789abcdef"
                        .into(),
                    Duration::ZERO,
                );
                let mut args = json!({"url":"http://example.org/"});
                if let Some(limit) = limit {
                    args["maxBytes"] = json!(limit);
                }
                let result = http.request(&args, &active).await;
                eprintln!("request {result:?}");
                match expected {
                    Ok(length) => {
                        assert_eq!(result.unwrap()["body"].as_str().unwrap().len(), length);
                    }
                    Err(message) => assert!(result.unwrap_err().contains(message)),
                }
                server.join().unwrap();
                eprintln!("joined");
            }
            let http = Http::new().unwrap();
            assert!(http
                .request(&json!({"url":"http://example.org/","maxBytes":0}), &active)
                .await
                .unwrap_err()
                .contains("正整数"));
        });
    }
    #[test]
    fn requests_timeout_or_cancel_when_the_instance_is_disabled() {
        run(async {
            let response = "HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
            let (http, server) = fixture(response.into(), Duration::from_millis(100));
            let active = AtomicBool::new(true);
            assert!(http
                .request(
                    &json!({"url":"http://example.org/","timeoutMs":25}),
                    &active
                )
                .await
                .unwrap_err()
                .contains("超时"));
            server.join().unwrap();
            let (http, server) = fixture(response.into(), Duration::from_millis(100));
            assert!(http
                .binary_request(
                    BinaryRequest {
                        url: "http://example.org/".into(),
                        method: "GET".into(),
                        headers: vec![],
                        body: vec![],
                        follow_redirects: true,
                        timeout_ms: 25,
                    },
                    &AtomicBool::new(true),
                )
                .await
                .unwrap_err()
                .contains("超时"));
            server.join().unwrap();
            let (http, server) = fixture(response.into(), Duration::from_millis(100));
            assert!(http
                .binary_request(
                    BinaryRequest {
                        url: "http://example.org/".into(),
                        method: "GET".into(),
                        headers: vec![],
                        body: vec![],
                        follow_redirects: true,
                        timeout_ms: 0,
                    },
                    &AtomicBool::new(true),
                )
                .await
                .is_ok());
            server.join().unwrap();
            let (http, server) = fixture(response.into(), Duration::from_millis(150));
            let active = Arc::new(AtomicBool::new(true));
            let cancelled = active.clone();
            let toggle = std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(40));
                cancelled.store(false, Ordering::SeqCst);
            });
            assert!(http
                .request(&json!({"url":"http://example.org/"}), &active)
                .await
                .unwrap_err()
                .contains("禁用"));
            toggle.join().unwrap();
            server.join().unwrap();
            let (http, server) = fixture(response.into(), Duration::from_millis(150));
            let active = Arc::new(AtomicBool::new(true));
            let cancelled = active.clone();
            let toggle = std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(40));
                cancelled.store(false, Ordering::SeqCst);
            });
            assert!(http
                .binary_request(
                    BinaryRequest {
                        url: "http://example.org/".into(),
                        method: "GET".into(),
                        headers: vec![],
                        body: vec![],
                        follow_redirects: true,
                        timeout_ms: 0,
                    },
                    &active,
                )
                .await
                .unwrap_err()
                .contains("失效"));
            toggle.join().unwrap();
            server.join().unwrap();
        });
    }
    #[test]
    fn network_urls_allow_arbitrary_hosts_ports_credentials_and_ips() {
        for input in [
            "https://example.org/a",
            "http://sub.other.test/b",
            "https://user:secret@example.org/",
            "https://example.org:444/",
            "https://127.0.0.1/",
            "http://[::1]/",
            "http://localhost/",
            "http://[::1]:8080/a",
        ] {
            assert!(checked_url(input).is_ok(), "{input}");
        }
        for input in [
            "file:///example.org",
            "ftp://example.org/a",
            "https://",
            "http://",
        ] {
            assert!(checked_url(input).is_err(), "{input}");
        }
        let long = format!("https://example.org/{}", "a".repeat(3000));
        assert!(checked_url(&long).is_ok());
    }
    #[test]
    fn playback_urls_allow_explicit_ports_and_credentials() {
        for (input, port) in [
            ("https://xy1x2xy.mcdn.bilivideo.cn:8082/a", 8082),
            ("https://b-example.edge.mountaintoys.cn:4483/b", 4483),
        ] {
            let url = checked_url(input).unwrap();
            assert_eq!(url.port(), Some(port));
        }
        assert!(checked_url("https://user:secret@x.bilivideo.cn:8082/a").is_ok());
    }
    #[test]
    fn binary_transport_keeps_bytes_and_each_cookie() {
        run(async {
            let (http, server) = fixture("HTTP/1.1 200 OK\r\nSet-Cookie: first=one; HttpOnly\r\nSet-Cookie: second=two; Secure\r\nContent-Length: 4\r\nConnection: close\r\n\r\na\0bc".into(), Duration::ZERO);
            let response = http
                .binary_request(
                    BinaryRequest {
                        url: "http://example.org/".into(),
                        method: "POST".into(),
                        headers: vec![("x-platform-check".into(), "signature".into())],
                        body: vec![0, 255, 1],
                        follow_redirects: true,
                        timeout_ms: 1000,
                    },
                    &AtomicBool::new(true),
                )
                .await
                .unwrap();
            assert_eq!(response.body, b"a\0bc");
            assert_eq!(response.url, "http://example.org/");
            assert_eq!(
                response
                    .headers
                    .iter()
                    .filter(|(key, _)| key == "set-cookie")
                    .count(),
                2
            );
            server.join().unwrap();
        });
    }
    #[test]
    fn binary_requests_follow_redirects_and_keep_large_bodies() {
        run(async {
            let body = "x".repeat(3 * 1024 * 1024);
            let (http, final_url, server) = redirect_fixture(body.clone());
            let response = http
                .binary_request(
                    BinaryRequest {
                        url: "http://example.org/start".into(),
                        method: "GET".into(),
                        headers: vec![],
                        body: vec![],
                        follow_redirects: true,
                        timeout_ms: 0,
                    },
                    &AtomicBool::new(true),
                )
                .await
                .unwrap();
            assert_eq!(response.status, 200);
            assert_eq!(response.url, final_url);
            assert_eq!(response.body.len(), body.len());
            server.join().unwrap();
        });
    }
    #[test]
    fn binary_requests_accept_arbitrary_methods_headers_and_bodies() {
        run(async {
            let body = vec![0x5a; 128 * 1024];
            let (http, server, captured) = capture_fixture(
                "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok".into(),
                Duration::ZERO,
            );
            let response = http
                .binary_request(
                    BinaryRequest {
                        url: "http://example.org/cache".into(),
                        method: "PURGE".into(),
                        headers: vec![("x-custom-header".into(), "custom-value".into())],
                        body: body.clone(),
                        follow_redirects: false,
                        timeout_ms: 0,
                    },
                    &AtomicBool::new(true),
                )
                .await
                .unwrap();
            assert_eq!(response.status, 200);
            assert_eq!(response.url, "http://example.org/cache");
            assert_eq!(response.body, b"ok");
            let request = String::from_utf8(captured.lock().unwrap().clone()).unwrap();
            assert!(request.starts_with("PURGE /cache HTTP/1.1"), "{request}");
            assert!(request
                .to_ascii_lowercase()
                .contains("x-custom-header: custom-value"));
            assert!(request.ends_with(&String::from_utf8(body).unwrap()));
            server.join().unwrap();
        });
    }
    #[test]
    fn request_headers_keep_only_library_syntax_validation() {
        for header in [
            "Content-Length",
            "Transfer-Encoding",
            "Proxy-Authorization",
            "Connection",
            "Upgrade",
            "Sec-WebSocket-Protocol",
            "X-Custom-Extension",
        ] {
            let builder =
                reqwest::Client::new().request(reqwest::Method::POST, "https://example.org/");
            assert!(
                apply_headers(builder, [(header, "value")], "test").is_ok(),
                "{header}"
            );
        }
        let builder = reqwest::Client::new().request(reqwest::Method::POST, "https://example.org/");
        assert!(apply_headers(builder, [("bad name", "value")], "test").is_err());
        let builder = reqwest::Client::new().request(reqwest::Method::POST, "https://example.org/");
        assert!(apply_headers(builder, [("x-test", "bad\nvalue")], "test").is_err());
    }
}
