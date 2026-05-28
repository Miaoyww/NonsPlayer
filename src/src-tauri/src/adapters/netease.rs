use std::sync::Mutex;

use async_trait::async_trait;

use super::{Adapter, AdapterMetadata, CapabilityType, LoginStatus, SearchResult};
use crate::error::{Error, Result};
use crate::models::{account::Account, album::Album, artist::Artist, playlist::Playlist, song::Song};

const DEFAULT_API_BASE: &str = "http://localhost:3000";

pub struct NeteaseAdapter {
    metadata: AdapterMetadata,
    client: reqwest::Client,
    api_base: String,
    cookies: Mutex<String>,
    account: Mutex<Option<Account>>,
    /// unikey from /login/qr/key, stored during QR login flow
    qr_key: Mutex<Option<String>>,
}

impl NeteaseAdapter {
    pub fn new(api_base: Option<String>) -> Self {
        let base = api_base.unwrap_or_else(|| DEFAULT_API_BASE.to_string());
        Self {
            metadata: AdapterMetadata {
                slug: "netease".into(),
                platform: "netease".into(),
                display_platform: "网易云音乐".into(),
                author: "NonsPlayer".into(),
                description: "网易云音乐适配器，支持搜索、歌单、每日推荐、二维码登录".into(),
                version: "0.1.0".into(),
            },
            client: reqwest::Client::new(),
            api_base: base,
            cookies: Mutex::new(String::new()),
            account: Mutex::new(None),
            qr_key: Mutex::new(None),
        }
    }

    // -- Helpers --

    /// POST to the API proxy, returning the raw `body` field of the response.
    async fn call_api(&self, endpoint: &str, params: &[(&str, &str)]) -> Result<serde_json::Value> {
        let url = format!("{}{}", self.api_base, endpoint);
        let cookie = self.cookies.lock().unwrap().clone();

        let mut req = self.client.post(&url).form(params);
        if !cookie.is_empty() {
            req = req.header("Cookie", &cookie);
        }

        let resp = req.send().await.map_err(|e| Error::Http(e))?;
        let text = resp.text().await.map_err(|e| Error::Other(format!("read error: {}", e)))?;
        let json: serde_json::Value = serde_json::from_str(&text).map_err(|e| Error::Other(format!("parse error: {}", e)))?;

        // The proxy wraps responses as { status, body, cookie }
        Ok(json["body"].clone())
    }

    /// Like call_api but checks that body.code == 200 (standard success).
    async fn call_api_ok(&self, endpoint: &str, params: &[(&str, &str)]) -> Result<serde_json::Value> {
        let body = self.call_api(endpoint, params).await?;
        let code = body["code"].as_i64().unwrap_or(-1);
        if code != 200 {
            let msg = body["message"]
                .as_str()
                .or_else(|| body["msg"].as_str())
                .unwrap_or("unknown error");
            return Err(Error::Other(format!("API error {}: {}", code, msg)));
        }
        Ok(body)
    }

    /// Save cookies from a login response. The `cookie` field in the body is a semicolon-joined string.
    fn save_cookies(&self, body: &serde_json::Value) -> bool {
        if let Some(c) = body["cookie"].as_str() {
            *self.cookies.lock().unwrap() = c.to_string();
            return true;
        }
        false
    }

    fn cookies_str(&self) -> String {
        self.cookies.lock().unwrap().clone()
    }

    // -- Mappers --

    fn map_song(raw: &serde_json::Value) -> Song {
        let id = raw["id"].to_string();
        let name = raw["name"].as_str().unwrap_or("").to_string();
        let dt_ms = raw["dt"].as_f64().unwrap_or(0.0);
        let duration = dt_ms / 1000.0;
        let fee = raw["fee"].as_i64().unwrap_or(0);
        let available = fee == 0 || fee == 8; // 0=free, 8=包月

        let minutes = (duration / 60.0) as u64;
        let seconds = (duration % 60.0) as u64;
        let duration_text = format!("{}:{:02}", minutes, seconds);

        // Artists: "ar" field from song/detail, "artists" from cloudsearch
        let ar = raw.get("ar").or_else(|| raw.get("artists"));
        let artists: Vec<Artist> = ar
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .map(|a| Artist {
                        id: format!("netease_artist_{}", a["id"]),
                        name: a["name"].as_str().unwrap_or("").to_string(),
                        avatar_url: String::new(),
                        adapter_slug: "netease".into(),
                        ..Artist::empty()
                    })
                    .collect()
            })
            .unwrap_or_default();

        // Album: "al" field from song/detail, "album" from cloudsearch
        let al = raw.get("al").or_else(|| raw.get("album"));
        let album_name = al.and_then(|v| v["name"].as_str()).unwrap_or("").to_string();
        let album_id = al.and_then(|v| v["id"].as_str()).unwrap_or("").to_string();
        let album_pic = al.and_then(|v| v["picUrl"].as_str()).unwrap_or("");
        let album = Album {
            id: format!("netease_album_{}", album_id),
            name: album_name.clone(),
            avatar_url: format!("{}?param=300y300", album_pic),
            small_avatar_url: format!("{}?param=50y50", album_pic),
            middle_avatar_url: format!("{}?param=200y200", album_pic),
            artists: artists.clone(),
            artists_name: artists.iter().map(|a| a.name.clone()).collect::<Vec<_>>().join("/"),
            adapter_slug: "netease".into(),
            ..Album::empty()
        };

        let artists_name = artists.iter().map(|a| a.name.clone()).collect::<Vec<_>>().join("/");
        let avatar_url = al.and_then(|v| v["picUrl"].as_str()).unwrap_or("").to_string();

        Song {
            id: format!("netease_song_{}", id),
            name,
            avatar_url: avatar_url.clone(),
            small_avatar_url: format!("{}?param=50y50", avatar_url),
            middle_avatar_url: format!("{}?param=200y200", avatar_url),
            album,
            artists,
            duration,
            duration_text,
            available,
            album_name,
            artists_name,
            adapter_slug: "netease".into(),
            url: String::new(),
            is_liked: false,
            trans: raw["alia"]
                .as_array()
                .and_then(|arr| arr.first())
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            ..Song::empty()
        }
    }

    fn map_search_playlist(raw: &serde_json::Value) -> Playlist {
        let id = raw["id"].to_string();
        let name = raw["name"].as_str().unwrap_or("").to_string();
        let cover = raw["coverImgUrl"].as_str().unwrap_or("");
        let track_count = raw["trackCount"].as_u64().unwrap_or(0) as u32;
        let play_count = raw["playCount"].as_u64().unwrap_or(0) as u32;
        let creator = raw["creator"]
            .get("nickname")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        Playlist {
            id: format!("netease_playlist_{}", id),
            name: name.clone(),
            avatar_url: format!("{}?param=300y300", cover),
            small_avatar_url: format!("{}?param=50y50", cover),
            middle_avatar_url: format!("{}?param=200y200", cover),
            title: name,
            creator,
            description: raw["description"].as_str().unwrap_or("").to_string(),
            tags: raw["tags"]
                .as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default(),
            musics_count: track_count,
            play_count,
            adapter_slug: "netease".into(),
            ..Playlist::empty()
        }
    }

    fn map_recommend_playlist(raw: &serde_json::Value) -> Playlist {
        let id = raw["id"].to_string();
        let name = raw["name"].as_str().unwrap_or("").to_string();
        let cover = raw["picUrl"].as_str().unwrap_or("");
        let track_count = raw["trackCount"].as_u64().unwrap_or(0) as u32;
        let play_count = raw["playcount"].as_u64().unwrap_or(0) as u32;
        let creator_name = raw["creator"]
            .get("nickname")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        Playlist {
            id: format!("netease_playlist_{}", id),
            name: name.clone(),
            avatar_url: format!("{}?param=300y300", cover),
            small_avatar_url: format!("{}?param=50y50", cover),
            middle_avatar_url: format!("{}?param=200y200", cover),
            title: name,
            creator: creator_name,
            description: raw["copywriter"].as_str().unwrap_or("").to_string(),
            musics_count: track_count,
            play_count,
            adapter_slug: "netease".into(),
            ..Playlist::empty()
        }
    }

    fn map_playlist_full(raw: &serde_json::Value) -> Playlist {
        // raw is `playlist` from /playlist/detail response
        let playlist = raw.get("playlist").unwrap_or(raw);
        let id = playlist["id"].to_string();
        let name = playlist["name"].as_str().unwrap_or("").to_string();
        let cover = playlist["coverImgUrl"].as_str().unwrap_or("");
        let track_count = playlist["trackCount"].as_u64().unwrap_or(0) as u32;
        let play_count = playlist["playCount"].as_u64().unwrap_or(0) as u32;
        let creator = playlist["creator"]
            .get("nickname")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let create_time = playlist["createTime"]
            .as_i64()
            .map(|ts| ts.to_string())
            .unwrap_or_default();

        // Map tracks if present
        let musics: Vec<Song> = playlist["tracks"]
            .as_array()
            .map(|arr| arr.iter().map(Self::map_song).collect())
            .unwrap_or_default();

        let track_ids: Vec<String> = playlist["trackIds"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .map(|t| format!("netease_song_{}", t["id"]))
                    .collect()
            })
            .unwrap_or_default();

        Playlist {
            id: format!("netease_playlist_{}", id),
            name: name.clone(),
            avatar_url: format!("{}?param=300y300", cover),
            small_avatar_url: format!("{}?param=50y50", cover),
            middle_avatar_url: format!("{}?param=200y200", cover),
            title: name,
            creator,
            create_time,
            description: playlist["description"].as_str().unwrap_or("").to_string(),
            tags: playlist["tags"]
                .as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default(),
            music_track_ids: track_ids,
            musics,
            is_initialized: true,
            musics_count: track_count,
            play_count,
            adapter_slug: "netease".into(),
            ..Playlist::empty()
        }
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

    // -- Music --

    async fn get_song(&self, id: &str) -> Result<Song> {
        let netease_id = id.strip_prefix("netease_song_").unwrap_or(id);
        let raw = self.call_api_ok("/song/detail", &[("ids", netease_id)]).await?;
        let songs = raw["songs"].as_array().ok_or_else(|| Error::SongNotFound(id.into()))?;
        let song = songs.first().ok_or_else(|| Error::SongNotFound(id.into()))?;
        Ok(Self::map_song(song))
    }

    async fn get_songs(&self, ids: &[String]) -> Result<Vec<Song>> {
        let netease_ids: Vec<String> = ids
            .iter()
            .map(|id| id.strip_prefix("netease_song_").unwrap_or(id).to_string())
            .collect();
        let ids_str = netease_ids.join(",");
        let raw = self.call_api_ok("/song/detail", &[("ids", &ids_str)]).await?;
        let songs = raw["songs"]
            .as_array()
            .map(|arr| arr.iter().map(Self::map_song).collect())
            .unwrap_or_default();
        Ok(songs)
    }

    async fn get_song_url(&self, id: &str) -> Result<String> {
        let netease_id = id.strip_prefix("netease_song_").unwrap_or(id);

        for key in &["exhigh", "lossless", "standard"] {
            if let Ok(raw) = self
                .call_api_ok("/song/url/v1", &[("id", netease_id), ("level", key)])
                .await
            {
                if let Some(data) = raw["data"].as_array() {
                    if let Some(first) = data.first() {
                        if let Some(url) = first["url"].as_str() {
                            if !url.is_empty() {
                                return Ok(url.to_string());
                            }
                        }
                    }
                }
            }
        }

        Err(Error::Other("no playable URL found".into()))
    }

    async fn get_lyric(&self, id: &str) -> Result<String> {
        let netease_id = id.strip_prefix("netease_song_").unwrap_or(id);

        // Try /lyric/new first for YRC (word-level) lyrics
        if let Ok(raw) = self.call_api("/lyric/new", &[("id", netease_id)]).await {
            if let Some(yrc) = raw["yrc"]["lyric"].as_str() {
                if !yrc.is_empty() {
                    return Ok(yrc.to_string());
                }
            }
            if let Some(lrc) = raw["lrc"]["lyric"].as_str() {
                if !lrc.is_empty() {
                    return Ok(lrc.to_string());
                }
            }
        }

        // Fall back to old /lyric endpoint
        if let Ok(raw) = self.call_api("/lyric", &[("id", netease_id)]).await {
            if let Some(lrc) = raw["lrc"]["lyric"].as_str() {
                if !lrc.is_empty() {
                    return Ok(lrc.to_string());
                }
            }
        }

        Ok(String::new())
    }

    async fn toggle_like(&self, id: &str, like: bool) -> Result<bool> {
        let netease_id = id.strip_prefix("netease_song_").unwrap_or(id);
        let like_str = if like { "true" } else { "false" };
        let cookie = self.cookies_str();
        if cookie.is_empty() {
            return Err(Error::NotImplemented("login required to like".into()));
        }

        let raw = self
            .call_api_ok("/like", &[("id", netease_id), ("like", like_str)])
            .await?;
        Ok(raw["code"].as_i64().unwrap_or(-1) == 200)
    }

    // -- Album / Artist / Playlist --

    async fn get_album(&self, id: &str) -> Result<Album> {
        let netease_id = id.strip_prefix("netease_album_").unwrap_or(id);
        let raw = self.call_api_ok("/album", &[("id", netease_id)]).await?;
        let album = &raw["album"];

        let artist_raw = &album["artist"];
        let artist = Artist {
            id: format!("netease_artist_{}", artist_raw["id"]),
            name: artist_raw["name"].as_str().unwrap_or("").to_string(),
            avatar_url: artist_raw["picUrl"].as_str().unwrap_or("").to_string(),
            adapter_slug: "netease".into(),
            ..Artist::empty()
        };

        let cover = album["picUrl"].as_str().unwrap_or("");
        let songs: Vec<Song> = album["songs"]
            .as_array()
            .map(|arr| arr.iter().map(Self::map_song).collect())
            .unwrap_or_default();

        Ok(Album {
            id: format!("netease_album_{}", album["id"]),
            name: album["name"].as_str().unwrap_or("").to_string(),
            avatar_url: format!("{}?param=300y300", cover),
            small_avatar_url: format!("{}?param=50y50", cover),
            middle_avatar_url: format!("{}?param=200y200", cover),
            artists: vec![artist.clone()],
            artists_name: artist.name.clone(),
            description: album["description"].as_str().unwrap_or("").to_string(),
            songs,
            adapter_slug: "netease".into(),
            ..Album::empty()
        })
    }

    async fn get_artist(&self, id: &str) -> Result<Artist> {
        let netease_id = id.strip_prefix("netease_artist_").unwrap_or(id);
        let raw = self.call_api_ok("/artists", &[("id", netease_id)]).await?;
        let artist = &raw["artist"];

        Ok(Artist {
            id: format!("netease_artist_{}", artist["id"]),
            name: artist["name"].as_str().unwrap_or("").to_string(),
            avatar_url: artist["picUrl"].as_str().unwrap_or("").to_string(),
            description: artist["briefDesc"].as_str().unwrap_or("").to_string(),
            adapter_slug: "netease".into(),
            ..Artist::empty()
        })
    }

    async fn get_playlist(&self, id: &str) -> Result<Playlist> {
        let netease_id = id.strip_prefix("netease_playlist_").unwrap_or(id);
        let raw = self.call_api_ok("/playlist/detail", &[("id", netease_id), ("s", "50")]).await?;
        Ok(Self::map_playlist_full(&raw))
    }

    // -- Search --

    async fn search(&self, keyword: &str) -> Result<SearchResult> {
        if keyword.trim().is_empty() {
            return Ok(SearchResult {
                songs: vec![],
                albums: vec![],
                artists: vec![],
                playlists: vec![],
            });
        }

        // Search all types: 1=song, 10=album, 100=artist, 1000=playlist
        let limit = "20";
        let songs_raw = self.call_api_ok("/cloudsearch", &[("keywords", keyword), ("type", "1"), ("limit", limit)]).await;
        let albums_raw = self.call_api_ok("/cloudsearch", &[("keywords", keyword), ("type", "10"), ("limit", limit)]).await;
        let artists_raw = self.call_api_ok("/cloudsearch", &[("keywords", keyword), ("type", "100"), ("limit", "10")]).await;
        let playlists_raw = self.call_api_ok("/cloudsearch", &[("keywords", keyword), ("type", "1000"), ("limit", limit)]).await;

        let songs: Vec<Song> = songs_raw
            .ok()
            .and_then(|r| {
                r["result"]["songs"]
                    .as_array()
                    .map(|arr| arr.iter().map(Self::map_song).collect())
            })
            .unwrap_or_default();

        let albums: Vec<Album> = albums_raw
            .ok()
            .and_then(|r| {
                r["result"]["albums"].as_array().map(|arr| {
                    arr.iter()
                        .map(|v| {
                            let cover = v["picUrl"].as_str().unwrap_or("");
                            let artist = Artist {
                                id: format!("netease_artist_{}", v["artist"]["id"]),
                                name: v["artist"]["name"].as_str().unwrap_or("").to_string(),
                                ..Artist::empty()
                            };
                            Album {
                                id: format!("netease_album_{}", v["id"]),
                                name: v["name"].as_str().unwrap_or("").to_string(),
                                avatar_url: format!("{}?param=300y300", cover),
                                small_avatar_url: format!("{}?param=50y50", cover),
                                middle_avatar_url: format!("{}?param=200y200", cover),
                                artists: vec![artist.clone()],
                                artists_name: artist.name,
                                adapter_slug: "netease".into(),
                                ..Album::empty()
                            }
                        })
                        .collect()
                })
            })
            .unwrap_or_default();

        let artists: Vec<Artist> = artists_raw
            .ok()
            .and_then(|r| {
                r["result"]["artists"].as_array().map(|arr| {
                    arr.iter()
                        .map(|v| Artist {
                            id: format!("netease_artist_{}", v["id"]),
                            name: v["name"].as_str().unwrap_or("").to_string(),
                            avatar_url: v["picUrl"].as_str().unwrap_or("").to_string(),
                            adapter_slug: "netease".into(),
                            ..Artist::empty()
                        })
                        .collect()
                })
            })
            .unwrap_or_default();

        let playlists: Vec<Playlist> = playlists_raw
            .ok()
            .and_then(|r| {
                r["result"]["playlists"]
                    .as_array()
                    .map(|arr| arr.iter().map(Self::map_search_playlist).collect())
            })
            .unwrap_or_default();

        Ok(SearchResult {
            songs,
            albums,
            artists,
            playlists,
        })
    }

    // -- Account --

    async fn login_qr_url(&self) -> Result<(String, String)> {
        // Step 1: get unikey
        let key_raw = self.call_api("/login/qr/key", &[]).await?;
        let unikey = key_raw["data"]["unikey"]
            .as_str()
            .ok_or_else(|| Error::Other("failed to get qr key".into()))?
            .to_string();
        *self.qr_key.lock().unwrap() = Some(unikey.clone());

        // Step 2: get QR image (this is generated locally by the proxy, not from NetEase)
        let qr_raw = self
            .call_api("/login/qr/create", &[("key", &unikey), ("qrimg", "true")])
            .await?;
        let qr_img = qr_raw["data"]["qrimg"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok((unikey, qr_img))
    }

    async fn check_login(&self, key: &str) -> Result<LoginStatus> {
        let raw = self.call_api("/login/qr/check", &[("key", key)]).await?;
        let code = raw["code"].as_i64().unwrap_or(-1);

        match code {
            801 => Ok(LoginStatus::Waiting {
                qr_url: String::new(),
            }),
            802 => Ok(LoginStatus::Scanned),
            803 => {
                // Login confirmed — save cookies
                self.save_cookies(&raw);

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

        let raw = self.call_api_ok("/user/account", &[]).await?;
        let user_id = raw["account"]["id"].to_string();
        let nickname = raw["profile"]["nickname"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let avatar = raw["profile"]["avatarUrl"].as_str().unwrap_or("").to_string();

        Ok(Account {
            id: format!("netease_user_{}", user_id),
            name: if nickname.is_empty() { "网易云用户".into() } else { nickname },
            avatar_url: avatar,
            token: cookie,
            is_logged_in: true,
            ..Account::empty()
        })
    }

    async fn get_user_playlists(&self) -> Result<Vec<Playlist>> {
        // Extract uid from cached account, or fetch fresh
        let uid = {
            let account_guard = self.account.lock().unwrap();
            account_guard
                .as_ref()
                .and_then(|a| a.id.strip_prefix("netease_user_").map(|s| s.to_string()))
        };
        // MutexGuard is dropped here — safe to await now

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

        let raw = self
            .call_api_ok("/user/playlist", &[("uid", &uid), ("limit", "50")])
            .await?;

        let playlists: Vec<Playlist> = raw["playlist"]
            .as_array()
            .map(|arr| arr.iter().map(Self::map_search_playlist).collect())
            .unwrap_or_default();

        Ok(playlists)
    }

    // -- Recommend --

    async fn get_recommended_playlists(&self, count: u32) -> Result<Vec<Playlist>> {
        let cookie = self.cookies_str();

        // Try authenticated recommend first, fall back to anonymous /personalized
        let playlists = if !cookie.is_empty() {
            match self.call_api_ok("/recommend/resource", &[]).await {
                Ok(r) => r["recommend"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .take(count as usize)
                            .map(Self::map_recommend_playlist)
                            .collect()
                    })
                    .unwrap_or_default(),
                Err(_) => {
                    let r = self
                        .call_api_ok("/personalized", &[("limit", &count.to_string())])
                        .await?;
                    r["result"]
                        .as_array()
                        .map(|arr| arr.iter().map(Self::map_recommend_playlist).collect())
                        .unwrap_or_default()
                }
            }
        } else {
            let r = self
                .call_api_ok("/personalized", &[("limit", &count.to_string())])
                .await?;
            r["result"]
                .as_array()
                .map(|arr| arr.iter().map(Self::map_recommend_playlist).collect())
                .unwrap_or_default()
        };

        Ok(playlists)
    }

    async fn get_daily_recommended(&self) -> Result<Vec<Song>> {
        let raw = self.call_api_ok("/recommend/songs", &[]).await?;
        let songs: Vec<Song> = raw["data"]["dailySongs"]
            .as_array()
            .map(|arr| arr.iter().map(Self::map_song).collect())
            .unwrap_or_default();
        Ok(songs)
    }
}
