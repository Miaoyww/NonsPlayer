use compact_str::CompactString;

use crate::{
    amll::{
        AmllLyricLine,
        AmllLyricResult,
        AmllLyricWord,
        AmllMetadata,
        LyricWordBase,
        TtmlToAmllOptions,
    },
    constants::{
        meta_keys,
        vals,
    },
    model::{
        PlatformId,
        SubLyricContent,
        Syllable,
        TTMLMetadata,
        TTMLResult,
    },
};

struct LyricBase {
    words: Option<Vec<Syllable>>,
    translations: Option<Vec<SubLyricContent>>,
    romanizations: Option<Vec<SubLyricContent>>,
    text: CompactString,
    start_time: u32,
    end_time: u32,
    is_bg: bool,
    is_duet: bool,
}

/// 将本解析器复杂的数据结构降级为 AMLL 所使用的较简单的数据结构
#[must_use]
pub fn to_amll_lyrics(result: TTMLResult, options: Option<&TtmlToAmllOptions>) -> AmllLyricResult {
    let trans_lang = options.and_then(|o| o.translation_language.as_deref());
    let rom_lang = options.and_then(|o| o.romanization_language.as_deref());

    let meta = result.metadata;
    let mut amll_lines = Vec::with_capacity(result.lines.len());

    let mut last_singer_id: Option<CompactString> = None;
    let mut last_was_duet: bool = false;

    for line in result.lines {
        let is_duet = get_line_duet_status(
            line.agent_id.as_ref(),
            &meta,
            &mut last_singer_id,
            &mut last_was_duet,
        );

        let bg_vocal = line.background_vocal;

        let main_line = convert_to_amll_line(
            LyricBase {
                words: line.words,
                translations: line.translations,
                romanizations: line.romanizations,
                text: line.text.into(),
                start_time: line.start_time,
                end_time: line.end_time,
                is_bg: false,
                is_duet,
            },
            trans_lang,
            rom_lang,
        );
        amll_lines.push(main_line);

        if let Some(bg) = bg_vocal {
            let bg_line = convert_to_amll_line(
                LyricBase {
                    words: bg.words,
                    translations: bg.translations,
                    romanizations: bg.romanizations,
                    text: bg.text.into(),
                    start_time: bg.start_time,
                    end_time: bg.end_time,
                    is_bg: true,
                    is_duet,
                },
                trans_lang,
                rom_lang,
            );
            amll_lines.push(bg_line);
        }
    }

    let mut capacity = 9;
    if let Some(p) = &meta.platform_ids {
        capacity += p.len();
    }
    if let Some(r) = &meta.raw_properties {
        capacity += r.len();
    }
    let mut metadata: Vec<AmllMetadata> = Vec::with_capacity(capacity);

    let mut push_meta = |key: &'static str, vals: Option<Vec<CompactString>>| {
        if let Some(mut v) = vals {
            v.retain(|s| !s.trim().is_empty());

            if !v.is_empty() {
                metadata.push(AmllMetadata {
                    key: CompactString::const_new(key),
                    value: v,
                });
            }
        }
    };

    push_meta(meta_keys::TITLE, meta.title);
    push_meta(meta_keys::ARTISTS, meta.artist);
    push_meta(meta_keys::ALBUM, meta.album);
    push_meta(meta_keys::SONGWRITERS, meta.songwriters);
    push_meta(meta_keys::ISRC, meta.isrc);
    push_meta(meta_keys::GITHUB_ID, meta.author_ids);
    push_meta(meta_keys::GITHUB_USER_NAME, meta.author_names);

    if let Some(lang) = meta.language {
        push_meta(meta_keys::LANGUAGE, Some(vec![lang]));
    }
    if let Some(timing_mode) = meta.timing_mode {
        push_meta(meta_keys::TIMING_MODE, Some(vec![timing_mode]));
    }

    if let Some(platform_ids) = meta.platform_ids {
        metadata.extend(platform_ids.into_iter().map(|(k, v)| {
            let key_str = match k {
                PlatformId::NcmMusicId => meta_keys::NCM_ID,
                PlatformId::QqMusicId => meta_keys::QQ_ID,
                PlatformId::SpotifyId => meta_keys::SPOTIFY_ID,
                PlatformId::AppleMusicId => meta_keys::APPLE_ID,
            };
            AmllMetadata {
                key: CompactString::const_new(key_str),
                value: v,
            }
        }));
    }

    if let Some(raw) = meta.raw_properties {
        metadata.extend(
            raw.into_iter()
                .map(|(key, value)| AmllMetadata { key, value }),
        );
    }

    AmllLyricResult {
        lines: amll_lines,
        metadata,
    }
}

fn convert_to_amll_line(
    base: LyricBase,
    trans_lang: Option<&str>,
    rom_lang: Option<&str>,
) -> AmllLyricLine {
    let LyricBase {
        words,
        translations,
        romanizations,
        text,
        start_time,
        end_time,
        is_bg,
        is_duet,
    } = base;

    let trans_content = take_sub_lyric(translations, trans_lang);
    let translated_lyric = trans_content.map_or_else(Default::default, |t| t.text);

    let rom_content = take_sub_lyric(romanizations, rom_lang);
    let roman_lyric = rom_content
        .as_ref()
        .filter(|r| r.words.as_ref().is_none_or(Vec::is_empty))
        .map_or_else(Default::default, |r| r.text.clone());

    let amll_words = match words {
        Some(line_words) if !line_words.is_empty() => {
            build_amll_words(line_words, rom_content.as_ref())
        }
        _ => {
            let roman_word = rom_content
                .as_ref()
                .map(|r| r.text.trim().into())
                .filter(|text: &CompactString| !text.is_empty());

            vec![AmllLyricWord {
                start_time,
                end_time,
                word: text,
                roman_word,
                obscene: None,
                empty_beat: None,
                ruby: None,
            }]
        }
    };

    AmllLyricLine {
        words: amll_words,
        translated_lyric,
        roman_lyric,
        is_bg,
        is_duet,
        start_time,
        end_time,
    }
}

fn take_sub_lyric(
    subs: Option<Vec<SubLyricContent>>,
    target_lang: Option<&str>,
) -> Option<SubLyricContent> {
    let mut subs = subs?;

    if let Some(target) = target_lang
        && let Some(pos) = subs
            .iter()
            .position(|s| s.language.as_deref() == Some(target))
    {
        return Some(subs.swap_remove(pos));
    }

    if subs.is_empty() {
        None
    } else {
        Some(subs.swap_remove(0))
    }
}

/// 判断该行的对唱状态
///
/// 使用 Apple Music 风格的对唱识别逻辑
fn get_line_duet_status(
    agent_id: Option<&CompactString>,
    metadata: &TTMLMetadata,
    last_singer_id: &mut Option<CompactString>,
    last_was_duet: &mut bool,
) -> bool {
    let fallback = CompactString::const_new(vals::AGENT_DEFAULT);
    let id = agent_id.unwrap_or(&fallback);

    let agent = metadata.agents.as_ref().and_then(|m| m.get(id));
    let agent_type = agent
        .and_then(|a| a.type_.as_deref())
        .unwrap_or(vals::PERSON);

    // 合唱始终非对唱，且不影响其他 agent type 的交替计算
    if agent_type.eq_ignore_ascii_case(vals::GROUP) {
        return false;
    }

    let current_is_duet = match last_singer_id.as_ref() {
        // 如果第一次遇到的演唱者类型是 Other，强制为对唱，否则非对唱
        None => agent_type.eq_ignore_ascii_case(vals::OTHER),
        // 与上一个非 Group 演唱者相同，保持对唱状态
        Some(last_id) if last_id == id => *last_was_duet,
        // 与上一个非 Group 演唱者不同，翻转对唱侧
        _ => !*last_was_duet,
    };

    *last_singer_id = Some(id.clone());
    *last_was_duet = current_is_duet;

    current_is_duet
}

fn find_best_roman_match<'a>(
    main_start_time: i64,
    main_end_time: i64,
    rom_words: &'a [Syllable],
    search_start_index: &mut usize,
) -> Option<&'a str> {
    /// 交并比阈值，至少有 10% 的面积重合
    const MIN_IOU_THRESHOLD: f64 = 0.1;

    /// 快速通道，优先匹配时间戳完全相同的主歌词和音译音节，同时避免浮点数误差
    const FAST_TRACK_TOLERANCE_MS: i64 = 2;

    let mut max_iou = 0.0;
    let mut best_match_index = None;

    let mut j = *search_start_index;
    while j < rom_words.len() {
        let sub = &rom_words[j];
        let sub_start_time = i64::from(sub.start_time);
        let sub_end_time = i64::from(sub.end_time);

        if (main_start_time - sub_start_time).abs() <= FAST_TRACK_TOLERANCE_MS {
            *search_start_index = j + 1;
            return Some(sub.text.trim());
        }

        // 交集
        let overlap_start = main_start_time.max(sub_start_time);
        let overlap_end = main_end_time.min(sub_end_time);
        let intersection = 0.max(overlap_end - overlap_start) as f64;

        if intersection > 0.0 {
            // 并集
            let union_start = main_start_time.min(sub_start_time);
            let union_end = main_end_time.max(sub_end_time);
            let union_duration = 1.max(union_end - union_start) as f64;

            let iou = intersection / union_duration;

            if iou > max_iou {
                max_iou = iou;
                best_match_index = Some(j);
            }
        }

        if sub_start_time >= main_end_time {
            break;
        }

        j += 1;
    }

    if let Some(idx) = best_match_index
        && max_iou >= MIN_IOU_THRESHOLD
    {
        *search_start_index = idx + 1;
        return Some(rom_words[idx].text.trim());
    }

    None
}

fn build_amll_words(
    words: impl IntoIterator<Item = Syllable>,
    rom_content: Option<&SubLyricContent>,
) -> Vec<AmllLyricWord> {
    let rom_words = rom_content.and_then(|r| r.words.as_deref()).unwrap_or(&[]);

    let mut roman_search_start_index = 0;

    words
        .into_iter()
        .map(|word| {
            let roman_word = find_best_roman_match(
                i64::from(word.start_time),
                i64::from(word.end_time),
                rom_words,
                &mut roman_search_start_index,
            )
            .map(Into::into);

            let ruby = word.ruby.map(|rubies| {
                rubies
                    .into_iter()
                    .map(|r| LyricWordBase {
                        start_time: r.start_time,
                        end_time: r.end_time,
                        word: r.text,
                    })
                    .collect()
            });

            let mut final_text = word.text;
            if word.ends_with_space.unwrap_or(false) {
                final_text.push(' ');
            }

            AmllLyricWord {
                start_time: word.start_time,
                end_time: word.end_time,
                word: final_text,
                roman_word,
                obscene: word.obscene,
                empty_beat: word.empty_beat,
                ruby,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        Agent,
        BackgroundVocal,
        LyricLine,
    };

    fn create_syllable(text: &str, start: u32, end: u32, ends_with_space: bool) -> Syllable {
        Syllable {
            text: text.into(),
            start_time: start,
            end_time: end,
            ends_with_space: Some(ends_with_space),
            ..Default::default()
        }
    }

    #[test]
    fn test_basic_conversion_and_metadata() {
        let mut metadata = TTMLMetadata::default();
        metadata.push_title("Test Song".into());
        metadata.push_artist("Test Artist".into());
        metadata.push_platform_id(PlatformId::NcmMusicId, "12345".into());
        metadata.language = Some("en-US".into());

        let line = LyricLine {
            text: "Hello World".to_string(),
            start_time: 0,
            end_time: 1000,
            words: Some(vec![
                create_syllable("Hello", 0, 500, true),
                create_syllable("World", 500, 1000, false),
            ]),
            ..Default::default()
        };

        let ttml_result = TTMLResult {
            metadata,
            lines: vec![line],
        };

        let amll_result = to_amll_lyrics(ttml_result, None);

        let meta_map: std::collections::HashMap<_, _> = amll_result
            .metadata
            .into_iter()
            .map(|m| (m.key.to_string(), m.value))
            .collect();

        assert_eq!(meta_map.get(meta_keys::TITLE).unwrap()[0], "Test Song");
        assert_eq!(meta_map.get(meta_keys::ARTISTS).unwrap()[0], "Test Artist");
        assert_eq!(meta_map.get(meta_keys::NCM_ID).unwrap()[0], "12345");
        assert_eq!(meta_map.get(meta_keys::LANGUAGE).unwrap()[0], "en-US");

        assert_eq!(amll_result.lines.len(), 1);
        let amll_line = &amll_result.lines[0];
        assert_eq!(amll_line.start_time, 0);
        assert_eq!(amll_line.end_time, 1000);
        assert!(!amll_line.is_bg);
        assert_eq!(amll_line.words.len(), 2);
        assert_eq!(amll_line.words[0].word, "Hello ");
        assert_eq!(amll_line.words[1].word, "World");
    }

    #[test]
    fn test_background_vocal_flattening() {
        let mut line = LyricLine {
            text: "Main Line".to_string(),
            start_time: 0,
            end_time: 2000,
            words: Some(vec![
                create_syllable("Main", 0, 1000, true),
                create_syllable("Line", 1000, 2000, false),
            ]),
            ..Default::default()
        };

        line.background_vocal = Some(BackgroundVocal {
            text: "(Bg Line)".to_string(),
            start_time: 500,
            end_time: 1500,
            words: Some(vec![
                create_syllable("Bg", 500, 1000, true),
                create_syllable("Line", 1000, 1500, false),
            ]),
            ..Default::default()
        });

        let ttml_result = TTMLResult {
            metadata: TTMLMetadata::default(),
            lines: vec![line],
        };

        let amll_result = to_amll_lyrics(ttml_result, None);

        assert_eq!(amll_result.lines.len(), 2);

        assert!(!amll_result.lines[0].is_bg);
        assert_eq!(amll_result.lines[0].words[0].word, "Main ");

        assert!(amll_result.lines[1].is_bg);
        assert_eq!(amll_result.lines[1].words[0].word, "Bg ");
        assert_eq!(amll_result.lines[1].start_time, 500);
    }

    #[test]
    fn test_sub_lyric_extraction() {
        let mut line = LyricLine {
            text: "你好".to_string(),
            start_time: 0,
            end_time: 1000,
            words: Some(vec![
                create_syllable("你", 0, 500, false),
                create_syllable("好", 500, 1000, false),
            ]),
            ..Default::default()
        };

        line.translations = Some(vec![
            SubLyricContent {
                language: Some("en".into()),
                automatically_created: None,
                text: "Hello".to_string(),
                words: None,
            },
            SubLyricContent {
                language: Some("zh-Hant".into()),
                automatically_created: None,
                text: "你好".to_string(),
                words: None,
            },
        ]);

        line.romanizations = Some(vec![SubLyricContent {
            language: Some("zh-Latn".into()),
            automatically_created: None,
            text: "ni hao".to_string(),
            words: Some(vec![
                create_syllable("ni", 5, 505, true),
                create_syllable("hao", 498, 1005, false),
            ]),
        }]);

        let ttml_result = TTMLResult {
            metadata: TTMLMetadata::default(),
            lines: vec![line],
        };

        let options = TtmlToAmllOptions {
            translation_language: Some("zh-Hant".to_string()),
            romanization_language: Some("zh-Latn".to_string()),
        };

        let amll_result = to_amll_lyrics(ttml_result, Some(&options));
        let amll_line = &amll_result.lines[0];

        assert_eq!(amll_line.translated_lyric, "你好");

        assert_eq!(amll_line.words.len(), 2);
        assert_eq!(amll_line.words[0].roman_word.as_deref(), Some("ni"));
        assert_eq!(amll_line.words[1].roman_word.as_deref(), Some("hao"));
    }

    #[test]
    fn test_duet_state_machine() {
        let mut metadata = TTMLMetadata::default();

        metadata.insert_agent(Agent {
            id: "v1".into(),
            type_: Some(vals::PERSON.into()),
            ..Default::default()
        });
        metadata.insert_agent(Agent {
            id: "v2".into(),
            type_: Some(vals::PERSON.into()),
            ..Default::default()
        });
        metadata.insert_agent(Agent {
            id: "v3".into(),
            type_: Some(vals::PERSON.into()),
            ..Default::default()
        });
        metadata.insert_agent(Agent {
            id: "v4".into(),
            type_: Some(vals::OTHER.into()),
            ..Default::default()
        });
        metadata.insert_agent(Agent {
            id: "v1000".into(),
            type_: Some(vals::GROUP.into()),
            ..Default::default()
        });

        let create_line = |agent_id: &str| LyricLine {
            agent_id: Some(agent_id.into()),
            words: Some(vec![create_syllable("Test", 0, 100, false)]),
            ..Default::default()
        };

        let ttml_result = TTMLResult {
            metadata,
            lines: vec![
                create_line("v4"),    // 对唱
                create_line("v4"),    // 对唱
                create_line("v1"),    // 非对唱
                create_line("v1"),    // 非对唱
                create_line("v2"),    // 对唱
                create_line("v3"),    // 非对唱
                create_line("v1000"), // 非对唱
                create_line("v1000"), // 非对唱
                create_line("v1"),    // 对唱
                create_line("v2"),    // 非对唱
            ],
        };

        let amll_result = to_amll_lyrics(ttml_result, None);

        let duet_states: Vec<bool> = amll_result.lines.iter().map(|l| l.is_duet).collect();
        assert_eq!(
            duet_states,
            vec![
                true, true, false, false, true, false, false, false, true, false
            ],
            "对唱状态机计算错误"
        );
    }

    #[test]
    fn test_iou_match() {
        let rom_words = vec![create_syllable("a", 300, 800, false)];
        let mut search_idx = 0;

        let result = find_best_roman_match(200, 700, &rom_words, &mut search_idx);

        assert_eq!(result, Some("a"));
        assert_eq!(search_idx, 1);
    }

    #[test]
    fn test_iou_below_threshold() {
        let rom_words = vec![create_syllable("a", 800, 1000, false)];
        let mut search_idx = 0;

        let result = find_best_roman_match(0, 810, &rom_words, &mut search_idx);

        assert_eq!(result, None, "重叠过小，不应匹配");
        assert_eq!(search_idx, 0, "未匹配成功时，游标不应推进");
    }

    #[test]
    fn test_sequential_matching_and_state() {
        let rom_words = vec![
            create_syllable("a", 0, 500, false),
            create_syllable("b", 500, 1000, false),
            create_syllable("c", 1000, 1500, false),
        ];
        let mut search_idx = 0;

        let res1 = find_best_roman_match(10, 490, &rom_words, &mut search_idx);
        assert_eq!(res1, Some("a"));
        assert_eq!(search_idx, 1);

        let res2 = find_best_roman_match(1020, 1480, &rom_words, &mut search_idx);
        assert_eq!(res2, Some("c"));
        assert_eq!(search_idx, 3, "游标应正确跳过未匹配项，推进到 idx 3");
    }

    #[test]
    fn test_empty_roman_word_match() {
        let rom_words = vec![
            create_syllable("ko", 120_250, 120_690, false),
            create_syllable("u", 120_695, 120_870, false),
            create_syllable("ki", 121_070, 121_470, false),
        ];

        let mut search_idx = 0;

        let res_1 = find_best_roman_match(120_250, 120_690, &rom_words, &mut search_idx);
        assert_eq!(res_1, Some("ko"), "应精准匹配 'ko'");
        assert_eq!(search_idx, 1);

        let res_space = find_best_roman_match(120_690, 120_695, &rom_words, &mut search_idx);
        assert_eq!(res_space, None, "不应匹配到任何罗马音");

        let res_2 = find_best_roman_match(120_695, 120_870, &rom_words, &mut search_idx);
        assert_eq!(res_2, Some("u"), "应精准匹配 'u'");
        assert_eq!(search_idx, 2);

        let res_3 = find_best_roman_match(121_070, 121_470, &rom_words, &mut search_idx);
        assert_eq!(res_3, Some("ki"), "应精准匹配 'ki'");
        assert_eq!(search_idx, 3);
    }

    #[test]
    fn test_main_word_shorter_than_roman_word() {
        let rom_words = vec![create_syllable("shi", 100, 300, false)];
        let mut search_idx = 0;

        let result = find_best_roman_match(150, 250, &rom_words, &mut search_idx);

        assert_eq!(
            result,
            Some("shi"),
            "当主歌词被音译词完全包裹（且满足 IOU 阈值）时，应正确匹配"
        );
        assert_eq!(search_idx, 1, "匹配成功，游标应当推进");
    }
}
