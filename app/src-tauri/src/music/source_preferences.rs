//! Device-owned source roles, independent of accounts, adapter lifecycle and playback.
use super::{identity::SourceId, manager::AdapterManager};
use crate::{model::AppResult, storage::Store};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

const KEY: &str = "musicSourcePreferencesV1";
const MAX_SUPPLEMENTARY: usize = 16;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourcePreferences {
    pub current: SourceId,
    pub supplementary: Vec<SourceId>,
    pub revision: u64,
}

pub struct SourceSettings {
    store: Arc<Store>,
    adapters: Arc<AdapterManager>,
    state: Mutex<SourcePreferences>,
    intent: Mutex<u64>,
}

impl SourceSettings {
    pub fn new(store: Arc<Store>, adapters: Arc<AdapterManager>) -> AppResult<Self> {
        let state = match store.setting(KEY)? {
            Some(value) => {
                serde_json::from_str::<SourcePreferences>(&value).map_err(|_| "音乐来源配置损坏")?
            }
            None => SourcePreferences {
                current: SourceId::try_from("netease".to_owned()).map_err(|e| e.to_string())?,
                supplementary: vec![],
                revision: 0,
            },
        };
        Self::validate_shape(&state.current, &state.supplementary)?;
        // Retain missing/disabled saved sources so startup never silently changes the workspace.
        Ok(Self {
            store,
            adapters,
            state: Mutex::new(state),
            intent: Mutex::new(0),
        })
    }

    fn validate_shape(current: &SourceId, supplementary: &[SourceId]) -> AppResult<()> {
        if current.as_str() == "local"
            || supplementary.len() > MAX_SUPPLEMENTARY
            || supplementary.iter().enumerate().any(|(index, source)| {
                source == current
                    || source.as_str() == "local"
                    || supplementary[..index].contains(source)
            })
        {
            return Err("补充来源须唯一且不能包含当前来源或本地音乐，最多 16 个".into());
        }
        Ok(())
    }

    pub fn get(&self) -> AppResult<SourcePreferences> {
        Ok(self
            .state
            .lock()
            .map_err(|_| "音乐来源配置锁不可用")?
            .clone())
    }

    pub fn update(
        &self,
        current: SourceId,
        supplementary: Vec<SourceId>,
        revision: u64,
    ) -> AppResult<SourcePreferences> {
        self.update_intent(current, supplementary, revision, None)
    }

    pub fn begin_intent(&self) -> AppResult<u64> {
        let mut intent = self.intent.lock().map_err(|_| "来源操作锁不可用")?;
        *intent = intent.checked_add(1).ok_or("来源操作代次已耗尽")?;
        Ok(*intent)
    }

    pub fn use_source(&self, current: SourceId, intent: u64) -> AppResult<SourcePreferences> {
        if !self
            .adapters
            .statuses()
            .map_err(|e| e.to_string())?
            .iter()
            .any(|status| status.enabled && status.descriptor.source == current)
        {
            return Err("当前来源不存在或已停用".into());
        }
        let saved = self.get()?;
        let mut supplementary = saved.supplementary.clone();
        if saved.current != current && supplementary.contains(&current) {
            supplementary.retain(|source| source != &current);
            supplementary.insert(0, saved.current);
        }
        self.update_intent(current, supplementary, saved.revision, Some(intent))
    }

    fn update_intent(
        &self,
        current: SourceId,
        supplementary: Vec<SourceId>,
        revision: u64,
        expected_intent: Option<u64>,
    ) -> AppResult<SourcePreferences> {
        // Keep intent checking and publication in one critical section. A delayed login
        // cannot win after a newer login, saved-account selection or explicit switch.
        let mut intent = self.intent.lock().map_err(|_| "来源操作锁不可用")?;
        if expected_intent.is_some_and(|expected| expected != *intent) {
            return Err("来源操作已被更新的操作取代".into());
        }
        let next_intent = intent.checked_add(1).ok_or("来源操作代次已耗尽")?;
        let mut state = self.state.lock().map_err(|_| "音乐来源配置锁不可用")?;
        if state.revision != revision {
            return Err("音乐来源配置已变化，请刷新后重试".into());
        }
        Self::validate_shape(&current, &supplementary)?;
        let statuses = self.adapters.statuses().map_err(|e| e.to_string())?;
        // Existing unavailable preferences can be retained while repairing another role.
        for source in std::iter::once(&current).chain(supplementary.iter()) {
            let retained = source == &state.current || state.supplementary.contains(source);
            if !retained
                && !statuses
                    .iter()
                    .any(|item| item.enabled && &item.descriptor.source == source)
            {
                return Err("音乐来源不存在或已停用".into());
            }
        }
        if (expected_intent.is_some() || current != state.current)
            && !statuses
                .iter()
                .any(|item| item.enabled && item.descriptor.source == current)
        {
            return Err("当前来源不存在或已停用".into());
        }
        if state.current == current && state.supplementary == supplementary {
            *intent = next_intent;
            return Ok(state.clone());
        }
        let next = SourcePreferences {
            current,
            supplementary,
            revision: state
                .revision
                .checked_add(1)
                .ok_or("音乐来源配置修订号已耗尽")?,
        };
        self.store.set_setting(
            KEY,
            &serde_json::to_string(&next).map_err(|e| e.to_string())?,
        )?;
        *state = next.clone();
        *intent = next_intent;
        Ok(next)
    }
}

#[tauri::command]
pub fn music_source_intent(settings: tauri::State<'_, Arc<SourceSettings>>) -> AppResult<u64> {
    settings.begin_intent()
}

#[tauri::command]
pub async fn music_use_source(
    current: SourceId,
    intent: u64,
    settings: tauri::State<'_, Arc<SourceSettings>>,
    app: tauri::AppHandle,
) -> AppResult<SourcePreferences> {
    use tauri::Emitter;
    let settings = settings.inner().clone();
    let next = tauri::async_runtime::spawn_blocking(move || settings.use_source(current, intent))
        .await
        .map_err(|_| "来源切换任务失败")??;
    let _ = app.emit("music-source-preferences-changed", &next);
    Ok(next)
}

#[tauri::command]
pub fn music_source_preferences(
    settings: tauri::State<'_, Arc<SourceSettings>>,
) -> AppResult<SourcePreferences> {
    settings.get()
}

#[tauri::command]
pub async fn music_set_source_preferences(
    current: SourceId,
    supplementary: Vec<SourceId>,
    revision: u64,
    settings: tauri::State<'_, Arc<SourceSettings>>,
    app: tauri::AppHandle,
) -> AppResult<SourcePreferences> {
    use tauri::Emitter;
    let settings = settings.inner().clone();
    let next = tauri::async_runtime::spawn_blocking(move || {
        settings.update(current, supplementary, revision)
    })
    .await
    .map_err(|_| "音乐来源配置任务失败")??;
    let _ = app.emit("music-source-preferences-changed", &next);
    Ok(next)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::music::account::SessionContext;
    fn source(value: &str) -> SourceId {
        SourceId::try_from(value.to_owned()).unwrap()
    }
    fn settings(store: Arc<Store>) -> SourceSettings {
        let adapters = Arc::new(AdapterManager::default());
        adapters
            .register(
                Arc::new(nons_adapter_netease::NeteaseAdapter::new(Arc::new(
                    nons_adapter_netease::platform::Netease::with_cookie(None),
                ))),
                Arc::new(|| SessionContext {
                    source: source("netease"),
                    account: None,
                    generation: 0,
                }),
            )
            .unwrap();
        SourceSettings::new(store, adapters).unwrap()
    }
    #[test]
    fn restored_roles_keep_missing_source_order_and_revision() {
        let directory = tempfile::tempdir().unwrap();
        let store = Arc::new(Store::open(&directory.path().join("settings.sqlite3")).unwrap());
        let saved = SourcePreferences {
            current: source("qqmusic"),
            supplementary: vec![source("netease"), source("missing")],
            revision: 7,
        };
        store
            .set_setting(KEY, &serde_json::to_string(&saved).unwrap())
            .unwrap();
        let settings = settings(store.clone());
        assert_eq!(settings.get().unwrap(), saved);
        let next = settings
            .update(
                source("netease"),
                vec![source("qqmusic"), source("missing")],
                7,
            )
            .unwrap();
        assert_eq!(next.revision, 8);
        assert_eq!(
            SourceSettings::new(store, settings.adapters.clone())
                .unwrap()
                .get()
                .unwrap(),
            next
        );
        assert!(settings.update(source("netease"), vec![], 7).is_err());
        assert_eq!(settings.get().unwrap(), next);
    }
    #[test]
    fn invalid_roles_and_disk_failure_do_not_publish() {
        let directory = tempfile::tempdir().unwrap();
        let store = Arc::new(Store::open(&directory.path().join("settings.sqlite3")).unwrap());
        let saved = SourcePreferences {
            current: source("missing"),
            supplementary: vec![],
            revision: 0,
        };
        store
            .set_setting(KEY, &serde_json::to_string(&saved).unwrap())
            .unwrap();
        let settings = settings(store.clone());
        assert!(settings
            .update(source("netease"), vec![source("netease")], 0)
            .is_err());
        assert!(settings
            .update(source("netease"), vec![source("unknown")], 0)
            .is_err());
        assert!(settings.update(source("local"), vec![], 0).is_err());
        settings
            .adapters
            .set_enabled(&source("netease"), false)
            .unwrap();
        assert!(settings.update(source("netease"), vec![], 0).is_err());
        settings
            .adapters
            .set_enabled(&source("netease"), true)
            .unwrap();
        store
            .0
            .lock()
            .unwrap()
            .execute_batch("PRAGMA query_only = ON")
            .unwrap();
        assert!(settings.update(source("netease"), vec![], 0).is_err());
        assert_eq!(settings.get().unwrap(), saved);
    }
    #[test]
    fn newer_intent_and_noop_explicit_switch_reject_old_login() {
        let directory = tempfile::tempdir().unwrap();
        let store = Arc::new(Store::open(&directory.path().join("settings.sqlite3")).unwrap());
        let settings = settings(store);
        let old = settings.begin_intent().unwrap();
        let latest = settings.begin_intent().unwrap();
        assert!(settings.use_source(source("netease"), old).is_err());
        assert!(settings.use_source(source("netease"), latest).is_ok());
        assert!(settings.use_source(source("netease"), latest).is_err());
        let pending = settings.begin_intent().unwrap();
        settings.update(source("netease"), vec![], 0).unwrap();
        assert!(settings.use_source(source("netease"), pending).is_err());
        let pending = settings.begin_intent().unwrap();
        settings
            .adapters
            .set_enabled(&source("netease"), false)
            .unwrap();
        assert!(settings.use_source(source("netease"), pending).is_err());
        assert_eq!(settings.get().unwrap().revision, 0);
    }
}
