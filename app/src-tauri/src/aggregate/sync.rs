use super::store::{self, Link};
use crate::{
    model::{AppResult, Track},
    music::{
        adapter::PageRequest,
        business::{BusinessRequest, BusinessResponse},
        identity::{EntityKind, EntityRef},
        service::MusicService,
    },
    storage::now_seconds,
    Backend,
};
use serde::Serialize;
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
    time::Duration,
};
use tauri::State;
use tokio::sync::{watch, Semaphore};
#[cfg(test)]
#[path = "sync_tests.rs"]
mod tests;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub key: String,
    pub running: bool,
    pub count: usize,
    pub playlist_id: Option<String>,
    pub error: Option<String>,
}
pub(crate) struct Job {
    pub cancel: watch::Sender<bool>,
    progress: Mutex<Progress>,
}
pub struct SyncJobs {
    pub(crate) active: Mutex<HashMap<String, Arc<Job>>>,
    slots: Arc<Semaphore>,
}
impl Default for SyncJobs {
    fn default() -> Self {
        Self {
            active: Mutex::new(HashMap::new()),
            slots: Arc::new(Semaphore::new(2)),
        }
    }
}
fn scope(music: &MusicService, link: &Link, expected: &str) -> AppResult<()> {
    let current = music
        .accounts
        .as_ref()
        .ok_or("账号管理器不可用")?
        .session(&link.reference.source);
    if current.account.as_ref() != Some(&link.account.id) {
        return Err("原关联账号未选中，同步已暂停".into());
    }
    if music
        .adapters
        .cache_identity(&link.reference.source)
        .map_err(|e| e.to_string())?
        != expected
    {
        return Err("账号或来源已变化，同步已取消".into());
    }
    Ok(())
}
async fn owned(music: &MusicService, link: &Link, expected: &str) -> AppResult<String> {
    let mut cursor = None;
    let mut seen = HashSet::new();
    for _ in 0..1000 {
        scope(music, link, expected)?;
        let request = BusinessRequest::LibraryCollections {
            kind: EntityKind::Playlist,
            filter: "created".into(),
            page: PageRequest {
                cursor: cursor.clone(),
                limit: 30,
            },
        };
        let BusinessResponse::Entities(page) = music
            .adapters
            .business(&link.reference.source, &request)
            .await
            .map_err(|e| e.to_string())?
        else {
            return Err("自建歌单响应无效".into());
        };
        scope(music, link, expected)?;
        // Both the authenticated created-list and stable creator identity must agree.
        if let Some(item) = page.items.into_iter().find(|p| {
            p.reference == link.reference && p.creator.as_ref() == Some(&link.account) && !p.liked
        }) {
            return Ok(item.name);
        }
        cursor = page.next_cursor;
        match &cursor {
            None => break,
            Some(c) => {
                if !seen.insert(serde_json::to_string(c).map_err(|e| e.to_string())?) {
                    return Err("自建歌单游标重复".into());
                }
            }
        }
    }
    Err("原歌单不可访问或不属于关联账号，同步已暂停".into())
}
async fn read(
    music: &MusicService,
    link: &Link,
    expected: &str,
    job: &Job,
) -> AppResult<(String, Vec<Track>)> {
    owned(music, link, expected).await?;
    let mut cursor = None;
    let mut seen = HashSet::new();
    let mut keys = HashSet::new();
    let mut tracks = Vec::new();
    for _ in 0..1000 {
        if *job.cancel.borrow() {
            return Err("同步已取消".into());
        }
        scope(music, link, expected)?;
        let request = BusinessRequest::Tracks {
            entity: link.reference.clone(),
            page: PageRequest {
                cursor: cursor.clone(),
                limit: 100,
            },
        };
        let BusinessResponse::Tracks(page) = music
            .adapters
            .business(&link.reference.source, &request)
            .await
            .map_err(|e| e.to_string())?
        else {
            return Err("歌单分页响应无效".into());
        };
        scope(music, link, expected)?;
        if *job.cancel.borrow() {
            return Err("同步已取消".into());
        }
        for value in page.page.items {
            let track = Track::from_music(value).map_err(|e| e.to_string())?;
            if keys.insert(track.key.clone()) {
                tracks.push(track);
                if tracks.len() > store::MAX_TRACKS {
                    return Err("原歌单超过 10000 首，未应用同步".into());
                }
            }
        }
        if let Ok(mut status) = job.progress.lock() {
            status.count = tracks.len()
        }
        cursor = page.page.next_cursor;
        match &cursor {
            None => {
                let title = owned(music, link, expected).await?;
                return Ok((title, tracks));
            }
            Some(c) => {
                if !seen.insert(serde_json::to_string(c).map_err(|e| e.to_string())?) {
                    return Err("歌单分页游标重复".into());
                }
            }
        }
    }
    Err("歌单分页超过安全上限，未应用同步".into())
}
#[tauri::command]
pub async fn aggregate_sync(
    id: Option<String>,
    reference: Option<EntityRef>,
    force: Option<bool>,
    app: tauri::AppHandle,
    backend: State<'_, Backend>,
) -> AppResult<Progress> {
    let store = backend.store.clone();
    let music = backend.music.clone();
    let jobs = backend.aggregate.clone();
    let existing = if let Some(id) = id.clone() {
        let s = store.clone();
        Some(
            tauri::async_runtime::spawn_blocking(move || store::playlist(&s, &id))
                .await
                .map_err(|e| e.to_string())??,
        )
    } else {
        None
    };
    let link = if let Some(p) = &existing {
        p.link.clone().ok_or("这个歌单未关联平台")?
    } else {
        let reference = reference.ok_or("缺少原歌单")?;
        if reference.kind != EntityKind::Playlist {
            return Err("只能派生歌单".into());
        }
        let account = music
            .accounts
            .as_ref()
            .ok_or("账号管理器不可用")?
            .current_record(&reference.source)
            .map_err(|e| e.to_string())?
            .ok_or("请先登录原歌单所属账号")?
            .reference;
        Link {
            reference,
            account,
            last_success: 0,
            last_attempt: 0,
            error: None,
        }
    };
    let found = if existing.is_none() {
        let s = store.clone();
        let all = tauri::async_runtime::spawn_blocking(move || store::playlists(&s))
            .await
            .map_err(|e| e.to_string())??;
        all.into_iter().find(|p| {
            p.link
                .as_ref()
                .is_some_and(|l| l.reference == link.reference && l.account == link.account)
        })
    } else {
        existing
    };
    let id = found.as_ref().map(|p| p.id.clone());
    let key = id.clone().unwrap_or(
        serde_json::to_string(&(&link.account, &link.reference)).map_err(|e| e.to_string())?,
    );
    let expected = music
        .adapters
        .cache_identity(&link.reference.source)
        .map_err(|e| e.to_string())?;
    scope(&music, &link, &expected)?;
    // Joining an active task takes priority over automatic retry throttling.
    if let Some(job) = jobs.active.lock().map_err(|_| "同步状态不可用")?.get(&key) {
        let status = job.progress.lock().map_err(|_| "同步状态不可用")?;
        if status.running {
            return Ok(status.clone());
        }
    }
    let now = now_seconds();
    if !force.unwrap_or(false)
        && found
            .as_ref()
            .and_then(|p| p.link.as_ref())
            .is_some_and(|l| now - l.last_success < 600 || now - l.last_attempt < 30)
    {
        return Ok(Progress {
            key,
            running: false,
            count: 0,
            playlist_id: id,
            error: None,
        });
    }
    let (cancel, mut cancelled) = watch::channel(false);
    let progress = Progress {
        key: key.clone(),
        running: true,
        count: 0,
        playlist_id: id.clone(),
        error: None,
    };
    let job = Arc::new(Job {
        cancel,
        progress: Mutex::new(progress.clone()),
    });
    {
        let mut active = jobs.active.lock().map_err(|_| "同步状态不可用")?;
        if let Some(old) = active.get(&key) {
            let p = old.progress.lock().map_err(|_| "同步状态不可用")?;
            if p.running {
                return Ok(p.clone());
            }
        }
        if active.len() >= 256 {
            active.retain(|_, j| j.progress.lock().map(|p| p.running).unwrap_or(true));
        }
        if active.len() >= 256 {
            return Err("等待同步的歌单过多".into());
        }
        active.insert(key.clone(), job.clone());
    }
    let deadline = std::time::Instant::now() + Duration::from_secs(600);
    let task_job = job.clone();
    tauri::async_runtime::spawn(async move {
        let work = async {
            let _permit = jobs
                .slots
                .clone()
                .acquire_owned()
                .await
                .map_err(|e| e.to_string())?;
            if let Some(id) = id.clone() {
                let s = store.clone();
                tauri::async_runtime::spawn_blocking(move || {
                    s.0.lock()
                        .map_err(|_| "曲库锁不可用")?
                        .execute(
                            "UPDATE aggregate_links SET last_attempt=?1 WHERE playlist_id=?2",
                            rusqlite::params![now_seconds(), id],
                        )
                        .map_err(|e| e.to_string())?;
                    Ok::<_, String>(())
                })
                .await
                .map_err(|e| e.to_string())??;
            }
            let (title, tracks) = read(&music, &link, &expected, &task_job).await?;
            let s = store.clone();
            let m = music.clone();
            let l = link.clone();
            let id = id.clone();
            let task_job = task_job.clone();
            tauri::async_runtime::spawn_blocking(move || {
                store::apply_checked(&s, id.as_deref(), &l, &title, &tracks, || {
                    if *task_job.cancel.borrow() || std::time::Instant::now() >= deadline {
                        return Err("同步已取消或超时，旧内容已保留".into());
                    }
                    scope(&m, &l, &expected)
                })
            })
            .await
            .map_err(|e| e.to_string())?
        };
        let result = tokio::select! {biased; _=cancelled.changed()=>Err("同步已取消".into()),result=tokio::time::timeout(Duration::from_secs(600),work)=>result.unwrap_or_else(|_|Err("同步超过 10 分钟，原有内容已保留".into()))};
        // Cancel even on timeout so an already scheduled blocking commit cannot publish later.
        job.cancel.send_replace(true);
        if let Some(id) = &id {
            let s = store.clone();
            let id = id.clone();
            let error = result.as_ref().err().cloned();
            let _ = tauri::async_runtime::spawn_blocking(move || {
                s.0.lock()
                    .map_err(|_| "曲库锁不可用")?
                    .execute(
                        "UPDATE aggregate_links SET last_attempt=?1,error=?2 WHERE playlist_id=?3",
                        rusqlite::params![now_seconds(), error, id],
                    )
                    .map_err(|e| e.to_string())?;
                Ok::<_, String>(())
            })
            .await;
        }
        if let Ok(mut p) = job.progress.lock() {
            p.running = false;
            match &result {
                Ok(id) => p.playlist_id = Some(id.clone()),
                Err(e) => p.error = Some(e.clone()),
            }
        }
        if let Ok(id) = result {
            super::changed(&app, &id)
        } else if let Some(id) = &id {
            super::changed(&app, id)
        }
    });
    Ok(progress)
}
#[tauri::command]
pub fn aggregate_sync_status(
    key: String,
    backend: State<'_, Backend>,
) -> AppResult<Option<Progress>> {
    backend
        .aggregate
        .active
        .lock()
        .map_err(|_| "同步状态不可用")?
        .get(&key)
        .map(|j| {
            j.progress
                .lock()
                .map(|p| p.clone())
                .map_err(|_| "同步状态不可用".into())
        })
        .transpose()
}
#[tauri::command]
pub fn aggregate_sync_cancel(key: String, backend: State<'_, Backend>) -> AppResult<()> {
    if let Some(job) = backend
        .aggregate
        .active
        .lock()
        .map_err(|_| "同步状态不可用")?
        .get(&key)
    {
        job.cancel.send_replace(true);
    }
    Ok(())
}
