use crate::{Adapter, Error, Event, Host, Response, Result as AdapterResult};
use std::cell::RefCell;

wit_bindgen::generate!({
    path: ["../../plugins/wit", "../../packages/adapter-host/wit", "interfaces/wasi-clocks", "interfaces/wasi-random"],
    inline: "package nons:qqmusic; world adapter { import nons:host/transport@1.0.0; import nons:plugin/host@1.0.0; import wasi:clocks/wall-clock@0.2.0; import wasi:random/random@0.2.0; export initialize: func() -> result<_, string>; export shutdown: func() -> result<_, string>; export call: func(method: string, args: string) -> result<string, string>; }",
    world: "nons:qqmusic/adapter",
    generate_all,
});

struct Component;
struct Transport;
thread_local! { static ADAPTER: RefCell<Adapter> = RefCell::new(Adapter::default()); }
use nons::host::transport;

impl Host for Transport {
    fn http(
        &self,
        method: &str,
        url: &str,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
        timeout_ms: u32,
    ) -> AdapterResult<Response> {
        let response = transport::http_request(&transport::Request {
            url: url.into(),
            method: method.into(),
            headers: headers
                .into_iter()
                .map(|(name, value)| transport::Header { name, value })
                .collect(),
            body,
            timeout_ms: timeout_ms.clamp(1, 15000),
        })
        .map_err(|e| Error::host(&e, "host.http"))?;
        Ok(Response {
            status: response.status,
            headers: response
                .headers
                .into_iter()
                .map(|h| (h.name, h.value))
                .collect(),
            body: response.body,
        })
    }
    fn now_ms(&self) -> u64 {
        let now = wasi::clocks::wall_clock::now();
        now.seconds
            .saturating_mul(1000)
            .saturating_add((now.nanoseconds / 1_000_000) as u64)
    }
    fn random(&self, len: usize) -> AdapterResult<Vec<u8>> {
        if len > 65536 {
            return Err(Error::new("invalidData", "host.random-size"));
        }
        Ok(wasi::random::random::get_random_bytes(len as u64))
    }
    fn state_get(&self, key: &str) -> AdapterResult<Option<String>> {
        transport::state_get(key).map_err(|e| Error::host(&e, "host.state-get"))
    }
    fn state_set(&self, key: &str, value: &str) -> AdapterResult<()> {
        transport::state_set(key, value).map_err(|e| Error::host(&e, "host.state-set"))
    }
    fn credential_read(&self) -> AdapterResult<Vec<u8>> {
        transport::credential_read().map_err(|e| Error::host(&e, "host.credential-read"))
    }
    fn credential_publish(&self, metadata: &str, data: &[u8]) -> AdapterResult<()> {
        transport::credential_publish(metadata, data)
            .map_err(|e| Error::host(&e, "host.credential-publish"))
    }
    fn credential_replace(&self, data: &[u8]) -> AdapterResult<()> {
        transport::credential_replace(data).map_err(|e| Error::host(&e, "host.credential-replace"))
    }
    fn credential_invalidate(&self) -> AdapterResult<()> {
        transport::credential_invalidate()
            .map_err(|e| Error::host(&e, "host.credential-invalidate"))
    }
    fn websocket_open(&self, url: &str, protocol: &str, timeout_ms: u32) -> AdapterResult<u32> {
        transport::websocket_open(url, protocol, timeout_ms)
            .map_err(|e| Error::host(&e, "host.websocket-open"))
    }
    fn websocket_send(&self, socket: u32, data: &[u8]) -> AdapterResult<()> {
        transport::websocket_send(socket, data).map_err(|e| Error::host(&e, "host.websocket-send"))
    }
    fn websocket_close(&self, socket: u32) -> AdapterResult<()> {
        transport::websocket_close(socket).map_err(|e| Error::host(&e, "host.websocket-close"))
    }
    fn next_event(&self, timeout_ms: u32) -> AdapterResult<Option<Event>> {
        Ok(transport::next_event(timeout_ms)
            .map_err(|e| Error::host(&e, "host.next-event"))?
            .map(|e| match e {
                transport::Event::Binary(message) => Event::Binary {
                    socket: message.socket,
                    data: message.data,
                },
                transport::Event::Closed(socket) => Event::Closed(socket),
            }))
    }
    fn diagnostic(&self, stage: &str, class: &str, code: &str) {
        let _ = nons::plugin::host::call(
            "diagnostic.report",
            &serde_json::json!({"stage":stage,"class":class,"code":code}).to_string(),
        );
    }
}
impl Guest for Component {
    fn initialize() -> std::result::Result<(), String> {
        Ok(())
    }
    fn shutdown() -> std::result::Result<(), String> {
        ADAPTER.with(|adapter| adapter.borrow_mut().shutdown(&Transport));
        Ok(())
    }
    fn call(method: String, args: String) -> std::result::Result<String, String> {
        Ok(ADAPTER.with(|adapter| adapter.borrow_mut().call(&Transport, &method, &args)))
    }
}
export!(Component);
