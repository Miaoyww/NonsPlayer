//! QRC wrapper compatibility adapted from Lyricify XmlUtils (Apache-2.0)
//! and AF-Media-Bar's damaged-wrapper line fallback (MIT). See notices/.
use crate::{model::AppResult, storage::MAX_LYRIC_BYTES};
use quick_xml::{events::Event, Reader};
use regex::Regex;
use std::sync::LazyLock;

static CONTENT_START: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"<Lyric_1\b[^>]*?\bLyricContent\s*=\s*(["'])"#).unwrap());
static NEXT_ATTRIBUTE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^\s+[\w:.-]+\s*=\s*["']"#).unwrap());
static QRC_LINE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^[ \t]*\[\d+,\d+\][^\r\n]*\(-?\d+,\d+\)[^\r\n]*").unwrap());
static LRC_LINE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^[ \t]*(?:\[\d+:\d{1,2}(?:[.:]\d{1,3})?\])+[^\r\n]*").unwrap()
});

pub(super) fn unwrap(text: &str) -> AppResult<String> {
    if text.len() > MAX_LYRIC_BYTES {
        return Err("QRC 正文过大".into());
    }
    // Reject DTD even when a damaged wrapper would otherwise enter line recovery.
    if text.contains("<!DOCTYPE") {
        return Err("QRC 正文不支持 DTD".into());
    }
    let error = match read_content(text) {
        Ok(Some(content)) if !content.trim().is_empty() => return Ok(content),
        Ok(_) => "QRC 缺少有效 LyricContent".into(),
        Err(error) => error,
    };
    let wrapped = CONTENT_START.captures(text).map(|captures| {
        let start = captures.get(0).unwrap().end();
        (&text[start..], captures[1].chars().next().unwrap())
    });
    if let Some((body, quote)) = wrapped {
        // Lyricify treats a quote as the attribute terminator only before another
        // attribute or the element's end. Literal lyric quotes remain in the body.
        for (end, _) in body.match_indices(quote) {
            let suffix = &body[end + 1..];
            if suffix.trim_start().starts_with('>')
                || suffix.trim_start().starts_with("/>")
                || NEXT_ATTRIBUTE.is_match(suffix)
            {
                let raw = &body[..end];
                if !raw.trim().is_empty() {
                    return Ok(html_escape::decode_html_entities(raw).into_owned());
                }
                break;
            }
        }
    }
    // AF-Media-Bar scans valid timed lines once when wrapper recovery fails.
    // The same path handles original QRC and the independent LRC auxiliary fields.
    let payload = wrapped.map_or(text, |(body, _)| body);
    let pattern = if QRC_LINE.is_match(payload) {
        &*QRC_LINE
    } else {
        &*LRC_LINE
    };
    let lines = pattern
        .find_iter(payload)
        .map(|line| line.as_str().trim_start())
        .collect::<Vec<_>>()
        .join("\n");
    if !lines.is_empty() {
        return Ok(html_escape::decode_html_entities(&lines).into_owned());
    }
    Err(error)
}

fn read_content(text: &str) -> AppResult<Option<String>> {
    let mut reader = Reader::from_str(text);
    let mut content = None;
    loop {
        match reader
            .read_event()
            .map_err(|error| format!("QRC 正文 XML 无效: {error}"))?
        {
            Event::Start(element) | Event::Empty(element)
                if element.local_name().as_ref() == b"Lyric_1" =>
            {
                for attribute in element.attributes() {
                    let attribute =
                        attribute.map_err(|error| format!("QRC 正文属性无效: {error}"))?;
                    if attribute.key.as_ref() == b"LyricContent" {
                        let raw = reader
                            .decoder()
                            .decode(attribute.value.as_ref())
                            .map_err(|error| format!("QRC 正文编码无效: {error}"))?;
                        content = Some(html_escape::decode_html_entities(&raw).into_owned());
                    }
                }
            }
            Event::DocType(_) => return Err("QRC 正文不支持 DTD".into()),
            Event::Eof => return Ok(content),
            _ => {}
        }
    }
}
