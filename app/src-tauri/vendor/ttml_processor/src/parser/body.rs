use compact_str::CompactString;
use quick_xml::{
    Reader,
    events::{
        BytesStart,
        Event,
    },
};

use crate::{
    constants::{
        attrs,
        tags,
    },
    error::{
        ParseErrorKind,
        Result,
        ResultExt as _,
        TTMLProcessorError,
        TTMLResultExt as _,
        TimestampExt as _,
    },
    model::{
        BackgroundVocal,
        LyricLine,
    },
    parser::{
        ext::{
            BytesStartExt as _,
            QNameExt as _,
        },
        span::process_span,
        state::ParserContext,
        timestamp::parse_timestamp,
        utils::{
            Walk,
            is_spacing_text,
            mark_last_syllable_space,
            resolve_xml_entity,
            walk_inner,
        },
    },
    utils::{
        build_full_text,
        normalize_line_text,
        normalize_words_spaces,
    },
};

/// 解析 `<body>` 标签，其中包含多个歌词区块
pub fn parse_body(
    reader: &mut Reader<&[u8]>,
    body_event: &BytesStart,
    context: &mut ParserContext,
    lines: &mut Vec<LyricLine>,
) -> Result<()> {
    if lines.capacity() < 64 {
        lines.reserve(64 - lines.len());
    }

    // 音频时长；属性缺失或格式无法识别时按缺失处理
    if let Some(dur) = body_event.get_attr_value(attrs::DUR, reader, context)?
        && let Some(ms) = parse_timestamp(dur.trim().as_bytes())
    {
        context.metadata.audio_mut().duration = Some(ms);
    }

    walk_inner(reader, context, tags::BODY, |reader, context, event| {
        if let Event::Start(e) = event
            && e.name().is(tags::DIV)
        {
            parse_section(reader, &e, context, lines)?;
            return Ok(Walk::Handled);
        }
        Ok(Walk::Continue)
    })
}

/// 解析歌词区块，即一个 `<div>` 标签
///
/// ## 示例
/// ```xml
/// <div begin="10.522" end="43.125" itunes:songPart="Verse">
///     <p begin="10.522" end="13.518" itunes:key="L3" ttm:agent="v1">
///         ...
///     </p>
///     <p begin="12.791" end="19.911" itunes:key="L4" ttm:agent="v1">
///         ...
///     </p>
///     <p begin="19.257" end="24.640" itunes:key="L5" ttm:agent="v1">
///         ...
///     </p>
///     <p begin="24.262" end="29.397" itunes:key="L6" ttm:agent="v1">
///         ...
///     </p>
///     <p begin="28.487" end="34.116" itunes:key="L7" ttm:agent="v1">
///         ...
///     </p>
///     <p begin="32.977" end="38.719" itunes:key="L8" ttm:agent="v1">
///        ...
///     </p>
///     <p begin="37.894" end="43.125" itunes:key="L9" ttm:agent="v1">
///         ...
///     </p>
/// </div>
/// ```
fn parse_section(
    reader: &mut Reader<&[u8]>,
    div_event: &BytesStart,
    context: &mut ParserContext,
    lines: &mut Vec<LyricLine>,
) -> Result<()> {
    // 当前 div 的 songPart，如 "Verse", "Chorus"
    let mut current_song_part =
        div_event.get_attr_value(attrs::ITUNES_SONGPART, reader, context)?;
    if current_song_part.is_none() {
        current_song_part =
            div_event.get_attr_value(attrs::ITUNES_SONGPART_KEBAB, reader, context)?;
    }

    // 维护 block_index
    context.current_block_index += 1;
    if context.last_song_part != current_song_part {
        context.last_song_part = current_song_part;
    }

    walk_inner(reader, context, tags::DIV, |reader, context, event| {
        if let Event::Start(e) = event
            && e.name().is(tags::P)
        {
            if let Some(line) = parse_line(reader, &e, context)? {
                lines.push(line);
            }
            return Ok(Walk::Handled);
        }
        Ok(Walk::Continue)
    })
}

/// 解析一行歌词，即一个 `<p>` 标签
///
/// ## 示例
/// ```xml
/// <p begin="10.522" end="13.518" itunes:key="L3" ttm:agent="v1">
///     <span begin="10.522" end="10.716">It</span>
///     <span begin="10.716" end="10.908">was</span>
///     <span begin="10.908" end="11.193">my</span>
///     <span begin="11.193" end="11.759">wedding</span>
///     <span begin="11.759" end="12.645">day</span>
///     <span ttm:role="x-bg">
///         <span begin="11.724" end="11.894">(It</span>
///         <span begin="11.894" end="12.059">was</span>
///         <span begin="12.059" end="12.314">our</span>
///         <span begin="12.314" end="12.851">wedding</span>
///         <span begin="12.851" end="13.518">day)</span>
///     </span>
/// </p>
/// ```
fn parse_line(
    reader: &mut Reader<&[u8]>,
    p_event: &BytesStart,
    context: &mut ParserContext,
) -> Result<Option<LyricLine>> {
    let mut line_id: Option<CompactString> = None;
    let mut agent_id: Option<CompactString> = None;
    let mut start_time = None;
    let mut end_time = None;

    p_event.for_each_attr(reader, context, tags::P, |attr| {
        match attr.key.as_ref() {
            attrs::b::ITUNES_KEY => {
                let val = attr
                    .unescape_value()
                    .map_err(|e| ParseErrorKind::EntityError(e.to_string().into()))
                    .with_attr_context(reader, context, attrs::ITUNES_KEY)
                    .with_offending_bytes(&attr.value)?;
                line_id = Some(CompactString::from(&*val));
            }
            attrs::b::TTM_AGENT => {
                let val = attr
                    .unescape_value()
                    .map_err(|e| ParseErrorKind::EntityError(e.to_string().into()))
                    .with_attr_context(reader, context, attrs::TTM_AGENT)
                    .with_offending_bytes(&attr.value)?;
                agent_id = Some(CompactString::from(&*val));
            }
            attrs::b::BEGIN => {
                start_time = Some(
                    parse_timestamp(&attr.value)
                        .context_invalid_timestamp(attr.value.as_ref())
                        .with_attr_context(reader, context, attrs::BEGIN)
                        .with_offending_bytes(&attr.value)?,
                );
            }
            attrs::b::END => {
                end_time = Some(
                    parse_timestamp(&attr.value)
                        .context_invalid_timestamp(attr.value.as_ref())
                        .with_attr_context(reader, context, attrs::END)
                        .with_offending_bytes(&attr.value)?,
                );
            }
            _ => {}
        }
        Ok(())
    })?;

    let start_time = start_time
        .ok_or_else(|| ParseErrorKind::MissingAttribute(CompactString::const_new(attrs::BEGIN)))
        .with_context(reader, context)?;
    let end_time = end_time
        .ok_or_else(|| ParseErrorKind::MissingAttribute(CompactString::const_new(attrs::END)))
        .with_context(reader, context)?;

    context.current_line_id.clone_from(&line_id);

    let mut line = LyricLine {
        id: line_id.clone(),
        agent_id,
        song_part: context.last_song_part.clone(),
        block_index: Some(context.current_block_index),
        start_time,
        end_time,
        ..Default::default()
    };

    // 提取之前从 `<iTunesMetadata>` 解析出来的外挂翻译和音译并放入主歌词行中
    if let Some(id) = &line.id {
        if !context.translations_map.is_empty() {
            line.translations = context.translations_map.remove(id);
        }
        if !context.romanizations_map.is_empty() {
            line.romanizations = context.romanizations_map.remove(id);
        }

        let bg_trans = if context.bg_translations_map.is_empty() {
            None
        } else {
            context.bg_translations_map.remove(id)
        };
        let bg_rom = if context.bg_romanizations_map.is_empty() {
            None
        } else {
            context.bg_romanizations_map.remove(id)
        };

        if bg_trans.is_some() || bg_rom.is_some() {
            line.background_vocal = Some(BackgroundVocal {
                translations: bg_trans,
                romanizations: bg_rom,
                ..Default::default()
            });
        }
    }

    let mut raw_text = String::new();

    // 解析内部的 span
    walk_inner(
        reader,
        context,
        tags::P,
        |reader, context, event| match event {
            Event::Start(e) if e.name().is(tags::SPAN) => {
                process_span(reader, context, &e, &mut line, false)?;
                Ok(Walk::Handled)
            }
            Event::Text(e) => {
                let text =
                    std::str::from_utf8(e.as_ref()).map_err(TTMLProcessorError::Utf8Error)?;
                if is_spacing_text(text) {
                    mark_last_syllable_space(&mut line, false);
                }
                raw_text.push_str(text);
                Ok(Walk::Continue)
            }
            Event::GeneralRef(reference) => {
                raw_text.push_str(
                    &resolve_xml_entity(&reference)
                        .with_context(reader, context)
                        .with_offending_bytes(reference.as_ref())?,
                );
                Ok(Walk::Continue)
            }
            _ => Ok(Walk::Continue),
        },
    )?;

    post_process_line(&mut line, raw_text);

    context.current_line_id = None;

    Ok(Some(line))
}

/// 后处理一行歌词，包括：
/// * 移除整行歌词的尾随空格
/// * 从逐字歌词拼接逐行文本
/// * 规范化歌词音节、翻译、音译中的空格
/// * 移除背景人声中的括号
fn post_process_line(line: &mut LyricLine, raw_line_text: String) {
    // 移除部分歌词可能出现的尾随空格
    if let Some(words) = &mut line.words
        && let Some(last) = words.last_mut()
    {
        last.ends_with_space = None;
    }
    if let Some(bg) = &mut line.background_vocal
        && let Some(bg_words) = &mut bg.words
        && let Some(last) = bg_words.last_mut()
    {
        last.ends_with_space = None;
    }

    // 规范化主歌词的内容
    match &mut line.words {
        Some(words) if !words.is_empty() => {
            normalize_words_spaces(words);
            line.text = build_full_text(words, false);
        }
        _ => {
            line.words = None;
            line.text = raw_line_text;
            normalize_line_text(&mut line.text);
        }
    }

    // 规范化主歌词的翻译
    if let Some(translations) = &mut line.translations {
        for t in translations {
            // 对于主歌词，我们相信歌词制作者已经正确添加了空格，不在这里乱加空格
            t.normalize(false);
        }
    }

    // 规范化主歌词的音译
    if let Some(romanizations) = &mut line.romanizations {
        for r in romanizations {
            // 对于逐字音译，始终使用空格连接，因为主要来源之一的 AMLL TTML Tool
            // 并不会在逐字音译之间添加空格
            r.normalize(true);
        }
    }

    // 后处理背景人声的括号、空格与文本拼接
    if let Some(bg) = &mut line.background_vocal {
        bg.normalize();
    }
}

#[cfg(test)]
mod tests {
    use quick_xml::{
        Reader,
        events::Event,
    };

    use super::*;
    use crate::model::{
        BackgroundVocal,
        SubLyricContent,
        Syllable,
    };

    fn get_start_event<'a>(reader: &mut Reader<&'a [u8]>, tag: &str) -> BytesStart<'a> {
        let mut buf = Vec::new();
        loop {
            match reader
                .read_event_into(&mut buf)
                .expect("Failed to read XML")
            {
                Event::Start(e) if e.name().as_ref() == tag.as_bytes() => return e.into_owned(),
                Event::Eof => panic!("Reached EOF before finding start tag"),
                _ => (),
            }
        }
    }

    #[test]
    fn test_post_process_line() {
        let mut line = LyricLine {
            start_time: 1000,
            end_time: 2000,
            background_vocal: Some(BackgroundVocal {
                words: Some(vec![
                    Syllable {
                        text: "(Ah".into(),
                        ends_with_space: Some(true),
                        ..Default::default()
                    },
                    Syllable {
                        text: "ha)".into(),
                        ..Default::default()
                    },
                ]),
                ..Default::default()
            }),
            ..Default::default()
        };

        post_process_line(&mut line, "  raw   line   text  ".into());

        assert_eq!(line.text, "raw line text");
        assert_eq!(line.start_time, 1000);
        assert_eq!(line.end_time, 2000);

        let bg = line.background_vocal.as_ref().unwrap();
        assert_eq!(bg.text, "Ah ha");
        let bg_words = bg.words.as_ref().unwrap();
        assert_eq!(bg_words[0].text, "Ah");
        assert_eq!(bg_words[1].text, "ha");
    }

    #[test]
    fn test_parse_line() {
        let xml = r#"<p begin="10.522" end="13.518" itunes:key="L3" ttm:agent="v1">
            <span begin="10.522" end="11.000">Hello</span>
        </p>"#;
        let mut reader = Reader::from_str(xml);
        let start_event = get_start_event(&mut reader, "p");

        let mut context = ParserContext::default();

        context.translations_map.insert(
            "L3".into(),
            vec![SubLyricContent {
                language: Some("zh".into()),
                automatically_created: None,
                text: "你好".into(),
                words: None,
            }],
        );
        context.last_song_part = Some("Verse".into());
        context.current_block_index = 1;

        let line = parse_line(&mut reader, &start_event, &mut context)
            .expect("Failed to parse line")
            .unwrap();

        assert_eq!(line.id.as_deref(), Some("L3"));
        assert_eq!(line.agent_id.as_deref(), Some("v1"));
        assert_eq!(line.song_part.as_deref(), Some("Verse"));
        assert_eq!(line.block_index, Some(1));

        assert_eq!(line.start_time, 10522);
        assert_eq!(line.end_time, 13518);

        assert_eq!(line.text, "Hello");
        assert_eq!(line.words.unwrap().len(), 1);

        let translations = line.translations.unwrap();
        assert_eq!(translations.len(), 1);
        assert_eq!(translations[0].text, "你好");
        assert!(context.translations_map.is_empty());
    }

    #[test]
    fn test_parse_section() {
        let xml = r#"<div itunes:songPart="Chorus">
            <p begin="0.0" end="1.0" itunes:key="L1">Line 1</p>
            <p begin="1.0" end="2.0" itunes:key="L2">Line 2</p>
        </div>"#;
        let mut reader = Reader::from_str(xml);
        let start_event = get_start_event(&mut reader, "div");

        let mut context = ParserContext::default();
        let mut lines = Vec::new();

        parse_section(&mut reader, &start_event, &mut context, &mut lines)
            .expect("Failed to parse section");

        assert_eq!(context.last_song_part.as_deref(), Some("Chorus"));
        assert_eq!(context.current_block_index, 1);

        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].id.as_deref(), Some("L1"));
        assert_eq!(lines[0].song_part.as_deref(), Some("Chorus"));
        assert_eq!(lines[0].block_index, Some(1));
        assert_eq!(lines[1].id.as_deref(), Some("L2"));
        assert_eq!(lines[1].song_part.as_deref(), Some("Chorus"));
        assert_eq!(lines[1].block_index, Some(1));
    }

    #[test]
    fn test_parse_body() {
        let xml = r#"<body dur="3:36.120">
            <div itunes:songPart="Verse">
                <p begin="0.0" end="1.0">V1</p>
            </div>
            <div itunes:songPart="Chorus">
                <p begin="1.0" end="2.0">C1</p>
            </div>
        </body>"#;
        let mut reader = Reader::from_str(xml);
        let body_event = get_start_event(&mut reader, "body");

        let mut context = ParserContext::default();
        let mut lines = Vec::new();

        parse_body(&mut reader, &body_event, &mut context, &mut lines)
            .expect("Failed to parse body");

        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].song_part.as_deref(), Some("Verse"));
        assert_eq!(lines[0].block_index, Some(1));
        assert_eq!(lines[1].song_part.as_deref(), Some("Chorus"));
        assert_eq!(lines[1].block_index, Some(2));
        assert_eq!(context.current_block_index, 2);
        assert_eq!(
            context.metadata.audio.as_ref().unwrap().duration,
            Some(216_120)
        );
    }
}
