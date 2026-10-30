use crate::model::AppResult;
use std::{future::Future, path::Path, pin::Pin, sync::Arc, time::Duration};
use wasmtime::{
    component::{Component, Linker},
    Config, Engine, ResourceLimiter, Store, StoreLimits, StoreLimitsBuilder,
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

const MAX_JSON: usize = 64 * 1024;
/// Limits depend on the caller's role, never the platform or guest language.
#[derive(Clone, Copy)]
pub(crate) struct Budget {
    pub component_bytes: usize,
    memory_bytes: usize,
    memories: usize,
    fuel: Option<u64>,
    deadline: Duration,
}
impl Budget {
    pub const PLUGIN: Self = Self {
        component_bytes: 16 * 1024 * 1024,
        memory_bytes: 64 * 1024 * 1024,
        memories: 1,
        fuel: Some(10_000_000),
        deadline: Duration::from_secs(5),
    };
    // Temporary adapter budget authorized for composition validation.
    pub const ADAPTER: Self = Self {
        component_bytes: 32 * 1024 * 1024,
        memory_bytes: 256 * 1024 * 1024,
        memories: 4,
        fuel: None,
        deadline: Duration::from_secs(15),
    };
}
struct MemoryLimits {
    inner: StoreLimits,
    maximum: usize,
    allocated: usize,
    pending_growth: usize,
}
impl MemoryLimits {
    fn new(budget: Budget) -> Self {
        Self {
            inner: StoreLimitsBuilder::new()
                .memory_size(budget.memory_bytes)
                .table_elements(10000)
                .instances(16)
                .memories(budget.memories)
                .tables(8)
                .trap_on_grow_failure(true)
                .build(),
            maximum: budget.memory_bytes,
            allocated: 0,
            pending_growth: 0,
        }
    }
}
impl ResourceLimiter for MemoryLimits {
    fn memory_growing(
        &mut self,
        current: usize,
        desired: usize,
        maximum: Option<usize>,
    ) -> wasmtime::Result<bool> {
        let growth = desired.saturating_sub(current);
        if growth > self.maximum.saturating_sub(self.allocated) {
            return Err(wasmtime::Error::msg("WASM 总线性内存超过预算"));
        }
        let allowed = self.inner.memory_growing(current, desired, maximum)?;
        if allowed {
            self.pending_growth = growth;
            self.allocated += growth;
        }
        Ok(allowed)
    }
    fn memory_grow_failed(&mut self, error: wasmtime::Error) -> wasmtime::Result<()> {
        self.allocated = self.allocated.saturating_sub(self.pending_growth);
        self.pending_growth = 0;
        self.inner.memory_grow_failed(error)
    }
    fn table_growing(
        &mut self,
        current: usize,
        desired: usize,
        maximum: Option<usize>,
    ) -> wasmtime::Result<bool> {
        self.inner.table_growing(current, desired, maximum)
    }
    fn table_grow_failed(&mut self, error: wasmtime::Error) -> wasmtime::Result<()> {
        self.inner.table_grow_failed(error)
    }
    fn instances(&self) -> usize {
        self.inner.instances()
    }
    fn tables(&self) -> usize {
        self.inner.tables()
    }
    fn memories(&self) -> usize {
        self.inner.memories()
    }
}
struct State {
    host: Arc<dyn HostHandler>,
    limits: MemoryLimits,
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
    engine_with_budget(Budget::PLUGIN)
}
pub(crate) fn engine_with_budget(budget: Budget) -> AppResult<Engine> {
    let mut config = Config::new();
    config
        .wasm_component_model(true)
        .consume_fuel(budget.fuel.is_some())
        .epoch_interruption(true);
    compilation::configure_engine(&mut config);
    let engine = Engine::new(&config).map_err(|e| e.to_string())?;
    if budget.fuel.is_none() {
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
    }
    Ok(engine)
}
pub struct Runtime {
    store: Store<State>,
    bindings: Plugin,
    pub faulted: bool,
    budget: Budget,
}
impl Runtime {
    pub(crate) async fn load_host(
        engine: Engine,
        path: &Path,
        host: Arc<dyn HostHandler>,
    ) -> AppResult<Self> {
        Self::load_host_with_budget(engine, path, host, Budget::PLUGIN).await
    }
    pub(crate) async fn load_host_with_budget(
        engine: Engine,
        path: &Path,
        host: Arc<dyn HostHandler>,
        budget: Budget,
    ) -> AppResult<Self> {
        use std::io::Read;
        let mut bytes = Vec::new();
        std::fs::File::open(path)
            .map_err(|e| e.to_string())?
            .take(budget.component_bytes as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() > budget.component_bytes {
            return Err("WASM 入口过大".into());
        }
        let compile_engine = engine.clone();
        let compiled = tauri::async_runtime::spawn_blocking(move || {
            compile_component(&compile_engine, &bytes)
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| format!("无效 WASM Component: {e}"))?;
        log::info!(target: "nons::wasm", "stage=compile_component cache={} elapsed_ms={}", compiled.cache, compiled.elapsed.as_millis());
        Self::from_component_with_budget(engine, &compiled.component, host, budget).await
    }
    /// Shared execution machinery; the caller supplies its own capability boundary.
    pub(crate) async fn from_component_with_budget(
        engine: Engine,
        component: &Component,
        host: Arc<dyn HostHandler>,
        budget: Budget,
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
        let limits = MemoryLimits::new(budget);
        let mut table = wasmtime::component::ResourceTable::new();
        table.set_max_capacity(64);
        let mut store = Store::new(
            &engine,
            State {
                host,
                limits,
                wasi: wasmtime_wasi::WasiCtxBuilder::new()
                    .max_random_size(65536)
                    .build(),
                table,
            },
        );
        store.limiter(|state| &mut state.limits);
        if let Some(fuel) = budget.fuel {
            store.set_fuel(fuel).map_err(|e| e.to_string())?;
            store
                .fuel_async_yield_interval(Some(10000))
                .map_err(|e| e.to_string())?;
            store.set_epoch_deadline(budget.deadline.as_millis().div_ceil(10) as u64);
        } else {
            store.epoch_deadline_async_yield_and_update(1);
            store.set_epoch_deadline(1);
        }
        let bindings = tokio::time::timeout(
            budget.deadline,
            Plugin::instantiate_async(&mut store, component, &linker),
        )
        .await
        .inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("wasm.instantiate.timeout", error);
        })
        .map_err(|_| "WASM 实例化超时")?
        .map_err(|e| e.to_string())?;
        let mut runtime = Self {
            store,
            bindings,
            faulted: false,
            budget,
        };
        runtime.reset_budget()?;
        tokio::time::timeout(
            budget.deadline,
            runtime.bindings.call_initialize(&mut runtime.store),
        )
        .await
        .inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("wasm.initialize.timeout", error);
        })
        .map_err(|_| "插件初始化超时")?
        .map_err(|e| e.to_string())??;
        Ok(runtime)
    }
    fn reset_budget(&mut self) -> AppResult<()> {
        if let Some(fuel) = self.budget.fuel {
            self.store.set_fuel(fuel).map_err(|e| e.to_string())?;
            self.store
                .set_epoch_deadline(self.budget.deadline.as_millis().div_ceil(10) as u64);
        } else {
            self.store.set_epoch_deadline(1);
        }
        Ok(())
    }
    pub async fn call(&mut self, method: &str, args: &str) -> AppResult<String> {
        if method.len() > 128 || args.len() > MAX_JSON {
            return Err("插件调用过大".into());
        }
        self.reset_budget()?;
        let outcome = tokio::time::timeout(
            self.budget.deadline,
            self.bindings.call_call(&mut self.store, method, args),
        )
        .await;
        let result = match outcome {
            Ok(Ok(result)) => result?,
            Ok(Err(error)) => {
                self.faulted = true;
                return Err(format!("插件执行失败: {error:#}"));
            }
            Err(error) => {
                crate::infrastructure::diagnostics::warn("wasm.invoke.timeout", error);
                self.faulted = true;
                return Err("插件执行超时".into());
            }
        };
        if result.len() > MAX_JSON {
            self.faulted = true;
            return Err("插件响应过大".into());
        }
        if let Err(error) = serde_json::from_str::<serde_json::Value>(&result) {
            crate::infrastructure::diagnostics::warn("wasm.response.decode", error);
            self.faulted = true;
            return Err("插件返回了无效 JSON".into());
        }
        Ok(result)
    }
    pub async fn shutdown(&mut self) {
        if let Err(error) = self.reset_budget() {
            crate::infrastructure::diagnostics::warn("wasm.shutdown.budget", error);
            return;
        }
        match tokio::time::timeout(
            Duration::from_secs(1),
            self.bindings.call_shutdown(&mut self.store),
        )
        .await
        {
            Ok(Ok(Ok(()))) => {}
            Ok(Ok(Err(error))) => {
                crate::infrastructure::diagnostics::warn("wasm.shutdown.guest", error);
            }
            Ok(Err(error)) => {
                crate::infrastructure::diagnostics::warn("wasm.shutdown.runtime", error);
            }
            Err(error) => {
                crate::infrastructure::diagnostics::warn("wasm.shutdown.timeout", error);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    #[test]
    fn adapter_work_is_not_limited_by_instruction_count() {
        run(async {
            let mut runtime = Runtime::load_host_with_budget(
                engine_with_budget(Budget::ADAPTER).unwrap(),
                &fixture("runtime-fixture"),
                Arc::new(FakeHost::default()),
                Budget::ADAPTER,
            )
            .await
            .unwrap();
            assert_eq!(
                runtime
                    .call("work", "null")
                    .await
                    .expect("adapter instructions must not run out of fuel"),
                "50000000"
            );
            assert!(
                runtime.store.get_fuel().is_err(),
                "fuel instrumentation must be disabled, not assigned a larger quota"
            );
        });
    }
    #[test]
    fn adapter_without_fuel_still_obeys_wall_timeout() {
        run(async {
            let mut budget = Budget::ADAPTER;
            budget.deadline = Duration::from_millis(100);
            let mut runtime = Runtime::load_host_with_budget(
                engine_with_budget(budget).unwrap(),
                &fixture("runtime-fixture"),
                Arc::new(FakeHost::default()),
                budget,
            )
            .await
            .unwrap();
            let started = std::time::Instant::now();
            assert!(runtime
                .call("loop", "null")
                .await
                .unwrap_err()
                .contains("超时"));
            assert!(runtime.faulted);
            assert!(started.elapsed() < Duration::from_secs(1));
        });
    }
    #[test]
    fn composed_memories_share_one_total_budget() {
        let mut budget = Budget::ADAPTER;
        budget.memory_bytes = 4 * 65536;
        let mut store = Store::new(&engine().unwrap(), MemoryLimits::new(budget));
        store.limiter(|limits| limits);
        let first = wasmtime::Memory::new(&mut store, wasmtime::MemoryType::new(3, None)).unwrap();
        assert!(wasmtime::Memory::new(&mut store, wasmtime::MemoryType::new(2, None)).is_err());
        assert_eq!(store.data().allocated, 3 * 65536);
        let second = wasmtime::Memory::new(&mut store, wasmtime::MemoryType::new(1, None)).unwrap();
        assert!(first.grow(&mut store, 1).is_err());
        assert!(second.grow(&mut store, 1).is_err());
        assert_eq!(store.data().allocated, 4 * 65536);
    }
    #[test]
    fn ordinary_plugins_still_allow_only_one_memory() {
        let engine = engine().unwrap();
        let module =
            wasmtime::Module::from_binary(&engine, &[0, 97, 115, 109, 1, 0, 0, 0, 5, 3, 1, 0, 1])
                .unwrap();
        let mut store = Store::new(&engine, MemoryLimits::new(Budget::PLUGIN));
        store.limiter(|limits| limits);
        wasmtime::Instance::new(&mut store, &module, &[]).unwrap();
        assert!(wasmtime::Instance::new(&mut store, &module, &[]).is_err());
    }
    #[test]
    fn failed_allocation_returns_reserved_bytes() {
        let mut limits = MemoryLimits::new(Budget::ADAPTER);
        assert!(limits.memory_growing(0, 65536, None).unwrap());
        assert!(limits
            .memory_grow_failed(wasmtime::Error::msg("allocation failed"))
            .is_err());
        assert_eq!(limits.allocated, 0);
    }
    #[derive(Default)]
    struct FakeHost {
        events: Mutex<Vec<serde_json::Value>>,
        http_requests: Mutex<Vec<serde_json::Value>>,
    }
    impl HostHandler for FakeHost {
        fn call<'a>(
            &'a self,
            operation: &'a str,
            args: &'a str,
        ) -> Pin<Box<dyn Future<Output = AppResult<String>> + Send + 'a>> {
            Box::pin(async move {
                match operation {
                    "config.get" => Ok(r#"{"values":{"detectLinks":true}}"#.into()),
                    "http.request" => {
                        self.http_requests
                            .lock()
                            .unwrap()
                            .push(serde_json::from_str(args).unwrap());
                        Ok(r#"{"status":302,"headers":{"location":"https://music.163.com/#/song?id=347230"},"body":""}"#.into())
                    }
                    "music.read-track" => {
                        let args: serde_json::Value = serde_json::from_str(args).unwrap();
                        assert_eq!(
                            args["reference"],
                            serde_json::json!({"source":"netease","kind":"track","id":"347230"})
                        );
                        Ok(serde_json::json!({"reference":{"source":"netease","kind":"track","id":"347230"},"artists":[],"title":"Fixture song","artist":"Fixture artist","album":"Fixture album","durationMs":1000,"cover":nons_music_adapter_sdk::cover::CoverImage::default()}).to_string())
                    }
                    "events.emit" => {
                        self.events
                            .lock()
                            .unwrap()
                            .push(serde_json::from_str(args).unwrap());
                        Ok("null".into())
                    }
                    "wait" => {
                        tokio::time::sleep(Duration::from_secs(30)).await;
                        Ok("null".into())
                    }
                    _ => Err("permission denied".into()),
                }
            })
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
    #[test]
    #[ignore = "The Nons core no longer bundles plugins; the adapter/plugin store supplies components."]
    fn actual_island_component_calls_host_and_emits_song() {
        run(async {
            let host = Arc::new(FakeHost::default());
            let mut runtime =
                Runtime::load_host(engine().unwrap(), &fixture("netease-island"), host.clone())
                    .await
                    .expect(
                        "The plugin store must provide this component for the integration test",
                    );
            assert_eq!(
                runtime
                    .call(
                        "event:clipboard-text",
                        r#"{"text":"fixture text https://evil.test/private https://music.163.com/#/song?id=347230"}"#
                    )
                    .await
                    .unwrap(),
                "null"
            );
            {
                let events = host.events.lock().unwrap();
                assert_eq!(events.len(), 1);
                assert_eq!(events[0]["event"], "song-detected");
                assert_eq!(events[0]["payload"]["title"], "Fixture song");
            }
            runtime
                .call(
                    "event:clipboard-text",
                    r#"{"text":"unrelated private text https://evil.test/song?id=347230"}"#,
                )
                .await
                .unwrap();
            assert_eq!(host.events.lock().unwrap().len(), 1);
            assert!(host.http_requests.lock().unwrap().is_empty());
            runtime
                .call(
                    "event:clipboard-text",
                    r#"{"text":"分享 https://163cn.tv/fixture"}"#,
                )
                .await
                .unwrap();
            assert_eq!(host.events.lock().unwrap().len(), 2);
            {
                let requests = host.http_requests.lock().unwrap();
                assert_eq!(requests.len(), 1);
                assert_eq!(requests[0]["url"], "https://163cn.tv/fixture");
            }
            assert!(runtime.call("unknown", "null").await.is_err());
            assert!(
                !runtime.faulted,
                "guest business errors preserve the instance"
            );
            runtime.shutdown().await;
        });
    }
    #[test]
    fn fuel_memory_trap_and_invalid_results_are_isolated() {
        run(async {
            let engine = engine().unwrap();
            for method in ["loop", "memory", "trap", "invalid-json"] {
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
    #[test]
    fn wall_timeout_cancels_async_host_wait() {
        run(async {
            let mut runtime = Runtime::load_host(
                engine().unwrap(),
                &fixture("runtime-fixture"),
                Arc::new(FakeHost::default()),
            )
            .await
            .unwrap();
            let started = std::time::Instant::now();
            assert!(runtime
                .call("host-wait", "null")
                .await
                .unwrap_err()
                .contains("超时"));
            assert!(runtime.faulted);
            assert!(started.elapsed() < Duration::from_secs(7));
        });
    }
}
