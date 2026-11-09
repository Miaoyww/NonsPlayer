mod body;
mod ext;
mod head;
mod span;
mod sub_lyric;
mod utils;

use quick_xml::Writer;
use serde::{
    Deserialize,
    Serialize,
};

use crate::{
    constants::{
        attrs,
        tags,
        vals,
    },
    error::Result,
    generator::ext::ElementWriterExt as _,
    model::TTMLResult,
};

/// TTML 生成器配置
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GeneratorConfig {
    /// 是否使用 Apple Music 格式规则
    ///
    /// - `true`:
    ///    - 逐行翻译/音译写入到 `<head>` 中
    ///    - 背景人声在主歌词之前开始会先写入
    ///    - 背景人声容器不包含 `begin` 和 `end` 属性
    /// - `false`:
    ///    - 逐行翻译/音译写入为内嵌 `x-translation` / `x-roman`
    ///
    /// 注意：逐字翻译/音译始终写入到 `<head>` 中
    pub use_apple_format_rules: bool,

    /// 是否输出格式化后的 XML 而不是压缩成一行的
    pub format: bool,

    /// 是否按逐行格式生成歌词
    ///
    /// - `true`:
    ///    - `<tt>` 上写入 `itunes:timing="Line"`，忽略 `metadata.timing_mode`
    ///    - `<p>` 内只写入整行纯文本，不写入逐字 `<span>`
    ///    - 逐字翻译/音译降级为逐行文本，写入位置仍然遵循 `use_apple_format_rules`
    ///    - 不写入背景人声，其翻译/音译也一并不写入
    /// - `false`:
    ///    - 按逐字格式生成
    pub line_timing: bool,
}

/// 将解析后的 TTML 结构体生成为 TTML 字符串
///
/// ## Errors
/// 会在 quick-xml 序列化失败或内容不是 UTF-8 编码时返回错误
pub fn generate_ttml(result: &TTMLResult, config: &GeneratorConfig) -> Result<String> {
    let buffer = Vec::with_capacity(102_400);

    let mut writer = if config.format {
        Writer::new_with_indent(buffer, b' ', 4)
    } else {
        Writer::new(buffer)
    };

    // 逐行模式下强制写入 Line，忽略元数据里记录的计时模式
    let timing_mode = if config.line_timing {
        Some(vals::TIMING_LINE)
    } else {
        result.metadata.timing_mode.as_deref()
    };

    // <tt>
    writer
        .create_element(tags::TT)
        .with_attributes([
            (attrs::XMLNS, vals::NS_TTML),
            (attrs::XMLNS_ITUNES, vals::NS_ITUNES),
            (attrs::XMLNS_TTM, vals::NS_TTM),
            (attrs::XMLNS_TTS, vals::NS_TTS),
            (attrs::XMLNS_AMLL, vals::NS_AMLL),
        ])
        .with_attribute_opt((attrs::ITUNES_TIMING, timing_mode))
        .with_attribute_opt((attrs::XML_LANG, result.metadata.language.as_deref()))
        .write_inner_content(|writer| {
            // <head>
            head::write_head(writer, &result.metadata, &result.lines, config)?;

            // <body>
            body::write_body(
                writer,
                &result.lines,
                result
                    .metadata
                    .audio
                    .as_ref()
                    .and_then(|audio| audio.duration),
                config,
            )?;
            Ok(())
        })?;

    Ok(String::from_utf8(writer.into_inner())?)
}
