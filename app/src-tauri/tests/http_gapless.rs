//! Online gapless regression through the same playbin and HTTP source as the app.
#[path = "support/infrastructure.rs"]
mod infrastructure;
#[path = "../src/playback/network.rs"]
mod network;
use gstreamer::{self as gst, prelude::*};
use std::{
    io::{Read, Write},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
fn wav(samples: &[i16]) -> Vec<u8> {
    let bytes = (samples.len() * 2) as u32;
    let mut out = Vec::new();
    out.extend(b"RIFF");
    out.extend((bytes + 36).to_le_bytes());
    out.extend(b"WAVEfmt ");
    out.extend(16u32.to_le_bytes());
    out.extend(1u16.to_le_bytes());
    out.extend(2u16.to_le_bytes());
    out.extend(48000u32.to_le_bytes());
    out.extend(192000u32.to_le_bytes());
    out.extend(4u16.to_le_bytes());
    out.extend(16u16.to_le_bytes());
    out.extend(b"data");
    out.extend(bytes.to_le_bytes());
    for sample in samples {
        out.extend(sample.to_le_bytes());
    }
    out
}

#[test]
fn next_http_track_read_failure_recovers_without_interrupting_current_audio() {
    gst::init().unwrap();
    network::prefer_http_source();
    let samples: Vec<i16> = (0..48_000 * 4 * 2)
        .map(|i| ((i * 37) % 20001) as i16 - 10000)
        .collect();
    let data = wav(&samples);
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let uri = format!("http://{}/audio.wav", listener.local_addr().unwrap());
    let stop = Arc::new(AtomicBool::new(false));
    let shutdown = stop.clone();
    let thread = std::thread::spawn(move || {
        let mut requests = 0;
        while !shutdown.load(Ordering::Relaxed) {
            let Ok((mut socket, _)) = listener.accept() else {
                std::thread::sleep(Duration::from_millis(2));
                continue;
            };
            socket
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut request = Vec::new();
            let mut buffer = [0; 1024];
            while !request.windows(4).any(|s| s == b"\r\n\r\n") {
                match socket.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => request.extend_from_slice(&buffer[..n]),
                }
            }
            let start = String::from_utf8_lossy(&request)
                .to_ascii_lowercase()
                .lines()
                .find_map(|s| {
                    s.strip_prefix("range: bytes=")?
                        .split('-')
                        .next()?
                        .parse::<usize>()
                        .ok()
                })
                .unwrap_or(0);
            if start >= data.len() {
                continue;
            }
            let header=format!("HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nContent-Range: bytes {}-{}/{}\r\nContent-Type: audio/wav\r\nConnection: close\r\n\r\n",data.len()-start,start,data.len()-1,data.len());
            requests += 1;
            let _ = socket.write_all(header.as_bytes());
            // First song is healthy; only the speculative next-song read is cut short.
            let end = if requests == 2 {
                data.len() * 3 / 4
            } else {
                data.len()
            };
            std::thread::sleep(Duration::from_millis(20));
            let _ = socket.write_all(&data[start..end]);
            std::thread::sleep(Duration::from_millis(20));
        }
    });
    let player = gst::ElementFactory::make("playbin3").build().unwrap();
    player.set_property_from_str("flags", "audio+soft-volume+buffering");
    player.set_property("buffer-size", 4_194_304i32);
    player.set_property("buffer-duration", 10_000_000_000i64);
    let sink = gst::ElementFactory::make("fakesink")
        .property("sync", true)
        .property("signal-handoffs", true)
        .build()
        .unwrap();
    let received = Arc::new(Mutex::new(Vec::new()));
    let captured = received.clone();
    sink.connect("handoff", false, move |values| {
        let buffer = values[1].get::<gst::Buffer>().unwrap();
        captured
            .lock()
            .unwrap()
            .extend_from_slice(buffer.map_readable().unwrap().as_slice());
        None
    });
    player.set_property("audio-sink", sink);
    player.connect("source-setup", false, |values| {
        let source = values[1].get::<gst::Element>().unwrap();
        network::configure_source(&source);
        source.set_property("proxy", "");
        None
    });
    let next = Arc::new(Mutex::new(Some(uri.clone())));
    player.connect("about-to-finish", false, move |values| {
        if let Some(uri) = next.lock().unwrap().take() {
            values[0]
                .get::<gst::Element>()
                .unwrap()
                .set_property("uri", uri);
        }
        None
    });
    player.set_property("uri", uri);
    let _ = player.set_state(gst::State::Playing);
    let deadline = Instant::now() + Duration::from_secs(15);
    let result = loop {
        if Instant::now() > deadline {
            break Err("timed out".to_string());
        }
        if let Some(message) = player
            .bus()
            .unwrap()
            .timed_pop(gst::ClockTime::from_mseconds(20))
        {
            match message.view() {
                gst::MessageView::Buffering(value) => {
                    let _ = player.set_state(if value.percent() < 100 {
                        gst::State::Paused
                    } else {
                        gst::State::Playing
                    });
                }
                gst::MessageView::Error(error) => {
                    break Err(format!(
                        "音频播放失败：{}; {:?}",
                        error.error(),
                        error.debug()
                    ))
                }
                gst::MessageView::Eos(_) => break Ok(()),
                _ => {}
            }
        }
    };
    player.set_state(gst::State::Null).unwrap();
    stop.store(true, Ordering::Relaxed);
    thread.join().unwrap();
    result.unwrap();
    let expected: Vec<u8> = samples
        .iter()
        .cycle()
        .take(samples.len() * 2)
        .flat_map(|s| s.to_le_bytes())
        .collect();
    assert_eq!(*received.lock().unwrap(), expected);
}
