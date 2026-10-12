//! Anonymous QQ lyric fallback for the already matched candidate.
use super::{decode_qrc_field, read_json, Candidate};
use crate::model::{AppResult, Lyrics, MAX_LYRIC_BYTES};
use base64::{engine::general_purpose::STANDARD, Engine};
use regex::Regex;
use serde_json::{json, Value};
use std::{sync::LazyLock, time::Duration};

pub(super) async fn fetch(
    client: &reqwest::Client,
    candidate: &Candidate,
    skip_qrc: bool,
    endpoint: &str,
) -> AppResult<Option<Lyrics>> {
    let request = client.post(endpoint)
        .header("Referer", "https://c.y.qq.com/")
        .header("User-Agent", "okhttp/3.14.9")
        .json(&json!({
            "comm": {"ct":11,"cv":"1003006","v":"1003006","uid":"0","tmeAppID":"qqmusiclight"},
            "req_1": {
                "method":"GetPlayLyricInfo", "module":"music.musichallSong.PlayLyricInfo",
                "param": {
                    "songID":candidate.id, "songName":STANDARD.encode(&candidate.title),
                    "singerName":STANDARD.encode(candidate.artists.join(", ")),
                    "albumName":STANDARD.encode(&candidate.album), "interval":candidate.duration.floor() as u64,
                    "crypt":1,"ct":19,"cv":2111,"type":0,
                    "qrc":if skip_qrc {0} else {1},"qrc_t":0,"lrc_t":0,
                    "trans":1,"trans_t":0,"roma":1,"roma_t":0
                }
            }
        }));
    tokio::time::timeout(Duration::from_millis(1500), async {
        let value = read_json(request).await?;
        tauri::async_runtime::spawn_blocking(move || decode(value))
            .await
            .map_err(|e| e.to_string())?
    })
    .await
    .map_err(|_| "QQ 补充歌词查询超时".to_string())?
}

fn decode(value: Value) -> AppResult<Option<Lyrics>> {
    if value["code"].as_i64() != Some(0) || value["req_1"]["code"].as_i64() != Some(0) {
        return Err("QQ 补充歌词返回错误".into());
    }
    let data = &value["req_1"]["data"];
    let Some(original) = data["lyric"].as_str().filter(|s| !s.trim().is_empty()) else {
        return Ok(None);
    };
    let content = decode_qrc_field(original)?;
    static WORDS: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?m)^\[\d+,\d+\].+\(\d+,\d+\)").unwrap());
    static TIMED: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\[\d+:\d{2}(?:[.:]\d+)?\][^\r\n]*\S").unwrap());
    let format = if WORDS.is_match(&content) {
        "qrc"
    } else if TIMED.is_match(&content) {
        "lrc"
    } else {
        return Err("QQ 补充歌词缺少有效时间轴".into());
    };
    let auxiliary = |field: &str| {
        data[field]
            .as_str()
            .filter(|s| !s.trim().is_empty())
            .and_then(|s| decode_qrc_field(s).ok())
            .filter(|s| !s.trim().is_empty())
    };
    let translation = auxiliary("trans");
    let romanization = auxiliary("roma");
    if content.len()
        + translation.as_ref().map_or(0, String::len)
        + romanization.as_ref().map_or(0, String::len)
        > MAX_LYRIC_BYTES
    {
        return Err("QQ 补充歌词正文与附加歌词过大".into());
    }
    Ok(Some(Lyrics {
        match_score: None,
        source: "qq".into(),
        format: format.into(),
        content,
        translation,
        romanization,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn encrypted_tracks_and_damaged_auxiliary_do_not_discard_original() {
        let fixture: Value =
            serde_json::from_str(include_str!("../../../../tests/fixtures/qrc.json")).unwrap();
        let response = json!({"code":0,"req_1":{"code":0,"data":{
            "lyric":fixture["encrypted"],"trans":fixture["translation"],"roma":fixture["romanization"]
        }}});
        let result = decode(response.clone()).unwrap().unwrap();
        assert_eq!(result.format, "qrc");
        assert_eq!(result.content, fixture["main"].as_str().unwrap());
        assert!(result.translation.unwrap().contains("你好世界"));
        let mut damaged = response;
        damaged["req_1"]["data"]["trans"] = json!("damaged");
        assert!(decode(damaged).unwrap().unwrap().translation.is_none());
    }
    #[test]
    fn lrc_empty_errors_and_budget_are_distinct() {
        let response = |data| json!({"code":0,"req_1":{"code":0,"data":data}});
        let lyrics = decode(response(
            json!({"lyric":"[00:01.00]Original","trans":"[00:01.00]译文"}),
        ))
        .unwrap()
        .unwrap();
        assert_eq!(lyrics.format, "lrc");
        assert_eq!(lyrics.translation.as_deref(), Some("[00:01.00]译文"));
        assert!(decode(response(json!({"lyric":""}))).unwrap().is_none());
        assert!(decode(json!({"code":1})).is_err());
        assert!(decode(response(json!({"lyric":"<html>error</html>"}))).is_err());
        assert!(decode(response(
            json!({"lyric":format!("[00:01.00]{}","a".repeat(MAX_LYRIC_BYTES))})
        ))
        .is_err());
    }
}
