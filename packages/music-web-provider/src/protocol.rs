use crate::{AdapterResult, Config, Http, Request, Response};
use nons_music_adapter_sdk::identity::MusicTrack;
use serde_json::Value;
use url::Url;

pub(crate) struct Target {
    pub site: &'static str,
    pub id: String,
    pub aliases: Vec<String>,
}
impl Target {
    pub fn new(track: &MusicTrack) -> AdapterResult<Self> {
        let platform = track.reference.source.as_str();
        let site = match platform {
            "netease" => "netease",
            "qqmusic" => "qq",
            "kugou" => "kugou",
            "kuwo" => "kuwo",
            _ => return Err("unsupported"),
        };
        let raw = track.reference.id.as_str();
        let id = if platform == "qqmusic" {
            raw.strip_prefix("id:").unwrap_or(raw)
        } else {
            raw
        };
        let valid = match platform {
            "netease" | "kuwo" => id.bytes().all(|b| b.is_ascii_digit()),
            "kugou" => id.len() == 32 && id.bytes().all(|b| b.is_ascii_hexdigit()),
            "qqmusic" => id.bytes().all(|b| b.is_ascii_alphanumeric()),
            _ => false,
        };
        if !valid || id.is_empty() || id.len() > 128 {
            return Err("invalidData");
        }
        let mut aliases = vec![id.to_owned()];
        // QQ endpoints can return the numeric ID when a song MID was requested.
        if platform == "qqmusic" {
            aliases.extend(
                track
                    .lyric_identifiers
                    .iter()
                    .filter(|i| matches!(i.kind.as_str(), "song-id" | "song-mid"))
                    .map(|i| i.id.as_str().to_owned()),
            );
        }
        Ok(Self {
            site,
            id: id.to_owned(),
            aliases,
        })
    }
    fn matches(&self, id: &str) -> bool {
        self.aliases.iter().any(|a| {
            if self.site == "kugou" {
                a.eq_ignore_ascii_case(id)
            } else {
                a == id
            }
        })
    }
}

pub(crate) fn checked_url(input: &str, base: &str) -> AdapterResult<Url> {
    if input.is_empty() {
        return Err("invalidData");
    }
    let url = Url::parse(base)
        .and_then(|b| b.join(input))
        .map_err(|_| "invalidData")?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err("permissionDenied");
    }
    Ok(url)
}

pub(crate) struct Client<'a, H> {
    pub http: &'a H,
    config: Config,
    cookies: Vec<(String, String)>,
}
impl<'a, H: Http> Client<'a, H> {
    pub fn new(http: &'a H, config: Config) -> Self {
        Self {
            http,
            config,
            cookies: vec![],
        }
    }
    pub fn headers(&self, url: &Url) -> Vec<(String, String)> {
        // Public CDN resources must work without custom headers: the playback
        // transport consumes a URI. Probe them under the same condition.
        let mut headers = vec![];
        if url.origin() == Url::parse(self.config.base).unwrap().origin() {
            headers.push(("User-Agent".into(), self.config.user_agent.into()));
            headers.push(("Referer".into(), self.config.base.into()));
            if !self.cookies.is_empty() {
                headers.push((
                    "Cookie".into(),
                    self.cookies
                        .iter()
                        .map(|(k, v)| format!("{k}={v}"))
                        .collect::<Vec<_>>()
                        .join("; "),
                ));
            }
        }
        headers
    }
    fn set_cookie(&mut self, name: &str, value: &str) -> AdapterResult<()> {
        if name.is_empty()
            || !name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
            || !value
                .bytes()
                .all(|b| b.is_ascii_graphic() && b != b';' && b != b',' && b != b'"')
        {
            return Err("invalidData");
        }
        self.cookies.retain(|(key, _)| key != name);
        self.cookies.push((name.into(), value.into()));
        Ok(())
    }
    pub fn get(&mut self, url: &mut Url, extra: &[(String, String)]) -> AdapterResult<Response> {
        *url = checked_url(url.as_str(), self.config.base)?;
        let mut headers = self.headers(url);
        headers.extend_from_slice(extra);
        let response = self.http.send(Request {
            url: url.to_string(),
            method: "GET",
            headers,
            body: vec![],
            follow_redirects: true,
            timeout_ms: 0,
        })?;
        *url = checked_url(&response.url, self.config.base)?;
        response.check()?;
        Ok(response)
    }
    pub fn locate(&mut self, target: &Target) -> AdapterResult<Url> {
        // Each invocation owns its anonymous session; no persistent cookie or URL cache.
        for _ in 0..2 {
            let response = self.get(&mut Url::parse(self.config.base).unwrap(), &[])?;
            for (_, value) in response
                .headers
                .iter()
                .filter(|(k, _)| k.eq_ignore_ascii_case("set-cookie"))
            {
                if let Some((name, value)) = value.split(';').next().and_then(|c| c.split_once('='))
                {
                    self.set_cookie(name.trim(), value.trim())?;
                }
            }
            let text = std::str::from_utf8(&response.body).map_err(|_| "invalidData")?;
            if let Some(tail) = text.split("document.cookie=\"mk_visit=").nth(1) {
                self.set_cookie(
                    "mk_visit",
                    tail.split([';', '"']).next().ok_or("invalidData")?,
                )?;
            } else {
                break;
            }
        }
        let mut headers = self.headers(&Url::parse(self.config.base).unwrap());
        headers.extend([
            (
                "Origin".into(),
                self.config.base.trim_end_matches('/').into(),
            ),
            ("X-Requested-With".into(), "XMLHttpRequest".into()),
            ("Accept".into(), "application/json".into()),
            (
                "Content-Type".into(),
                "application/x-www-form-urlencoded; charset=UTF-8".into(),
            ),
        ]);
        let body = url::form_urlencoded::Serializer::new(String::new())
            .extend_pairs([
                ("input", target.id.as_str()),
                ("filter", "id"),
                ("type", target.site),
                ("page", "1"),
            ])
            .finish()
            .into_bytes();
        let response = self.http.send(Request {
            url: self.config.base.into(),
            method: "POST",
            headers,
            body,
            follow_redirects: true,
            timeout_ms: 0,
        })?;
        checked_url(&response.url, self.config.base)?;
        response.check()?;
        let data: Value = serde_json::from_slice(&response.body).map_err(|_| "invalidData")?;
        match data["code"].as_u64() {
            Some(200) => {}
            Some(403) => return Err("permissionDenied"),
            Some(429) => return Err("rateLimited"),
            _ => return Err("notFound"),
        }
        let rows = data["data"].as_array().ok_or("notFound")?;
        let row = rows
            .iter()
            .find(|r| {
                let id = match &r["songid"] {
                    Value::String(s) => s.clone(),
                    Value::Number(n) => n.to_string(),
                    _ => return false,
                };
                target.matches(&id)
            })
            .ok_or("notFound")?;
        if row["isPreview"] == true || row["preview"] == true || row["is_trial"] == true {
            return Err("permissionDenied");
        }
        let url = row["url"]
            .as_str()
            .filter(|u| !u.is_empty())
            .ok_or("notFound")?;
        checked_url(url, self.config.base)
    }
}
