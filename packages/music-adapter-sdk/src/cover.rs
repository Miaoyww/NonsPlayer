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
    pub fn validate_hosts(&self, hosts: &std::collections::BTreeSet<String>) -> MusicResult<()> {
        self.validate_adapter()?;
        if self.is_empty() {
            return Ok(());
        }
        for size in [
            CoverSize::Small,
            CoverSize::Normal,
            CoverSize::Large,
            CoverSize::Exlarge,
            CoverSize::Original,
        ] {
            checked_url(self.url(size), hosts)?;
        }
        Ok(())
    }
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

/// Exact DNS names or explicit subdomain suffixes; no unrestricted `*`, IPs or credentials.
pub fn valid_host_rule(rule: &str) -> bool {
    let host = rule.strip_prefix("*.").unwrap_or(rule);
    host.len() <= 253
        && host.contains('.')
        && host.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        })
        && matches!(url::Host::parse(host), Ok(url::Host::Domain(domain)) if domain == host)
}
pub fn checked_url(
    input: &str,
    hosts: &std::collections::BTreeSet<String>,
) -> MusicResult<url::Url> {
    let url = url::Url::parse(input).map_err(|_| ErrorCode::InvalidData)?;
    if input.len() > 2048
        || !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
        || !url.host_str().is_some_and(|host| {
            hosts.iter().any(|rule| {
                valid_host_rule(rule)
                    && (rule == host
                        || rule.strip_prefix("*.").is_some_and(|suffix| {
                            host.strip_suffix(suffix)
                                .is_some_and(|prefix| prefix.ends_with('.') && prefix.len() > 1)
                        }))
            })
        })
    {
        return Err(ErrorCode::PermissionDenied.into());
    }
    Ok(url)
}

#[cfg(test)]
mod host_tests {
    use super::*;
    #[test]
    fn explicit_cover_hosts_do_not_allow_lookalikes_credentials_ips_or_redirect_targets() {
        let hosts = ["*.cdn.example".into(), "images.example".into()]
            .into_iter()
            .collect();
        for url in [
            "https://p1.cdn.example/a.jpg",
            "https://images.example/a.jpg",
        ] {
            assert!(checked_url(url, &hosts).is_ok());
        }
        for url in [
            "https://cdn.example/a.jpg",
            "https://fakecdn.example/a.jpg",
            "https://p1.cdn.example.evil/a.jpg",
            "https://user:secret@images.example/a.jpg",
            "https://images.example:444/a.jpg",
            "https://127.0.0.1/a.jpg",
            "file:///a.jpg",
            "https://other.example/a.jpg",
        ] {
            assert!(checked_url(url, &hosts).is_err(), "{url}");
        }
        for rule in ["*", "*.127.0.0.1", "localhost", "*.bad..example"] {
            assert!(!valid_host_rule(rule));
        }
    }
}
