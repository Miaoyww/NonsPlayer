//! AMLL 所使用的较简单的数据结构
//!
//! AMLL 格式不区分翻译/音译内容的来源，因此 TTML 中
//! `<translation>` / `<transliteration>` 的 `automaticallyCreated` 标志
//! 在 TTML 与 AMLL 互转时不会保留。
//!
//! 同理，[`crate::model::TTMLMetadata::audio`] 中的音频元数据也没有对应的
//! AMLL 键值，转换时同样会被丢弃。

mod model;
mod to_amll;
mod to_ttml;

pub use model::*;
pub use to_amll::to_amll_lyrics;
pub use to_ttml::to_ttml_result;
