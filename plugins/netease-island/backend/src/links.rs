use regex::Regex;
use serde_json::{json, Value};
use std::sync::LazyLock;
use url::Url;

fn allowed(url: &Url) -> bool {
    matches!(url.scheme(), "http" | "https")
        && url.username().is_empty()
        && url.password().is_none()
        && url.port().is_none()
        && matches!(
            url.host_str(),
            Some("music.163.com" | "y.music.163.com" | "m.music.163.com" | "163cn.tv")
        )
}

pub fn candidates(text: &str) -> Vec<Url> {
    static RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"https?://[^\s<>"'，。；）)]+"#).expect("constant URL expression")
    });
    RE.find_iter(text)
        .filter_map(|item| {
            let url = Url::parse(item.as_str()).ok()?;
            let share =
                matches!(url.host_str(), Some("163cn.tv" | "y.music.163.com")) || direct(&url);
            (allowed(&url) && share && url.as_str().len() <= 2048).then_some(url)
        })
        .take(4)
        .collect()
}

fn direct(url: &Url) -> bool {
    matches!(url.path(), "/song" | "/song/" | "/m/song")
        || url.fragment().is_some_and(|f| f.starts_with("/song?"))
}

pub fn song_id(url: &Url) -> Option<u64> {
    if !allowed(url) || !direct(url) {
        return None;
    }
    let query = url
        .fragment()
        .and_then(|f| f.strip_prefix("/song?"))
        .or_else(|| url.query())?;
    url::form_urlencoded::parse(query.as_bytes()).find_map(|(key, value)| {
        if key != "id" {
            return None;
        }
        let id = value.parse::<u64>().ok()?;
        (id > 0 && id <= 9_007_199_254_740_991).then_some(id)
    })
}

// One event shares at most three HTTP requests. With 500ms each, song lookup
// retains its 3s budget inside the host's 5s guest execution deadline.
pub fn resolve(
    mut url: Url,
    remaining: &mut usize,
    mut request: impl FnMut(&Value) -> Result<Value, String>,
) -> Result<Url, String> {
    for _ in 0..3 {
        if !allowed(&url) {
            return Err("分享链接跳转到非允许地址".into());
        }
        if direct(&url) {
            return Ok(url);
        }
        if *remaining == 0 {
            return Err("分享链接解析请求已达上限".into());
        }
        *remaining -= 1;
        let response = request(
            &json!({"url":url.as_str(),"method":"GET","responseType":"none","timeoutMs":500}),
        )?;
        let status = response["status"].as_u64().ok_or("无效 HTTP 状态")?;
        if !(300..400).contains(&status) {
            return Err("不支持的分享链接".into());
        }
        let location = response["headers"]["location"]
            .as_str()
            .ok_or("短链没有跳转地址")?;
        if location.len() > 2048 {
            return Err("分享链接过长".into());
        }
        url = url
            .join(location)
            .map_err(|error| format!("无效跳转地址：{error}"))?;
    }
    if allowed(&url) && direct(&url) {
        Ok(url)
    } else {
        Err("短链跳转次数过多".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recognizes_song_urls_and_filters_clipboard_text() {
        let text = "私密内容 https://evil.test/music.163.com/song?id=1 https://music.163.com.evil.test/song?id=2 https://music.163.com/song?id=3";
        assert_eq!(candidates(text).len(), 1);
        assert_eq!(song_id(&candidates(text)[0]), Some(3));
        assert_eq!(
            song_id(&Url::parse("https://music.163.com/#/song?id=456").unwrap()),
            Some(456)
        );
        for input in [
            "https://user:pass@music.163.com/song?id=1",
            "https://music.163.com/song?id=-1",
            "https://music.163.com/song?id=9007199254740992",
            "https://music.163.com/playlist?id=1",
        ] {
            assert!(song_id(&Url::parse(input).unwrap()).is_none());
        }
    }
    #[test]
    fn plugin_follows_relative_redirects_and_rejects_external_hops() {
        let mut remaining = 3;
        let mut calls = 0;
        let result = resolve(Url::parse("https://163cn.tv/abc").unwrap(), &mut remaining, |args| {
            calls += 1;
            assert_eq!(args["responseType"], "none");
            Ok(json!({"status":302,"headers":{"location":"https://music.163.com/#/song?id=123"}}))
        }).unwrap();
        assert_eq!(song_id(&result), Some(123));
        assert_eq!(calls, 1);
        assert_eq!(remaining, 2);
        let relative = resolve(
            Url::parse("https://y.music.163.com/share/abc").unwrap(),
            &mut remaining,
            |_| Ok(json!({"status":302,"headers":{"location":"/song?id=456"}})),
        )
        .unwrap();
        assert_eq!(song_id(&relative), Some(456));
        assert!(resolve(
            Url::parse("https://163cn.tv/abc").unwrap(),
            &mut remaining,
            |_| { Ok(json!({"status":302,"headers":{"location":"https://evil.test/private"}})) }
        )
        .is_err());
        let mut budget = 3;
        assert!(resolve(
            Url::parse("https://163cn.tv/abc").unwrap(),
            &mut budget,
            |_| { Ok(json!({"status":302,"headers":{"location":"/again"}})) }
        )
        .is_err());
        assert_eq!(budget, 0);
    }
}
