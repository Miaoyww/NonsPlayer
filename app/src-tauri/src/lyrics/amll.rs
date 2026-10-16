use super::{identity::Identity, valid_ttml, Lookup};
use crate::{
    model::{AppResult, Lyrics},
    storage::{Store, MAX_LYRIC_BYTES},
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, time::Duration};

#[derive(Clone, Serialize)]
pub(super) struct Request {
    pub identity: Identity,
    pub endpoints: Vec<String>,
}

impl Request {
    pub fn new(identity: Identity, store: &Store) -> AppResult<Option<Self>> {
        let Some(directory) = identity.directory() else {
            return Ok(None);
        };
        let configured: BTreeMap<String, Vec<String>> = store
            .setting("lyricEndpointsV2")?
            .map(|v| serde_json::from_str(&v).map_err(|e| e.to_string()))
            .transpose()?
            .unwrap_or_default();
        let endpoints = if let Some(endpoints) = configured.get(&identity.platform) {
            endpoints.clone()
        } else if identity.platform == "netease" {
            store
                .setting("lyricEndpoints")?
                .map(|v| serde_json::from_str(&v).map_err(|e| e.to_string()))
                .transpose()?
                .unwrap_or_else(|| {
                    vec![
                        official(directory),
                        "https://amll-ttml-db.stevexmh.net/ncm/{id}".into(),
                    ]
                })
        } else {
            vec![official(directory)]
        };
        Ok(Some(Self {
            identity,
            endpoints,
        }))
    }

    pub fn key(&self) -> AppResult<String> {
        Ok(format!(
            "amll:v2:{:x}",
            Sha256::digest(serde_json::to_vec(self).map_err(|e| e.to_string())?)
        ))
    }

    /// Try each ID before moving to another mirror. Every request shares the same cap.
    fn addresses(&self) -> Vec<String> {
        self.endpoints
            .iter()
            .flat_map(|endpoint| {
                self.identity
                    .identifiers
                    .iter()
                    .filter_map(|id| address(endpoint, id.id.as_str()))
            })
            .take(3)
            .collect()
    }

    pub async fn lookup(&self, client: &reqwest::Client) -> Lookup {
        self.lookup_addresses(client, self.addresses()).await
    }

    async fn lookup_addresses(&self, client: &reqwest::Client, addresses: Vec<String>) -> Lookup {
        tokio::time::timeout(Duration::from_secs(1), async {
            // Unvisited IDs/endpoints cannot establish a complete negative result.
            let mut all_missing = !addresses.is_empty()
                && self.endpoints.len() * self.identity.identifiers.len() == addresses.len();
            for address in addresses {
                let Ok(mut response) = client
                    .get(address)
                    .timeout(Duration::from_millis(500))
                    .send()
                    .await
                else {
                    all_missing = false;
                    continue;
                };
                if response.status() == reqwest::StatusCode::NOT_FOUND {
                    continue;
                }
                if !response.status().is_success()
                    || response
                        .content_length()
                        .is_some_and(|n| n > MAX_LYRIC_BYTES as u64)
                {
                    all_missing = false;
                    continue;
                }
                let mut bytes = Vec::new();
                let mut complete = true;
                loop {
                    match response.chunk().await {
                        Ok(Some(chunk)) if bytes.len() + chunk.len() <= MAX_LYRIC_BYTES => {
                            bytes.extend_from_slice(&chunk)
                        }
                        Ok(None) => break,
                        _ => {
                            complete = false;
                            break;
                        }
                    }
                }
                if complete {
                    if let Ok(content) = String::from_utf8(bytes) {
                        if valid_ttml(&content) {
                            return Lookup::Found(Lyrics {
                                match_score: None,
                                source: "amll".into(),
                                format: "ttml".into(),
                                content,
                                translation: None,
                                romanization: None,
                            });
                        }
                    }
                }
                all_missing = false;
            }
            if all_missing {
                Lookup::Missing
            } else {
                Lookup::Failed
            }
        })
        .await
        .unwrap_or(Lookup::Failed)
    }
}

fn official(directory: &str) -> String {
    format!("https://raw.githubusercontent.com/amll-dev/amll-ttml-db/refs/heads/main/{directory}/{{id}}.ttml")
}

fn address(template: &str, id: &str) -> Option<String> {
    // IDs are safe single path segments; literal dot segments must never normalize directories.
    if id.is_empty() || !id.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return None;
    }
    let probe = url::Url::parse(&template.replace("{id}", "LYRICIDPLACEHOLDER")).ok()?;
    if !probe.path().contains("LYRICIDPLACEHOLDER")
        || probe
            .host_str()
            .is_none_or(|host| host.contains("lyricidplaceholder"))
        || probe
            .query()
            .is_some_and(|v| v.contains("LYRICIDPLACEHOLDER"))
        || probe
            .fragment()
            .is_some_and(|v| v.contains("LYRICIDPLACEHOLDER"))
    {
        return None;
    }
    let url = url::Url::parse(&template.replace("{id}", id)).ok()?;
    (url.scheme() == "https"
        && url.host_str().is_some()
        && url.username().is_empty()
        && url.password().is_none())
    .then(|| url.to_string())
}

pub(super) fn validate_endpoints(endpoints: &[String]) -> AppResult<()> {
    if endpoints.is_empty()
        || endpoints.len() > 3
        || endpoints
            .iter()
            .any(|v| v.len() > 2048 || !v.contains("{id}") || address(v, "1").is_none())
    {
        return Err("设置 1–3 个 HTTPS 歌词地址，使用路径中的 {id} 表示歌曲 ID".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TestDir;
    use nons_music_adapter_sdk::identity::LyricIdentifier;
    use std::{
        io::{Read, Write},
        net::TcpListener,
    };

    fn identity(platform: &str, ids: &[(&str, &str)]) -> Identity {
        Identity {
            platform: platform.into(),
            identifiers: ids
                .iter()
                .map(|(kind, id)| LyricIdentifier {
                    kind: (*kind).into(),
                    id: (*id).to_string().try_into().unwrap(),
                })
                .collect(),
        }
    }
    #[test]
    fn legacy_mirrors_only_apply_to_netease_and_platform_config_changes_cache_key() {
        let root = TestDir::new();
        let store = Store::open(&root.0.join("library.sqlite3")).unwrap();
        store
            .set_setting("lyricEndpoints", r#"["https://old.example/ncm/{id}"]"#)
            .unwrap();
        let ncm = Request::new(identity("netease", &[("song-id", "123")]), &store)
            .unwrap()
            .unwrap();
        assert_eq!(ncm.addresses(), ["https://old.example/ncm/123"]);
        let qq = Request::new(
            identity("qqmusic", &[("song-id", "123"), ("song-mid", "001mid")]),
            &store,
        )
        .unwrap()
        .unwrap();
        assert!(qq.addresses().iter().all(|v| v.contains("/qq-lyrics/")));
        assert_ne!(qq.key().unwrap(), ncm.key().unwrap());
        store
            .set_setting(
                "lyricEndpointsV2",
                r#"{"qqmusic":["https://new.example/qq/{id}"]}"#,
            )
            .unwrap();
        let changed = Request::new(qq.identity.clone(), &store).unwrap().unwrap();
        assert_ne!(changed.key().unwrap(), qq.key().unwrap());
        assert_eq!(changed.addresses()[0], "https://new.example/qq/123");
        let mut other_type = changed.clone();
        other_type.identity.identifiers[0].kind = "song-mid".into();
        assert_ne!(changed.key().unwrap(), other_type.key().unwrap());
    }
    #[test]
    fn requests_share_a_three_attempt_cap_and_ids_cannot_rewrite_url_structure() {
        let request = Request {
            identity: identity("qqmusic", &[("song-id", "123"), ("song-mid", "001mid")]),
            endpoints: vec![
                "https://a.example/qq/{id}".into(),
                "https://b.example/qq/{id}".into(),
            ],
        };
        assert_eq!(request.addresses().len(), 3);
        assert_eq!(request.addresses()[1], "https://a.example/qq/001mid");
        for template in [
            "https://{id}.example/a",
            "https://example/a?id={id}",
            "https://user:pass@example/{id}",
            "http://example/{id}",
        ] {
            assert!(validate_endpoints(&[template.into()]).is_err());
        }
        for id in ["../a", ".", "a/b", "a?b", "%2e%2e", ""] {
            assert!(address("https://example/qq/{id}", id).is_none());
        }
    }
    #[test]
    fn missing_errors_and_valid_ttml_are_distinct_over_real_http() {
        for (status, body, expected) in [
            (404, "", 0),
            (500, "", 1),
            (200, "broken", 1),
            (200, "<tt><body/></tt>", 2),
        ] {
            let server = TcpListener::bind("127.0.0.1:0").unwrap();
            let address = format!("http://{}/123.ttml", server.local_addr().unwrap());
            let worker = std::thread::spawn(move || {
                let (mut stream, _) = server.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut data = [0; 4096];
                assert!(stream.read(&mut data).unwrap() > 0);
                write!(
                    stream,
                    "HTTP/1.1 {status} OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
                .unwrap();
            });
            let request = Request {
                identity: identity("netease", &[("song-id", "123")]),
                endpoints: vec!["fixture".into()],
            };
            let result = tauri::async_runtime::block_on(
                request.lookup_addresses(&reqwest::Client::new(), vec![address]),
            );
            assert!(matches!(
                (result, expected),
                (Lookup::Missing, 0) | (Lookup::Failed, 1) | (Lookup::Found(_), 2)
            ));
            worker.join().unwrap();
        }
    }
}
