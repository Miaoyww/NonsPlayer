use compact_str::CompactString;
use quick_xml::{
    Reader,
    events::{
        BytesStart,
        Event,
        attributes::Attribute,
    },
    name::QName,
};

use crate::{
    error::{
        OptionExt as _,
        ParseErrorKind,
        Result,
        ResultExt as _,
        TTMLResultExt as _,
    },
    parser::state::ParserContext,
};

pub trait QNameExt {
    fn is(&self, tag: &str) -> bool;
}

impl QNameExt for QName<'_> {
    fn is(&self, tag: &str) -> bool {
        self.as_ref() == tag.as_bytes()
    }
}

pub trait BytesStartExt {
    fn get_attr_value(
        &self,
        key: &str,
        reader: &Reader<&[u8]>,
        context: &ParserContext,
    ) -> Result<Option<CompactString>>;

    fn get_required_attr(
        &self,
        key: &str,
        reader: &Reader<&[u8]>,
        context: &ParserContext,
    ) -> Result<CompactString>;

    fn for_each_attr<'a, F>(
        &'a self,
        reader: &Reader<&[u8]>,
        context: &ParserContext,
        tag_name: &str,
        f: F,
    ) -> Result<()>
    where
        F: FnMut(quick_xml::events::attributes::Attribute<'a>) -> Result<()>;
}

impl BytesStartExt for BytesStart<'_> {
    /// 获取指定的属性，未找到则返回 `None`
    ///
    /// 若读取过程中出现 XML 格式错误（AttrError），将注入当前位置和标签栈上下文
    fn get_attr_value(
        &self,
        key: &str,
        reader: &Reader<&[u8]>,
        context: &ParserContext,
    ) -> Result<Option<CompactString>> {
        for attr_result in self.attributes().with_checks(false) {
            let attr = attr_result
                .map_err(ParseErrorKind::from)
                .with_attr_context(reader, context, key)?;

            if attr.key.as_ref() == key.as_bytes() {
                let unescaped = attr
                    .unescape_value()
                    .map_err(|e| ParseErrorKind::EntityError(e.to_string().into()))
                    .with_attr_context(reader, context, key)
                    .with_offending_bytes(&attr.value)?;

                return Ok(Some(CompactString::from(&*unescaped)));
            }
        }
        Ok(None)
    }

    /// 获取指定的必填属性
    ///
    /// 若未找到，返回带 [`ParserContext`] 的 `MissingAttribute` 错误
    fn get_required_attr(
        &self,
        key: &str,
        reader: &Reader<&[u8]>,
        context: &ParserContext,
    ) -> Result<CompactString> {
        self.get_attr_value(key, reader, context)?
            .context_missing_attr(reader, context, key)
    }

    /// 提供一个闭包一次性遍历所有属性，并自动处理底层的错误转换与上下文注入
    fn for_each_attr<'a, F>(
        &'a self,
        reader: &Reader<&[u8]>,
        context: &ParserContext,
        tag_name: &str,
        mut f: F,
    ) -> Result<()>
    where
        F: FnMut(Attribute<'a>) -> Result<()>,
    {
        for attr_res in self.attributes().with_checks(false) {
            let attr = attr_res
                .map_err(ParseErrorKind::from)
                .with_attr_context(reader, context, tag_name)?;
            f(attr)?;
        }
        Ok(())
    }
}

pub trait ReaderExt<'a> {
    /// 封装底层的切片 `read_event`，并在发生 XML 语法错误时自动注入上下文
    fn read_event_with_context(&mut self, context: &ParserContext) -> Result<Event<'a>>;
}

impl<'a> ReaderExt<'a> for Reader<&'a [u8]> {
    fn read_event_with_context(&mut self, context: &ParserContext) -> Result<Event<'a>> {
        let event = self.read_event().map_err(ParseErrorKind::XmlError);
        event.with_context(self, context)
    }
}
