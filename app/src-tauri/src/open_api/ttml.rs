use std::{
    cmp::Reverse,
    collections::{BTreeMap, BinaryHeap},
    panic::{catch_unwind, AssertUnwindSafe},
};

use amll_lyric::{lrc, qrc, yrc, LyricLineOwned};
use quick_xml::{events::Event, Reader};
use serde::Deserialize;
use ttml_processor::{
    amll::{to_ttml_result, AmllLyricLine, AmllLyricResult, AmllLyricWord},
    generate_ttml,
    model::TTMLResult,
    GeneratorConfig,
};

use crate::{
    infrastructure::diagnostics,
    model::{Lyrics, MAX_LYRIC_BYTES},
};

const AUXILIARY_TOLERANCE_MS: u64 = 500;
const FALLBACK_LINE_DURATION_MS: u64 = 5_000;
const MAX_TIMESTAMP_MS: u64 = 59_999_999;
const MAX_XML_DEPTH: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LyricTiming {
    Word,
    Line,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct GeneratedTtml {
    pub(crate) content: String,
    pub(crate) timing: LyricTiming,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ConvertStage {
    Input,
    Parse,
    Normalize,
    Generate,
    Output,
}

impl ConvertStage {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Input => "input",
            Self::Parse => "parse",
            Self::Normalize => "normalize",
            Self::Generate => "generate",
            Self::Output => "output",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ConversionError {
    pub(crate) stage: ConvertStage,
    pub(crate) detail: String,
}

impl ConversionError {
    fn new(stage: ConvertStage, detail: impl Into<String>) -> Self {
        Self {
            stage,
            detail: detail.into(),
        }
    }
}

impl std::fmt::Display for ConversionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.stage.as_str(), self.detail)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LyricCue {
    pub(crate) start_ms: u64,
    pub(crate) end_ms: u64,
}

pub(crate) fn convert(
    lyrics: &Lyrics,
    duration_ms: u64,
    cue: Option<LyricCue>,
) -> Result<GeneratedTtml, ConversionError> {
    ensure_input_size(lyrics)?;
    let cue = cue.filter(|_| lyrics.source == "local");
    match catch_unwind(AssertUnwindSafe(|| convert_inner(lyrics, duration_ms, cue))) {
        Ok(Ok(output)) => {
            if output.content.len() > MAX_LYRIC_BYTES {
                return Err(ConversionError::new(
                    ConvertStage::Output,
                    "TTML exceeds 2 MiB",
                ));
            }
            Ok(output)
        }
        Ok(Err(error)) => {
            diagnostics::warn(
                &format!("open_api.lyrics.convert.{}", error.stage.as_str()),
                &error.detail,
            );
            Err(error)
        }
        Err(panic) => {
            let detail = panic_message(panic);
            diagnostics::warn("open_api.lyrics.convert.panic", &detail);
            Err(ConversionError::new(ConvertStage::Parse, detail))
        }
    }
}

fn convert_inner(
    lyrics: &Lyrics,
    duration_ms: u64,
    cue: Option<LyricCue>,
) -> Result<GeneratedTtml, ConversionError> {
    match lyrics.format.as_str() {
        "ttml" => convert_ttml(lyrics, duration_ms, cue),
        "qrc" | "yrc" | "lrc" => convert_timed(lyrics, duration_ms, cue),
        format => Err(ConversionError::new(
            ConvertStage::Input,
            format!("unsupported format {format}"),
        )),
    }
}

fn convert_ttml(
    lyrics: &Lyrics,
    duration_ms: u64,
    cue: Option<LyricCue>,
) -> Result<GeneratedTtml, ConversionError> {
    if !valid_ttml(&lyrics.content) {
        return Err(ConversionError::new(
            ConvertStage::Parse,
            "invalid TTML envelope",
        ));
    }
    let mut parsed = ttml_processor::parse_ttml(&lyrics.content)
        .map_err(|error| ConversionError::new(ConvertStage::Parse, error.to_string()))?;
    if parsed.lines.is_empty() {
        return Err(ConversionError::new(
            ConvertStage::Parse,
            "TTML has no lines",
        ));
    }
    let timing = parsed
        .metadata
        .timing_mode
        .as_deref()
        .map(|mode| {
            if mode.eq_ignore_ascii_case("Line") {
                LyricTiming::Line
            } else {
                LyricTiming::Word
            }
        })
        .unwrap_or_else(|| {
            if parsed
                .lines
                .iter()
                .any(|line| line.words.as_ref().is_some_and(|words| words.len() > 1))
            {
                LyricTiming::Word
            } else {
                LyricTiming::Line
            }
        });
    if lyrics.translation.is_none() && lyrics.romanization.is_none() && cue.is_none() {
        return Ok(GeneratedTtml {
            content: lyrics.content.clone(),
            timing,
        });
    }

    let mut starts = Vec::new();
    for line in &parsed.lines {
        starts.push(u64::from(line.start_time));
        if let Some(background) = &line.background_vocal {
            starts.push(u64::from(background.start_time));
        }
    }
    let translations = parse_auxiliary(lyrics.translation.as_deref().unwrap_or_default())?;
    let romanizations = parse_auxiliary(lyrics.romanization.as_deref().unwrap_or_default())?;
    let translations = match_auxiliary(&starts, &translations);
    let romanizations = match_auxiliary(&starts, &romanizations);
    let mut target = 0usize;
    for line in &mut parsed.lines {
        merge_rich_auxiliary(
            &mut line.translations,
            &mut line.romanizations,
            translations[target].as_deref(),
            romanizations[target].as_deref(),
        );
        target += 1;
        if let Some(background) = &mut line.background_vocal {
            merge_rich_auxiliary(
                &mut background.translations,
                &mut background.romanizations,
                translations[target].as_deref(),
                romanizations[target].as_deref(),
            );
            target += 1;
        }
    }
    if let Some(cue) = cue.filter(|cue| cue.end_ms > cue.start_ms) {
        parsed.lines.retain(|line| {
            u64::from(line.start_time) >= cue.start_ms && u64::from(line.start_time) < cue.end_ms
        });
        if parsed.lines.is_empty() {
            return Err(ConversionError::new(
                ConvertStage::Normalize,
                "CUE range has no lyrics",
            ));
        }
        for line in &mut parsed.lines {
            shift_rich_range(&mut line.start_time, &mut line.end_time, cue);
            if let Some(words) = &mut line.words {
                for word in words {
                    shift_rich_range(&mut word.start_time, &mut word.end_time, cue);
                    if let Some(ruby) = &mut word.ruby {
                        for tag in ruby {
                            shift_rich_range(&mut tag.start_time, &mut tag.end_time, cue);
                        }
                    }
                }
            }
            shift_rich_sublyrics(&mut line.translations, cue);
            shift_rich_sublyrics(&mut line.romanizations, cue);
            if let Some(background) = &mut line.background_vocal {
                shift_rich_range(&mut background.start_time, &mut background.end_time, cue);
                if let Some(words) = &mut background.words {
                    for word in words {
                        shift_rich_range(&mut word.start_time, &mut word.end_time, cue);
                        if let Some(ruby) = &mut word.ruby {
                            for tag in ruby {
                                shift_rich_range(&mut tag.start_time, &mut tag.end_time, cue);
                            }
                        }
                    }
                }
                shift_rich_sublyrics(&mut background.translations, cue);
                shift_rich_sublyrics(&mut background.romanizations, cue);
            }
        }
        parsed.metadata.audio_mut().duration = Some(to_timestamp(duration_ms));
    }
    let line_timing = timing == LyricTiming::Line && !has_word_level_rich_content(&parsed);
    generate_rich(&parsed, timing, line_timing)
}

fn convert_timed(
    lyrics: &Lyrics,
    duration_ms: u64,
    cue: Option<LyricCue>,
) -> Result<GeneratedTtml, ConversionError> {
    validate_source_timings(&lyrics.format, &lyrics.content)?;
    let stripped;
    let content = match lyrics.format.as_str() {
        "qrc" => &lyrics.content,
        "yrc" | "lrc" => {
            stripped = strip_netease_metadata(&lyrics.content);
            stripped.as_str()
        }
        _ => unreachable!(),
    };
    let mut lines: Vec<PreparedLine> = match lyrics.format.as_str() {
        "qrc" => qrc::parse_qrc(content)
            .into_iter()
            .map(LyricLineOwned::from)
            .map(PreparedLine::from)
            .collect(),
        "yrc" => yrc::parse_yrc(content)
            .into_iter()
            .map(LyricLineOwned::from)
            .map(PreparedLine::from)
            .collect(),
        "lrc" if lyrics.source == "local" => parse_local_lrc(content)
            .into_iter()
            .map(PreparedLine::from)
            .collect(),
        "lrc" => lrc::parse_lrc(content)
            .into_iter()
            .map(LyricLineOwned::from)
            .map(PreparedLine::from)
            .collect(),
        _ => unreachable!(),
    };
    lines.retain(|line| line.words.iter().any(|word| !word.text.trim().is_empty()));
    if lines.is_empty() {
        return Err(ConversionError::new(
            ConvertStage::Parse,
            "lyrics have no lines",
        ));
    }

    let starts: Vec<_> = lines.iter().map(|line| line.start_time).collect();
    let translations = parse_auxiliary(lyrics.translation.as_deref().unwrap_or_default())?;
    let romanizations = parse_auxiliary(lyrics.romanization.as_deref().unwrap_or_default())?;
    let translations = match_auxiliary(&starts, &translations);
    let romanizations = match_auxiliary(&starts, &romanizations);
    let mut normalized = normalize_lines(lines, duration_ms, lyrics.format == "lrc")?;
    for (index, line) in normalized.iter_mut().enumerate() {
        if clean_auxiliary(&line.translated_lyric).trim().is_empty() {
            line.translated_lyric =
                clean_auxiliary(translations[index].as_deref().unwrap_or_default()).to_owned();
        }
        if clean_auxiliary(&line.roman_lyric).trim().is_empty() {
            line.roman_lyric =
                clean_auxiliary(romanizations[index].as_deref().unwrap_or_default()).to_owned();
        }
    }
    apply_prepared_cue(&mut normalized, cue)?;
    let timing = if lyrics.format == "lrc" {
        LyricTiming::Line
    } else {
        LyricTiming::Word
    };
    generate_legacy(&normalized, timing)
}

fn normalize_lines(
    lines: Vec<PreparedLine>,
    duration_ms: u64,
    final_to_max: bool,
) -> Result<Vec<PreparedLine>, ConversionError> {
    let mut result = lines;
    result.sort_by_key(|line| line.start_time);
    result.retain(|line| line.words.iter().any(|word| !word.text.trim().is_empty()));
    if result.is_empty() {
        return Err(ConversionError::new(
            ConvertStage::Normalize,
            "lyrics have no text",
        ));
    }
    let mut next_distinct = vec![None; result.len()];
    let mut greater_start = None;
    for index in (0..result.len()).rev() {
        let start = result[index].start_time;
        if greater_start.is_some_and(|time| time > start) {
            next_distinct[index] = greater_start;
        }
        if greater_start.is_none_or(|time| start < time) {
            greater_start = Some(start);
        }
    }
    for (index, line) in result.iter_mut().enumerate() {
        let start = line
            .words
            .first()
            .map_or(line.start_time, |word| word.start_time);
        let end = if line.end_time > start {
            line.end_time
        } else {
            next_distinct[index].unwrap_or_else(|| {
                if final_to_max {
                    MAX_TIMESTAMP_MS
                } else {
                    duration_ms.max(start.saturating_add(FALLBACK_LINE_DURATION_MS))
                }
            })
        }
        .min(MAX_TIMESTAMP_MS);
        if end < start {
            return Err(ConversionError::new(
                ConvertStage::Normalize,
                "line timeline is reversed",
            ));
        }
        let line_end = line
            .words
            .iter()
            .map(|word| word.start_time)
            .max()
            .unwrap_or(start)
            .max(end)
            .min(MAX_TIMESTAMP_MS);
        line.start_time = start;
        line.end_time = line_end;
        for word_index in 0..line.words.len() {
            let word_start = line.words[word_index].start_time;
            let next_start = line.words.get(word_index + 1).map(|word| word.start_time);
            let word_end = if line.words[word_index].end_time > word_start {
                line.words[word_index].end_time
            } else {
                line_end
            };
            let end = match next_start {
                Some(next) if next >= word_start && word_end > next => next,
                _ => word_end,
            };
            line.words[word_index].end_time = end.max(word_start).min(MAX_TIMESTAMP_MS);
        }
    }
    Ok(result)
}

fn apply_prepared_cue(
    lines: &mut Vec<PreparedLine>,
    cue: Option<LyricCue>,
) -> Result<(), ConversionError> {
    let Some(cue) = cue.filter(|cue| cue.end_ms > cue.start_ms) else {
        return Ok(());
    };
    lines.retain(|line| line.start_time >= cue.start_ms && line.start_time < cue.end_ms);
    if lines.is_empty() {
        return Err(ConversionError::new(
            ConvertStage::Normalize,
            "CUE range has no lyrics",
        ));
    }
    for line in lines {
        line.start_time -= cue.start_ms;
        line.end_time = line.end_time.min(cue.end_ms) - cue.start_ms;
        for word in &mut line.words {
            word.start_time = word.start_time.clamp(cue.start_ms, cue.end_ms) - cue.start_ms;
            word.end_time = word.end_time.clamp(cue.start_ms, cue.end_ms) - cue.start_ms;
        }
    }
    Ok(())
}

fn generate_legacy(
    lines: &[PreparedLine],
    timing: LyricTiming,
) -> Result<GeneratedTtml, ConversionError> {
    let amll = AmllLyricResult {
        metadata: Vec::new(),
        lines: lines
            .iter()
            .map(|line| {
                let mut words = Vec::with_capacity(line.words.len());
                for word in &line.words {
                    let (text, ends_with_space) = split_one_trailing_space(word.text.clone());
                    words.push(AmllLyricWord {
                        word: if ends_with_space {
                            format!("{text} ").into()
                        } else {
                            text.into()
                        },
                        start_time: to_timestamp(word.start_time),
                        end_time: to_timestamp(word.end_time),
                        ..Default::default()
                    });
                }
                AmllLyricLine {
                    words,
                    translated_lyric: line.translated_lyric.clone(),
                    roman_lyric: line.roman_lyric.clone(),
                    is_bg: line.is_bg,
                    is_duet: line.is_duet,
                    start_time: to_timestamp(line.start_time),
                    end_time: to_timestamp(line.end_time),
                }
            })
            .collect(),
    };
    generate(&amll, timing)
}

fn generate(
    lines: &AmllLyricResult,
    timing: LyricTiming,
) -> Result<GeneratedTtml, ConversionError> {
    let line_timing = timing == LyricTiming::Line;
    let mut result = to_ttml_result(lines.clone(), None);
    result.metadata.timing_mode = Some(if line_timing { "Line" } else { "Word" }.into());
    generate_result(&result, timing, timing == LyricTiming::Line)
}

fn generate_rich(
    lines: &TTMLResult,
    timing: LyricTiming,
    line_timing: bool,
) -> Result<GeneratedTtml, ConversionError> {
    generate_result(lines, timing, line_timing)
}

fn generate_result(
    result: &TTMLResult,
    timing: LyricTiming,
    line_timing: bool,
) -> Result<GeneratedTtml, ConversionError> {
    let output = generate_ttml(
        result,
        &GeneratorConfig {
            use_apple_format_rules: false,
            format: false,
            line_timing,
        },
    )
    .map_err(|error| ConversionError::new(ConvertStage::Generate, error.to_string()))?;
    Ok(GeneratedTtml {
        content: output,
        timing,
    })
}

fn merge_rich_auxiliary(
    translations: &mut Option<Vec<ttml_processor::model::SubLyricContent>>,
    romanizations: &mut Option<Vec<ttml_processor::model::SubLyricContent>>,
    translation: Option<&str>,
    romanization: Option<&str>,
) {
    if translations.as_ref().is_none_or(|values| {
        values
            .iter()
            .all(|value| clean_auxiliary(&value.text).trim().is_empty())
    }) {
        *translations = translation.map(|text| vec![rich_auxiliary(text)]);
    }
    if romanizations.as_ref().is_none_or(|values| {
        values
            .iter()
            .all(|value| clean_auxiliary(&value.text).trim().is_empty())
    }) {
        *romanizations = romanization.map(|text| vec![rich_auxiliary(text)]);
    }
}

fn rich_auxiliary(text: &str) -> ttml_processor::model::SubLyricContent {
    ttml_processor::model::SubLyricContent {
        text: clean_auxiliary(text).to_owned(),
        ..Default::default()
    }
}

fn shift_rich_range(start: &mut u32, end: &mut u32, cue: LyricCue) {
    let start_ms = u64::from(*start);
    let end_ms = u64::from(*end);
    *start = to_timestamp(start_ms.clamp(cue.start_ms, cue.end_ms) - cue.start_ms);
    *end = to_timestamp(end_ms.clamp(cue.start_ms, cue.end_ms) - cue.start_ms);
}

fn shift_rich_sublyrics(
    values: &mut Option<Vec<ttml_processor::model::SubLyricContent>>,
    cue: LyricCue,
) {
    let Some(values) = values else {
        return;
    };
    for value in values {
        if let Some(words) = &mut value.words {
            for word in words {
                shift_rich_range(&mut word.start_time, &mut word.end_time, cue);
                if let Some(ruby) = &mut word.ruby {
                    for tag in ruby {
                        shift_rich_range(&mut tag.start_time, &mut tag.end_time, cue);
                    }
                }
            }
        }
    }
}

fn has_word_level_rich_content(result: &TTMLResult) -> bool {
    fn sublyrics_have_words(values: &Option<Vec<ttml_processor::model::SubLyricContent>>) -> bool {
        values.as_ref().is_some_and(|values| {
            values
                .iter()
                .any(|value| value.words.as_ref().is_some_and(|words| !words.is_empty()))
        })
    }
    result.lines.iter().any(|line| {
        line.words
            .as_ref()
            .is_some_and(|words| words.len() > 1 || words.iter().any(|word| word.ruby.is_some()))
            || line.background_vocal.is_some()
            || sublyrics_have_words(&line.translations)
            || sublyrics_have_words(&line.romanizations)
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PreparedWord {
    text: String,
    start_time: u64,
    end_time: u64,
}

impl From<amll_lyric::LyricWordOwned> for PreparedWord {
    fn from(word: amll_lyric::LyricWordOwned) -> Self {
        Self {
            text: word.word,
            start_time: word.start_time,
            end_time: word.end_time,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PreparedLine {
    words: Vec<PreparedWord>,
    translated_lyric: String,
    roman_lyric: String,
    is_bg: bool,
    is_duet: bool,
    start_time: u64,
    end_time: u64,
}

impl From<LyricLineOwned> for PreparedLine {
    fn from(line: LyricLineOwned) -> Self {
        Self {
            words: line.words.into_iter().map(PreparedWord::from).collect(),
            translated_lyric: line.translated_lyric,
            roman_lyric: line.roman_lyric,
            is_bg: line.is_bg,
            is_duet: line.is_duet,
            start_time: line.start_time,
            end_time: line.end_time,
        }
    }
}

fn ensure_input_size(lyrics: &Lyrics) -> Result<(), ConversionError> {
    let size = [
        Some(lyrics.content.as_str()),
        lyrics.translation.as_deref(),
        lyrics.romanization.as_deref(),
    ]
    .into_iter()
    .flatten()
    .try_fold(0usize, |total, value| total.checked_add(value.len()));
    match size {
        Some(size) if size <= MAX_LYRIC_BYTES => Ok(()),
        Some(_) | None => Err(ConversionError::new(
            ConvertStage::Input,
            "lyrics exceed 2 MiB",
        )),
    }
}

fn validate_source_timings(format: &str, content: &str) -> Result<(), ConversionError> {
    if format == "lrc" {
        for line in content.lines() {
            let mut rest = line;
            while let Some(open) = rest.find('[') {
                rest = &rest[open + 1..];
                let Some(close) = rest.find(']') else {
                    break;
                };
                let timestamp = &rest[..close];
                rest = &rest[close + 1..];
                if !timestamp.contains(':') {
                    continue;
                }
                if timestamp
                    .bytes()
                    .filter(|byte| byte.is_ascii_digit())
                    .count()
                    > 15
                {
                    return Err(ConversionError::new(
                        ConvertStage::Input,
                        "LRC timestamp exceeds integer range",
                    ));
                }
                let Some(time) = parse_lrc_timestamp(timestamp) else {
                    continue;
                };
                if time > MAX_TIMESTAMP_MS {
                    return Err(ConversionError::new(
                        ConvertStage::Input,
                        "LRC timestamp exceeds supported range",
                    ));
                }
            }
        }
        return Ok(());
    }
    if !matches!(format, "qrc" | "yrc") {
        return Ok(());
    }
    for (line_index, line) in content.lines().enumerate() {
        for (open, close) in [('[', ']'), ('(', ')')] {
            let mut rest = line;
            while let Some(start) = rest.find(open) {
                rest = &rest[start + 1..];
                let Some(end) = rest.find(close) else {
                    break;
                };
                let value = &rest[..end];
                rest = &rest[end + 1..];
                let Some((timestamp, duration)) = value.split_once(',') else {
                    continue;
                };
                if matches!(open, '[')
                    || format == "yrc"
                    || timestamp.bytes().all(|b| b.is_ascii_digit())
                {
                    let timestamp = parse_digits(timestamp)?;
                    let duration = parse_digits(duration.split(',').next().unwrap_or_default())?;
                    if timestamp > MAX_TIMESTAMP_MS
                        || duration > MAX_TIMESTAMP_MS
                        || timestamp
                            .checked_add(duration)
                            .is_none_or(|end| end > MAX_TIMESTAMP_MS)
                    {
                        return Err(ConversionError::new(
                            ConvertStage::Input,
                            format!("{format} timestamp overflows at line {}", line_index + 1),
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}

fn parse_digits(value: &str) -> Result<u64, ConversionError> {
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(ConversionError::new(
            ConvertStage::Input,
            "timestamp is not numeric",
        ));
    }
    value
        .parse()
        .map_err(|_| ConversionError::new(ConvertStage::Input, "timestamp exceeds integer range"))
}

fn parse_lrc_timestamp(value: &str) -> Option<u64> {
    let (minutes, rest) = value.split_once(':')?;
    let (seconds, fraction) = rest.split_once('.').or_else(|| rest.split_once(':'))?;
    if fraction.is_empty() || fraction.len() > 3 {
        return None;
    }
    let minutes = minutes.parse::<u64>().ok()?;
    let seconds = seconds.parse::<u64>().ok()?;
    let digits = fraction.len();
    let mut fraction = fraction.parse::<u64>().ok()?;
    fraction *= 10u64.pow(3 - digits as u32);
    minutes
        .checked_mul(60_000)?
        .checked_add(seconds.checked_mul(1_000)?)?
        .checked_add(fraction)
}

fn valid_ttml(content: &str) -> bool {
    if content.contains("<!DOCTYPE") || !crate::lyrics::valid_ttml(content) {
        return false;
    }
    let mut reader = Reader::from_str(content);
    let mut depth = 0usize;
    loop {
        match reader.read_event() {
            Ok(Event::Start(_)) => {
                depth += 1;
                if depth > MAX_XML_DEPTH {
                    return false;
                }
            }
            Ok(Event::End(_)) => depth = depth.saturating_sub(1),
            Ok(Event::Eof) => return depth == 0,
            Ok(_) => {}
            Err(_) => return false,
        }
    }
}

fn panic_message(panic: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = panic.downcast_ref::<&str>() {
        (*message).to_owned()
    } else if let Some(message) = panic.downcast_ref::<String>() {
        message.clone()
    } else {
        "lyrics parser panicked".into()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AuxiliaryLine {
    start: u64,
    text: String,
}

fn parse_auxiliary(content: &str) -> Result<Vec<AuxiliaryLine>, ConversionError> {
    if content.trim().is_empty() {
        return Ok(Vec::new());
    }
    let format = if content.lines().any(looks_like_yrc) {
        "yrc"
    } else if content.lines().any(looks_like_qrc) {
        "qrc"
    } else {
        "lrc"
    };
    validate_source_timings(format, content)?;
    let lines = if format == "yrc" {
        yrc::parse_yrc(content)
            .into_iter()
            .map(LyricLineOwned::from)
            .collect::<Vec<_>>()
    } else if format == "qrc" {
        qrc::parse_qrc(content)
            .into_iter()
            .map(LyricLineOwned::from)
            .collect::<Vec<_>>()
    } else {
        lrc::parse_lrc(content)
            .into_iter()
            .map(LyricLineOwned::from)
            .collect::<Vec<_>>()
    };
    let mut result: Vec<_> = lines
        .into_iter()
        .filter_map(|line| {
            let start = line
                .words
                .first()
                .map_or(line.start_time, |word| word.start_time);
            let account = line
                .words
                .into_iter()
                .map(|word| word.word)
                .collect::<String>()
                .trim()
                .to_owned();
            (!account.is_empty()).then_some(AuxiliaryLine {
                start,
                text: account,
            })
        })
        .collect();
    result.sort_by_key(|line| line.start);
    Ok(result)
}

fn looks_like_yrc(line: &str) -> bool {
    let Some(rest) = timed_prefix(line) else {
        return false;
    };
    rest.starts_with('(')
}

fn looks_like_qrc(line: &str) -> bool {
    timed_prefix(line).is_some()
}

fn timed_prefix(line: &str) -> Option<&str> {
    let rest = line.strip_prefix('[')?;
    let comma = rest.find(',')?;
    if comma == 0 || !rest[..comma].bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let rest = &rest[comma + 1..];
    let close = rest.find(']')?;
    if close == 0 || !rest[..close].bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    Some(&rest[close + 1..])
}

fn clean_auxiliary(text: &str) -> &str {
    if text.trim() == "//" {
        ""
    } else {
        text
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Pair {
    distance: u64,
    line: usize,
    entry: usize,
    left: usize,
    right: usize,
    left_version: u32,
    right_version: u32,
}

struct Bucket {
    time: u64,
    lines: Vec<usize>,
    entries: Vec<usize>,
    line_cursor: usize,
    entry_cursor: usize,
    previous: Option<usize>,
    next: Option<usize>,
    active: bool,
    version: u32,
}

fn match_auxiliary(starts: &[u64], entries: &[AuxiliaryLine]) -> Vec<Option<String>> {
    let mut matched = vec![None; starts.len()];
    if starts.is_empty() || entries.is_empty() {
        return matched;
    }
    let mut by_time = BTreeMap::<u64, Bucket>::new();
    for (index, start) in starts.iter().copied().enumerate() {
        by_time
            .entry(start)
            .or_insert_with(empty_bucket)
            .lines
            .push(index);
    }
    for (index, entry) in entries.iter().enumerate() {
        by_time
            .entry(entry.start)
            .or_insert_with(empty_bucket)
            .entries
            .push(index);
    }
    let mut buckets: Vec<_> = by_time
        .into_iter()
        .map(|(time, mut bucket)| {
            bucket.time = time;
            bucket
        })
        .collect();
    for bucket in &mut buckets {
        while bucket.line_cursor < bucket.lines.len() && bucket.entry_cursor < bucket.entries.len()
        {
            matched[bucket.lines[bucket.line_cursor]] =
                Some(entries[bucket.entries[bucket.entry_cursor]].text.clone());
            bucket.line_cursor += 1;
            bucket.entry_cursor += 1;
        }
        bucket.active =
            bucket.line_cursor < bucket.lines.len() || bucket.entry_cursor < bucket.entries.len();
    }
    let remaining: Vec<_> = buckets
        .iter()
        .enumerate()
        .filter_map(|(index, bucket)| bucket.active.then_some(index))
        .collect();
    for (position, index) in remaining.iter().copied().enumerate() {
        buckets[index].previous = position.checked_sub(1).map(|value| remaining[value]);
        buckets[index].next = remaining.get(position + 1).copied();
    }

    let mut queue = BinaryHeap::new();
    for index in &remaining {
        add_pair(&buckets, Some(*index), buckets[*index].next, &mut queue);
    }
    while let Some(Reverse(pair)) = queue.pop() {
        let valid = {
            let left = &buckets[pair.left];
            let right = &buckets[pair.right];
            let left_line = left.lines.get(left.line_cursor).copied();
            let left_entry = left.entries.get(left.entry_cursor).copied();
            let right_line = right.lines.get(right.line_cursor).copied();
            let right_entry = right.entries.get(right.entry_cursor).copied();
            left.active
                && right.active
                && left.next == Some(pair.right)
                && left.version == pair.left_version
                && right.version == pair.right_version
                && (left_line == Some(pair.line) || left_entry == Some(pair.entry))
                && (right_line == Some(pair.line) || right_entry == Some(pair.entry))
                && (left_line == Some(pair.line)) != (right_line == Some(pair.line))
        };
        if !valid {
            continue;
        }
        matched[pair.line] = Some(entries[pair.entry].text.clone());
        let left_previous = buckets[pair.left].previous;
        let right_next = buckets[pair.right].next;
        consume(&mut buckets[pair.left]);
        consume(&mut buckets[pair.right]);
        let left_active = buckets[pair.left].active;
        let right_active = buckets[pair.right].active;
        if !left_active && !right_active {
            reconnect(&mut buckets, left_previous, right_next);
            add_pair(&buckets, left_previous, right_next, &mut queue);
        } else if left_active && right_active {
            add_pair(&buckets, left_previous, Some(pair.left), &mut queue);
            add_pair(&buckets, Some(pair.left), Some(pair.right), &mut queue);
            add_pair(&buckets, Some(pair.right), right_next, &mut queue);
        } else if left_active {
            reconnect(&mut buckets, Some(pair.left), right_next);
            add_pair(&buckets, left_previous, Some(pair.left), &mut queue);
            add_pair(&buckets, Some(pair.left), right_next, &mut queue);
        } else {
            reconnect(&mut buckets, left_previous, Some(pair.right));
            add_pair(&buckets, left_previous, Some(pair.right), &mut queue);
            add_pair(&buckets, Some(pair.right), right_next, &mut queue);
        }
    }
    matched
}

fn empty_bucket() -> Bucket {
    Bucket {
        time: 0,
        lines: Vec::new(),
        entries: Vec::new(),
        line_cursor: 0,
        entry_cursor: 0,
        previous: None,
        next: None,
        active: true,
        version: 0,
    }
}

fn add_pair(
    buckets: &[Bucket],
    left: Option<usize>,
    right: Option<usize>,
    queue: &mut BinaryHeap<Reverse<Pair>>,
) {
    let (Some(left), Some(right)) = (left, right) else {
        return;
    };
    let first = &buckets[left];
    let second = &buckets[right];
    if !first.active || !second.active {
        return;
    }
    let left_is_line = first.line_cursor < first.lines.len();
    let right_is_line = second.line_cursor < second.lines.len();
    if left_is_line == right_is_line {
        return;
    }
    let (line, entry) = if left_is_line {
        (left, right)
    } else {
        (right, left)
    };
    let (line_bucket, entry_bucket) = (&buckets[line], &buckets[entry]);
    let distance = first.time.abs_diff(second.time);
    if distance > AUXILIARY_TOLERANCE_MS {
        return;
    }
    queue.push(Reverse(Pair {
        distance,
        line: line_bucket.lines[line_bucket.line_cursor],
        entry: entry_bucket.entries[entry_bucket.entry_cursor],
        left,
        right,
        left_version: first.version,
        right_version: second.version,
    }));
}

fn consume(bucket: &mut Bucket) {
    bucket.version = bucket.version.wrapping_add(1);
    if bucket.line_cursor < bucket.lines.len() {
        bucket.line_cursor += 1;
    } else {
        bucket.entry_cursor += 1;
    }
    if bucket.line_cursor == bucket.lines.len() && bucket.entry_cursor == bucket.entries.len() {
        bucket.active = false;
    }
}

fn reconnect(buckets: &mut [Bucket], previous: Option<usize>, next: Option<usize>) {
    if let Some(previous) = previous {
        buckets[previous].next = next;
    }
    if let Some(next) = next {
        buckets[next].previous = previous;
    }
}

fn strip_netease_metadata(content: &str) -> String {
    content
        .lines()
        .filter(|line| !is_netease_metadata(line))
        .collect::<Vec<_>>()
        .join("\n")
}

fn is_netease_metadata(line: &str) -> bool {
    let mut rest = line;
    loop {
        rest = rest.trim_start();
        if let Some(value) = rest.strip_prefix('[') {
            let Some(close) = value.find(']') else {
                return false;
            };
            rest = &value[close + 1..];
        } else {
            break;
        }
    }
    if !rest.starts_with('{') {
        return false;
    }
    #[derive(Deserialize)]
    struct Metadata {
        c: Vec<MetadataItem>,
    }
    #[derive(Deserialize)]
    struct MetadataItem {
        tx: Option<String>,
    }
    serde_json::from_str::<Metadata>(rest)
        .is_ok_and(|metadata| metadata.c.iter().any(|item| item.tx.is_some()))
}

#[derive(Clone, Debug)]
struct InlineTranslation {
    original_end: usize,
    translation: String,
}

fn parse_local_lrc(content: &str) -> Vec<LyricLineOwned> {
    let mut lines: Vec<_> = lrc::parse_lrc(content)
        .into_iter()
        .map(LyricLineOwned::from)
        .collect();
    if lines.is_empty() {
        return lines;
    }
    if apply_inline_translations(&mut lines) {
        return lines;
    }
    group_same_time_translations(lines)
}

fn apply_inline_translations(lines: &mut [LyricLineOwned]) -> bool {
    let matches: Vec<_> = lines
        .iter()
        .map(|line| split_inline_translation(&line_text(line)))
        .collect();
    let count = matches.iter().flatten().count();
    let detected = if lines.len() <= 2 {
        count == lines.len()
    } else {
        count >= 2 && (count * 4 >= lines.len() || count >= 6)
    };
    if !detected {
        return false;
    }
    for (line, translation) in lines.iter_mut().zip(matches) {
        if let Some(translation) = translation {
            line.words = without_translation(line, translation.original_end);
            line.translated_lyric = translation.translation;
        }
    }
    true
}

fn split_inline_translation(value: &str) -> Option<InlineTranslation> {
    let chars: Vec<_> = value.char_indices().collect();
    for (position, (index, character)) in chars.iter().copied().enumerate() {
        if !matches!(character, '/' | '／') {
            continue;
        }
        let previous_is_space = value[..index]
            .chars()
            .next_back()
            .is_some_and(char::is_whitespace);
        let next = chars.get(position + 1).copied();
        let (next_index, next_is_space) = next
            .map(|(next_index, next_character)| (next_index, next_character.is_whitespace()))
            .unwrap_or((value.len(), false));
        let repeated_slash = character == '/' && next.is_some_and(|(_, next)| next == '/');
        if !repeated_slash && !(previous_is_space && next_is_space) {
            continue;
        }
        let mut translation_start = next_index;
        if repeated_slash {
            translation_start += 1;
        }
        let translation = value[translation_start..].trim();
        if !value[..index].trim().is_empty() && !translation.is_empty() {
            return Some(InlineTranslation {
                original_end: index,
                translation: translation.to_owned(),
            });
        }
    }

    let (index, _) = chars
        .iter()
        .copied()
        .find(|(_, character)| matches!(character, '/' | '／'))?;
    let original = value[..index].trim();
    let translation = value[index + value[index..].chars().next()?.len_utf8()..].trim();
    if original.is_empty()
        || translation.is_empty()
        || !(contains_cjk(original) || contains_cjk(translation))
    {
        return None;
    }
    Some(InlineTranslation {
        original_end: index,
        translation: translation.to_owned(),
    })
}

fn contains_cjk(value: &str) -> bool {
    value.chars().any(|character| {
        matches!(
            character as u32,
            0x1100..=0x11ff
                | 0x3040..=0x31ff
                | 0x3400..=0x4dbf
                | 0x4e00..=0x9fff
                | 0xac00..=0xd7af
        )
    })
}

fn without_translation(line: &LyricLineOwned, end: usize) -> Vec<amll_lyric::LyricWordOwned> {
    let mut result = Vec::new();
    let mut offset = 0usize;
    for word in &line.words {
        if offset >= end {
            break;
        }
        let remaining = end - offset;
        if word.word.len() <= remaining {
            result.push(word.clone());
        } else {
            let kept = word.word[..remaining.min(word.word.len())].trim_end();
            if !kept.is_empty() {
                let mut word = word.clone();
                word.word = kept.to_owned();
                word.end_time = word.start_time;
                result.push(word);
            }
            break;
        }
        offset += word.word.len();
    }
    if result.is_empty() {
        line.words.clone()
    } else {
        result
    }
}

fn group_same_time_translations(lines: Vec<LyricLineOwned>) -> Vec<LyricLineOwned> {
    let mut groups = BTreeMap::<u64, Vec<LyricLineOwned>>::new();
    for line in lines {
        groups.entry(line.start_time).or_default().push(line);
    }
    let found = groups.values().any(|group| {
        let original = line_text(&group[0]).trim().to_owned();
        group.len() > 1
            && group
                .iter()
                .skip(1)
                .any(|line| line_text(line).trim() != original)
    });
    if !found {
        return groups.into_values().flatten().collect();
    }
    groups
        .into_values()
        .filter_map(|group| {
            let mut group = group.into_iter();
            let mut original = group.next()?;
            let base = line_text(&original).trim().to_owned();
            let translation = group
                .map(|line| line_text(&line).trim().to_owned())
                .filter(|line| !line.is_empty() && line != &base)
                .collect::<Vec<_>>()
                .join(" / ");
            if !translation.is_empty() {
                original.translated_lyric = translation;
            }
            Some(original)
        })
        .collect()
}

fn line_text(line: &LyricLineOwned) -> String {
    line.words.iter().map(|word| word.word.as_str()).collect()
}

fn split_one_trailing_space(mut text: String) -> (String, bool) {
    if text.ends_with(' ') {
        text.pop();
        (text, true)
    } else {
        (text, false)
    }
}

fn to_timestamp(value: u64) -> u32 {
    u32::try_from(value.min(u64::from(u32::MAX))).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Case {
        name: String,
        lyrics: Lyrics,
        duration_ms: u64,
        cue: Option<Cue>,
        expected: Vec<ExpectedLine>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Cue {
        start_ms: u64,
        end_ms: u64,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ExpectedLine {
        start_time: u32,
        end_time: u32,
        text: String,
        translation: String,
        romanization: String,
        words: Vec<ExpectedWord>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ExpectedWord {
        text: String,
        start_time: u32,
        end_time: u32,
    }

    fn fixtures() -> Vec<Case> {
        serde_json::from_str(include_str!("../../../tests/fixtures/open-api-lyrics.json")).unwrap()
    }

    fn lyrics(content: &str, format: &str, source: &str) -> Lyrics {
        Lyrics {
            match_score: None,
            source: source.into(),
            format: format.into(),
            content: content.into(),
            translation: None,
            romanization: None,
        }
    }

    fn assert_matches_fixture(case: Case) -> (String, String) {
        let name = case.name.clone();
        let cue = case.cue.map(|cue| LyricCue {
            start_ms: cue.start_ms,
            end_ms: cue.end_ms,
        });
        let output = convert(&case.lyrics, case.duration_ms, cue).unwrap();
        let parsed = ttml_processor::parse_ttml(&output.content).unwrap();
        assert_eq!(parsed.lines.len(), case.expected.len(), "{}", case.name);
        for (line, expected) in parsed.lines.iter().zip(&case.expected) {
            assert_eq!(line.start_time, expected.start_time, "{}", case.name);
            assert_eq!(line.end_time, expected.end_time, "{}", case.name);
            assert_eq!(line.text, expected.text, "{}", case.name);
            assert_eq!(
                line.translations
                    .as_ref()
                    .and_then(|values| values.first())
                    .map_or("", |value| value.text.as_str()),
                expected.translation,
                "{}",
                case.name
            );
            assert_eq!(
                line.romanizations
                    .as_ref()
                    .and_then(|values| values.first())
                    .map_or("", |value| value.text.as_str()),
                expected.romanization,
                "{}",
                case.name
            );
            let actual_words: Vec<_> = line.words.as_ref().map_or_else(
                || vec![(line.text.clone(), line.start_time, line.end_time)],
                |words| {
                    words
                        .iter()
                        .map(|word| {
                            (
                                if word.ends_with_space == Some(true) {
                                    format!("{} ", word.text)
                                } else {
                                    word.text.to_string()
                                },
                                word.start_time,
                                word.end_time,
                            )
                        })
                        .collect()
                },
            );
            assert_eq!(
                actual_words,
                expected
                    .words
                    .iter()
                    .map(|word| (word.text.clone(), word.start_time, word.end_time))
                    .collect::<Vec<_>>(),
                "{}",
                case.name
            );
        }
        (name, output.content)
    }

    #[test]
    fn conversion_matches_shared_ts_fixtures() {
        let output: Vec<_> = fixtures().into_iter().map(assert_matches_fixture).collect();
        if let Ok(path) = std::env::var("NONS_TTML_FIXTURE_OUTPUT") {
            let values: Vec<_> = output
                .into_iter()
                .map(|(name, content)| serde_json::json!({ "name": name, "content": content }))
                .collect();
            std::fs::write(path, serde_json::to_vec_pretty(&values).unwrap()).unwrap();
        }
    }

    #[test]
    fn exact_and_nearest_auxiliary_matching_is_bounded_and_deterministic() {
        let starts = [1000, 1400, 2000, 3000];
        let entries = [
            AuxiliaryLine {
                start: 1100,
                text: "甲".into(),
            },
            AuxiliaryLine {
                start: 1400,
                text: "//".into(),
            },
            AuxiliaryLine {
                start: 2500,
                text: "丙".into(),
            },
            AuxiliaryLine {
                start: 3501,
                text: "丁".into(),
            },
        ];
        let matched = match_auxiliary(&starts, &entries);
        assert_eq!(
            matched,
            vec![
                Some("甲".into()),
                Some("//".into()),
                Some("丙".into()),
                None
            ]
        );
    }

    #[test]
    fn auxiliary_tolerance_is_inclusive_at_exactly_500ms() {
        let starts = [1000];
        assert_eq!(
            match_auxiliary(
                &starts,
                &[AuxiliaryLine {
                    start: 500,
                    text: "edge".into()
                }]
            ),
            vec![Some("edge".into())]
        );
        assert_eq!(
            match_auxiliary(
                &starts,
                &[AuxiliaryLine {
                    start: 499,
                    text: "outside".into()
                }]
            ),
            vec![None]
        );
    }

    #[test]
    fn qrc_single_word_keeps_word_timing_and_lrc_uses_line_timing() {
        let output = convert(
            &lyrics("[1000,800]word(1000,800)", "qrc", "qq"),
            10_000,
            None,
        )
        .unwrap();
        assert_eq!(output.timing, LyricTiming::Word);
        let parsed = ttml_processor::parse_ttml(&output.content).unwrap();
        assert!(parsed.lines[0].words.is_some());
        assert_eq!(parsed.metadata.timing_mode.as_deref(), Some("Word"));
        let line = convert(&lyrics("[00:01.00]line", "lrc", "qq"), 10_000, None).unwrap();
        assert_eq!(line.timing, LyricTiming::Line);
    }

    #[test]
    fn cue_does_not_shift_online_lyrics() {
        let output = convert(
            &lyrics(
                "[1000,1000]A(1000,1000)\n[3000,1000]B(3000,1000)",
                "qrc",
                "qq",
            ),
            10_000,
            Some(LyricCue {
                start_ms: 2_000,
                end_ms: 8_000,
            }),
        )
        .unwrap();
        let parsed = ttml_processor::parse_ttml(&output.content).unwrap();
        assert_eq!(
            parsed
                .lines
                .iter()
                .map(|line| line.start_time)
                .collect::<Vec<_>>(),
            vec![1000, 3000]
        );
    }

    #[test]
    fn spaces_and_xml_entities_round_trip() {
        let mut source = lyrics("[1000,1000]A & (1000,400)<B>(1400,600)", "qrc", "qq");
        source.translation = Some("[00:01.00]A &amp; <B>".into());
        let output = convert(&source, 10_000, None).unwrap();
        let parsed = ttml_processor::parse_ttml(&output.content).unwrap();
        assert_eq!(parsed.lines[0].text, "A & <B>");
        assert_eq!(
            parsed.lines[0].translations.as_ref().unwrap()[0].text,
            "A &amp; <B>"
        );
        let words = parsed.lines[0].words.as_ref().unwrap();
        assert_eq!(words[0].text, "A &");
        assert_eq!(words[0].ends_with_space, Some(true));
    }

    #[test]
    fn native_ttml_is_returned_unchanged_with_extensions() {
        let source = lyrics(
            r#"<tt xmlns="http://www.w3.org/ns/ttml" xmlns:itunes="http://music.apple.com/lyric-ttml-internal" xmlns:ttm="http://www.w3.org/ns/ttml#metadata"><body><div begin="1.000" end="2.000"><p begin="1.000" end="2.000" itunes:key="L1"><span begin="1.000" end="1.500">Hello </span><span begin="1.500" end="2.000">世界</span><span ttm:role="x-translation">你好</span></p></div></body></tt>"#,
            "ttml",
            "local",
        );
        let output = convert(&source, 10_000, None).unwrap();
        assert_eq!(output.content, source.content);
        assert_eq!(output.timing, LyricTiming::Word);
    }

    #[test]
    fn oversized_input_and_output_are_rejected() {
        let mut source = lyrics("[00:01.00]A", "lrc", "qq");
        source.content = "x".repeat(MAX_LYRIC_BYTES + 1);
        assert_eq!(
            convert(&source, 10_000, None).unwrap_err().stage,
            ConvertStage::Input
        );
    }

    #[test]
    fn overflowing_timestamps_are_rejected_before_parser_arithmetic() {
        let qrc = lyrics(
            "[999999999999999999999,1]A(999999999999999999999,1)",
            "qrc",
            "qq",
        );
        assert_eq!(
            convert(&qrc, 10_000, None).unwrap_err().stage,
            ConvertStage::Input
        );
        let lrc = lyrics("[999999999999999999999:00.000]A", "lrc", "qq");
        assert_eq!(
            convert(&lrc, 10_000, None).unwrap_err().stage,
            ConvertStage::Input
        );
    }

    #[test]
    fn auxiliary_translation_and_romanization_overflow_is_rejected_before_parse() {
        let primary = "[1000,1000]A(1000,1000)";
        let mut translated = lyrics(primary, "qrc", "qq");
        translated.translation = Some("[999999999999999999999,1]X(999999999999999999999,1)".into());
        assert_eq!(
            convert(&translated, 10_000, None).unwrap_err().stage,
            ConvertStage::Input
        );

        let mut romanized = lyrics(primary, "qrc", "qq");
        romanized.romanization = Some("[999999999999999999999,1]X(999999999999999999999,1)".into());
        assert_eq!(
            convert(&romanized, 10_000, None).unwrap_err().stage,
            ConvertStage::Input
        );
    }

    #[test]
    fn doctype_and_excessive_xml_depth_are_rejected() {
        let doctype = lyrics(
            r#"<!DOCTYPE tt SYSTEM "external"><tt xmlns="http://www.w3.org/ns/ttml"><body><div><p begin="1.000" end="2.000">A</p></div></body></tt>"#,
            "ttml",
            "amll",
        );
        assert_eq!(
            convert(&doctype, 10_000, None).unwrap_err().stage,
            ConvertStage::Parse
        );
        let nested = format!(
            r#"<tt xmlns="http://www.w3.org/ns/ttml"><body>{}<p begin="1.000" end="2.000">A</p>{}</body></tt>"#,
            "<div>".repeat(260),
            "</div>".repeat(260)
        );
        let deep = lyrics(&nested, "ttml", "amll");
        assert_eq!(
            convert(&deep, 10_000, None).unwrap_err().stage,
            ConvertStage::Parse
        );
    }

    #[test]
    fn cue_shifts_rich_translation_words_and_keeps_background_vocals() {
        let source = lyrics(
            r#"<tt xmlns="http://www.w3.org/ns/ttml" xmlns:itunes="http://music.apple.com/lyric-ttml-internal" xmlns:ttm="http://www.w3.org/ns/ttml#metadata" itunes:timing="Line"><head><metadata><iTunesMetadata xmlns="http://music.apple.com/lyric-ttml-internal"><translations><translation type="replacement" xml:lang="zh-Hans"><text for="L1"><span begin="1.700" end="1.900">译文</span></text></translation></translations></iTunesMetadata></metadata></head><body><div><p begin="1.500" end="2.500" itunes:key="L1"><span begin="1.500" end="2.000">原文</span><span begin="2.000" end="2.500" ttm:role="x-bg">背景</span></p></div></body></tt>"#,
            "ttml",
            "local",
        );
        let output = convert(
            &source,
            10_000,
            Some(LyricCue {
                start_ms: 1_000,
                end_ms: 3_000,
            }),
        )
        .unwrap();
        let parsed = ttml_processor::parse_ttml(&output.content).unwrap();
        let line = &parsed.lines[0];
        assert_eq!(line.start_time, 500);
        assert!(line.background_vocal.is_some());
        let translation = &line.translations.as_ref().unwrap()[0];
        assert_eq!(translation.text, "译文");
        let word = &translation.words.as_ref().unwrap()[0];
        assert_eq!(word.start_time, 700);
        assert_eq!(word.end_time, 900);
    }
}
