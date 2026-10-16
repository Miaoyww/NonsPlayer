//! Explicit live validation, separate from deterministic regression tests.
use super::*;
use crate::music::{
    accounts::{AccountManager, CredentialStore},
    manager::AdapterManager,
};
use std::{collections::BTreeMap, sync::Mutex, time::Instant};

#[derive(Default)]
struct Secrets(Mutex<BTreeMap<String, Vec<u8>>>);

#[tokio::test]
#[ignore = "Requires built GD Studio Component; deterministic Host, no network or credentials"]
async fn gdstudio_component_resolves_under_native_host_budget() {
    use crate::wasm_runtime::{host_io, Budget, HostHandler, Runtime};
    struct AudioHost;
    impl HostHandler for AudioHost {
        fn system_capabilities(&self) -> bool {
            true
        }
        fn http_request(
            &self,
            request: host_io::Request,
        ) -> Pin<Box<dyn Future<Output = AppResult<host_io::Response>> + Send + '_>> {
            Box::pin(async move {
                if request.url == "https://music.gdstudio.org/time" {
                    return Ok(host_io::Response {
                        status: 200,
                        headers: vec![],
                        body: b"1792158848".to_vec(),
                    });
                }
                if request
                    .url
                    .starts_with("https://music.gdstudio.org/api.php")
                {
                    assert!(!request.headers.iter().any(|h| h.name == "X-Request-Key"));
                    return Ok(host_io::Response {
                        status: 200,
                        headers: vec![],
                        body: br#"{"code":0,"url":"https://m1.music.126.net/test.mp3"}"#.to_vec(),
                    });
                }
                assert!(!request.headers.iter().any(|h| h.name == "X-Request-Key"));
                let total = 6890 * 1044;
                let offset = request
                    .headers
                    .iter()
                    .find_map(|h| h.value.strip_prefix("bytes="))
                    .and_then(|r| r.split('-').next())
                    .and_then(|v| v.parse::<usize>().ok())
                    .unwrap();
                let (range, body) = if offset > 0 {
                    (
                        format!("bytes {}-{}/{total}", offset, total - 1),
                        vec![0; total - offset],
                    )
                } else {
                    let mut body = vec![0; 65536];
                    for start in (0..64500).step_by(1044) {
                        body[start..start + 4].copy_from_slice(&[255, 251, 224, 0]);
                    }
                    (format!("bytes 0-65535/{total}"), body)
                };
                Ok(host_io::Response {
                    status: 206,
                    headers: vec![host_io::Header {
                        name: "content-range".into(),
                        value: range,
                    }],
                    body,
                })
            })
        }
        fn call<'a>(
            &'a self,
            _: &'a str,
            _: &'a str,
        ) -> Pin<Box<dyn Future<Output = AppResult<String>> + Send + 'a>> {
            Box::pin(async { Ok("null".into()) })
        }
    }
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../adapters/gdstudio/dist/adapter.component.wasm");
    let mut runtime = Runtime::load_host_with_budget(
        runtime::engine_with_budget(Budget::ADAPTER).unwrap(),
        &path,
        Arc::new(AudioHost),
        Budget::ADAPTER,
    )
    .await
    .unwrap();
    let descriptor: serde_json::Value = serde_json::from_str(
        &runtime
            .call("music.resource-descriptor", "null")
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(descriptor["data"]["id"], "gdstudio");
    let response = runtime.call("music.resolve-resource", r#"{"request":{"track":{"reference":{"source":"netease","kind":"track","id":"opaque:1"},"durationMs":180000},"preferredQuality":"exhigh","allowDowngrade":false}}"#).await.unwrap();
    let response: serde_json::Value = serde_json::from_str(&response).unwrap();
    assert_eq!(response["status"], "ok", "{response}");
    assert_eq!(response["data"]["metadata"]["actualQuality"], "320k");
    assert_eq!(response["data"]["metadata"]["extent"]["kind"], "full");
    assert_eq!(response["data"]["headers"], serde_json::json!([]));
}

// Runs the composed interpreter under the same fuel budget as a real request.
// The JS/Node Component harness does not meter Wasmtime fuel.
#[tokio::test]
#[ignore = "Requires built QQ Component; deterministic Host, no network or credentials"]
async fn qq_component_large_playlist_respects_page_and_execution_budget() {
    use crate::wasm_runtime::{host_io, Budget, HostHandler, Runtime};
    use std::{future::Future, pin::Pin};
    struct PlaylistHost;
    impl HostHandler for PlaylistHost {
        fn system_capabilities(&self) -> bool {
            true
        }
        fn credential_read(&self) -> Pin<Box<dyn Future<Output = AppResult<Vec<u8>>> + Send + '_>> {
            Box::pin(async { Ok(Vec::new()) })
        }
        fn http_request(
            &self,
            request: host_io::Request,
        ) -> Pin<Box<dyn Future<Output = AppResult<host_io::Response>> + Send + '_>> {
            Box::pin(async move {
                let song = serde_json::json!({ "id": 42, "mid": "SongMID", "title": "预算测试曲目", "interval": 123, "singer": [{ "mid": "ArtistMID", "name": "测试歌手" }], "album": { "mid": "AlbumMID", "name": "测试专辑" }, "action": { "switch": 1 }, "file": { "media_mid": "AudioMID" } });
                let payload = if request.url.contains("fcg_ucc_getcdinfo_byids_cp") {
                    serde_json::json!({ "code": 0, "cdlist": [{ "disstid": "123", "dissname": "大歌单", "songlist": vec![song; 4500], "total_song_num": 4500 }] })
                } else {
                    let request: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
                    assert_eq!(request["request"]["method"], "uniform_get_Dissinfo");
                    assert_eq!(request["request"]["param"]["song_num"], 25);
                    serde_json::json!({ "code": 0, "request": { "code": 0, "data": { "dirinfo": { "dissname": "大歌单" }, "songlist": vec![song; 25], "total_song_num": 4500 } } })
                };
                let body = serde_json::to_vec(&payload).unwrap();
                Ok(host_io::Response {
                    status: 200,
                    headers: vec![host_io::Header {
                        name: "content-type".into(),
                        value: "application/json".into(),
                    }],
                    body,
                })
            })
        }
        fn call<'a>(
            &'a self,
            _: &'a str,
            _: &'a str,
        ) -> Pin<Box<dyn Future<Output = AppResult<String>> + Send + 'a>> {
            Box::pin(async { Ok("null".into()) })
        }
    }
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../adapters/qqmusic/dist/adapter.component.wasm");
    let mut runtime = Runtime::load_host_with_budget(
        runtime::engine_with_budget(Budget::ADAPTER).unwrap(),
        &path,
        Arc::new(PlaylistHost),
        Budget::ADAPTER,
    )
    .await
    .unwrap();
    let started = Instant::now();
    let response = runtime.call("music.business", r#"{"session":{"source":"qqmusic"},"request":{"operation":"tracks","entity":{"source":"qqmusic","kind":"playlist","id":"123"},"page":{"limit":25,"cursor":null}}}"#).await;
    eprintln!(
        "large playlist elapsed_ms={}",
        started.elapsed().as_millis()
    );
    let response: serde_json::Value = serde_json::from_str(
        &response.expect("A page of a large playlist must fit the normal adapter execution budget"),
    )
    .unwrap();
    assert_eq!(response["status"], "ok", "{response}");
    assert_eq!(
        response["data"]["data"]["items"].as_array().unwrap().len(),
        25
    );
    assert_eq!(response["data"]["data"]["total"], 4500);
    assert!(response["data"]["data"]["nextCursor"].is_string());
}
impl CredentialStore for Secrets {
    fn read(&self, key: &str) -> MusicResult<Option<Vec<u8>>> {
        Ok(self.0.lock().unwrap().get(key).cloned())
    }
    fn write(&self, key: &str, data: &[u8]) -> MusicResult<()> {
        self.0.lock().unwrap().insert(key.into(), data.into());
        Ok(())
    }
    fn delete(&self, key: &str) -> MusicResult<()> {
        self.0.lock().unwrap().remove(key);
        Ok(())
    }
}

#[tokio::test]
#[ignore = "Requires built XiaoBai Component and live public APIs; isolated accounts"]
async fn xiaobai_component_live_lemon() {
    let provider = "xiaobai";
    use crate::music::provider::ResourceProviderDescriptor;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let package = root.join(format!("adapters/{provider}"));
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(package.join("manifest.json")).unwrap()).unwrap();
    let descriptor: ResourceProviderDescriptor =
        serde_json::from_value(manifest["resourceProvider"].clone()).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let store = Arc::new(
        crate::storage::Store::open(&directory.path().join("acceptance.sqlite3")).unwrap(),
    );
    let accounts =
        Arc::new(AccountManager::new(store.clone(), Arc::new(Secrets::default())).unwrap());
    let (music, _) = crate::music::bundled::service(accounts.clone()).unwrap();
    let services = Arc::new(
        host::Services::new(
            accounts,
            Arc::downgrade(&music.adapters),
            store,
            Arc::new(crate::infrastructure::http::Http::new().unwrap()),
            serde_json::from_value(manifest["permissions"].clone()).unwrap(),
            SourceDescriptor {
                source: descriptor.id.clone(),
                display_name: descriptor.display_name.clone(),
                description: None,
                platforms: Default::default(),
                contract_version: descriptor.contract_version,
                login_methods: vec![],
                capabilities: Default::default(),
                cover_hosts: descriptor.cover_hosts.clone(),
                share_link_templates: Default::default(),
            },
        )
        .unwrap(),
    );
    let adapter = ResourceComponent::load(
        &package.join("dist/adapter.component.wasm"),
        descriptor.clone(),
        services,
    )
    .await
    .unwrap();
    let adapter = Arc::new(adapter);
    music
        .adapters
        .register_resource_provider(adapter.clone())
        .unwrap();
    use crate::music::provider::{ResourceProvider, ResourceRequest};
    let track: MusicTrack = serde_json::from_value(serde_json::json!({
        "reference":{"source":"qqmusic","kind":"track","id":"000akynZ2Rbro5"},
        "title":"Lemon","aliases":[],"artist":"米津玄師","artists":[],"album":"Lemon","albumReference":null,"durationMs":256000,"cover":"","associations":[]
    }))
    .unwrap();
    let mut request = ResourceRequest {
        track,
        preferred_quality: "hires".into(),
        allow_downgrade: true,
    };
    let (ticket, source) = music
        .adapters
        .resolve_with_providers(&request, Some(provider), 1)
        .await
        .unwrap();
    assert_eq!(source.provider, provider);
    assert_eq!(source.platform.as_str(), "qqmusic");
    assert_eq!(source.track.id.as_str(), "000akynZ2Rbro5");
    assert_eq!(ticket.quality(), "320k");
    assert!(ticket.check(1).is_ok());
    println!(
        "Website live Lemon: provider={} platform={} actual_quality={}",
        source.provider,
        source.platform.as_str(),
        ticket.quality()
    );
    let (session, adapter_generation) = music.adapters.stamp(&descriptor.id).unwrap();
    let context = RequestContext {
        session,
        adapter_generation,
        deadline: Instant::now() + std::time::Duration::from_secs(12),
        cancellation: Default::default(),
    };
    request.allow_downgrade = false;
    let error = match adapter.resolve(&request, &context).await {
        Ok(_) => panic!("lossy audio must not satisfy Hi-Res without downgrade"),
        Err(error) => error,
    };
    assert_eq!(error.code, ErrorCode::PermissionDenied);
}

#[tokio::test]
#[ignore = "Requires built GD Studio Component and live public APIs; isolated accounts"]
async fn gdstudio_component_live_resource() {
    use crate::music::provider::ResourceProviderDescriptor;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let package = root.join("adapters/gdstudio");
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(package.join("manifest.json")).unwrap()).unwrap();
    let descriptor: ResourceProviderDescriptor =
        serde_json::from_value(manifest["resourceProvider"].clone()).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let store = Arc::new(
        crate::storage::Store::open(&directory.path().join("acceptance.sqlite3")).unwrap(),
    );
    let accounts =
        Arc::new(AccountManager::new(store.clone(), Arc::new(Secrets::default())).unwrap());
    let (music, _) = crate::music::bundled::service(accounts.clone()).unwrap();
    let services = Arc::new(
        host::Services::new(
            accounts,
            Arc::downgrade(&music.adapters),
            store,
            Arc::new(crate::infrastructure::http::Http::new().unwrap()),
            serde_json::from_value(manifest["permissions"].clone()).unwrap(),
            SourceDescriptor {
                source: descriptor.id.clone(),
                display_name: descriptor.display_name.clone(),
                description: None,
                platforms: Default::default(),
                contract_version: descriptor.contract_version,
                login_methods: vec![],
                capabilities: Default::default(),
                cover_hosts: descriptor.cover_hosts.clone(),
                share_link_templates: Default::default(),
            },
        )
        .unwrap(),
    );
    let adapter = ResourceComponent::load(
        &package.join("dist/adapter.component.wasm"),
        descriptor.clone(),
        services,
    )
    .await
    .unwrap();
    music
        .adapters
        .register_resource_provider(Arc::new(adapter))
        .unwrap();
    // Read known platform metadata directly; GD never performs a search.
    let reference = serde_json::from_value(
        serde_json::json!({"source":"netease","kind":"track","id":"347230"}),
    )
    .unwrap();
    let metadata = music.adapters.read_track(&reference).await.unwrap();
    let track = crate::model::Track::from_music(metadata).unwrap();
    let (resolved, ticket, source) = music
        .resolve_selected(
            track,
            "exhigh",
            true,
            1,
            Some(crate::music::service::PlaybackSelection {
                provider: Some("gdstudio".into()),
                track: None,
            }),
        )
        .await
        .unwrap();
    assert!(resolved.track.duration_ms > 0);
    let ticket = ticket.unwrap();
    let source = source.unwrap();
    assert_eq!(source.provider, "gdstudio");
    assert_eq!(source.platform.as_str(), "netease");
    assert!(ticket.check(1).is_ok());
    println!(
        "GD Studio live: provider={} quality={}",
        source.provider,
        ticket.quality()
    );
}

#[tokio::test]
#[ignore = "Requires built QQ Component and direct public internet; optional QR scan requires a human"]
async fn qq_component_live_search_and_optional_login() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let package = root.join("adapters/qqmusic");
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(package.join("manifest.json")).unwrap()).unwrap();
    let descriptor: SourceDescriptor =
        serde_json::from_value(manifest["descriptor"].clone()).unwrap();
    let source = descriptor.source.clone();
    let directory = tempfile::tempdir().unwrap();
    runtime::configure_compilation_cache(&directory.path().join("cache"));
    let store = Arc::new(
        crate::storage::Store::open(&directory.path().join("acceptance.sqlite3")).unwrap(),
    );
    let accounts =
        Arc::new(AccountManager::new(store.clone(), Arc::new(Secrets::default())).unwrap());
    let manager = Arc::new(AdapterManager::default());
    accounts.bind(&manager);
    let services = Arc::new(
        host::Services::new(
            accounts.clone(),
            Arc::downgrade(&manager),
            store,
            Arc::new(crate::infrastructure::http::Http::new().unwrap()),
            serde_json::from_value(manifest["permissions"].clone()).unwrap(),
            descriptor,
        )
        .unwrap(),
    );
    let started = Instant::now();
    let adapter = Arc::new(
        ExternalAdapter::load_with_host(
            &package.join("dist/adapter.component.wasm"),
            services.clone(),
        )
        .await
        .unwrap(),
    );
    eprintln!(
        "Component load: {:.1} ms",
        started.elapsed().as_secs_f64() * 1000.0
    );
    let started = Instant::now();
    let warm =
        ExternalAdapter::load_with_host(&package.join("dist/adapter.component.wasm"), services)
            .await
            .unwrap();
    eprintln!(
        "Component cached load: {:.1} ms",
        started.elapsed().as_secs_f64() * 1000.0
    );
    drop(warm);
    let current = accounts.clone();
    let current_source = source.clone();
    manager
        .register(
            adapter.clone(),
            Arc::new(move || current.session(&current_source)),
        )
        .unwrap();
    let started = Instant::now();
    let response = manager
        .business(
            &source,
            &BusinessRequest::Search {
                keyword: "晴天".into(),
                kind: EntityKind::Track,
                page: PageRequest {
                    cursor: None,
                    limit: 5,
                },
            },
        )
        .await
        .unwrap();
    let BusinessResponse::Tracks(page) = response else {
        panic!("Expected tracks")
    };
    assert!(!page.page.items.is_empty());
    eprintln!(
        "Search: {:.1} ms; tracks: {}",
        started.elapsed().as_secs_f64() * 1000.0,
        page.page.items.len()
    );
    let track = manager
        .read_track(&page.page.items[0].reference)
        .await
        .unwrap();
    assert_eq!(track.reference, page.page.items[0].reference);
    let methods = std::env::var("NONS_QQ_QR_METHOD").unwrap_or_default();
    for method in methods.split(',').filter(|value| !value.is_empty()) {
        let challenge = manager
            .account(
                &source,
                &AccountRequest::BeginLogin {
                    method: Some(method.to_owned()),
                },
                &accounts,
            )
            .await
            .unwrap();
        let AccountPresentation::Challenge { key, image } = challenge else {
            panic!("Expected challenge")
        };
        let report = root.join(".local/qq-acceptance");
        std::fs::create_dir_all(&report).unwrap();
        if let Some(data) = image.strip_prefix("data:image/png;base64,") {
            use base64::Engine;
            std::fs::write(
                report.join(format!("{method}.png")),
                base64::engine::general_purpose::STANDARD
                    .decode(data)
                    .unwrap(),
            )
            .unwrap();
        }
        std::fs::write(
            report.join(format!("{method}-challenge.json")),
            serde_json::to_vec(&serde_json::json!({"image":image})).unwrap(),
        )
        .unwrap();
        eprintln!("QR ready ({method}); waiting up to 180 seconds for phone confirmation");
        let deadline = Instant::now() + std::time::Duration::from_secs(180);
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            assert!(
                Instant::now() < deadline,
                "Human scan did not complete within challenge lifetime"
            );
            let progress = manager
                .account(
                    &source,
                    &AccountRequest::PollLogin { key: key.clone() },
                    &accounts,
                )
                .await
                .unwrap();
            let AccountPresentation::Progress { code, .. } = progress else {
                panic!("Expected progress")
            };
            assert_ne!(code, 800, "Login challenge failed or expired");
            if code == 803 {
                break;
            }
        }
        assert!(accounts.session(&source).account.is_some());
        assert!(adapter.sessions.is_empty());
        eprintln!("AccountManager accepted login and challenge resources were released");
    }
    manager.set_enabled(&source, false).unwrap();
}
