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

/// Artwork metadata always uses semantic sizes, including host-owned local paths.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CoverImage {
    Variants(CoverVariants),
}
impl Default for CoverImage {
    fn default() -> Self {
        Self::from(String::new())
    }
}
impl From<String> for CoverImage {
    fn from(value: String) -> Self {
        Self::Variants(CoverVariants {
            original: value.clone(),
            small: value.clone(),
            normal: value.clone(),
            large: value.clone(),
            exlarge: value,
        })
    }
}
impl From<&str> for CoverImage {
    fn from(value: &str) -> Self {
        Self::from(value.to_owned())
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
    pub fn validate_hosts(&self, _hosts: &std::collections::BTreeSet<String>) -> MusicResult<()> {
        self.validate_adapter()?;
        Ok(())
    }
    pub fn url(&self, size: CoverSize) -> &str {
        match self {
            Self::Variants(value) => match size {
                CoverSize::Small => &value.small,
                CoverSize::Normal => &value.normal,
                CoverSize::Large => &value.large,
                CoverSize::Exlarge => &value.exlarge,
                CoverSize::Original => &value.original,
            },
        }
    }
    /// Live adapters return either empty artwork or valid URLs for every semantic size.
    pub fn validate_adapter(&self) -> MusicResult<()> {
        match self {
            Self::Variants(value) => {
                if [
                    &value.original,
                    &value.small,
                    &value.normal,
                    &value.large,
                    &value.exlarge,
                ]
                .iter()
                .all(|url| url.is_empty())
                {
                    return Ok(());
                }
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
        assert!(serde_json::from_str::<CoverImage>(r#""https://cdn.example/old""#).is_err());
        assert!(CoverImage::default().validate_adapter().is_ok());
        let local = CoverImage::from("C:/covers/local.png");
        assert_eq!(
            serde_json::to_value(&local).unwrap()["original"],
            "C:/covers/local.png"
        );
        assert!(local.validate_adapter().is_err());
    }
}

/// Host declarations may be exact hosts, IPs, explicit subdomain suffixes or `*`.
pub fn valid_host_rule(rule: &str) -> bool {
    if rule == "*" {
        return true;
    }
    let host = rule.strip_prefix("*.").unwrap_or(rule);
    let Ok(url) = url::Url::parse(&format!("http://{host}")) else {
        return false;
    };
    !host.is_empty()
        && host.len() <= 260
        && url.host_str().is_some()
        && url.path() == "/"
        && url.query().is_none()
        && url.fragment().is_none()
}
pub fn checked_url(
    input: &str,
    _hosts: &std::collections::BTreeSet<String>,
) -> MusicResult<url::Url> {
    let url = url::Url::parse(input).map_err(|_| ErrorCode::InvalidData)?;
    if input.len() > 2048 || !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err(ErrorCode::PermissionDenied.into());
    }
    Ok(url)
}

#[cfg(test)]
mod host_tests {
    use super::*;
    #[test]
    fn cover_urls_allow_any_host_and_port_but_keep_http_syntax() {
        let hosts = ["*.cdn.example".into(), "images.example".into()]
            .into_iter()
            .collect();
        for url in [
            "https://p1.cdn.example/a.jpg",
            "https://images.example/a.jpg",
            "https://fakecdn.example/a.jpg",
            "https://p1.cdn.example.evil/a.jpg",
            "https://user:secret@images.example/a.jpg",
            "https://images.example:444/a.jpg",
            "https://127.0.0.1/a.jpg",
            "https://other.example/a.jpg",
        ] {
            assert!(checked_url(url, &hosts).is_ok());
        }
        for url in ["file:///a.jpg", "ftp://example.org/a.jpg"] {
            assert!(checked_url(url, &hosts).is_err(), "{url}");
        }
        for rule in [
            "*",
            "*.127.0.0.1",
            "localhost",
            "[::1]:8080",
            "images.example:443",
            "user:secret@images.example:443",
        ] {
            assert!(valid_host_rule(rule), "{rule}");
        }
        for rule in ["*.", "bad host", "images.example/path"] {
            assert!(!valid_host_rule(rule), "{rule}");
        }
    }
}
