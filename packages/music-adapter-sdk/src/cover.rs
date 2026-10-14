//! Semantic artwork sizes. Pixel dimensions and platform URL rules belong to adapters.
use crate::{ErrorCode, MusicResult};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CoverSize {
    Small,
    #[default]
    Normal,
    Large,
    Exlarge,
    Original,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoverVariants {
    pub original: String,
    pub small: String,
    pub normal: String,
    pub large: String,
    pub exlarge: String,
}

/// Strings are retained only for old saved metadata and local artwork paths.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CoverImage {
    Variants(CoverVariants),
    Legacy(String),
}
impl Default for CoverImage {
    fn default() -> Self {
        Self::Legacy(String::new())
    }
}
impl From<String> for CoverImage {
    fn from(value: String) -> Self {
        Self::Legacy(value)
    }
}
impl From<&str> for CoverImage {
    fn from(value: &str) -> Self {
        Self::Legacy(value.into())
    }
}
impl std::ops::Deref for CoverImage {
    type Target = str;
    fn deref(&self) -> &str {
        self.url(CoverSize::Original)
    }
}
impl std::fmt::Display for CoverImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self)
    }
}
impl PartialEq<&str> for CoverImage {
    fn eq(&self, value: &&str) -> bool {
        &**self == *value
    }
}
impl CoverImage {
    pub fn url(&self, size: CoverSize) -> &str {
        match self {
            Self::Legacy(value) => value,
            Self::Variants(value) => match size {
                CoverSize::Small => &value.small,
                CoverSize::Normal => &value.normal,
                CoverSize::Large => &value.large,
                CoverSize::Exlarge => &value.exlarge,
                CoverSize::Original => &value.original,
            },
        }
    }
    /// Live adapters must return all semantic sizes. Legacy saved strings stay readable.
    pub fn validate_adapter(&self) -> MusicResult<()> {
        match self {
            Self::Legacy(value) if value.is_empty() => Ok(()),
            Self::Legacy(_) => Err(ErrorCode::InvalidData.into()),
            Self::Variants(value) => {
                for input in [
                    &value.original,
                    &value.small,
                    &value.normal,
                    &value.large,
                    &value.exlarge,
                ] {
                    let url = url::Url::parse(input).map_err(|_| ErrorCode::InvalidData)?;
                    if input.len() > 2048
                        || !matches!(url.scheme(), "http" | "https")
                        || url.host_str().is_none()
                        || !url.username().is_empty()
                        || url.password().is_some()
                    {
                        return Err(ErrorCode::InvalidData.into());
                    }
                }
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn semantic_sizes_accept_adapter_defined_urls_and_reject_incomplete_live_artwork() {
        let image: CoverImage = serde_json::from_str(r#"{"original":"https://cdn.example/raw","small":"https://cdn.example/tiny","normal":"https://cdn.example/card","large":"https://cdn.example/detail","exlarge":"https://cdn.example/media"}"#).unwrap();
        assert!(image.validate_adapter().is_ok());
        assert_eq!(image.url(CoverSize::Small), "https://cdn.example/tiny");
        assert!(serde_json::from_str::<CoverImage>(
            r#"{"original":"https://cdn.example/raw","small":"https://cdn.example/tiny"}"#
        )
        .is_err());
        let legacy: CoverImage = serde_json::from_str(r#""https://cdn.example/old""#).unwrap();
        assert_eq!(legacy.url(CoverSize::Original), "https://cdn.example/old");
        assert!(legacy.validate_adapter().is_err());
        assert!(CoverImage::default().validate_adapter().is_ok());
        let local = CoverImage::from("C:/covers/local.png");
        assert_eq!(
            serde_json::to_string(&local).unwrap(),
            r#""C:/covers/local.png""#
        );
    }
}
