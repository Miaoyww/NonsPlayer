use super::*;
use std::{
    io::{Read, Write},
    net::TcpListener,
};

fn run_chain(
    responses: Vec<(&'static str, u16, String, u64)>,
    skip_qrc: bool,
) -> (Option<Lyrics>, Vec<(String, String)>) {
    let server = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", server.local_addr().unwrap());
    let worker = std::thread::spawn(move || {
        let mut requests = Vec::new();
        for (path, status, body, delay) in responses {
            let (mut stream, _) = server.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut bytes = Vec::new();
            let mut buffer = [0; 4096];
            loop {
                let count = stream.read(&mut buffer).unwrap();
                assert!(count > 0);
                bytes.extend_from_slice(&buffer[..count]);
                if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                    let head = String::from_utf8_lossy(&bytes[..end]);
                    let length = head
                        .lines()
                        .find_map(|l| {
                            l.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .and_then(|s| s.trim().parse::<usize>().ok())
                        })
                        .unwrap_or(0);
                    if bytes.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            let request = String::from_utf8(bytes).unwrap();
            let first = request.lines().next().unwrap().to_owned();
            assert!(first.contains(path), "unexpected request: {first}");
            requests.push((first, request.split_once("\r\n\r\n").unwrap().1.to_owned()));
            std::thread::sleep(Duration::from_millis(delay));
            let _ = write!(stream, "HTTP/1.1 {status} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
        }
        requests
    });
    let qrc = format!("{base}/qrc");
    let musicu = format!("{base}/musicu");
    let lrc = format!("{base}/lrc");
    let endpoints = LyricEndpoints {
        qrc: &qrc,
        musicu: &musicu,
        lrc: &lrc,
    };
    let result = tauri::async_runtime::block_on(fetch_candidate(
        &reqwest::Client::new(),
        &tests::candidate(),
        skip_qrc,
        &endpoints,
        Duration::from_secs(5),
    ))
    .unwrap();
    (result, worker.join().unwrap())
}

fn modern(lyric: &str, trans: &str) -> String {
    json!({"code":0,"req_1":{"code":0,"data":{"lyric":lyric,"trans":trans}}}).to_string()
}

#[test]
fn qrc_failure_or_timeout_uses_rich_fallback_before_translationless_legacy_lrc() {
    let fixture: Value =
        serde_json::from_str(include_str!("../../../../tests/fixtures/qrc.json")).unwrap();
    for (status, delay) in [(500, 0), (200, 1050)] {
        let (lyrics, requests) = run_chain(
            vec![
                ("/qrc", status, "broken".into(), delay),
                (
                    "/musicu",
                    200,
                    modern(
                        fixture["encrypted"].as_str().unwrap(),
                        fixture["translation"].as_str().unwrap(),
                    ),
                    0,
                ),
            ],
            false,
        );
        let lyrics = lyrics.unwrap();
        assert_eq!(lyrics.format, "qrc");
        assert!(lyrics.translation.unwrap().contains("你好世界"));
        assert_eq!(requests.len(), 2);
        let request: Value = serde_json::from_str(&requests[1].1).unwrap();
        assert_eq!(request["req_1"]["param"]["songID"], 97773);
        assert_eq!(request["req_1"]["param"]["qrc"], 1);
        assert_eq!(request["req_1"]["method"], "GetPlayLyricInfo");
        assert_eq!(request["comm"]["uid"], "0");
    }
}

#[test]
fn parser_fallback_requests_lrc_from_rich_endpoint_without_retrying_qrc() {
    let (lyrics, requests) = run_chain(
        vec![(
            "/musicu",
            200,
            modern("[00:01.00]Original", "[00:01.00]译文"),
            0,
        )],
        true,
    );
    let lyrics = lyrics.unwrap();
    assert_eq!(lyrics.format, "lrc");
    assert!(lyrics.translation.is_some());
    let request: Value = serde_json::from_str(&requests[0].1).unwrap();
    assert_eq!(request["req_1"]["param"]["qrc"], 0);
}

#[test]
fn rich_endpoint_errors_continue_to_legacy_without_changing_the_candidate() {
    let (lyrics, requests) = run_chain(
        vec![
            ("/musicu", 200, "invalid json".into(), 0),
            (
                "/lrc",
                200,
                json!({"code":0,"lyric":STANDARD.encode("[00:01.00]Original"),"trans":""})
                    .to_string(),
                0,
            ),
        ],
        true,
    );
    assert_eq!(lyrics.unwrap().format, "lrc");
    assert!(requests[1].0.contains("songmid=abc"));
}

#[test]
fn missing_translation_is_supplemented_without_replacing_word_timing() {
    let fixture: Value =
        serde_json::from_str(include_str!("../../../../tests/fixtures/qrc.json")).unwrap();
    let primary = format!(
        "<response><content><![CDATA[{}]]></content></response>",
        fixture["encrypted"].as_str().unwrap()
    );
    for auxiliary in [
        modern("[00:01.00]Different segmentation", "[00:01.00]译文"),
        "invalid json".into(),
    ] {
        let (lyrics, _) = run_chain(
            vec![
                ("/qrc", 200, primary.clone(), 0),
                ("/musicu", 200, auxiliary.clone(), 0),
            ],
            false,
        );
        let lyrics = lyrics.unwrap();
        assert_eq!(lyrics.format, "qrc");
        assert_eq!(lyrics.content, fixture["main"].as_str().unwrap());
        assert_eq!(lyrics.translation.is_some(), auxiliary != "invalid json");
    }
}

#[test]
fn supplement_keeps_existing_auxiliary_and_combined_payload_bounded() {
    let lyrics = |text: &str| Lyrics {
        match_score: None,
        source: "qq".into(),
        format: "lrc".into(),
        content: text.into(),
        translation: None,
        romanization: None,
    };
    let mut original = lyrics(&"a".repeat(MAX_LYRIC_BYTES - 3));
    let mut extra = lyrics("unused");
    extra.translation = Some("译".into());
    extra.romanization = Some("too large".into());
    supplement(&mut original, extra);
    assert_eq!(original.translation.as_deref(), Some("译"));
    assert!(original.romanization.is_none());
    let mut extra = lyrics("unused");
    extra.translation = Some("replacement".into());
    supplement(&mut original, extra);
    assert_eq!(original.translation.as_deref(), Some("译"));
}

#[test]
#[ignore = "requires the live QQ Music service"]
fn live_70047_rich_fallback_keeps_translation_in_both_modes() {
    let candidate = Candidate {
        id: Some(492216502),
        mid: "003ZvoTo3mBM0k".into(),
        title: "70047 (feat. RINO & Davis West)".into(),
        artists: vec!["Pretty Patterns".into(), "RINO".into(), "Davis West".into()],
        album: "70047 (feat. RINO & Davis West)".into(),
        duration: 214.0,
    };
    for skip_qrc in [false, true] {
        let value = tauri::async_runtime::block_on(play_lyric::fetch(
            &reqwest::Client::new(),
            &candidate,
            skip_qrc,
            LYRIC_ENDPOINTS.musicu,
        ))
        .unwrap()
        .unwrap();
        assert!(value
            .translation
            .as_ref()
            .is_some_and(|s| !s.trim().is_empty()));
        assert_eq!(value.format, if skip_qrc { "lrc" } else { "qrc" });
        println!(
            "QQ rich fallback: {} original={} translation={}",
            value.format,
            value.content.len(),
            value.translation.unwrap().len()
        );
    }
}
