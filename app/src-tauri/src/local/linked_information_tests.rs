use super::*;
use crate::{local::tag_editor::write_download_tags, test_support::TestDir};
use std::io::Write;

fn wav(path: &std::path::Path) {
    let data_bytes = 16_000u32;
    let mut data = Vec::with_capacity((data_bytes + 44) as usize);
    data.extend_from_slice(b"RIFF");
    data.extend_from_slice(&(data_bytes + 36).to_le_bytes());
    data.extend_from_slice(b"WAVEfmt ");
    data.extend_from_slice(&16u32.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&8_000u32.to_le_bytes());
    data.extend_from_slice(&16_000u32.to_le_bytes());
    data.extend_from_slice(&2u16.to_le_bytes());
    data.extend_from_slice(&16u16.to_le_bytes());
    data.extend_from_slice(b"data");
    data.extend_from_slice(&data_bytes.to_le_bytes());
    data.resize((data_bytes + 44) as usize, 0);
    std::fs::write(path, data).unwrap();
}

#[test]
fn linked_download_returns_local_tags_and_audio_properties_without_storing_the_path_in_track() {
    let directory = TestDir::new();
    let store = Store::open(&directory.0.join("library.sqlite")).unwrap();
    let path = directory.0.join("download.wav");
    wav(&path);
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .unwrap();
    write_download_tags(&mut file, "本地标题", "本地艺术家", "本地专辑", None).unwrap();
    file.flush().unwrap();
    store
        .link_download("netease:42", path.to_str().unwrap(), Some("lossless"))
        .unwrap();

    let linked = linked_local_information(&store, "netease:42")
        .unwrap()
        .unwrap();
    assert_eq!(linked.title.as_deref(), Some("本地标题"));
    assert_eq!(linked.artist.as_deref(), Some("本地艺术家"));
    assert_eq!(linked.album.as_deref(), Some("本地专辑"));
    assert_eq!(linked.path, path.to_str().unwrap());
    assert_eq!(linked.information.sample_rate, Some(8_000));
    assert_eq!(linked.information.format, "WAV");
}

#[test]
fn missing_linked_download_is_not_reported_as_a_local_file() {
    let directory = TestDir::new();
    let store = Store::open(&directory.0.join("library.sqlite")).unwrap();
    store
        .link_download("netease:42", "missing.wav", Some("lossless"))
        .unwrap();
    assert!(linked_local_information(&store, "netease:42")
        .unwrap()
        .is_none());
}

#[test]
fn unlinked_streaming_track_has_no_local_information() {
    let directory = TestDir::new();
    let store = Store::open(&directory.0.join("library.sqlite")).unwrap();
    assert!(linked_local_information(&store, "netease:42")
        .unwrap()
        .is_none());
}
