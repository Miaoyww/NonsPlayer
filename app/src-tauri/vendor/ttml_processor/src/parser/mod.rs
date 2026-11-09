mod body;
mod ext;
mod head;
mod span;
pub mod state;
mod sub_lyric;
mod timestamp;
mod utils;

use quick_xml::{
    Reader,
    events::Event,
};

use crate::{
    constants::{
        attrs,
        tags,
    },
    error::{
        Result,
        TTMLProcessorError,
    },
    model::TTMLResult,
    parser::{
        ext::{
            BytesStartExt as _,
            ReaderExt as _,
        },
        state::ParserContext,
    },
};

/// 解析 TTML 字符串为 [`TTMLResult`]
///
/// ## Errors
/// 会在 TTML 格式不正确时（例如时间戳格式错误、格式损坏等）返回错误
pub fn parse_ttml(ttml_content: &str) -> Result<TTMLResult> {
    let mut reader = Reader::from_str(ttml_content);
    reader.config_mut().expand_empty_elements = true;

    let mut context = ParserContext::default();
    let mut lines = Vec::with_capacity(64);

    loop {
        match reader.read_event_with_context(&context)? {
            Event::Start(ref e) => {
                let tag_name = std::str::from_utf8(e.name().as_ref())
                    .map_err(TTMLProcessorError::Utf8Error)?
                    .to_string();

                context.tag_stack.push(tag_name.clone().into());

                if tag_name == tags::TT {
                    context.metadata.language =
                        e.get_attr_value(attrs::XML_LANG, &reader, &context)?;
                    context.metadata.timing_mode =
                        e.get_attr_value(attrs::ITUNES_TIMING, &reader, &context)?;
                } else if tag_name == tags::HEAD {
                    head::parse_head(&mut reader, &mut context)?;

                    context.tag_stack.pop();
                } else if tag_name == tags::BODY {
                    body::parse_body(&mut reader, e, &mut context, &mut lines)?;

                    context.tag_stack.pop();
                }
            }
            Event::End(_) => {
                context.tag_stack.pop();
            }
            Event::Eof => break,
            _ => (),
        }
    }

    Ok(TTMLResult {
        metadata: context.metadata,
        lines,
    })
}
