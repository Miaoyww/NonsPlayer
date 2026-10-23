//! Exercise the actual bundled source when the connection closes in the song's tail.
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
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

fn receive_tail(
    disconnects: usize,
    ranges: bool,
    retries: u32,
    stall: bool,
) -> (gst::Message, Vec<u8>, Vec<usize>, Vec<u8>) {
    gst::init().unwrap();
    network::prefer_http_source();
    let expected: Vec<u8> = (0..65_536).map(|i| (i % 251) as u8).collect();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let uri = format!("http://{}/tail", listener.local_addr().unwrap());
    let stop = Arc::new(AtomicBool::new(false));
    let shutdown = stop.clone();
    let offsets = Arc::new(Mutex::new(Vec::new()));
    let requests = offsets.clone();
    let data = expected.clone();
    let server = std::thread::spawn(move || {
        while !shutdown.load(Ordering::Relaxed) {
            let Ok((mut socket, _)) = listener.accept() else {
                std::thread::sleep(Duration::from_millis(2));
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
            while !request.windows(4).any(|s| s == b"\r\n\r\n") && request.len() < 8192 {
                match socket.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(size) => request.extend_from_slice(&buffer[..size]),
                }
            }
            let request = String::from_utf8_lossy(&request).to_ascii_lowercase();
            let start = if ranges {
                request
                    .lines()
                    .find_map(|line| {
                        line.strip_prefix("range: bytes=")?
                            .split('-')
                            .next()?
                            .parse::<usize>()
                            .ok()
                    })
                    .unwrap_or(0)
            } else {
                0
            };
            let mut offsets = requests.lock().unwrap();
            offsets.push(start);
            let attempt = offsets.len();
            drop(offsets);
            let range = if ranges {
                format!(
                    "Content-Range: bytes {}-{}/{}\r\n",
                    start,
                    data.len() - 1,
                    data.len()
                )
            } else {
                String::new()
            };
            let status = if ranges {
                "206 Partial Content"
            } else {
                "200 OK"
            };
            let headers = format!(
                "HTTP/1.1 {status}\r\nContent-Length: {}\r\n{range}Connection: close\r\n\r\n",
                data.len() - start
            );
            let end = if attempt <= disconnects {
                start + (data.len() - start) * 3 / 4
            } else {
                data.len()
            };
            let _ = socket.write_all(headers.as_bytes());
            // Let send() deliver the headers before simulating a body-read failure.
            std::thread::sleep(Duration::from_millis(20));
            let _ = socket.write_all(&data[start..end]);
            std::thread::sleep(Duration::from_millis(20));
            if stall && attempt <= disconnects {
                std::thread::sleep(Duration::from_millis(1300));
            }
            // Socket closes before the advertised Content-Length on selected requests.
        }
    });
    let source = gst::ElementFactory::make("nonshttpsrc")
        .property("location", uri)
        .property("proxy", "")
        .property("timeout", 1u32)
        .build()
        .unwrap();
    network::configure_source(&source);
    source.set_property("proxy", "");
    source.set_property("timeout", 1u32);
    source.set_property("retries", retries);
    let sink = gst::ElementFactory::make("fakesink")
        .property("sync", false)
        .property("signal-handoffs", true)
        .build()
        .unwrap();
    let bytes = Arc::new(Mutex::new(Vec::new()));
    let captured = bytes.clone();
    sink.connect("handoff", false, move |values| {
        let buffer = values[1].get::<gst::Buffer>().unwrap();
        let mut bytes = captured.lock().unwrap();
        assert_eq!(buffer.offset(), bytes.len() as u64);
        bytes.extend_from_slice(buffer.map_readable().unwrap().as_slice());
        None
    });
    let pipeline = gst::Pipeline::new();
    pipeline.add_many([&source, &sink]).unwrap();
    source.link(&sink).unwrap();
    let _ = pipeline.set_state(gst::State::Playing);
    let message = pipeline
        .bus()
        .unwrap()
        .timed_pop_filtered(
            gst::ClockTime::from_seconds(5),
            &[gst::MessageType::Eos, gst::MessageType::Error],
        )
        .expect("source timed out");
    pipeline.set_state(gst::State::Null).unwrap();
    stop.store(true, Ordering::Relaxed);
    server.join().unwrap();
    let received = bytes.lock().unwrap().clone();
    let offsets = offsets.lock().unwrap().clone();
    (message, received, offsets, expected)
}

#[test]
fn tail_disconnect_resumes_without_missing_or_repeating_bytes() {
    let (message, received, offsets, expected) = receive_tail(1, true, 2, false);
    if let gst::MessageView::Error(error) = message.view() {
        panic!("音频播放失败：{}; {:?}", error.error(), error.debug());
    }
    assert_eq!(message.type_(), gst::MessageType::Eos);
    assert_eq!(received, expected);
    assert_eq!(offsets.len(), 2);
    assert_eq!(offsets[1], 49_152);
}

#[test]
fn repeated_disconnects_stop_after_two_retries() {
    let (message, _, offsets, _) = receive_tail(usize::MAX, true, 2, false);
    assert_eq!(message.type_(), gst::MessageType::Error);
    if offsets.len() != 3 {
        if let gst::MessageView::Error(error) = message.view() {
            panic!(
                "requests={offsets:?}: {} {:?}",
                error.error(),
                error.debug()
            );
        }
    }
    assert_eq!(offsets.len(), 3);
}

#[test]
fn nonseekable_disconnect_is_reported_instead_of_replaying_bytes() {
    let (message, _, offsets, _) = receive_tail(1, false, 2, false);
    assert_eq!(message.type_(), gst::MessageType::Error);
    assert_eq!(offsets.len(), 1);
}

#[test]
fn tail_read_timeout_resumes_from_the_last_emitted_byte() {
    let (message, received, offsets, expected) = receive_tail(1, true, 2, true);
    assert_eq!(message.type_(), gst::MessageType::Eos, "{message:?}");
    assert_eq!(received, expected);
    assert_eq!(offsets, vec![0, 49_152]);
}

#[test]
fn retries_can_be_disabled() {
    let (message, _, offsets, _) = receive_tail(1, true, 0, false);
    assert_eq!(message.type_(), gst::MessageType::Error);
    assert_eq!(offsets.len(), 1);
}
