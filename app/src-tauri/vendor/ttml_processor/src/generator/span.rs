use quick_xml::{
    Writer,
    events::{
        BytesText,
        Event,
    },
};

use crate::{
    constants::{
        attrs,
        tags,
        vals,
    },
    error::Result,
    generator::{
        GeneratorConfig,
        ext::ElementWriterExt as _,
        sub_lyric::should_write_inline,
        utils::{
            format_timestamp,
            line_text,
        },
    },
    model::{
        BackgroundVocal,
        LyricLine,
        RubyTag,
        SubLyricContent,
        Syllable,
    },
};

/// 写入一行歌词内的所有 span（音节、背景人声等）
pub fn write_line_spans(
    writer: &mut Writer<Vec<u8>>,
    line: &LyricLine,
    should_write_inline_subline: bool,
    config: &GeneratorConfig,
) -> Result<()> {
    // 逐行模式下不输出背景人声
    let background_vocal = line
        .background_vocal
        .as_ref()
        .filter(|_| !config.line_timing);

    // Apple Music 特有，背景人声先开始就先在主歌词之前写入
    let should_write_bg_first = config.use_apple_format_rules
        && background_vocal
            .zip(line.words.as_ref().and_then(|w| w.first()))
            .is_some_and(|(bg, first_word)| bg.start_time < first_word.start_time);

    if should_write_bg_first && let Some(bg) = background_vocal {
        write_bg_vocal(writer, bg, should_write_inline_subline, config)?;
    }

    // 写入主歌词音节或逐行文本
    let words = line.words.as_deref().filter(|w| !w.is_empty());
    if let Some(words) = words.filter(|_| !config.line_timing) {
        for syllable in words {
            let trailing_space = syllable.ends_with_space.unwrap_or_default();
            // 格式化输出时将尾随空格写入 span 内容以便保留空格
            //
            // 写在后面的话，格式化输出时 quick-xml 也会把一堆带空格的
            // span 写在一行以保留这些空格，不美观
            let space_in_span = config.format && trailing_space && syllable.ruby.is_none();
            let suffix = if space_in_span { Some(" ") } else { None };
            write_syllable(writer, syllable, None, suffix)?;

            if trailing_space && !space_in_span {
                writer.write_event(Event::Text(BytesText::new(" ")))?;
            }
        }
    } else {
        // 逐行歌词，直接将整行文本作为纯文本写入
        let text = line_text(&line.text, words, false);
        if !text.is_empty() {
            writer.write_event(Event::Text(BytesText::new(&text)))?;
        }
    }

    // 写入内嵌翻译/音译
    if should_write_inline_subline {
        write_inline_subline(
            writer,
            line.translations.as_deref(),
            line.romanizations.as_deref(),
            config,
        )?;
    }

    if !should_write_bg_first && let Some(bg) = background_vocal {
        write_bg_vocal(writer, bg, should_write_inline_subline, config)?;
    }

    Ok(())
}

/// 写入单个音节 span
fn write_syllable(
    writer: &mut Writer<Vec<u8>>,
    syllable: &Syllable,
    prefix: Option<&str>,
    suffix: Option<&str>,
) -> Result<()> {
    // Ruby 容器
    if let Some(ruby_tags) = &syllable.ruby {
        return write_ruby_container(writer, syllable, ruby_tags);
    }

    // 普通音节
    let begin = format_timestamp(syllable.start_time);
    let end = format_timestamp(syllable.end_time);

    let mut span = writer
        .create_element(tags::SPAN)
        .with_attribute((attrs::BEGIN, begin.as_str()))
        .with_attribute((attrs::END, end.as_str()));

    if syllable.obscene == Some(true) {
        span = span.with_attribute((attrs::AMLL_OBSCENE, vals::TRUE_STR));
    }
    if let Some(beat) = syllable.empty_beat {
        let beat_str = beat.to_string();
        span = span.with_attribute((attrs::AMLL_EMPTY_BEAT, beat_str.as_str()));
    }

    span.write_inner_content(|writer| {
        if let Some(p) = prefix {
            writer.write_event(Event::Text(BytesText::new(p)))?;
        }
        writer.write_event(Event::Text(BytesText::new(&syllable.text)))?;
        if let Some(s) = suffix {
            writer.write_event(Event::Text(BytesText::new(s)))?;
        }
        Ok(())
    })?;

    Ok(())
}

/// 写入 Ruby 注音容器
fn write_ruby_container(
    writer: &mut Writer<Vec<u8>>,
    syllable: &Syllable,
    ruby_tags: &[RubyTag],
) -> Result<()> {
    let mut container = writer
        .create_element(tags::SPAN)
        .with_attribute((attrs::TTS_RUBY, vals::RUBY_CONTAINER));

    if syllable.obscene == Some(true) {
        container = container.with_attribute((attrs::AMLL_OBSCENE, vals::TRUE_STR));
    }
    if let Some(beat) = syllable.empty_beat {
        let beat_str = beat.to_string();
        container = container.with_attribute((attrs::AMLL_EMPTY_BEAT, beat_str.as_str()));
    }

    container.write_inner_content(|writer| {
        // 写入 base 文本
        writer
            .create_element(tags::SPAN)
            .with_attributes([(attrs::TTS_RUBY, vals::RUBY_BASE)])
            .write_text_content(BytesText::new(&syllable.text))?;

        // 写入 textContainer
        writer
            .create_element(tags::SPAN)
            .with_attribute((attrs::TTS_RUBY, vals::RUBY_TEXT_CONTAINER))
            .write_inner_content(|writer| {
                for ruby_tag in ruby_tags {
                    let r_begin = format_timestamp(ruby_tag.start_time);
                    let r_end = format_timestamp(ruby_tag.end_time);

                    writer
                        .create_element(tags::SPAN)
                        .with_attributes([
                            (attrs::TTS_RUBY, vals::RUBY_TEXT),
                            (attrs::BEGIN, r_begin.as_str()),
                            (attrs::END, r_end.as_str()),
                        ])
                        .write_text_content(BytesText::new(&ruby_tag.text))?;
                }
                Ok(())
            })?;

        Ok(())
    })?;

    Ok(())
}

/// 写入背景人声 span
fn write_bg_vocal(
    writer: &mut Writer<Vec<u8>>,
    bg: &BackgroundVocal,
    should_write_inline_subline: bool,
    config: &GeneratorConfig,
) -> Result<()> {
    let mut bg_span = writer
        .create_element(tags::SPAN)
        .with_attribute((attrs::TTM_ROLE, vals::ROLE_BG));

    // Apple Music 特有，不在背景歌词容器上写入 begin 和 end 属性
    if !config.use_apple_format_rules {
        let bg_begin = format_timestamp(bg.start_time);
        let bg_end = format_timestamp(bg.end_time);
        bg_span = bg_span
            .with_attribute((attrs::BEGIN, bg_begin.as_str()))
            .with_attribute((attrs::END, bg_end.as_str()));
    }

    bg_span.write_inner_content(|writer| {
        // 写入背景人声的音节
        if let Some(words) = &bg.words.as_ref().filter(|w| !w.is_empty()) {
            let len = words.len();
            for (i, syllable) in words.iter().enumerate() {
                let is_first = i == 0;
                let is_last = i == len - 1;
                let has_ruby = syllable.ruby.is_some();
                let trailing_space = syllable.ends_with_space.unwrap_or_default();

                if has_ruby {
                    // 有 ruby 时将括号写在 container 外部
                    if is_first {
                        writer.write_event(Event::Text(BytesText::new("(")))?;
                    }

                    write_syllable(writer, syllable, None, None)?;

                    if is_last {
                        writer.write_event(Event::Text(BytesText::new(")")))?;
                    }

                    // 对于 ruby 容器暂时把空格始终写到容器后
                    if trailing_space {
                        writer.write_event(Event::Text(BytesText::new(" ")))?;
                    }
                } else {
                    // 没有 Ruby 注音就把括号写在 span 内
                    let base_suffix = if is_last { ")" } else { "" };
                    let space_in_span = config.format && trailing_space;
                    let suffix_buf = if space_in_span {
                        format!("{base_suffix} ")
                    } else {
                        String::new()
                    };
                    let suffix: Option<&str> = if space_in_span {
                        Some(&suffix_buf)
                    } else if !base_suffix.is_empty() {
                        Some(base_suffix)
                    } else {
                        None
                    };
                    let prefix = if is_first { Some("(") } else { None };

                    write_syllable(writer, syllable, prefix, suffix)?;

                    if trailing_space && !space_in_span {
                        writer.write_event(Event::Text(BytesText::new(" ")))?;
                    }
                }
            }
        } else if !bg.text.is_empty() {
            // 逐行歌词，用括号包裹文本
            writer.write_event(Event::Text(BytesText::new("(")))?;
            writer.write_event(Event::Text(BytesText::new(&bg.text)))?;
            writer.write_event(Event::Text(BytesText::new(")")))?;
        }

        // 写入内嵌翻译/音译
        // AMLL 的内嵌翻译和音译无括号
        if should_write_inline_subline {
            write_inline_subline(
                writer,
                bg.translations.as_deref(),
                bg.romanizations.as_deref(),
                config,
            )?;
        }

        Ok(())
    })?;

    Ok(())
}

fn write_inline_subline(
    writer: &mut Writer<Vec<u8>>,
    translations: Option<&[SubLyricContent]>,
    romanizations: Option<&[SubLyricContent]>,
    config: &GeneratorConfig,
) -> Result<()> {
    if config.use_apple_format_rules {
        return Ok(());
    }

    let mut write_items =
        |items: Option<&[SubLyricContent]>, role: &str, space_joined: bool| -> Result<()> {
            if let Some(contents) = items
                && should_write_inline(contents, config)
            {
                for item in contents {
                    // 逐行模式下逐字内容也降级为逐行文本
                    let text = line_text(&item.text, item.words.as_deref(), space_joined);
                    writer
                        .create_element(tags::SPAN)
                        .with_attribute((attrs::TTM_ROLE, role))
                        .with_attribute_opt((attrs::XML_LANG, item.language.as_deref()))
                        .write_text_content(BytesText::new(&text))?;
                }
            }
            Ok(())
        };

    write_items(translations, vals::ROLE_TRANS, false)?;
    write_items(romanizations, vals::ROLE_ROM, true)?;

    Ok(())
}
