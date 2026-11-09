//! 解析器内部使用的相关状态

use std::collections::HashMap;

use compact_str::CompactString;

use crate::model::{
    SubLyricContent,
    TTMLMetadata,
};

/// 解析器的内部状态，用于暂存跨标签的数据
#[derive(Debug, Default)]
pub struct ParserContext {
    /// 解析出来的元数据内容
    pub metadata: TTMLMetadata,

    /// 主歌词的翻译
    pub translations_map: HashMap<CompactString, Vec<SubLyricContent>>,
    /// 主歌词的音译
    pub romanizations_map: HashMap<CompactString, Vec<SubLyricContent>>,

    /// 背景人声的翻译
    pub bg_translations_map: HashMap<CompactString, Vec<SubLyricContent>>,
    /// 背景人声的音译
    pub bg_romanizations_map: HashMap<CompactString, Vec<SubLyricContent>>,

    /// 当前的递增区块索引
    pub current_block_index: u32,
    /// 上次遇到的歌曲组成部分名称
    pub last_song_part: Option<CompactString>,

    /// 当前的 XML 标签路径栈 (例如 `["tt", "body", "div", "p", "span"]`)
    pub tag_stack: Vec<CompactString>,
    /// 当前解析到的歌词行 ID (对应 itunes:key)，离开 <p> 标签时应清空
    pub current_line_id: Option<CompactString>,
    /// 是否已经遇到过 `<iTunesMetadata>` 下的 `<audio>`，仅第一个生效
    pub seen_audio_element: bool,
}
