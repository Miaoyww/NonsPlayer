use serde::Deserialize;

// All Netease API responses use camelCase field names. This attribute
// automatically maps track_count -> trackCount, pic_url -> picUrl, etc.
// Single-word fields (id, name, dt, fee, etc.) are unaffected.

// ── /api/v3/song/detail ──

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SongDetailResponse {
    pub songs: Vec<SongItem>,
    pub code: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SongItem {
    pub id: serde_json::Value,
    pub name: String,
    #[serde(default)]
    pub dt: f64,
    #[serde(default)]
    pub fee: i64,
    #[serde(default)]
    pub ar: Vec<ArtistItem>,
    #[serde(default)]
    pub artists: Vec<ArtistItem>,
    #[serde(default)]
    pub al: Option<AlbumItem>,
    #[serde(default)]
    pub album: Option<AlbumItem>,
    #[serde(default)]
    pub alia: Vec<String>,
    #[serde(default)]
    pub mv: serde_json::Value,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ArtistItem {
    pub id: serde_json::Value,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub pic_url: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AlbumItem {
    pub id: serde_json::Value,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub pic_url: Option<String>,
}

// ── /eapi/song/enhance/player/url/v1 ──

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SongUrlResponse {
    pub data: Vec<SongUrlItem>,
    pub code: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SongUrlItem {
    pub id: Option<serde_json::Value>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub br: Option<f64>,
    #[serde(default)]
    pub size: Option<f64>,
    #[serde(default)]
    pub r#type: Option<String>,
    #[serde(default)]
    pub level: Option<String>,
    #[serde(default)]
    pub encode_type: Option<String>,
    #[serde(default)]
    pub time: Option<f64>,
    #[serde(default)]
    pub free_trial_info: Option<serde_json::Value>,
}

// ── /eapi/song/lyric/v1 ──

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricResponse {
    #[serde(default)]
    pub lrc: Option<LyricBlock>,
    #[serde(default)]
    pub tlyric: Option<LyricBlock>,
    #[serde(default)]
    pub romalrc: Option<LyricBlock>,
    #[serde(default)]
    pub yrc: Option<LyricBlock>,
    #[serde(default)]
    pub ytlrc: Option<LyricBlock>,
    #[serde(default)]
    pub yromalrc: Option<LyricBlock>,
    pub code: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricBlock {
    #[serde(default)]
    pub version: i64,
    #[serde(default)]
    pub lyric: String,
}

// ── /api/song/lyric (legacy, plain) ──

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyLyricResponse {
    #[serde(default)]
    pub lrc: Option<LyricBlock>,
    #[serde(default)]
    pub tlyric: Option<LyricBlock>,
    pub code: Option<i64>,
}

// ── /weapi/v1/album/<id> ──

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumResponse {
    pub album: AlbumDetail,
    pub code: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumDetail {
    pub id: serde_json::Value,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub pic_url: Option<String>,
    #[serde(default)]
    pub artist: Option<ArtistItem>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub publish_time: Option<f64>,
    #[serde(default)]
    pub company: Option<String>,
    #[serde(default)]
    pub songs: Vec<SongItem>,
    #[serde(default)]
    pub resource_state: Option<bool>,
}

// ── /weapi/v1/artist/<id> ──

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtistResponse {
    pub artist: ArtistDetail,
    #[serde(default)]
    pub hot_songs: Vec<SongItem>,
    pub code: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtistDetail {
    pub id: serde_json::Value,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub pic_url: Option<String>,
    #[serde(default)]
    pub brief_desc: Option<String>,
    #[serde(default)]
    pub album_size: Option<f64>,
    #[serde(default)]
    pub music_size: Option<f64>,
    #[serde(default)]
    pub mv_size: Option<f64>,
}

// ── /api/v6/playlist/detail ──

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistDetailResponse {
    pub playlist: PlaylistDetail,
    pub code: Option<i64>,
    #[serde(default)]
    pub privileges: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistDetail {
    pub id: serde_json::Value,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub cover_img_url: Option<String>,
    #[serde(default)]
    pub track_count: Option<f64>,
    #[serde(default)]
    pub play_count: Option<f64>,
    #[serde(default)]
    pub creator: Option<PlaylistCreator>,
    #[serde(default)]
    pub create_time: Option<f64>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub track_ids: Vec<TrackIdItem>,
    #[serde(default)]
    pub tracks: Vec<SongItem>,
    #[serde(default)]
    pub special_type: Option<f64>,
    #[serde(default)]
    pub user_id: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackIdItem {
    pub id: serde_json::Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistCreator {
    #[serde(default)]
    pub nickname: String,
}

// ── /eapi/cloudsearch/pc ──

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudSearchResponse {
    pub result: CloudSearchResult,
    pub code: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudSearchResult {
    #[serde(default)]
    pub song_count: Option<f64>,
    #[serde(default)]
    pub songs: Vec<SongItem>,
    #[serde(default)]
    pub album_count: Option<f64>,
    #[serde(default)]
    pub albums: Vec<CloudSearchAlbum>,
    #[serde(default)]
    pub artist_count: Option<f64>,
    #[serde(default)]
    pub artists: Vec<CloudSearchArtist>,
    #[serde(default)]
    pub playlist_count: Option<f64>,
    #[serde(default)]
    pub playlists: Vec<CloudSearchPlaylist>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudSearchAlbum {
    pub id: serde_json::Value,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub pic_url: Option<String>,
    #[serde(default)]
    pub artist: Option<ArtistItem>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudSearchArtist {
    pub id: serde_json::Value,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub pic_url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudSearchPlaylist {
    pub id: serde_json::Value,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub cover_img_url: Option<String>,
    #[serde(default)]
    pub track_count: Option<f64>,
    #[serde(default)]
    pub play_count: Option<f64>,
    #[serde(default)]
    pub creator: Option<PlaylistCreator>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub special_type: Option<f64>,
    #[serde(default)]
    pub user_id: Option<serde_json::Value>,
}

// ── /weapi/login/qrcode/unikey ──

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QrKeyResponse {
    pub data: QrKeyData,
    pub code: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QrKeyData {
    #[serde(default)]
    pub unikey: Option<String>,
    pub code: Option<i64>,
}

// ── /weapi/login/qrcode/client/login ──

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QrCheckResponse {
    pub code: Option<i64>,
    #[serde(default)]
    pub cookie: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
}

// ── /api/nuser/account/get ──

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserAccountResponse {
    pub account: Option<AccountInfo>,
    pub profile: Option<ProfileInfo>,
    pub code: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountInfo {
    pub id: serde_json::Value,
    #[serde(default)]
    pub user_name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileInfo {
    #[serde(default)]
    pub user_id: Option<serde_json::Value>,
    #[serde(default)]
    pub nickname: Option<String>,
    #[serde(default)]
    pub avatar_url: Option<String>,
    #[serde(default)]
    pub signature: Option<String>,
    #[serde(default)]
    pub birthday: Option<f64>,
    #[serde(default)]
    pub gender: Option<f64>,
    #[serde(default)]
    pub followeds: Option<f64>,
    #[serde(default)]
    pub follows: Option<f64>,
    #[serde(default)]
    pub playlist_count: Option<f64>,
}

// ── /api/user/playlist ──

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserPlaylistResponse {
    #[serde(default)]
    pub playlist: Vec<CloudSearchPlaylist>,
    #[serde(default)]
    pub more: Option<bool>,
    pub code: Option<i64>,
}

// ── /weapi/v1/discovery/recommend/resource ──

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecommendResourceResponse {
    #[serde(default)]
    pub recommend: Vec<RecommendPlaylistItem>,
    pub code: Option<i64>,
}

// ── /weapi/personalized/playlist ──

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonalizedResponse {
    #[serde(default)]
    pub result: Vec<RecommendPlaylistItem>,
    pub code: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecommendPlaylistItem {
    pub id: serde_json::Value,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub pic_url: Option<String>,
    #[serde(default)]
    pub track_count: Option<f64>,
    #[serde(default)]
    pub playcount: Option<f64>,
    #[serde(default)]
    pub creator: Option<PlaylistCreator>,
    #[serde(default)]
    pub copywriter: Option<String>,
}

// ── /api/v3/discovery/recommend/songs ──

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecommendSongsResponse {
    pub data: RecommendSongsData,
    pub code: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecommendSongsData {
    #[serde(default)]
    pub daily_songs: Vec<SongItem>,
    #[serde(default)]
    pub recommend_reasons: Vec<serde_json::Value>,
}

// ── /api/register/anonimous ──

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterAnonymousResponse {
    pub code: Option<i64>,
    #[serde(default)]
    pub cookie: Option<String>,
}
