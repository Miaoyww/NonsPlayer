use compact_str::CompactString;
use quick_xml::{
    Reader,
    events::{
        BytesStart,
        Event,
    },
};

use crate::{
    constants::{
        attrs,
        meta_keys,
        tags,
    },
    error::{
        Result,
        TTMLProcessorError,
    },
    model::{
        Agent,
        PlatformId,
    },
    parser::{
        ext::{
            BytesStartExt as _,
            QNameExt as _,
        },
        state::ParserContext,
        sub_lyric::{
            SubLyricType,
            parse_sub_lyrics,
        },
        timestamp::parse_decimal_seconds,
        utils::{
            Walk,
            read_text_content,
            walk_inner,
        },
    },
};

/// 解析 `<head>` 标签
///
/// ## 示例
/// ```xml
/// <head>
///     <metadata>
///         <ttm:agent type="person" xml:id="v1" />
///         <ttm:agent type="person" xml:id="v2" />
///         <ttm:agent type="person" xml:id="v3" />
///         <ttm:agent type="group" xml:id="v4" />
///         <amll:meta key="title" value="We Don't Talk About Bruno" />
///         <iTunesMetadata xmlns="http://music.apple.com/lyric-ttml-internal">
///             <translations>
///                 <translation type="subtitle" xml:lang="zh-Hans">
///                     翻译内容...
///                 </translation>
///             </translations>
///             <transliterations>
///                 <transliteration>
///                     音译内容...
///                 </transliterations>
///             </transliterations>
///             <songwriters>
///                 <songwriter>Lin-Manuel Miranda</songwriter>
///             </songwriters>
///         </iTunesMetadata>
///     </metadata>
/// </head>
/// ```
pub fn parse_head(reader: &mut Reader<&[u8]>, context: &mut ParserContext) -> Result<()> {
    walk_inner(reader, context, tags::HEAD, |reader, context, event| {
        let Event::Start(e) = event else {
            return Ok(Walk::Continue);
        };
        let qname = e.name();
        let tag_name =
            std::str::from_utf8(qname.as_ref()).map_err(TTMLProcessorError::Utf8Error)?;

        match tag_name {
            tags::TRANSLATIONS => {
                parse_sub_lyrics(
                    reader,
                    context,
                    tags::TRANSLATIONS,
                    tags::TRANSLATION,
                    SubLyricType::Translation,
                )?;
                Ok(Walk::Handled)
            }
            tags::TRANSLITERATIONS => {
                parse_sub_lyrics(
                    reader,
                    context,
                    tags::TRANSLITERATIONS,
                    tags::TRANSLITERATION,
                    SubLyricType::Transliteration,
                )?;
                Ok(Walk::Handled)
            }

            tags::TTM_AGENT => {
                parse_agent(reader, &e, context)?;
                Ok(Walk::Handled)
            }

            tags::SONGWRITERS => {
                parse_songwriters(reader, context)?;
                Ok(Walk::Handled)
            }

            tags::TTM_TITLE => {
                let title = read_text_content(reader, context, tags::TTM_TITLE)?;
                let trimmed = title.trim();
                if !trimmed.is_empty() {
                    context.metadata.push_title(trimmed.into());
                }
                Ok(Walk::Handled)
            }

            tags::AMLL_META => {
                if let (Ok(key), Ok(value)) = (
                    e.get_required_attr(attrs::KEY, reader, context),
                    e.get_required_attr(attrs::VALUE, reader, context),
                ) {
                    parse_amll_meta(context, &key, &value);
                }
                Ok(Walk::Continue)
            }

            tags::ITUNES_METADATA => {
                if let Some(value) = e.get_attr_value(attrs::LEADING_SILENCE, reader, context)?
                    && let Some(ms) = parse_decimal_seconds(value.trim().as_bytes())
                    && let Ok(ms) = u32::try_from(ms)
                {
                    context.metadata.audio_mut().leading_silence = Some(ms);
                }
                Ok(Walk::Continue)
            }

            tags::AUDIO => {
                parse_audio_element(reader, &e, context)?;
                Ok(Walk::Continue)
            }
            _ => Ok(Walk::Continue),
        }
    })
}

/// 解析 `<iTunesMetadata>` 下的 `<audio>` 标签
///
/// ## 示例
/// ```xml
/// <iTunesMetadata leadingSilence="0.040">
///     <audio lyricOffset="-0.271" role="spatial" />
/// </iTunesMetadata>
/// ```
///
/// 仅保留文档中第一个 `<audio>`，且只处理 `<iTunesMetadata>` 的直接子元素；
/// `role` 为空串时按缺失处理，其他无法识别的属性一律忽略。
fn parse_audio_element(
    reader: &Reader<&[u8]>,
    e: &BytesStart,
    context: &mut ParserContext,
) -> Result<()> {
    let is_direct_child = context
        .tag_stack
        .len()
        .checked_sub(2)
        .and_then(|idx| context.tag_stack.get(idx))
        .is_some_and(|parent| parent.as_str() == tags::ITUNES_METADATA);

    if !is_direct_child || context.seen_audio_element {
        return Ok(());
    }
    context.seen_audio_element = true;

    let role = e
        .get_attr_value(attrs::ROLE, reader, context)?
        .and_then(|value| {
            let value = value.trim();
            (!value.is_empty()).then(|| CompactString::from(value))
        });

    let lyric_offset = e
        .get_attr_value(attrs::LYRIC_OFFSET, reader, context)?
        .and_then(|value| parse_decimal_seconds(value.trim().as_bytes()))
        .and_then(|ms| i32::try_from(ms).ok());

    if role.is_some() || lyric_offset.is_some() {
        let audio = context.metadata.audio_mut();
        audio.role = role;
        audio.lyric_offset = lyric_offset;
    }

    Ok(())
}

/// 解析 `<ttm:agent>` 标签
///
/// ## 示例
/// ```xml
/// <ttm:agent type="person" xml:id="v1" />
/// ```
///
/// 带演唱者名称：
/// ```xml
/// <ttm:agent type="person" xml:id="v1">
///     <ttm:name type="full">Ryan Gosling</ttm:name>
/// </ttm:agent>
/// ```
fn parse_agent(
    reader: &mut Reader<&[u8]>,
    e: &BytesStart,
    context: &mut ParserContext,
) -> Result<()> {
    let id = e.get_required_attr(attrs::XML_ID, reader, context)?;
    let type_ = e.get_attr_value(attrs::TYPE, reader, context)?;
    let mut name = None;

    walk_inner(
        reader,
        context,
        tags::TTM_AGENT,
        |reader, context, event| {
            if let Event::Start(e) = event
                && e.name().is(tags::TTM_NAME)
            {
                name = Some(read_text_content(reader, context, tags::TTM_NAME)?);
                return Ok(Walk::Handled);
            }
            Ok(Walk::Continue)
        },
    )?;

    let agent = Agent { id, name, type_ };

    context.metadata.insert_agent(agent);

    Ok(())
}

fn parse_amll_meta(context: &mut ParserContext, key: &str, value: &str) {
    let meta = &mut context.metadata;
    let val: CompactString = value.trim().into();
    if val.is_empty() {
        return;
    }

    match key {
        meta_keys::MUSIC_NAME => meta.push_title(val),
        meta_keys::ARTISTS => meta.push_artist(val),
        meta_keys::ALBUM => meta.push_album(val),
        meta_keys::ISRC => meta.push_isrc(val),
        meta_keys::GITHUB_ID => meta.push_author_id(val),
        meta_keys::GITHUB_USER_NAME => meta.push_author_name(val),

        meta_keys::NCM_ID | meta_keys::QQ_ID | meta_keys::SPOTIFY_ID | meta_keys::APPLE_ID => {
            let platform = match key {
                meta_keys::NCM_ID => PlatformId::NcmMusicId,
                meta_keys::QQ_ID => PlatformId::QqMusicId,
                meta_keys::SPOTIFY_ID => PlatformId::SpotifyId,
                meta_keys::APPLE_ID => PlatformId::AppleMusicId,
                _ => unreachable!(),
            };
            meta.push_platform_id(platform, val);
        }

        _ => {
            meta.push_raw_property(key.into(), val);
        }
    }
}

/// 解析 `<songwriters>` 标签
///
/// ## 示例
/// ```xml
/// <songwriters>
///     <songwriter>Lin-Manuel Miranda</songwriter>
/// </songwriters>
/// ```
fn parse_songwriters(reader: &mut Reader<&[u8]>, context: &mut ParserContext) -> Result<()> {
    walk_inner(
        reader,
        context,
        tags::SONGWRITERS,
        |reader, context, event| {
            if let Event::Start(e) = event
                && e.name().is(tags::SONGWRITER)
            {
                let text = read_text_content(reader, context, tags::SONGWRITER)?;
                let text: CompactString = text.trim().into();
                if !text.is_empty() {
                    context.metadata.push_songwriter(text);
                }
                return Ok(Walk::Handled);
            }
            Ok(Walk::Continue)
        },
    )
}

#[cfg(test)]
mod tests {
    use quick_xml::{
        Reader,
        events::Event,
    };

    use super::*;
    use crate::parser::state::ParserContext;

    fn advance_to_start_tag(reader: &mut Reader<&[u8]>, tag_name: &str) {
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) if e.name().is(tag_name) => break,
                Ok(Event::Eof) => panic!("Reached EOF before finding tag: {tag_name}"),
                _ => (),
            }
            buf.clear();
        }
    }

    #[test]
    fn test_parse_amll_meta() {
        let mut context = ParserContext::default();

        parse_amll_meta(&mut context, meta_keys::MUSIC_NAME, "测试歌曲");
        parse_amll_meta(&mut context, meta_keys::ARTISTS, "测试歌手");

        parse_amll_meta(&mut context, meta_keys::NCM_ID, "12345");
        parse_amll_meta(&mut context, meta_keys::APPLE_ID, "67890");

        parse_amll_meta(&mut context, "customKey", "customValue");

        assert_eq!(context.metadata.title.as_ref().unwrap()[0], "测试歌曲");
        assert_eq!(context.metadata.artist.as_ref().unwrap()[0], "测试歌手");

        let platforms = context.metadata.platform_ids.as_ref().unwrap();
        assert_eq!(platforms.get(&PlatformId::NcmMusicId).unwrap()[0], "12345");
        assert_eq!(
            platforms.get(&PlatformId::AppleMusicId).unwrap()[0],
            "67890"
        );

        let raw_props = context.metadata.raw_properties.as_ref().unwrap();
        assert_eq!(raw_props.get("customKey").unwrap()[0], "customValue");
    }

    #[test]
    fn test_parse_songwriters() {
        let xml = r"
        <songwriters>
            <songwriter>Lin-Manuel Miranda</songwriter>
            <songwriter>Germaine Franco</songwriter>
            <songwriter>   </songwriter> 
        </songwriters>";

        let mut reader = Reader::from_str(xml);
        let mut context = ParserContext::default();

        advance_to_start_tag(&mut reader, tags::SONGWRITERS);

        parse_songwriters(&mut reader, &mut context).expect("Failed to parse songwriters");

        let writers = context.metadata.songwriters.as_ref().unwrap();
        assert_eq!(writers.len(), 2);
        assert_eq!(writers[0], "Lin-Manuel Miranda");
        assert_eq!(writers[1], "Germaine Franco");
    }

    #[test]
    fn test_parse_agent() {
        let xml = r#"
        <ttm:agent type="person" xml:id="v1">
            <ttm:name type="full">Ryan Gosling</ttm:name>
        </ttm:agent>"#;

        let mut reader = Reader::from_str(xml);
        let mut context = ParserContext::default();

        let mut buf = Vec::new();
        let start_event = loop {
            match reader.read_event_into(&mut buf).unwrap() {
                Event::Start(e) if e.name().is(tags::TTM_AGENT) => {
                    break e.into_owned();
                }
                _ => (),
            }
        };

        parse_agent(&mut reader, &start_event, &mut context).expect("Failed to parse agent");

        let agents = context.metadata.agents.as_ref().unwrap();
        assert!(agents.contains_key("v1"));

        let agent = agents.get("v1").unwrap();
        assert_eq!(agent.id, "v1");
        assert_eq!(agent.type_.as_deref(), Some("person"));
        assert_eq!(agent.name.as_deref(), Some("Ryan Gosling"));
    }

    #[test]
    fn test_parse_head() {
        let xml = r#"
        <head>
            <metadata>
                <ttm:title>We Don't Talk About Bruno</ttm:title>
                <amll:meta key="album" value="Encanto" />
            </metadata>
        </head>"#;

        let mut reader = Reader::from_str(xml);

        reader.config_mut().expand_empty_elements = true;

        let mut context = ParserContext::default();

        advance_to_start_tag(&mut reader, tags::HEAD);

        parse_head(&mut reader, &mut context).expect("Failed to parse head");

        assert_eq!(
            context.metadata.title.as_ref().unwrap()[0],
            "We Don't Talk About Bruno"
        );
        assert_eq!(context.metadata.album.as_ref().unwrap()[0], "Encanto");
    }

    #[test]
    fn test_parse_audio_metadata() {
        let xml = r#"
        <head>
            <metadata>
                <iTunesMetadata leadingSilence="0.040">
                    <audio lyricOffset="-0.271" role="spatial" />
                </iTunesMetadata>
            </metadata>
        </head>"#;

        let mut reader = Reader::from_str(xml);
        reader.config_mut().expand_empty_elements = true;

        let mut context = ParserContext::default();

        advance_to_start_tag(&mut reader, tags::HEAD);

        parse_head(&mut reader, &mut context).expect("Failed to parse head");

        let audio = context.metadata.audio.as_ref().unwrap();
        assert_eq!(audio.leading_silence, Some(40));
        assert_eq!(audio.lyric_offset, Some(-271));
        assert_eq!(audio.role.as_deref(), Some("spatial"));
    }

    #[test]
    fn test_parse_audio_element_outside_itunes_metadata() {
        let xml = r#"
        <head>
            <metadata>
                <audio lyricOffset="0.5" role="spatial" />
            </metadata>
        </head>"#;

        let mut reader = Reader::from_str(xml);
        reader.config_mut().expand_empty_elements = true;

        let mut context = ParserContext::default();

        advance_to_start_tag(&mut reader, tags::HEAD);

        parse_head(&mut reader, &mut context).expect("Failed to parse head");

        assert!(context.metadata.audio.is_none());
    }
}
