//! Netease CDN resizing is adapter-owned; the host only names semantic sizes.
use nons_music_adapter_sdk::cover::{CoverImage, CoverVariants};
pub(crate) fn image(input: &str) -> CoverImage {
    if input.is_empty() {
        return CoverImage::default();
    }
    let Ok(mut original) = url::Url::parse(input) else {
        return CoverImage::default();
    };
    let pairs = original
        .query_pairs()
        .filter(|(key, _)| key != "param")
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect::<Vec<_>>();
    original.set_query(None);
    if !pairs.is_empty() {
        original.query_pairs_mut().extend_pairs(pairs);
    }
    let sized = |pixels: u16| {
        let mut url = original.clone();
        url.query_pairs_mut()
            .append_pair("param", &format!("{pixels}y{pixels}"));
        url.to_string()
    };
    CoverImage::Variants(CoverVariants {
        small: sized(128),
        normal: sized(256),
        large: sized(512),
        exlarge: sized(1024),
        original: original.to_string(),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    use nons_music_adapter_sdk::cover::CoverSize;
    #[test]
    fn preserves_other_parameters_and_keeps_original_unsized() {
        let image = image("https://p1.music.126.net/cover.jpg?token=public&param=512y512");
        assert_eq!(
            image.url(CoverSize::Original),
            "https://p1.music.126.net/cover.jpg?token=public"
        );
        for (size, pixels) in [
            (CoverSize::Small, 128),
            (CoverSize::Normal, 256),
            (CoverSize::Large, 512),
            (CoverSize::Exlarge, 1024),
        ] {
            assert!(image
                .url(size)
                .ends_with(&format!("&param={pixels}y{pixels}")));
        }
        assert!(image.validate_adapter().is_ok());
    }
}
