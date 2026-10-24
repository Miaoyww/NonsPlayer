//! Bundled native package composition. Platform protocols are owned by adapters/netease.
use super::{
    account::*, accounts::AccountManager, adapter::*, identity::*, manager::AdapterManager,
    service::MusicService, ErrorCode, MusicResult,
};
use std::sync::{Arc, Weak};

pub(super) fn default_source() -> SourceId {
    SourceId::try_from("netease".to_owned()).expect("bundled source identifier is valid")
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ComponentPackage {
    descriptor: Option<SourceDescriptor>,
    resource_provider: Option<super::provider::ResourceProviderDescriptor>,
    permissions: super::external::host::Permissions,
}
pub(crate) struct PreparedComponent {
    directory: std::path::PathBuf,
    package: ComponentPackage,
}

/// Only trusted application resources are discovered here. Installing external packages
/// and authorizing their permissions is a separate lifecycle, not directory discovery.
pub(crate) fn prepare_components(
    directory: &std::path::Path,
    adapters: &AdapterManager,
) -> crate::model::AppResult<Vec<PreparedComponent>> {
    log::info!(target: "nons::adapter", "load_components_start directory={}", directory.display());
    let packages = std::fs::read_dir(directory)
        .inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("translate_error", error);
        })
        .map_err(|_| "随包适配器目录不可读")?;
    let mut prepared = Vec::new();
    for package in packages {
        let package = package
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("translate_error", error);
            })
            .map_err(|_| "随包适配器目录项不可读")?;
        if !package
            .file_type()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("translate_error", error);
            })
            .map_err(|_| "随包适配器类型不可读")?
            .is_dir()
        {
            continue;
        }
        let directory = package.path();
        use std::io::Read;
        let mut bytes = Vec::new();
        std::fs::File::open(directory.join("manifest.json"))
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("translate_error", error);
            })
            .map_err(|_| "随包适配器描述缺失")?
            .take(16385)
            .read_to_end(&mut bytes)
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("translate_error", error);
            })
            .map_err(|_| "随包适配器描述不可读")?;
        if bytes.len() > 16384 {
            return Err("随包适配器描述过大".into());
        }
        let package: ComponentPackage = serde_json::from_slice(&bytes)
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("translate_error", error);
            })
            .map_err(|_| "随包适配器描述无效")?;
        match (&package.descriptor, &package.resource_provider) {
            (Some(descriptor), None) => descriptor.validate(),
            (None, Some(descriptor)) => descriptor.validate(),
            _ => Err(ErrorCode::InvalidData.into()),
        }
        .map_err(|e| e.to_string())?;
        prepared.push(PreparedComponent { directory, package });
    }
    for item in &prepared {
        if let Some(descriptor) = &item.package.descriptor {
            adapters
                .begin_loading(descriptor.clone())
                .map_err(|e| e.to_string())?;
        }
        if let Some(descriptor) = &item.package.resource_provider {
            adapters
                .begin_resource_loading(descriptor.clone())
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(prepared)
}
pub(crate) async fn load_components(
    packages: Vec<PreparedComponent>,
    music: &Arc<MusicService>,
    store: Arc<crate::storage::Store>,
    http: Arc<crate::infrastructure::http::Http>,
    settings: &super::settings::AdapterSettings,
) -> crate::model::AppResult<()> {
    let accounts = music.accounts.clone().ok_or("账号服务未就绪")?;
    let mut failures = Vec::new();
    for PreparedComponent { directory, package } in packages {
        if let Some(descriptor) = package.resource_provider {
            let result: crate::model::AppResult<()> = async {
                // This descriptor binds only the generic Host identity. It is never
                // registered as a catalog source, account source or workspace choice.
                let host_identity = SourceDescriptor {
                    source: descriptor.id.clone(),
                    display_name: descriptor.display_name.clone(),
                    description: descriptor.description.clone(),
                    platforms: Default::default(),
                    contract_version: descriptor.contract_version,
                    login_methods: vec![],
                    capabilities: Default::default(),
                    cover_hosts: descriptor.cover_hosts.clone(),
                    share_link_templates: Default::default(),
                };
                let services = Arc::new(super::external::host::Services::new(
                    accounts.clone(),
                    Arc::downgrade(&music.adapters),
                    store.clone(),
                    http.clone(),
                    package.permissions,
                    host_identity,
                )?);
                let adapter = super::external::ResourceComponent::load(
                    &directory.join("adapter.component.wasm"),
                    descriptor.clone(),
                    services,
                )
                .await
                .map_err(|e| e.to_string())?;
                let enabled = store
                    .setting(&format!(
                        "resourceProviderEnabledV1:{}",
                        descriptor.id.as_str()
                    ))?
                    .as_deref()
                    != Some("false");
                music
                    .adapters
                    .register_resource_provider_with_enabled(Arc::new(adapter), enabled)
                    .map_err(|e| e.to_string())?;
                Ok(())
            }
            .await;
            if let Err(error) = result {
                music
                    .adapters
                    .resource_loading_failed(&descriptor.id, &error);
                failures.push(format!(
                    "音源 {} 加载失败：{}",
                    descriptor.display_name, error
                ));
            }
            continue;
        }
        let descriptor = package.descriptor.ok_or("适配器描述缺失")?;
        let source = descriptor.source.clone();
        let started = std::time::Instant::now();
        log::info!(target: "nons::adapter", "source={} stage=load_component state=start", source.as_str());
        let result: crate::model::AppResult<()> = async {
            let services = Arc::new(super::external::host::Services::new(
                accounts.clone(),
                Arc::downgrade(&music.adapters),
                store.clone(),
                http.clone(),
                package.permissions,
                descriptor,
            )?);
            let adapter = super::external::ExternalAdapter::load_with_host(
                &directory.join("adapter.component.wasm"),
                services,
            )
            .await
            .map_err(|e| format!("随包适配器 {} 加载失败：{}", source.as_str(), e))?;
            let sessions = accounts.clone();
            let session_source = source.clone();
            music
                .adapters
                .register(
                    Arc::new(adapter),
                    Arc::new(move || sessions.session(&session_source)),
                )
                .map_err(|e| e.to_string())?;
            settings.restore_enabled(&source)?;
            Ok(())
        }
        .await;
        if let Err(error) = result {
            music.adapters.loading_failed(&source, &error);
            failures.push(error);
            continue;
        }
        log::info!(target: "nons::adapter", "source={} stage=register state=ready elapsed_ms={}", source.as_str(), started.elapsed().as_millis());
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("；"))
    }
}

struct HostAccounts {
    accounts: Arc<AccountManager>,
    adapters: Weak<AdapterManager>,
    source: SourceId,
}
impl ProviderAccounts for HostAccounts {
    fn session(&self) -> SessionContext {
        self.accounts.session(&self.source)
    }
    fn credential(&self) -> MusicResult<Option<OpaqueCredential>> {
        let adapters = self
            .adapters
            .upgrade()
            .ok_or(ErrorCode::SourceUnavailable)?;
        let expected = adapters.stamp(&self.source)?;
        let result = self.accounts.provider_credential(&self.source)?;
        if adapters.stamp(&self.source)? != expected {
            return Err(ErrorCode::StaleContext.into());
        }
        Ok(result)
    }
    fn read(&self, context: &RequestContext) -> MusicResult<Option<OpaqueCredential>> {
        if context.session.source != self.source {
            return Err(ErrorCode::PermissionDenied.into());
        }
        let adapters = self
            .adapters
            .upgrade()
            .ok_or(ErrorCode::SourceUnavailable)?;
        let (session, generation) = adapters.stamp(&self.source)?;
        context.check(&session, generation)?;
        let result = if context.session.account.is_some() {
            Some(self.accounts.scoped_access(context)?.read(context)?)
        } else {
            self.accounts.provider_credential(&self.source)?
        };
        let (session, generation) = adapters.stamp(&self.source)?;
        context.check(&session, generation)?;
        Ok(result)
    }
    fn invalidate<'a>(&'a self, expected: SessionContext) -> AdapterFuture<'a, ()> {
        Box::pin(async move {
            if expected.source != self.source {
                return Err(ErrorCode::PermissionDenied.into());
            }
            self.accounts
                .blocking(move |accounts| accounts.invalidate(&expected))
                .await
        })
    }
}

pub(crate) fn service(
    accounts: Arc<AccountManager>,
) -> crate::model::AppResult<(MusicService, Arc<nons_adapter_netease::platform::Netease>)> {
    let adapters = Arc::new(AdapterManager::default());
    accounts.bind(&adapters);
    let bridge = Arc::new(HostAccounts {
        accounts: accounts.clone(),
        adapters: Arc::downgrade(&adapters),
        source: SourceId::try_from("netease".to_owned()).unwrap(),
    });
    let client = Arc::new(nons_adapter_netease::platform::Netease::with_accounts(
        bridge,
    ));
    let session_client = client.clone();
    adapters
        .register(
            Arc::new(nons_adapter_netease::NeteaseAdapter::new(client.clone())),
            Arc::new(move || nons_adapter_netease::session(&session_client)),
        )
        .map_err(|e| e.to_string())?;
    let mut music = MusicService::new(adapters);
    music.accounts = Some(accounts);
    Ok((music, client))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::music::accounts::CredentialStore;
    use std::collections::BTreeMap;
    #[derive(Default)]
    struct Secrets(std::sync::Mutex<BTreeMap<String, Vec<u8>>>);
    #[tokio::test]
    async fn discovered_package_stays_visible_when_component_loading_fails() {
        let directory = tempfile::tempdir().unwrap();
        let store = Arc::new(
            crate::storage::Store::open(&directory.path().join("accounts.sqlite3")).unwrap(),
        );
        let accounts =
            Arc::new(AccountManager::new(store.clone(), Arc::new(Secrets::default())).unwrap());
        let (music, client) = service(accounts).unwrap();
        let music = Arc::new(music);
        let settings =
            super::super::settings::AdapterSettings::new(store.clone(), music.adapters.clone())
                .unwrap();
        let mut descriptor = nons_adapter_netease::NeteaseAdapter::new(client)
            .descriptor()
            .clone();
        descriptor.source = SourceId::try_from("pending-fixture".to_owned()).unwrap();
        let packages = directory.path().join("packages");
        let package = packages.join("fixture");
        std::fs::create_dir_all(&package).unwrap();
        std::fs::write(package.join("manifest.json"), serde_json::to_vec(&serde_json::json!({ "descriptor": descriptor, "permissions": { "httpHosts": [], "websocketHosts": [], "system": false, "state": false, "credentials": false } })).unwrap()).unwrap();
        let prepared = prepare_components(&packages, &music.adapters).unwrap();
        let pending = settings
            .list()
            .unwrap()
            .into_iter()
            .find(|status| status.descriptor.source == descriptor.source)
            .unwrap();
        assert!(pending.loading && !pending.enabled && pending.load_error.is_none());
        assert!(music.adapters.stamp(&descriptor.source).is_err());
        assert!(load_components(
            prepared,
            &music,
            store,
            Arc::new(crate::infrastructure::http::Http::new().unwrap()),
            &settings
        )
        .await
        .is_err());
        let failed = settings
            .list()
            .unwrap()
            .into_iter()
            .find(|status| status.descriptor.source == descriptor.source)
            .unwrap();
        assert!(!failed.loading && !failed.enabled && failed.load_error.is_some());
    }
    #[tokio::test]
    #[ignore = "Requires the built bundled QQ Component; no credentials or network needed"]
    async fn built_qq_component_registers_from_the_real_startup_directory() {
        let directory = tempfile::tempdir().unwrap();
        let store = Arc::new(
            crate::storage::Store::open(&directory.path().join("accounts.sqlite3")).unwrap(),
        );
        let accounts =
            Arc::new(AccountManager::new(store.clone(), Arc::new(Secrets::default())).unwrap());
        let (music, _) = service(accounts).unwrap();
        let music = Arc::new(music);
        let settings =
            super::super::settings::AdapterSettings::new(store.clone(), music.adapters.clone())
                .unwrap();
        let packages = prepare_components(
            &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bundled-adapters"),
            &music.adapters,
        )
        .unwrap();
        assert!(music
            .adapters
            .statuses()
            .unwrap()
            .iter()
            .any(|status| status.descriptor.source.as_str() == "qqmusic"
                && status.loading
                && !status.enabled));
        load_components(
            packages,
            &music,
            store,
            Arc::new(crate::infrastructure::http::Http::new().unwrap()),
            &settings,
        )
        .await
        .expect("QQ 音乐必须经真实启动加载路径注册");
        assert!(music
            .adapters
            .statuses()
            .unwrap()
            .iter()
            .any(|status| status.descriptor.source.as_str() == "qqmusic"));
        let qq = SourceId::try_from("qqmusic".to_owned()).unwrap();
        settings.set_enabled(qq.clone(), false).unwrap();
        assert!(settings
            .list()
            .unwrap()
            .iter()
            .any(|status| status.descriptor.source == qq && !status.enabled));
        settings.set_enabled(qq.clone(), true).unwrap();
        settings.reload(qq.clone()).unwrap();
        assert!(settings
            .list()
            .unwrap()
            .iter()
            .any(|status| status.descriptor.source == qq && status.enabled));
    }
    impl CredentialStore for Secrets {
        fn read(&self, key: &str) -> MusicResult<Option<Vec<u8>>> {
            Ok(self.0.lock().unwrap().get(key).cloned())
        }
        fn write(&self, key: &str, bytes: &[u8]) -> MusicResult<()> {
            self.0.lock().unwrap().insert(key.into(), bytes.into());
            Ok(())
        }
        fn delete(&self, key: &str) -> MusicResult<()> {
            self.0.lock().unwrap().remove(key);
            Ok(())
        }
    }
    #[test]
    fn native_package_bridge_revokes_credentials_and_preserves_accounts() {
        let directory = tempfile::tempdir().unwrap();
        let store = Arc::new(
            crate::storage::Store::open(&directory.path().join("accounts.sqlite3")).unwrap(),
        );
        let accounts = Arc::new(AccountManager::new(store, Arc::new(Secrets::default())).unwrap());
        let source = SourceId::try_from("netease".to_owned()).unwrap();
        let record = AccountRecord {
            reference: AccountRef {
                source: source.clone(),
                id: OpaqueId::try_from("42".to_owned()).unwrap(),
            },
            display_name: "Listener".into(),
            avatar: Default::default(),
        };
        accounts
            .accept(
                &accounts.session(&source),
                record.clone(),
                OpaqueCredential::new(b"test-cookie".to_vec()).unwrap(),
            )
            .unwrap();
        let (service, client) = service(accounts.clone()).unwrap();
        let bridge = HostAccounts {
            accounts: accounts.clone(),
            adapters: Arc::downgrade(&service.adapters),
            source: source.clone(),
        };
        let (session, generation) = service.adapters.stamp(&source).unwrap();
        let mut context = RequestContext {
            session,
            adapter_generation: generation,
            deadline: std::time::Instant::now() + std::time::Duration::from_secs(10),
            cancellation: Cancellation::default(),
        };
        assert_eq!(
            bridge.read(&context).unwrap().unwrap().expose(),
            b"test-cookie"
        );
        context.session.source = SourceId::try_from("other".to_owned()).unwrap();
        assert_eq!(
            bridge.read(&context).err().unwrap().code,
            ErrorCode::PermissionDenied
        );
        context.session.source = source.clone();
        service.adapters.reload(&source).unwrap();
        assert_eq!(
            bridge.read(&context).err().unwrap().code,
            ErrorCode::StaleContext
        );
        assert_eq!(
            client.account_credentials().unwrap().as_deref(),
            Some("test-cookie")
        );
        service.adapters.set_enabled(&source, false).unwrap();
        assert!(client.account_credentials().is_err());
        assert_eq!(
            bridge.read(&context).err().unwrap().code,
            ErrorCode::SourceUnavailable
        );
        assert_eq!(accounts.records(&source).unwrap(), vec![record]);
        service.adapters.set_enabled(&source, true).unwrap();
        assert_eq!(
            client.account_credentials().unwrap().as_deref(),
            Some("test-cookie")
        );
    }
}
