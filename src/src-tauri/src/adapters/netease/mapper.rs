use crate::models::{album::Album, artist::Artist, playlist::Playlist, song::Song};

use super::models::{
    CloudSearchAlbum, CloudSearchArtist, CloudSearchPlaylist, PlaylistDetailResponse,
    RecommendPlaylistItem, SongItem,
};

pub fn val_to_string(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        _ => String::new(),
    }
}

fn artist_item_to_artist(a: &super::models::ArtistItem) -> Artist {
    Artist {
        id: format!("netease_artist_{}", val_to_string(&a.id)),
        name: a.name.clone(),
        avatar_url: a.pic_url.as_deref().unwrap_or("").to_string(),
        adapter_slug: "netease".into(),
        ..Artist::empty()
    }
}

pub fn map_song(raw: &SongItem) -> Song {
    let id = val_to_string(&raw.id);
    let name = raw.name.clone();
    let dt_ms = raw.dt;
    let duration = dt_ms / 1000.0;
    let fee = raw.fee;
    let available = fee == 0 || fee == 8;

    let minutes = (duration / 60.0) as u64;
    let seconds = (duration % 60.0) as u64;
    let duration_text = format!("{}:{:02}", minutes, seconds);

    let artists: Vec<Artist> = if !raw.ar.is_empty() {
        raw.ar.iter().map(artist_item_to_artist).collect()
    } else if !raw.artists.is_empty() {
        raw.artists.iter().map(artist_item_to_artist).collect()
    } else {
        vec![]
    };

    let al = raw.al.as_ref().or(raw.album.as_ref());
    let album_id = al.map(|v| val_to_string(&v.id)).unwrap_or_default();
    let album_name = al
        .and_then(|v| if v.name.is_empty() { None } else { Some(v.name.as_str()) })
        .unwrap_or("")
        .to_string();
    let album_pic = al.and_then(|v| v.pic_url.as_deref()).unwrap_or("");

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
    let avatar_url = album_pic.to_string();

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
        trans: raw.alia.first().cloned(),
        ..Song::empty()
    }
}

pub fn map_search_playlist(raw: &CloudSearchPlaylist) -> Playlist {
    let id = val_to_string(&raw.id);
    let name = raw.name.clone();
    let cover = raw.cover_img_url.as_deref().unwrap_or("");
    let track_count = raw.track_count.unwrap_or(0.0) as u32;
    let play_count = raw.play_count.unwrap_or(0.0) as u32;
    let creator = raw
        .creator
        .as_ref()
        .map(|c| c.nickname.clone())
        .unwrap_or_default();

    log::debug!(
        "[netease] map_search_playlist id={} name={} trackCount={} playCount={} creator={}",
        id, name, track_count, play_count, creator
    );

    Playlist {
        id: format!("netease_playlist_{}", id),
        name: name.clone(),
        avatar_url: format!("{}?param=300y300", cover),
        small_avatar_url: format!("{}?param=50y50", cover),
        middle_avatar_url: format!("{}?param=200y200", cover),
        title: name,
        creator,
        description: raw.description.clone().unwrap_or_default(),
        tags: raw.tags.clone(),
        musics_count: track_count,
        play_count,
        adapter_slug: "netease".into(),
        ..Playlist::empty()
    }
}

pub fn map_recommend_playlist(raw: &RecommendPlaylistItem) -> Playlist {
    let id = val_to_string(&raw.id);
    let name = raw.name.clone();
    let cover = raw.pic_url.as_deref().unwrap_or("");
    let track_count = raw.track_count.unwrap_or(0.0) as u32;
    let play_count = raw.playcount.unwrap_or(0.0) as u32;
    let creator = raw
        .creator
        .as_ref()
        .map(|c| c.nickname.clone())
        .unwrap_or_default();

    log::debug!(
        "[netease] map_recommend_playlist id={} name={} trackCount={} playCount={} creator={}",
        id, name, track_count, play_count, creator
    );

    Playlist {
        id: format!("netease_playlist_{}", id),
        name: name.clone(),
        avatar_url: format!("{}?param=300y300", cover),
        small_avatar_url: format!("{}?param=50y50", cover),
        middle_avatar_url: format!("{}?param=200y200", cover),
        title: name,
        creator,
        description: raw.copywriter.clone().unwrap_or_default(),
        musics_count: track_count,
        play_count,
        adapter_slug: "netease".into(),
        ..Playlist::empty()
    }
}

pub fn map_playlist_full(raw: &PlaylistDetailResponse) -> Playlist {
    let playlist = &raw.playlist;
    let id = val_to_string(&playlist.id);
    let name = playlist.name.clone();
    let cover = playlist.cover_img_url.as_deref().unwrap_or("");
    let track_count = playlist.track_count.unwrap_or(0.0) as u32;
    let play_count = playlist.play_count.unwrap_or(0.0) as u32;
    let creator = playlist
        .creator
        .as_ref()
        .map(|c| c.nickname.clone())
        .unwrap_or_default();
    let create_time = playlist
        .create_time
        .map(|ts| (ts as i64).to_string())
        .unwrap_or_default();

    let musics: Vec<Song> = playlist.tracks.iter().map(map_song).collect();

    let track_ids: Vec<String> = playlist
        .track_ids
        .iter()
        .map(|t| format!("netease_song_{}", val_to_string(&t.id)))
        .collect();

    Playlist {
        id: format!("netease_playlist_{}", id),
        name: name.clone(),
        avatar_url: format!("{}?param=300y300", cover),
        small_avatar_url: format!("{}?param=50y50", cover),
        middle_avatar_url: format!("{}?param=200y200", cover),
        title: name,
        creator,
        create_time,
        description: playlist.description.clone().unwrap_or_default(),
        tags: playlist.tags.clone(),
        music_track_ids: track_ids,
        musics,
        is_initialized: true,
        musics_count: track_count,
        play_count,
        adapter_slug: "netease".into(),
        ..Playlist::empty()
    }
}

pub fn map_cloudsearch_album(raw: &CloudSearchAlbum) -> Album {
    let id = val_to_string(&raw.id);
    let cover = raw.pic_url.as_deref().unwrap_or("");
    let artist = raw
        .artist
        .as_ref()
        .map(|a| Artist {
            id: format!("netease_artist_{}", val_to_string(&a.id)),
            name: a.name.clone(),
            ..Artist::empty()
        })
        .unwrap_or_else(Artist::empty);
    Album {
        id: format!("netease_album_{}", id),
        name: raw.name.clone(),
        avatar_url: format!("{}?param=300y300", cover),
        small_avatar_url: format!("{}?param=50y50", cover),
        middle_avatar_url: format!("{}?param=200y200", cover),
        artists: vec![artist.clone()],
        artists_name: artist.name,
        adapter_slug: "netease".into(),
        ..Album::empty()
    }
}

pub fn map_cloudsearch_artist(raw: &CloudSearchArtist) -> Artist {
    Artist {
        id: format!("netease_artist_{}", val_to_string(&raw.id)),
        name: raw.name.clone(),
        avatar_url: raw.pic_url.as_deref().unwrap_or("").to_string(),
        adapter_slug: "netease".into(),
        ..Artist::empty()
    }
}
