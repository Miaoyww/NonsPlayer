//! 用于解析 `<iTunesMetadata>` 中的翻译或音译内容的模块

use compact_str::CompactString;
use quick_xml::{
    Reader,
    events::Event,
};

use crate::{
    constants::{
        attrs,
        tags,
        vals,
    },
    error::{
        Result,
        ResultExt as _,
        TTMLProcessorError,
        TTMLResultExt as _,
    },
    model::{
        SubLyricContent,
        Syllable,
    },
    parser::{
        ext::{
            BytesStartExt as _,
            QNameExt as _,
        },
        state::ParserContext,
        utils::{
            Walk,
            is_spacing_text,
            mark_slice_last_space,
            parse_basic_syllable,
            resolve_xml_entity,
            walk_inner,
        },
    },
    utils::{
        build_full_text,
        normalize_words_spaces,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubLyricType {
    Translation,
    Transliteration,
}

/// 解析 `automaticallyCreated` 属性
///
/// 仅 `true` / `1` 视为自动生成；`false` / `0` 与其他无法识别的取值都与属性缺失等价
fn parse_automatically_created(value: Option<&CompactString>) -> Option<bool> {
    let value = value?;
    matches!(value.as_str(), "true" | "1").then_some(true)
}

/// 解析 `<iTunesMetadata>` 中的翻译或音译内容
///
/// ## 示例
/// ```xml
/// <translations>
///     <translation type="subtitle" xml:lang="zh-Hans">
///         ...
///     </translation>
/// </translations>
/// ```
///
/// ```xml
/// <translations>
///     <translation type="replacement" xml:lang="zh-Hans">
///         ...
///     </translation>
/// </translations>
/// ```
///
/// ```xml
/// <transliterations>
///     <transliteration xml:lang="zh-Latn-pinyin">
///         ...
///     </transliteration>
/// </transliterations>
/// ```
pub fn parse_sub_lyrics(
    reader: &mut Reader<&[u8]>,
    context: &mut ParserContext,
    group_tag: &str,
    item_tag: &str,
    sub_type: SubLyricType,
) -> Result<()> {
    walk_inner(reader, context, group_tag, |reader, context, event| {
        if let Event::Start(e) = event
            && e.name().is(item_tag)
        {
            let lang = e.get_attr_value(attrs::XML_LANG, reader, context)?;
            let automatically_created = parse_automatically_created(
                e.get_attr_value(attrs::AUTOMATICALLY_CREATED, reader, context)?
                    .as_ref(),
            );
            parse_sub_lyric_block(
                reader,
                context,
                item_tag,
                lang.as_ref(),
                automatically_created,
                sub_type,
            )?;
            return Ok(Walk::Handled);
        }
        Ok(Walk::Continue)
    })
}

/// 解析一个 `<text>` 标签
///
/// ## 示例
/// ```xml
/// <text for="L20">...</text>
/// ```
fn parse_sub_lyric_block(
    reader: &mut Reader<&[u8]>,
    context: &mut ParserContext,
    item_tag: &str,
    lang: Option<&CompactString>,
    automatically_created: Option<bool>,
    sub_type: SubLyricType,
) -> Result<()> {
    walk_inner(reader, context, item_tag, |reader, context, event| {
        if let Event::Start(e) = event
            && e.name().is(tags::TEXT)
        {
            let line_id = e.get_required_attr(attrs::FOR, reader, context)?;
            parse_sub_lyric_text(
                reader,
                context,
                line_id,
                lang,
                automatically_created,
                sub_type,
            )?;
            return Ok(Walk::Handled);
        }
        Ok(Walk::Continue)
    })
}

/// 解析 `<text>` 标签中的内容
///
/// ## 示例
/// ```xml
/// <text for="L21">
///     没错 他潜入你梦境 乐见你尖叫
///     <span xmlns:ttm="http://www.w3.org/ns/ttml#metadata" ttm:role="x-bg" xmlns="http://www.w3.org/ns/ttml">
///         (嘿)
///     </span>
/// </text>
/// ```
///
/// ```xml
/// <text for="L1">
///     <span begin="27.549" end="28.088">窗</span>
///     <span begin="28.088" end="29.033">外</span>
///     <span begin="29.033" end="29.350">的</span>
///     <span begin="29.350" end="29.647">麻</span>
///     <span begin="29.647" end="30.419">雀</span>
///     <span begin="31.009" end="31.512">在</span>
///     <span begin="31.512" end="32.712">电线杆</span>
///     <span begin="32.712" end="33.023">上</span>
///     <span begin="33.023" end="33.291">多</span>
///     <span begin="33.291" end="33.956">嘴</span>
/// </text>
/// ```
///
/// ```xml
/// <text for="L10">
///     <span begin="54.500" end="55.060">rèn</span>
///     <span begin="55.060" end="55.400">yán</span>
///     <span begin="55.400" end="56.550">yǔ</span>
///     <span begin="57.310" end="57.560">huǎn</span>
///     <span begin="57.560" end="57.910">huǎn</span>
///     <span begin="57.910" end="58.580">rù</span>
///     <span begin="58.580" end="59.580">mián</span>
///     <span ttm:role="x-bg">
///         <span begin="58.560" end="58.890">(huǎn</span>
///         <span begin="58.890" end="59.110">huǎn</span>
///         <span begin="59.110" end="59.490">rù</span>
///         <span begin="59.490" end="1:00.440">mián)</span>
///     </span>
/// </text>
/// ```
fn parse_sub_lyric_text(
    reader: &mut Reader<&[u8]>,
    context: &mut ParserContext,
    line_id: CompactString,
    lang: Option<&CompactString>,
    automatically_created: Option<bool>,
    sub_type: SubLyricType,
) -> Result<()> {
    context.current_line_id = Some(line_id.clone());

    let mut main_words: Vec<Syllable> = Vec::new();
    let mut bg_words: Vec<Syllable> = Vec::new();

    let mut raw_main_text = String::new();
    let mut raw_bg_text = String::new();

    let mut in_bg_span = false;

    walk_inner(
        reader,
        context,
        tags::TEXT,
        |reader, context, event| match event {
            Event::Start(e) if e.name().is(tags::SPAN) => {
                let mut is_current_bg = false;
                for attr_res in e.attributes().with_checks(false).flatten() {
                    if attr_res.key.as_ref() == attrs::b::TTM_ROLE {
                        is_current_bg = attr_res.value.as_ref() == vals::b::ROLE_BG;
                        break;
                    }
                }

                if is_current_bg {
                    in_bg_span = true;
                    Ok(Walk::Continue)
                } else {
                    let syllable = parse_basic_syllable(reader, context, &e)?;

                    if in_bg_span {
                        bg_words.push(syllable);
                    } else {
                        main_words.push(syllable);
                    }
                    Ok(Walk::Handled)
                }
            }
            Event::End(e) if e.name().is(tags::SPAN) => {
                in_bg_span = false;
                Ok(Walk::Continue)
            }
            Event::Text(e) => {
                let text_str =
                    std::str::from_utf8(e.as_ref()).map_err(TTMLProcessorError::Utf8Error)?;
                if is_spacing_text(text_str) {
                    let target_words = if in_bg_span {
                        &mut bg_words
                    } else {
                        &mut main_words
                    };
                    mark_slice_last_space(target_words);
                }

                if in_bg_span {
                    raw_bg_text.push_str(text_str);
                } else {
                    raw_main_text.push_str(text_str);
                }
                Ok(Walk::Continue)
            }
            Event::GeneralRef(reference) => {
                let ch_str = resolve_xml_entity(&reference)
                    .with_context(reader, context)
                    .with_offending_bytes(reference.as_ref())?;
                if in_bg_span {
                    raw_bg_text.push_str(&ch_str);
                } else {
                    raw_main_text.push_str(&ch_str);
                }
                Ok(Walk::Continue)
            }
            _ => Ok(Walk::Continue),
        },
    )?;

    context.current_line_id = None;

    // 移除部分歌词可能出现的尾随空格
    if let Some(last) = main_words.last_mut() {
        last.ends_with_space = None;
    }
    if let Some(last) = bg_words.last_mut() {
        last.ends_with_space = None;
    }

    let build_content = |mut words: Vec<Syllable>, raw_text: String| -> Option<SubLyricContent> {
        if words.is_empty() {
            let text = raw_text.trim().to_string();
            if text.is_empty() {
                None
            } else {
                Some(SubLyricContent {
                    language: lang.cloned(),
                    automatically_created,
                    text,
                    words: None,
                })
            }
        } else {
            normalize_words_spaces(&mut words);
            let text = build_full_text(&words, false);
            Some(SubLyricContent {
                language: lang.cloned(),
                automatically_created,
                text,
                words: Some(words),
            })
        }
    };

    if let Some(main_content) = build_content(main_words, raw_main_text) {
        let map = match sub_type {
            SubLyricType::Translation => &mut context.translations_map,
            SubLyricType::Transliteration => &mut context.romanizations_map,
        };
        map.entry(line_id.clone()).or_default().push(main_content);
    }

    if let Some(bg_content) = build_content(bg_words, raw_bg_text) {
        let bg_map = match sub_type {
            SubLyricType::Translation => &mut context.bg_translations_map,
            SubLyricType::Transliteration => &mut context.bg_romanizations_map,
        };
        bg_map.entry(line_id).or_default().push(bg_content);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use quick_xml::events::Event;

    use super::*;
    use crate::parser::state::ParserContext;

    fn advance_to_start_tag(reader: &mut Reader<&[u8]>, tag_name: &str) {
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) if e.name().is(tag_name) => break,
                Ok(Event::Eof) => panic!("Reached EOF before finding tag: {tag_name}"),
                _ => (),
            }
            buf.clear();
        }
    }

    #[test]
    fn test_parse_automatically_created() {
        let value = |s: &str| CompactString::from(s);

        assert_eq!(
            parse_automatically_created(Some(&value("true"))),
            Some(true)
        );
        assert_eq!(parse_automatically_created(Some(&value("1"))), Some(true));
        assert_eq!(parse_automatically_created(Some(&value("false"))), None);
        assert_eq!(parse_automatically_created(Some(&value("0"))), None);
        assert_eq!(parse_automatically_created(Some(&value("auto"))), None);
        assert_eq!(parse_automatically_created(None), None);
    }

    #[test]
    fn test_parse_sub_lyric_text() {
        let xml = r#"
        <text for="L1">
            <span begin="0.000" end="1.000">测</span>
            <span begin="1.000" end="2.000">试</span>
            <span ttm:role="x-bg">
                <span begin="1.000" end="2.000">(背景)</span>
            </span>
        </text>"#;

        let mut reader = Reader::from_str(xml);
        let mut context = ParserContext::default();
        let lang = "zh-Hans".into();

        advance_to_start_tag(&mut reader, "text");

        parse_sub_lyric_text(
            &mut reader,
            &mut context,
            "L1".into(),
            Some(&lang),
            None,
            SubLyricType::Translation,
        )
        .expect("Failed to parse sub lyric text");

        assert!(context.translations_map.contains_key("L1"));
        let main_content = &context.translations_map["L1"][0];
        assert_eq!(main_content.language.as_deref(), Some("zh-Hans"));
        assert_eq!(main_content.text, "测试");
        assert_eq!(main_content.words.as_ref().unwrap().len(), 2);
        assert_eq!(main_content.words.as_ref().unwrap()[0].text, "测");

        assert!(context.bg_translations_map.contains_key("L1"));
        let bg_content = &context.bg_translations_map["L1"][0];
        assert_eq!(bg_content.text, "(背景)");
        assert_eq!(bg_content.words.as_ref().unwrap().len(), 1);
        assert_eq!(bg_content.words.as_ref().unwrap()[0].text, "(背景)");
    }

    #[test]
    fn test_parse_sub_lyric_block() {
        let xml = r#"
        <translation xml:lang="zh-Hans">
            <text for="L10">
                <span begin="0.0" end="1.0">第</span>
                <span begin="1.0" end="2.0">十</span>
                <span begin="2.0" end="3.0">行</span>
            </text>
            <text for="L11">只有纯文本翻译</text>
        </translation>"#;

        let mut reader = Reader::from_str(xml);
        let mut context = ParserContext::default();
        let lang = "zh-Hans".into();

        advance_to_start_tag(&mut reader, "translation");

        parse_sub_lyric_block(
            &mut reader,
            &mut context,
            "translation",
            Some(&lang),
            None,
            SubLyricType::Translation,
        )
        .expect("Failed to parse sub lyric block");

        assert!(context.translations_map.contains_key("L10"));
        let l10_content = &context.translations_map["L10"][0];
        assert_eq!(l10_content.text, "第十行");
        assert!(l10_content.words.is_some());
        assert_eq!(l10_content.words.as_ref().unwrap().len(), 3);

        assert!(context.translations_map.contains_key("L11"));
        let l11_content = &context.translations_map["L11"][0];
        assert_eq!(l11_content.text, "只有纯文本翻译");
        assert!(l11_content.words.is_none());
    }

    #[test]
    fn test_parse_sub_lyrics() {
        let xml = r#"
        <transliterations>
            <transliteration xml:lang="zh-Latn-pinyin">
                <text for="L1">
                    <span begin="0.0" end="1.0">pīn</span> <span begin="1.0" end="2.0">yīn</span>
                </text>
                <text for="L2">
                    <span begin="1.0" end="2.0">abc</span>
                    <span begin="1.0" end="2.0">def</span>
                </text>
            </transliteration>
        </transliterations>"#;

        let mut reader = Reader::from_str(xml);
        let mut context = ParserContext::default();

        advance_to_start_tag(&mut reader, "transliterations");

        parse_sub_lyrics(
            &mut reader,
            &mut context,
            "transliterations",
            "transliteration",
            SubLyricType::Transliteration,
        )
        .expect("Failed to parse sub lyrics");

        assert!(context.romanizations_map.contains_key("L1"));
        assert!(context.translations_map.is_empty());

        let rom_content1 = &context.romanizations_map["L1"][0];
        assert_eq!(rom_content1.language.as_deref(), Some("zh-Latn-pinyin"));
        assert_eq!(rom_content1.text, "pīn yīn");
        assert_eq!(rom_content1.words.as_ref().unwrap().len(), 2);

        let rom_content2 = &context.romanizations_map["L2"][0];
        assert_eq!(rom_content2.language.as_deref(), Some("zh-Latn-pinyin"));
        assert_eq!(rom_content2.text, "abcdef");
        assert_eq!(rom_content2.words.as_ref().unwrap().len(), 2);
    }
}
