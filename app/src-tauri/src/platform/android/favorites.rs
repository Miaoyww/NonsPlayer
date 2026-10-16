//! One bounded, serialized worker for system-card favorites. Never runs on the audio actor.
use crate::{
    model::{AppResult, TrackSource},
    music::{
        business::{BusinessRequest, BusinessResponse},
        identity::EntityRef,
    },
};
use std::{
    collections::HashSet,
    time::{Duration, Instant},
};
use tauri::{Emitter, Manager};

pub(super) struct Request {
    pub id: u64,
    pub key: String,
    pub sequence: u64,
    pub liked: Option<bool>,
}

#[derive(Default)]
struct Favorites {
    key: String,
    reference: Option<EntityRef>,
    scope: String,
    ids: HashSet<EntityRef>,
    available: bool,
    sequence: u64,
    refreshed: Option<Instant>,
}
impl Favorites {
    fn publish(&mut self, pending: bool) {
        self.sequence += 1;
        let _ = (super::playback::call(
            "mediaFavorite",
            serde_json::json!({
                "key": self.key, "sequence": self.sequence,
                "liked": self.reference.as_ref().is_some_and(|r| self.ids.contains(r)),
                "available": self.available && !pending,
            }),
        ))
        .inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("super::playback::call", error);
        });
    }
    async fn refresh(&mut self, backend: &crate::Backend) {
        let track = backend
            .player
            .snapshot()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("optional_result", error);
            })
            .ok()
            .and_then(|s| s.current().cloned());
        let key = track.as_ref().map_or_else(String::new, |t| t.key.clone());
        let reference = track
            .filter(|t| !matches!(t.source, TrackSource::Local { .. }))
            .and_then(|t| {
                nons_music_adapter_sdk::migration::convert_track(t)
                    .inspect_err(|error| {
                        crate::infrastructure::diagnostics::warn("optional_result", error);
                    })
                    .ok()
            })
            .map(|t| t.track.reference);
        let scope = reference
            .as_ref()
            .and_then(|r| {
                backend
                    .music
                    .adapters
                    .cache_identity(&r.source)
                    .inspect_err(|error| {
                        crate::infrastructure::diagnostics::warn("optional_result", error);
                    })
                    .ok()
            })
            .unwrap_or_default();
        let changed = scope != self.scope || reference != self.reference || key != self.key;
        let expired = self.refreshed.is_none_or(|time| {
            time.elapsed()
                >= if self.available {
                    Duration::from_secs(300)
                } else {
                    Duration::from_secs(20)
                }
        });
        if !changed && !expired {
            return;
        }
        let needs_query = scope != self.scope || expired;
        self.key = key;
        self.reference = reference;
        if needs_query {
            self.available = false;
            self.ids.clear();
            self.scope = scope.clone();
        }
        self.publish(false);
        if !needs_query {
            return;
        }
        self.refreshed = Some(Instant::now());
        let Some(reference) = &self.reference else {
            return;
        };
        // The manager enforces capabilities, login, instance generations and read budgets.
        if let Ok(BusinessResponse::Favorites(ids)) = (backend
            .music
            .adapters
            .business(&reference.source, &BusinessRequest::Favorites)
            .await)
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("optional_success", error);
            })
        {
            if backend
                .music
                .adapters
                .cache_identity(&reference.source)
                .inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn("optional_result", error);
                })
                .ok()
                .as_deref()
                != Some(&scope)
                || backend
                    .player
                    .snapshot()
                    .inspect_err(|error| {
                        crate::infrastructure::diagnostics::warn("optional_result", error);
                    })
                    .ok()
                    .and_then(|s| s.current().map(|t| t.key.clone()))
                    .as_deref()
                    != Some(&self.key)
            {
                self.refreshed = None;
                return;
            }
            self.ids = ids.into_iter().collect();
            self.available = true;
            self.publish(false);
        }
    }
    async fn toggle(
        &mut self,
        request: &Request,
        app: &tauri::AppHandle,
        backend: &crate::Backend,
    ) -> AppResult<()> {
        let reference = self.reference.clone().ok_or("当前歌曲不支持收藏")?;
        if !self.available
            || request.sequence != self.sequence
            || request.key != self.key
            || backend
                .player
                .snapshot()?
                .current()
                .is_none_or(|t| t.key != request.key)
            || backend
                .music
                .adapters
                .cache_identity(&reference.source)
                .inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn("optional_result", error);
                })
                .ok()
                .as_deref()
                != Some(&self.scope)
        {
            return Err("收藏状态已变化，请重试".into());
        }
        let stamp = backend
            .music
            .adapters
            .stamp(&reference.source)
            .map_err(|e| e.to_string())?;
        let liked = request
            .liked
            .unwrap_or_else(|| !self.ids.contains(&reference));
        if liked == self.ids.contains(&reference) {
            return Ok(());
        }
        self.publish(true);
        let result = backend
            .music
            .adapters
            .business(
                &reference.source,
                &BusinessRequest::SetFavorite {
                    track: reference.clone(),
                    liked,
                },
            )
            .await
            .map_err(|e| e.to_string());
        match result {
            Ok(BusinessResponse::Write(_))
                if backend
                    .music
                    .adapters
                    .stamp(&reference.source)
                    .inspect_err(|error| {
                        crate::infrastructure::diagnostics::warn("optional_result", error);
                    })
                    .ok()
                    .as_ref()
                    == Some(&stamp) =>
            {
                if liked {
                    self.ids.insert(reference.clone());
                } else {
                    self.ids.remove(&reference);
                }
                self.scope = backend
                    .music
                    .adapters
                    .cache_identity(&reference.source)
                    .map_err(|e| e.to_string())?;
                let _ = (app.emit("music-changed", &reference.source)).inspect_err(|error| {
                    crate::infrastructure::diagnostics::warn("app.emit", error);
                });
                // Also refresh the legacy bundled-provider account UI through its existing API/cache.
                let _ =
                    (app.emit("media-favorite-changed", &reference.source)).inspect_err(|error| {
                        crate::infrastructure::diagnostics::warn("app.emit", error);
                    });
                self.publish(false);
                Ok(())
            }
            Ok(_) => {
                self.available = false;
                self.refreshed = None;
                self.publish(false);
                Err("收藏会话已失效".into())
            }
            Err(error) => {
                self.refreshed = None;
                self.publish(false);
                Err(error)
            }
        }
    }
}

pub(super) fn initialize() -> tokio::sync::mpsc::Sender<Request> {
    let (sender, mut receiver) = tokio::sync::mpsc::channel::<Request>(1);
    tauri::async_runtime::spawn(async move {
        let Some(app) = super::APP.get().cloned() else {
            return;
        };
        let mut favorites = Favorites::default();
        let mut interval = tokio::time::interval(Duration::from_secs(2));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                request = receiver.recv() => {
                    let Some(request) = request else { break; };
                    let result = if let Some(backend) = app.try_state::<crate::Backend>() {
                        favorites.toggle(&request, &app, &backend).await
                    } else { Err("音乐服务尚未就绪".into()) };
                    let _ = (super::playback::call("mediaCommandResult", serde_json::json!({
                        "id": request.id, "error": result.err(),
                    }))).inspect_err(|error| { crate::infrastructure::diagnostics::warn("super::playback::call", error); });
                }
                _ = interval.tick() => {
                    if let Some(backend) = app.try_state::<crate::Backend>() { favorites.refresh(&backend).await; }
                }
            }
        }
    });
    sender
}
