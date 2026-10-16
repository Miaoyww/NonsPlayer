use crate::{AdapterResult, Http, Request, Response};

wit_bindgen::generate!({
    path: ["../../plugins/wit", "../../packages/adapter-host/wit"],
    inline: "package nons:jbsou; world adapter { import nons:host/transport@1.0.0; export initialize: func() -> result<_, string>; export shutdown: func() -> result<_, string>; export call: func(method: string, args: string) -> result<string, string>; }",
    world: "nons:jbsou/adapter",
    generate_all,
});
struct Adapter;
struct Host;
impl Http for Host {
    fn send(&self, request: Request) -> AdapterResult<Response> {
        use nons::host::transport as host;
        let headers = request
            .headers
            .into_iter()
            .map(|(name, value)| host::Header { name, value })
            .collect();
        let response = host::http_request(&host::Request {
            url: request.url,
            method: request.method.into(),
            headers,
            body: request.body,
            timeout_ms: 4000,
        })
        .map_err(|error| match error.as_str() {
            "cancelled" => "cancelled",
            "timeout" => "timeout",
            "staleContext" => "staleContext",
            "permissionDenied" => "permissionDenied",
            _ => "network",
        })?;
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
}
impl Guest for Adapter {
    fn initialize() -> std::result::Result<(), String> {
        Ok(())
    }
    fn shutdown() -> std::result::Result<(), String> {
        Ok(())
    }
    fn call(method: String, args: String) -> std::result::Result<String, String> {
        Ok(crate::call(&Host, &method, &args))
    }
}
export!(Adapter);
