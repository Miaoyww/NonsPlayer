//! Per-track user choices are settings; lyric bodies remain in the bounded lyric cache.
use super::{qq_lyrics, LyricService};
use crate::{
    application::Backend,
    model::{AppResult, Lyrics},
    storage::{Store, MAX_LYRIC_BYTES},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{path::Path, time::Duration};
use tauri::{Emitter, State};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Candidate {
    pub source: Source,
    pub id: Option<u64>,
    pub mid: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: u64,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum Source {
    Qq,
    Netease,
}

impl Source {
    pub(super) fn select(self, sources: &mut super::LyricSources, skips: &mut super::LyricSkips) {
        sources.amll = false;
        sources.qq = self == Source::Qq;
        skips.amll = true;
        skips.qq = self != Source::Qq;
        skips.netease = self != Source::Netease;
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LocalTtml {
    pub path: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Correction {
    #[serde(default)]
    pub source: Option<Source>,
    #[serde(default)]
    pub candidate: Option<Candidate>,
    #[serde(default)]
    pub local_ttml: Option<LocalTtml>,
    #[serde(default)]
    pub offset_ms: Option<i32>,
}

impl Candidate {
    pub(super) fn validate(&self) -> AppResult<()> {
        if self.id == Some(0)
            || self.mid.len() > 128
            || !self.mid.bytes().all(|c| c.is_ascii_alphanumeric())
            || [&self.title, &self.artist, &self.album]
                .iter()
                .any(|v| v.len() > 1024)
            || (self.source == Source::Netease && self.id.is_none())
            || (self.source == Source::Qq && self.id.is_none() && self.mid.is_empty())
        {
            return Err("歌词候选信息无效".into());
        }
        Ok(())
    }
}

impl LocalTtml {
    fn validate(&self) -> AppResult<()> {
        if self.path.is_empty()
            || self.path.len() > 32768
            || !Path::new(&self.path)
                .extension()
                .is_some_and(|extension| extension.to_string_lossy().eq_ignore_ascii_case("ttml"))
        {
            return Err("只能选择 TTML 歌词文件".into());
        }
        Ok(())
    }
}

impl Correction {
    pub(super) fn validate(&self) -> AppResult<()> {
        if self.source.is_some() && (self.candidate.is_some() || self.local_ttml.is_some()) {
            return Err("歌词来源选择无效".into());
        }
        if self.candidate.is_some() && self.local_ttml.is_some() {
            return Err("歌词来源选择无效".into());
        }
        if let Some(candidate) = &self.candidate {
            candidate.validate()?;
        }
        if let Some(local) = &self.local_ttml {
            local.validate()?;
        }
        Ok(())
    }
}

fn setting_key(key: &str) -> String {
    format!("lyricCorrectionV1:{:x}", Sha256::digest(key.as_bytes()))
}

fn read_correction(store: &Store, key: &str) -> AppResult<Correction> {
    store
        .setting(&setting_key(key))?
        .map(|value| serde_json::from_str(&value).map_err(|e| e.to_string()))
        .unwrap_or_else(|| Ok(Correction::default()))
}

fn save_offset(store: &Store, key: &str, offset_ms: i32) -> AppResult<()> {
    if !(-60000..=60000).contains(&offset_ms) {
        return Err("歌词偏移须在 -60 至 60 秒之间".into());
    }
    let mut correction = read_correction(store, key)?;
    correction.offset_ms = Some(offset_ms);
    store.set_setting(
        &setting_key(key),
        &serde_json::to_string(&correction).map_err(|e| e.to_string())?,
    )
}

impl LyricService {
    pub fn set_offset(&self, key: &str, offset_ms: i32) -> AppResult<()> {
        save_offset(&self.store, key, offset_ms)
    }

    pub fn correction(&self, key: &str) -> AppResult<Correction> {
        read_correction(&self.store, key)
    }

    pub async fn candidate_lyrics(
        &self,
        candidate: &Candidate,
        skip_qrc: bool,
        refresh: bool,
    ) -> AppResult<Option<Lyrics>> {
        candidate.validate()?;
        let key = format!(
            "manual:v1:{}:{skip_qrc}",
            serde_json::to_string(candidate).map_err(|e| e.to_string())?
        );
        if refresh {
            self.cache.refresh_lyrics(&[&key])?;
        }
        if let Some(cached) = self.cache.get(&key)?.filter(|v| v.fresh) {
            return Ok(cached.value);
        }
        let generation = self.cache.generation();
        let _permit = self
            .correction_requests
            .try_acquire()
            .map_err(|_| "歌词查询繁忙，请稍后重试")?;
        let value = tokio::time::timeout(Duration::from_secs(6), async {
            match candidate.source {
                Source::Qq => qq_lyrics::fetch_manual(&self.client, candidate, skip_qrc).await,
                Source::Netease => {
                    self.netease
                        .lyrics(candidate.id.ok_or("缺少网易云歌曲 ID")?)
                        .await
                }
            }
        })
        .await
        .map_err(|_| "歌词查询超时")??;
        if let Some(value) = &value {
            if value.content.len()
                + value.translation.as_ref().map_or(0, String::len)
                + value.romanization.as_ref().map_or(0, String::len)
                > MAX_LYRIC_BYTES
            {
                return Err("歌词内容过大".into());
            }
        }
        // A failure is never cached as absence. Explicit selections never fall back to another recording.
        self.cache.cache_result(
            &key,
            value.as_ref(),
            value.is_none().then_some(86400),
            generation,
        )?;
        Ok(value)
    }

    pub async fn local_ttml_lyrics(&self, local: &LocalTtml) -> AppResult<Lyrics> {
        local.validate()?;
        super::local_lyrics::read_ttml(local.path.clone()).await
    }
}

#[tauri::command]
pub fn lyric_correction(key: String, backend: State<'_, Backend>) -> AppResult<Correction> {
    backend.store.track(&key)?;
    backend.lyrics.correction(&key)
}

#[tauri::command]
pub async fn lyric_candidates(
    key: String,
    source: Source,
    query: String,
    backend: State<'_, Backend>,
) -> AppResult<Vec<Candidate>> {
    backend.store.track(&key)?;
    if query.trim().is_empty() || query.len() > 256 {
        return Err("请输入 1–256 字节的搜索词".into());
    }
    let _permit = backend
        .lyrics
        .correction_requests
        .try_acquire()
        .map_err(|_| "歌词查询繁忙，请稍后重试")?;
    tokio::time::timeout(Duration::from_secs(6), async {
        match source {
            Source::Qq => qq_lyrics::search_manual(&backend.lyrics.client, query.trim()).await,
            Source::Netease => Ok(backend
                .lyrics
                .netease
                .search(query.trim(), 0)
                .await?
                .into_iter()
                .take(20)
                .filter_map(|track| {
                    Some(Candidate {
                        source,
                        id: Some(track.netease_id()?),
                        mid: String::new(),
                        title: track.title,
                        artist: track.artist,
                        album: track.album,
                        duration_ms: track.duration_ms,
                    })
                })
                .collect()),
        }
    })
    .await
    .map_err(|_| "歌词搜索超时")?
}

#[tauri::command]
pub async fn preview_lyric_candidate(
    key: String,
    candidate: Candidate,
    skip_qrc: Option<bool>,
    backend: State<'_, Backend>,
) -> AppResult<Option<Lyrics>> {
    backend.store.track(&key)?;
    backend
        .lyrics
        .candidate_lyrics(&candidate, skip_qrc.unwrap_or(false), false)
        .await
}

#[tauri::command]
pub async fn pick_local_ttml(app: tauri::AppHandle) -> AppResult<Option<String>> {
    #[cfg(desktop)]
    {
        use tauri_plugin_dialog::DialogExt;

        let selected = tauri::async_runtime::spawn_blocking(move || {
            app.dialog()
                .file()
                .set_title("选择本地 TTML 歌词")
                .add_filter("TTML 歌词", &["ttml"])
                .blocking_pick_file()
        })
        .await
        .map_err(|e| e.to_string())?;
        selected
            .map(|path| {
                path.into_path()
                    .map(|value| value.to_string_lossy().into_owned())
                    .map_err(|e| e.to_string())
            })
            .transpose()
    }
    #[cfg(not(desktop))]
    {
        let _ = app;
        Err("当前平台不支持选择本地 TTML 歌词".into())
    }
}

#[tauri::command]
pub async fn preview_local_ttml(path: String) -> AppResult<Lyrics> {
    super::local_lyrics::read_ttml(path).await
}

#[tauri::command]
pub fn set_lyric_offset(
    key: String,
    offset_ms: i32,
    app: tauri::AppHandle,
    backend: State<'_, Backend>,
) -> AppResult<()> {
    backend.store.track(&key)?;
    backend.lyrics.set_offset(&key, offset_ms)?;
    app.emit(
        "lyric-offset-changed",
        serde_json::json!({ "key": key, "offsetMs": offset_ms }),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_lyric_correction(
    key: String,
    correction: Correction,
    app: tauri::AppHandle,
    backend: State<'_, Backend>,
) -> AppResult<()> {
    backend.store.track(&key)?;
    correction.validate()?;
    if correction
        .offset_ms
        .is_some_and(|offset| !(-60000..=60000).contains(&offset))
    {
        return Err("歌词偏移须在 -60 至 60 秒之间".into());
    }
    backend.lyrics.cache.set_lyric_setting(
        &setting_key(&key),
        &serde_json::to_string(&correction).map_err(|e| e.to_string())?,
    )?;
    app.emit("lyric-correction-changed", &key)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Store;
    #[test]
    fn offset_updates_preserve_selection_cache_and_other_tracks() {
        use super::super::lyric_cache::LyricCache;
        use std::sync::Arc;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("library.sqlite3");
        {
            let store = Arc::new(Store::open(&path).unwrap());
            let cache = LyricCache::new(store.clone()).unwrap();
            let generation = cache.generation();
            cache
                .cache_result("cached-lyrics", None, None, generation)
                .unwrap();
            let choice = Correction {
                source: Some(Source::Qq),
                ..Default::default()
            };
            store
                .set_setting(
                    &setting_key("song:1"),
                    &serde_json::to_string(&choice).unwrap(),
                )
                .unwrap();
            for offset in [100, 0, -100, -60000, 60000] {
                save_offset(&store, "song:1", offset).unwrap();
                let saved = read_correction(&store, "song:1").unwrap();
                assert_eq!(saved.source, choice.source);
                assert_eq!(saved.offset_ms, Some(offset));
            }
            assert!(save_offset(&store, "song:1", 60001).is_err());
            assert!(save_offset(&store, "song:1", -60001).is_err());
            assert_eq!(cache.generation(), generation);
            assert!(cache.get("cached-lyrics").unwrap().is_some());
            assert_eq!(
                read_correction(&store, "song:2").unwrap(),
                Correction::default()
            );
        }
        let store = Store::open(&path).unwrap();
        assert_eq!(
            read_correction(&store, "song:1").unwrap().offset_ms,
            Some(60000)
        );
    }

    #[test]
    fn choices_are_persistent_settings_and_track_keys_are_isolated() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("library.sqlite3");
        let choice = Correction {
            source: None,
            candidate: Some(Candidate {
                source: Source::Qq,
                id: Some(123),
                mid: "abc123".into(),
                title: "Selected version".into(),
                artist: "Artist".into(),
                album: "Album".into(),
                duration_ms: 10000,
            }),
            local_ttml: None,
            offset_ms: Some(-850),
        };
        let key = setting_key("online:qq:1");
        {
            let store = Store::open(&path).unwrap();
            store
                .set_setting(&key, &serde_json::to_string(&choice).unwrap())
                .unwrap();
            store
                .clear_lyric_cache(crate::storage::LyricCacheScope::Session)
                .unwrap();
        }
        let store = Store::open(&path).unwrap();
        assert_eq!(
            serde_json::from_str::<Correction>(&store.setting(&key).unwrap().unwrap()).unwrap(),
            choice
        );
        assert!(store
            .setting(&setting_key("online:netease:1"))
            .unwrap()
            .is_none());
    }
    #[test]
    fn correction_offset_preserves_zero_and_an_unset_adjustment() {
        let follows_global = Correction {
            source: None,
            candidate: None,
            local_ttml: None,
            offset_ms: None,
        };
        let overrides_global = Correction {
            source: None,
            candidate: None,
            local_ttml: None,
            offset_ms: Some(0),
        };
        assert_eq!(
            serde_json::from_str::<Correction>(r#"{"candidate":null}"#).unwrap(),
            follows_global
        );
        assert_ne!(follows_global, overrides_global);
        assert_eq!(overrides_global.offset_ms, Some(0));
    }
    #[test]
    fn source_selection_is_persisted_without_binding_a_candidate() {
        let correction = Correction {
            source: Some(Source::Netease),
            offset_ms: Some(250),
            ..Default::default()
        };
        assert!(correction.validate().is_ok());
        let decoded: Correction =
            serde_json::from_str(&serde_json::to_string(&correction).unwrap()).unwrap();
        assert_eq!(decoded, correction);
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("library.sqlite3");
        let key = setting_key("online:netease:1");
        {
            let store = Store::open(&path).unwrap();
            store
                .set_setting(&key, &serde_json::to_string(&correction).unwrap())
                .unwrap();
        }
        let store = Store::open(&path).unwrap();
        assert_eq!(
            serde_json::from_str::<Correction>(&store.setting(&key).unwrap().unwrap()).unwrap(),
            correction
        );
        assert!(store
            .setting(&setting_key("online:qq:1"))
            .unwrap()
            .is_none());
        assert!(Correction {
            source: Some(Source::Qq),
            candidate: Some(Candidate {
                source: Source::Qq,
                id: Some(1),
                mid: "abc".into(),
                title: "Song".into(),
                artist: "Artist".into(),
                album: "Album".into(),
                duration_ms: 1000,
            }),
            ..Default::default()
        }
        .validate()
        .is_err());
    }
    #[test]
    fn selecting_a_platform_limits_matching_to_that_platform_and_keeps_qrc_retry() {
        for selected in [Source::Qq, Source::Netease] {
            let mut sources = super::super::LyricSources {
                qq: false,
                ..Default::default()
            };
            let mut skips = super::super::LyricSkips {
                amll: false,
                qq: true,
                netease: false,
                qrc: true,
            };
            selected.select(&mut sources, &mut skips);
            assert!(!sources.amll);
            assert!(skips.amll);
            assert_eq!(sources.qq, selected == Source::Qq);
            assert_eq!(skips.qq, selected != Source::Qq);
            assert_eq!(skips.netease, selected != Source::Netease);
            assert!(skips.qrc);
        }
    }
    #[test]
    fn manual_candidates_reject_invalid_ids_and_unbounded_metadata() {
        let mut c = Candidate {
            source: Source::Qq,
            id: None,
            mid: "abc123".into(),
            title: "Song".into(),
            artist: "Artist".into(),
            album: "Album".into(),
            duration_ms: 1000,
        };
        assert!(c.validate().is_ok());
        c.mid = "../bad".into();
        assert!(c.validate().is_err());
        c.mid.clear();
        assert!(c.validate().is_err());
        c.id = Some(1);
        c.source = Source::Netease;
        assert!(c.validate().is_ok());
        c.title = "a".repeat(1025);
        assert!(c.validate().is_err());
    }

    #[test]
    fn local_ttml_paths_and_exclusive_selection_are_validated() {
        let local = LocalTtml {
            path: "C:\\Lyrics\\song.ttml".into(),
        };
        assert!(local.validate().is_ok());
        assert!(LocalTtml {
            path: "C:\\Lyrics\\song.lrc".into()
        }
        .validate()
        .is_err());
        assert!(LocalTtml {
            path: String::new()
        }
        .validate()
        .is_err());
        assert!(Correction {
            source: None,
            candidate: Some(Candidate {
                source: Source::Qq,
                id: Some(1),
                mid: "abc".into(),
                title: "Song".into(),
                artist: "Artist".into(),
                album: "Album".into(),
                duration_ms: 1000,
            }),
            local_ttml: Some(local),
            offset_ms: None,
        }
        .validate()
        .is_err());
    }
}
