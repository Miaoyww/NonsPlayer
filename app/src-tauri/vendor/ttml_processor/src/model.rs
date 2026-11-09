//! TTML 解析和生成器使用的数据结构

use compact_str::CompactString;
use indexmap::IndexMap;
use serde::{
    Deserialize,
    Serialize,
};
use serde_with::skip_serializing_none;

use crate::utils::{
    build_full_text,
    normalize_line_text,
    normalize_words_spaces,
    strip_outer_parens,
    strip_outer_parens_from_words,
};

/// 翻译/音译的内容
#[skip_serializing_none]
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubLyricContent {
    /// 该内容的 BCP-47 语言代码
    pub language: Option<CompactString>,

    /// 该内容是否由系统自动生成
    ///
    /// 对应 `<translation>` / `<transliteration>` 上的 `automaticallyCreated` 属性。
    /// 仅解析与生成 head 中的这两个元素；内嵌的 `x-translation` / `x-roman`
    /// span 与 AMLL 转换不会保留该标志。属性缺失、取值为 `false` 或无法识别时均为 `None`。
    pub automatically_created: Option<bool>,

    /// 完整文本
    pub text: String,

    /// 逐字音节信息
    pub words: Option<Vec<Syllable>>,
}

/// 背景人声内容
#[skip_serializing_none]
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackgroundVocal {
    /// 完整的文本内容
    /// - 如果是逐字歌词，这里是所有字拼接后的结果
    pub text: String,

    /// 开始时间，单位毫秒
    pub start_time: u32,

    /// 结束时间，单位毫秒
    pub end_time: u32,

    /// 逐字音节信息
    ///
    /// 如果为空，一般就是逐行歌词
    pub words: Option<Vec<Syllable>>,

    /// 翻译内容
    pub translations: Option<Vec<SubLyricContent>>,

    /// 音译内容
    pub romanizations: Option<Vec<SubLyricContent>>,
}

/// 一个主歌词行
#[skip_serializing_none]
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricLine {
    /// 完整的文本内容
    /// - 如果是逐字歌词，这里是所有字拼接后的结果
    pub text: String,

    /// 开始时间，单位毫秒
    pub start_time: u32,

    /// 结束时间，单位毫秒
    pub end_time: u32,

    /// 逐字音节信息
    ///
    /// 如果为空，一般就是逐行歌词
    pub words: Option<Vec<Syllable>>,

    /// 翻译内容
    pub translations: Option<Vec<SubLyricContent>>,

    /// 音译内容
    pub romanizations: Option<Vec<SubLyricContent>>,

    /// 背景人声内容
    pub background_vocal: Option<BackgroundVocal>,

    /// 行 ID
    ///
    /// 例如 "L1", "L2"...
    pub id: Option<CompactString>,

    /// 演唱者 ID
    ///
    /// 可用于在 metadata.agents 中查找具体名字
    pub agent_id: Option<CompactString>,

    /// 歌曲结构组成
    ///
    /// 例如: "Verse", "Chorus", "Intro", "Outro"
    pub song_part: Option<CompactString>,

    /// 所属的递增区块索引
    ///
    /// 用于区分连续出现但属于不同 div 的同名 songPart
    pub block_index: Option<u32>,
}

impl LyricLine {
    /// 向主歌词行追加一个音节
    pub fn push_word(&mut self, syllable: Syllable) {
        self.words
            .get_or_insert_with(|| Vec::with_capacity(12))
            .push(syllable);
    }

    /// 向主歌词行追加一条翻译内容
    pub fn push_translation(&mut self, content: SubLyricContent) {
        self.translations.get_or_insert_with(Vec::new).push(content);
    }

    /// 向主歌词行追加一条音译内容
    pub fn push_romanization(&mut self, content: SubLyricContent) {
        self.romanizations
            .get_or_insert_with(Vec::new)
            .push(content);
    }

    /// 获取或初始化 `BackgroundVocal` 的可变引用
    pub fn bg_vocal_mut(&mut self) -> &mut BackgroundVocal {
        self.background_vocal
            .get_or_insert_with(BackgroundVocal::default)
    }

    /// 根据当前的 words 重新构建 text 字段
    ///
    /// 一般用来从逐字歌词生成逐行文本，没有 words 字段则什么也不会做
    pub fn rebuild_text(&mut self) {
        if let Some(words) = &self.words {
            self.text = build_full_text(words, false);
        }
    }
}

impl BackgroundVocal {
    /// 向背景人声追加一个音节
    pub fn push_word(&mut self, syllable: Syllable) {
        self.words
            .get_or_insert_with(|| Vec::with_capacity(12))
            .push(syllable);
    }

    /// 向背景人声追加一条翻译内容
    pub fn push_translation(&mut self, content: SubLyricContent) {
        self.translations.get_or_insert_with(Vec::new).push(content);
    }

    /// 向背景人声追加一条音译内容
    pub fn push_romanization(&mut self, content: SubLyricContent) {
        self.romanizations
            .get_or_insert_with(Vec::new)
            .push(content);
    }

    /// 根据当前的 words 重新构建 text 字段
    ///
    /// 一般用来从逐字歌词生成逐行文本，没有 words 字段则什么也不会做
    pub fn rebuild_text(&mut self) {
        if let Some(words) = &self.words {
            self.text = build_full_text(words, false);
        }
    }

    /// 后处理背景人声及其 [`SubLyricContent`] 的括号、空格与文本拼接
    ///
    /// 包含：
    /// * 移除两侧的括号
    /// * 规范化逐字音节的空格
    /// * 构建逐行文本
    pub fn normalize(&mut self) {
        // 背景人声的主歌词
        if let Some(bg_words) = &mut self.words {
            strip_outer_parens_from_words(bg_words);
            normalize_words_spaces(bg_words);
            self.text = build_full_text(bg_words, false);
        } else {
            strip_outer_parens(&mut self.text);
            normalize_line_text(&mut self.text);
        }

        // 背景人声的翻译
        if let Some(translations) = &mut self.translations {
            for t in translations {
                t.normalize_with_parens(false);
            }
        }

        // 背景人声的音译
        if let Some(romanizations) = &mut self.romanizations {
            for r in romanizations {
                r.normalize_with_parens(true);
            }
        }
    }
}

impl Syllable {
    /// 向音节追加一个 Ruby 注音音节
    pub fn push_ruby(&mut self, tag: RubyTag) {
        self.ruby.get_or_insert_with(Vec::new).push(tag);
    }
}

/// Ruby 标注的单个注音音节
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RubyTag {
    /// 注音文本内容
    pub text: CompactString,

    /// 该注音的开始时间，单位毫秒
    pub start_time: u32,

    /// 该注音的结束时间，单位毫秒
    pub end_time: u32,
}

/// 一个歌词音节
#[skip_serializing_none]
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Syllable {
    ///  该音节的内容
    /// - 如果是普通音节，为常规歌词文本
    /// - 如果是 Ruby 标注，这里对应 ruby 的基文本，通常为汉字
    pub text: CompactString,

    /// 该音节的开始时间，单位毫秒
    /// - 如果是 Ruby 标注，此值为第一个 [`RubyTag`] 的 startTime
    pub start_time: u32,

    /// 该音节的结束时间，单位毫秒
    /// - 如果是 Ruby 标注，此值为最后一个 [`RubyTag`] 的 endTime
    pub end_time: u32,

    /// 该音节后面是否应该跟着一个空格
    ///
    /// 注意必须根据此标志在歌词后面添加空格，text 中不应包含空格
    pub ends_with_space: Option<bool>,

    /// Ruby 标注信息
    ///
    /// 如果存在此属性，说明该音节是一个 Ruby 容器
    pub ruby: Option<Vec<RubyTag>>,

    /// 单词内容是否包含冒犯性的不雅用语
    pub obscene: Option<bool>,

    /// 单词的空拍数量，一般只用于方便歌词打轴
    pub empty_beat: Option<u32>,
}

/// 演唱者信息结构
#[skip_serializing_none]
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Agent {
    /// 演唱者的 ID
    ///
    /// 如果是 AMLL 的 TTML，只有 v1 和 v2 分别指代非对唱和对唱。
    /// 如果是 Apple Music 的 TTML，还会出现 v3、v4 等指代每个演唱者，以及 v1000 用于指代合唱。
    pub id: CompactString,

    /// 演唱者名称
    pub name: Option<CompactString>,

    /// 演唱者类型
    ///
    /// 通常为 "person"、"group"、"other"，也有可能是其他字符串
    pub type_: Option<CompactString>,
}

/// 元数据中的各个平台 ID
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PlatformId {
    /// 网易云音乐 ID
    NcmMusicId,

    /// QQ 音乐 ID
    QqMusicId,

    /// Spotify ID
    SpotifyId,

    /// Apple Music ID
    AppleMusicId,
}

/// 音频相关信息
///
/// 对应 Apple Music TTML 中的以下内容：
/// - `<body dur>`：音频时长
/// - `<iTunesMetadata leadingSilence>`：前导静音
/// - `<iTunesMetadata><audio lyricOffset role>`：音频偏移与音频角色
///
/// 所有时间均以毫秒为单位。字段缺失或取值无法识别时为 `None`。
#[skip_serializing_none]
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioMetadata {
    /// 音频时长，单位毫秒
    ///
    /// 对应 `<body>` 上的 `dur` 属性。
    pub duration: Option<u32>,

    /// 音频偏移，单位毫秒
    ///
    /// 对应 `<audio>` 上的 `lyricOffset` 属性，原始值为秒，可以为负数。
    pub lyric_offset: Option<i32>,

    /// 音频角色
    ///
    /// 对应 `<audio>` 上的 `role` 属性，例如 `"spatial"`。
    /// 本库只保存该值，不解释其具体语义。
    pub role: Option<CompactString>,

    /// 前导静音，单位毫秒
    ///
    /// 对应 `<iTunesMetadata>` 上的 `leadingSilence` 属性，原始值为秒。
    pub leading_silence: Option<u32>,
}

impl AudioMetadata {
    /// 是否所有字段均为 `None`
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.duration.is_none()
            && self.lyric_offset.is_none()
            && self.role.is_none()
            && self.leading_silence.is_none()
    }
}

/// TTML 歌词的元数据内容
#[skip_serializing_none]
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TTMLMetadata {
    /// 歌词主语言代码 (BCP-47)
    pub language: Option<CompactString>,

    /// 计时模式
    pub timing_mode: Option<CompactString>,

    /// 音频相关信息
    ///
    /// 解析自 `<body dur>`、`<iTunesMetadata leadingSilence>` 与 `<audio>`；
    /// 仅在解析到至少一项时存在，且不会通过 AMLL 转换保留。
    pub audio: Option<AudioMetadata>,

    /// 歌曲创作者列表
    pub songwriters: Option<Vec<CompactString>>,

    /// 歌曲标题列表
    pub title: Option<Vec<CompactString>>,

    /// 艺术家名称列表
    pub artist: Option<Vec<CompactString>>,

    /// 专辑名称列表
    pub album: Option<Vec<CompactString>>,

    /// ISRC 号码列表
    pub isrc: Option<Vec<CompactString>>,

    /// 歌词作者 GitHub 数字 ID 列表
    pub author_ids: Option<Vec<CompactString>>,

    /// 歌词作者 GitHub 用户名列表
    pub author_names: Option<Vec<CompactString>>,

    /// 演唱者映射表
    pub agents: Option<IndexMap<CompactString, Agent>>,

    /// 平台关联 ID
    pub platform_ids: Option<IndexMap<PlatformId, Vec<CompactString>>>,

    /// 其他原始的自定义属性
    pub raw_properties: Option<IndexMap<CompactString, Vec<CompactString>>>,
}

impl TTMLMetadata {
    /// 获取或初始化 [`AudioMetadata`] 的可变引用
    pub fn audio_mut(&mut self) -> &mut AudioMetadata {
        self.audio.get_or_insert_with(AudioMetadata::default)
    }

    pub fn push_title(&mut self, title: CompactString) {
        self.title.get_or_insert_with(Vec::new).push(title);
    }

    pub fn push_artist(&mut self, artist: CompactString) {
        self.artist.get_or_insert_with(Vec::new).push(artist);
    }

    pub fn push_album(&mut self, album: CompactString) {
        self.album.get_or_insert_with(Vec::new).push(album);
    }

    pub fn push_songwriter(&mut self, songwriter: CompactString) {
        self.songwriters
            .get_or_insert_with(Vec::new)
            .push(songwriter);
    }

    pub fn push_isrc(&mut self, isrc: CompactString) {
        self.isrc.get_or_insert_with(Vec::new).push(isrc);
    }

    pub fn push_author_id(&mut self, author_id: CompactString) {
        self.author_ids.get_or_insert_with(Vec::new).push(author_id);
    }

    pub fn push_author_name(&mut self, author_name: CompactString) {
        self.author_names
            .get_or_insert_with(Vec::new)
            .push(author_name);
    }

    pub fn insert_agent(&mut self, agent: Agent) {
        self.agents
            .get_or_insert_with(IndexMap::new)
            .insert(agent.id.clone(), agent);
    }

    pub fn push_platform_id(&mut self, platform: PlatformId, id: CompactString) {
        self.platform_ids
            .get_or_insert_with(IndexMap::new)
            .entry(platform)
            .or_default()
            .push(id);
    }

    pub fn push_raw_property(&mut self, key: CompactString, value: CompactString) {
        self.raw_properties
            .get_or_insert_with(IndexMap::new)
            .entry(key)
            .or_default()
            .push(value);
    }
}

impl SubLyricContent {
    /// 根据当前的 words 重新构建 text 字段
    ///
    /// 一般用来从逐字歌词生成逐行文本，没有 words 字段则什么也不会做
    pub fn rebuild_text(&mut self) {
        if let Some(words) = &self.words {
            self.text = build_full_text(words, false);
        }
    }

    /// 规范化翻译/音译内容的空格与文本
    ///
    /// - 如果有逐字音节，规范化音节空格并拼接全文
    /// - 否则，直接规范化行文本
    ///
    /// `space_joined`：为 `true` 时音节间始终插入空格（一般用于连接逐字音译）
    pub fn normalize(&mut self, space_joined: bool) {
        if let Some(words) = &mut self.words {
            normalize_words_spaces(words);
            self.text = build_full_text(words, space_joined);
        } else {
            normalize_line_text(&mut self.text);
        }
    }

    /// 规范化翻译/音译内容，同时移除最外层括号
    ///
    /// * 一般用于背景人声的翻译/音译
    pub fn normalize_with_parens(&mut self, space_joined: bool) {
        if let Some(words) = &mut self.words {
            strip_outer_parens_from_words(words);
            normalize_words_spaces(words);
            self.text = build_full_text(words, space_joined);
        } else {
            strip_outer_parens(&mut self.text);
            normalize_line_text(&mut self.text);
        }
    }
}

/// 解析器返回的结果对象
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TTMLResult {
    /// TTML 歌词的元数据内容
    pub metadata: TTMLMetadata,

    /// 所有的歌词行
    pub lines: Vec<LyricLine>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_core_structs_equality() {
        let res1 = TTMLResult {
            metadata: TTMLMetadata {
                language: Some("zh-CN".into()),
                ..Default::default()
            },
            lines: vec![LyricLine {
                text: "测试".into(),
                start_time: 100,
                end_time: 200,
                words: Some(vec![Syllable {
                    text: "测".into(),
                    start_time: 100,
                    end_time: 150,
                    ..Default::default()
                }]),
                ..Default::default()
            }],
        };

        let res2 = res1.clone();
        assert_eq!(res1, res2);

        let mut res3 = res1.clone();
        res3.lines[0].end_time = 300;
        assert_ne!(res1, res3);
    }
}
