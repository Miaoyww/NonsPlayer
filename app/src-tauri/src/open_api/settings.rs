use crate::{application::Backend, model::AppResult, storage::Store};
use tauri::{Emitter, State};

pub const GLOBAL_OFFSET_KEY: &str = "lyricGlobalOffsetMs";
pub const DEFAULT_GLOBAL_OFFSET_MS: i64 = -200;
pub const MAX_GLOBAL_OFFSET_MS: i64 = 60_000;

pub fn read(store: &Store) -> AppResult<i64> {
    let Some(value) = store.setting(GLOBAL_OFFSET_KEY).inspect_err(|error| {
        crate::infrastructure::diagnostics::warn("open_api.offset.read", error);
    })?
    else {
        return Ok(DEFAULT_GLOBAL_OFFSET_MS);
    };
    let parsed = value.parse::<i64>().inspect_err(|error| {
        crate::infrastructure::diagnostics::warn("open_api.offset.parse", error);
    });
    if let Ok(value) = parsed {
        if (-MAX_GLOBAL_OFFSET_MS..=MAX_GLOBAL_OFFSET_MS).contains(&value) {
            return Ok(value);
        }
    }
    crate::infrastructure::diagnostics::warn(
        "open_api.offset.range",
        format_args!("persisted={value}"),
    );
    Ok(DEFAULT_GLOBAL_OFFSET_MS)
}

pub fn validate(value: i64) -> AppResult<()> {
    if (-MAX_GLOBAL_OFFSET_MS..=MAX_GLOBAL_OFFSET_MS).contains(&value) {
        Ok(())
    } else {
        Err("全局歌词偏移须在 -60 至 60 秒之间".into())
    }
}

pub fn save(store: &Store, value: i64) -> AppResult<i64> {
    validate(value)?;
    store
        .set_setting(GLOBAL_OFFSET_KEY, &value.to_string())
        .inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("open_api.offset.save", error);
        })?;
    Ok(value)
}

#[tauri::command]
pub async fn lyric_global_offset(
    legacy_offset_ms: Option<i64>,
    app: tauri::AppHandle,
    backend: State<'_, Backend>,
    manager: State<'_, std::sync::Arc<super::Manager>>,
) -> AppResult<i64> {
    let _guard = manager.offset_lock().await;
    let store = backend.store.clone();
    if store.setting(GLOBAL_OFFSET_KEY)?.is_some() {
        return read(&store);
    }
    let Some(legacy) = legacy_offset_ms else {
        return Ok(DEFAULT_GLOBAL_OFFSET_MS);
    };
    validate(legacy).inspect_err(|error| {
        crate::infrastructure::diagnostics::warn("open_api.offset.legacy", error);
    })?;
    let value = tauri::async_runtime::spawn_blocking(move || save(&store, legacy))
        .await
        .inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("open_api.offset.legacy.join", error);
        })
        .map_err(|error| error.to_string())??;
    if let Err(error) = app.emit("lyric-global-offset-changed", value) {
        crate::infrastructure::diagnostics::warn("open_api.offset.legacy.emit", error);
    }
    Ok(value)
}

#[tauri::command]
pub async fn set_lyric_global_offset(
    offset_ms: i64,
    app: tauri::AppHandle,
    backend: State<'_, Backend>,
    manager: State<'_, std::sync::Arc<super::Manager>>,
) -> AppResult<i64> {
    validate(offset_ms).inspect_err(|error| {
        crate::infrastructure::diagnostics::warn("open_api.offset.validate", error);
    })?;
    let _guard = manager.offset_lock().await;
    let store = backend.store.clone();
    if read(&store)? == offset_ms {
        return Ok(offset_ms);
    }
    let value = tauri::async_runtime::spawn_blocking(move || save(&store, offset_ms))
        .await
        .inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("open_api.offset.join", error);
        })
        .map_err(|error| error.to_string())??;
    if let Err(error) = app.emit("lyric-global-offset-changed", value) {
        crate::infrastructure::diagnostics::warn("open_api.offset.emit", error);
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TestDir;

    #[test]
    fn offset_roundtrips_negative_values_and_rejects_out_of_range() {
        let root = TestDir::new();
        let path = root.0.join("nons.sqlite3");
        let store = Store::open(&path).unwrap();
        assert_eq!(read(&store).unwrap(), DEFAULT_GLOBAL_OFFSET_MS);
        assert_eq!(save(&store, -60_000).unwrap(), -60_000);
        drop(store);

        let reopened = Store::open(&path).unwrap();
        assert_eq!(read(&reopened).unwrap(), -60_000);
        assert!(save(&reopened, -60_001).is_err());
        assert!(save(&reopened, 60_001).is_err());
        assert_eq!(read(&reopened).unwrap(), -60_000);
    }
}
