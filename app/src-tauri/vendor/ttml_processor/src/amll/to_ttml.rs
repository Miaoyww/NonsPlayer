use compact_str::CompactString;

use crate::{
    amll::{
        AmllLyricLine,
        AmllLyricResult,
        AmllMetadata,
        AmllToTtmlOptions,
    },
    constants::{
        meta_keys,
        vals,
    },
    model::{
        Agent,
        BackgroundVocal,
        LyricLine,
        PlatformId,
        RubyTag,
        SubLyricContent,
        Syllable,
        TTMLMetadata,
        TTMLResult,
    },
    utils::{
        normalize_words_spaces,
        strip_outer_parens_from_words,
    },
};

/// 将 AMLL 格式的歌词和元数据转换为 [`TTMLResult`] 结构
///
/// 会对文本进行规范化，例如清理空格、移除背景人声括号等
#[must_use]
pub fn to_ttml_result(
    amll_result: AmllLyricResult,
    options: Option<&AmllToTtmlOptions>,
) -> TTMLResult {
    let trans_lang = options
        .and_then(|o| o.translation_language.as_deref())
        .unwrap_or(vals::TRANSLATION_DEFAULT_LANGUAGE);
    let rom_lang = options.and_then(|o| o.romanization_language.as_deref());

    let mut metadata = convert_metadata(amll_result.metadata);
    let mut lines = Vec::with_capacity(amll_result.lines.len());

    let mut has_duet_in_song = false;
    let mut last_was_bg = false;
    let mut last_main_is_duet = false;
    let mut line_index = 1;

    for amll_line in amll_result.lines {
        let is_bg = amll_line.is_bg;
        let original_is_duet = amll_line.is_duet;
        let start_time = amll_line.start_time;
        let end_time = amll_line.end_time;

        let (syllables, translations, romanizations) =
            convert_single_line(amll_line, trans_lang, rom_lang);

        // 如果遇到连续的背景歌词，将除了第一行之外的所有背景歌词全部提升为主歌词
        let should_be_main = !is_bg || lines.is_empty() || last_was_bg;

        let mut words_opt = (!syllables.is_empty()).then_some(syllables);
        let trans_opt = (!translations.is_empty()).then_some(translations);
        let rom_opt = (!romanizations.is_empty()).then_some(romanizations);

        if should_be_main {
            if is_bg && let Some(words) = &mut words_opt {
                strip_outer_parens_from_words(words);
            }

            let is_duet = if is_bg && !lines.is_empty() {
                last_main_is_duet
            } else {
                last_main_is_duet = original_is_duet;
                original_is_duet
            };

            if is_duet {
                has_duet_in_song = true;
            }

            let agent_id = if is_duet {
                vals::AGENT_DEFAULT_DUET
            } else {
                vals::AGENT_DEFAULT
            };

            let mut line = LyricLine {
                text: String::new(),
                start_time,
                end_time,
                words: words_opt,
                translations: trans_opt,
                romanizations: rom_opt,
                background_vocal: None,
                id: Some(CompactString::new(format!("L{line_index}"))),
                agent_id: Some(CompactString::const_new(agent_id)),
                song_part: None,
                block_index: None,
            };

            line.rebuild_text();
            lines.push(line);
            line_index += 1;
        } else if let Some(last_line) = lines.last_mut() {
            let mut bg_vocal = BackgroundVocal {
                text: String::new(),
                start_time,
                end_time,
                words: words_opt,
                translations: trans_opt,
                romanizations: rom_opt,
            };

            bg_vocal.normalize();
            last_line.background_vocal = Some(bg_vocal);
        }

        last_was_bg = is_bg;
    }

    metadata.insert_agent(Agent {
        id: CompactString::const_new(vals::AGENT_DEFAULT),
        name: None,
        type_: Some(CompactString::const_new(vals::PERSON)),
    });

    if has_duet_in_song {
        metadata.insert_agent(Agent {
            id: CompactString::const_new(vals::AGENT_DEFAULT_DUET),
            name: None,
            type_: Some(CompactString::const_new(vals::PERSON)),
        });
    }

    TTMLResult { metadata, lines }
}

fn convert_metadata(amll_metadata: Vec<AmllMetadata>) -> TTMLMetadata {
    let mut metadata = TTMLMetadata::default();

    for AmllMetadata { key, value } in amll_metadata {
        let mut iter = value.into_iter();

        match key.as_str() {
            meta_keys::LANGUAGE => metadata.language = iter.next(),
            meta_keys::TIMING_MODE => metadata.timing_mode = iter.next(),

            meta_keys::TITLE | meta_keys::MUSIC_NAME => iter.for_each(|v| metadata.push_title(v)),
            meta_keys::ARTISTS => iter.for_each(|v| metadata.push_artist(v)),
            meta_keys::ALBUM => iter.for_each(|v| metadata.push_album(v)),
            meta_keys::SONGWRITERS => iter.for_each(|v| metadata.push_songwriter(v)),
            meta_keys::ISRC => iter.for_each(|v| metadata.push_isrc(v)),
            meta_keys::GITHUB_ID => iter.for_each(|v| metadata.push_author_id(v)),
            meta_keys::GITHUB_USER_NAME => iter.for_each(|v| metadata.push_author_name(v)),
            meta_keys::NCM_ID => {
                iter.for_each(|v| metadata.push_platform_id(PlatformId::NcmMusicId, v));
            }
            meta_keys::QQ_ID => {
                iter.for_each(|v| metadata.push_platform_id(PlatformId::QqMusicId, v));
            }
            meta_keys::SPOTIFY_ID => {
                iter.for_each(|v| metadata.push_platform_id(PlatformId::SpotifyId, v));
            }
            meta_keys::APPLE_ID => {
                iter.for_each(|v| metadata.push_platform_id(PlatformId::AppleMusicId, v));
            }
            _ => iter.for_each(|v| metadata.push_raw_property(key.clone(), v)),
        }
    }
    metadata
}

fn convert_single_line(
    amll_line: AmllLyricLine,
    trans_lang: &str,
    rom_lang: Option<&str>,
) -> (Vec<Syllable>, Vec<SubLyricContent>, Vec<SubLyricContent>) {
    let has_rom_words = amll_line.words.iter().any(|w| w.roman_word.is_some());

    let capacity = amll_line.words.len();
    let mut syllables = Vec::with_capacity(capacity);
    let mut rom_syllables = if has_rom_words {
        Vec::with_capacity(capacity)
    } else {
        Vec::new()
    };

    for w in amll_line.words {
        let ruby = w.ruby.map(|r_list| {
            r_list
                .into_iter()
                .map(|r| RubyTag {
                    text: r.word,
                    start_time: r.start_time,
                    end_time: r.end_time,
                })
                .collect()
        });

        syllables.push(Syllable {
            text: w.word,
            start_time: w.start_time,
            end_time: w.end_time,
            ends_with_space: None,
            ruby,
            obscene: w.obscene,
            empty_beat: w.empty_beat,
        });

        if has_rom_words {
            let rom_raw = w.roman_word.unwrap_or_default();
            rom_syllables.push(Syllable {
                text: rom_raw,
                start_time: w.start_time,
                end_time: w.end_time,
                ..Default::default()
            });
        }
    }

    normalize_words_spaces(&mut syllables);

    let mut translations = Vec::new();
    if !amll_line.translated_lyric.is_empty() {
        let mut trans = SubLyricContent {
            language: Some(trans_lang.into()),
            automatically_created: None,
            text: amll_line.translated_lyric,
            words: None,
        };

        trans.normalize(false);
        translations.push(trans);
    }

    let mut romanizations = Vec::new();
    if has_rom_words {
        let mut rom_content = SubLyricContent {
            language: rom_lang.map(CompactString::new),
            automatically_created: None,
            text: String::new(),
            words: Some(rom_syllables),
        };
        rom_content.normalize(true);
        rom_content.text = amll_line.roman_lyric;
        romanizations.push(rom_content);
    } else if !amll_line.roman_lyric.is_empty() {
        let mut rom_content = SubLyricContent {
            language: rom_lang.map(CompactString::new),
            automatically_created: None,
            text: amll_line.roman_lyric,
            words: None,
        };

        rom_content.normalize(false);
        romanizations.push(rom_content);
    }

    (syllables, translations, romanizations)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::amll::AmllLyricWord;

    fn create_syllable(word: &str, start_time: u32, end_time: u32) -> AmllLyricWord {
        AmllLyricWord {
            word: CompactString::new(word),
            start_time,
            end_time,
            ..Default::default()
        }
    }

    #[test]
    fn test_metadata_conversion() {
        let amll_result = AmllLyricResult {
            lines: vec![],
            metadata: vec![
                AmllMetadata {
                    key: CompactString::const_new(meta_keys::TITLE),
                    value: vec![CompactString::new("Test Song")],
                },
                AmllMetadata {
                    key: CompactString::const_new(meta_keys::ARTISTS),
                    value: vec![
                        CompactString::new("Artist A"),
                        CompactString::new("Artist B"),
                    ],
                },
                AmllMetadata {
                    key: CompactString::const_new(meta_keys::NCM_ID),
                    value: vec![CompactString::new("123456")],
                },
                AmllMetadata {
                    key: CompactString::const_new("CUSTOM_KEY"),
                    value: vec![CompactString::new("custom_val")],
                },
            ],
        };

        let result = to_ttml_result(amll_result, None);
        let meta = result.metadata;

        assert_eq!(meta.title.as_ref().unwrap()[0], "Test Song");
        assert_eq!(meta.artist.as_ref().unwrap().len(), 2);
        assert_eq!(meta.artist.as_ref().unwrap()[1], "Artist B");

        let platform_ids = meta.platform_ids.unwrap();
        assert_eq!(
            platform_ids.get(&PlatformId::NcmMusicId).unwrap()[0],
            "123456"
        );

        let raw_props = meta.raw_properties.unwrap();
        assert_eq!(raw_props.get("CUSTOM_KEY").unwrap()[0], "custom_val");

        let agents = meta.agents.unwrap();
        assert!(agents.contains_key(vals::AGENT_DEFAULT));
    }

    #[test]
    fn test_bg_and_duet_logic() {
        let amll_result = AmllLyricResult {
            metadata: vec![],
            lines: vec![
                AmllLyricLine {
                    words: vec![
                        create_syllable("Hello ", 0, 500),
                        create_syllable("World", 500, 1000),
                    ],
                    translated_lyric: String::new(),
                    roman_lyric: String::new(),
                    is_bg: false,
                    is_duet: false,
                    start_time: 0,
                    end_time: 1000,
                },
                AmllLyricLine {
                    words: vec![
                        create_syllable("你好 ", 1000, 1500),
                        create_syllable("世界", 1500, 2000),
                    ],
                    translated_lyric: String::new(),
                    roman_lyric: String::new(),
                    is_bg: false,
                    is_duet: true,
                    start_time: 1000,
                    end_time: 2000,
                },
                AmllLyricLine {
                    words: vec![create_syllable("(echo)", 1500, 2000)],
                    translated_lyric: String::new(),
                    roman_lyric: String::new(),
                    is_bg: true,
                    is_duet: false,
                    start_time: 1500,
                    end_time: 2000,
                },
                AmllLyricLine {
                    words: vec![create_syllable("(solo)", 2000, 3000)],
                    translated_lyric: String::new(),
                    roman_lyric: String::new(),
                    is_bg: true,
                    is_duet: false,
                    start_time: 2000,
                    end_time: 3000,
                },
            ],
        };

        let result = to_ttml_result(amll_result, None);

        assert_eq!(result.lines.len(), 3);

        assert_eq!(result.lines[0].text, "Hello World");
        assert_eq!(
            result.lines[0].words.as_ref().unwrap()[0].ends_with_space,
            Some(true)
        );
        assert_eq!(
            result.lines[0].agent_id.as_deref(),
            Some(vals::AGENT_DEFAULT)
        );
        assert!(result.lines[0].background_vocal.is_none());

        assert_eq!(result.lines[1].text, "你好 世界");
        assert_eq!(
            result.lines[1].words.as_ref().unwrap()[0].ends_with_space,
            Some(true)
        );
        assert_eq!(
            result.lines[1].agent_id.as_deref(),
            Some(vals::AGENT_DEFAULT_DUET)
        );

        let bg = result.lines[1]
            .background_vocal
            .as_ref()
            .expect("Should have background vocal");
        assert_eq!(bg.text, "echo");

        assert_eq!(result.lines[2].text, "solo");
        assert_eq!(
            result.lines[2].agent_id.as_deref(),
            Some(vals::AGENT_DEFAULT_DUET)
        );

        let agents = result.metadata.agents.unwrap();
        assert!(agents.contains_key(vals::AGENT_DEFAULT_DUET));
    }

    #[test]
    fn test_sub_lyric() {
        let mut word_with_rom = create_syllable("君", 0, 500);
        word_with_rom.roman_word = Some("kimi".into());

        let amll_result = AmllLyricResult {
            metadata: vec![],
            lines: vec![AmllLyricLine {
                words: vec![word_with_rom],
                translated_lyric: "You".to_string(),
                roman_lyric: "ki mi".to_string(),
                is_bg: false,
                is_duet: false,
                start_time: 0,
                end_time: 500,
            }],
        };

        let options = AmllToTtmlOptions {
            translation_language: Some("en-US".to_string()),
            romanization_language: Some("ja-Latn".to_string()),
        };

        let result = to_ttml_result(amll_result, Some(&options));
        let line = &result.lines[0];

        let trans = line.translations.as_ref().unwrap();
        assert_eq!(trans.len(), 1);
        assert_eq!(trans[0].language.as_deref(), Some("en-US"));
        assert_eq!(trans[0].text, "You");

        let roms = line.romanizations.as_ref().unwrap();
        assert_eq!(roms.len(), 1);
        assert_eq!(roms[0].language.as_deref(), Some("ja-Latn"));

        let rom_words = roms[0].words.as_ref().unwrap();
        assert_eq!(rom_words[0].text, "kimi");
    }
}
