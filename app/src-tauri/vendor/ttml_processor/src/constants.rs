//! TTML 解析和生成器内部使用的常量定义

#![allow(clippy::string_lit_as_bytes)]
#![allow(dead_code)]

macro_rules! define_const_strs_and_bytes {
    ($($name:ident = $value:literal;)*) => {
        $(
            pub const $name: &str = $value;
        )*

        pub mod b {
            $(
                pub const $name: &[u8] = $value.as_bytes();
            )*
        }
    };
}

pub mod tags {
    define_const_strs_and_bytes! {
        TT = "tt";
        HEAD = "head";
        BODY = "body";
        DIV = "div";
        P = "p";
        SPAN = "span";
        METADATA = "metadata";

        TRANSLATIONS = "translations";
        TRANSLITERATIONS = "transliterations";
        TRANSLATION = "translation";
        TRANSLITERATION = "transliteration";

        TTM_AGENT = "ttm:agent";
        TTM_NAME = "ttm:name";
        TTM_TITLE = "ttm:title";
        AMLL_META = "amll:meta";

        SONGWRITERS = "songwriters";
        SONGWRITER = "songwriter";
        TEXT = "text";
        ITUNES_METADATA = "iTunesMetadata";
        AUDIO = "audio";
    }
}

pub mod attrs {
    define_const_strs_and_bytes! {
        XMLNS = "xmlns";
        XMLNS_TTM = "xmlns:ttm";
        XMLNS_TTS = "xmlns:tts";
        XMLNS_ITUNES = "xmlns:itunes";
        XMLNS_AMLL = "xmlns:amll";

        XML_LANG = "xml:lang";
        XML_ID = "xml:id";
        ITUNES_TIMING = "itunes:timing";
        ITUNES_SONGPART = "itunes:songPart";
        ITUNES_SONGPART_KEBAB = "itunes:song-part";
        ITUNES_KEY = "itunes:key";

        TTM_AGENT = "ttm:agent";
        TTM_ROLE = "ttm:role";
        TTS_RUBY = "tts:ruby";

        AMLL_OBSCENE = "amll:obscene";
        AMLL_EMPTY_BEAT = "amll:empty-beat";

        BEGIN = "begin";
        END = "end";
        DUR = "dur";
        LEADING_SILENCE = "leadingSilence";
        LYRIC_OFFSET = "lyricOffset";
        ROLE = "role";
        TYPE = "type";
        AUTOMATICALLY_CREATED = "automaticallyCreated";
        FOR = "for";
        KEY = "key";
        VALUE = "value";
    }
}

pub mod vals {
    define_const_strs_and_bytes! {
        ROLE_BG = "x-bg";
        ROLE_TRANS = "x-translation";
        ROLE_ROM = "x-roman";

        RUBY_CONTAINER = "container";
        RUBY_BASE = "base";
        RUBY_TEXT_CONTAINER = "textContainer";
        RUBY_TEXT = "text";

        TIMING_LINE = "Line";

        TRUE_STR = "true";

        NS_TTML = "http://www.w3.org/ns/ttml";
        NS_TTM = "http://www.w3.org/ns/ttml#metadata";
        NS_TTS = "http://www.w3.org/ns/ttml#styling";
        NS_ITUNES = "http://music.apple.com/lyric-ttml-internal";
        NS_AMLL = "http://www.example.com/ns/amll";

        PERSON = "person";
        GROUP = "group";
        OTHER = "other";
        AGENT_DEFAULT = "v1";
        AGENT_DEFAULT_DUET = "v2";

        TRANSLATION_DEFAULT_LANGUAGE = "zh-Hans";
        TRANSLATION_SUBTITLE = "subtitle";
        TRANSLATION_REPLACEMENT = "replacement";

        NAME_FULL = "full";
    }
}

pub mod meta_keys {
    define_const_strs_and_bytes! {
        LANGUAGE = "language";
        TIMING_MODE = "timingMode";

        TITLE = "title";
        MUSIC_NAME = "musicName";
        ARTISTS = "artists";
        SONGWRITERS = "songwriters";
        ALBUM = "album";

        ISRC = "isrc";
        GITHUB_ID = "ttmlAuthorGithub";
        GITHUB_USER_NAME = "ttmlAuthorGithubLogin";

        NCM_ID = "ncmMusicId";
        QQ_ID = "qqMusicId";
        SPOTIFY_ID = "spotifyId";
        APPLE_ID = "appleMusicId";
    }
}
