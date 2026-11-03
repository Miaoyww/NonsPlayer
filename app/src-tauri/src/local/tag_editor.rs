use crate::{
    application::Backend,
    model::{AppResult, Track, TrackSource},
    player::Command,
    storage::Store,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use lofty::{
    config::WriteOptions,
    picture::{Picture, PictureType},
    prelude::*,
    probe::Probe,
    tag::{ItemKey, Tag},
};
use serde::{Deserialize, Serialize};
use std::{
    io::{Cursor, Seek},
    path::Path,
    str::FromStr,
};
use tauri::{Emitter, State};

const MAX_TEXT_BYTES: usize = 16 * 1024;
const MAX_COVER_BYTES: u64 = 64 * 1024 * 1024;
const COVER_PREVIEW_SIZE: u32 = 512;

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalTrackTags {
    pub title: String,
    pub artist: String,
    pub album_artist: String,
    pub album: String,
    pub year: String,
    pub track_number: Option<u32>,
    pub disc_number: Option<u32>,
    pub genre: String,
    pub comment: String,
}

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "camelCase")]
pub enum CoverChange {
    Keep,
    Remove,
    Replace { path: String },
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LocalTrackTagUpdate {
    pub title: String,
    pub artist: String,
    pub album_artist: String,
    pub album: String,
    pub year: String,
    pub track_number: Option<u32>,
    pub disc_number: Option<u32>,
    pub genre: String,
    pub comment: String,
    pub cover: CoverChange,
}

#[tauri::command]
pub async fn local_track_tags(
    key: String,
    backend: State<'_, Backend>,
) -> AppResult<LocalTrackTags> {
    let path = editable_path(&backend.store.track(&key)?)?;
    tauri::async_runtime::spawn_blocking(move || read_tags(&path))
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn local_track_update_tags(
    key: String,
    tags: LocalTrackTagUpdate,
    app: tauri::AppHandle,
    backend: State<'_, Backend>,
) -> AppResult<Track> {
    let store = backend.store.clone();
    let covers = backend.covers.clone();
    let fallback = backend.fallback_cover.clone();
    let updated = tauri::async_runtime::spawn_blocking(move || {
        write_tags(&store, &covers, &fallback, &key, tags)
    })
    .await
    .map_err(|error| error.to_string())??;
    if let Err(error) = backend.player.send(Command::RefreshTrack(updated.clone())) {
        crate::infrastructure::diagnostics::warn("local.tags.refresh_queue", error);
    }
    if let Err(error) = app.emit(
        "local-library-updated",
        serde_json::json!({"scanning": false}),
    ) {
        crate::infrastructure::diagnostics::warn("local.tags.changed.emit", error);
    }
    Ok(updated)
}

#[tauri::command]
pub async fn local_tag_cover_preview(path: String) -> AppResult<String> {
    tauri::async_runtime::spawn_blocking(move || {
        let bytes = cover_bytes(Path::new(&path))?;
        let preview = super::cover_art::thumbnail(&bytes, COVER_PREVIEW_SIZE)
            .ok_or("无法读取封面图片，请选择 JPEG、PNG、BMP 或 GIF")?;
        Ok(format!(
            "data:image/png;base64,{}",
            STANDARD.encode(preview)
        ))
    })
    .await
    .map_err(|error| error.to_string())?
}

fn editable_path(track: &Track) -> AppResult<String> {
    let TrackSource::Local { path, cue, .. } = &track.source else {
        return Err("不是本地歌曲".into());
    };
    if cue.is_some() {
        return Err("CUE 分轨暂不支持编辑标签".into());
    }
    if Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("ncm"))
    {
        return Err("NCM 音频暂不支持编辑标签".into());
    }
    if super::source::is_document(path) {
        return Err("Android 文档目录暂不支持编辑标签".into());
    }
    Ok(path.clone())
}

fn read_tags(path: &str) -> AppResult<LocalTrackTags> {
    let tagged = Probe::open(path)
        .map_err(|error| format!("无法打开音频文件：{error}"))?
        .read()
        .map_err(|error| format!("无法读取音频标签：{error}"))?;
    let tag = tagged.primary_tag().or_else(|| tagged.first_tag());
    Ok(LocalTrackTags {
        title: text(tag, ItemKey::TrackTitle),
        artist: text(tag, ItemKey::TrackArtist),
        album_artist: text(tag, ItemKey::AlbumArtist),
        album: text(tag, ItemKey::AlbumTitle),
        year: tag
            .and_then(|tag| {
                tag.get_string(ItemKey::RecordingDate)
                    .or_else(|| tag.get_string(ItemKey::Year))
            })
            .unwrap_or_default()
            .to_owned(),
        track_number: tag.and_then(|tag| tag.track()),
        disc_number: tag.and_then(|tag| tag.disk()),
        genre: text(tag, ItemKey::Genre),
        comment: text(tag, ItemKey::Comment),
    })
}

fn text(tag: Option<&Tag>, key: ItemKey) -> String {
    tag.and_then(|tag| tag.get_string(key))
        .unwrap_or_default()
        .to_owned()
}

fn write_tags(
    store: &Store,
    covers: &Path,
    fallback: &str,
    key: &str,
    update: LocalTrackTagUpdate,
) -> AppResult<Track> {
    let track = store.track(key)?;
    let path = editable_path(&track)?;
    let values = TagValues {
        title: &update.title,
        artist: &update.artist,
        album_artist: &update.album_artist,
        album: &update.album,
        year: &update.year,
        track_number: update.track_number,
        disc_number: update.disc_number,
        genre: &update.genre,
        comment: &update.comment,
    };
    validate_values(&values)?;
    let cover = match &update.cover {
        CoverChange::Keep => None,
        CoverChange::Remove => Some(None),
        CoverChange::Replace { path } => {
            let bytes = cover_bytes(Path::new(path))?;
            if !super::cover_art::valid_bytes(&bytes) {
                return Err("无法读取封面图片，请选择 JPEG、PNG、BMP 或 GIF".into());
            }
            let mut picture = Picture::from_reader(&mut Cursor::new(bytes))
                .map_err(|error| format!("封面图片格式不受支持：{error}"))?;
            picture.set_pic_type(PictureType::CoverFront);
            Some(Some(picture))
        }
    };

    let mut tagged = Probe::open(&path)
        .map_err(|error| format!("无法打开音频文件：{error}"))?
        .read()
        .map_err(|error| format!("无法读取音频标签：{error}"))?;
    let tag = ensure_tag(&mut tagged)?;
    apply_values(tag, &values)?;
    if let Some(picture) = cover {
        while !tag.pictures().is_empty() {
            tag.remove_picture(0);
        }
        if let Some(picture) = picture {
            tag.push_picture(picture);
        }
    }
    tag.save_to_path(&path, WriteOptions::default())
        .map_err(|error| format!("无法写入音频标签：{error}"))?;

    refresh_track(store, covers, fallback, key, &path)
        .map_err(|error| format!("标签已写入文件，但曲库刷新失败：{error}"))
}

pub(crate) fn write_download_tags(
    file: &mut std::fs::File,
    title: &str,
    artist: &str,
    album: &str,
    cover: Option<&[u8]>,
) -> AppResult<()> {
    let title = clip(title);
    let artist = clip(artist);
    let album = clip(album);
    let values = TagValues {
        title: &title,
        artist: &artist,
        album_artist: "",
        album: &album,
        year: "",
        track_number: None,
        disc_number: None,
        genre: "",
        comment: "",
    };
    validate_values(&values)?;
    let picture = cover
        .map(|bytes| {
            let mut picture = Picture::from_reader(&mut Cursor::new(bytes))
                .map_err(|error| format!("封面图片格式不受支持：{error}"))?;
            picture.set_pic_type(PictureType::CoverFront);
            Ok::<_, String>(picture)
        })
        .transpose()?;
    file.rewind().map_err(|error| error.to_string())?;
    let mut tagged = Probe::new(&mut *file)
        .guess_file_type()
        .map_err(|error| format!("无法识别音频格式：{error}"))?
        .read()
        .map_err(|error| format!("无法读取音频标签：{error}"))?;
    let tag = ensure_tag(&mut tagged)?;
    apply_values(tag, &values)?;
    if let Some(picture) = picture {
        while !tag.pictures().is_empty() {
            tag.remove_picture(0);
        }
        tag.push_picture(picture);
    }
    file.rewind().map_err(|error| error.to_string())?;
    tag.save_to(&mut *file, WriteOptions::default())
        .map_err(|error| format!("无法写入音频标签：{error}"))?;
    file.sync_all().map_err(|error| error.to_string())
}

struct TagValues<'a> {
    title: &'a str,
    artist: &'a str,
    album_artist: &'a str,
    album: &'a str,
    year: &'a str,
    track_number: Option<u32>,
    disc_number: Option<u32>,
    genre: &'a str,
    comment: &'a str,
}

fn ensure_tag(tagged: &mut lofty::file::TaggedFile) -> AppResult<&mut Tag> {
    if tagged.primary_tag().is_none() && tagged.first_tag().is_none() {
        let tag_type = tagged.primary_tag_type();
        let _ = tagged.insert_tag(Tag::new(tag_type));
    }
    if tagged.primary_tag().is_some() {
        return tagged
            .primary_tag_mut()
            .ok_or_else(|| "当前音频格式不支持写入标签".into());
    }
    tagged
        .first_tag_mut()
        .ok_or_else(|| "当前音频格式不支持写入标签".into())
}

fn apply_values(tag: &mut Tag, values: &TagValues<'_>) -> AppResult<()> {
    set_text(tag, ItemKey::TrackTitle, values.title);
    set_text(tag, ItemKey::TrackArtist, values.artist);
    set_text(tag, ItemKey::AlbumArtist, values.album_artist);
    set_text(tag, ItemKey::AlbumTitle, values.album);
    set_text(tag, ItemKey::Genre, values.genre);
    set_text(tag, ItemKey::Comment, values.comment);
    if values.year.is_empty() {
        tag.remove_date();
    } else {
        tag.set_date(parse_year(values.year)?);
    }
    if let Some(track_number) = values.track_number {
        tag.set_track(track_number);
    } else {
        tag.remove_track();
    }
    if let Some(disc_number) = values.disc_number {
        tag.set_disk(disc_number);
    } else {
        tag.remove_disk();
    }
    Ok(())
}

fn refresh_track(
    store: &Store,
    covers: &Path,
    fallback: &str,
    key: &str,
    path: &str,
) -> AppResult<Track> {
    store.save_file_stat(key, "", 0)?;
    let track = super::library::read_track(Path::new(path), covers, fallback, store)?;
    store.refresh_local_albums()?;
    Ok(track)
}

fn set_text(tag: &mut Tag, key: ItemKey, value: &str) {
    if value.is_empty() {
        tag.remove_key(key);
    } else {
        tag.insert_text(key, value.to_owned());
    }
}

fn parse_year(value: &str) -> AppResult<lofty::tag::items::Timestamp> {
    const MESSAGE: &str = "年份格式无效，请使用 YYYY、YYYY-MM 或 YYYY-MM-DD";
    let timestamp =
        lofty::tag::items::Timestamp::from_str(value).map_err(|_| MESSAGE.to_owned())?;
    if timestamp.to_string() != value
        || timestamp.year == 0
        || timestamp.year > 9999
        || timestamp
            .month
            .is_some_and(|month| !(1..=12).contains(&month))
        || timestamp.day.is_some_and(|day| !(1..=31).contains(&day))
        || timestamp.hour.is_some_and(|hour| hour > 23)
        || timestamp.minute.is_some_and(|minute| minute > 59)
        || timestamp.second.is_some_and(|second| second > 59)
    {
        return Err(MESSAGE.into());
    }
    Ok(timestamp)
}

fn validate_values(values: &TagValues<'_>) -> AppResult<()> {
    for (name, value) in [
        ("标题", values.title),
        ("艺术家", values.artist),
        ("专辑艺术家", values.album_artist),
        ("专辑", values.album),
        ("年份", values.year),
        ("流派", values.genre),
        ("备注", values.comment),
    ] {
        if value.len() > MAX_TEXT_BYTES {
            return Err(format!("{name}过长，最多 {MAX_TEXT_BYTES} 字节"));
        }
        if value.contains('\0') {
            return Err(format!("{name}包含无效字符"));
        }
    }
    if values.track_number == Some(0) || values.disc_number == Some(0) {
        return Err("曲目号与碟号必须大于 0".into());
    }
    Ok(())
}

fn clip(value: &str) -> String {
    if value.len() <= MAX_TEXT_BYTES {
        return value.to_owned();
    }
    let mut end = MAX_TEXT_BYTES;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].to_owned()
}

fn cover_bytes(path: &Path) -> AppResult<Vec<u8>> {
    let metadata = std::fs::metadata(path).map_err(|error| format!("无法读取封面文件：{error}"))?;
    if !metadata.is_file() {
        return Err("请选择图片文件作为封面".into());
    }
    if metadata.len() > MAX_COVER_BYTES {
        return Err("封面图片不能超过 64 MiB".into());
    }
    std::fs::read(path).map_err(|error| format!("无法读取封面文件：{error}"))
}

#[cfg(test)]
#[path = "tag_editor_tests.rs"]
mod tests;
