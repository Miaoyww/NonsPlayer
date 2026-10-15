//! Explicit live validation, separate from deterministic regression tests.
use super::*;
use crate::music::{
    accounts::{AccountManager, CredentialStore},
    manager::AdapterManager,
};
use std::{collections::BTreeMap, sync::Mutex, time::Instant};

#[derive(Default)]
struct Secrets(Mutex<BTreeMap<String, Vec<u8>>>);
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
        ExternalAdapter::load_with_host(&package.join("dist/adapter.component.wasm"), services)
            .await
            .unwrap(),
    );
    eprintln!(
        "Component load: {:.1} ms",
        started.elapsed().as_secs_f64() * 1000.0
    );
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
