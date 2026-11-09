use std::{
    borrow::Cow,
    collections::BTreeSet,
};

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
        utils::{
            format_timestamp,
            line_text,
        },
    },
    model::{
        LyricLine,
        SubLyricContent,
        Syllable,
    },
};

/// 翻译/音译的类型
#[derive(Clone, Copy)]
pub enum SubLyricKind {
    Translation,
    Transliteration,
}

impl SubLyricKind {
    const fn group_tag(self) -> &'static str {
        match self {
            Self::Translation => tags::TRANSLATIONS,
            Self::Transliteration => tags::TRANSLITERATIONS,
        }
    }

    const fn item_tag(self) -> &'static str {
        match self {
            Self::Translation => tags::TRANSLATION,
            Self::Transliteration => tags::TRANSLITERATION,
        }
    }

    /// 逐字内容拼接为逐行文本时，音节之间是否始终插入空格
    ///
    /// 与解析器 `SubLyricContent::normalize` 的约定保持一致
    const fn space_joined(self) -> bool {
        matches!(self, Self::Transliteration)
    }
}

struct LineSubLyricEntry<'a> {
    line_id: &'a str,
    main_contents: &'a [SubLyricContent],
    bg_contents: Option<&'a [SubLyricContent]>,
}

/// 写入翻译或音译块到 head
pub fn write_sub_lyrics(
    writer: &mut Writer<Vec<u8>>,
    lines: &[LyricLine],
    kind: SubLyricKind,
    config: &GeneratorConfig,
) -> Result<()> {
    let entries: Vec<LineSubLyricEntry<'_>> = lines
        .iter()
        .filter_map(|line| {
            let line_id = line.id.as_deref()?;
            let main_contents = match kind {
                SubLyricKind::Translation => line.translations.as_deref(),
                SubLyricKind::Transliteration => line.romanizations.as_deref(),
            };
            let main_contents =
                main_contents.filter(|contents| should_write_to_head(contents, config));

            // 逐行模式下不输出背景人声，其翻译/音译也一并跳过
            let bg_contents = line
                .background_vocal
                .as_ref()
                .filter(|_| !config.line_timing)
                .and_then(|bg| {
                    let bg_sub = match kind {
                        SubLyricKind::Translation => bg.translations.as_deref(),
                        SubLyricKind::Transliteration => bg.romanizations.as_deref(),
                    };
                    bg_sub.filter(|contents| should_write_to_head(contents, config))
                });

            if main_contents.is_none() && bg_contents.is_none() {
                return None;
            }
            Some(LineSubLyricEntry {
                line_id,
                main_contents: main_contents.unwrap_or_default(),
                bg_contents,
            })
        })
        .collect();

    if entries.is_empty() {
        return Ok(());
    }

    let mut languages: BTreeSet<Option<&str>> = BTreeSet::new();

    for entry in &entries {
        let all_contents = entry
            .main_contents
            .iter()
            .chain(entry.bg_contents.unwrap_or(&[]));

        for content in all_contents {
            languages.insert(content.language.as_deref());
        }
    }

    writer
        .create_element(kind.group_tag())
        .write_inner_content(|writer| {
            for &lang in &languages {
                let mut item = writer.create_element(kind.item_tag());

                if is_automatically_created(&entries, lang) {
                    item = item.with_attribute((attrs::AUTOMATICALLY_CREATED, vals::TRUE_STR));
                }

                if let Some(lang_str) = lang {
                    item = item.with_attribute((attrs::XML_LANG, lang_str));
                }

                // 音译不写入 `type`；翻译则根据逐字/逐行形式推断
                if matches!(kind, SubLyricKind::Translation) {
                    let contents = entries.iter().flat_map(|entry| {
                        entry
                            .main_contents
                            .iter()
                            .chain(entry.bg_contents.unwrap_or(&[]))
                    });
                    item = item.with_attribute((
                        attrs::TYPE,
                        translation_type(
                            contents.filter(|content| content.language.as_deref() == lang),
                            config,
                        ),
                    ));
                }

                item.write_inner_content(|writer| {
                    for entry in &entries {
                        let main_content = entry
                            .main_contents
                            .iter()
                            .find(|c| c.language.as_deref() == lang);

                        let bg_content = entry
                            .bg_contents
                            .and_then(|bgs| bgs.iter().find(|c| c.language.as_deref() == lang));

                        if main_content.is_none() && bg_content.is_none() {
                            continue;
                        }

                        writer
                            .create_element(tags::TEXT)
                            .with_attribute((attrs::FOR, entry.line_id))
                            .write_inner_content(|writer| {
                                if let Some(content) = main_content {
                                    write_sub_lyric_content(
                                        writer,
                                        content,
                                        false,
                                        config,
                                        kind.space_joined(),
                                    )?;
                                }
                                if let Some(bg_content) = bg_content {
                                    writer
                                        .create_element(tags::SPAN)
                                        .with_attribute((attrs::TTM_ROLE, vals::ROLE_BG))
                                        .write_inner_content(|writer| {
                                            write_sub_lyric_content(
                                                writer,
                                                bg_content,
                                                true,
                                                config,
                                                kind.space_joined(),
                                            )?;
                                            Ok(())
                                        })?;
                                }
                                Ok(())
                            })?;
                    }
                    Ok(())
                })?;
            }
            Ok(())
        })?;

    Ok(())
}

/// 判断 `<translation>` / `<transliteration>` 块是否应写入 `automaticallyCreated="true"`
///
/// 解析时同一块内的所有内容共享该标志；手工构造的模型若在同一语言下混用，
/// 只要存在任一 `Some(true)` 即视为自动生成
fn is_automatically_created(entries: &[LineSubLyricEntry<'_>], lang: Option<&str>) -> bool {
    entries.iter().any(|entry| {
        entry
            .main_contents
            .iter()
            .chain(entry.bg_contents.unwrap_or(&[]))
            .any(|content| {
                content.language.as_deref() == lang && content.automatically_created == Some(true)
            })
    })
}

/// 推断 `<translation>` 块应写入的 `type` 属性
///
/// Apple Music 的约定：
/// - 逐行翻译（没有逐字音节，或每行的音节数不超过 1）写入 `subtitle`
/// - 逐字翻译（存在包含多个音节的翻译行）写入 `replacement`
///
/// 启用 `line_timing` 时逐字内容会被降级为逐行文本，因此同样按 `subtitle` 处理
fn translation_type<'a>(
    mut contents: impl Iterator<Item = &'a SubLyricContent>,
    config: &GeneratorConfig,
) -> &'static str {
    let is_word_level = !config.line_timing
        && contents.any(|content| content.words.as_ref().is_some_and(|words| words.len() > 1));

    if is_word_level {
        vals::TRANSLATION_REPLACEMENT
    } else {
        vals::TRANSLATION_SUBTITLE
    }
}

fn write_sub_lyric_content(
    writer: &mut Writer<Vec<u8>>,
    content: &SubLyricContent,
    is_bg: bool,
    config: &GeneratorConfig,
    space_joined: bool,
) -> Result<()> {
    let words = content.words.as_deref();

    // 逐行模式下将逐字内容降级为逐行文本
    if let Some(words) = words.filter(|_| !config.line_timing) {
        write_sub_lyric_syllables(writer, words, is_bg, config.format)?;
        return Ok(());
    }

    let text = line_text(&content.text, words, space_joined);

    if is_bg {
        writer.write_event(Event::Text(BytesText::new("(")))?;
        writer.write_event(Event::Text(BytesText::new(&text)))?;
        writer.write_event(Event::Text(BytesText::new(")")))?;
    } else {
        writer.write_event(Event::Text(BytesText::new(&text)))?;
    }

    Ok(())
}

fn write_sub_lyric_syllables(
    writer: &mut Writer<Vec<u8>>,
    words: &[Syllable],
    is_bg: bool,
    format_xml: bool,
) -> Result<()> {
    for (i, syllable) in words.iter().enumerate() {
        let begin = format_timestamp(syllable.start_time);
        let end = format_timestamp(syllable.end_time);
        let trailing_space = syllable.ends_with_space.unwrap_or_default();

        let base_text = if is_bg {
            if words.len() == 1 {
                Cow::Owned(format!("({})", syllable.text))
            } else if i == 0 {
                Cow::Owned(format!("({}", syllable.text))
            } else if i == words.len() - 1 {
                Cow::Owned(format!("{})", syllable.text))
            } else {
                Cow::Borrowed(syllable.text.as_str())
            }
        } else {
            Cow::Borrowed(syllable.text.as_str())
        };

        let text = if format_xml && trailing_space {
            Cow::Owned(format!("{base_text} "))
        } else {
            base_text
        };

        writer
            .create_element(tags::SPAN)
            .with_attribute((attrs::BEGIN, begin.as_str()))
            .with_attribute((attrs::END, end.as_str()))
            .write_text_content(BytesText::new(&text))?;

        if trailing_space && !format_xml {
            writer.write_event(Event::Text(BytesText::new(" ")))?;
        }
    }
    Ok(())
}

/// 判断是否应该将翻译/音译写入到 `<iTunesMetadata> `中
/// - 逐字翻译：始终写入
/// - 逐行翻译：仅当 `use_apple_format_rules` 为 true 时写入
/// - 启用了 `line_timing`：逐字内容会降级为逐行，因此与逐行翻译同等对待
fn should_write_to_head(contents: &[SubLyricContent], config: &GeneratorConfig) -> bool {
    config.use_apple_format_rules
        || (!config.line_timing && contents.iter().any(|c| c.words.is_some()))
}

/// 判断是否应该将翻译/音译作为内嵌 span 写入到歌词行中
/// - 存在逐字内容：从不内嵌（无论是否有逐行内容）
/// - 仅有逐行内容：内嵌
/// - 启用了 `use_apple_format_rules`：始终不内嵌
/// - 启用了 `line_timing`：逐字内容会降级为逐行，因此也可以内嵌
pub fn should_write_inline(contents: &[SubLyricContent], config: &GeneratorConfig) -> bool {
    if config.use_apple_format_rules {
        return false;
    }
    config.line_timing || !contents.iter().any(|c| c.words.is_some())
}

/// 检查给定歌词行数组中是否存在任何需要写入 `<iTunesMetadata>` 的翻译或音译内容
pub fn has_any_itunes_sub_lyrics(lines: &[LyricLine], config: &GeneratorConfig) -> bool {
    let check = |contents: Option<&[SubLyricContent]>| {
        contents.is_some_and(|c| should_write_to_head(c, config))
    };

    lines.iter().any(|line| {
        check(line.translations.as_deref())
            || check(line.romanizations.as_deref())
            || line
                .background_vocal
                .as_ref()
                .filter(|_| !config.line_timing)
                .is_some_and(|bg| {
                    check(bg.translations.as_deref()) || check(bg.romanizations.as_deref())
                })
    })
}

/// 检查给定歌词行中是否有任何需要写成内嵌 span 的翻译或音译
pub fn has_inline_sub_lyrics(line: &LyricLine, config: &GeneratorConfig) -> bool {
    if config.use_apple_format_rules {
        return false;
    }

    let check = |contents: Option<&[SubLyricContent]>| {
        contents.is_some_and(|c| should_write_inline(c, config))
    };

    check(line.translations.as_deref())
        || check(line.romanizations.as_deref())
        || line
            .background_vocal
            .as_ref()
            .filter(|_| !config.line_timing)
            .is_some_and(|bg| {
                check(bg.translations.as_deref()) || check(bg.romanizations.as_deref())
            })
}
