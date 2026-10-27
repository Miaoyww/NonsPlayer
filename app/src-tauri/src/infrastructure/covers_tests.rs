use super::*;
use std::sync::atomic::AtomicUsize;

fn image(red: u8) -> Vec<u8> {
    let mut output = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(2, 2, image::Rgb([red, 0, 0])))
        .write_to(&mut output, image::ImageFormat::Png)
        .unwrap();
    output.into_inner()
}

fn downloaded(red: u8) -> Downloaded {
    Downloaded {
        bytes: image(red),
        mime: "image/png".into(),
    }
}

#[tokio::test]
async fn warm_disk_hit_survives_cache_restart_without_downloading() {
    let root = tempfile::tempdir().unwrap();
    let cache = Arc::new(CoverCache::new(root.path().into()));
    let first = cache
        .resolve_with(
            "https://cover.test/128.png".into(),
            || async { Ok(downloaded(1)) },
            || {},
            now(),
        )
        .await
        .unwrap();
    assert_eq!(std::fs::read(&first.asset.path).unwrap(), image(1));
    assert!(!first.asset.path.to_string_lossy().starts_with("data:"));
    drop(first);
    drop(cache);
    let restarted = Arc::new(CoverCache::new(root.path().into()));
    let hit = restarted
        .resolve_with(
            "https://cover.test/128.png".into(),
            || async { panic!("warm cover must not use network") },
            || {},
            now(),
        )
        .await
        .unwrap();
    assert_eq!(std::fs::read(&hit.asset.path).unwrap(), image(1));
}

#[tokio::test]
async fn stale_cover_returns_immediately_and_refresh_publishes_a_new_asset() {
    let root = tempfile::tempdir().unwrap();
    let cache = Arc::new(CoverCache::new(root.path().into()));
    let url = "https://cover.test/256.png".to_string();
    let first = cache
        .resolve_with(url.clone(), || async { Ok(downloaded(2)) }, || {}, now())
        .await
        .unwrap();
    let started = Arc::new(tokio::sync::Notify::new());
    let release = Arc::new(tokio::sync::Notify::new());
    let refreshed = Arc::new(tokio::sync::Notify::new());
    let stale = cache
        .resolve_with(
            url.clone(),
            {
                let started = started.clone();
                let release = release.clone();
                move || {
                    let started = started.clone();
                    let release = release.clone();
                    async move {
                        started.notify_one();
                        release.notified().await;
                        Ok(downloaded(3))
                    }
                }
            },
            {
                let refreshed = refreshed.clone();
                move || refreshed.notify_one()
            },
            now() + FRESH_SECONDS + 1,
        )
        .await
        .unwrap();
    assert_eq!(stale.asset.path, first.asset.path);
    tokio::time::timeout(Duration::from_secs(3), started.notified())
        .await
        .unwrap();
    assert_eq!(std::fs::read(&stale.asset.path).unwrap(), image(2));
    release.notify_one();
    tokio::time::timeout(Duration::from_secs(3), refreshed.notified())
        .await
        .unwrap();
    let updated = cache
        .resolve_with(
            url,
            || async { panic!("refreshed cover must be on disk") },
            || {},
            now(),
        )
        .await
        .unwrap();
    assert_ne!(updated.asset.path, first.asset.path);
    assert_eq!(std::fs::read(updated.asset.path).unwrap(), image(3));
}

#[tokio::test]
async fn failed_background_refresh_preserves_stale_cover_and_is_backed_off() {
    let root = tempfile::tempdir().unwrap();
    let cache = Arc::new(CoverCache::new(root.path().into()));
    let url = "https://cover.test/stale.png".to_string();
    let first = cache
        .resolve_with(url.clone(), || async { Ok(downloaded(4)) }, || {}, now())
        .await
        .unwrap();
    let attempts = Arc::new(AtomicUsize::new(0));
    let finished = Arc::new(tokio::sync::Notify::new());
    let loader = {
        let attempts = attempts.clone();
        let finished = finished.clone();
        move || {
            let attempts = attempts.clone();
            let finished = finished.clone();
            async move {
                attempts.fetch_add(1, Ordering::SeqCst);
                finished.notify_one();
                Err("network failure".into())
            }
        }
    };
    let stale = cache
        .resolve_with(
            url.clone(),
            loader.clone(),
            || panic!("failed refresh must not emit"),
            now() + FRESH_SECONDS + 1,
        )
        .await
        .unwrap();
    assert_eq!(stale.asset.path, first.asset.path);
    tokio::time::timeout(Duration::from_secs(3), finished.notified())
        .await
        .unwrap();
    cache
        .resolve_with(
            url,
            loader,
            || panic!("backed off refresh must not emit"),
            now() + FRESH_SECONDS + 1,
        )
        .await
        .unwrap();
    assert_eq!(attempts.load(Ordering::SeqCst), 1);
    assert_eq!(std::fs::read(first.asset.path).unwrap(), image(4));
}

#[tokio::test]
async fn concurrent_cold_requests_download_once() {
    let root = tempfile::tempdir().unwrap();
    let cache = Arc::new(CoverCache::new(root.path().into()));
    let started = Arc::new(tokio::sync::Notify::new());
    let release = Arc::new(tokio::sync::Notify::new());
    let loads = Arc::new(AtomicUsize::new(0));
    let loader = {
        let started = started.clone();
        let release = release.clone();
        let loads = loads.clone();
        move || {
            let started = started.clone();
            let release = release.clone();
            let loads = loads.clone();
            async move {
                loads.fetch_add(1, Ordering::SeqCst);
                started.notify_one();
                release.notified().await;
                Ok(downloaded(5))
            }
        }
    };
    let mut requests = Vec::new();
    for _ in 0..16 {
        let cache = cache.clone();
        let loader = loader.clone();
        requests.push(tokio::spawn(async move {
            cache
                .resolve_with(
                    "https://cover.test/coalesced.png".into(),
                    loader,
                    || {},
                    now(),
                )
                .await
        }));
    }
    tokio::time::timeout(Duration::from_secs(3), started.notified())
        .await
        .unwrap();
    release.notify_one();
    for request in requests {
        tokio::time::timeout(Duration::from_secs(3), request)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
    }
    assert_eq!(loads.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn clear_rejects_a_download_started_before_clearing() {
    let root = tempfile::tempdir().unwrap();
    let cache = Arc::new(CoverCache::new(root.path().into()));
    let started = Arc::new(tokio::sync::Notify::new());
    let release = Arc::new(tokio::sync::Notify::new());
    let one = {
        let cache = cache.clone();
        let started = started.clone();
        let release = release.clone();
        tokio::spawn(async move {
            cache
                .resolve_with(
                    "https://cover.test/inflight.png".into(),
                    move || {
                        let started = started.clone();
                        let release = release.clone();
                        async move {
                            started.notify_one();
                            release.notified().await;
                            Ok(downloaded(5))
                        }
                    },
                    || {},
                    now(),
                )
                .await
        })
    };
    tokio::time::timeout(Duration::from_secs(3), started.notified())
        .await
        .unwrap();
    cache.clear().await.unwrap();
    release.notify_one();
    assert!(tokio::time::timeout(Duration::from_secs(3), one)
        .await
        .unwrap()
        .unwrap()
        .is_err());
    assert!(cache
        .disk()
        .await
        .unwrap()
        .lookup("https://cover.test/inflight.png")
        .unwrap()
        .asset
        .is_none());
}

#[tokio::test]
async fn missing_file_is_downloaded_again_and_invalidated_old_assets_cannot_reappear() {
    let root = tempfile::tempdir().unwrap();
    let cache = Arc::new(CoverCache::new(root.path().into()));
    let url = "https://cover.test/missing.png".to_string();
    let first = cache
        .resolve_with(url.clone(), || async { Ok(downloaded(6)) }, || {}, now())
        .await
        .unwrap();
    std::fs::remove_file(&first.asset.path).unwrap();
    let next = cache
        .resolve_with(url.clone(), || async { Ok(downloaded(7)) }, || {}, now())
        .await
        .unwrap();
    assert_eq!(std::fs::read(&next.asset.path).unwrap(), image(7));
    cache.invalidate(url.clone()).await.unwrap();
    assert!(next.disk.read(&next.asset).is_err());
    assert!(next.disk.lookup(&url).unwrap().asset.is_none());
}

#[tokio::test]
async fn failed_cover_transport_preserves_timeout_cause_and_stack() {
    crate::infrastructure::diagnostics::test_log::start();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_millis(30))
        .build()
        .unwrap();
    let error = download(
        url::Url::parse(&format!("http://{address}/cover.png")).unwrap(),
        &client,
    )
    .await
    .unwrap_err();
    assert!(error.downcast_ref::<reqwest::Error>().unwrap().is_timeout());
    crate::infrastructure::diagnostics::report_error("covers.load", &error);
    let output = crate::infrastructure::diagnostics::test_log::take().join("\n");
    assert!(output.contains("covers.http.send"), "{output}");
    assert!(output.contains("operation timed out"), "{output}");
    assert!(output.contains("backtrace_kind="), "{output}");
    assert!(!output.contains(&address.to_string()), "{output}");
}

#[test]
fn supported_mime_types_remain_explicit() {
    assert_eq!(image_mime("image/jpg"), Some("image/jpeg"));
    assert_eq!(image_mime("IMAGE/JPEG; charset=binary"), Some("image/jpeg"));
    assert_eq!(image_mime("application/octet-stream"), None);
}

#[tokio::test]
async fn covers_larger_than_eight_mib_download_and_round_trip_through_cache() {
    use std::io::{Read, Write};

    let payload = vec![7u8; 8 * 1024 * 1024 + 1];
    for chunked in [false, true] {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let body = payload.clone();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(10)))
                .unwrap();
            stream
                .set_write_timeout(Some(Duration::from_secs(10)))
                .unwrap();
            let mut request = Vec::new();
            while !request.ends_with(b"\r\n\r\n") {
                let mut byte = [0];
                stream.read_exact(&mut byte).unwrap();
                request.push(byte[0]);
            }
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nConnection: close\r\n"
            )
            .unwrap();
            if chunked {
                write!(stream, "Transfer-Encoding: chunked\r\n\r\n").unwrap();
                for chunk in body.chunks(64 * 1024) {
                    write!(stream, "{:x}\r\n", chunk.len()).unwrap();
                    stream.write_all(chunk).unwrap();
                    stream.write_all(b"\r\n").unwrap();
                }
                stream.write_all(b"0\r\n\r\n").unwrap();
            } else {
                write!(stream, "Content-Length: {}\r\n\r\n", body.len()).unwrap();
                stream.write_all(&body).unwrap();
            }
        });
        let client = reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap();
        let url = format!("http://{address}/cover.png");
        let result = download(url::Url::parse(&url).unwrap(), &client).await;
        server.join().unwrap();
        let downloaded = result.unwrap();
        assert_eq!(downloaded.bytes, payload);

        let root = tempfile::tempdir().unwrap();
        let cache = Arc::new(CoverCache::new(root.path().into()));
        let resolved = cache
            .resolve_with(
                url,
                move || {
                    let bytes = downloaded.bytes.clone();
                    async move {
                        Ok(Downloaded {
                            bytes,
                            mime: "image/png".into(),
                        })
                    }
                },
                || {},
                now(),
            )
            .await
            .unwrap();
        assert_eq!(resolved.disk.read(&resolved.asset).unwrap(), payload);
    }
}
