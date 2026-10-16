//! Bounded random-access media inspection using Lofty's existing parsers.
use crate::{protocol::Client, AdapterResult, Http};
use lofty::{
    config::{apply_global_options, GlobalOptions, ParseOptions},
    file::{AudioFile, FileType},
    probe::Probe,
    properties::FileProperties,
};
use std::io::{self, Read, Seek, SeekFrom};
use url::Url;

const CHUNK: u64 = 65536;
const MAX_SIZE: u64 = 512 * 1024 * 1024;
pub(crate) fn requested_quality(quality: &str) -> AdapterResult<u32> {
    match quality {
        "standard" | "128k" | "low" => Ok(128),
        "320k" | "exhigh" | "higher" => Ok(320),
        "lossless" | "flac" | "jyeffect" | "sky" | "atmos" => Ok(10000),
        "hires" | "flac24bit" | "master" | "jymaster" => Ok(10001),
        _ => Err("unsupported"),
    }
}

pub(crate) struct Remote<'a, 'b, H> {
    client: &'a mut Client<'b, H>,
    url: Url,
    size: u64,
    position: u64,
    chunks: Vec<(u64, Vec<u8>)>,
    requests: u8,
    bytes: usize,
    error: Option<&'static str>,
    etag: Option<String>,
}
impl<'a, 'b, H: Http> Remote<'a, 'b, H> {
    pub fn new(client: &'a mut Client<'b, H>, url: Url) -> AdapterResult<Self> {
        let mut reader = Self {
            client,
            url,
            size: 0,
            position: 0,
            chunks: vec![],
            requests: 0,
            bytes: 0,
            error: None,
            etag: None,
        };
        reader.fetch(0)?;
        Ok(reader)
    }
    fn fetch(&mut self, start: u64) -> AdapterResult<()> {
        if self.requests >= 8 || self.bytes >= 256 * 1024 {
            return Err("invalidData");
        }
        self.requests += 1;
        let end = if self.size == 0 {
            CHUNK - 1
        } else {
            (start + CHUNK - 1).min(self.size - 1)
        };
        let mut headers = vec![
            ("Range".into(), format!("bytes={start}-{end}")),
            ("Accept-Encoding".into(), "identity".into()),
        ];
        if let Some(etag) = &self.etag {
            headers.push(("If-Range".into(), etag.clone()));
        }
        let response = self.client.get(&mut self.url, &headers)?;
        if response.status != 206 || response.body.len() > CHUNK as usize {
            return Err("invalidData");
        }
        let range = response
            .header("content-range")
            .and_then(|s| s.strip_prefix("bytes "))
            .ok_or("invalidData")?;
        let (bounds, size) = range.split_once('/').ok_or("invalidData")?;
        let (from, to) = bounds.split_once('-').ok_or("invalidData")?;
        let parse = |s: &str| s.parse::<u64>().map_err(|_| "invalidData");
        let (from, to, size) = (parse(from)?, parse(to)?, parse(size)?);
        if !(513..=MAX_SIZE).contains(&size)
            || from != start
            || to != end.min(size - 1)
            || from > to
            || to - from + 1 != response.body.len() as u64
            || (self.size != 0 && size != self.size)
        {
            return Err("invalidData");
        }
        if self.requests == 1 {
            self.etag = response
                .header("etag")
                .filter(|s| !s.starts_with("W/"))
                .map(str::to_owned);
        } else if let Some(etag) = &self.etag {
            if response.header("etag") != Some(etag.as_str()) {
                return Err("invalidData");
            }
        }
        self.bytes += response.body.len();
        if self.bytes > 256 * 1024 {
            return Err("invalidData");
        }
        self.size = size;
        self.chunks.push((start, response.body));
        Ok(())
    }
    fn read_inner(&mut self, buffer: &mut [u8]) -> AdapterResult<usize> {
        if buffer.is_empty() || self.position == self.size {
            return Ok(0);
        }
        if buffer.len() > CHUNK as usize {
            return Err("invalidData");
        }
        if !self.chunks.iter().any(|(offset, bytes)| {
            self.position >= *offset && self.position < offset + bytes.len() as u64
        }) {
            self.fetch(self.position)?;
        }
        let (offset, bytes) = self
            .chunks
            .iter()
            .find(|(offset, bytes)| {
                self.position >= *offset && self.position < offset + bytes.len() as u64
            })
            .ok_or("invalidData")?;
        let offset = (self.position - offset) as usize;
        let count = buffer.len().min(bytes.len() - offset);
        buffer[..count].copy_from_slice(&bytes[offset..offset + count]);
        self.position += count as u64;
        Ok(count)
    }
}
impl<H: Http> Read for Remote<'_, '_, H> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.read_inner(buffer).map_err(|code| {
            self.error = Some(code);
            io::Error::other(code)
        })
    }
}
impl<H: Http> Seek for Remote<'_, '_, H> {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        let next = match position {
            SeekFrom::Start(p) => p as i128,
            SeekFrom::End(p) => self.size as i128 + p as i128,
            SeekFrom::Current(p) => self.position as i128 + p as i128,
        };
        if next < 0 || next > self.size as i128 {
            self.error = Some("invalidData");
            return Err(io::Error::other("invalidData"));
        }
        self.position = next as u64;
        Ok(self.position)
    }
}

fn properties(reader: &mut (impl Read + Seek)) -> AdapterResult<(FileProperties, bool)> {
    let probe = Probe::new(&mut *reader)
        .guess_file_type()
        .map_err(|_| "invalidData")?;
    let kind = probe.file_type().ok_or("invalidData")?;
    reader.rewind().map_err(|_| "invalidData")?;
    let options = ParseOptions::new()
        .read_tags(false)
        .read_cover_art(false)
        .max_junk_bytes(4096);
    match kind {
        FileType::Mpeg => {
            let file =
                lofty::mpeg::MpegFile::read_from(reader, options).map_err(|_| "invalidData")?;
            if !matches!(file.properties().layer(), lofty::mpeg::Layer::Layer3) {
                return Err("unsupported");
            }
            Ok((file.properties().clone().into(), false))
        }
        FileType::Flac => {
            let file =
                lofty::flac::FlacFile::read_from(reader, options).map_err(|_| "invalidData")?;
            Ok((file.properties().clone().into(), true))
        }
        FileType::Mp4 => {
            let file =
                lofty::mp4::Mp4File::read_from(reader, options).map_err(|_| "invalidData")?;
            let lossless = match file.properties().codec() {
                Some(lofty::mp4::Mp4Codec::AAC) => false,
                Some(lofty::mp4::Mp4Codec::ALAC) => true,
                _ => return Err("unsupported"),
            };
            Ok((file.properties().clone().into(), lossless))
        }
        _ => Err("unsupported"),
    }
}
pub(crate) fn inspect<H: Http>(
    client: &mut Client<'_, H>,
    url: Url,
    expected: u64,
    requested: u32,
    downgrade: bool,
) -> AdapterResult<(String, String, Vec<(String, String)>)> {
    apply_global_options(
        GlobalOptions::new()
            .allocation_limit(65536)
            .use_custom_resolvers(false),
    );
    let mut reader = Remote::new(client, url)?;
    let result = properties(&mut reader);
    if let Some(code) = reader.error {
        return Err(code);
    }
    let (props, lossless) = result?;
    let duration = props.duration().as_millis() as u64;
    let bitrate = props
        .audio_bitrate()
        .filter(|b| *b > 0)
        .ok_or("invalidData")?;
    if duration == 0 || expected == 0 || duration.abs_diff(expected) > 2000.max(expected / 100) {
        return Err("permissionDenied");
    }
    let actual = if lossless {
        if props.bit_depth().unwrap_or(0) > 16 || props.sample_rate().unwrap_or(0) > 48000 {
            10001
        } else {
            10000
        }
    } else {
        bitrate
    };
    if !downgrade && actual.saturating_add(if lossless { 0 } else { 12 }) < requested {
        return Err("permissionDenied");
    }
    let quality = match actual {
        10000 => "lossless".into(),
        10001 => "hires".into(),
        n => format!("{n}k"),
    };
    let headers = reader.client.headers(&reader.url);
    Ok((reader.url.to_string(), quality, headers))
}
