//! AF-Media-Bar metadata score and query policy (MIT); see notices/.
use crate::model::{Lyrics, Track};
use nons_music_adapter_sdk::lyric::LyricMatchSubject;
use regex::Regex;
use serde::Serialize;
use std::sync::LazyLock;

pub(crate) const MINIMUM_SCORE: u32 = 60;
pub(crate) const PREFERRED_MINIMUM_SCORE: u32 = 80;

// Stable score ordering: the preferred QQ source wins ties.
pub(crate) fn select(preferred: Option<Lyrics>, fallback: Option<Lyrics>) -> Option<Lyrics> {
    match (preferred, fallback) {
        (Some(qq), Some(other)) if other.match_score.unwrap_or(0) > qq.match_score.unwrap_or(0) => {
            Some(other)
        }
        (Some(qq), _) => Some(qq),
        (None, other) => other,
    }
}

pub(crate) fn without_translation(text: &str) -> String {
    static SUFFIX: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"\s*[（(][^（）()]*[\x{4e00}-\x{9fff}][^（）()]*[）)]\s*$").unwrap()
    });
    static VERSION: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(?i)现场|現場|伴奏|翻唱|混音|重制|重製|倍速|live|remix|cover|acoustic|instrumental",
        )
        .unwrap()
    });
    if let Some(suffix) = SUFFIX.find(text) {
        if !VERSION.is_match(suffix.as_str()) {
            return text[..suffix.start()].trim().into();
        }
    }
    text.trim().into()
}

fn clean_title(text: &str) -> String {
    static PRIORITY: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"《(.+?)》|「(.+?)」").unwrap());
    static DECORATION: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"【.*?】|“.*?”").unwrap());
    let title = if let Some(value) = PRIORITY
        .captures(text)
        .and_then(|captures| captures.get(1).or_else(|| captures.get(2)))
    {
        value.as_str().to_owned()
    } else {
        let title = DECORATION.replace_all(text, "");
        let title = title.trim();
        if title.is_empty() {
            text.trim().to_owned()
        } else {
            title.to_owned()
        }
    };
    without_translation(&title)
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Subject {
    pub title: String,
    pub artists: Vec<String>,
    pub album: String,
    pub duration_ms: u64,
}

impl Subject {
    pub fn from_track(track: &Track, separators: &[String]) -> Self {
        Self {
            title: clean_title(&track.title),
            artists: split_artists(&track.artist, separators)
                .into_iter()
                .map(|artist| without_translation(&artist))
                .collect(),
            album: track.album.clone(),
            duration_ms: track.duration_ms,
        }
    }

    pub fn from_match(track: &Track, value: &LyricMatchSubject, separators: &[String]) -> Self {
        let title = clean_title(&value.title);
        let artists = value
            .artists
            .iter()
            .flat_map(|artist| split_artists(artist, separators))
            .map(|artist| without_translation(&artist))
            .filter(|artist| !artist.is_empty())
            .collect();
        Self {
            title: if title.is_empty() {
                clean_title(&track.title)
            } else {
                title
            },
            artists,
            album: value.album.clone().unwrap_or_else(|| track.album.clone()),
            duration_ms: value.duration_ms.unwrap_or(track.duration_ms),
        }
    }
}

pub(crate) fn queries(track: &Track, subject: &Subject, _separators: &[String]) -> Vec<String> {
    let mut titles = vec![subject.title.clone()];
    let raw = without_translation(track.title.trim());
    if !raw.is_empty() && raw != subject.title {
        titles.push(raw);
    }
    let artist = subject.artists.join(" ");
    let raw_artist = track.artist.trim();
    let mut result = Vec::new();
    let mut queries = Vec::new();
    for title in &titles {
        if !artist.is_empty() {
            queries.push(format!("{title} {artist}"));
        }
        if !raw_artist.is_empty() {
            queries.push(format!("{title} {raw_artist}"));
        }
    }
    for artist in &subject.artists {
        queries.extend(titles.iter().map(|title| format!("{title} {artist}")));
    }
    queries.extend(titles.iter().cloned());
    for query in queries {
        let query = query.trim().to_owned();
        if !query.is_empty() && !result.contains(&query) {
            result.push(query);
        }
    }
    result
}

pub(crate) fn compare(left: &str, right: &str) -> f64 {
    let left = left.trim().to_lowercase();
    let right = right.trim().to_lowercase();
    if left.is_empty() && right.is_empty() {
        return 1.0;
    }
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }
    similarity(&left, &right)
}

fn fingerprint(text: &str) -> String {
    static NON_WORD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[\p{P}\p{S}]").unwrap());
    let text = NON_WORD.replace_all(&text.to_lowercase(), " ").into_owned();
    let mut tokens: Vec<_> = text.split(' ').filter(|s| !s.is_empty()).collect();
    tokens.sort_by_key(|s| s.encode_utf16().collect::<Vec<_>>());
    tokens.join(" ")
}

pub(crate) fn score(
    subject: &Subject,
    title: &str,
    candidate_names: &[String],
    album: &str,
    duration: f64,
    separators: &[String],
) -> u32 {
    let request_artists = &subject.artists;
    let candidate_artists: Vec<_> = candidate_names
        .iter()
        .flat_map(|a| split_artists(a, separators))
        .collect();
    let total = if subject.title.trim().is_empty() || title.trim().is_empty() {
        let value = request_artists
            .iter()
            .flat_map(|a| {
                candidate_artists.iter().map(move |b| {
                    let left = fingerprint(&format!("{} {a}", subject.title));
                    let right = fingerprint(&format!("{} {b}", title));
                    if left.is_empty() || right.is_empty() {
                        0.0
                    } else {
                        similarity(&left, &right)
                    }
                })
            })
            .fold(0.0, f64::max);
        value
    } else {
        let artist = (!request_artists.is_empty() && !candidate_artists.is_empty()).then(|| {
            request_artists
                .iter()
                .flat_map(|a| candidate_artists.iter().map(move |b| compare(a, b)))
                .fold(0.0, f64::max)
        });
        let local = subject.duration_ms as f64 / 1000.0;
        let remote = duration;
        let duration = (local > 0.0 && remote > 0.0 && remote.is_finite())
            .then(|| (1.0 - ((local - remote).abs() - 1.0).max(0.0) / 9.0).max(0.0));
        let album = (!subject.album.trim().is_empty() && !album.trim().is_empty())
            .then(|| compare(&subject.album, album));
        let mut weight = 0.4;
        let mut value = 0.4 * compare(&subject.title, title);
        if let Some(artist) = artist {
            weight += 0.4;
            value += 0.4 * artist;
        }
        if let Some(album) = album {
            weight += 0.1;
            value += 0.1 * album;
        }
        if let Some(duration) = duration {
            weight += 0.1;
            value += 0.1 * duration;
        }
        value / weight
    };
    (100.0 * total).round_ties_even() as u32
}

// Faithful F23.StringSimilarity JaroWinkler port: UTF-16, uncapped prefix,
// length-dependent bonus, integer half-transpositions and f32 Jaro arithmetic.
// Copyright 2016 feature[23], MIT; see notices/F23-StringSimilarity-LICENSE.txt.
pub(crate) fn similarity(left: &str, right: &str) -> f64 {
    let a: Vec<_> = left.encode_utf16().collect();
    let b: Vec<_> = right.encode_utf16().collect();
    if a == b {
        return 1.0;
    }
    let (max, min) = if a.len() > b.len() {
        (&a, &b)
    } else {
        (&b, &a)
    };
    let range = (max.len() / 2).saturating_sub(1);
    let mut indexes = vec![None; min.len()];
    let mut flags = vec![false; max.len()];
    for (i, character) in min.iter().enumerate() {
        for j in i.saturating_sub(range)..(i + range + 1).min(max.len()) {
            if !flags[j] && *character == max[j] {
                indexes[i] = Some(j);
                flags[j] = true;
                break;
            }
        }
    }
    let first: Vec<_> = min
        .iter()
        .zip(indexes)
        .filter_map(|(c, i)| i.map(|_| c))
        .collect();
    if first.is_empty() {
        return 0.0;
    }
    let second: Vec<_> = max
        .iter()
        .zip(flags)
        .filter_map(|(c, f)| f.then_some(c))
        .collect();
    let half = first.iter().zip(second).filter(|(a, b)| *a != b).count() / 2;
    let m = first.len() as f32;
    let jaro = ((m / a.len() as f32 + m / b.len() as f32 + (m - half as f32) / m) as f64) / 3.0;
    if jaro > 0.7 {
        let prefix = a.iter().zip(&b).take_while(|(a, b)| a == b).count();
        jaro + 0.1f64.min(1.0 / max.len() as f64) * prefix as f64 * (1.0 - jaro)
    } else {
        jaro
    }
}

// AF-Media-Bar LyricsArtistPolicy: longest literals first, case-insensitive, deduplicated.
pub(crate) fn split_artists(text: &str, separators: &[String]) -> Vec<String> {
    let mut ordered: Vec<_> = separators.iter().filter(|s| !s.trim().is_empty()).collect();
    ordered.sort_by_key(|s| std::cmp::Reverse(s.chars().count()));
    let mut result = Vec::<String>::new();
    let mut start = 0;
    let mut index = 0;
    let add = |part: &str, result: &mut Vec<String>| {
        let part = part.trim();
        if !part.is_empty()
            && !result
                .iter()
                .any(|existing| existing.to_lowercase() == part.to_lowercase())
        {
            result.push(part.into());
        }
    };
    while index < text.len() {
        let found = ordered.iter().find(|separator| {
            text[index..]
                .get(..separator.len())
                .is_some_and(|prefix| prefix.to_lowercase() == separator.to_lowercase())
        });
        if let Some(separator) = found {
            add(&text[start..index], &mut result);
            index += separator.len();
            start = index;
        } else {
            index += text[index..].chars().next().unwrap().len_utf8();
        }
    }
    add(&text[start..], &mut result);
    if result.is_empty() {
        result.push(String::new());
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::TrackSource;

    fn track() -> Track {
        Track {
            key: "local:test".into(),
            title: "Home".into(),
            artist: "Sarah Kang & Sam Ock".into(),
            album: "Home".into(),
            duration_ms: 342400,
            aliases: vec![],
            artists: vec![],
            cover: Default::default(),
            source: TrackSource::Local {
                path: String::new(),
                netease_id: None,
                cue: None,
            },
        }
    }
    fn lyrics(source: &str, score: u32) -> Lyrics {
        Lyrics {
            source: source.into(),
            format: "lrc".into(),
            content: "[00:01.00]Home".into(),
            translation: None,
            romanization: None,
            match_score: Some(score),
        }
    }

    #[test]
    fn fallback_beats_low_qq_but_qq_wins_ties_and_survives_missing_fallback() {
        assert_eq!(
            select(Some(lyrics("qq", 79)), Some(lyrics("netease", 95)))
                .unwrap()
                .source,
            "netease"
        );
        assert_eq!(
            select(Some(lyrics("qq", 79)), Some(lyrics("netease", 79)))
                .unwrap()
                .source,
            "qq"
        );
        assert_eq!(select(Some(lyrics("qq", 59)), None).unwrap().source, "qq");
        assert_eq!(
            select(None, Some(lyrics("netease", 60))).unwrap().source,
            "netease"
        );
    }

    #[test]
    fn custom_separators_apply_to_both_sides_with_longest_literal_and_case_insensitive_dedup() {
        let separators = vec![" & ".into(), "&".into(), " feat. ".into()];
        let subject = Subject::from_track(&track(), &separators);
        assert_eq!(
            split_artists("Sarah Kang FEAT. Sam Ock & sam ock", &separators),
            ["Sarah Kang", "Sam Ock"]
        );
        assert_eq!(
            score(
                &subject,
                "Home",
                &["Other & Sam Ock".into()],
                "Home",
                342.4,
                &separators
            ),
            100
        );
        assert_eq!(split_artists("AC/DC", &[]), ["AC/DC"]);
    }

    #[test]
    fn search_preserves_original_and_all_individual_artist_queries_then_title_only() {
        let mut request = track();
        request.artist = "A & B & C & D".into();
        let subject = Subject::from_track(&request, &[" & ".into()]);
        assert_eq!(
            queries(&request, &subject, &[" & ".into()]),
            [
                "Home A B C D",
                "Home A & B & C & D",
                "Home A",
                "Home B",
                "Home C",
                "Home D",
                "Home"
            ]
        );
    }

    #[test]
    fn weights_preview_duration_empty_title_and_empty_fields_follow_af_policy() {
        let mut request = track();
        request.artist = "Sam Ock".into();
        let mut subject = Subject::from_track(&request, &[]);
        let names = vec!["Sam Ock".into()];
        assert_eq!(score(&subject, "Home", &names, "Home", 342.4, &[]), 100);
        assert_eq!(score(&subject, "Home", &names, "Home", 30.0, &[]), 90);
        assert_eq!(score(&subject, "Home", &names, "Home", 347.9, &[]), 95);
        subject.title.clear();
        subject.artists = vec!["Sam Ock Home".into()];
        assert_eq!(score(&subject, "Home", &names, "Other", 1.0, &[]), 100);
        assert_eq!(compare("", ""), 1.0);
        assert_eq!(compare("", "Home"), 0.0);
    }

    #[test]
    fn titles_prefer_bracketed_song_names_and_remove_decorations() {
        assert_eq!(clean_title("【MV】《Home》官方版"), "Home");
        assert_eq!(clean_title("【翻唱】「Home」"), "Home");
        assert_eq!(clean_title("“Home” 现场"), "现场");
        assert_eq!(clean_title("【MV】"), "【MV】");
    }

    #[test]
    fn missing_subject_fields_do_not_lose_weight() {
        let request = track();
        let subject = Subject::from_match(
            &request,
            &LyricMatchSubject {
                title: "《Home》".into(),
                artists: vec![],
                album: Some(String::new()),
                duration_ms: Some(request.duration_ms),
            },
            &[],
        );
        assert_eq!(subject.title, "Home");
        assert_eq!(
            score(&subject, "Home", &["Unknown".into()], "Other", 342.4, &[]),
            100
        );
        assert!(
            score(
                &subject,
                "Different",
                &["Unknown".into()],
                "Other",
                342.4,
                &[]
            ) >= 50
        );
    }
}
