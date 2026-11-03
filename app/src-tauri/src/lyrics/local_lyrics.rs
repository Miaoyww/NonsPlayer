//! On-demand local lyrics. Never populate the online runtime or TTML caches.
use crate::model::{AppResult, Lyrics, Track, TrackSource, MAX_LYRIC_BYTES};
use lofty::{config::ParseOptions, prelude::*, probe::Probe};
use std::{
    ffi::OsString,
    io::Read,
    path::{Path, PathBuf},
};

static READS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(2);

pub async fn get(track: Track, skip_embedded: bool) -> AppResult<Option<Lyrics>> {
    let TrackSource::Local { path, .. } = track.source else {
        return Ok(None);
    };
    let permit = READS.acquire().await.map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        read(Path::new(&path), skip_embedded)
    })
    .await
    .map_err(|e| e.to_string())?
}

pub async fn read_ttml(path: String) -> AppResult<Lyrics> {
    let permit = READS.acquire().await.map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        read_ttml_file(Path::new(&path))
    })
    .await
    .map_err(|e| e.to_string())?
}

fn read_ttml_file(path: &Path) -> AppResult<Lyrics> {
    if !path
        .extension()
        .is_some_and(|extension| extension.to_string_lossy().eq_ignore_ascii_case("ttml"))
    {
        return Err("只能选择 TTML 歌词文件".into());
    }
    let file = crate::local::source::open(path).map_err(|e| format!("本地歌词无法读取：{e}"))?;
    let mut bytes = Vec::new();
    file.take(MAX_LYRIC_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("本地歌词无法读取：{e}"))?;
    if bytes.len() > MAX_LYRIC_BYTES {
        return Err("本地歌词超过 2MiB 上限".into());
    }
    let content = decode(bytes)?;
    if content.trim().is_empty() {
        return Err("本地歌词内容为空".into());
    }
    if !super::valid_ttml(&content) {
        return Err("所选文件不是有效的 TTML 歌词".into());
    }
    Ok(Lyrics {
        match_score: None,
        source: "local".into(),
        format: "ttml".into(),
        content,
        translation: None,
        romanization: None,
    })
}

fn embedded(path: &Path) -> Option<String> {
    let tagged = if crate::encoded_audio::is_encoded(path) {
        crate::encoded_audio::read_tags(&path.to_string_lossy())
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn(
                    "lyrics.local.embedded.read_ncm_tags",
                    error,
                );
            })
            .ok()?
            .0
    } else {
        Probe::new(crate::local::source::open(path).ok()?)
            .guess_file_type()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn_error(
                    "lyrics.local.embedded.open_audio",
                    error,
                );
            })
            .ok()?
            .options(
                ParseOptions::new()
                    .read_properties(false)
                    .read_cover_art(false),
            )
            .read()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn_error(
                    "lyrics.local.embedded.read_tags",
                    error,
                );
            })
            .ok()?
    };
    let tag = tagged.primary_tag().or_else(|| tagged.first_tag())?;
    // ID3v2 USLT maps to UnsyncLyrics in Lofty 0.25, even when its text contains LRC timestamps.
    [ItemKey::Lyrics, ItemKey::UnsyncLyrics]
        .into_iter()
        .filter_map(|key| tag.get_string(key))
        .find(|text| !text.trim().is_empty() && text.len() <= MAX_LYRIC_BYTES)
        .map(str::to_owned)
}

fn decode(bytes: Vec<u8>) -> AppResult<String> {
    let text = if bytes.starts_with(&[0xff, 0xfe]) || bytes.starts_with(&[0xfe, 0xff]) {
        if bytes.len() % 2 != 0 {
            return Err("本地歌词 UTF-16 内容不完整".into());
        }
        let little = bytes[0] == 0xff;
        let units: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|pair| {
                if little {
                    u16::from_le_bytes([pair[0], pair[1]])
                } else {
                    u16::from_be_bytes([pair[0], pair[1]])
                }
            })
            .collect();
        String::from_utf16(&units)
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn_error(
                    "lyrics.local.sidecar.decode_utf16",
                    error,
                );
            })
            .map_err(|_| "本地歌词 UTF-16 编码无效")?
    } else {
        String::from_utf8(bytes)
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn_error(
                    "lyrics.local.sidecar.decode_utf8",
                    error,
                );
            })
            .map_err(|_| "本地歌词不是有效的 UTF-8 或带 BOM 的 UTF-16")?
    };
    if text.len() > MAX_LYRIC_BYTES {
        return Err("本地歌词超过 2MiB 上限".into());
    }
    Ok(text.trim_start_matches('\u{feff}').to_owned())
}

fn companion(audio: &Path, suffix: &str) -> Option<PathBuf> {
    let mut name = OsString::from(audio.file_stem()?);
    name.push(suffix);
    Some(audio.with_file_name(name))
}

fn read_companion(audio: &Path, suffixes: &[&str], operation: &'static str) -> Option<String> {
    suffixes.iter().find_map(|suffix| {
        let path = companion(audio, suffix)?;
        let file = match crate::local::source::open(&path) {
            Ok(file) => file,
            Err(error) => {
                if error.kind() != std::io::ErrorKind::NotFound {
                    crate::infrastructure::diagnostics::warn(&format!("{operation}.open"), error);
                }
                return None;
            }
        };
        let mut bytes = Vec::new();
        if let Err(error) = file
            .take(MAX_LYRIC_BYTES as u64 + 1)
            .read_to_end(&mut bytes)
        {
            crate::infrastructure::diagnostics::warn(&format!("{operation}.read"), error);
            return None;
        }
        if bytes.len() > MAX_LYRIC_BYTES {
            crate::infrastructure::diagnostics::warn(
                &format!("{operation}.too_large"),
                path.display(),
            );
            return None;
        }
        let text = match decode(bytes) {
            Ok(text) => text,
            Err(error) => {
                crate::infrastructure::diagnostics::warn(&format!("{operation}.decode"), error);
                return None;
            }
        };
        (!text.trim().is_empty()).then_some(text)
    })
}

fn read(path: &Path, skip_embedded: bool) -> AppResult<Option<Lyrics>> {
    let content = if !skip_embedded { embedded(path) } else { None };
    let content = match content {
        Some(text) => text,
        None => {
            let file = match crate::local::source::open(path.with_extension("lrc")) {
                Ok(file) => file,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
                Err(error) => return Err(format!("本地歌词无法读取：{error}")),
            };
            let mut bytes = Vec::new();
            file.take(MAX_LYRIC_BYTES as u64 + 1)
                .read_to_end(&mut bytes)
                .map_err(|e| format!("本地歌词无法读取：{e}"))?;
            if bytes.len() > MAX_LYRIC_BYTES {
                return Err("本地歌词超过 2MiB 上限".into());
            }
            decode(bytes)?
        }
    };
    let translation = read_companion(
        path,
        &["_trans.lrc", ".trans.lrc"],
        "lyrics.local.translation",
    );
    let romanization = read_companion(
        path,
        &["_roma.lrc", ".roma.lrc"],
        "lyrics.local.romanization",
    );
    Ok((!content.trim().is_empty()).then_some(Lyrics {
        source: "local".into(),
        format: "lrc".into(),
        content,
        translation,
        romanization,
        match_score: None,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TestDir;
    const TEXT: &str = "[00:01.00]本地歌词😀\n[00:02.00]第二行";

    #[tokio::test]
    async fn stream_tracks_never_enter_local_file_reader() {
        let track = Track::from_music(nons_music_adapter_sdk::identity::MusicTrack {
            title: "song".into(),
            aliases: Vec::new(),
            artist: String::new(),
            artists: Vec::new(),
            album_reference: None,
            album: String::new(),
            duration_ms: 0,
            cover: Default::default(),
            reference: nons_music_adapter_sdk::identity::EntityRef {
                source: "netease".to_string().try_into().unwrap(),
                kind: nons_music_adapter_sdk::identity::EntityKind::Track,
                id: (1).to_string().try_into().unwrap(),
            },
            associations: vec![],
            lyric_identifiers: vec![],
            unavailable_reason: None,
        })
        .unwrap();
        assert!(get(track, false).await.unwrap().is_none());
    }

    #[test]
    fn sidecar_encodings_and_invalid_input() {
        assert_eq!(decode(TEXT.as_bytes().to_vec()).unwrap(), TEXT);
        assert_eq!(
            decode(format!("\u{feff}{TEXT}").into_bytes()).unwrap(),
            TEXT
        );
        for little in [false, true] {
            let mut bytes = if little {
                vec![0xff, 0xfe]
            } else {
                vec![0xfe, 0xff]
            };
            for unit in TEXT.encode_utf16() {
                bytes.extend(if little {
                    unit.to_le_bytes()
                } else {
                    unit.to_be_bytes()
                });
            }
            assert_eq!(decode(bytes).unwrap(), TEXT);
        }
        assert!(decode(vec![0xff]).is_err());
        assert!(decode(vec![0xff, 0xfe, 0]).is_err());
        assert!(decode(vec![0xff, 0xfe, 0, 0xd8]).is_err());
    }

    #[test]
    fn same_name_only_and_changes_are_read_without_cache() {
        let dir = TestDir::new();
        let audio = dir.0.join("song.flac");
        std::fs::write(dir.0.join("other.lrc"), TEXT).unwrap();
        assert!(read(&audio, false).unwrap().is_none());
        let sidecar = audio.with_extension("lrc");
        std::fs::write(&sidecar, TEXT).unwrap();
        let lyrics = read(&audio, false).unwrap().unwrap();
        assert_eq!(lyrics.source, "local");
        assert!(lyrics.translation.is_none());
        assert_eq!(lyrics.content, TEXT);
        std::fs::write(&sidecar, "[00:03.00]已修改").unwrap();
        assert_eq!(
            read(&audio, false).unwrap().unwrap().content,
            "[00:03.00]已修改"
        );
        std::fs::remove_file(&sidecar).unwrap();
        assert!(read(&audio, false).unwrap().is_none());
    }

    #[test]
    fn companion_translation_and_romanization_are_optional() {
        let dir = TestDir::new();
        let audio = dir.0.join("song.flac");
        std::fs::write(audio.with_extension("lrc"), TEXT).unwrap();
        assert!(read(&audio, false).unwrap().unwrap().translation.is_none());

        std::fs::write(dir.0.join("song_trans.lrc"), "[00:01.00]译文").unwrap();
        std::fs::write(dir.0.join("song_roma.lrc"), "[00:01.00]roman").unwrap();
        let lyrics = read(&audio, false).unwrap().unwrap();
        assert_eq!(lyrics.translation.as_deref(), Some("[00:01.00]译文"));
        assert_eq!(lyrics.romanization.as_deref(), Some("[00:01.00]roman"));
        assert_eq!(lyrics.content, TEXT);
    }

    #[test]
    fn dotted_companion_names_are_supported_after_primary_names() {
        let dir = TestDir::new();
        let audio = dir.0.join("song.mp3");
        std::fs::write(audio.with_extension("lrc"), TEXT).unwrap();
        std::fs::write(dir.0.join("song.trans.lrc"), "[00:01.00]点号译文").unwrap();
        std::fs::write(dir.0.join("song.roma.lrc"), "[00:01.00]dot roman").unwrap();
        let lyrics = read(&audio, false).unwrap().unwrap();
        assert_eq!(lyrics.translation.as_deref(), Some("[00:01.00]点号译文"));
        assert_eq!(lyrics.romanization.as_deref(), Some("[00:01.00]dot roman"));

        std::fs::write(dir.0.join("song_trans.lrc"), "[00:01.00]主译文").unwrap();
        assert_eq!(
            read(&audio, false).unwrap().unwrap().translation.as_deref(),
            Some("[00:01.00]主译文")
        );
    }

    #[test]
    fn companion_files_ignore_bad_input_and_fall_back_to_dotted_names() {
        let dir = TestDir::new();
        let audio = dir.0.join("song.flac");
        std::fs::write(&audio, b"not audio").unwrap();
        std::fs::write(audio.with_extension("lrc"), TEXT).unwrap();
        std::fs::write(dir.0.join("song_trans.lrc"), [0xff]).unwrap();
        std::fs::write(dir.0.join("song.trans.lrc"), "[00:01.00]备用译文").unwrap();
        std::fs::write(dir.0.join("song.roma.lrc"), vec![b'a'; MAX_LYRIC_BYTES + 1]).unwrap();
        let lyrics = read(&audio, true).unwrap().unwrap();
        assert_eq!(lyrics.translation.as_deref(), Some("[00:01.00]备用译文"));
        assert!(lyrics.romanization.is_none());
    }

    #[test]
    fn oversized_empty_and_bad_sidecars() {
        let dir = TestDir::new();
        let audio = dir.0.join("song.mp3");
        let sidecar = audio.with_extension("lrc");
        std::fs::write(&sidecar, vec![b'a'; MAX_LYRIC_BYTES + 1]).unwrap();
        assert!(read(&audio, true).unwrap_err().contains("2MiB"));
        std::fs::write(&sidecar, "  \n").unwrap();
        assert!(read(&audio, true).unwrap().is_none());
        std::fs::write(&sidecar, [0xff]).unwrap();
        assert!(read(&audio, true).is_err());
    }

    #[test]
    fn selected_ttml_is_bounded_and_validated() {
        let dir = TestDir::new();
        let path = dir.0.join("song.ttml");
        std::fs::write(&path, "<tt><body><p>歌词</p></body></tt>").unwrap();
        let lyrics = read_ttml_file(&path).unwrap();
        assert_eq!(lyrics.source, "local");
        assert_eq!(lyrics.format, "ttml");
        assert_eq!(lyrics.content, "<tt><body><p>歌词</p></body></tt>");

        std::fs::write(&path, "<html><body>Unavailable</body></html>").unwrap();
        assert!(read_ttml_file(&path).unwrap_err().contains("TTML"));

        std::fs::write(&path, "  ").unwrap();
        assert!(read_ttml_file(&path).unwrap_err().contains("为空"));

        std::fs::write(&path, vec![b'a'; MAX_LYRIC_BYTES + 1]).unwrap();
        assert!(read_ttml_file(&path).unwrap_err().contains("2MiB"));

        let non_ttml = dir.0.join("song.lrc");
        std::fs::write(&non_ttml, "<tt><body/></tt>").unwrap();
        assert!(read_ttml_file(&non_ttml)
            .unwrap_err()
            .contains("只能选择 TTML"));
    }

    #[test]
    fn embedded_lyrics_precede_sidecar_and_can_be_skipped() {
        use lofty::{
            config::WriteOptions,
            id3::v2::{Frame, Id3v2Tag, UnsynchronizedTextFrame},
            TextEncoding,
        };
        let dir = TestDir::new();
        let audio = dir.0.join("song.mp3");
        let mut tag = Id3v2Tag::default();
        tag.insert(Frame::UnsynchronizedText(UnsynchronizedTextFrame::new(
            TextEncoding::UTF8,
            *b"eng",
            "",
            TEXT,
        )));
        let mut bytes = Vec::new();
        tag.dump_to(&mut bytes, WriteOptions::default()).unwrap();
        // Two matching MPEG frames let the reader locate the audio boundary.
        for _ in 0..2 {
            bytes.extend([0xff, 0xfb, 0x90, 0x00]);
            bytes.resize(bytes.len() + 413, 0);
        }
        std::fs::write(&audio, bytes).unwrap();
        let tagged = Probe::open(&audio)
            .unwrap()
            .options(
                ParseOptions::new()
                    .read_properties(false)
                    .read_cover_art(false),
            )
            .read()
            .unwrap();
        assert_eq!(
            tagged
                .primary_tag()
                .unwrap()
                .get_string(ItemKey::UnsyncLyrics),
            Some(TEXT)
        );
        std::fs::write(audio.with_extension("lrc"), "[00:01.00]外挂").unwrap();
        std::fs::write(dir.0.join("song_trans.lrc"), "[00:01.00]译文").unwrap();
        let embedded = read(&audio, false).unwrap().unwrap();
        assert_eq!(embedded.content, TEXT);
        assert_eq!(embedded.translation.as_deref(), Some("[00:01.00]译文"));
        let sidecar = read(&audio, true).unwrap().unwrap();
        assert_eq!(sidecar.content, "[00:01.00]外挂");
        assert_eq!(sidecar.translation.as_deref(), Some("[00:01.00]译文"));
    }
}
