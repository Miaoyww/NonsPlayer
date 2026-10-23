//! Real HTTP + decode regression: Range support without Accept-Ranges.
#[path = "support/infrastructure.rs"]
mod infrastructure;
#[path = "../src/playback/network.rs"]
#[allow(dead_code)]
mod network;
use gstreamer::{self as gst, prelude::*};
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

struct HttpAudio {
    uri: String,
    stop: Arc<AtomicBool>,
    ranges: Arc<AtomicUsize>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl HttpAudio {
    fn new(supports_ranges: bool) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let uri = format!("http://{}/audio.wav", listener.local_addr().unwrap());
        let stop = Arc::new(AtomicBool::new(false));
        let ranges = Arc::new(AtomicUsize::new(0));
        let shutdown = stop.clone();
        let requests = ranges.clone();
        let thread = std::thread::spawn(move || {
            let size = 48_000u32 * 2 * 12;
            let mut audio = Vec::new();
            audio.extend(b"RIFF");
            audio.extend((size + 36).to_le_bytes());
            audio.extend(b"WAVEfmt ");
            audio.extend(16u32.to_le_bytes());
            audio.extend(1u16.to_le_bytes());
            audio.extend(1u16.to_le_bytes());
            audio.extend(48_000u32.to_le_bytes());
            audio.extend(96_000u32.to_le_bytes());
            audio.extend(2u16.to_le_bytes());
            audio.extend(16u16.to_le_bytes());
            audio.extend(b"data");
            audio.extend(size.to_le_bytes());
            audio.resize(size as usize + 44, 0);
            while !shutdown.load(Ordering::Relaxed) {
                let Ok((mut socket, _)) = listener.accept() else {
                    std::thread::sleep(Duration::from_millis(5));
                    continue;
                };
                socket
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                socket
                    .set_write_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut request = Vec::new();
                let mut buffer = [0; 1024];
                while !request.windows(4).any(|s| s == b"\r\n\r\n") {
                    match socket.read(&mut buffer) {
                        Ok(0) | Err(_) => break,
                        Ok(count) => request.extend_from_slice(&buffer[..count]),
                    }
                    if request.len() > 8192 {
                        break;
                    }
                }
                let request = String::from_utf8_lossy(&request).to_ascii_lowercase();
                let range = request.lines().find_map(|line| {
                    line.strip_prefix("range: bytes=")?
                        .split('-')
                        .next()?
                        .parse::<usize>()
                        .ok()
                });
                let start = if supports_ranges {
                    range.unwrap_or(0)
                } else {
                    0
                };
                if start >= audio.len() {
                    continue;
                }
                if start > 0 {
                    requests.fetch_add(1, Ordering::Relaxed);
                }
                let status = if supports_ranges && range.is_some() {
                    "206 Partial Content"
                } else {
                    "200 OK"
                };
                let content_range = if supports_ranges && range.is_some() {
                    format!(
                        "Content-Range: bytes {}-{}/{}\r\n",
                        start,
                        audio.len() - 1,
                        audio.len()
                    )
                } else {
                    String::new()
                };
                // Deliberately omit Accept-Ranges, as the affected CDN does.
                let headers = format!("HTTP/1.1 {status}\r\nContent-Type: audio/wav\r\nContent-Length: {}\r\n{content_range}Connection: close\r\n\r\n", audio.len()-start);
                if socket.write_all(headers.as_bytes()).is_ok() {
                    let _ = socket.write_all(&audio[start..]);
                }
            }
        });
        Self {
            uri,
            stop,
            ranges,
            thread: Some(thread),
        }
    }
}

impl Drop for HttpAudio {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        self.thread.take().unwrap().join().unwrap();
    }
}

struct Playback(gst::Element);
impl Playback {
    fn new(uri: &str) -> Self {
        gst::init().unwrap();
        network::prefer_http_source();
        let player = gst::ElementFactory::make("playbin3").build().unwrap();
        player.set_property_from_str("flags", "audio+soft-volume+buffering");
        player.set_property("buffer-size", 4_194_304_i32);
        player.set_property("buffer-duration", 10_000_000_000_i64);
        let sink = gst::ElementFactory::make("fakesink")
            .property("sync", true)
            .build()
            .unwrap();
        player.set_property("audio-sink", sink);
        player.connect("source-setup", false, |values| {
            let source = values[1].get::<gst::Element>().unwrap();
            assert_eq!(source.factory().unwrap().name(), "nonshttpsrc");
            // Local fixture bypasses the user's network/proxy settings.
            source.set_property("proxy", "");
            None
        });
        player.set_property("uri", uri);
        player.set_state(gst::State::Playing).unwrap();
        let result = Self(player);
        result.wait_at(0, 500);
        result
    }
    fn wait_at(&self, target: u64, advance: u64) {
        let deadline = Instant::now() + Duration::from_secs(8);
        loop {
            let position = self
                .0
                .query_position::<gst::ClockTime>()
                .map(|p| p.mseconds());
            if position.is_some_and(|p| p >= target + advance && p < target + advance + 1500) {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "position did not reach {target}+{advance}: {position:?}"
            );
            if let Some(message) = self
                .0
                .bus()
                .unwrap()
                .timed_pop(gst::ClockTime::from_mseconds(20))
            {
                if let gst::MessageView::Error(error) = message.view() {
                    panic!("{}", error.error());
                }
            }
        }
    }
}
impl Drop for Playback {
    fn drop(&mut self) {
        let _ = self.0.set_state(gst::State::Null);
    }
}

#[test]
fn range_without_declaration_seeks_forward_backward_and_paused() {
    let server = HttpAudio::new(true);
    let player = Playback::new(&server.uri);
    for target in [8000, 2000] {
        player
            .0
            .seek_simple(
                gst::SeekFlags::FLUSH | gst::SeekFlags::ACCURATE,
                gst::ClockTime::from_mseconds(target),
            )
            .unwrap();
        player.wait_at(target, 100);
    }
    player.0.set_state(gst::State::Paused).unwrap();
    let _ = player.0.state(gst::ClockTime::from_seconds(3));
    player
        .0
        .seek_simple(
            gst::SeekFlags::FLUSH | gst::SeekFlags::ACCURATE,
            gst::ClockTime::from_seconds(6),
        )
        .unwrap();
    player.wait_at(6000, 0);
    assert!(
        server.ranges.load(Ordering::Relaxed) > 0,
        "seek must request a nonzero byte offset"
    );
}

#[test]
fn server_ignoring_ranges_is_not_marked_seekable() {
    let server = HttpAudio::new(false);
    let player = Playback::new(&server.uri);
    let mut query = gst::query::Seeking::new(gst::Format::Time);
    assert!(player.0.query(&mut query));
    assert!(!query.result().0);
    assert!(player
        .0
        .seek_simple(
            gst::SeekFlags::FLUSH | gst::SeekFlags::ACCURATE,
            gst::ClockTime::from_seconds(6)
        )
        .is_err());
    player.wait_at(0, 700);
}
