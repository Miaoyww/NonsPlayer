use super::{checked, track_from_json, AccountProfile, Netease};
use crate::model::{AppResult, Track};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

const COLLECTION_LIMIT: usize = 30;
const TRACK_LIMIT: usize = 100;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Collection {
    pub id: u64,
    pub kind: String,
    pub name: String,
    pub cover: nons_music_adapter_sdk::cover::CoverImage,
    pub subtitle: String,
    pub track_count: u64,
    pub creator_id: u64,
    pub liked: bool,
    pub play_count: Option<u64>,
    pub published_at: Option<u64>,
    pub artists: Vec<crate::model::MusicCredit>,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionPage {
    pub items: Vec<Collection>,
    pub more: bool,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityDetail {
    pub item: Collection,
    pub description: Option<String>,
    pub album_count: Option<u64>,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackPage {
    pub description: Option<String>,
    pub tracks: Vec<Track>,
    pub total: usize,
    pub more: bool,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibrarySummary {
    pub profile: AccountProfile,
    pub liked_playlist: Option<Collection>,
    pub liked_tracks: Vec<Track>,
    pub liked_error: Option<String>,
}

impl Netease {
    async fn owned_playlist(&self, id: u64) -> AppResult<Value> {
        if id == 0 {
            return Err("歌单 ID 无效".into());
        }
        let profile = self.library_account().await?;
        let body = checked(
            self.client
                .playlist_detail(&self.query()?.param("id", &id.to_string())),
        )
        .await?;
        let playlist = body.get("playlist").ok_or("歌单详情缺失")?;
        if playlist.pointer("/creator/userId").and_then(Value::as_u64) != Some(profile.user_id) {
            return Err("只能编辑自己创建的歌单".into());
        }
        Ok(playlist.clone())
    }

    pub async fn add_playlist_song(&self, playlist_id: u64, song_id: u64) -> AppResult<()> {
        if song_id == 0 {
            return Err("歌曲 ID 无效".into());
        }
        let playlist = self.owned_playlist(playlist_id).await?;
        if playlist.get("specialType").and_then(Value::as_u64) == Some(5) {
            return self.set_song_liked(song_id, true).await;
        }
        checked(
            self.client.playlist_tracks(
                &self
                    .query()?
                    .param("op", "add")
                    .param("pid", &playlist_id.to_string())
                    .param("tracks", &song_id.to_string()),
            ),
        )
        .await?;
        Ok(())
    }

    pub async fn update_library_playlist(
        &self,
        id: u64,
        name: &str,
        description: &str,
    ) -> AppResult<()> {
        if name.trim().is_empty() || name.chars().count() > 40 || description.chars().count() > 1000
        {
            return Err("歌单名称须为 1–40 个字符，简介最多 1000 个字符".into());
        }
        let playlist = self.owned_playlist(id).await?;
        if playlist.get("specialType").and_then(Value::as_u64) == Some(5) {
            return Err("不能编辑我喜欢的音乐的信息".into());
        }
        // The SDK batch endpoint interpolates JSON strings; escape user text and preserve tags.
        let escape = |text: &str| -> AppResult<String> {
            let encoded = serde_json::to_string(text).map_err(|e| e.to_string())?;
            Ok(encoded[1..encoded.len() - 1].to_string())
        };
        let tags = array(&playlist, "tags")
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join(";");
        let body = checked(
            self.client.playlist_update(
                &self
                    .query()?
                    .param("id", &id.to_string())
                    .param("name", &escape(name.trim())?)
                    .param("desc", &escape(description)?)
                    .param("tags", &escape(&tags)?),
            ),
        )
        .await?;
        for key in [
            "/api/playlist/desc/update",
            "/api/playlist/tags/update",
            "/api/playlist/update/name",
        ] {
            if body
                .get(key)
                .and_then(|v| v.get("code"))
                .and_then(Value::as_u64)
                != Some(200)
            {
                return Err("部分歌单信息未能保存，请刷新后重试".into());
            }
        }
        Ok(())
    }

    pub async fn delete_library_playlist(&self, id: u64) -> AppResult<()> {
        let playlist = self.owned_playlist(id).await?;
        if playlist.get("specialType").and_then(Value::as_u64) == Some(5) {
            return Err("不能删除我喜欢的音乐".into());
        }
        checked(
            self.client
                .playlist_delete(&self.query()?.param("id", &id.to_string())),
        )
        .await?;
        Ok(())
    }

    async fn library_account(&self) -> AppResult<AccountProfile> {
        self.profile()
            .await?
            .ok_or_else(|| "请先登录网易云音乐".into())
    }

    pub async fn remove_playlist_song(&self, playlist_id: u64, song_id: u64) -> AppResult<()> {
        if playlist_id == 0 || song_id == 0 {
            return Err("歌单或歌曲 ID 无效".into());
        }
        let profile = self.library_account().await?;
        let query = self.query()?.param("id", &playlist_id.to_string());
        let body = checked(self.client.playlist_detail(&query)).await?;
        let playlist = body.get("playlist").ok_or("歌单详情缺失")?;
        if playlist.pointer("/creator/userId").and_then(Value::as_u64) != Some(profile.user_id) {
            return Err("只能编辑自己创建的歌单".into());
        }
        if playlist.get("specialType").and_then(Value::as_u64) == Some(5) {
            return self.set_song_liked(song_id, false).await;
        }
        let query = self
            .query()?
            .param("op", "del")
            .param("pid", &playlist_id.to_string())
            .param("tracks", &song_id.to_string());
        checked(self.client.playlist_tracks(&query)).await?;
        Ok(())
    }

    pub async fn library_summary(&self) -> AppResult<LibrarySummary> {
        let profile = self.library_account().await?;
        let query = self
            .query()?
            .param("uid", &profile.user_id.to_string())
            .param("limit", "30")
            .param("offset", "0");
        let body = checked(self.client.user_playlist(&query)).await?;
        let liked_playlist = array(&body, "playlist")
            .iter()
            .filter_map(|v| collection(v, "playlist"))
            .find(|c| c.liked && c.creator_id == profile.user_id);
        let (liked_tracks, liked_error) = if let Some(liked) = &liked_playlist {
            match self.library_tracks("playlist", liked.id, 0, 12).await {
                Ok(page) => (page.tracks, None),
                Err(error) => (Vec::new(), Some(error)),
            }
        } else {
            (Vec::new(), None)
        };
        Ok(LibrarySummary {
            profile,
            liked_playlist,
            liked_tracks,
            liked_error,
        })
    }

    pub async fn library_collections(
        &self,
        kind: &str,
        offset: u32,
        filter: &str,
    ) -> AppResult<CollectionPage> {
        if offset > 100_000 || !["all", "mine", "liked"].contains(&filter) {
            return Err("收藏分页参数无效".into());
        }
        let mut query = self
            .query()?
            .param("limit", &COLLECTION_LIMIT.to_string())
            .param("offset", &offset.to_string());
        let mut owner_id = 0;
        let body = match kind {
            "playlist" => {
                let profile = self.library_account().await?;
                owner_id = profile.user_id;
                query = query.param("uid", &profile.user_id.to_string());
                checked(self.client.user_playlist(&query)).await?
            }
            "album" => checked(self.client.album_sublist(&query)).await?,
            "artist" => checked(self.client.artist_sublist(&query)).await?,
            _ => return Err("收藏类型无效".into()),
        };
        let values = array(
            &body,
            if kind == "playlist" {
                "playlist"
            } else {
                "data"
            },
        );
        let more = body
            .get("more")
            .or_else(|| body.get("hasMore"))
            .and_then(Value::as_bool)
            .unwrap_or(values.len() == COLLECTION_LIMIT);
        let items = values
            .iter()
            .take(COLLECTION_LIMIT)
            .filter_map(|v| collection(v, kind))
            .filter(|c| collection_matches(c, kind, filter, owner_id))
            .collect();
        Ok(CollectionPage { items, more })
    }

    pub async fn library_tracks(
        &self,
        kind: &str,
        id: u64,
        offset: u32,
        limit: usize,
    ) -> AppResult<TrackPage> {
        if limit > TRACK_LIMIT {
            return Err("歌曲分页参数无效".into());
        }
        self.collection_tracks(kind, id, offset, limit).await
    }

    async fn collection_tracks(
        &self,
        kind: &str,
        id: u64,
        offset: u32,
        limit: usize,
    ) -> AppResult<TrackPage> {
        if id == 0 || offset > 100_000 || limit == 0 || limit > 1000 {
            return Err("歌曲分页参数无效".into());
        }
        let query = self.query()?.param("id", &id.to_string());
        match kind {
            "playlist" => {
                let body = checked(self.client.playlist_detail(&query)).await?;
                let playlist = body.get("playlist").ok_or("歌单详情缺失")?;
                let ids = playlist_ids(playlist, offset as usize, limit);
                let total = array(playlist, "trackIds").len();
                let description = collection_description(playlist);
                if ids.is_empty() {
                    return Ok(TrackPage {
                        description,
                        tracks: Vec::new(),
                        total,
                        more: false,
                    });
                }
                let query = self.query()?.param(
                    "ids",
                    &ids.iter().map(u64::to_string).collect::<Vec<_>>().join(","),
                );
                let songs = checked(self.client.song_detail(&query)).await?;
                // Match detail results to the playlist's order, including removed/unavailable songs.
                Ok(TrackPage {
                    description,
                    tracks: ordered_tracks(&ids, array(&songs, "songs")),
                    total,
                    more: (offset as usize).saturating_add(limit) < total,
                })
            }
            "album" | "artist" => {
                let body = if kind == "album" {
                    checked(self.client.album(&query)).await?
                } else {
                    checked(self.client.artist_top_song(&query)).await?
                };
                let songs = array(&body, "songs");
                Ok(track_page(
                    songs.iter(),
                    songs.len(),
                    offset as usize,
                    limit,
                ))
            }
            _ => Err("歌曲集合类型无效".into()),
        }
    }

    pub async fn music_entity_detail(&self, kind: &str, id: u64) -> AppResult<EntityDetail> {
        if id == 0 {
            return Err("音乐 ID 无效".into());
        }
        let query = self.query()?.param("id", &id.to_string());
        let body = match kind {
            "artist" => checked(self.client.artist_detail(&query)).await?,
            "album" => checked(self.client.album(&query)).await?,
            "playlist" => checked(self.client.playlist_detail(&query)).await?,
            _ => return Err("音乐详情类型无效".into()),
        };
        let value = if kind == "artist" {
            body.pointer("/data/artist")
        } else if kind == "playlist" {
            body.get("playlist")
        } else {
            body.get("album")
        }
        .ok_or("音乐详情缺失")?;
        let mut item = collection(value, kind).ok_or("音乐详情 ID 缺失")?;
        if kind == "artist" {
            item.cover = value
                .get("avatar")
                .or_else(|| value.get("cover"))
                .and_then(Value::as_str)
                .unwrap_or(&item.cover)
                .into();
            item.track_count = value.get("musicSize").and_then(Value::as_u64).unwrap_or(0);
        }
        Ok(EntityDetail {
            item,
            description: collection_description(value).or_else(|| {
                value
                    .get("briefDesc")
                    .and_then(Value::as_str)
                    .map(|v| v.chars().take(16_000).collect())
            }),
            album_count: value.get("albumSize").and_then(Value::as_u64),
        })
    }

    pub async fn artist_albums(&self, id: u64, offset: u32) -> AppResult<CollectionPage> {
        if id == 0 || offset > 100_000 {
            return Err("专辑分页参数无效".into());
        }
        let query = self
            .query()?
            .param("id", &id.to_string())
            .param("limit", "30")
            .param("offset", &offset.to_string());
        let body = checked(self.client.artist_album(&query)).await?;
        let items = array(&body, "hotAlbums")
            .iter()
            .filter_map(|v| collection(v, "album"))
            .collect();
        Ok(CollectionPage {
            items,
            more: body.get("more").and_then(Value::as_bool).unwrap_or(false),
        })
    }

    pub async fn artist_tracks(&self, id: u64, offset: u32) -> AppResult<TrackPage> {
        if id == 0 || offset > 100_000 {
            return Err("歌曲分页参数无效".into());
        }
        let query = self
            .query()?
            .param("id", &id.to_string())
            .param("limit", "100")
            .param("offset", &offset.to_string())
            .param("order", "hot");
        let body = checked(self.client.artist_songs(&query)).await?;
        let songs = array(&body, "songs");
        Ok(TrackPage {
            description: None,
            tracks: songs.iter().filter_map(track_from_json).collect(),
            total: body
                .get("total")
                .and_then(Value::as_u64)
                .unwrap_or(songs.len() as u64) as usize,
            more: body.get("more").and_then(Value::as_bool).unwrap_or(false),
        })
    }

    pub async fn library_history(&self, week: bool, offset: u32) -> AppResult<TrackPage> {
        if offset > 100_000 {
            return Err("听歌记录分页参数无效".into());
        }
        let profile = self.library_account().await?;
        let query = self
            .query()?
            .param("uid", &profile.user_id.to_string())
            .param("type", if week { "1" } else { "0" });
        let body = checked(self.client.user_record(&query)).await?;
        let values = array(&body, if week { "weekData" } else { "allData" });
        Ok(track_page(
            values.iter().map(|v| &v["song"]),
            values.len(),
            offset as usize,
            TRACK_LIMIT,
        ))
    }

    pub async fn library_create_playlist(&self, name: &str, private: bool) -> AppResult<()> {
        if name.trim().is_empty() || name.chars().count() > 40 {
            return Err("歌单名称须为 1–40 个字符".into());
        }
        self.library_account().await?;
        let query = self
            .query()?
            .param("name", name.trim())
            .param("privacy", if private { "10" } else { "0" })
            .param("type", "NORMAL");
        checked(self.client.playlist_create(&query)).await?;
        Ok(())
    }
}

pub(super) fn array<'a>(value: &'a Value, key: &str) -> &'a [Value] {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
}
fn text<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or_default()
}
fn collection_description(value: &Value) -> Option<String> {
    value
        .get("description")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.chars().take(16_000).collect())
}

pub(super) fn collection(value: &Value, kind: &str) -> Option<Collection> {
    let id = value.get("id")?.as_u64().filter(|id| *id > 0)?;
    if kind == "playlist"
        && (text(value, "type") == "VIDEO"
            || value.get("playlistType").and_then(Value::as_str) == Some("VIDEO"))
    {
        return None;
    }
    let creator = &value["creator"];
    let subtitle = if kind == "playlist" {
        text(creator, "nickname").into()
    } else if kind == "album" {
        array(value, "artists")
            .iter()
            .map(|v| text(v, "name"))
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" / ")
    } else {
        format!(
            "{} 张专辑",
            value.get("albumSize").and_then(Value::as_u64).unwrap_or(0)
        )
    };
    Some(Collection {
        id,
        kind: kind.into(),
        name: text(value, "name").into(),
        cover: crate::cover::image(
            ["img1v1Url", "picUrl", "coverImgUrl"]
                .iter()
                .map(|key| text(value, key))
                .find(|s| !s.is_empty())
                .unwrap_or_default(),
        ),
        subtitle,
        track_count: value
            .get("trackCount")
            .or_else(|| value.get("size"))
            .and_then(Value::as_u64)
            .unwrap_or(0),
        published_at: value
            .get("publishTime")
            .and_then(Value::as_u64)
            .filter(|v| *v > 0),
        artists: array(value, "artists")
            .iter()
            .filter_map(|v| {
                Some(crate::model::MusicCredit {
                    name: v.get("name")?.as_str()?.into(),
                    id: v.get("id").and_then(Value::as_u64).filter(|id| *id > 0),
                })
            })
            .collect(),
        creator_id: creator.get("userId").and_then(Value::as_u64).unwrap_or(0),
        liked: value.get("specialType").and_then(Value::as_u64) == Some(5),
        play_count: value
            .get("playCount")
            .or_else(|| value.get("playcount"))
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite() && *v >= 0.0)
            .map(|v| v as u64),
    })
}
fn collection_matches(collection: &Collection, kind: &str, filter: &str, owner_id: u64) -> bool {
    if kind != "playlist" {
        return true;
    }
    !collection.liked
        && match filter {
            "mine" => collection.creator_id == owner_id,
            "liked" => collection.creator_id != owner_id,
            _ => true,
        }
}
fn ordered_tracks(ids: &[u64], songs: &[Value]) -> Vec<Track> {
    let mut tracks: HashMap<_, _> = songs
        .iter()
        .filter_map(track_from_json)
        .filter_map(|track| Some((track.netease_id()?, track)))
        .collect();
    ids.iter().filter_map(|id| tracks.remove(id)).collect()
}
fn playlist_ids(playlist: &Value, offset: usize, limit: usize) -> Vec<u64> {
    array(playlist, "trackIds")
        .iter()
        .skip(offset)
        .take(limit)
        .filter_map(|v| v.get("id")?.as_u64().filter(|id| *id > 0))
        .collect()
}
fn track_page<'a>(
    songs: impl Iterator<Item = &'a Value>,
    total: usize,
    offset: usize,
    limit: usize,
) -> TrackPage {
    TrackPage {
        description: None,
        tracks: songs
            .skip(offset)
            .take(limit)
            .filter_map(track_from_json)
            .collect(),
        total,
        more: offset.saturating_add(limit) < total,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn playlist_play_counts_accept_both_api_fields_and_omit_missing_values() {
        for (input, expected) in [
            (json!({"id":1,"playCount":12345.0}), Some(12345)),
            (json!({"id":1,"playcount":987}), Some(987)),
            (json!({"id":1,"playCount":0}), Some(0)),
            (json!({"id":1}), None),
            (json!({"id":1,"playCount":-1}), None),
        ] {
            assert_eq!(collection(&input, "playlist").unwrap().play_count, expected);
        }
    }
    #[test]
    fn playlist_description_preserves_lines_and_omits_missing_content() {
        assert_eq!(
            collection_description(&json!({"description":"  第一行\n第二行  "})).as_deref(),
            Some("第一行\n第二行")
        );
        for value in [
            json!({}),
            json!({"description":null}),
            json!({"description":"  "}),
        ] {
            assert!(collection_description(&value).is_none());
        }
    }

    #[test]
    fn liked_playlist_is_identified_by_type_not_position() {
        let regular = json!({"id":1,"name":"Created","creator":{"userId":123},"specialType":0});
        let liked = json!({"id":2,"name":"Liked","creator":{"userId":123},"specialType":5});
        assert!(!collection(&regular, "playlist").unwrap().liked);
        assert!(collection(&liked, "playlist").unwrap().liked);
    }
    #[test]
    fn playlist_pagination_handles_empty_and_out_of_range_without_sdk_slice_panic() {
        let playlist = json!({"trackIds":[{"id":1},{"id":2},{"id":3}]});
        assert_eq!(playlist_ids(&playlist, 1, 1), vec![2]);
        assert!(playlist_ids(&playlist, 100, 100).is_empty());
        assert!(playlist_ids(&json!({"trackIds":[]}), 0, 100).is_empty());
    }
    #[test]
    fn playlist_filters_use_owner_and_do_not_leak_into_other_collection_tabs() {
        let mine = collection(&json!({"id":1,"creator":{"userId":123}}), "playlist").unwrap();
        let other = collection(&json!({"id":2,"creator":{"userId":456}}), "playlist").unwrap();
        assert!(collection_matches(&mine, "playlist", "mine", 123));
        assert!(!collection_matches(&other, "playlist", "mine", 123));
        assert!(collection_matches(&other, "playlist", "liked", 123));
        assert!(!collection_matches(&mine, "playlist", "liked", 123));
        let album = collection(&json!({"id":3}), "album").unwrap();
        assert!(collection_matches(&album, "album", "liked", 123));
    }
    #[test]
    fn details_preserve_playlist_order_when_api_returns_shuffled_or_missing_songs() {
        let songs = [
            json!({"id":3,"name":"Third","ar":[{"name":"Artist"}],"al":{"name":"Album"}}),
            json!({"id":1,"name":"First","ar":[{"name":"Artist"}],"al":{"name":"Album"}}),
        ];
        let tracks = ordered_tracks(&[1, 2, 3], &songs);
        assert_eq!(
            tracks.iter().map(|t| t.key.as_str()).collect::<Vec<_>>(),
            vec!["netease:1", "netease:3"]
        );
    }
    #[test]
    fn album_and_artist_metadata_are_adapted_to_collection_cards() {
        let album = collection(&json!({"id":2,"name":"Album","picUrl":"https://example.com/a.jpg","artists":[{"name":"Singer"}]}), "album").unwrap();
        assert_eq!(album.subtitle, "Singer");
        assert_eq!(album.cover, "https://example.com/a.jpg");
        assert!(collection(&json!({"id":1,"type":"VIDEO"}), "playlist").is_none());
    }

    #[test]
    fn album_cards_keep_release_date_song_count_and_artist_ids() {
        let album = collection(
            &json!({"id":2,"size":11,"publishTime":1684771200000u64,
            "artists":[{"id":9,"name":"A"},{"id":0,"name":"B"}]}),
            "album",
        )
        .unwrap();
        assert_eq!(album.track_count, 11);
        assert_eq!(album.published_at, Some(1684771200000));
        assert_eq!(album.artists[0].id, Some(9));
        assert_eq!(album.artists[1].id, None);
        assert_eq!(album.play_count, None);
    }
}
