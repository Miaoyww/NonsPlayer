//! CUE describes virtual tracks; the referenced audio is never modified.
use crate::model::{AppResult, Track, TrackSource};
use cue_lib::{core::DataType, probe::CuesheetProbe};
use nons_music_adapter_sdk::queue::CueTrack;
use sha2::{Digest, Sha256};
use std::{io::Read, path::Path};

const MAX_CUE_BYTES: u64 = 2 * 1024 * 1024;

#[cfg(test)]
#[path = "cue_tests.rs"]
mod tests;

pub(super) struct Sheet {
    pub file: String,
    pub title: Option<String>,
    pub performer: Option<String>,
    pub tracks: Vec<Entry>,
}
pub(super) struct Entry {
    pub number: u32,
    pub title: Option<String>,
    pub performer: Option<String>,
    pub start_ms: u64,
}

pub(super) fn is_cue(path: &Path) -> bool {
    path.extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("cue"))
}

fn decode(bytes: &[u8]) -> AppResult<String> {
    let (encoding, skip) = encoding_rs::Encoding::for_bom(bytes).unwrap_or_else(|| {
        (
            if std::str::from_utf8(bytes).is_ok() {
                encoding_rs::UTF_8
            } else {
                encoding_rs::GB18030
            },
            0,
        )
    });
    encoding
        .decode_without_bom_handling_and_without_replacement(&bytes[skip..])
        .map(|text| text.into_owned())
        .ok_or_else(|| "CUE 编码无效，支持 UTF-8、带 BOM 的 UTF-16 和 GB18030".into())
}

fn timestamp_ms(timestamp: cue_lib::core::CueTimestamp) -> AppResult<u64> {
    // cue_lib 0.1.0 rounds each frame to 13 ms in as_duration/as_millis.
    // Its validated timestamp has private fields; Display preserves the exact frame.
    let stamp = timestamp.to_string();
    let parts = stamp
        .split(':')
        .map(|part| part.parse::<u128>().map_err(|e| e.to_string()))
        .collect::<AppResult<Vec<_>>>()?;
    let millis = (parts[0] * 60 + parts[1]) * 1000 + parts[2] * 1000 / 75;
    u64::try_from(millis).map_err(|_| "CUE 时间超出支持范围".into())
}

pub(super) fn parse(text: &str) -> AppResult<Sheet> {
    // Reject multiple files before handing the sheet to the single-file library.
    let files = text
        .lines()
        .filter(|line| {
            line.split_whitespace()
                .next()
                .is_some_and(|word| word.eq_ignore_ascii_case("FILE"))
        })
        .count();
    if files != 1 {
        return Err(if files > 1 {
            "暂不支持多 FILE CUE，普通音频文件仍可导入"
        } else {
            "CUE 缺少 FILE"
        }
        .into());
    }
    let probe = CuesheetProbe::new(text).map_err(|e| format!("CUE 解析失败：{e}"))?;
    let file = probe
        .file_info()
        .ok_or("CUE 缺少音频文件")?
        .name
        .to_string();
    let mut tracks = Vec::new();
    let mut reader = probe.tracks();
    while let Some(track) = reader
        .next_track()
        .map_err(|e| format!("CUE 曲目解析失败：{e}"))?
    {
        if track.track_data_type() != DataType::Audio {
            return Err("CUE 包含非音频轨道".into());
        }
        let number = u32::from(track.track_no().into_inner());
        let start_ms = timestamp_ms(track.start_index())?;
        if number == 0
            || tracks.iter().any(|entry: &Entry| entry.number == number)
            || tracks
                .last()
                .is_some_and(|entry| entry.start_ms >= start_ms)
        {
            return Err("CUE 轨号重复或起始时间未递增".into());
        }
        tracks.push(Entry {
            number,
            title: track.title().map(|v| v.to_string()),
            performer: track.performer().map(|v| v.to_string()),
            start_ms,
        });
    }
    if tracks.is_empty() {
        return Err("CUE 没有音频曲目".into());
    }
    Ok(Sheet {
        file,
        title: probe.album_title().map(|v| v.to_string()),
        performer: probe.performer().map(|v| v.to_string()),
        tracks,
    })
}

pub(super) fn read(path: &Path) -> AppResult<Sheet> {
    let mut bytes = Vec::new();
    super::source::open(path)
        .map_err(|e| e.to_string())?
        .take(MAX_CUE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_CUE_BYTES {
        return Err("CUE 超过 2MiB 上限".into());
    }
    parse(&decode(&bytes)?)
}

pub(super) fn expand(sheet: Sheet, path: &Path, base: &Track) -> AppResult<Vec<Track>> {
    let sheet_path = super::source::canonicalize(path)
        .map_err(|e| e.to_string())?
        .to_string_lossy()
        .trim_start_matches(r"\\?\")
        .to_owned();
    let mut tracks = Vec::with_capacity(sheet.tracks.len());
    for (index, entry) in sheet.tracks.iter().enumerate() {
        // INDEX 01 to the next INDEX 01 retains the recorded pregap, as foobar does.
        let end_ms = sheet
            .tracks
            .get(index + 1)
            .map_or(base.duration_ms, |next| next.start_ms);
        if entry.start_ms >= end_ms || end_ms > base.duration_ms {
            return Err("CUE 曲目时间超出音频长度".into());
        }
        let mut track = base.clone();
        track.key = format!(
            "local:{:x}",
            Sha256::digest(format!("cue:{sheet_path}:{}", entry.number).as_bytes())
        );
        track.title = entry
            .title
            .clone()
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| format!("音轨 {:02}", entry.number));
        track.artist = entry
            .performer
            .clone()
            .or_else(|| sheet.performer.clone())
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| base.artist.clone());
        track.album = sheet
            .title
            .clone()
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| base.album.clone());
        track.artists.clear();
        track.aliases.clear();
        track.duration_ms = end_ms - entry.start_ms;
        if let TrackSource::Local {
            cue, netease_id, ..
        } = &mut track.source
        {
            *netease_id = None;
            *cue = Some(CueTrack {
                sheet_path: sheet_path.clone(),
                track_number: entry.number,
                start_ms: entry.start_ms,
                end_ms,
            });
        }
        tracks.push(track);
    }
    Ok(tracks)
}
