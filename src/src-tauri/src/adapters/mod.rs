pub mod local;
pub mod netease;

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::models::{account::Account, album::Album, artist::Artist, playlist::Playlist, song::Song};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum CapabilityType {
    Music,
    Search,
    Album,
    Artist,
    Playlist,
    Account,
    Recommend,
}

#[derive(Clone, Debug, Serialize)]
pub struct AdapterMetadata {
    pub slug: String,
    pub platform: String,
    pub display_platform: String,
    pub author: String,
    pub description: String,
    pub version: String,
    pub capabilities: Vec<CapabilityType>,
}

/// Search result aggregation
#[derive(Clone, Debug, Serialize)]
pub struct SearchResult {
    pub songs: Vec<Song>,
    pub albums: Vec<Album>,
    pub artists: Vec<Artist>,
    pub playlists: Vec<Playlist>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum LoginStatus {
    #[serde(rename = "waiting")]
    Waiting { qr_url: String },
    #[serde(rename = "scanned")]
    Scanned,
    #[serde(rename = "confirmed")]
    Confirmed { account: Account },
    #[serde(rename = "timeout")]
    Timeout,
    #[serde(rename = "cancelled")]
    Cancelled,
}

#[async_trait]
pub trait Adapter: Send + Sync {
    fn metadata(&self) -> &AdapterMetadata;

    /// Returns which capability types this adapter supports
    fn capabilities(&self) -> Vec<CapabilityType> {
        vec![CapabilityType::Music]
    }

    // -- Music --
    async fn get_song(&self, id: &str) -> Result<Song>;
    async fn get_songs(&self, ids: &[String]) -> Result<Vec<Song>>;
    async fn get_song_url(&self, id: &str) -> Result<String>;
    async fn get_lyric(&self, id: &str) -> Result<String>;
    async fn toggle_like(&self, id: &str, like: bool) -> Result<bool>;

    // -- Album / Artist / Playlist --
    async fn get_album(&self, id: &str) -> Result<Album>;
    async fn get_artist(&self, id: &str) -> Result<Artist>;
    async fn get_playlist(&self, id: &str) -> Result<Playlist>;

    // -- Search --
    async fn search(&self, keyword: &str) -> Result<SearchResult>;

    // -- Account (default: not supported) --
    async fn login_qr_url(&self) -> Result<(String, String)> {
        Err(Error::NotImplemented("login not supported".into()))
    }
    async fn check_login(&self, _key: &str) -> Result<LoginStatus> {
        Err(Error::NotImplemented("login not supported".into()))
    }
    async fn get_account(&self) -> Result<Account> {
        Err(Error::NotImplemented("account not supported".into()))
    }
    async fn get_user_playlists(&self) -> Result<Vec<Playlist>> {
        Ok(vec![])
    }
    async fn get_favorite_playlist(&self) -> Result<Option<Playlist>> {
        Ok(None)
    }

    // -- Recommend (default: not supported) --
    async fn get_recommended_playlists(&self, _count: u32) -> Result<Vec<Playlist>> {
        Err(Error::NotImplemented("recommend not supported".into()))
    }
    async fn get_daily_recommended(&self) -> Result<Vec<Song>> {
        Err(Error::NotImplemented("recommend not supported".into()))
    }

    // -- Discover (default: not supported) --
    async fn get_top_playlists(&self) -> Result<Vec<TopPlaylistGroup>> {
        Err(Error::NotImplemented("top playlists not supported".into()))
    }
    async fn get_playlist_cats(&self) -> Result<Vec<PlaylistCategory>> {
        Err(Error::NotImplemented("playlist cats not supported".into()))
    }
    async fn get_playlist_square(
        &self,
        _cat: &str,
        _order: &str,
        _limit: u32,
        _offset: u32,
        _high_quality: bool,
    ) -> Result<(Vec<Playlist>, usize)> {
        Err(Error::NotImplemented("playlist square not supported".into()))
    }
}

/// A group of playlists (e.g. "Official", "Featured")
#[derive(Clone, Debug, Serialize)]
pub struct TopPlaylistGroup {
    pub name: String,
    pub playlists: Vec<Playlist>,
}

/// A playlist category with its tags
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlaylistCategory {
    pub name: String,
    pub tags: Vec<String>,
}

/// Thread-safe adapter registry.
/// Uses RwLock for interior mutability so it can be shared via Tauri State.
pub struct AdapterManager {
    adapters: RwLock<HashMap<String, Arc<dyn Adapter>>>,
}

impl AdapterManager {
    pub fn new() -> Self {
        Self {
            adapters: RwLock::new(HashMap::new()),
        }
    }

    /// Register an adapter. Takes ownership via Arc; &self suffices thanks to RwLock.
    pub fn register(&self, adapter: impl Adapter + 'static) {
        let meta = adapter.metadata().clone();
        log::info!(
            "[adapter] loaded  {}  v{}  (slug=\"{}\", platform=\"{}\")",
            meta.display_platform, meta.version, meta.slug, meta.platform
        );
        self.adapters
            .write()
            .unwrap()
            .insert(meta.slug.clone(), Arc::new(adapter));
    }

    /// Look up an adapter by slug.
    pub fn get(&self, slug: &str) -> Option<Arc<dyn Adapter>> {
        self.adapters.read().unwrap().get(slug).cloned()
    }

    /// List metadata for all registered adapters.
    pub fn list(&self) -> Vec<AdapterMetadata> {
        self.adapters
            .read()
            .unwrap()
            .values()
            .map(|a| {
                let mut meta = a.metadata().clone();
                meta.capabilities = a.capabilities();
                meta
            })
            .collect()
    }
}

impl Default for AdapterManager {
    fn default() -> Self {
        Self::new()
    }
}
