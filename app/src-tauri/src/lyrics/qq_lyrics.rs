//! QQ search protocol adapted from Lyricify-Lyrics-Helper (Apache-2.0).
//! Metadata policy ported from AF-Media-Bar (MIT). See notices/.
use crate::lyric_matching::queries;
#[cfg(test)]
use crate::lyric_matching::{compare, similarity, without_translation};
use crate::{
    model::{AppResult, Lyrics, Track},
    storage::MAX_LYRIC_BYTES,
};
use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use quick_xml::{events::Event, Reader};
use regex::Regex;
use serde_json::{json, Value};
use std::{
    sync::LazyLock,
    time::{Duration, Instant},
};

#[cfg(test)]
#[path = "qq_lyrics/fallback_tests.rs"]
mod fallback_tests;
#[path = "qq_lyrics/play_lyric.rs"]
mod play_lyric;
#[path = "qq_lyrics/qrc_xml.rs"]
mod qrc_xml;

struct LyricEndpoints<'a> {
    qrc: &'a str,
    musicu: &'a str,
    lrc: &'a str,
}

const LYRIC_ENDPOINTS: LyricEndpoints<'static> = LyricEndpoints {
    qrc: "https://c.y.qq.com/qqmusic/fcgi-bin/lyric_download.fcg",
    musicu: "https://u.y.qq.com/cgi-bin/musicu.fcg",
    lrc: "https://c.y.qq.com/lyric/fcgi-bin/fcg_query_lyric_new.fcg",
};

pub const MINIMUM_SCORE: u32 = crate::lyric_matching::PREFERRED_MINIMUM_SCORE;

pub(super) async fn search_manual(
    client: &reqwest::Client,
    query: &str,
) -> AppResult<Vec<super::correction::Candidate>> {
    let request = client.post("https://u.y.qq.com/cgi-bin/musicu.fcg")
        .header("Referer", "https://c.y.qq.com/")
        .json(&json!({"req_1": {"method": "DoSearchForQQMusicDesktop", "module": "music.search.SearchCgiService",
            "param": {"num_per_page": "20", "page_num": "1", "query": query, "search_type": 0}}}));
    let matches = read_json(request)
        .await
        .and_then(candidates)
        .map_err(|error| report_request_error("lyrics.qq.search.manual", error))?;
    Ok(matches
        .into_iter()
        .take(20)
        .map(|c| super::correction::Candidate {
            source: super::correction::Source::Qq,
            id: c.id,
            mid: c.mid,
            title: c.title,
            artist: c.artists.join(" / "),
            album: c.album,
            duration_ms: (c.duration.max(0.0) * 1000.0) as u64,
        })
        .collect())
}

pub(super) async fn fetch_manual(
    client: &reqwest::Client,
    value: &super::correction::Candidate,
    skip_qrc: bool,
) -> AppResult<Option<Lyrics>> {
    fetch_candidate(
        client,
        &Candidate {
            id: value.id,
            mid: value.mid.clone(),
            title: value.title.clone(),
            artists: vec![value.artist.clone()],
            album: value.album.clone(),
            duration: value.duration_ms as f64 / 1000.0,
        },
        skip_qrc,
        &LYRIC_ENDPOINTS,
        Duration::from_secs(6),
    )
    .await
}

#[derive(Debug)]
struct Candidate {
    id: Option<u64>,
    mid: String,
    title: String,
    artists: Vec<String>,
    album: String,
    duration: f64,
}

pub async fn lookup(
    client: &reqwest::Client,
    track: &Track,
    skip_qrc: bool,
    separators: &[String],
) -> AppResult<Option<Lyrics>> {
    // A single bounded stage; timeout and transport errors must remain retryable.
    tokio::time::timeout(
        Duration::from_secs(6),
        lookup_inner(client, track, skip_qrc, separators),
    )
    .await
    .inspect_err(|error| {
        crate::infrastructure::diagnostics::warn_error("lyrics.qq.search.timeout", error);
    })
    .map_err(|_| "QQ 歌词查询超时".to_string())?
}

async fn lookup_inner(
    client: &reqwest::Client,
    track: &Track,
    skip_qrc: bool,
    separators: &[String],
) -> AppResult<Option<Lyrics>> {
    let started = Instant::now();
    let exact = exact_candidates(track, separators);
    if !exact.is_empty() {
        return fetch_exact(client, exact, skip_qrc, &LYRIC_ENDPOINTS).await;
    }
    if track.title.trim().is_empty() || track.artist.trim().is_empty() {
        return Ok(None);
    }
    if [&track.title, &track.artist, &track.album]
        .iter()
        .any(|s| s.len() > 1024)
    {
        return Err("QQ 匹配元数据过长".into());
    }
    let mut best: Option<(Candidate, u32)> = None;
    let mut failed = false;
    for query in queries(track, separators) {
        let request = client.post("https://u.y.qq.com/cgi-bin/musicu.fcg")
            .header("Referer", "https://c.y.qq.com/")
            .json(&json!({"req_1": {"method": "DoSearchForQQMusicDesktop", "module": "music.search.SearchCgiService",
                "param": {"num_per_page": "20", "page_num": "1", "query": query, "search_type": 0}}}));
        let response = match read_json(request).await.and_then(candidates) {
            Ok(value) => value,
            Err(error) => {
                crate::infrastructure::diagnostics::report_error(
                    "lyrics.qq.search.request",
                    &error,
                );
                failed = true;
                continue;
            }
        };
        for candidate in response {
            let score = crate::lyric_matching::score(
                track,
                &candidate.title,
                &candidate.artists,
                &candidate.album,
                candidate.duration,
                separators,
            );
            if best.as_ref().is_none_or(|(_, previous)| score > *previous) {
                best = Some((candidate, score));
            }
        }
        if best
            .as_ref()
            .is_some_and(|(_, score)| *score >= MINIMUM_SCORE)
        {
            break;
        }
    }
    let Some((candidate, score)) = select_candidate(best) else {
        return if failed {
            Err("QQ 歌曲搜索失败".into())
        } else {
            Ok(None)
        };
    };
    let budget = Duration::from_secs(6)
        .saturating_sub(started.elapsed())
        .saturating_sub(Duration::from_millis(25));
    let mut lyrics =
        fetch_candidate(client, &candidate, skip_qrc, &LYRIC_ENDPOINTS, budget).await?;
    if let Some(value) = &mut lyrics {
        value.match_score = Some(score);
    }
    Ok(lyrics)
}

pub(super) fn has_exact_identity(track: &Track) -> bool {
    super::identity::Identity::of(track).is_some_and(|v| v.platform == "qqmusic")
}

fn exact_candidates(track: &Track, separators: &[String]) -> Vec<Candidate> {
    let Some(identity) = super::identity::Identity::of(track).filter(|v| v.platform == "qqmusic")
    else {
        return vec![];
    };
    let ids: Vec<_> = identity
        .identifiers
        .iter()
        .filter(|v| v.kind == "song-id")
        .filter_map(|v| v.id.as_str().parse::<u64>().ok())
        .collect();
    let mids: Vec<_> = identity
        .identifiers
        .iter()
        .filter(|v| v.kind == "song-mid")
        .map(|v| v.id.as_str())
        .collect();
    (0..ids.len().max(mids.len()))
        .map(|index| Candidate {
            id: ids.get(index).or(ids.first()).copied(),
            mid: mids
                .get(index)
                .or(mids.first())
                .copied()
                .unwrap_or_default()
                .into(),
            title: track.title.clone(),
            artists: crate::lyric_matching::split_artists(&track.artist, separators),
            album: track.album.clone(),
            duration: track.duration_ms as f64 / 1000.0,
        })
        .collect()
}

async fn fetch_exact(
    client: &reqwest::Client,
    candidates: Vec<Candidate>,
    skip_qrc: bool,
    endpoints: &LyricEndpoints<'_>,
) -> AppResult<Option<Lyrics>> {
    let started = Instant::now();
    let mut failed = false;
    for candidate in candidates {
        let budget = Duration::from_secs(6).saturating_sub(started.elapsed());
        match tokio::time::timeout(
            budget,
            fetch_candidate(client, &candidate, skip_qrc, endpoints, budget),
        )
        .await
        {
            Ok(Ok(Some(mut value))) => {
                // Exact platform identity outranks metadata candidates without reordering sources.
                value.match_score = Some(100);
                return Ok(Some(value));
            }
            Ok(Ok(None)) => {}
            _ => failed = true,
        }
    }
    if failed {
        Err("QQ 精准歌词查询失败".into())
    } else {
        Ok(None)
    }
}

async fn fetch_candidate(
    client: &reqwest::Client,
    candidate: &Candidate,
    skip_qrc: bool,
    endpoints: &LyricEndpoints<'_>,
    budget: Duration,
) -> AppResult<Option<Lyrics>> {
    let started = Instant::now();
    let mut primary = None;
    let mut failed = false;
    if let Some(id) = candidate.id.filter(|_| !skip_qrc) {
        // Reserve time for the legacy LRC request within the shared 6-second budget.
        let request = client
            .post(endpoints.qrc)
            .header("Referer", "https://c.y.qq.com/")
            .form(&[
                ("version", "15"),
                ("miniversion", "82"),
                ("lrctype", "4"),
                ("musicid", &id.to_string()),
            ]);
        let qrc = async {
            let response = read_text(request)
                .await
                .map_err(|error| report_request_error("lyrics.qq.qrc.request", error))?;
            tauri::async_runtime::spawn_blocking(move || decode_qrc_response(&response))
                .await
                .map_err(|e| e.to_string())?
        };
        if let Ok(Ok(Some(lyrics))) = (tokio::time::timeout(Duration::from_millis(900), async {
            qrc.await.inspect_err(|error| {
                failed = true;
                crate::infrastructure::diagnostics::warn("lyrics.qq.qrc", error)
            })
        })
        .await)
            .inspect_err(|error| {
                failed = true;
                crate::infrastructure::diagnostics::warn_error("lyrics.qq.qrc.timeout", error);
            })
        {
            if lyrics.translation.is_some() {
                return Ok(Some(lyrics));
            }
            primary = Some(lyrics);
        }
    }
    // Leave room for LRC, or return the already usable QRC before the outer deadline.
    let reserve = if primary.is_some() {
        Duration::from_millis(25)
    } else {
        Duration::from_millis(1400)
    };
    let modern_budget = budget
        .saturating_sub(started.elapsed())
        .saturating_sub(reserve);
    let modern = if candidate.id.is_some() && !modern_budget.is_zero() {
        tokio::time::timeout(
            modern_budget,
            play_lyric::fetch(client, candidate, skip_qrc, endpoints.musicu),
        )
        .await
        .inspect_err(|error| {
            failed = true;
            crate::infrastructure::diagnostics::warn_error("lyrics.qq.play_lyric.timeout", error);
        })
        .ok()
        .and_then(|result| {
            result
                .inspect_err(|error| {
                    failed = true;
                    crate::infrastructure::diagnostics::warn("lyrics.qq.modern", error)
                })
                .ok()
        })
        .flatten()
    } else {
        None
    };
    if let Some(mut lyrics) = primary {
        if let Some(extra) = modern {
            supplement(&mut lyrics, extra);
        }
        return Ok(Some(lyrics));
    }
    if modern
        .as_ref()
        .is_some_and(|l| !skip_qrc || l.format != "qrc")
    {
        return Ok(modern);
    }
    failed |= modern.is_some(); // A response rejected by the requested format is retryable.
                                // Exhaust the richer endpoint before falling back to the legacy songmid LRC.
    if candidate.mid.is_empty() {
        return if failed {
            Err("QQ 歌词接口请求失败".into())
        } else {
            Ok(None)
        };
    }
    let request = client
        .get(endpoints.lrc)
        .header("Referer", "https://c.y.qq.com/")
        .query(&[
            ("songmid", candidate.mid.as_str()),
            ("g_tk", "5381"),
            ("format", "json"),
            ("inCharset", "utf8"),
            ("outCharset", "utf-8"),
            ("platform", "yqq"),
            ("needNewCode", "0"),
            ("loginUin", "0"),
            ("hostUin", "0"),
        ]);
    let response = read_json(request)
        .await
        .map_err(|error| report_request_error("lyrics.qq.lrc.request", error))?;
    let mut lyrics = decode_lyrics(response)?;
    if lyrics.is_none() && failed {
        return Err("QQ 歌词接口请求失败".into());
    }
    if let Some(value) = &mut lyrics {
        if let Some(extra) = modern {
            supplement(value, extra);
        }
    }
    Ok(lyrics)
}

fn supplement(lyrics: &mut Lyrics, extra: Lyrics) {
    let mut bytes = lyrics.content.len()
        + lyrics.translation.as_ref().map_or(0, String::len)
        + lyrics.romanization.as_ref().map_or(0, String::len);
    for (target, candidate) in [
        (&mut lyrics.translation, extra.translation),
        (&mut lyrics.romanization, extra.romanization),
    ] {
        if target.is_none() {
            if let Some(text) = candidate.filter(|s| bytes + s.len() <= MAX_LYRIC_BYTES) {
                bytes += text.len();
                *target = Some(text);
            }
        }
    }
}

fn report_request_error(operation: &str, error: anyhow::Error) -> String {
    crate::infrastructure::diagnostics::report_error(operation, &error);
    format!("{error:#}")
}

async fn read_json(request: reqwest::RequestBuilder) -> Result<Value> {
    serde_json::from_str(&read_text(request).await?).context("QQ 响应 JSON 解析失败")
}

async fn read_text(request: reqwest::RequestBuilder) -> Result<String> {
    let mut response = request
        .timeout(Duration::from_millis(1400))
        .send()
        .await
        .context("QQ HTTP 请求发送失败")?
        .error_for_status()
        .context("QQ HTTP 响应状态错误")?;
    if response
        .content_length()
        .is_some_and(|n| n > MAX_LYRIC_BYTES as u64)
    {
        anyhow::bail!("QQ 响应过大");
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.context("QQ HTTP 响应正文读取失败")? {
        if bytes.len() + chunk.len() > MAX_LYRIC_BYTES {
            anyhow::bail!("QQ 响应过大");
        }
        bytes.extend_from_slice(&chunk);
    }
    String::from_utf8(bytes).context("QQ 响应 UTF-8 解码失败")
}

fn candidates(value: Value) -> Result<Vec<Candidate>> {
    if value["code"].as_i64() != Some(0) || value["req_1"]["code"].as_i64() != Some(0) {
        let code = value["code"]
            .as_i64()
            .map_or_else(|| "missing_or_invalid".to_owned(), |code| code.to_string());
        let request_code = value["req_1"]["code"]
            .as_i64()
            .map_or_else(|| "missing_or_invalid".to_owned(), |code| code.to_string());
        anyhow::bail!(
            "QQ 搜索返回错误 stage=response_code code={code} request_code={request_code}"
        );
    }
    let list = value
        .pointer("/req_1/data/body/song/list")
        .and_then(Value::as_array)
        .context("QQ 搜索响应无效 stage=song_list")?;
    let mut result = Vec::new();
    for track in list.iter().take(20) {
        for entry in
            std::iter::once(track).chain(track["grp"].as_array().into_iter().flatten().take(20))
        {
            if entry["title"].as_str().is_some_and(|s| s.len() > 1024)
                || entry["album"]["title"]
                    .as_str()
                    .is_some_and(|s| s.len() > 1024)
                || entry["singer"].as_array().is_some_and(|artists| {
                    artists.len() > 32
                        || artists
                            .iter()
                            .any(|a| a["name"].as_str().is_some_and(|s| s.len() > 1024))
                })
            {
                anyhow::bail!("QQ 搜索元数据过长");
            }
            if let Some(mid) = entry["mid"]
                .as_str()
                .filter(|s| !s.is_empty() && s.len() <= 128)
            {
                result.push(Candidate {
                    id: entry["id"].as_u64().filter(|id| *id > 0),
                    mid: mid.into(),
                    title: entry["title"].as_str().unwrap_or_default().into(),
                    artists: entry["singer"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(|a| a["name"].as_str().map(str::to_owned))
                        .collect(),
                    album: entry["album"]["title"].as_str().unwrap_or_default().into(),
                    duration: entry["interval"].as_f64().unwrap_or_default(),
                });
            }
        }
    }
    Ok(result)
}

fn decode_qrc_response(response: &str) -> AppResult<Option<Lyrics>> {
    let response = response
        .trim()
        .trim_start_matches("<!--")
        .trim_end_matches("-->")
        .trim();
    let mut reader = Reader::from_str(response);
    let mut field = None;
    let mut original = String::new();
    let mut translation = String::new();
    let mut romanization = String::new();
    loop {
        match reader
            .read_event()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn_error("lyrics.qq.qrc.parse_xml", error);
            })
            .map_err(|_| "QQ QRC 响应 XML 无效")?
        {
            Event::Start(e) => {
                field = match e.local_name().as_ref() {
                    b"content" => Some(0),
                    b"contentts" => Some(1),
                    b"contentroma" => Some(2),
                    _ => None,
                };
            }
            Event::CData(e) => {
                if let Some(field) = field {
                    let text = e
                        .decode()
                        .inspect_err(|error| {
                            crate::infrastructure::diagnostics::warn_error(
                                "lyrics.qq.qrc.decode_cdata",
                                error,
                            );
                        })
                        .map_err(|_| "QQ QRC 字段编码无效")?;
                    match field {
                        0 => original.push_str(&text),
                        1 => translation.push_str(&text),
                        _ => romanization.push_str(&text),
                    }
                }
            }
            Event::Text(e) => {
                if let Some(field) = field {
                    let text = e
                        .xml_content()
                        .inspect_err(|error| {
                            crate::infrastructure::diagnostics::warn_error(
                                "lyrics.qq.qrc.decode_text",
                                error,
                            );
                        })
                        .map_err(|_| "QQ QRC 字段编码无效")?;
                    match field {
                        0 => original.push_str(&text),
                        1 => translation.push_str(&text),
                        _ => romanization.push_str(&text),
                    }
                }
            }
            Event::End(_) => field = None,
            Event::DocType(_) => return Err("QQ QRC 不支持 DTD".into()),
            Event::Eof => break,
            _ => {}
        }
    }
    if original.trim().is_empty() {
        return Ok(None);
    }
    let content = decode_qrc_field(&original)?;
    static WORDS: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?m)^\[\d+,\d+\].+\(\d+,\d+\)").unwrap());
    if !WORDS.is_match(&content) {
        return Err("QQ QRC 缺少逐词时间轴".into());
    }
    // Auxiliary failures must not discard usable word-level lyrics.
    let auxiliary = |text: &str| {
        (!text.trim().is_empty())
            .then(|| {
                decode_qrc_field(text)
                    .inspect_err(|error| {
                        crate::infrastructure::diagnostics::warn(
                            "lyrics.qq.qrc.decode_auxiliary",
                            error,
                        );
                    })
                    .ok()
            })
            .flatten()
    };
    let translation = auxiliary(&translation);
    let romanization = auxiliary(&romanization);
    if content.len()
        + translation.as_ref().map_or(0, String::len)
        + romanization.as_ref().map_or(0, String::len)
        > MAX_LYRIC_BYTES
    {
        return Err("QQ QRC 正文与附加歌词过大".into());
    }
    Ok(Some(Lyrics {
        match_score: None,
        source: "qq".into(),
        format: "qrc".into(),
        content,
        translation,
        romanization,
    }))
}

fn decode_qrc_field(text: &str) -> AppResult<String> {
    if text.len() > MAX_LYRIC_BYTES {
        return Err("QRC 字段过大".into());
    }
    // Some fields contain plaintext LRC or QRC XML rather than ciphertext.
    let text = if text.trim_start().starts_with('[') || text.trim_start().starts_with('<') {
        text.to_owned()
    } else {
        crate::qrc_decrypt::decrypt(text)?
    };
    if !text.trim_start().starts_with('<') {
        return Ok(text);
    }
    qrc_xml::unwrap(&text)
}

fn decode_lyrics(value: Value) -> AppResult<Option<Lyrics>> {
    if value["code"].as_i64() != Some(0) {
        return Err("QQ 歌词返回错误".into());
    }
    if !value["lyric"].is_string() {
        return Err("QQ 歌词响应无效".into());
    }
    let decode = |field: &str| -> AppResult<Option<String>> {
        let Some(text) = value[field].as_str().filter(|s| !s.trim().is_empty()) else {
            return Ok(None);
        };
        let bytes = STANDARD
            .decode(text)
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn_error(
                    "lyrics.qq.lrc.decode_base64",
                    error,
                );
            })
            .map_err(|_| "QQ 歌词编码无效")?;
        let text = String::from_utf8(bytes)
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn_error("lyrics.qq.lrc.decode_utf8", error);
            })
            .map_err(|_| "QQ 歌词编码无效")?;
        Ok((!text.trim().is_empty()).then_some(text))
    };
    let Some(content) = decode("lyric")? else {
        return Ok(None);
    };
    static TIMED: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\[\d+:\d{2}(?:[.:]\d+)?\][^\r\n]*\S").unwrap());
    if !TIMED.is_match(&content) {
        return Err("QQ 歌词缺少有效时间轴".into());
    }
    Ok(Some(Lyrics {
        match_score: None,
        source: "qq".into(),
        format: "lrc".into(),
        content,
        translation: decode("trans")?,
        romanization: decode("roma")?,
    }))
}

#[cfg(test)]
fn metadata_score(track: &Track, candidate: &Candidate) -> u32 {
    crate::lyric_matching::score(
        track,
        &candidate.title,
        &candidate.artists,
        &candidate.album,
        candidate.duration,
        &[" / ".into()],
    )
}

fn select_candidate(best: Option<(Candidate, u32)>) -> Option<(Candidate, u32)> {
    best
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn qrc_wrapper_keeps_literal_quotes_angles_and_auxiliary_attributes() {
        let content =
            "[ti:Sample]\n[1000,1000]A & B < C \"Color's\"(1000,1000)\n[2000,500]Next(2000,500)\n";
        for quote in ['\"', '\''] {
            let xml = format!(
                "<QrcInfos><Other LyricContent=\"wrong\"/><Lyric_1 LyricType=\"1\" LyricContent={quote}{content}{quote} Language=\"ja\"/></QrcInfos>"
            );
            assert_eq!(decode_qrc_field(&xml).unwrap(), content);
        }
    }
    #[test]
    fn broken_qrc_wrappers_recover_only_timestamped_lines() {
        for line in ["[1000,1000]A \"Color\"(1000,1000)", "[00:01.00]译文"] {
            let xml = format!("<Lyric_1 LyricContent=\"unterminated\n{line}\n</QrcInfos>");
            assert_eq!(decode_qrc_field(&xml).unwrap(), line);
        }
        assert_eq!(
            decode_qrc_field("<Qrc><Lyric_1 LyricContent=\"\"/>\n[00:01.00]Text\n</Qrc>").unwrap(),
            "[00:01.00]Text"
        );
        assert!(decode_qrc_field("<Lyric_1 LyricContent=\"broken").is_err());
        assert!(decode_qrc_field(
            "<!DOCTYPE Qrc><Qrc><Lyric_1 LyricContent=\"broken\n[1000,1000]Text(1000,1000)"
        )
        .is_err());
        assert!(decode_qrc_field(&format!(
            "<Lyric_1 LyricContent=\"broken\n[1000,1000]{}(1000,1000)",
            "a".repeat(MAX_LYRIC_BYTES)
        ))
        .is_err());
    }
    #[test]
    fn qrc_attributes_preserve_bare_ampersands_and_decode_valid_entities() {
        let xml = "<QrcInfos><LyricInfo><Lyric_1 LyricContent=\"[ti:A & B]\n[1000,1000]A &amp; B &#39;x&#39; &unknown;(1000,1000)\"/></LyricInfo></QrcInfos>";
        assert_eq!(
            decode_qrc_field(xml).unwrap(),
            "[ti:A & B]\n[1000,1000]A & B 'x' &unknown;(1000,1000)"
        );
    }
    #[test]
    fn low_qq_candidate_is_retained_for_fallback_comparison() {
        assert!(select_candidate(Some((candidate(), 79))).is_some());
    }
    #[test]
    fn encrypted_qrc_xml_auxiliary_fields_and_failures() {
        // Ciphertext generated by AMLL encryptQrcHex from original synthetic text.
        let fixture: Value =
            serde_json::from_str(include_str!("../../../tests/fixtures/qrc.json")).unwrap();
        let response = format!("<!--<response><content><![CDATA[{}]]></content><contentts><![CDATA[{}]]></contentts><contentroma><![CDATA[{}]]></contentroma></response>-->",
            fixture["encrypted"].as_str().unwrap(), fixture["translation"].as_str().unwrap(), fixture["romanization"].as_str().unwrap());
        let result = decode_qrc_response(&response).unwrap().unwrap();
        assert_eq!(result.format, "qrc");
        assert_eq!(result.content, fixture["main"].as_str().unwrap());
        assert_eq!(
            result.translation.as_deref(),
            Some("[00:01.00]你好世界\n[00:03.00]再次")
        );
        assert!(result.romanization.unwrap().contains("(1000,400)"));
        let bad_auxiliary = response.replace(fixture["translation"].as_str().unwrap(), "damaged");
        assert!(decode_qrc_response(&bad_auxiliary)
            .unwrap()
            .unwrap()
            .translation
            .is_none());
        assert!(decode_qrc_response("<response><content>not-hex</content></response>").is_err());
        assert!(decode_qrc_response("<response><content/></response>")
            .unwrap()
            .is_none());
        assert!(decode_qrc_field(fixture["oversized"].as_str().unwrap()).is_err());
        assert!(decode_qrc_field("0").is_err());
        assert!(decode_qrc_field("0000000000000000").is_err());
    }
    #[test]
    #[ignore = "requires the live QQ Music service"]
    fn live_qq_search_and_qrc_download() {
        let client = reqwest::Client::builder()
            .user_agent("NonsPlayer/0.1")
            .connect_timeout(Duration::from_millis(500))
            .build()
            .unwrap();
        let value =
            tauri::async_runtime::block_on(lookup(&client, &track(), false, &[" / ".into()]))
                .unwrap()
                .expect("晴天 should have matched QQ lyrics");
        assert_eq!(value.source, "qq");
        assert_eq!(value.format, "qrc");
        assert!(value.content.contains("["));
        println!("QQ live lookup: {} UTF-8 bytes", value.content.len());
        let lrc = tauri::async_runtime::block_on(lookup(&client, &track(), true, &[" / ".into()]))
            .unwrap()
            .unwrap();
        assert_eq!(lrc.format, "lrc");
    }
    pub(super) fn track() -> Track {
        Track::from_music(nons_music_adapter_sdk::identity::MusicTrack {
            title: "晴天".into(),
            aliases: vec![],
            artists: vec![],
            album_reference: None,
            artist: "周杰伦 / 杨瑞代".into(),
            album: "叶惠美".into(),
            duration_ms: 269000,
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
        .unwrap()
    }
    pub(super) fn candidate() -> Candidate {
        Candidate {
            id: Some(97773),
            mid: "abc".into(),
            title: "晴天".into(),
            artists: vec!["周杰伦".into()],
            album: "叶惠美".into(),
            duration: 269.0,
        }
    }
    #[test]
    fn score_preserves_weights_best_artist_and_preview_duration() {
        let mut c = candidate();
        assert_eq!(metadata_score(&track(), &c), 100);
        c.duration = 30.0;
        assert_eq!(metadata_score(&track(), &c), 90);
        c.artists = vec!["无关歌手".into()];
        assert!(metadata_score(&track(), &c) < MINIMUM_SCORE);
        c = candidate();
        c.duration = 274.5;
        assert_eq!(metadata_score(&track(), &c), 95);
    }
    #[test]
    fn f23_prefix_bonus_is_not_standard_winkler() {
        assert!((similarity("abcdefghijx", "abcdefghijy") - 0.9944903597687237).abs() < 0.000001);
        assert!((similarity("MARTHA", "MARHTA") - 0.9611110849380493).abs() < 0.000001);
        assert_eq!(compare(" 晴天 ", "晴天"), 1.0);
    }
    #[test]
    fn search_keeps_versions_and_includes_grouped_candidates() {
        assert_eq!(without_translation("Hello（你好）"), "Hello");
        assert_eq!(without_translation("Hello（现场版）"), "Hello（现场版）");
        let result = candidates(json!({"code":0,"req_1":{"code":0,"data":{"body":{"song":{"list":[{"mid":"a","title":"晴天","grp":[{"mid":"b","title":"晴天"}]}]}}}}})).unwrap();
        assert_eq!(result.len(), 2);
    }
    #[test]
    fn bad_responses_are_errors_not_permanent_misses() {
        assert!(candidates(json!({"code":0})).is_err());
        assert!(decode_lyrics(json!({"code":1})).is_err());
        assert!(decode_lyrics(json!({"code":0,"lyric":"not base64"})).is_err());
        assert!(
            decode_lyrics(json!({"code":0,"lyric":STANDARD.encode("<html>error</html>")})).is_err()
        );
        assert!(decode_lyrics(json!({"code":0,"lyric":""}))
            .unwrap()
            .is_none());
        let lyrics = decode_lyrics(json!({"code":0,"lyric":STANDARD.encode("[00:01.00]晴天"),"trans":STANDARD.encode("[00:01.00]Sunny day")})).unwrap().unwrap();
        assert_eq!(lyrics.source, "qq");
        assert_eq!(lyrics.translation.as_deref(), Some("[00:01.00]Sunny day"));
    }

    #[test]
    fn search_failure_preserves_numeric_response_codes_without_response_payloads() {
        let error = candidates(json!({
            "code": 1000,
            "req_1": {"code": -101, "message": "SECRET"},
            "credential": "SECRET"
        }))
        .unwrap_err();
        let reason = format!("{error:#}");
        assert!(reason.contains("stage=response_code code=1000 request_code=-101"));
        assert!(!reason.contains("SECRET"));

        let invalid = candidates(json!({"code":"SECRET", "req_1":{"code":{ "private":"SECRET" }}}))
            .unwrap_err();
        assert!(invalid.to_string().contains("missing_or_invalid"));
        assert!(!invalid.to_string().contains("SECRET"));

        crate::infrastructure::diagnostics::test_log::start();
        report_request_error("lyrics.qq.search.request", error);
        let records = crate::infrastructure::diagnostics::test_log::take();
        let record = records
            .iter()
            .find(|record| record.contains("lyrics.qq.search.request"))
            .unwrap();
        assert!(record.contains("code=1000 request_code=-101"));
        assert!(record.contains("backtrace="));
        assert!(record.contains("backtrace_kind="));
        assert!(record.contains("qq_lyrics"));
        assert!(!record.contains("disabled backtrace"));
        assert!(!record.contains("SECRET"));
    }

    #[test]
    fn qq_http_failure_keeps_typed_transport_cause_and_context_until_reporting() {
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        let error = tauri::async_runtime::block_on(read_json(client.get("http://127.0.0.1:0/")))
            .unwrap_err();
        assert!(error.to_string().contains("QQ HTTP 请求发送失败"));
        assert!(error
            .chain()
            .any(|cause| cause.downcast_ref::<reqwest::Error>().is_some()));
        assert!(error.chain().count() > 2);

        crate::infrastructure::diagnostics::test_log::start();
        report_request_error("lyrics.qq.search.request", error);
        let records = crate::infrastructure::diagnostics::test_log::take();
        let record = records
            .iter()
            .find(|record| record.contains("lyrics.qq.search.request"))
            .unwrap();
        assert!(record.contains("QQ HTTP 请求发送失败"));
        assert!(record.contains("backtrace="));
        assert!(record.contains("qq_lyrics"));
        assert!(!record.contains("http://127.0.0.1"));
    }
}
