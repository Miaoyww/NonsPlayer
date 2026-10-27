use std::io::{Read, Seek, SeekFrom, Write};

use aes::Aes128;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use cipher::block_padding::Pkcs7;
use cipher::{BlockDecryptMut, KeyInit};
use serde::{Deserialize, Serialize};

use crate::error::{Errors, Result};

const HEADER_KEY: [u8; 16] = [
    0x68, 0x7A, 0x48, 0x52, 0x41, 0x6D, 0x73, 0x6F, 0x35, 0x6B, 0x49, 0x6E, 0x62, 0x61, 0x78, 0x57,
];

const INFO_KEY: [u8; 16] = [
    0x23, 0x31, 0x34, 0x6C, 0x6A, 0x6B, 0x5F, 0x21, 0x5C, 0x5D, 0x26, 0x30, 0x55, 0x3C, 0x27, 0x28,
];

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum NcmId {
    String(String),
    Integer(u64),
}

/// The ncm file information.
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RawNcmInfo {
    /// The name of music
    #[serde(rename = "musicName")]
    pub name: String,
    /// The id of music
    #[serde(rename = "musicId")]
    pub id: NcmId,
    /// The album of music, it's an url
    pub album: String,
    /// The artist of music, first item is name, second item is id
    pub artist: Vec<(String, NcmId)>,
    // The bit rate of music
    pub bitrate: NcmId,
    /// The duration of music
    pub duration: NcmId,
    /// The format of music, is maybe 'mp3' or 'flac'
    pub format: String,
    /// The id of MV
    #[serde(rename = "mvId")]
    pub mv_id: Option<NcmId>,
    /// The alias of music
    pub alias: Option<Vec<String>>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct NcmInfo {
    pub name: String,
    /// The id of music
    pub id: u64,
    /// The album of music, it's an url
    pub album: String,
    /// The artist of music, first item is name, second item is id
    pub artist: Vec<(String, u64)>,
    // The bit rate of music
    pub bitrate: u64,
    /// The duration of music
    pub duration: u64,
    /// The format of music, is maybe 'mp3' or 'flac'
    pub format: String,
    /// The id of MV
    pub mv_id: Option<u64>,
    /// The alias of music
    pub alias: Option<Vec<String>>,
}

/// The ncm file dump wrapper.
pub struct Ncmdump<S>
where
    S: Read,
{
    reader: S,
    cursor: u64,
    data_offset: u64,
    info: (u64, u64),
    image: (u64, u64),
    key_box: [u8; 256],
}

impl From<RawNcmInfo> for NcmInfo {
    fn from(raw_info: RawNcmInfo) -> Self {
        Self {
            name: raw_info.name,
            id: raw_info.id.get_id().unwrap_or(0),
            album: raw_info.album,
            artist: raw_info
                .artist
                .into_iter()
                .map(|(name, id)| (name, id.get_id().unwrap_or(0)))
                .collect::<Vec<(String, u64)>>(),
            bitrate: raw_info.bitrate.get_id().unwrap_or(0),
            duration: raw_info.duration.get_id().unwrap_or(0),
            format: raw_info.format,
            mv_id: match raw_info.mv_id {
                Some(id) => match id.get_id() {
                    Ok(inner) => Some(inner),
                    Err(_) => None,
                },
                None => None,
            },
            alias: raw_info.alias,
        }
    }
}

impl NcmId {
    pub fn get_id(self) -> Result<u64> {
        match self {
            NcmId::String(s) => {
                if s.is_empty() {
                    return Err(Errors::InfoDecodeError);
                }
                s.parse().map_err(|_| Errors::InfoDecodeError)
            }
            NcmId::Integer(num) => Ok(num),
        }
    }
}

impl<S> Ncmdump<S>
where
    S: Read,
{
    #[inline]
    fn base(&self) -> u64 {
        self.data_offset
    }

    fn get_key(key: &[u8]) -> Result<Vec<u8>> {
        let key_buffer = key.iter().map(|byte| byte ^ 0x64).collect::<Vec<u8>>();
        let decrypt_buffer = Self::decrypt(&key_buffer, &HEADER_KEY)?;
        if decrypt_buffer.len() <= 17 {
            return Err(Errors::DecryptError);
        }
        Ok(decrypt_buffer[17..].to_vec())
    }

    fn encrypt(&mut self, offset: u64, buffer: &mut [u8]) {
        for (i, byte) in buffer.iter_mut().enumerate() {
            let j = ((offset + i as u64 + 1) & 0xff) as usize;
            let k = (self.key_box[j as usize].wrapping_add(j as u8)) as usize;
            let key_index = self.key_box[k].wrapping_add(self.key_box[j]) as usize;
            *byte ^= self.key_box[key_index]
        }
    }

    fn decrypt(data: &[u8], key: &[u8; 16]) -> Result<Vec<u8>> {
        let result = Aes128::new(key.into())
            .decrypt_padded_vec_mut::<Pkcs7>(data)
            .map_err(|_| Errors::DecryptError)?;
        Ok(result)
    }

    fn build_key_box(key: &[u8]) -> [u8; 256] {
        let mut j = 0;
        let mut key_box = [0u8; 256];
        key_box
            .iter_mut()
            .enumerate()
            .for_each(|(i, k)| *k = i as u8);

        let key_stream = key.iter().cycle();
        for (i, &k) in (0..256).zip(key_stream) {
            j = key_box[i].wrapping_add(j).wrapping_add(k);
            key_box.swap(i, j as usize);
        }
        key_box
    }

    /// Check the file format by header.
    fn check_format(buffer: &[u8]) -> bool {
        buffer.starts_with(b"CTENFDAM")
    }
}

impl<S> Ncmdump<S>
where
    S: Read + Seek,
{
    /// Create a Ncmdump from a seekable reader.
    /// Usually, the reader is a `File` or `Cursor`.
    ///
    /// # Example
    ///
    /// From a file.
    ///
    /// ```rust
    /// # use std::fs::File;
    /// #
    /// # use ncmdump::Ncmdump;
    /// #
    /// let file = File::open("res/test.ncm").expect("Can't open file");
    /// let _ = Ncmdump::from_reader(file).unwrap();
    /// ```
    /// Or from a Cursor.
    /// ```rust
    /// # use std::fs::File;
    /// # use std::io::{Cursor, Read};
    /// #
    /// # use ncmdump::Ncmdump;
    /// #
    /// # let mut file = File::open("res/test.ncm").expect("Can't open file.");
    /// # let mut data = Vec::new();
    /// # file.read_to_end(&mut data).expect("Can't read file");
    /// let cursor = Cursor::new(data);
    /// let _ = Ncmdump::from_reader(cursor).unwrap();
    /// ```
    pub fn from_reader(mut reader: S) -> Result<Self> {
        // check format
        let mut format = [0; 10];
        reader
            .read_exact(&mut format)
            .map_err(|_| Errors::InvalidFileType)?;
        if !Self::check_format(&format) {
            return Err(Errors::InvalidFileType);
        }

        let mut key_length = [0; 4];
        reader
            .read_exact(&mut key_length)
            .map_err(|_| Errors::InvalidKeyLength)?;
        let key_length = u32::from_le_bytes(key_length) as usize;
        if key_length == 0 || key_length > 4096 {
            return Err(Errors::InvalidKeyLength);
        }
        let mut key = vec![0u8; key_length];
        reader
            .read_exact(&mut key)
            .map_err(|_| Errors::InvalidKeyLength)?;
        let key = Self::get_key(&key)?;
        let key_box = Self::build_key_box(&key);

        // reader.seek(SeekFrom::Current(key_length as i64))?;
        let mut info_length = [0; 4];
        reader
            .read_exact(&mut info_length)
            .map_err(|_| Errors::InvalidInfoLength)?;
        let info_start = reader.stream_position()?;
        let info_length = u32::from_le_bytes(info_length) as u64;

        if info_length > 2 * 1024 * 1024 {
            return Err(Errors::InvalidInfoLength);
        }
        reader.seek(SeekFrom::Current(info_length as i64))?;
        reader.seek(SeekFrom::Current(5))?;
        let mut cover_frame_len = [0; 4];
        reader.read_exact(&mut cover_frame_len)?;
        let cover_frame_len = u32::from_le_bytes(cover_frame_len) as u64;

        let mut image_length = [0; 4];
        reader
            .read_exact(&mut image_length)
            .map_err(|_| Errors::InvalidImageLength)?;
        let image_start = reader.stream_position()?;
        let image_length = u32::from_le_bytes(image_length) as u64;

        if image_length > cover_frame_len {
            return Err(Errors::InvalidImageLength);
        }
        let data_offset = image_start + cover_frame_len;
        if data_offset >= reader.seek(SeekFrom::End(0))? {
            return Err(Errors::InvalidFileType);
        }
        reader.seek(SeekFrom::Start(data_offset))?;
        Ok(Self {
            reader,
            key_box,
            cursor: 0,
            data_offset,
            info: (info_start, info_length),
            image: (image_start, image_length),
        })
    }

    /// Utils for get bytes.
    fn get_bytes(&mut self, start: u64, length: u64) -> Result<Vec<u8>> {
        let reader = self.reader.by_ref();
        let mut buf = Vec::new();
        reader.seek(SeekFrom::Start(start))?;
        reader.take(length).read_to_end(&mut buf)?;
        Ok(buf)
    }

    /// Decode the information buffer and just return the information.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::fs::File;
    /// use std::path::Path;
    ///
    /// use anyhow::Result;
    /// use ncmdump::Ncmdump;
    ///
    /// fn main() -> Result<()> {
    ///     let file = File::open("res/test.ncm")?;
    ///     let mut ncm = Ncmdump::from_reader(file)?;
    ///     let info = ncm.get_info();
    ///     println!("{:?}", info);
    ///     Ok(())
    /// }
    /// ```
    pub fn get_info(&mut self) -> Result<NcmInfo> {
        let (start, length) = self.info;
        let info_bytes = self.get_bytes(start, length)?;
        let info_tmp = info_bytes
            .iter()
            .map(|item| item ^ 0x63)
            .collect::<Vec<u8>>();
        if info_tmp.len() < 22 {
            return Err(Errors::InfoDecodeError);
        }
        let info_key = STANDARD
            .decode(&info_tmp[22..])
            .map_err(|_| Errors::InfoDecodeError)?;
        let info_data = Self::decrypt(&info_key, &INFO_KEY)?;
        if info_data.len() < 6 {
            return Err(Errors::InfoDecodeError);
        }
        let info_str =
            String::from_utf8(info_data[6..].to_vec()).map_err(|_| Errors::InfoDecodeError)?;
        let info =
            serde_json::from_str::<RawNcmInfo>(&info_str).map_err(|_| Errors::InfoDecodeError)?;
        Ok(NcmInfo::from(info))
    }

    /// Get the image bytes from ncmdump, if it's exists.
    ///
    /// # Example:
    ///
    /// ```rust
    /// use std::fs::File;
    /// use std::path::Path;
    ///
    /// use anyhow::Result;
    /// use ncmdump::Ncmdump;
    ///
    /// fn main() -> Result<()> {
    ///     use std::io::Write;
    /// let file = File::open("res/test.ncm")?;
    ///     let mut ncm = Ncmdump::from_reader(file)?;
    ///     let image = ncm.get_image()?;
    ///
    ///     let mut target = File::options()
    ///         .create(true)
    ///         .write(true)
    ///         .open("res/test.jpeg")?;
    ///     target.write_all(&image)?;
    ///     Ok(())
    /// }
    /// ```
    pub fn get_image(&mut self) -> Result<Vec<u8>> {
        let (start, end) = self.image;
        let image = self.get_bytes(start, end)?;
        Ok(image)
    }

    /// Get the music data from ncmdump.
    ///
    /// # Example:
    ///
    /// ```rust
    /// use std::fs::File;
    /// use std::io::Write;
    /// use std::path::Path;
    ///
    /// use anyhow::Result;
    /// use ncmdump::Ncmdump;
    ///
    /// fn main() -> Result<()> {
    ///     let file = File::open("res/test.ncm")?;
    ///     let mut ncm = Ncmdump::from_reader(file)?;
    ///     let music = ncm.get_data()?;
    ///
    ///     let mut target = File::options()
    ///         .create(true)
    ///         .write(true)
    ///         .open("res/test.flac")?;
    ///     target.write_all(&music)?;
    ///     Ok(())
    /// }
    /// ```
    pub fn get_data(&mut self) -> Result<Vec<u8>> {
        let mut data = Vec::new();
        let mut buffer = [0; 0x8000];
        while let Ok(size) = self.read(&mut buffer) {
            if size == 0 {
                break;
            }
            data.write_all(&buffer[..size])?;
        }
        Ok(data)
    }
}

impl<R> Read for Ncmdump<R>
where
    R: Read + Seek,
{
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let size = self.reader.read(buf)?;
        self.encrypt(self.cursor, &mut buf[..size]);
        self.cursor += size as u64;
        Ok(size)
    }
}

impl<R> Seek for Ncmdump<R>
where
    R: Read + Seek,
{
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let base = self.base();
        let pos = match pos {
            SeekFrom::Start(p) => SeekFrom::Start(p + base),
            _ => pos,
        };
        self.cursor = self.reader.seek(pos)?.checked_sub(base).ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "seek before audio")
        })?;
        Ok(self.cursor)
    }
}
