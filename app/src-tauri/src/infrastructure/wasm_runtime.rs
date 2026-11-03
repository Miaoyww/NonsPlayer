use crate::model::AppResult;
use std::{future::Future, path::Path, pin::Pin, sync::Arc, time::Duration};
use wasmtime::{
    component::{Component, Linker},
    Config, Engine, Store,
};
mod compilation;
pub(crate) use compilation::compile as compile_component;
pub(crate) use compilation::configure_cache as configure_compilation_cache;
pub(crate) use compilation::{
    cache_status as compilation_cache_status, clear_cache as clear_compilation_cache,
};

wasmtime::component::bindgen!({
    path: "../../plugins/wit",
    world: "plugin",
    imports: { default: async },
    exports: { default: async },
});
pub(crate) mod host_io {
    wasmtime::component::bindgen!({ path: "../../packages/adapter-host/wit", world: "host", imports: { default: async } });
    pub use nons::host::transport::{
        Event, Header, Host as TransportHost, Message, Request, Response,
    };
}
type HostFuture<'a, T> = Pin<Box<dyn Future<Output = AppResult<T>> + Send + 'a>>;

pub(crate) trait HostHandler: Send + Sync {
    /// Declared by the trusted loader, never chosen by guest request arguments.
    fn system_capabilities(&self) -> bool {
        false
    }
    fn http_request(&self, _value: host_io::Request) -> HostFuture<'_, host_io::Response> {
        Box::pin(async { Err("permissionDenied".into()) })
    }
    fn websocket_open(
        &self,
        _url: String,
        _protocol: String,
        _timeout: u32,
    ) -> HostFuture<'_, u32> {
        Box::pin(async { Err("permissionDenied".into()) })
    }
    fn websocket_send(&self, _socket: u32, _data: Vec<u8>) -> HostFuture<'_, ()> {
        Box::pin(async { Err("permissionDenied".into()) })
    }
    fn websocket_close(&self, _socket: u32) -> HostFuture<'_, ()> {
        Box::pin(async { Err("permissionDenied".into()) })
    }
    fn next_event(&self, _timeout: u32) -> HostFuture<'_, Option<host_io::Event>> {
        Box::pin(async { Err("permissionDenied".into()) })
    }
    fn credential_read(&self) -> HostFuture<'_, Vec<u8>> {
        Box::pin(async { Err("permissionDenied".into()) })
    }
    fn credential_invalidate(&self) -> HostFuture<'_, ()> {
        Box::pin(async { Err("permissionDenied".into()) })
    }
    fn credential_replace(&self, _data: Vec<u8>) -> HostFuture<'_, ()> {
        Box::pin(async { Err("permissionDenied".into()) })
    }
    fn credential_publish(&self, _record: String, _data: Vec<u8>) -> HostFuture<'_, ()> {
        Box::pin(async { Err("permissionDenied".into()) })
    }
    fn state_get(&self, _key: String) -> HostFuture<'_, Option<String>> {
        Box::pin(async { Err("permissionDenied".into()) })
    }
    fn state_set(&self, _key: String, _value: String) -> HostFuture<'_, ()> {
        Box::pin(async { Err("permissionDenied".into()) })
    }
    fn call<'a>(
        &'a self,
        operation: &'a str,
        args: &'a str,
    ) -> Pin<Box<dyn Future<Output = AppResult<String>> + Send + 'a>>;
}

struct State {
    host: Arc<dyn HostHandler>,
    wasi: wasmtime_wasi::WasiCtx,
    table: wasmtime::component::ResourceTable,
}
impl wasmtime_wasi::WasiView for State {
    fn ctx(&mut self) -> wasmtime_wasi::WasiCtxView<'_> {
        wasmtime_wasi::WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}
struct PollData;
impl wasmtime::component::HasData for PollData {
    type Data<'a> = &'a mut wasmtime::component::ResourceTable;
}
impl nons::plugin::host::Host for State {
    async fn call(&mut self, operation: String, args: String) -> Result<String, String> {
        self.host
            .call(&operation, &args)
            .await
            .inspect_err(|error| {
                if operation != "diagnostic.report" {
                    crate::infrastructure::diagnostics::warn(&operation, error);
                }
            })
    }
}
impl host_io::TransportHost for State {
    async fn http_request(&mut self, value: host_io::Request) -> Result<host_io::Response, String> {
        self.host.http_request(value).await
    }
    async fn websocket_open(
        &mut self,
        url: String,
        protocol: String,
        timeout_ms: u32,
    ) -> Result<u32, String> {
        self.host.websocket_open(url, protocol, timeout_ms).await
    }
    async fn websocket_send(&mut self, socket: u32, data: Vec<u8>) -> Result<(), String> {
        self.host.websocket_send(socket, data).await
    }
    async fn websocket_close(&mut self, socket: u32) -> Result<(), String> {
        self.host.websocket_close(socket).await
    }
    async fn next_event(&mut self, timeout_ms: u32) -> Result<Option<host_io::Event>, String> {
        self.host.next_event(timeout_ms).await
    }
    async fn credential_read(&mut self) -> Result<Vec<u8>, String> {
        self.host.credential_read().await
    }
    async fn credential_invalidate(&mut self) -> Result<(), String> {
        self.host.credential_invalidate().await
    }
    async fn credential_replace(&mut self, data: Vec<u8>) -> Result<(), String> {
        self.host.credential_replace(data).await
    }
    async fn credential_publish(&mut self, record: String, data: Vec<u8>) -> Result<(), String> {
        self.host.credential_publish(record, data).await
    }
    async fn state_get(&mut self, key: String) -> Result<Option<String>, String> {
        self.host.state_get(key).await
    }
    async fn state_set(&mut self, key: String, value: String) -> Result<(), String> {
        self.host.state_set(key, value).await
    }
}
pub fn engine() -> AppResult<Engine> {
    let mut config = Config::new();
    config
        .wasm_component_model(true)
        .consume_fuel(false)
        .epoch_interruption(true);
    compilation::configure_engine(&mut config);
    let engine = Engine::new(&config).map_err(|e| e.to_string())?;
    // Cooperative yield only: guest computation returns to the async executor
    // every 10ms. Nothing is denied or counted; without it a guest loop would
    // hold the calling thread forever and the app could not stay responsive.
    // Weak ownership lets the timer exit after the last runtime/adapter is dropped.
    let weak = engine.weak();
    std::thread::Builder::new()
        .name("nons-wasm-epoch".into())
        .spawn(move || loop {
            std::thread::sleep(Duration::from_millis(10));
            let Some(engine) = weak.upgrade() else { break };
            engine.increment_epoch();
        })
        .map_err(|e| e.to_string())?;
    Ok(engine)
}
pub struct Runtime {
    store: Store<State>,
    bindings: Plugin,
    pub faulted: bool,
}
impl Runtime {
    pub(crate) async fn load_host(
        engine: Engine,
        path: &Path,
        host: Arc<dyn HostHandler>,
    ) -> AppResult<Self> {
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        let compile_engine = engine.clone();
        let compiled = tauri::async_runtime::spawn_blocking(move || {
            compile_component(&compile_engine, &bytes)
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| format!("无效 WASM Component: {e}"))?;
        log::info!(target: "nons::wasm", "stage=compile_component cache={} elapsed_ms={}", compiled.cache, compiled.elapsed.as_millis());
        Self::from_component(engine, &compiled.component, host).await
    }
    /// Shared execution machinery; the caller supplies its own capability boundary.
    pub(crate) async fn from_component(
        engine: Engine,
        component: &Component,
        host: Arc<dyn HostHandler>,
    ) -> AppResult<Self> {
        let mut linker = Linker::<State>::new(&engine);
        // Only our WIT imports are linked: no WASI filesystem, network or process APIs.
        Plugin::add_to_linker::<_, wasmtime::component::HasSelf<State>>(&mut linker, |state| state)
            .map_err(|e| e.to_string())?;
        host_io::Host_::add_to_linker::<_, wasmtime::component::HasSelf<State>>(
            &mut linker,
            |state| state,
        )
        .map_err(|e| e.to_string())?;
        if host.system_capabilities() {
            use wasmtime_wasi::p2::bindings::{clocks, io, random};
            use wasmtime_wasi::{
                cli::{WasiCli, WasiCliView},
                clocks::{WasiClocks, WasiClocksView},
                random::{WasiRandom, WasiRandomView},
            };
            clocks::wall_clock::add_to_linker::<State, WasiClocks>(&mut linker, State::clocks)
                .map_err(|e| e.to_string())?;
            clocks::monotonic_clock::add_to_linker::<State, WasiClocks>(&mut linker, State::clocks)
                .map_err(|e| e.to_string())?;
            random::random::add_to_linker::<State, WasiRandom>(&mut linker, State::random)
                .map_err(|e| e.to_string())?;
            random::insecure::add_to_linker::<State, WasiRandom>(&mut linker, State::random)
                .map_err(|e| e.to_string())?;
            random::insecure_seed::add_to_linker::<State, WasiRandom>(&mut linker, State::random)
                .map_err(|e| e.to_string())?;
            io::poll::add_to_linker::<State, PollData>(&mut linker, |s| &mut s.table)
                .map_err(|e| e.to_string())?;
            io::error::add_to_linker::<State, PollData>(&mut linker, |s| &mut s.table)
                .map_err(|e| e.to_string())?;
            io::streams::add_to_linker::<State, PollData>(&mut linker, |s| &mut s.table)
                .map_err(|e| e.to_string())?;
            // The engine retains stderr plumbing for timer errors. It is a sink,
            // never inherited process stderr or arbitrary CLI/environment access.
            wasmtime_wasi::p2::bindings::cli::stderr::add_to_linker::<State, WasiCli>(
                &mut linker,
                State::cli,
            )
            .map_err(|e| e.to_string())?;
        }
        let table = wasmtime::component::ResourceTable::new();
        let mut store = Store::new(
            &engine,
            State {
                host,
                wasi: wasmtime_wasi::WasiCtxBuilder::new()
                    .max_random_size(65536)
                    .build(),
                table,
            },
        );
        store.epoch_deadline_async_yield_and_update(1);
        store.set_epoch_deadline(1);
        let bindings = Plugin::instantiate_async(&mut store, component, &linker)
            .await
            .map_err(|e| e.to_string())?;
        let mut runtime = Self {
            store,
            bindings,
            faulted: false,
        };
        runtime.arm_yield();
        runtime
            .bindings
            .call_initialize(&mut runtime.store)
            .await
            .map_err(|e| e.to_string())??;
        Ok(runtime)
    }
    /// Re-arms the cooperative yield point. It restores no quota and denies no work.
    fn arm_yield(&mut self) {
        self.store.set_epoch_deadline(1);
    }
    pub async fn call(&mut self, method: &str, args: &str) -> AppResult<String> {
        self.arm_yield();
        let result = match self.bindings.call_call(&mut self.store, method, args).await {
            Ok(result) => result?,
            Err(error) => {
                self.faulted = true;
                return Err(format!("插件执行失败: {error:#}"));
            }
        };
        if let Err(error) = serde_json::from_str::<serde_json::Value>(&result) {
            crate::infrastructure::diagnostics::warn("wasm.response.decode", error);
            self.faulted = true;
            return Err("插件返回了无效 JSON".into());
        }
        Ok(result)
    }
    pub async fn shutdown(&mut self) {
        match self.bindings.call_shutdown(&mut self.store).await {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                crate::infrastructure::diagnostics::warn("wasm.shutdown.guest", error);
            }
            Err(error) => {
                crate::infrastructure::diagnostics::warn("wasm.shutdown.runtime", error);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn instructions_and_guest_memory_are_not_metered() {
        run(async {
            let mut runtime = Runtime::load_host(
                engine().unwrap(),
                &fixture("runtime-fixture"),
                Arc::new(FakeHost::default()),
            )
            .await
            .expect("Run pnpm plugins:fixtures before Rust tests");
            assert_eq!(
                runtime
                    .call("work", "null")
                    .await
                    .expect("instructions must not be counted"),
                "50000000"
            );
            assert!(
                runtime.store.get_fuel().is_err(),
                "fuel instrumentation must stay disabled"
            );
            // The fixture allocates 80MiB, above the old 64MiB Store ceiling.
            assert_eq!(runtime.call("memory", "null").await.unwrap(), "null");
            // And returns 70KiB, above the old 64KiB JSON response ceiling.
            let large = runtime.call("large", "null").await.unwrap();
            assert_eq!(large.len(), 70 * 1024 + 2);
        });
    }
    #[test]
    fn trap_and_invalid_results_are_isolated() {
        run(async {
            let engine = engine().unwrap();
            for method in ["trap", "invalid-json"] {
                let mut runtime = Runtime::load_host(
                    engine.clone(),
                    &fixture("runtime-fixture"),
                    Arc::new(FakeHost::default()),
                )
                .await
                .expect("Run pnpm plugins:fixtures before Rust tests");
                assert!(runtime.call(method, "null").await.is_err(), "{method}");
                assert!(runtime.faulted, "{method}");
                let mut healthy = Runtime::load_host(
                    engine.clone(),
                    &fixture("runtime-fixture"),
                    Arc::new(FakeHost::default()),
                )
                .await
                .unwrap();
                assert_eq!(healthy.call("healthy", "null").await.unwrap(), "null");
            }
        });
    }
    #[derive(Default)]
    struct FakeHost {}
    impl HostHandler for FakeHost {
        fn call<'a>(
            &'a self,
            _: &'a str,
            _: &'a str,
        ) -> Pin<Box<dyn Future<Output = AppResult<String>> + Send + 'a>> {
            Box::pin(async { Err("permission denied".into()) })
        }
    }
    fn fixture(name: &str) -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
            "../../plugins/{name}/backend/target/wasm32-unknown-unknown/release/{}.wasm",
            name.replace('-', "_")
        ))
    }
    fn run(future: impl Future<Output = ()>) {
        tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap()
            .block_on(future);
    }
}
