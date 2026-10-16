use super::*;
const BASE: &str = "https://fixture.music.invalid/";
const UA: &str = "fixture-agent";
const CONFIG: Config = Config {
    base: BASE,
    user_agent: UA,
    manifest: include_str!("fixture.json"),
};
fn call(http: &impl Http, method: &str, args: &str) -> String {
    super::call(http, method, args, CONFIG)
}

use std::{
    cell::RefCell,
    io::{Read, Seek, SeekFrom},
};

fn input(platform: &str, id: &str) -> Value {
    json!({"request":{"track":{"reference":{"source":platform,"kind":"track","id":id},"title":"Test track","aliases":[],"artist":"Test artist","artists":[],"album":"","albumReference":null,"durationMs":180000,"cover":"","associations":[]},"preferredQuality":"exhigh","allowDowngrade":false}})
}
fn mp3() -> Vec<u8> {
    let mut data = vec![0; 6890 * 1044];
    for offset in (0..data.len()).step_by(1044) {
        data[offset..offset + 4].copy_from_slice(&[255, 251, 224, 0]);
    }
    data
}
fn flac() -> Vec<u8> {
    let mut data = b"fLaC\x80\0\0\x22".to_vec();
    let mut info = [0u8; 34];
    info[..4].copy_from_slice(&[0x10, 0, 0x10, 0]);
    let packed = (44100u64 << 44) | (1u64 << 41) | (15u64 << 36) | (44100 * 180);
    info[10..18].copy_from_slice(&packed.to_be_bytes());
    data.extend(info);
    data.extend(vec![0u8; 65536]);
    data
}
struct Fixture {
    config: Config,
    requests: RefCell<Vec<Request>>,
    media: Vec<u8>,
    row: Value,
    status: u16,
    range: bool,
}
impl Fixture {
    fn new(id: &str) -> Self {
        Self {
            config: CONFIG,
            requests: RefCell::new(vec![]),
            media: mp3(),
            row: json!({"songid":id,"url":"api.php?get=url&id=test"}),
            status: 200,
            range: true,
        }
    }
    fn response(&self, request: &Request) -> Response {
        if request.method == "POST" {
            return Response {
                status: self.status,
                headers: vec![],
                body: json!({"code":200,"data":[self.row]})
                    .to_string()
                    .into_bytes(),
            };
        }
        let header = |name: &str| {
            request
                .headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(name))
                .map(|(_, v)| v.as_str())
        };
        if request.url == self.config.base {
            let body = if header("Cookie").is_some_and(|c| c.contains("mk_visit=visit")) {
                b"<html>ready</html>".to_vec()
            } else {
                b"<script>document.cookie=\"mk_visit=visit;path=/\";</script>".to_vec()
            };
            return Response {
                status: 200,
                headers: vec![(
                    "Set-Cookie".into(),
                    "session=anonymous; Path=/; Secure; HttpOnly".into(),
                )],
                body,
            };
        }
        if request.url.starts_with(self.config.base) {
            assert!(header("Cookie").unwrap().contains("mk_visit=visit"));
            return Response {
                status: 302,
                headers: vec![(
                    "Location".into(),
                    "https://m1.music.126.net/fixture.mp3".into(),
                )],
                body: vec![],
            };
        }
        assert!(header("User-Agent").is_none());
        assert!(header("Cookie").is_none());
        assert!(header("Referer").is_none());
        assert!(header("Origin").is_none());
        if !self.range {
            return Response {
                status: 200,
                headers: vec![],
                body: vec![],
            };
        }
        let (start, end) = header("Range")
            .unwrap()
            .strip_prefix("bytes=")
            .unwrap()
            .split_once('-')
            .unwrap();
        let (start, end) = (
            start.parse::<usize>().unwrap(),
            end.parse::<usize>().unwrap().min(self.media.len() - 1),
        );
        Response {
            status: 206,
            headers: vec![
                (
                    "Content-Range".into(),
                    format!("bytes {start}-{end}/{}", self.media.len()),
                ),
                ("ETag".into(), "\"fixture\"".into()),
            ],
            body: self.media[start..=end].to_vec(),
        }
    }
}
impl Http for Fixture {
    fn send(&self, request: Request) -> AdapterResult<Response> {
        let response = self.response(&request);
        self.requests.borrow_mut().push(request);
        Ok(response)
    }
}
fn invoke(http: &impl Http, input: &Value) -> Value {
    serde_json::from_str(&call(http, "music.resolve-resource", &input.to_string())).unwrap()
}
#[test]
fn four_platforms_use_only_exact_id_and_keep_site_cookies_off_cdn() {
    for (platform, site, id) in [
        ("netease", "netease", "347230"),
        ("qqmusic", "qq", "003OUlho2HcRHC"),
        ("kuwo", "kuwo", "156483"),
        ("kugou", "kugou", "AABBCCDDEEFF00112233445566778899"),
    ] {
        let fixture = Fixture::new(id);
        let result = invoke(&fixture, &input(platform, id));
        assert_eq!(result["status"], "ok", "{result}");
        assert_eq!(result["data"]["metadata"]["extent"]["kind"], "full");
        assert_eq!(result["data"]["metadata"]["actualQuality"], "320k");
        assert_eq!(result["data"]["headers"], json!([]));
        let requests = fixture.requests.borrow();
        let post = requests.iter().find(|r| r.method == "POST").unwrap();
        let form: std::collections::BTreeMap<_, _> = url::form_urlencoded::parse(&post.body)
            .into_owned()
            .collect();
        assert_eq!(form["type"], site);
        assert_eq!(form["input"], id);
        assert_eq!(form["filter"], "id");
        assert_eq!(form["page"], "1");
        assert!(
            requests
                .iter()
                .filter(|r| r.url != BASE && r.method == "GET")
                .count()
                <= 9
        );
    }
}
#[test]
fn qq_numeric_compatibility_and_known_song_id_alias_are_exact() {
    let fixture = Fixture::new("42");
    assert_eq!(invoke(&fixture, &input("qqmusic", "id:42"))["status"], "ok");
    let mut args = input("qqmusic", "003OUlho2HcRHC");
    args["request"]["track"]["lyricIdentifiers"] = json!([{"kind":"song-id","id":"42"}]);
    assert_eq!(invoke(&fixture, &args)["status"], "ok");
    assert_eq!(
        invoke(&fixture, &input("qqmusic", "differentMID"))["data"]["code"],
        "notFound"
    );
}
#[test]
fn search_descriptor_and_invalid_requests_do_no_network() {
    let fixture = Fixture::new("1");
    let descriptor: Value =
        serde_json::from_str(&call(&fixture, "music.resource-descriptor", "null")).unwrap();
    assert_eq!(descriptor["data"]["searchPlatforms"], json!([]));
    for platform in ["netease", "qqmusic", "kugou", "kuwo"] {
        let reply: Value = serde_json::from_str(&call(
            &fixture,
            "music.search-resources",
            &input(platform, "1").to_string(),
        ))
        .unwrap();
        assert_eq!(reply["data"]["code"], "unsupported");
    }
    assert_eq!(
        invoke(&fixture, &input("netease", "1&filter=name"))["data"]["code"],
        "invalidData"
    );
    let mut args = input("netease", "1");
    args["request"]["track"]["durationMs"] = json!(0);
    assert_eq!(invoke(&fixture, &args)["data"]["code"], "permissionDenied");
    args["request"]["track"]["durationMs"] = json!(180000);
    args["request"]["preferredQuality"] = json!("future-quality");
    assert_eq!(invoke(&fixture, &args)["data"]["code"], "unsupported");
    assert!(fixture.requests.borrow().is_empty());
}
#[test]
fn rejects_wrong_identity_preview_short_audio_and_forbidden_redirects() {
    let mut fixture = Fixture::new("2");
    assert_eq!(
        invoke(&fixture, &input("netease", "1"))["data"]["code"],
        "notFound"
    );
    fixture.row = json!({"songid":1,"url":"api.php?get=url&id=1","isPreview":true});
    assert_eq!(
        invoke(&fixture, &input("netease", "1"))["data"]["code"],
        "permissionDenied"
    );
    fixture.row = json!({"songid":1,"url":"https://evil.invalid/audio"});
    assert_eq!(
        invoke(&fixture, &input("netease", "1"))["data"]["code"],
        "permissionDenied"
    );
    fixture.row = json!({"songid":1,"url":"api.php?get=url&id=1"});
    fixture.media.truncate(1150 * 1044);
    assert_eq!(
        invoke(&fixture, &input("netease", "1"))["data"]["code"],
        "permissionDenied"
    );
}
#[test]
fn range_support_rate_limit_and_quality_are_enforced() {
    let mut fixture = Fixture::new("1");
    fixture.range = false;
    assert_eq!(
        invoke(&fixture, &input("netease", "1"))["data"]["code"],
        "invalidData"
    );
    fixture.range = true;
    fixture.status = 429;
    assert_eq!(
        invoke(&fixture, &input("netease", "1"))["data"]["code"],
        "rateLimited"
    );
    fixture.status = 200;
    let mut args = input("netease", "1");
    args["request"]["preferredQuality"] = json!("lossless");
    assert_eq!(invoke(&fixture, &args)["data"]["code"], "permissionDenied");
    args["request"]["allowDowngrade"] = json!(true);
    assert_eq!(
        invoke(&fixture, &args)["data"]["metadata"]["actualQuality"],
        "320k"
    );
    fixture.media = flac();
    args["request"]["allowDowngrade"] = json!(false);
    assert_eq!(
        invoke(&fixture, &args)["data"]["metadata"]["actualQuality"],
        "lossless"
    );
}
#[test]
fn remote_reader_bounds_network_reads_and_seeks() {
    let fixture = Fixture::new("1");
    let mut client = protocol::Client::new(&fixture, CONFIG);
    let mut reader = audio::Remote::new(
        &mut client,
        url::Url::parse("https://m1.music.126.net/fixture.mp3").unwrap(),
    )
    .unwrap();
    for offset in [0, 65536, 131072, 196608] {
        reader.seek(SeekFrom::Start(offset)).unwrap();
        reader.read_exact(&mut [0u8; 1]).unwrap();
    }
    reader.seek(SeekFrom::Start(262144)).unwrap();
    assert!(reader.read_exact(&mut [0u8; 1]).is_err());
    assert_eq!(fixture.requests.borrow().len(), 4);
    assert!(reader.seek(SeekFrom::End(1)).is_err());
}
#[test]
fn url_authorization_checks_suffix_credentials_and_scheme() {
    for url in [
        "https://music.126.net.evil.invalid/a",
        "https://127.0.0.1/a",
        "file:///audio",
        "https://user@fixture.music.invalid/a",
        "https://fixture.music.invalid:8443/a",
    ] {
        assert!(protocol::allowed(url, BASE, CONFIG).is_err(), "{url}");
    }
}

#[test]
fn adapter_manifests_keep_site_headers_and_permissions_separate() {
    for config in [
        Config {
            base: "https://www.jbsou.cn/",
            user_agent: UA,
            manifest: include_str!("../../../adapters/jbsou/manifest.json"),
        },
        Config {
            base: "https://music.90svip.cn/",
            user_agent: UA,
            manifest: include_str!("../../../adapters/xiaobai/manifest.json"),
        },
    ] {
        for (platform, id) in [
            ("netease", "347230"),
            ("qqmusic", "000akynZ2Rbro5"),
            ("kugou", "AABBCCDDEEFF00112233445566778899"),
            ("kuwo", "156483"),
        ] {
            let mut fixture = Fixture::new(id);
            fixture.config = config;
            let response: Value = serde_json::from_str(&super::call(
                &fixture,
                "music.resolve-resource",
                &input(platform, id).to_string(),
                config,
            ))
            .unwrap();
            assert_eq!(response["status"], "ok", "{response}");
            let requests = fixture.requests.borrow();
            let post = requests.iter().find(|r| r.method == "POST").unwrap();
            assert_eq!(post.url, config.base);
            assert!(post
                .headers
                .iter()
                .any(|(k, v)| k == "Origin" && v == config.base.trim_end_matches('/')));
            assert!(post
                .headers
                .iter()
                .any(|(k, v)| k == "Referer" && v == config.base));
            let other = if config.base.contains("jbsou") {
                "https://music.90svip.cn/api.php"
            } else {
                "https://www.jbsou.cn/api.php"
            };
            assert!(protocol::allowed(other, config.base, config).is_err());
        }
    }
}
