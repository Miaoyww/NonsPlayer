use std::collections::BTreeSet;

use compact_str::CompactString;
use quick_xml::{
    Writer,
    events::BytesText,
};

use crate::{
    constants::{
        attrs,
        meta_keys,
        tags,
        vals,
    },
    error::Result,
    generator::{
        GeneratorConfig,
        ext::ElementWriterExt as _,
        sub_lyric::{
            SubLyricKind,
            has_any_itunes_sub_lyrics,
            write_sub_lyrics,
        },
        utils::format_seconds,
    },
    model::{
        AudioMetadata,
        LyricLine,
        PlatformId,
        TTMLMetadata,
    },
};

/// 写入 `<head>` 部分
pub fn write_head(
    writer: &mut Writer<Vec<u8>>,
    metadata: &TTMLMetadata,
    lines: &[LyricLine],
    config: &GeneratorConfig,
) -> Result<()> {
    writer
        .create_element(tags::HEAD)
        .write_inner_content(|writer| {
            writer
                .create_element(tags::METADATA)
                .write_inner_content(|writer| {
                    // 写入 <ttm:agent>
                    write_agents(writer, metadata, lines)?;

                    // 写入 amll:meta 元数据
                    write_amll_metas(writer, metadata)?;

                    let has_songwriters = metadata
                        .songwriters
                        .as_deref()
                        .is_some_and(|sw| !sw.is_empty());

                    let has_sub_lyrics = has_any_itunes_sub_lyrics(lines, config);

                    let audio = metadata.audio.as_ref();
                    let has_audio_element = audio
                        .is_some_and(|audio| audio.lyric_offset.is_some() || audio.role.is_some());
                    let leading_silence = audio
                        .and_then(|audio| audio.leading_silence)
                        .map(|ms| format_seconds(i64::from(ms)));

                    if has_songwriters
                        || has_sub_lyrics
                        || has_audio_element
                        || leading_silence.is_some()
                    {
                        writer
                            .create_element(tags::ITUNES_METADATA)
                            .with_attribute((attrs::XMLNS, vals::NS_ITUNES))
                            .with_attribute_opt((
                                attrs::LEADING_SILENCE,
                                leading_silence.as_deref(),
                            ))
                            .write_inner_content(|writer| {
                                // 写入翻译
                                write_sub_lyrics(writer, lines, SubLyricKind::Translation, config)?;
                                // 写入音译
                                write_sub_lyrics(
                                    writer,
                                    lines,
                                    SubLyricKind::Transliteration,
                                    config,
                                )?;

                                // 写入 songwriters
                                if let Some(songwriters) = &metadata.songwriters
                                    && !songwriters.is_empty()
                                {
                                    writer
                                        .create_element(tags::SONGWRITERS)
                                        .write_inner_content(|writer| {
                                            for sw in songwriters {
                                                writer
                                                    .create_element(tags::SONGWRITER)
                                                    .write_text_content(BytesText::new(sw))?;
                                            }
                                            Ok(())
                                        })?;
                                }

                                // 写入音频偏移与角色
                                if has_audio_element {
                                    write_audio(writer, audio.expect("音频元素字段已确认存在"))?;
                                }

                                Ok(())
                            })?;
                    }

                    Ok(())
                })?;
            Ok(())
        })?;

    Ok(())
}

/// 写入 `<audio>` 元素
///
/// `lyricOffset` 在前、`role` 在后，与 Apple Music 的输出一致
fn write_audio(writer: &mut Writer<Vec<u8>>, audio: &AudioMetadata) -> Result<()> {
    let lyric_offset = audio.lyric_offset.map(|ms| format_seconds(i64::from(ms)));

    writer
        .create_element(tags::AUDIO)
        .with_attribute_opt((attrs::LYRIC_OFFSET, lyric_offset.as_deref()))
        .with_attribute_opt((attrs::ROLE, audio.role.as_deref()))
        .write_empty()?;

    Ok(())
}

fn write_agents(
    writer: &mut Writer<Vec<u8>>,
    metadata: &TTMLMetadata,
    lines: &[LyricLine],
) -> Result<()> {
    if let Some(agents) = &metadata.agents {
        for agent in agents.values() {
            let mut elem = writer.create_element(tags::TTM_AGENT);
            if let Some(type_) = agent.type_.as_deref() {
                elem = elem.with_attribute((attrs::TYPE, type_));
            }
            elem = elem.with_attribute((attrs::XML_ID, agent.id.as_str()));

            if let Some(name) = &agent.name {
                elem.write_inner_content(|writer| {
                    writer
                        .create_element(tags::TTM_NAME)
                        .with_attribute((attrs::TYPE, vals::NAME_FULL))
                        .write_text_content(BytesText::new(name))?;
                    Ok(())
                })?;
            } else {
                elem.write_empty()?;
            }
        }
    } else {
        write_fallback_agents(writer, lines)?;
    }

    Ok(())
}

/// 从行级的演唱者 ID 写入 `<ttm:agent>` 标签，如果全部行都没有演唱者 ID，则生成一个默认的 v1 演唱者
fn write_fallback_agents(writer: &mut Writer<Vec<u8>>, lines: &[LyricLine]) -> Result<()> {
    let mut collected_agent_ids = BTreeSet::new();
    for line in lines {
        if let Some(agent_id) = &line.agent_id {
            collected_agent_ids.insert(agent_id.as_str());
        }
    }

    if collected_agent_ids.is_empty() {
        writer
            .create_element(tags::TTM_AGENT)
            .with_attributes([(attrs::TYPE, "person"), (attrs::XML_ID, "v1")])
            .write_empty()?;
    } else {
        for agent_id in collected_agent_ids {
            writer
                .create_element(tags::TTM_AGENT)
                .with_attributes([(attrs::TYPE, "person"), (attrs::XML_ID, agent_id)])
                .write_empty()?;
        }
    }

    Ok(())
}

/// 写入 amll:meta 元数据
fn write_amll_metas(writer: &mut Writer<Vec<u8>>, metadata: &TTMLMetadata) -> Result<()> {
    let mut write_meta_list = |key: &str, values: Option<&[CompactString]>| -> Result<()> {
        for value in values.into_iter().flatten() {
            writer
                .create_element(tags::AMLL_META)
                .with_attributes([(attrs::KEY, key), (attrs::VALUE, value.as_str())])
                .write_empty()?;
        }
        Ok(())
    };

    write_meta_list(meta_keys::MUSIC_NAME, metadata.title.as_deref())?;
    write_meta_list(meta_keys::ARTISTS, metadata.artist.as_deref())?;
    write_meta_list(meta_keys::ALBUM, metadata.album.as_deref())?;
    write_meta_list(meta_keys::ISRC, metadata.isrc.as_deref())?;

    for (platform, ids) in metadata.platform_ids.iter().flatten() {
        let key = match platform {
            PlatformId::NcmMusicId => meta_keys::NCM_ID,
            PlatformId::QqMusicId => meta_keys::QQ_ID,
            PlatformId::SpotifyId => meta_keys::SPOTIFY_ID,
            PlatformId::AppleMusicId => meta_keys::APPLE_ID,
        };
        write_meta_list(key, Some(ids))?;
    }

    write_meta_list(meta_keys::GITHUB_ID, metadata.author_ids.as_deref())?;
    write_meta_list(
        meta_keys::GITHUB_USER_NAME,
        metadata.author_names.as_deref(),
    )?;

    if let Some(raw_props) = &metadata.raw_properties {
        for (key, values) in raw_props {
            write_meta_list(key, Some(values))?;
        }
    }

    Ok(())
}
