//! Per-track user choices are settings; lyric bodies remain in the bounded lyric cache.
use super::{qq_lyrics, LyricService};
use crate::{
    application::Backend,
    model::{AppResult, Lyrics},
    storage::MAX_LYRIC_BYTES,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::Duration;
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

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Correction {
    pub candidate: Option<Candidate>,
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

fn setting_key(key: &str) -> String {
    format!("lyricCorrectionV1:{:x}", Sha256::digest(key.as_bytes()))
}

impl LyricService {
    pub fn correction(&self, key: &str) -> AppResult<Correction> {
        self.store
            .setting(&setting_key(key))?
            .map(|value| serde_json::from_str(&value).map_err(|e| e.to_string()))
            .unwrap_or_else(|| Ok(Correction::default()))
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
            self.disk.refresh_lyrics(&[&key])?;
        }
        if let Some(cached) = self.store.cached_lyrics(&key)?.filter(|v| v.fresh) {
            return Ok(cached.value);
        }
        let generation = self.disk.generation();
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
        self.disk.cache_result(
            &key,
            value.as_ref(),
            if value.is_some() { 600 } else { 86400 },
            generation,
        )?;
        Ok(value)
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
pub fn set_lyric_correction(
    key: String,
    correction: Correction,
    app: tauri::AppHandle,
    backend: State<'_, Backend>,
) -> AppResult<()> {
    backend.store.track(&key)?;
    if correction
        .offset_ms
        .is_some_and(|offset| !(-60000..=60000).contains(&offset))
    {
        return Err("歌词偏移须在 -60 至 60 秒之间".into());
    }
    if let Some(candidate) = &correction.candidate {
        candidate.validate()?;
    }
    backend.lyrics.disk.set_lyric_setting(
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
    fn choices_are_persistent_settings_and_track_keys_are_isolated() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("library.sqlite3");
        let choice = Correction {
            candidate: Some(Candidate {
                source: Source::Qq,
                id: Some(123),
                mid: "abc123".into(),
                title: "Selected version".into(),
                artist: "Artist".into(),
                album: "Album".into(),
                duration_ms: 10000,
            }),
            offset_ms: Some(-850),
        };
        let key = setting_key("online:qq:1");
        {
            let store = Store::open(&path).unwrap();
            store
                .set_setting(&key, &serde_json::to_string(&choice).unwrap())
                .unwrap();
            store.clear_runtime_lyrics().unwrap();
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
    fn correction_offset_can_follow_global_or_override_it_with_zero() {
        let follows_global = Correction {
            candidate: None,
            offset_ms: None,
        };
        let overrides_global = Correction {
            candidate: None,
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
}
