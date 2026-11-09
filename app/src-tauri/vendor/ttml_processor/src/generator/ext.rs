use std::io::Write;

use quick_xml::{
    events::attributes::Attribute,
    writer::ElementWriter,
};

pub trait ElementWriterExt<'a, W: Write> {
    fn with_attribute_opt<V>(self, attr: (&'a str, Option<V>)) -> Self
    where
        Attribute<'a>: From<(&'a str, V)>;
}

impl<'a, W: Write> ElementWriterExt<'a, W> for ElementWriter<'a, W> {
    fn with_attribute_opt<V>(self, attr: (&'a str, Option<V>)) -> Self
    where
        Attribute<'a>: From<(&'a str, V)>,
    {
        let (key, value) = attr;
        if let Some(v) = value {
            self.with_attribute((key, v))
        } else {
            self
        }
    }
}
