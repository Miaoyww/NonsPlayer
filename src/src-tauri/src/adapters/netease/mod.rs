mod client;
mod crypto;
mod mapper;
pub mod models;

use std::sync::Mutex;

use async_trait::async_trait;

use super::{Adapter, AdapterMetadata, CapabilityType, LoginStatus, PlaylistCategory, SearchResult, TopPlaylistGroup};
use crate::error::{Error, Result};
use crate::models::{account::Account, album::Album, artist::Artist, playlist::Playlist, song::Song};

use client::{CryptoType, NeteaseClient, INTERFACE3_HOST, INTERFACE_HOST, MUSIC_HOST};
use mapper::{
    map_cloudsearch_album, map_cloudsearch_artist, map_playlist_full, map_recommend_playlist,
    map_search_playlist, map_song, map_toplist_item,
};

pub struct NeteaseAdapter {
    metadata: AdapterMetadata,
    client: NeteaseClient,
    account: Mutex<Option<Account>>,
    qr_key: Mutex<Option<String>>,
}

impl NeteaseAdapter {
    pub fn new() -> Self {
        Self {
            metadata: AdapterMetadata {
                slug: "netease".into(),
                platform: "netease".into(),
                display_platform: "网易云音乐".into(),
                author: "NonsPlayer".into(),
                description: "网易云音乐适配器，支持搜索、歌单、每日推荐、二维码登录".into(),
                version: "0.2.0".into(),
                capabilities: vec![],
            },
            client: NeteaseClient::new(),
            account: Mutex::new(None),
            qr_key: Mutex::new(None),
        }
    }

    fn cookies_str(&self) -> String {
        self.client.cookies_str()
    }
}

#[async_trait]
impl Adapter for NeteaseAdapter {
    fn metadata(&self) -> &AdapterMetadata {
        &self.metadata
    }

    fn capabilities(&self) -> Vec<CapabilityType> {
        vec![
            CapabilityType::Music,
            CapabilityType::Search,
            CapabilityType::Album,
            CapabilityType::Artist,
            CapabilityType::Playlist,
            CapabilityType::Account,
            CapabilityType::Recommend,
        ]
    }

    // ── Music ──

    async fn get_song(&self, id: &str) -> Result<Song> {
        let netease_id = id.strip_prefix("netease_song_").unwrap_or(id);
        let payload = serde_json::json!({
            "c": serde_json::json!([{"id": netease_id}]),
            "ids": format!("[{}]", netease_id),
        });
        let body = self
            .client
            .request_ok(
                CryptoType::Weapi,
                &format!("{}/weapi/v3/song/detail", MUSIC_HOST),
                &payload,
            )
            .await?;

        let resp: models::SongDetailResponse = serde_json::from_value(body)
            .map_err(|e| Error::Other(format!("parse error: {}", e)))?;
        let song = resp.songs.first().ok_or_else(|| Error::SongNotFound(id.into()))?;
        Ok(map_song(song))
    }

    async fn get_songs(&self, ids: &[String]) -> Result<Vec<Song>> {
        let netease_ids: Vec<String> = ids
            .iter()
            .map(|id| id.strip_prefix("netease_song_").unwrap_or(id).to_string())
            .collect();
        let ids_str = netease_ids.join(",");

        let payload = serde_json::json!({
            "c": serde_json::json!([{"id": &ids_str}]),
            "ids": format!("[{}]", ids_str),
        });
        let body = self
            .client
            .request_ok(
                CryptoType::Weapi,
                &format!("{}/weapi/v3/song/detail", MUSIC_HOST),
                &payload,
            )
            .await?;

        let resp: models::SongDetailResponse = serde_json::from_value(body)
            .map_err(|e| Error::Other(format!("parse error: {}", e)))?;
        let songs: Vec<Song> = resp.songs.iter().map(map_song).collect();
        Ok(songs)
    }

    async fn get_song_url(&self, id: &str) -> Result<String> {
        let netease_id = id.strip_prefix("netease_song_").unwrap_or(id);

        for level in &["exhigh", "lossless", "standard"] {
            let payload = serde_json::json!({
                "ids": format!("[{}]", netease_id),
                "level": level,
                "encodeType": "flac",
            });
            let body = self
                .client
                .request(
                    CryptoType::Eapi,
                    &format!("{}/eapi/song/enhance/player/url/v1", INTERFACE_HOST),
                    &payload,
                )
                .await?;

            if body["code"].as_i64().unwrap_or(-1) != 200 {
                continue;
            }

            let resp: models::SongUrlResponse = serde_json::from_value(body)
                .map_err(|e| Error::Other(format!("parse error: {}", e)))?;

            if let Some(url) = resp.data.first().and_then(|d| d.url.as_deref()) {
                if !url.is_empty() {
                    return Ok(url.to_string());
                }
            }
        }

        Err(Error::Other("no playable URL found".into()))
    }

    async fn get_lyric(&self, id: &str) -> Result<String> {
        let netease_id = id.strip_prefix("netease_song_").unwrap_or(id);

        // Try eapi lyric/v1 first (YRC word-level lyrics)
        let payload = serde_json::json!({
            "id": netease_id,
            "cp": false,
            "tv": 0, "lv": 0, "rv": 0, "kv": 0,
            "yv": 0, "ytv": 0, "yrv": 0,
        });
        if let Ok(body) = self
            .client
            .request(
                CryptoType::Eapi,
                &format!("{}/eapi/song/lyric/v1", INTERFACE3_HOST),
                &payload,
            )
            .await
        {
            if body["code"].as_i64().unwrap_or(-1) == 200 {
                if let Some(yrc) = body["yrc"]["lyric"].as_str() {
                    if !yrc.is_empty() {
                        return Ok(yrc.to_string());
                    }
                }
                if let Some(lrc) = body["lrc"]["lyric"].as_str() {
                    if !lrc.is_empty() {
                        return Ok(lrc.to_string());
                    }
                }
            }
        }

        // Fall back to legacy /api/song/lyric
        let payload = serde_json::json!({
            "id": netease_id,
            "tv": -1, "lv": -1, "rv": -1, "kv": -1,
        });
        if let Ok(body) = self
            .client
            .request(
                CryptoType::Api,
                &format!("{}/api/song/lyric?_nmclfl=1", MUSIC_HOST),
                &payload,
            )
            .await
        {
            if let Some(lrc) = body["lrc"]["lyric"].as_str() {
                if !lrc.is_empty() {
                    return Ok(lrc.to_string());
                }
            }
        }

        Ok(String::new())
    }

    async fn toggle_like(&self, id: &str, like: bool) -> Result<bool> {
        let netease_id = id.strip_prefix("netease_song_").unwrap_or(id);
        let cookie = self.cookies_str();
        if cookie.is_empty() {
            return Err(Error::NotImplemented("login required to like".into()));
        }

        let payload = serde_json::json!({
            "alg": "itembased",
            "trackId": netease_id,
            "like": like,
            "time": "3",
        });
        let body = self
            .client
            .request_ok(
                CryptoType::Weapi,
                &format!("{}/weapi/radio/like", MUSIC_HOST),
                &payload,
            )
            .await?;
        Ok(body["code"].as_i64().unwrap_or(-1) == 200)
    }

    // ── Album / Artist / Playlist ──

    async fn get_album(&self, id: &str) -> Result<Album> {
        let netease_id = id.strip_prefix("netease_album_").unwrap_or(id);
        let payload = serde_json::json!({});
        let body = self
            .client
            .request_ok(
                CryptoType::Weapi,
                &format!("{}/weapi/v1/album/{}", MUSIC_HOST, netease_id),
                &payload,
            )
            .await?;

        let resp: models::AlbumResponse = serde_json::from_value(body)
            .map_err(|e| Error::Other(format!("parse error: {}", e)))?;
        let album = &resp.album;

        let artist_raw = album.artist.as_ref();
        let artist = Artist {
            id: format!(
                "netease_artist_{}",
                artist_raw
                    .map(|a| crate::adapters::netease::mapper::val_to_string(&a.id))
                    .unwrap_or_default()
            ),
            name: artist_raw.map(|a| a.name.clone()).unwrap_or_default(),
            avatar_url: artist_raw
                .and_then(|a| a.pic_url.as_deref())
                .unwrap_or("")
                .to_string(),
            adapter_slug: "netease".into(),
            ..Artist::empty()
        };

        let cover = album.pic_url.as_deref().unwrap_or("");
        let songs: Vec<Song> = album.songs.iter().map(map_song).collect();

        Ok(Album {
            id: format!("netease_album_{}", mapper::val_to_string(&album.id)),
            name: album.name.clone(),
            avatar_url: format!("{}?param=300y300", cover),
            small_avatar_url: format!("{}?param=50y50", cover),
            middle_avatar_url: format!("{}?param=200y200", cover),
            artists: vec![artist.clone()],
            artists_name: artist.name.clone(),
            description: album.description.clone().unwrap_or_default(),
            songs,
            adapter_slug: "netease".into(),
            ..Album::empty()
        })
    }

    async fn get_artist(&self, id: &str) -> Result<Artist> {
        let netease_id = id.strip_prefix("netease_artist_").unwrap_or(id);
        let payload = serde_json::json!({});
        let body = self
            .client
            .request_ok(
                CryptoType::Weapi,
                &format!("{}/weapi/v1/artist/{}", MUSIC_HOST, netease_id),
                &payload,
            )
            .await?;

        let resp: models::ArtistResponse = serde_json::from_value(body)
            .map_err(|e| Error::Other(format!("parse error: {}", e)))?;
        let artist = &resp.artist;

        Ok(Artist {
            id: format!("netease_artist_{}", mapper::val_to_string(&artist.id)),
            name: artist.name.clone(),
            avatar_url: artist.pic_url.as_deref().unwrap_or("").to_string(),
            description: artist.brief_desc.clone().unwrap_or_default(),
            adapter_slug: "netease".into(),
            ..Artist::empty()
        })
    }

    async fn get_playlist(&self, id: &str) -> Result<Playlist> {
        let netease_id = id.strip_prefix("netease_playlist_").unwrap_or(id);
        let payload = serde_json::json!({
            "id": netease_id,
            "n": 100000,
            "s": 50,
        });
        let body = self
            .client
            .request_ok(
                CryptoType::Api,
                &format!("{}/api/v6/playlist/detail", MUSIC_HOST),
                &payload,
            )
            .await?;

        let resp: models::PlaylistDetailResponse = serde_json::from_value(body)
            .map_err(|e| Error::Other(format!("parse error: {}", e)))?;
        Ok(map_playlist_full(&resp))
    }

    // ── Search ──

    async fn search(&self, keyword: &str) -> Result<SearchResult> {
        if keyword.trim().is_empty() {
            return Ok(SearchResult {
                songs: vec![],
                albums: vec![],
                artists: vec![],
                playlists: vec![],
            });
        }

        let limit = "20";
        let url = format!("{}/eapi/cloudsearch/pc", INTERFACE_HOST);

        let songs_raw = self
            .client
            .request_ok(
                CryptoType::Eapi,
                &url,
                &serde_json::json!({"s": keyword, "type": 1, "limit": limit, "offset": 0, "total": true}),
            )
            .await;

        let albums_raw = self
            .client
            .request_ok(
                CryptoType::Eapi,
                &url,
                &serde_json::json!({"s": keyword, "type": 10, "limit": limit, "offset": 0, "total": true}),
            )
            .await;

        let artists_raw = self
            .client
            .request_ok(
                CryptoType::Eapi,
                &url,
                &serde_json::json!({"s": keyword, "type": 100, "limit": "10", "offset": 0, "total": true}),
            )
            .await;

        let playlists_raw = self
            .client
            .request_ok(
                CryptoType::Eapi,
                &url,
                &serde_json::json!({"s": keyword, "type": 1000, "limit": limit, "offset": 0, "total": true}),
            )
            .await;

        let songs: Vec<Song> = songs_raw
            .ok()
            .and_then(|r| {
                serde_json::from_value::<models::CloudSearchResponse>(r).ok()
            })
            .map(|resp| resp.result.songs.iter().map(map_song).collect())
            .unwrap_or_default();

        let albums: Vec<Album> = albums_raw
            .ok()
            .and_then(|r| {
                serde_json::from_value::<models::CloudSearchResponse>(r).ok()
            })
            .map(|resp| resp.result.albums.iter().map(map_cloudsearch_album).collect())
            .unwrap_or_default();

        let artists: Vec<Artist> = artists_raw
            .ok()
            .and_then(|r| {
                serde_json::from_value::<models::CloudSearchResponse>(r).ok()
            })
            .map(|resp| resp.result.artists.iter().map(map_cloudsearch_artist).collect())
            .unwrap_or_default();

        let playlists: Vec<Playlist> = playlists_raw
            .ok()
            .and_then(|r| {
                serde_json::from_value::<models::CloudSearchResponse>(r).ok()
            })
            .map(|resp| resp.result.playlists.iter().map(map_search_playlist).collect())
            .unwrap_or_default();

        Ok(SearchResult {
            songs,
            albums,
            artists,
            playlists,
        })
    }

    // ── Account ──

    async fn login_qr_url(&self) -> Result<(String, String)> {
        // Step 1: get unikey
        let body = self
            .client
            .request(
                CryptoType::Weapi,
                &format!("{}/weapi/login/qrcode/unikey", MUSIC_HOST),
                &serde_json::json!({"type": 1}),
            )
            .await?;

        let unikey = body["data"]["unikey"]
            .as_str()
            .ok_or_else(|| Error::Other("failed to get qr key".into()))?
            .to_string();
        *self.qr_key.lock().unwrap() = Some(unikey.clone());

        // Step 2: return QR URL for frontend to render
        let qr_url = format!("https://music.163.com/login?codekey={}", unikey);
        Ok((unikey, qr_url))
    }

    async fn check_login(&self, key: &str) -> Result<LoginStatus> {
        let body = self
            .client
            .request(
                CryptoType::Weapi,
                &format!("{}/weapi/login/qrcode/client/login", MUSIC_HOST),
                &serde_json::json!({"key": key, "type": 1}),
            )
            .await?;

        let code = body["code"].as_i64().unwrap_or(-1);

        match code {
            801 => Ok(LoginStatus::Waiting {
                qr_url: String::new(),
            }),
            802 => Ok(LoginStatus::Scanned),
            803 => {
                // Login confirmed — save cookies
                if let Some(cookie) = body["cookie"].as_str() {
                    self.client.save_cookies(cookie);
                }

                // Try to fetch account info
                let account = self.get_account().await.ok();
                *self.account.lock().unwrap() = account.clone();

                Ok(LoginStatus::Confirmed {
                    account: account.unwrap_or_else(|| Account {
                        id: String::new(),
                        md5: String::new(),
                        name: "网易云用户".into(),
                        token: self.cookies_str(),
                        avatar_url: String::new(),
                        is_logged_in: true,
                        key: key.to_string(),
                    }),
                })
            }
            800 => Ok(LoginStatus::Timeout),
            _ => Ok(LoginStatus::Cancelled),
        }
    }

    async fn get_account(&self) -> Result<Account> {
        let cookie = self.cookies_str();
        if cookie.is_empty() {
            return Ok(Account::not_logged_in());
        }

        let body = self
            .client
            .request_ok(
                CryptoType::Weapi,
                &format!("{}/weapi/nuser/account/get", MUSIC_HOST),
                &serde_json::json!({}),
            )
            .await?;

        let resp: models::UserAccountResponse = serde_json::from_value(body)
            .map_err(|e| Error::Other(format!("parse error: {}", e)))?;
        let user_id = resp
            .account
            .as_ref()
            .map(|a| mapper::val_to_string(&a.id))
            .unwrap_or_default();
        let nickname = resp
            .profile
            .as_ref()
            .and_then(|p| p.nickname.as_deref())
            .unwrap_or("")
            .to_string();
        let avatar = resp
            .profile
            .as_ref()
            .and_then(|p| p.avatar_url.as_deref())
            .unwrap_or("")
            .to_string();

        Ok(Account {
            id: format!("netease_user_{}", user_id),
            name: if nickname.is_empty() {
                "网易云用户".into()
            } else {
                nickname
            },
            avatar_url: avatar,
            token: cookie,
            is_logged_in: true,
            ..Account::empty()
        })
    }

    async fn get_user_playlists(&self) -> Result<Vec<Playlist>> {
        let uid = {
            let account_guard = self.account.lock().unwrap();
            account_guard
                .as_ref()
                .and_then(|a| a.id.strip_prefix("netease_user_").map(|s| s.to_string()))
        };

        let uid = match uid {
            Some(u) => u,
            None => {
                let account = self.get_account().await?;
                let uid = account
                    .id
                    .strip_prefix("netease_user_")
                    .unwrap_or("")
                    .to_string();
                if uid.is_empty() {
                    return Ok(vec![]);
                }
                uid
            }
        };

        let body = self
            .client
            .request_ok(
                CryptoType::Weapi,
                &format!("{}/weapi/user/playlist", MUSIC_HOST),
                &serde_json::json!({"uid": uid, "limit": 50, "offset": 0, "includeVideo": true}),
            )
            .await?;

        let resp: models::UserPlaylistResponse = serde_json::from_value(body)
            .map_err(|e| Error::Other(format!("parse error: {}", e)))?;
        let playlists: Vec<Playlist> = resp.playlist.iter().map(map_search_playlist).collect();
        Ok(playlists)
    }

    async fn get_favorite_playlist(&self) -> Result<Option<Playlist>> {
        let uid = {
            let account_guard = self.account.lock().unwrap();
            account_guard
                .as_ref()
                .and_then(|a| a.id.strip_prefix("netease_user_").map(|s| s.to_string()))
        };

        let uid = match uid {
            Some(u) => u,
            None => {
                let account = self.get_account().await?;
                account
                    .id
                    .strip_prefix("netease_user_")
                    .unwrap_or("")
                    .to_string()
            }
        };
        if uid.is_empty() {
            return Ok(None);
        }

        let body = self
            .client
            .request_ok(
                CryptoType::Weapi,
                &format!("{}/weapi/user/playlist", MUSIC_HOST),
                &serde_json::json!({"uid": uid, "limit": 50, "offset": 0, "includeVideo": true}),
            )
            .await?;

        let resp: models::UserPlaylistResponse = serde_json::from_value(body)
            .map_err(|e| Error::Other(format!("parse error: {}", e)))?;

        let fav = resp
            .playlist
            .iter()
            .find(|p| p.special_type.unwrap_or(0.0) == 5.0)
            .map(map_search_playlist);

        Ok(fav)
    }

    // ── Recommend ──

    async fn get_recommended_playlists(&self, count: u32) -> Result<Vec<Playlist>> {
        let logged_in = self.client.is_logged_in();

        let playlists = if logged_in {
            // Try authenticated recommend first
            match self
                .client
                .request_ok(
                    CryptoType::Weapi,
                    &format!("{}/weapi/v1/discovery/recommend/resource", MUSIC_HOST),
                    &serde_json::json!({}),
                )
                .await
            {
                Ok(body) => {
                    log::debug!("[netease] recommend/resource raw body: {}", body);
                    let resp: models::RecommendResourceResponse = serde_json::from_value(body)
                        .map_err(|e| Error::Other(format!("parse error: {}", e)))?;
                    resp.recommend
                        .iter()
                        .take(count as usize)
                        .map(map_recommend_playlist)
                        .collect()
                }
                Err(e) => {
                    log::debug!("[netease] recommend/resource failed ({}), falling back to /personalized", e);
                    let body = self
                        .client
                        .request_ok(
                            CryptoType::Weapi,
                            &format!("{}/weapi/personalized/playlist", MUSIC_HOST),
                            &serde_json::json!({"limit": count, "total": true, "n": 1000}),
                        )
                        .await?;
                    log::debug!("[netease] personalized raw body: {}", body);
                    let resp: models::PersonalizedResponse = serde_json::from_value(body)
                        .map_err(|e| Error::Other(format!("parse error: {}", e)))?;
                    resp.result.iter().map(map_recommend_playlist).collect()
                }
            }
        } else {
            // Anonymous: /personalized
            log::debug!("[netease] anonymous, using /personalized");
            let body = self
                .client
                .request_ok(
                    CryptoType::Weapi,
                    &format!("{}/weapi/personalized/playlist", MUSIC_HOST),
                    &serde_json::json!({"limit": count, "total": true, "n": 1000}),
                )
                .await?;
            log::debug!("[netease] personalized raw body: {}", body);
            let resp: models::PersonalizedResponse = serde_json::from_value(body)
                .map_err(|e| Error::Other(format!("parse error: {}", e)))?;
            resp.result.iter().map(map_recommend_playlist).collect()
        };

        Ok(playlists)
    }

    async fn get_daily_recommended(&self) -> Result<Vec<Song>> {
        let body = self
            .client
            .request_ok(
                CryptoType::Weapi,
                &format!("{}/weapi/v3/discovery/recommend/songs", MUSIC_HOST),
                &serde_json::json!({}),
            )
            .await?;

        let resp: models::RecommendSongsResponse = serde_json::from_value(body)
            .map_err(|e| Error::Other(format!("parse error: {}", e)))?;
        let songs: Vec<Song> = resp.data.daily_songs.iter().map(map_song).collect();
        Ok(songs)
    }

    // ── Discover ──

    async fn get_top_playlists(&self) -> Result<Vec<TopPlaylistGroup>> {
        let body = self
            .client
            .request_ok(
                CryptoType::Api,
                &format!("{}/api/toplist/detail", MUSIC_HOST),
                &serde_json::json!({}),
            )
            .await?;

        let resp: models::ToplistDetailResponse = serde_json::from_value(body)
            .map_err(|e| Error::Other(format!("parse toplist: {}", e)))?;

        let mut official = Vec::new();
        let mut featured = Vec::new();

        for item in &resp.list {
            // Skip items without a valid id (some API entries lack this field)
            if item.id.is_null() {
                continue;
            }
            let playlist = map_toplist_item(item);
            if item.toplist_type.is_some() {
                official.push(playlist);
            } else {
                featured.push(playlist);
            }
        }

        log::info!(
            "[netease] get_top_playlists: {} official, {} featured",
            official.len(),
            featured.len()
        );

        Ok(vec![
            TopPlaylistGroup {
                name: "Official".into(),
                playlists: official,
            },
            TopPlaylistGroup {
                name: "Featured".into(),
                playlists: featured,
            },
        ])
    }

    async fn get_playlist_cats(&self) -> Result<Vec<PlaylistCategory>> {
        let body = self
            .client
            .request_ok(
                CryptoType::Api,
                &format!("{}/api/playlist/catlist", MUSIC_HOST),
                &serde_json::json!({}),
            )
            .await?;

        let resp: models::CatlistResponse = serde_json::from_value(body)
            .map_err(|e| Error::Other(format!("parse catlist: {}", e)))?;

        // Build category map from the "categories" object
        let mut cat_map: std::collections::BTreeMap<String, Vec<String>> =
            std::collections::BTreeMap::new();

        // Parse categories: keys are numbers, values are category names
        if let Some(categories) = &resp.categories {
            for (key, val) in categories {
                let cat_num = key.parse::<i64>().unwrap_or(-1);
                let cat_name = match val {
                    serde_json::Value::String(s) => s.clone(),
                    _ => key.clone(),
                };
                // Collect sub items for this category
                let tags: Vec<String> = resp
                    .sub
                    .iter()
                    .filter(|c| c.category.map(|c| c as i64) == Some(cat_num))
                    .map(|c| c.name.clone())
                    .collect();
                if !tags.is_empty() {
                    cat_map.insert(cat_name, tags);
                }
            }
        }

        let cats: Vec<PlaylistCategory> = cat_map
            .into_iter()
            .map(|(name, tags)| PlaylistCategory { name, tags })
            .collect();

        log::info!("[netease] get_playlist_cats: {} categories", cats.len());

        Ok(cats)
    }

    async fn get_playlist_square(
        &self,
        cat: &str,
        order: &str,
        limit: u32,
        offset: u32,
        high_quality: bool,
    ) -> Result<(Vec<Playlist>, usize)> {
        if high_quality {
            let body = self
                .client
                .request_ok(
                    CryptoType::Api,
                    &format!("{}/api/top/playlist/highquality", MUSIC_HOST),
                    &serde_json::json!({
                        "cat": cat,
                        "limit": limit,
                        "before": offset,
                    }),
                )
                .await?;

            let resp: models::HighqualityPlaylistResponse = serde_json::from_value(body)
                .map_err(|e| Error::Other(format!("parse highquality: {}", e)))?;

            let playlists: Vec<Playlist> =
                resp.playlists.iter().map(map_search_playlist).collect();
            let total = resp.total.unwrap_or(0.0) as usize;
            let has_more = total > (offset + limit) as usize;

            log::info!(
                "[netease] get_playlist_square hq cat={} offset={} count={} total={} more={}",
                cat,
                offset,
                playlists.len(),
                total,
                has_more
            );

            Ok((playlists, if has_more { total } else { 0 }))
        } else {
            let body = self
                .client
                .request_ok(
                    CryptoType::Api,
                    &format!("{}/api/top/playlist", MUSIC_HOST),
                    &serde_json::json!({
                        "cat": cat,
                        "order": order,
                        "limit": limit,
                        "offset": offset,
                        "total": true,
                    }),
                )
                .await?;

            let resp: models::TopPlaylistResponse = serde_json::from_value(body)
                .map_err(|e| Error::Other(format!("parse top/playlist: {}", e)))?;

            let playlists: Vec<Playlist> =
                resp.playlists.iter().map(map_search_playlist).collect();
            let total = resp.total.unwrap_or(0.0) as usize;
            let has_more = resp.more.unwrap_or(false)
                || total > (offset + limit) as usize;

            log::info!(
                "[netease] get_playlist_square cat={} order={} offset={} count={} total={} more={}",
                cat,
                order,
                offset,
                playlists.len(),
                total,
                has_more
            );

            Ok((playlists, if has_more { total } else { 0 }))
        }
    }
}
