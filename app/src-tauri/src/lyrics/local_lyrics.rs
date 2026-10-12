//! On-demand local lyrics. Never populate the online runtime or TTML caches.
use crate::model::{AppResult, Lyrics, Track, TrackSource, MAX_LYRIC_BYTES};
use lofty::{config::ParseOptions, prelude::*, probe::Probe};
use std::{fs::File, io::Read, path::Path};

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

fn embedded(path: &Path) -> Option<String> {
    let tagged = if crate::encoded_audio::is_encoded(path) {
        crate::encoded_audio::read_tags(&path.to_string_lossy())
            .ok()?
            .0
    } else {
        Probe::open(path)
            .ok()?
            .options(
                ParseOptions::new()
                    .read_properties(false)
                    .read_cover_art(false),
            )
            .read()
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
        String::from_utf16(&units).map_err(|_| "本地歌词 UTF-16 编码无效")?
    } else {
        String::from_utf8(bytes).map_err(|_| "本地歌词不是有效的 UTF-8 或带 BOM 的 UTF-16")?
    };
    if text.len() > MAX_LYRIC_BYTES {
        return Err("本地歌词超过 2MiB 上限".into());
    }
    Ok(text.trim_start_matches('\u{feff}').to_owned())
}

fn read(path: &Path, skip_embedded: bool) -> AppResult<Option<Lyrics>> {
    let content = if !skip_embedded { embedded(path) } else { None };
    let content = match content {
        Some(text) => text,
        None => {
            let file = match File::open(path.with_extension("lrc")) {
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
    Ok((!content.trim().is_empty()).then_some(Lyrics {
        source: "local".into(),
        format: "lrc".into(),
        content,
        translation: None,
        romanization: None,
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
        let track = Track {
            key: "netease:1".into(),
            title: "song".into(),
            aliases: Vec::new(),
            artist: String::new(),
            artists: Vec::new(),
            album_id: None,
            album: String::new(),
            duration_ms: 0,
            cover: String::new(),
            source: TrackSource::Netease { id: 1 },
        };
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
        assert_eq!(read(&audio, false).unwrap().unwrap().content, TEXT);
        assert_eq!(
            read(&audio, true).unwrap().unwrap().content,
            "[00:01.00]外挂"
        );
    }
}
