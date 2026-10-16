//! Device adapter preferences; lifecycle changes are serialized and never query cached.
use super::{
    identity::SourceId,
    manager::{AdapterManager, AdapterStatus},
};
use crate::{model::AppResult, storage::Store};
use std::sync::{Arc, Mutex};

pub struct AdapterSettings {
    store: Arc<Store>,
    adapters: Arc<AdapterManager>,
    mutation: Mutex<()>,
}
impl AdapterSettings {
    pub(crate) fn set_resource_enabled(&self, id: SourceId, enabled: bool) -> AppResult<()> {
        let _guard = self
            .mutation
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("translate_error", error);
            })
            .map_err(|_| "适配器设置锁不可用")?;
        if !self
            .adapters
            .resource_provider_statuses()
            .map_err(|e| e.to_string())?
            .iter()
            .any(|s| s.descriptor.id == id && !s.loading && s.load_error.is_none())
        {
            return Err("音源不存在".into());
        }
        self.store.set_setting(
            &format!("resourceProviderEnabledV1:{}", id.as_str()),
            if enabled { "true" } else { "false" },
        )?;
        self.adapters
            .set_resource_provider_enabled(&id, enabled)
            .map_err(|e| e.to_string())
    }
    pub fn new(store: Arc<Store>, adapters: Arc<AdapterManager>) -> AppResult<Self> {
        let settings = Self {
            store,
            adapters,
            mutation: Mutex::new(()),
        };
        for status in settings.list()? {
            if settings
                .store
                .setting(&Self::key(&status.descriptor.source))?
                .as_deref()
                == Some("false")
            {
                settings
                    .adapters
                    .set_enabled(&status.descriptor.source, false)
                    .map_err(|e| e.to_string())?;
            }
        }
        Ok(settings)
    }
    fn key(source: &SourceId) -> String {
        format!("musicAdapterEnabledV1:{}", source.as_str())
    }
    pub(crate) fn restore_enabled(&self, source: &SourceId) -> AppResult<()> {
        let _guard = self
            .mutation
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("translate_error", error);
            })
            .map_err(|_| "适配器设置锁不可用")?;
        let enabled = self.store.setting(&Self::key(source))?.as_deref() != Some("false");
        self.adapters
            .set_enabled(source, enabled)
            .map_err(|e| e.to_string())
    }
    pub fn list(&self) -> AppResult<Vec<AdapterStatus>> {
        self.adapters.statuses().map_err(|e| e.to_string())
    }
    pub fn set_enabled(&self, source: SourceId, enabled: bool) -> AppResult<()> {
        let _guard = self
            .mutation
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("translate_error", error);
            })
            .map_err(|_| "适配器设置锁不可用")?;
        if !self.list()?.iter().any(|s| s.descriptor.source == source) {
            return Err("音乐来源不存在".into());
        }
        // Persist first: failed disk writes cannot silently change the running source.
        self.store
            .set_setting(&Self::key(&source), if enabled { "true" } else { "false" })?;
        self.adapters
            .set_enabled(&source, enabled)
            .map_err(|e| e.to_string())
    }
    pub fn reload(&self, source: SourceId) -> AppResult<()> {
        let _guard = self
            .mutation
            .lock()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("translate_error", error);
            })
            .map_err(|_| "适配器设置锁不可用")?;
        self.adapters.stamp(&source).map_err(|e| e.to_string())?;
        self.adapters.reload(&source).map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub async fn adapter_list(
    settings: tauri::State<'_, Arc<AdapterSettings>>,
) -> AppResult<Vec<AdapterStatus>> {
    settings.list()
}

#[tauri::command]
pub async fn adapter_set_enabled(
    source: SourceId,
    enabled: bool,
    settings: tauri::State<'_, Arc<AdapterSettings>>,
    app: tauri::AppHandle,
) -> AppResult<()> {
    let settings = settings.inner().clone();
    let changed = source.clone();
    tauri::async_runtime::spawn_blocking(move || settings.set_enabled(source, enabled))
        .await
        .inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("translate_error", error);
        })
        .map_err(|_| "适配器设置任务失败")??;
    notify(&app, &changed);
    Ok(())
}

#[tauri::command]
pub async fn adapter_reload(
    source: SourceId,
    settings: tauri::State<'_, Arc<AdapterSettings>>,
    app: tauri::AppHandle,
) -> AppResult<()> {
    let settings = settings.inner().clone();
    let changed = source.clone();
    tauri::async_runtime::spawn_blocking(move || settings.reload(source))
        .await
        .inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("translate_error", error);
        })
        .map_err(|_| "适配器设置任务失败")??;
    notify(&app, &changed);
    Ok(())
}

fn notify(app: &tauri::AppHandle, source: &SourceId) {
    use tauri::Emitter;
    let _ = (app.emit("adapters-changed", source)).inspect_err(|error| {
        crate::infrastructure::diagnostics::warn("app.emit", error);
    });
    let _ = (app.emit("music-changed", source)).inspect_err(|error| {
        crate::infrastructure::diagnostics::warn("app.emit", error);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::music::account::SessionContext;
    fn source() -> SourceId {
        SourceId::try_from("netease".to_owned()).unwrap()
    }
    fn manager() -> Arc<AdapterManager> {
        let manager = Arc::new(AdapterManager::default());
        manager
            .register(
                Arc::new(nons_adapter_netease::NeteaseAdapter::new(Arc::new(
                    nons_adapter_netease::platform::Netease::with_cookie(None),
                ))),
                Arc::new(|| SessionContext {
                    source: source(),
                    account: None,
                    generation: 0,
                }),
            )
            .unwrap();
        manager
    }
    #[test]
    fn disabled_preference_survives_restart_and_reload_does_not_enable() {
        let directory = tempfile::tempdir().unwrap();
        let store = Arc::new(Store::open(&directory.path().join("settings.sqlite3")).unwrap());
        let adapters = manager();
        let settings = AdapterSettings::new(store.clone(), adapters.clone()).unwrap();
        let original_scope = adapters.cache_identity(&source()).unwrap();
        settings.set_enabled(source(), false).unwrap();
        assert!(!settings.list().unwrap()[0].enabled);
        assert!(adapters.cache_identity(&source()).is_err());
        assert!(settings.reload(source()).is_err());
        let restored = AdapterSettings::new(store, manager()).unwrap();
        assert!(!restored.list().unwrap()[0].enabled);
        settings.set_enabled(source(), true).unwrap();
        assert_ne!(adapters.cache_identity(&source()).unwrap(), original_scope);
        let enabled_scope = adapters.cache_identity(&source()).unwrap();
        settings.reload(source()).unwrap();
        assert_ne!(adapters.cache_identity(&source()).unwrap(), enabled_scope);
        assert!(settings.list().unwrap()[0].enabled);
    }
    #[test]
    fn unknown_source_cannot_publish_a_preference_or_change_registered_source() {
        let directory = tempfile::tempdir().unwrap();
        let store = Arc::new(Store::open(&directory.path().join("settings.sqlite3")).unwrap());
        let settings = AdapterSettings::new(store.clone(), manager()).unwrap();
        let unknown = SourceId::try_from("unknown".to_owned()).unwrap();
        assert!(settings.set_enabled(unknown.clone(), false).is_err());
        assert!(store
            .setting(&AdapterSettings::key(&unknown))
            .unwrap()
            .is_none());
        assert!(settings.list().unwrap()[0].enabled);
    }
    #[test]
    fn persistence_failure_keeps_running_source_enabled() {
        let directory = tempfile::tempdir().unwrap();
        let store = Arc::new(Store::open(&directory.path().join("settings.sqlite3")).unwrap());
        let settings = AdapterSettings::new(store.clone(), manager()).unwrap();
        store
            .0
            .lock()
            .unwrap()
            .execute_batch("PRAGMA query_only = ON")
            .unwrap();
        assert!(settings.set_enabled(source(), false).is_err());
        assert!(settings.list().unwrap()[0].enabled);
    }
}
