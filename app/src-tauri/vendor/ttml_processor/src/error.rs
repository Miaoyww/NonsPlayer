use std::{
    result::Result as StdResult,
    str::Utf8Error,
    string::FromUtf8Error,
};

use compact_str::CompactString;
use quick_xml::{
    Reader,
    events::attributes::AttrError,
};
use thiserror::Error;

use crate::parser::state::ParserContext;

/// 记录解析错误发生时的上下文信息
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ErrorContext {
    /// 错误发生时解析器在文件中的字节偏移量
    pub byte_offset: u64,
    /// 当前解析到的歌词行 ID（例如 "L3"），如果尚未解析到则为 None
    pub line_id: Option<CompactString>,
    /// 当前的 XML 标签路径栈（例如 `["tt", "body", "div", "p", "span"]`）
    pub tag_stack: Vec<CompactString>,
    /// 正在处理的属性名
    pub current_attribute: Option<CompactString>,
    /// 引发错误的具体原文字符串
    pub offending_string: Option<CompactString>,
}

#[derive(Error, Debug)]
pub enum ParseErrorKind {
    #[error("XML attribute error: {0}")]
    AttrError(#[from] AttrError),

    #[error("Unknown XML entity: {0}")]
    EntityError(CompactString),

    #[error("Invalid timestamp format: {0}")]
    InvalidTimestamp(CompactString),

    #[error("Missing required attribute: {0}")]
    MissingAttribute(CompactString),

    #[error("Unexpected end of file")]
    UnexpectedEof,

    #[error("XML parsing error: {0}")]
    XmlError(#[from] quick_xml::Error),
}

#[derive(Error, Debug)]
pub enum TTMLProcessorError {
    #[error("{kind} (at byte offset {0})", .context.byte_offset)]
    ParseError {
        #[source]
        kind: ParseErrorKind,
        context: Box<ErrorContext>,
    },

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("UTF-8 decoding error: {0}")]
    Utf8Error(#[from] Utf8Error),

    #[error("UTF-8 from bytes error: {0}")]
    FromUtf8Error(#[from] FromUtf8Error),
}

// 手动实现转换以方便从 quick-xml 的 write_inner_content 返回的 io::Error 转换为我们自己的错误枚举
impl From<TTMLProcessorError> for std::io::Error {
    fn from(err: TTMLProcessorError) -> Self {
        Self::other(err)
    }
}

pub type Result<T> = StdResult<T, TTMLProcessorError>;

/// 错误上下文扩展，用于将 [`ParseErrorKind`] 转换为包含上下文信息的 [`TTMLProcessorError`]
pub(crate) trait ResultExt<T> {
    /// 注入基础上下文 (基于当前读取器位置和解析状态)
    ///
    /// # Errors
    ///
    /// 当前置函数返回 `Err(ParseErrorKind)` 时，返回带有上下文信息的
    /// [`TTMLProcessorError::ParseError`]
    fn with_context(
        self,
        reader: &Reader<&[u8]>,
        context: &ParserContext,
    ) -> StdResult<T, TTMLProcessorError>;

    /// 注入带有特定属性名的上下文
    ///
    /// # Errors
    ///
    /// 当前置函数返回 `Err(ParseErrorKind)` 时，返回带有属性名与上下文信息的
    /// [`TTMLProcessorError::ParseError`]
    fn with_attr_context(
        self,
        reader: &Reader<&[u8]>,
        context: &ParserContext,
        attr_name: &str,
    ) -> StdResult<T, TTMLProcessorError>;
}

impl<T> ResultExt<T> for StdResult<T, ParseErrorKind> {
    fn with_context(
        self,
        reader: &Reader<&[u8]>,
        context: &ParserContext,
    ) -> StdResult<T, TTMLProcessorError> {
        self.map_err(|kind| TTMLProcessorError::ParseError {
            kind,
            context: Box::new(ErrorContext {
                byte_offset: reader.buffer_position(),
                line_id: context.current_line_id.clone(),
                tag_stack: context.tag_stack.clone(),
                current_attribute: None,
                offending_string: None,
            }),
        })
    }

    fn with_attr_context(
        self,
        reader: &Reader<&[u8]>,
        context: &ParserContext,
        attr_name: &str,
    ) -> StdResult<T, TTMLProcessorError> {
        self.map_err(|kind| TTMLProcessorError::ParseError {
            kind,
            context: Box::new(ErrorContext {
                byte_offset: reader.buffer_position(),
                line_id: context.current_line_id.clone(),
                tag_stack: context.tag_stack.clone(),
                current_attribute: Some(attr_name.into()),
                offending_string: None,
            }),
        })
    }
}

pub(crate) trait OptionExt<T> {
    /// 将缺失属性错误转换为带上下文的解析错误。
    ///
    /// # Errors
    ///
    /// 当 `Option` 为 `None` 时，返回 [`TTMLProcessorError::ParseError`]，其错误类型为
    /// [`ParseErrorKind::MissingAttribute`]
    fn context_missing_attr(
        self,
        reader: &Reader<&[u8]>,
        context: &ParserContext,
        attr_name: &str,
    ) -> StdResult<T, TTMLProcessorError>;
}

impl<T> OptionExt<T> for Option<T> {
    fn context_missing_attr(
        self,
        reader: &Reader<&[u8]>,
        context: &ParserContext,
        attr_name: &str,
    ) -> StdResult<T, TTMLProcessorError> {
        self.ok_or_else(|| ParseErrorKind::MissingAttribute(attr_name.into()))
            .with_attr_context(reader, context, attr_name)
    }
}

pub(crate) trait TimestampExt<T> {
    /// 如果时间戳解析失败，利用传入的原始 bytes 构造 [`ParseErrorKind::InvalidTimestamp`] 错误
    ///
    /// # Errors
    ///
    /// 在出错时构造错误
    fn context_invalid_timestamp(self, bytes: &[u8]) -> StdResult<T, ParseErrorKind>;
}

/// 已包装错误的上下文扩展，用于在 [`TTMLProcessorError::ParseError`] 上追加信息
pub(crate) trait TTMLResultExt<T> {
    /// 为已有的解析错误注入引发错误的原文字符串（从字节切片进行 UTF-8 损失转换）
    fn with_offending_bytes(self, bytes: &[u8]) -> StdResult<T, TTMLProcessorError>;
}

impl<T> TTMLResultExt<T> for StdResult<T, TTMLProcessorError> {
    fn with_offending_bytes(self, bytes: &[u8]) -> Self {
        self.map_err(|mut e| {
            if let TTMLProcessorError::ParseError {
                ref mut context, ..
            } = e
            {
                let s: CompactString = String::from_utf8_lossy(bytes).into_owned().into();
                context.offending_string = Some(s);
            }
            e
        })
    }
}

impl<T> TimestampExt<T> for Option<T> {
    fn context_invalid_timestamp(self, bytes: &[u8]) -> StdResult<T, ParseErrorKind> {
        self.ok_or_else(|| {
            let err_str = String::from_utf8_lossy(bytes);
            ParseErrorKind::InvalidTimestamp(err_str.as_ref().into())
        })
    }
}
