use super::*;
const BASE: &str = "https://fixture.music.invalid/";
const CDN: &str = "https://m1.music.126.net/fixture.mp3";
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
    json!({"request":{"track":{"reference":{"source":platform,"kind":"track","id":id},"title":"Test track","aliases":[],"artist":"Test artist","artists":[],"album":"","albumReference":null,"durationMs":180000,"cover":{"original":"","small":"","normal":"","large":"","exlarge":""},"associations":[]},"preferredQuality":"exhigh","allowDowngrade":false}})
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
    cookies: Vec<(String, String)>,
    root_padding: usize,
    response_padding: usize,
    reported_size: Option<usize>,
    media_final_url: Option<String>,
    partial_range: bool,
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
            cookies: vec![(
                "session".into(),
                "anonymous; Path=/; Secure; HttpOnly".into(),
            )],
            root_padding: 0,
            response_padding: 0,
            reported_size: None,
            media_final_url: None,
            partial_range: false,
        }
    }
    fn media_response(
        &self,
        request: &Request,
        headers: Vec<(String, String)>,
        url: String,
    ) -> Response {
        if !self.range {
            return Response {
                status: 200,
                headers: vec![],
                body: vec![],
                url,
            };
        }
        let header = |name: &str| {
            request
                .headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(name))
                .map(|(_, v)| v.as_str())
        };
        let (start, end) = header("Range")
            .unwrap()
            .strip_prefix("bytes=")
            .unwrap()
            .split_once('-')
            .unwrap();
        let reported = self.reported_size.unwrap_or(self.media.len());
        let start = start.parse::<usize>().unwrap();
        let requested_end = end.parse::<usize>().unwrap().min(self.media.len() - 1);
        let end = if self.partial_range {
            start.saturating_add(1024).min(requested_end)
        } else {
            requested_end
        };
        let mut response_headers = headers;
        response_headers.extend([
            (
                "Content-Range".into(),
                format!("bytes {start}-{end}/{reported}"),
            ),
            ("ETag".into(), "\"fixture\"".into()),
        ]);
        Response {
            status: 206,
            headers: response_headers,
            body: self.media[start..=end].to_vec(),
            url,
        }
    }
    fn response(&self, request: &Request) -> Response {
        if request.method == "POST" {
            let mut value = json!({"code":200,"data":[self.row]});
            if self.response_padding > 0 {
                value["padding"] = json!("x".repeat(self.response_padding));
            }
            return Response {
                status: self.status,
                headers: vec![],
                body: value.to_string().into_bytes(),
                url: request.url.clone(),
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
            let mut body = if header("Cookie").is_some_and(|c| c.contains("mk_visit=visit")) {
                b"<html>ready</html>".to_vec()
            } else {
                b"<script>document.cookie=\"mk_visit=visit;path=/\";</script>".to_vec()
            };
            body.resize(body.len() + self.root_padding, b' ');
            return Response {
                status: 200,
                headers: self
                    .cookies
                    .iter()
                    .map(|(name, value)| ("Set-Cookie".into(), format!("{name}={value}")))
                    .collect(),
                body,
                url: request.url.clone(),
            };
        }
        if request.url.starts_with(self.config.base) {
            assert!(header("Cookie").unwrap().contains("mk_visit=visit"));
            assert!(header("User-Agent").is_some());
            assert!(header("Referer").is_some());
            return self.media_response(
                request,
                vec![],
                self.media_final_url.clone().unwrap_or_else(|| CDN.into()),
            );
        }
        assert!(header("User-Agent").is_none());
        assert!(header("Cookie").is_none());
        assert!(header("Referer").is_none());
        assert!(header("Origin").is_none());
        self.media_response(request, vec![], request.url.clone())
    }
}
impl Http for Fixture {
    fn send(&self, request: Request) -> AdapterResult<Response> {
        assert!(request.follow_redirects);
        assert_eq!(request.timeout_ms, 0);
        let response = self.response(&request);
        self.requests.borrow_mut().push(request);
        Ok(response)
    }
}
fn invoke(http: &impl Http, input: &Value) -> Value {
    serde_json::from_str(&call(http, "music.resolve-resource", &input.to_string())).unwrap()
}
#[test]
fn four_platforms_use_exact_id_and_leave_site_cookies_off_cdn() {
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
        assert_eq!(result["data"]["url"], CDN);
        let requests = fixture.requests.borrow();
        let post = requests.iter().find(|r| r.method == "POST").unwrap();
        let form: std::collections::BTreeMap<_, _> = url::form_urlencoded::parse(&post.body)
            .into_owned()
            .collect();
        assert_eq!(form["type"], site);
        assert_eq!(form["input"], id);
        assert_eq!(form["filter"], "id");
        assert_eq!(form["page"], "1");
        assert!(requests.iter().all(|request| request.follow_redirects));
        assert!(requests.iter().all(|request| request.timeout_ms == 0));
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
fn protocol_request_and_response_sizes_have_no_fixed_limits() {
    let mut fixture = Fixture::new("1");
    fixture.root_padding = 140 * 1024;
    fixture.response_padding = 80 * 1024;
    fixture.cookies = (0..12)
        .map(|index| (format!("cookie{index}"), format!("{}x", "v".repeat(4096))))
        .collect();
    let mut args = input("netease", "1").to_string();
    args.push_str(&" ".repeat(140 * 1024));
    let result: Value =
        serde_json::from_str(&call(&fixture, "music.resolve-resource", &args)).unwrap();
    assert_eq!(result["status"], "ok", "{result}");
    let requests = fixture.requests.borrow();
    let post = requests
        .iter()
        .find(|request| request.method == "POST")
        .unwrap();
    let cookie = post
        .headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("Cookie"))
        .map(|(_, value)| value)
        .unwrap();
    for index in 0..12 {
        assert!(cookie.contains(&format!("cookie{index}=v")), "{index}");
    }
}

#[test]
fn cookie_syntax_is_validated_without_size_or_count_limits() {
    let mut fixture = Fixture::new("1");
    fixture.cookies.push(("bad name".into(), "value".into()));
    assert_eq!(
        invoke(&fixture, &input("netease", "1"))["data"]["code"],
        "invalidData"
    );
}
#[test]
fn rejects_wrong_identity_preview_and_short_audio_but_accepts_other_hosts() {
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
    assert_eq!(invoke(&fixture, &input("netease", "1"))["status"], "ok");
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
fn remote_reader_has_no_request_or_byte_budget() {
    let fixture = Fixture::new("1");
    let mut client = protocol::Client::new(&fixture, CONFIG);
    let mut reader = audio::Remote::new(
        &mut client,
        url::Url::parse("https://m1.music.126.net/fixture.mp3").unwrap(),
    )
    .unwrap();
    for offset in (0..10).map(|index| index * 65536) {
        reader.seek(SeekFrom::Start(offset)).unwrap();
        reader.read_exact(&mut [0u8; 1]).unwrap();
    }
    assert_eq!(fixture.requests.borrow().len(), 10);
    assert!(reader.seek(SeekFrom::End(1)).is_err());
}

#[test]
fn remote_reader_accepts_media_larger_than_512_mib() {
    let mut fixture = Fixture::new("1");
    fixture.reported_size = Some(513 * 1024 * 1024);
    let mut client = protocol::Client::new(&fixture, CONFIG);
    let mut reader = audio::Remote::new(&mut client, url::Url::parse(CDN).unwrap()).unwrap();
    reader.read_exact(&mut [0u8; 1]).unwrap();
}

#[test]
fn remote_reader_refetches_revisited_chunks_without_accumulating() {
    const CHUNK: usize = 65536;
    let mut fixture = Fixture::new("1");
    for (index, offset) in (0..3).map(|index| (index, index * CHUNK + 1)) {
        fixture.media[offset] = index as u8 + 1;
    }
    let mut client = protocol::Client::new(&fixture, CONFIG);
    let mut reader = audio::Remote::new(&mut client, url::Url::parse(CDN).unwrap()).unwrap();
    let sequence = [0, 1, 2, 1, 0, 2, 0, 1, 0, 2, 1, 2];
    let mut expected_requests = 1;
    let mut previous = Some(0);
    for block in sequence {
        if previous != Some(block) {
            expected_requests += 1;
        }
        previous = Some(block);
        reader
            .seek(SeekFrom::Start((block * CHUNK + 1) as u64))
            .unwrap();
        let mut byte = [0];
        reader.read_exact(&mut byte).unwrap();
        assert_eq!(byte[0], block as u8 + 1);
    }
    assert_eq!(fixture.requests.borrow().len(), expected_requests);
}

#[test]
fn remote_reader_requires_the_exact_requested_content_range() {
    let mut fixture = Fixture::new("1");
    fixture.partial_range = true;
    let mut client = protocol::Client::new(&fixture, CONFIG);
    assert!(matches!(
        audio::Remote::new(&mut client, url::Url::parse(CDN).unwrap()),
        Err("invalidData")
    ));
}

#[test]
fn host_final_url_is_used_for_media_and_must_be_http() {
    let mut fixture = Fixture::new("1");
    fixture.media_final_url = Some("https://user:pass@127.0.0.1:8443/long?a=b".into());
    let result = invoke(&fixture, &input("netease", "1"));
    assert_eq!(result["status"], "ok", "{result}");
    assert_eq!(
        result["data"]["url"],
        "https://user:pass@127.0.0.1:8443/long?a=b"
    );

    fixture.media_final_url = Some("file:///audio".into());
    assert_eq!(
        invoke(&fixture, &input("netease", "1"))["data"]["code"],
        "permissionDenied"
    );
}
#[test]
fn url_authorization_keeps_http_scheme_only() {
    for url in [
        "https://music.126.net.evil.invalid/a",
        "https://127.0.0.1/a",
        "https://user@fixture.music.invalid/a",
        "https://fixture.music.invalid:8443/a",
    ] {
        assert!(protocol::checked_url(url, BASE).is_ok(), "{url}");
    }
    let long = format!("https://fixture.music.invalid/{}", "a".repeat(8192));
    assert!(protocol::checked_url(&long, BASE).is_ok());
    assert!(protocol::checked_url("file:///audio", BASE).is_err());
    assert!(protocol::checked_url("", BASE).is_err());
}

#[test]
fn adapter_manifests_keep_site_headers_and_permissions_separate() {
    for config in [
        CONFIG,
        Config {
            base: "https://music.90svip.cn/",
            user_agent: UA,
            manifest: include_str!("xiaobai-fixture.json"),
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
            let other = "https://other.music.invalid/api.php";
            assert!(protocol::checked_url(other, config.base).is_ok());
        }
    }
}
