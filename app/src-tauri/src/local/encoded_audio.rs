//! Container decoding shared by local indexing, playback and file tools.
use crate::model::AppResult;
use lofty::{file::TaggedFile, probe::Probe};
use ncmdump::{NcmInfo, Ncmdump};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
    sync::{Arc, Mutex, OnceLock, Weak},
};

pub const MAX_AUDIO_BYTES: u64 = 256 * 1024 * 1024;
const MAX_LEASES: usize = 4;

pub fn is_encoded(path: &Path) -> bool {
    path.extension()
        .is_some_and(|s| s.eq_ignore_ascii_case("ncm"))
}

pub struct Decoder<R: Read + Seek> {
    pub reader: Ncmdump<R>,
    pub info: Option<NcmInfo>,
    pub image: Vec<u8>,
    pub format: &'static str,
}
impl<R: Read + Seek> Decoder<R> {
    pub fn new(reader: R) -> AppResult<Self> {
        let mut reader =
            Ncmdump::from_reader(reader).map_err(|e| format!("NCM 文件无法解析：{e}"))?;
        // Missing metadata is permitted; audio magic is authoritative.
        let info = reader
            .get_info()
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn("local.ncm.read_metadata", error);
            })
            .ok();
        let image = reader.get_image().map_err(|e| e.to_string())?;
        reader.rewind().map_err(|e| e.to_string())?;
        let mut head = [0; 4];
        reader
            .read_exact(&mut head)
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn_error(
                    "local.ncm.read_audio_header",
                    error,
                );
            })
            .map_err(|_| "NCM 音频正文不完整")?;
        let format = if &head == b"fLaC" {
            "flac"
        } else if &head[..3] == b"ID3" || (head[0] == 0xff && head[1] & 0xe0 == 0xe0) {
            "mp3"
        } else {
            return Err("NCM 正文不是支持的 MP3 或 FLAC 音频".into());
        };
        reader.rewind().map_err(|e| e.to_string())?;
        Ok(Self {
            reader,
            info,
            image,
            format,
        })
    }
    pub fn copy(
        &mut self,
        target: &mut impl Write,
        mut check: impl FnMut() -> AppResult<()>,
    ) -> AppResult<u64> {
        let mut bytes = 0;
        let mut block = [0; 32 * 1024];
        loop {
            check()?;
            let count = self.reader.read(&mut block).map_err(|e| e.to_string())?;
            if count == 0 {
                return Ok(bytes);
            }
            bytes += count as u64;
            if bytes > MAX_AUDIO_BYTES {
                return Err("单个音频超过 256MiB 上限".into());
            }
            target
                .write_all(&block[..count])
                .map_err(|e| e.to_string())?;
        }
    }
}

pub fn read_tags(path: &str) -> AppResult<(TaggedFile, Option<NcmInfo>, Vec<u8>)> {
    if !is_encoded(Path::new(path)) {
        let tagged = Probe::open(path)
            .map_err(|e| e.to_string())?
            .read()
            .map_err(|e| e.to_string())?;
        return Ok((tagged, None, Vec::new()));
    }
    let mut decoder = Decoder::new(File::open(path).map_err(|e| e.to_string())?)?;
    let tagged = Probe::new(&mut decoder.reader)
        .guess_file_type()
        .map_err(|e| e.to_string())?
        .read()
        .map_err(|e| format!("NCM 音频元数据无法读取：{e}"))?;
    Ok((tagged, decoder.info, decoder.image))
}

pub fn write_tags<R: Read + Seek>(file: &mut File, decoder: &Decoder<R>) -> AppResult<()> {
    use lofty::{
        config::WriteOptions,
        picture::{Picture, PictureType},
        prelude::*,
        tag::{Tag, TagType},
    };
    file.rewind().map_err(|e| e.to_string())?;
    let tagged = Probe::new(&mut *file)
        .guess_file_type()
        .map_err(|e| e.to_string())?
        .read()
        .map_err(|e| e.to_string())?;
    let kind = if decoder.format == "flac" {
        TagType::VorbisComments
    } else {
        TagType::Id3v2
    };
    let mut tag = tagged.tag(kind).cloned().unwrap_or_else(|| Tag::new(kind));
    if let Some(info) = &decoder.info {
        if !info.name.is_empty() {
            tag.set_title(info.name.clone());
        }
        if !info.album.is_empty() {
            tag.set_album(info.album.clone());
        }
        if !info.artist.is_empty() {
            tag.set_artist(
                info.artist
                    .iter()
                    .map(|(name, _)| name.as_str())
                    .collect::<Vec<_>>()
                    .join("/"),
            );
        }
    }
    if !decoder.image.is_empty() && tag.pictures().is_empty() {
        if let Ok(mut picture) = (Picture::from_reader(&mut std::io::Cursor::new(&decoder.image)))
            .inspect_err(|error| {
                crate::infrastructure::diagnostics::warn_error("local.ncm.decode_cover", error);
            })
        {
            picture.set_pic_type(PictureType::CoverFront);
            tag.push_picture(picture);
        }
    }
    file.rewind().map_err(|e| e.to_string())?;
    tag.save_to(file, WriteOptions::default())
        .map_err(|e| e.to_string())
}

// Weak entries reuse a still-playing file, never retain decoded audio after playback.
// Four leases bound in-flight blocking jobs + current/next playback to 1GiB on disk.
type Leases = Vec<(String, Weak<tempfile::NamedTempFile>)>;
static LEASES: OnceLock<Mutex<Leases>> = OnceLock::new();
pub fn prepare(path: &str) -> AppResult<Arc<tempfile::NamedTempFile>> {
    let mut entries = LEASES
        .get_or_init(Default::default)
        .lock()
        .inspect_err(|error| {
            crate::infrastructure::diagnostics::warn("local.ncm.prepare.leases_lock", error);
        })
        .map_err(|_| "音频解析锁不可用")?;
    entries.retain(|(_, file)| file.strong_count() > 0);
    let metadata = std::fs::metadata(path).map_err(|e| e.to_string())?;
    if metadata.len() > MAX_AUDIO_BYTES + 20 * 1024 * 1024 {
        return Err("NCM 文件过大".into());
    }
    let key = format!(
        "{path}:{}:{:?}",
        metadata.len(),
        metadata.modified().map_err(|e| e.to_string())?
    );
    if let Some(file) = entries
        .iter()
        .find(|(k, _)| k == &key)
        .and_then(|(_, f)| f.upgrade())
    {
        return Ok(file);
    }
    if entries.len() >= MAX_LEASES {
        return Err("音频解析繁忙，请稍后重试".into());
    }
    let mut decoder = Decoder::new(File::open(path).map_err(|e| e.to_string())?)?;
    let mut file = tempfile::Builder::new()
        .prefix("nons-audio-")
        .suffix(&format!(".{}", decoder.format))
        .tempfile_in(crate::infrastructure::directories::temporary())
        .map_err(|e| e.to_string())?;
    decoder.copy(&mut file, || Ok(()))?;
    file.flush().map_err(|e| e.to_string())?;
    file.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
    let file = Arc::new(file);
    entries.push((key, Arc::downgrade(&file)));
    Ok(file)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use aes::Aes128;
    use base64::{engine::general_purpose::STANDARD, Engine};
    use cipher::{block_padding::Pkcs7, BlockEncryptMut, KeyInit};
    use std::io::Cursor;

    pub fn fixture(audio: &[u8], padding: usize) -> Vec<u8> {
        let key = b"test-stream-key";
        let mut key_plain = b"neteasecloudmusic".to_vec();
        key_plain.extend_from_slice(key);
        let key_cipher =
            Aes128::new(b"hzHRAmso5kInbaxW".into()).encrypt_padded_vec_mut::<Pkcs7>(&key_plain);
        let metadata = br#"music:{"musicName":"Fixture","musicId":123,"album":"Synthetic","artist":[["Test",1]],"bitrate":128000,"duration":1000,"format":"flac"}"#;
        let info_cipher =
            Aes128::new(b"#14ljk_!\\]&0U<'(".into()).encrypt_padded_vec_mut::<Pkcs7>(metadata);
        let info = format!("163 key(Don't modify):{}", STANDARD.encode(info_cipher));
        let mut data = b"CTENFDAM\0\0".to_vec();
        data.extend_from_slice(&(key_cipher.len() as u32).to_le_bytes());
        data.extend(key_cipher.into_iter().map(|b| b ^ 0x64));
        data.extend_from_slice(&(info.len() as u32).to_le_bytes());
        data.extend(info.as_bytes().iter().map(|b| b ^ 0x63));
        data.extend_from_slice(&[0; 5]);
        data.extend_from_slice(&(padding as u32).to_le_bytes());
        data.extend_from_slice(&0u32.to_le_bytes());
        data.resize(data.len() + padding, 0);
        let mut table = [0u8; 256];
        for (i, value) in table.iter_mut().enumerate() {
            *value = i as u8;
        }
        let mut j = 0u8;
        for i in 0..256 {
            j = table[i].wrapping_add(j).wrapping_add(key[i % key.len()]);
            table.swap(i, j as usize);
        }
        for (offset, byte) in audio.iter().enumerate() {
            let j = (offset + 1) & 255;
            let k = table[j].wrapping_add(j as u8) as usize;
            data.push(byte ^ table[table[k].wrapping_add(table[j]) as usize]);
        }
        data
    }
    pub fn flac() -> Vec<u8> {
        // Synthetic STREAMINFO, no third-party recording included.
        let mut data = b"fLaC\x80\0\0\x22".to_vec();
        let mut stream = [0u8; 34];
        stream[..4].copy_from_slice(&[0x10, 0, 0x10, 0]);
        let packed = (44100u64 << 44) | (1u64 << 41) | (15u64 << 36) | 44100;
        stream[10..18].copy_from_slice(&packed.to_be_bytes());
        data.extend_from_slice(&stream);
        data
    }
    #[test]
    fn padded_cover_roundtrip_seek_and_metadata() {
        let audio = flac();
        let mut decoder = Decoder::new(Cursor::new(fixture(&audio, 128))).unwrap();
        assert_eq!(decoder.format, "flac");
        assert_eq!(decoder.info.as_ref().unwrap().name, "Fixture");
        let mut out = Vec::new();
        decoder.copy(&mut out, || Ok(())).unwrap();
        assert_eq!(out, audio);
        decoder.reader.seek(SeekFrom::Start(4)).unwrap();
        let mut suffix = Vec::new();
        decoder.reader.read_to_end(&mut suffix).unwrap();
        assert_eq!(suffix, audio[4..]);
    }
    #[test]
    fn damaged_containers_and_cancelled_copy_fail() {
        let mut oversized = b"CTENFDAM\0\0".to_vec();
        oversized.extend_from_slice(&u32::MAX.to_le_bytes());
        assert!(Decoder::new(Cursor::new(oversized)).is_err());
        let good = fixture(&flac(), 40);
        for end in [0, 10, 20, good.len() - 45] {
            assert!(Decoder::new(Cursor::new(&good[..end])).is_err());
        }
        let mut decoder = Decoder::new(Cursor::new(good)).unwrap();
        assert!(decoder
            .copy(&mut Vec::new(), || Err("cancelled".into()))
            .is_err());
        assert!(Decoder::new(Cursor::new(fixture(b"unknown audio", 0))).is_err());
        assert_eq!(
            Decoder::new(Cursor::new(fixture(b"ID3\0mp3 body", 0)))
                .unwrap()
                .format,
            "mp3"
        );
    }
    #[test]
    fn playback_lease_reuses_then_invalidates_and_cleans_up() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("音楽.ncm");
        std::fs::write(&source, fixture(&flac(), 64)).unwrap();
        let first = prepare(source.to_str().unwrap()).unwrap();
        let same = prepare(source.to_str().unwrap()).unwrap();
        assert!(Arc::ptr_eq(&first, &same));
        let old = first.path().to_owned();
        let mut updated = flac();
        updated.extend_from_slice(b"changed");
        std::fs::write(&source, fixture(&updated, 64)).unwrap();
        let fresh = prepare(source.to_str().unwrap()).unwrap();
        assert!(!Arc::ptr_eq(&first, &fresh));
        assert_eq!(std::fs::read(fresh.path()).unwrap(), updated);
        drop(first);
        drop(same);
        assert!(!old.exists());
        let path = fresh.path().to_owned();
        drop(fresh);
        assert!(!path.exists());
    }
}
