mod library;
#[allow(unused_imports)] // Read-only examples include this module without Tauri command types.
pub use library::{Collection, CollectionPage, EntityDetail, LibrarySummary, TrackPage};
mod discovery;
mod search;
#[allow(unused_imports)] // Standalone probes do not use the Tauri command return type.
pub use discovery::PlaylistCategory;

use crate::model::{AppResult, Lyrics, Track, TrackSource};
use base64::Engine;
use ncm_api_rs::{
    request::{ApiClient, ApiResponse},
    Query,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{future::Future, sync::Mutex, time::Duration};

pub struct Netease {
    client: ApiClient,
    music_client: ApiClient,
    cookie: Mutex<Option<String>>,
    pub(crate) accounts:
        Option<std::sync::Arc<dyn nons_music_adapter_sdk::account::ProviderAccounts>>,
    frozen: bool,
    session_generation: std::sync::atomic::AtomicU64,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QrLogin {
    pub key: String,
    pub image: String,
}

#[derive(Serialize, Deserialize)]
pub struct LoginStatus {
    pub code: i64,
    pub message: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountProfile {
    pub user_id: u64,
    pub nickname: String,
    pub avatar_url: nons_music_adapter_sdk::cover::CoverImage,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SongInformation {
    pub artists: Vec<SongCredit>,
    pub album_id: Option<u64>,
    pub published_at: Option<u64>,
}
#[derive(Serialize, Deserialize)]
pub struct SongCredit {
    pub name: String,
    pub id: Option<u64>,
}

impl Netease {
    pub(crate) async fn invalidate_session(&self, expected_generation: u64) {
        if let Some(accounts) = &self.accounts {
            let expected = accounts.session();
            if expected.generation == expected_generation {
                let _ = accounts.invalidate(expected).await;
            }
            return;
        }
        // A late unauthenticated response from an old account cannot revoke a new login.
        let _ = self.session_generation.compare_exchange(
            expected_generation,
            expected_generation.wrapping_add(1),
            std::sync::atomic::Ordering::AcqRel,
            std::sync::atomic::Ordering::Acquire,
        );
    }
    pub fn session_generation(&self) -> u64 {
        if let Some(accounts) = &self.accounts {
            return accounts.session().generation;
        }
        self.session_generation
            .load(std::sync::atomic::Ordering::Acquire)
    }
    pub fn account_credentials(&self) -> AppResult<Option<String>> {
        if !self.frozen {
            if let Some(accounts) = &self.accounts {
                return accounts
                    .credential()
                    .map_err(|e| e.to_string())?
                    .map(|v| {
                        String::from_utf8(v.expose().to_vec())
                            .map_err(|_| "网易云凭据格式无效".to_owned())
                    })
                    .transpose();
            }
        }
        self.cookie
            .lock()
            .map(|cookie| cookie.clone())
            .map_err(|_| "登录状态锁不可用".into())
    }
    pub fn with_accounts(
        accounts: std::sync::Arc<dyn nons_music_adapter_sdk::account::ProviderAccounts>,
    ) -> Self {
        let client = ncm_api_rs::create_client(None);
        let mut music_client = client.clone();
        music_client.set_response_limit(2 * 1024 * 1024);
        Self {
            client,
            music_client,
            cookie: Mutex::new(None),
            accounts: Some(accounts),
            frozen: false,
            session_generation: Default::default(),
        }
    }
    pub(crate) fn scoped(
        &self,
        context: &nons_music_adapter_sdk::adapter::RequestContext,
    ) -> nons_music_adapter_sdk::MusicResult<Self> {
        if context.session.source.as_str() != "netease" {
            return Err(nons_music_adapter_sdk::ErrorCode::PermissionDenied.into());
        }
        let bytes = if let Some(accounts) = &self.accounts {
            accounts.read(context)?
        } else {
            self.account_credentials()
                .map_err(|_| nons_music_adapter_sdk::ErrorCode::Internal)?
                .map(|s| nons_music_adapter_sdk::account::OpaqueCredential::new(s.into_bytes()))
                .transpose()?
        };
        let cookie = bytes
            .map(|v| {
                String::from_utf8(v.expose().to_vec())
                    .map_err(|_| nons_music_adapter_sdk::ErrorCode::InvalidData)
            })
            .transpose()?;
        // Business endpoints can include full playlist metadata before pagination.
        // Retain the pinned client's bounded 16MiB budget; playback uses music_client.
        let client = self.client.clone();
        Ok(Self {
            client,
            music_client: self.music_client.clone(),
            cookie: Mutex::new(cookie),
            accounts: self.accounts.clone(),
            frozen: true,
            session_generation: std::sync::atomic::AtomicU64::new(context.session.generation),
        })
    }

    pub(crate) async fn song(&self, id: u64) -> nons_music_adapter_sdk::MusicResult<Track> {
        if id == 0 || id > 9_007_199_254_740_991 {
            return Err(nons_music_adapter_sdk::ErrorCode::InvalidData.into());
        }
        let body = music_checked(
            self.music_client.song_detail(
                &self
                    .query()
                    .map_err(|_| nons_music_adapter_sdk::ErrorCode::Internal)?
                    .param("ids", &id.to_string()),
            ),
        )
        .await?;
        body.pointer("/songs/0")
            .and_then(track_from_json)
            .ok_or_else(|| nons_music_adapter_sdk::ErrorCode::NotFound.into())
    }
    /// Probe-only constructor; the caller owns storage and authorization.
    pub fn with_cookie(cookie: Option<String>) -> Self {
        let client = ncm_api_rs::create_client(None);
        let mut music_client = client.clone();
        music_client.set_response_limit(2 * 1024 * 1024);
        Self {
            music_client,
            client,
            cookie: Mutex::new(cookie),
            accounts: None,
            frozen: false,
            session_generation: Default::default(),
        }
    }

    fn query(&self) -> AppResult<Query> {
        let query = Query::new();
        Ok(match self.account_credentials()? {
            Some(cookie) => query.cookie(&cookie),
            None => query,
        })
    }

    pub async fn search(&self, keyword: &str, offset: u32) -> AppResult<Vec<Track>> {
        if keyword.trim().is_empty() || keyword.len() > 256 {
            return Err("请输入不超过 256 字节的搜索词".into());
        }
        let query = self
            .query()?
            .param("keywords", keyword.trim())
            .param("type", "1")
            .param("limit", "50")
            .param("offset", &offset.to_string());
        let body = checked(self.client.cloudsearch(&query)).await?;
        let songs = body.pointer("/result/songs").and_then(Value::as_array);
        Ok(songs
            .into_iter()
            .flatten()
            .filter_map(track_from_json)
            .collect())
    }

    pub async fn liked_song_ids(&self) -> AppResult<Vec<u64>> {
        let profile = self.profile().await?.ok_or("请先登录网易云音乐")?;
        let query = self.query()?.param("uid", &profile.user_id.to_string());
        let body = checked(self.client.likelist(&query)).await?;
        let ids = body
            .get("ids")
            .and_then(Value::as_array)
            .ok_or("收藏列表响应缺少歌曲 ID")?;
        Ok(ids.iter().filter_map(Value::as_u64).collect())
    }

    pub async fn set_song_liked(&self, id: u64, liked: bool) -> AppResult<()> {
        if id == 0 {
            return Err("歌曲 ID 无效".into());
        }
        self.profile().await?.ok_or("请先登录网易云音乐")?;
        let query = self
            .query()?
            .param("id", &id.to_string())
            .param("like", if liked { "true" } else { "false" });
        checked(self.client.like(&query)).await?;
        Ok(())
    }

    pub async fn song_information(&self, id: u64) -> AppResult<SongInformation> {
        let query = self.query()?.param("ids", &id.to_string());
        let body = checked(self.client.song_detail(&query)).await?;
        let song = body.pointer("/songs/0").ok_or("歌曲详情缺失")?;
        let mut information = song_information_from_json(song);
        if information.published_at.is_none() {
            if let Some(album_id) = information.album_id {
                let query = self.query()?.param("id", &album_id.to_string());
                if let Ok(album) = checked(self.client.album(&query)).await {
                    information.published_at = album
                        .pointer("/album/publishTime")
                        .and_then(Value::as_u64)
                        .filter(|v| *v > 0);
                }
            }
        }
        Ok(information)
    }

    pub(crate) async fn resolve_resource(
        &self,
        id: u64,
        quality: &str,
        allow_downgrade: bool,
    ) -> nons_music_adapter_sdk::MusicResult<nons_music_adapter_sdk::resource::PlaybackResource>
    {
        use nons_music_adapter_sdk::{resource::*, ErrorCode};
        let query = self
            .query()
            .map_err(|_| ErrorCode::Internal)?
            .param("id", &id.to_string())
            .param("level", quality);
        let body = music_checked(self.music_client.song_url_v1(&query)).await?;
        let item = body.pointer("/data/0").ok_or(ErrorCode::NotFound)?;
        if item.get("id").and_then(Value::as_u64) != Some(id) {
            return Err(ErrorCode::InvalidData.into());
        }
        let uri = item
            .get("url")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .ok_or(ErrorCode::PermissionDenied)?;
        let actual = item
            .get("level")
            .and_then(Value::as_str)
            .filter(|v| !v.is_empty())
            .ok_or(ErrorCode::InvalidData)?
            .to_string();
        if !allow_downgrade && actual != quality {
            return Err(ErrorCode::PermissionDenied.into());
        }
        let extent = match item.get("freeTrialInfo").filter(|v| !v.is_null()) {
            Some(trial) => PlaybackExtent::Preview {
                start_ms: trial
                    .get("start")
                    .and_then(Value::as_u64)
                    .ok_or(ErrorCode::InvalidData)?
                    .checked_mul(1000)
                    .ok_or(ErrorCode::InvalidData)?,
                end_ms: trial
                    .get("end")
                    .and_then(Value::as_u64)
                    .ok_or(ErrorCode::InvalidData)?
                    .checked_mul(1000)
                    .ok_or(ErrorCode::InvalidData)?,
            },
            None => PlaybackExtent::Full,
        };
        let expires_at_ms = item
            .get("expi")
            .and_then(Value::as_u64)
            .and_then(|seconds| seconds.checked_mul(1000))
            .and_then(|ttl| crate::now_ms().checked_add(ttl));
        Ok(PlaybackResource {
            metadata: ResourceMetadata {
                actual_quality: actual,
                extent,
                expires_at_ms,
            },
            access: ResourceAccess::Http(HttpAccess {
                url: uri.into(),
                headers: vec![],
            }),
        })
    }

    pub async fn lyrics(&self, id: u64) -> AppResult<Option<Lyrics>> {
        let query = self.query()?.param("id", &id.to_string());
        let body = checked(self.client.lyric_new(&query)).await?;
        let word = body
            .pointer("/yrc/lyric")
            .and_then(Value::as_str)
            .filter(|s| !s.trim().is_empty());
        let line = body
            .pointer("/lrc/lyric")
            .and_then(Value::as_str)
            .filter(|s| !s.trim().is_empty());
        let Some(content) = word.or(line) else {
            return Ok(None);
        };
        if content.len() > crate::model::MAX_LYRIC_BYTES {
            return Err("网易云歌词过大".into());
        }
        Ok(Some(Lyrics {
            match_score: None,
            source: "netease".into(),
            format: if word.is_some() { "yrc" } else { "lrc" }.into(),
            content: content.into(),
            translation: body
                .pointer(if word.is_some() {
                    "/ytlrc/lyric"
                } else {
                    "/tlyric/lyric"
                })
                .and_then(Value::as_str)
                .map(str::to_owned),
            romanization: body
                .pointer(if word.is_some() {
                    "/yromalrc/lyric"
                } else {
                    "/romalrc/lyric"
                })
                .and_then(Value::as_str)
                .map(str::to_owned),
        }))
    }

    pub async fn qr_login(&self) -> AppResult<QrLogin> {
        let query = self.query()?;
        let body = checked(self.client.login_qr_key(&query)).await?;
        let key = qr_key(&body)?.to_string();
        let query = Query::new().param("key", &key);
        let body = checked(self.client.login_qr_create(&query)).await?;
        let url = body
            .pointer("/data/qrurl")
            .and_then(Value::as_str)
            .ok_or("二维码地址缺失")?;
        let qr = qrcode::QrCode::new(url).map_err(|_| "无法生成二维码图片")?;
        let svg = qr
            .render::<qrcode::render::svg::Color>()
            .min_dimensions(224, 224)
            .build();
        let image = format!(
            "data:image/svg+xml;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(svg)
        );
        Ok(QrLogin { key, image })
    }

    pub(crate) async fn poll_completion(
        &self,
        key: &str,
    ) -> AppResult<(
        LoginStatus,
        Option<nons_music_adapter_sdk::account::OpaqueCredential>,
    )> {
        if key.is_empty() || key.len() > 256 {
            return Err("登录二维码已失效，请重新生成".into());
        }
        let query = Query::new().param("key", key);
        let response = timed(self.client.login_qr_check(&query)).await?;
        let code = response
            .body
            .get("code")
            .and_then(Value::as_i64)
            .unwrap_or(response.status);
        let mut completion = None;
        if code == 803 {
            let mut tokens = std::collections::BTreeMap::new();
            for cookie in &response.cookie {
                if let Ok(cookie) = cookie::Cookie::parse(cookie.as_str()) {
                    if matches!(cookie.name(), "MUSIC_U" | "__csrf" | "MUSIC_A" | "NMTID") {
                        tokens.insert(cookie.name().to_string(), cookie.value().to_string());
                    }
                }
            }
            if !tokens.contains_key("MUSIC_U") {
                return Err("登录返回缺少有效会话，请重新扫码".into());
            }
            let value = tokens
                .into_iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join("; ");
            completion = Some(
                nons_music_adapter_sdk::account::OpaqueCredential::new(value.into_bytes())
                    .map_err(|e| e.to_string())?,
            );
        }
        let message = match code {
            800 => "二维码已过期",
            801 => "等待扫码",
            802 => "请在网易云音乐中确认",
            803 => "登录成功",
            _ => "登录状态异常，请重试",
        };
        Ok((
            LoginStatus {
                code,
                message: message.into(),
            },
            completion,
        ))
    }

    pub async fn profile(&self) -> AppResult<Option<AccountProfile>> {
        if self.account_credentials()?.is_none() {
            return Ok(None);
        }
        let generation = self.session_generation();
        let query = self.query()?;
        let body = checked(self.client.login_status(&query)).await?;
        let profile = account_profile(&body);
        if profile.is_none() && !self.frozen {
            self.invalidate_session(generation).await;
        }
        Ok(profile)
    }

    pub(crate) async fn remote_logout(&self) -> nons_music_adapter_sdk::MusicResult<()> {
        music_checked(
            self.music_client.logout(
                &self
                    .query()
                    .map_err(|_| nons_music_adapter_sdk::ErrorCode::Internal)?,
            ),
        )
        .await?;
        Ok(())
    }
    pub(crate) async fn profile_with_credential(
        &self,
        credential: &nons_music_adapter_sdk::account::OpaqueCredential,
    ) -> AppResult<Option<AccountProfile>> {
        let cookie = std::str::from_utf8(credential.expose()).map_err(|_| "网易云凭据格式无效")?;
        let body = checked(self.client.login_status(&Query::new().cookie(cookie))).await?;
        Ok(account_profile(&body))
    }
}

fn account_profile(body: &Value) -> Option<AccountProfile> {
    let profile = body
        .pointer("/data/profile")
        .or_else(|| body.get("profile"))?;
    let user_id = profile.get("userId")?.as_u64().filter(|id| *id > 0)?;
    Some(AccountProfile {
        user_id,
        nickname: profile
            .get("nickname")
            .and_then(Value::as_str)
            .filter(|name| !name.is_empty())
            .unwrap_or("网易云用户")
            .into(),
        avatar_url: crate::cover::image(
            profile
                .get("avatarUrl")
                .and_then(Value::as_str)
                .unwrap_or_default(),
        ),
    })
}

async fn timed<F: Future<Output = ncm_api_rs::error::Result<ApiResponse>>>(
    future: F,
) -> AppResult<ApiResponse> {
    tokio::time::timeout(Duration::from_secs(12), future)
        .await
        .map_err(|_| "网易云请求超时，请重试")?
        .map_err(|_| "网易云请求失败，请检查网络后重试".into())
}

async fn checked<F: Future<Output = ncm_api_rs::error::Result<ApiResponse>>>(
    future: F,
) -> AppResult<Value> {
    checked_with_limit(future, 16 * 1024 * 1024)
        .await
        .map_err(|error| error.to_string())
}

fn qr_key(body: &Value) -> AppResult<&str> {
    body.get("unikey")
        .and_then(Value::as_str)
        .filter(|key| !key.trim().is_empty())
        .or_else(|| {
            body.pointer("/data/unikey")
                .and_then(Value::as_str)
                .filter(|key| !key.trim().is_empty())
        })
        .ok_or_else(|| "网易云二维码响应缺少有效登录 key，请重新生成".into())
}

fn song_information_from_json(song: &Value) -> SongInformation {
    SongInformation {
        artists: song
            .get("ar")
            .or_else(|| song.get("artists"))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .map(|artist| SongCredit {
                name: artist
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .into(),
                id: artist.get("id").and_then(Value::as_u64).filter(|v| *v > 0),
            })
            .collect(),
        album_id: song
            .get("al")
            .or_else(|| song.get("album"))
            .and_then(|album| album.get("id"))
            .and_then(Value::as_u64)
            .filter(|v| *v > 0),
        published_at: song
            .get("publishTime")
            .and_then(Value::as_u64)
            .filter(|v| *v > 0),
    }
}

fn song_aliases(song: &Value) -> Vec<String> {
    let title = song.get("name").and_then(Value::as_str).unwrap_or_default();
    let mut names = Vec::new();
    for field in ["tns", "transNames", "alia", "alias"] {
        if let Some(values) = song.get(field).and_then(Value::as_array) {
            for value in values {
                if let Some(name) = value.as_str().map(str::trim).filter(|s| !s.is_empty()) {
                    if name != title && !names.iter().any(|existing| existing == name) {
                        names.push(name.to_owned());
                    }
                }
            }
        }
    }
    names
}

fn track_from_json(song: &Value) -> Option<Track> {
    let id = song.get("id")?.as_u64()?;
    let artists = song.get("ar").or_else(|| song.get("artists"))?.as_array()?;
    let artist = artists
        .iter()
        .filter_map(|v| v.get("name")?.as_str())
        .collect::<Vec<_>>()
        .join(" / ");
    let album = song.get("al").or_else(|| song.get("album"));
    Some(Track {
        key: format!("netease:{id}"),
        title: song.get("name")?.as_str()?.into(),
        aliases: song_aliases(song),
        artists: artists
            .iter()
            .filter_map(|v| {
                Some(crate::model::MusicCredit {
                    name: v.get("name")?.as_str()?.into(),
                    id: v.get("id").and_then(Value::as_u64).filter(|id| *id > 0),
                })
            })
            .collect(),
        album_id: album
            .and_then(|v| v.get("id"))
            .and_then(Value::as_u64)
            .filter(|id| *id > 0),
        artist: if artist.is_empty() {
            "未知艺术家".into()
        } else {
            artist
        },
        album: album
            .and_then(|v| v.get("name"))
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .unwrap_or("未知专辑")
            .into(),
        duration_ms: song
            .get("dt")
            .or_else(|| song.get("duration"))
            .and_then(Value::as_u64)
            .unwrap_or(0),
        cover: crate::cover::image(
            album
                .and_then(|v| v.get("picUrl"))
                .and_then(Value::as_str)
                .unwrap_or(""),
        ),
        source: TrackSource::Netease { id },
    })
}

// The patched pinned client also bounds the body before JSON decoding.
async fn music_checked<F: Future<Output = ncm_api_rs::error::Result<ApiResponse>>>(
    future: F,
) -> nons_music_adapter_sdk::MusicResult<Value> {
    checked_with_limit(future, 2 * 1024 * 1024).await
}

async fn checked_with_limit<F: Future<Output = ncm_api_rs::error::Result<ApiResponse>>>(
    future: F,
    response_limit: usize,
) -> nons_music_adapter_sdk::MusicResult<Value> {
    use nons_music_adapter_sdk::ErrorCode;
    let response = tokio::time::timeout(Duration::from_secs(12), future)
        .await
        .map_err(|_| ErrorCode::DeadlineExceeded)?
        .map_err(music_error)?;
    match response
        .body
        .get("code")
        .and_then(Value::as_i64)
        .unwrap_or(response.status)
    {
        200 => {
            if serde_json::to_vec(&response.body)
                .map_err(|_| ErrorCode::InvalidData)?
                .len()
                > response_limit
            {
                return Err(ErrorCode::InvalidData.into());
            }
            Ok(response.body)
        }
        301 | 302 => Err(ErrorCode::Unauthenticated.into()),
        403 => Err(ErrorCode::PermissionDenied.into()),
        404 => Err(ErrorCode::NotFound.into()),
        429 => Err(ErrorCode::RateLimited.into()),
        _ => Err(ErrorCode::Network.into()),
    }
}

fn music_error(error: ncm_api_rs::error::NcmError) -> nons_music_adapter_sdk::MusicError {
    use ncm_api_rs::error::NcmError;
    use nons_music_adapter_sdk::ErrorCode;
    let code = match error {
        NcmError::AuthRequired(_) => ErrorCode::Unauthenticated,
        NcmError::RateLimited(_) => ErrorCode::RateLimited,
        NcmError::Timeout(_) => ErrorCode::DeadlineExceeded,
        NcmError::InvalidParam(_) | NcmError::ResponseTooLarge | NcmError::Json(_) => {
            ErrorCode::InvalidData
        }
        NcmError::Api { code: 403, .. } => ErrorCode::PermissionDenied,
        NcmError::Api { code: 404, .. } => ErrorCode::NotFound,
        NcmError::Api { code: 451, .. } => ErrorCode::RegionRestricted,
        NcmError::Api {
            code: 429 | 503, ..
        } => ErrorCode::RateLimited,
        NcmError::Api {
            code: 301 | 302, ..
        } => ErrorCode::Unauthenticated,
        _ => ErrorCode::Network,
    };
    code.into()
}

#[cfg(test)]
mod tests {
    use super::{
        account_profile, checked, music_checked, qr_key, song_aliases, song_information_from_json,
        ApiResponse, Track,
    };
    use serde_json::json;

    #[tokio::test]
    async fn large_liked_playlist_raw_body_does_not_fail_the_small_preview() {
        // The platform returns full metadata before the host selects 12 preview IDs.
        let body = json!({"code":200,"playlist":{"trackIds":[{"id":1}],
            "tracks":[{"description":"x".repeat(3 * 1024 * 1024)}]}});
        // Exercise the scoped transport too: changing only post-decode validation
        // would still fail while the HTTP body is being accumulated.
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let raw = serde_json::to_vec(&body).unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(2)))
                .unwrap();
            let mut request = [0; 8192];
            let _ = stream.read(&mut request).unwrap();
            let header = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", raw.len());
            let _ = stream.write_all(header.as_bytes());
            let _ = stream.write_all(&raw);
        });
        let client = ncm_api_rs::create_client(None);
        let mut music_client = client.clone();
        music_client.set_response_limit(2 * 1024 * 1024);
        let api = super::Netease {
            client,
            music_client,
            cookie: Default::default(),
            accounts: None,
            frozen: false,
            session_generation: Default::default(),
        };
        let context = nons_music_adapter_sdk::adapter::RequestContext {
            session: nons_music_adapter_sdk::account::SessionContext {
                source: nons_music_adapter_sdk::identity::SourceId::try_from("netease".to_owned())
                    .unwrap(),
                account: None,
                generation: 0,
            },
            adapter_generation: 1,
            deadline: std::time::Instant::now() + std::time::Duration::from_secs(12),
            cancellation: Default::default(),
        };
        let scoped = api.scoped(&context).unwrap();
        let received = checked(scoped.client.request(
            "/api/v6/playlist/detail",
            json!({"id":1}),
            ncm_api_rs::request::RequestOption {
                crypto: ncm_api_rs::request::CryptoType::Api,
                domain: Some(format!("http://{address}")),
                ..Default::default()
            },
        ))
        .await;
        server.join().unwrap();
        assert!(
            received.is_ok(),
            "scoped playlist transport must accept bounded large bodies"
        );
        let result = checked(async {
            Ok(ApiResponse {
                status: 200,
                body,
                cookie: vec![],
            })
        })
        .await;
        assert!(
            result.is_ok(),
            "valid large playlist must not surface invalidData in preview: {result:?}"
        );
        assert!(
            music_checked(async {
                Ok(ApiResponse {
                    status: 200,
                    body: json!({"code":200,"padding":"x".repeat(3 * 1024 * 1024)}),
                    cookie: vec![],
                })
            })
            .await
            .is_err(),
            "single-track/playback budget stays strict"
        );
        assert!(
            checked(async {
                Ok(ApiResponse {
                    status: 200,
                    body: json!({"code":200,"padding":"x".repeat(16 * 1024 * 1024)}),
                    cookie: vec![],
                })
            })
            .await
            .is_err(),
            "business response budget stays bounded"
        );
    }

    #[test]
    fn song_navigation_keeps_structured_ids_without_splitting_artist_names() {
        let track = super::track_from_json(&json!({"id":1,"name":"Song",
            "ar":[{"id":9,"name":"AC/DC"},{"id":10,"name":"B"}],"al":{"id":20,"name":"Album"}}))
        .unwrap();
        assert_eq!(track.artists.len(), 2);
        assert_eq!(track.artists[0].name, "AC/DC");
        assert_eq!(track.artists[1].id, Some(10));
        assert_eq!(track.album_id, Some(20));
    }

    #[test]
    fn song_information_keeps_multiple_artist_ids_and_missing_fields_distinct() {
        let value = song_information_from_json(
            &serde_json::json!({"ar":[{"name":"甲","id":11},{"name":"乙","id":12}],"al":{"id":21},"publishTime":1595520000000u64}),
        );
        assert_eq!(value.artists.len(), 2);
        assert_eq!(value.artists[1].id, Some(12));
        assert_eq!(value.album_id, Some(21));
        assert_eq!(value.published_at, Some(1595520000000));
        let absent = song_information_from_json(
            &serde_json::json!({"artists":[{"name":"未知","id":0}],"album":{"id":0},"publishTime":0}),
        );
        assert_eq!(absent.artists[0].id, None);
        assert_eq!(absent.album_id, None);
        assert_eq!(absent.published_at, None);
    }
    #[test]
    fn aliases_merge_translation_and_alias_shapes_without_duplicates() {
        let song = json!({"name":"Причал", "tns":["码头", " "], "transNames":["码头"], "alia":["别名", "Причал"], "alias":["别名", "其他"]});
        assert_eq!(song_aliases(&song), vec!["码头", "别名", "其他"]);
        assert!(song_aliases(&json!({"name":"Song"})).is_empty());
    }

    #[test]
    fn stored_tracks_without_aliases_remain_readable() {
        let track: Track = serde_json::from_value(json!({"key":"netease:1", "title":"Song", "artist":"Artist", "album":"Album", "durationMs":1000, "cover":"", "source":{"kind":"netease", "id":1}})).unwrap();
        assert!(track.aliases.is_empty());
    }

    #[test]
    fn account_profile_accepts_both_response_shapes_and_rejects_logged_out() {
        let profile = json!({"userId": 123, "nickname": "Listener", "avatarUrl": "https://example.com/avatar.png"});
        for body in [
            json!({"profile": profile}),
            json!({"data": {"profile": profile}}),
        ] {
            let account = account_profile(&body).unwrap();
            assert_eq!(account.nickname, "Listener");
            assert_eq!(account.avatar_url, "https://example.com/avatar.png");
        }
        for body in [
            json!({"data": {"profile": null}}),
            json!({"profile": {"userId": 0}}),
        ] {
            assert!(account_profile(&body).is_none());
        }
    }

    #[test]
    fn qr_key_accepts_sdk_root_response() {
        assert_eq!(
            qr_key(&json!({"code": 200, "unikey": "test-key"})).unwrap(),
            "test-key"
        );
    }

    #[test]
    fn qr_key_accepts_wrapped_response() {
        assert_eq!(
            qr_key(&json!({"code": 200, "data": {"unikey": "test-key"}})).unwrap(),
            "test-key"
        );
    }

    #[test]
    fn qr_key_rejects_invalid_response() {
        for body in [
            json!({"code": 200}),
            json!({"unikey": ""}),
            json!({"unikey": 123}),
        ] {
            assert!(qr_key(&body).is_err());
        }
    }
}
