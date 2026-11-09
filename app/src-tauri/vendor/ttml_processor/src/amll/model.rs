//! AMLL 所使用的较简单的数据结构

use compact_str::CompactString;
use serde::{
    Deserialize,
    Serialize,
};
use serde_with::skip_serializing_none;

/// 一个歌词单词
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LyricWordBase {
    /// 单词的起始时间，单位为毫秒
    pub start_time: u32,

    /// 单词的结束时间，单位为毫秒
    pub end_time: u32,

    /// 单词内容
    pub word: CompactString,
}

/// 一个歌词单词
#[skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AmllLyricWord {
    /// 单词的起始时间，单位为毫秒
    pub start_time: u32,

    /// 单词的结束时间，单位为毫秒
    pub end_time: u32,

    /// 单词内容
    pub word: CompactString,

    /// 单词的音译内容
    pub roman_word: Option<CompactString>,

    /// 单词内容是否包含冒犯性的不雅用语
    pub obscene: Option<bool>,

    /// 单词的空拍数量，一般只用于方便歌词打轴
    pub empty_beat: Option<u32>,

    /// 单词的注音内容
    pub ruby: Option<Vec<LyricWordBase>>,
}

/// 一行歌词，存储多个单词
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AmllLyricLine {
    /// 该行的所有单词
    pub words: Vec<AmllLyricWord>,

    /// 该行的翻译
    pub translated_lyric: String,

    /// 该行的音译
    pub roman_lyric: String,

    /// 该行是否为背景歌词行
    #[serde(rename = "isBG")]
    pub is_bg: bool,

    /// 该行是否为对唱歌词行（即歌词行靠右对齐）
    pub is_duet: bool,

    /// 该行的开始时间
    ///
    /// **并不总是等于第一个单词的开始时间**
    pub start_time: u32,

    /// 该行的结束时间
    ///
    /// **并不总是等于最后一个单词的开始时间**
    pub end_time: u32,
}

/// 一个元数据，以 `[键, 值数组]` 的形式存储。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AmllMetadata {
    pub key: CompactString,
    pub value: Vec<CompactString>,
}

/// 一个 TTML 歌词对象，存储了歌词行信息和 AMLL 元数据信息
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AmllLyricResult {
    /// TTML 中存储的歌词行信息
    #[serde(rename = "lyricLines")]
    pub lines: Vec<AmllLyricLine>,

    /// 一个元数据表，以 `[键, 值数组]` 的形式存储
    pub metadata: Vec<AmllMetadata>,
}

/// 解析器生成的原始 TTML 数据结构转换为 AMLL 的数据结构时的配置选项
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct TtmlToAmllOptions {
    /// 提取翻译时的首选目标语言 (如 `"zh-Hans"`)
    ///
    /// 未提供或找不到指定的目标语言代码时提取第一个翻译
    pub translation_language: Option<String>,

    /// 提取音译时的首选目标语言 (如 `"ja-Latn"`)
    ///
    /// 未提供或找不到指定的目标语言代码时提取第一个音译
    pub romanization_language: Option<String>,
}

/// AMLL 简单的数据结构转换为解析器内部复杂的 TTML 数据结构时的配置选项
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct AmllToTtmlOptions {
    /// 翻译的目标语言代码
    ///
    /// 默认: `zh-Hans`
    pub translation_language: Option<String>,

    /// 音译的目标语言代码
    ///
    /// 默认: `None`
    pub romanization_language: Option<String>,
}
