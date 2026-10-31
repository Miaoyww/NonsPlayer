use super::*;
use crate::model::{Track, TrackSource};
use std::io::Write;

fn wave(rate: u32, channels: u16, frames: usize) -> Arc<tempfile::NamedTempFile> {
    let mut file = tempfile::NamedTempFile::new().unwrap();
    let size = (frames * channels as usize * 2) as u32;
    file.write_all(b"RIFF").unwrap();
    file.write_all(&(size + 36).to_le_bytes()).unwrap();
    file.write_all(b"WAVEfmt \x10\0\0\0\x01\0").unwrap();
    file.write_all(&channels.to_le_bytes()).unwrap();
    file.write_all(&rate.to_le_bytes()).unwrap();
    file.write_all(&(rate * u32::from(channels) * 2).to_le_bytes())
        .unwrap();
    file.write_all(&(channels * 2).to_le_bytes()).unwrap();
    file.write_all(&16u16.to_le_bytes()).unwrap();
    file.write_all(b"data").unwrap();
    file.write_all(&size.to_le_bytes()).unwrap();
    for frame in 0..frames {
        let value = (frame as i16).wrapping_mul(7);
        for _ in 0..channels {
            file.write_all(&value.to_le_bytes()).unwrap();
        }
    }
    file.flush().unwrap();
    Arc::new(file)
}

fn resolved(file: Arc<tempfile::NamedTempFile>) -> ResolvedTrack {
    ResolvedTrack {
        uri: url::Url::from_file_path(file.path()).unwrap().into(),
        decoded_audio: Some(file.clone()),
        quality: None,
        track: Track {
            key: "test".into(),
            title: "test".into(),
            aliases: vec![],
            artist: String::new(),
            artists: vec![],
            album: String::new(),
            duration_ms: 0,
            cover: String::new().into(),
            source: TrackSource::Local {
                path: file.path().to_string_lossy().into_owned(),
                netease_id: None,
                cue: None,
            },
        },
    }
}

fn decode_all(resolved: ResolvedTrack, rate: u32, channels: usize, position: u64) -> Vec<f32> {
    let decoder = Decoder::spawn(resolved, String::new(), rate, channels, position).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut samples = Vec::new();
    loop {
        let done = decoder.buffer.done.load(Ordering::Acquire);
        if let Some(chunk) = decoder.buffer.samples.pop() {
            samples.extend_from_slice(&chunk);
            decoder.buffer.recycle(chunk);
        } else if done {
            break;
        } else {
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(Instant::now() < deadline, "decoder stalled");
    }
    assert!(
        !decoder.buffer.failed.load(Ordering::Acquire),
        "{:?}",
        decoder.buffer.error.lock().unwrap()
    );
    samples
}

fn voice(
    rate: u32,
    channels: usize,
    duration_ms: u64,
    start_ms: u64,
    value: f32,
    frames: usize,
) -> Voice {
    let buffer = buffer::Buffer::new(rate, channels, duration_ms);
    for chunk in vec![value; frames * channels].chunks(buffer::CHUNK_FRAMES * channels) {
        let mut samples = buffer.take_chunk().unwrap();
        samples.extend_from_slice(chunk);
        buffer.samples.push(samples).unwrap();
    }
    buffer.done.store(true, Ordering::Release);
    Voice::new(buffer, start_ms, start_ms, value.to_string())
}

#[test]
fn ffmpeg_preserves_short_tail_and_accurately_seeks_inside_a_frame() {
    let file = wave(48000, 2, 10_007);
    let all = decode_all(resolved(file.clone()), 48000, 2, 0);
    assert_eq!(all.len(), 20_014);
    let seeked = decode_all(resolved(file), 48000, 2, 73);
    assert_eq!(seeked, all[73 * 48 * 2..]);
}

#[test]
fn resampling_drains_delayed_samples_and_mono_is_remixed_to_stereo() {
    let all = decode_all(resolved(wave(44100, 1, 4410)), 48000, 2, 0);
    assert!((all.len() as i64 - 9600).abs() <= 2, "{}", all.len());
    assert!(all.chunks_exact(2).all(|frame| frame[0] == frame[1]));
}

#[test]
fn cue_seek_and_end_are_enforced_at_the_pcm_boundary() {
    let mut track = resolved(wave(48000, 2, 48_000));
    if let TrackSource::Local { cue, .. } = &mut track.track.source {
        *cue = Some(nons_music_adapter_sdk::queue::CueTrack {
            sheet_path: "test.cue".into(),
            track_number: 2,
            start_ms: 100,
            end_ms: 300,
        });
    }
    let all = decode_all(track, 48000, 2, 75);
    assert_eq!(all.len(), 125 * 48 * 2);
    assert!((all[0] - (175i16 * 48).wrapping_mul(7) as f32 / 32768.0).abs() < 0.0001);
}

#[test]
fn stalled_decoder_has_bounded_memory_and_stop_releases_full_buffer() {
    let decoder =
        Decoder::spawn(resolved(wave(48000, 2, 96_000)), String::new(), 48000, 2, 0).unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    while !decoder.buffer.samples.is_full() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(decoder.buffer.samples.is_full());
    assert!(decoder.buffer.samples.capacity() * buffer::CHUNK_FRAMES <= 25_600);
    let start = Instant::now();
    drop(decoder);
    assert!(start.elapsed() < Duration::from_millis(200));
}

#[test]
fn silence_during_underrun_does_not_advance_the_media_clock() {
    let current = voice(1000, 2, 1000, 700, 0.5, 10);
    let mut mixer = Mixer::new(current, true, 1.0, false, 0);
    let mut output = [0.0f32; 40];
    mixer.render(&mut output);
    assert_eq!(&output[..20], &[0.5; 20]);
    assert_eq!(&output[20..], &[0.0; 20]);
    assert_eq!(mixer.current.position_ms(), 710);
    assert!(mixer.current.finished());
}

#[test]
fn pause_fade_reverses_from_current_gain_and_freezes_when_paused() {
    let mut mixer = Mixer::new(voice(1000, 2, 1000, 0, 0.8, 500), true, 1.0, false, 0);
    mixer.configure(true, 0);
    mixer.intent(false, 1.0);
    let mut down = [0.0f32; 100];
    mixer.render(&mut down);
    assert!((down[98] - 0.6).abs() < 0.001);
    mixer.intent(true, 1.0);
    let mut up = [0.0f32; 100];
    mixer.render(&mut up);
    assert!(up[0] >= down[98] && up[0] < 0.61);
    mixer.intent(false, 1.0);
    mixer.render(&mut [0.0f32; 400]);
    let position = mixer.current.position_ms();
    let mut silence = [1.0f32; 100];
    mixer.render(&mut silence);
    assert_eq!(silence, [0.0; 100]);
    assert_eq!(mixer.current.position_ms(), position);
}

#[test]
fn crossfade_uses_one_output_clock_and_preserves_channel_frames() {
    let mut mixer = Mixer::new(voice(1000, 2, 100, 0, 0.8, 100), true, 1.0, false, 40);
    mixer.next = Some(voice(1000, 2, 100, 0, 0.2, 100));
    let mut output = [0.0f32; 200];
    mixer.render(&mut output);
    assert!(mixer.transition_started && mixer.transition_finished);
    assert_eq!(mixer.current.position_ms(), 40);
    assert_eq!(mixer.outgoing.as_ref().unwrap().position_ms(), 100);
    assert!(output.chunks_exact(2).all(|frame| frame[0] == frame[1]));
    assert!((output[120] - 0.785).abs() < 0.001);
    assert!((output[198] - 0.2).abs() < 0.001);
    assert!(output
        .iter()
        .all(|sample| sample.is_finite() && *sample <= 0.8));
}

#[test]
fn crossfade_freezes_during_pause_and_does_not_jump_gain_on_starvation() {
    let mut mixer = Mixer::new(voice(1000, 2, 100, 0, 0.8, 100), true, 1.0, false, 40);
    let next = voice(1000, 2, 100, 0, 0.2, 10);
    next.buffer.done.store(false, Ordering::Release);
    let buffer = next.buffer.clone();
    mixer.next = Some(next);
    mixer.render(&mut [0.0f32; 140]);
    assert_eq!(mixer.current.position_ms(), 10);
    mixer.intent(false, 1.0);
    mixer.render(&mut [0.0f32; 100]);
    assert_eq!(mixer.current.position_ms(), 10);
    mixer.intent(true, 1.0);
    mixer.render(&mut [0.0f32; 20]);
    assert_eq!(mixer.outgoing.as_ref().unwrap().position_ms(), 70);
    let mut samples = buffer.take_chunk().unwrap();
    samples.extend_from_slice(&[0.2; 20]);
    buffer.samples.push(samples).unwrap();
    let mut output = [0.0f32; 20];
    mixer.render(&mut output);
    assert!((output[0] - 0.635).abs() < 0.001);
    assert_eq!(mixer.current.position_ms(), 20);
}

#[test]
fn short_incoming_limits_overlap_and_early_eof_releases_the_transition() {
    let mut mixer = Mixer::new(voice(1000, 2, 100, 0, 0.8, 100), true, 1.0, false, 80);
    mixer.next = Some(voice(1000, 2, 20, 0, 0.2, 20));
    mixer.render(&mut [0.0f32; 158]);
    assert!(!mixer.transition_started);
    mixer.render(&mut [0.0f32; 42]);
    assert!(mixer.transition_finished);
    assert_eq!(mixer.outgoing.as_ref().unwrap().position_ms(), 100);
    assert_eq!(mixer.current.position_ms(), 20);

    let mut mixer = Mixer::new(voice(1000, 2, 100, 0, 0.8, 100), true, 1.0, false, 80);
    // The declared duration may exceed the actual decoded audio.
    mixer.next = Some(voice(1000, 2, 100, 0, 0.2, 20));
    let mut output = [0.0f32; 200];
    mixer.render(&mut output);
    assert!(mixer.transition_finished);
    assert_eq!(mixer.outgoing.as_ref().unwrap().position_ms(), 100);
    assert!(
        output[100] > 0.0,
        "outgoing tail was discarded after incoming EOF"
    );
}

#[test]
fn play_fade_waits_for_pcm_but_pause_can_finish_during_buffering() {
    let current = voice(1000, 2, 1000, 0, 0.8, 0);
    current.buffer.done.store(false, Ordering::Release);
    let buffer = current.buffer.clone();
    let mut mixer = Mixer::new(current, true, 1.0, true, 0);
    mixer.render(&mut [0.0f32; 800]);
    let mut chunk = buffer.take_chunk().unwrap();
    chunk.extend_from_slice(&[0.8; 20]);
    buffer.samples.push(chunk).unwrap();
    let mut output = [0.0f32; 20];
    mixer.render(&mut output);
    assert!((output[0] - 0.004).abs() < 0.0001);
    mixer.intent(false, 1.0);
    mixer.render(&mut [0.0f32; 400]);
    assert_eq!(mixer.current.position_ms(), 10);
    buffer.done.store(true, Ordering::Release);
}

#[test]
fn cancelling_preload_preserves_a_promoted_transition_and_its_gain() {
    let mut mixer = Mixer::new(voice(1000, 2, 100, 0, 0.8, 100), true, 1.0, false, 40);
    mixer.next = Some(voice(1000, 2, 100, 0, 0.2, 100));
    mixer.render(&mut [0.0f32; 140]);
    let mut engine = AudioEngine::new().unwrap();
    engine.session = Some(Session {
        stream: None,
        mixer: Arc::new(Mutex::new(mixer)),
        decoders: vec![],
        output_failed: Arc::new(AtomicBool::new(false)),
        latency_ms: Arc::new(AtomicU64::new(0)),
        drained_at: None,
    });
    // The actor has already promoted the incoming voice, so control.next is empty.
    engine.cancel_prepared().unwrap();
    let mut mixer = engine.session.as_ref().unwrap().mixer.lock().unwrap();
    assert!(mixer.outgoing.is_some());
    let mut output = [0.0f32; 2];
    mixer.render(&mut output);
    assert!((output[0] - 0.635).abs() < 0.001);
}

#[test]
fn output_failure_is_distinct_from_decode_failure_and_early_eof_waits_for_outgoing() {
    let mut mixer = Mixer::new(voice(1000, 2, 100, 0, 0.8, 100), true, 1.0, false, 80);
    mixer.next = Some(voice(1000, 2, 100, 0, 0.2, 10));
    mixer.render(&mut [0.0f32; 80]);
    assert!(mixer.current.finished() && !mixer.transition_finished);
    let failed = Arc::new(AtomicBool::new(false));
    let mut engine = AudioEngine::new().unwrap();
    engine.session = Some(Session {
        stream: None,
        mixer: Arc::new(Mutex::new(mixer)),
        decoders: vec![],
        output_failed: failed.clone(),
        latency_ms: Arc::new(AtomicU64::new(0)),
        drained_at: None,
    });
    assert_ne!(engine.poll().unwrap().kind, FeedbackKind::Ended);
    failed.store(true, Ordering::Release);
    assert_eq!(engine.poll().unwrap().kind, FeedbackKind::DeviceDefault);
    assert_eq!(engine.poll().unwrap().kind, FeedbackKind::DeviceError);
    assert_eq!(
        output_drain_ms(Duration::from_millis(12), 960, 48000, 2),
        22
    );
    assert_eq!(output_drain_ms(Duration::ZERO, 2, 48000, 2), 1);
}

#[test]
fn preload_reuses_decoded_pcm_and_poll_promotes_before_reporting_the_next_clock() {
    let mut engine = AudioEngine::new().unwrap();
    let first = Decoder::spawn(resolved(wave(48000, 2, 9600)), String::new(), 48000, 2, 0).unwrap();
    let second =
        Decoder::spawn(resolved(wave(48000, 2, 9600)), String::new(), 48000, 2, 0).unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    while (!first.buffer.done.load(Ordering::Acquire)
        || !second.buffer.done.load(Ordering::Acquire))
        && Instant::now() < deadline
    {
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(
        first.buffer.done.load(Ordering::Acquire) && second.buffer.done.load(Ordering::Acquire)
    );
    let next = second.buffer.clone();
    let mut mixer = Mixer::new(
        Voice::new(first.buffer.clone(), 0, 0, "first".into()),
        true,
        1.0,
        false,
        100,
    );
    mixer.next = Some(Voice::new(next.clone(), 2000, 2000, "next".into()));
    mixer.render(&mut vec![0.0f32; 150 * 48 * 2]);
    assert!(mixer.transition_started && !mixer.transition_finished);
    engine.session = Some(Session {
        stream: None,
        mixer: Arc::new(Mutex::new(mixer)),
        decoders: vec![first, second],
        output_failed: Arc::new(AtomicBool::new(false)),
        latency_ms: Arc::new(AtomicU64::new(0)),
        drained_at: None,
    });
    engine.control.next = Some(NextSource {
        source: "next".into(),
        start_ms: 2000,
        end_ms: Some(2200),
    });
    let feedback = engine.poll().unwrap();
    assert!(feedback.kind == FeedbackKind::Transition);
    assert_eq!(feedback.next_source.as_deref(), Some("next"));
    assert_eq!(feedback.position_ms, 2050);
    assert_eq!(
        crate::audio::track_position(
            Some(&{
                let mut track = resolved(wave(48000, 2, 48)).track;
                if let TrackSource::Local { cue, .. } = &mut track.source {
                    *cue = Some(nons_music_adapter_sdk::queue::CueTrack {
                        sheet_path: "x".into(),
                        track_number: 1,
                        start_ms: 2000,
                        end_ms: 2200,
                    });
                }
                track
            }),
            feedback.position_ms
        ),
        50
    );
    assert!(Arc::ptr_eq(
        &engine
            .session
            .as_ref()
            .unwrap()
            .mixer
            .lock()
            .unwrap()
            .current
            .buffer,
        &next
    ));
}

#[test]
#[ignore = "requires an available physical audio output"]
fn physical_output_consumes_ffmpeg_pcm_and_reports_eof_after_device_drain() {
    let mut engine = AudioEngine::new().unwrap();
    assert!(!output_devices().unwrap().is_empty());
    engine
        .load_internal(resolved(wave(48000, 2, 14_400)), None, 0, 0, true, 0.0)
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut progressed = false;
    let mut ended = false;
    while Instant::now() < deadline {
        if let Some(feedback) = engine.poll() {
            assert!(feedback.error.is_none(), "{:?}", feedback.error);
            assert!(feedback.kind != FeedbackKind::DeviceDefault);
            progressed |= feedback.position_ms > 0;
            if feedback.kind == FeedbackKind::Ended {
                ended = true;
                break;
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(progressed && ended, "physical output never drained");
    let old_serial = engine.control.serial;
    engine
        .load_internal(resolved(wave(48000, 2, 14_400)), None, 0, 100, false, 0.0)
        .unwrap();
    let feedback = engine.poll().unwrap();
    assert!(feedback.kind == FeedbackKind::Paused);
    assert_eq!(feedback.position_ms, 100);
    assert!(feedback.serial > old_serial);
}

#[tokio::test]
async fn ffmpeg_http_uses_controlled_ranges_and_stop_cancels_a_hung_probe() {
    use axum::{
        body::Body,
        http::{Request, Response, StatusCode},
        Router,
    };
    let file = wave(48000, 2, 24_000);
    let bytes = Arc::new(std::fs::read(file.path()).unwrap());
    let requests = Arc::new(AtomicU64::new(0));
    let observed = requests.clone();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let upstream = tokio::spawn(async move {
        axum::serve(
            listener,
            Router::new().fallback(move |request: Request<Body>| {
                let bytes = bytes.clone();
                let requests = observed.clone();
                async move {
                    requests.fetch_add(1, Ordering::Relaxed);
                    assert_eq!(
                        request.headers().get("authorization").unwrap(),
                        "Bearer audio-fixture"
                    );
                    if request.uri().path() == "/hang" {
                        std::future::pending::<()>().await;
                    }
                    let range = request.headers().get("range").unwrap().to_str().unwrap();
                    let mut parts = range.strip_prefix("bytes=").unwrap().split('-');
                    let start: usize = parts.next().unwrap().parse().unwrap();
                    let end = parts
                        .next()
                        .filter(|end| !end.is_empty())
                        .map(|end| end.parse::<usize>().unwrap())
                        .unwrap_or(bytes.len() - 1)
                        .min(bytes.len() - 1);
                    Response::builder()
                        .status(StatusCode::PARTIAL_CONTENT)
                        .header(
                            "content-range",
                            format!("bytes {start}-{end}/{}", bytes.len()),
                        )
                        .header("content-length", end - start + 1)
                        .body(Body::from(bytes[start..=end].to_vec()))
                        .unwrap()
                }
            }),
        )
        .await
        .unwrap();
    });
    let mut transport = WebAudio::new().unwrap();
    let mut source = resolved(file);
    source.decoded_audio = None;
    source.uri = format!("http://{address}/audio");
    source.track.source = TrackSource::Online {
        track: Box::new(nons_music_adapter_sdk::identity::MusicTrack {
            reference: nons_music_adapter_sdk::identity::EntityRef {
                source: "test".to_owned().try_into().unwrap(),
                kind: nons_music_adapter_sdk::identity::EntityKind::Track,
                id: "1".to_owned().try_into().unwrap(),
            },
            title: "test".into(),
            aliases: vec![],
            artist: "test".into(),
            artists: vec![],
            album: String::new(),
            album_reference: None,
            duration_ms: 500,
            cover: Default::default(),
            associations: vec![],
            lyric_identifiers: vec![],
            unavailable_reason: None,
        }),
    };
    let ticket = crate::music::manager::ResourceTicket::for_audio_test(
        source.uri.clone(),
        vec![("Authorization".into(), "Bearer audio-fixture".into())],
    );
    let uri = transport
        .install_source(source.clone(), Some(ticket.clone()), false)
        .unwrap();
    let shared = tokio::task::spawn_blocking(move || {
        let decoder = Decoder::spawn(source, uri, 48000, 2, 137).unwrap();
        let deadline = Instant::now() + Duration::from_secs(3);
        while !decoder.buffer.done.load(Ordering::Acquire) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(2));
        }
        assert!(decoder.buffer.done.load(Ordering::Acquire));
        assert!(
            !decoder.buffer.failed.load(Ordering::Acquire),
            "{:?}",
            decoder.buffer.error.lock().unwrap()
        );
        let count: usize = std::iter::from_fn(|| decoder.buffer.samples.pop())
            .map(|chunk| chunk.len())
            .sum();
        assert_eq!(count, (500 - 137) * 48 * 2);
        decoder.buffer.clone()
    })
    .await
    .unwrap();
    assert!(shared.done.load(Ordering::Acquire));
    assert!(requests.load(Ordering::Relaxed) > 0);
    let mut hung = resolved(wave(48000, 2, 100));
    hung.decoded_audio = None;
    hung.uri = format!("http://{address}/hang");
    let uri = transport.install_source(hung, Some(ticket), true).unwrap();
    let buffer = buffer::Buffer::new(48000, 2, 0);
    let handle = buffer.cancel.clone();
    let probe = tokio::task::spawn_blocking(move || {
        ffmpeg_audio::HttpAudioSource::new_with_cancel_handle(&uri, handle).is_err()
    });
    let deadline = Instant::now() + Duration::from_secs(2);
    let before = requests.load(Ordering::Relaxed);
    while requests.load(Ordering::Relaxed) == before && Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(2)).await;
    }
    assert!(
        requests.load(Ordering::Relaxed) > before,
        "hung probe did not reach the upstream"
    );
    let start = Instant::now();
    buffer.stop();
    assert!(tokio::time::timeout(Duration::from_millis(500), probe)
        .await
        .unwrap()
        .unwrap());
    assert!(start.elapsed() < Duration::from_millis(500));
    upstream.abort();
}
