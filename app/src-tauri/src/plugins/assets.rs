use crate::model::AppResult;
use http_range::HttpRange;
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
};
use tauri::http::{header, HeaderMap, Method, StatusCode};

const MAX_RESOURCE_BYTES: u64 = 8 * 1024 * 1024;

pub(super) struct ResourceResponse {
    pub status: StatusCode,
    pub body: Vec<u8>,
    pub content_type: String,
    pub content_length: u64,
    pub content_range: Option<String>,
}

impl ResourceResponse {
    pub(super) fn from_bytes(bytes: Vec<u8>, content_type: &'static str, method: &Method) -> Self {
        let content_length = bytes.len() as u64;
        Self {
            status: StatusCode::OK,
            body: if method == Method::HEAD {
                Vec::new()
            } else {
                bytes
            },
            content_type: content_type.into(),
            content_length,
            content_range: None,
        }
    }

    pub(super) fn method_not_allowed() -> Self {
        Self {
            status: StatusCode::METHOD_NOT_ALLOWED,
            body: Vec::new(),
            content_type: "application/octet-stream".into(),
            content_length: 0,
            content_range: None,
        }
    }
}

pub(super) fn read(
    path: &Path,
    method: &Method,
    headers: &HeaderMap,
) -> AppResult<ResourceResponse> {
    if method != Method::GET && method != Method::HEAD {
        return Ok(ResourceResponse::method_not_allowed());
    }

    let file = File::open(path).map_err(|error| error.to_string())?;
    let size = file.metadata().map_err(|error| error.to_string())?.len();
    if size > MAX_RESOURCE_BYTES {
        return Err("资源过大".into());
    }

    let ranges = if method == Method::GET && !headers.contains_key(header::IF_RANGE) {
        headers
            .get(header::RANGE)
            .map(|value| HttpRange::parse_bytes(value.as_bytes(), size))
    } else {
        None
    };
    let range = match ranges {
        Some(Err(_)) => {
            return Ok(ResourceResponse {
                status: StatusCode::RANGE_NOT_SATISFIABLE,
                body: Vec::new(),
                content_type: mime_guess::from_path(path)
                    .first_or_octet_stream()
                    .to_string(),
                content_length: 0,
                content_range: Some(format!("bytes */{size}")),
            });
        }
        // Multiple ranges are served as the complete representation, matching local streams.
        Some(Ok(ranges)) if ranges.len() == 1 && ranges[0].length > 0 => Some(ranges[0]),
        _ => None,
    };

    let (offset, length) = range.map_or((0, size), |range| (range.start, range.length));
    let content_range = range.map(|_| format!("bytes {offset}-{}/{size}", offset + length - 1));
    let body = if method == Method::HEAD || length == 0 {
        Vec::new()
    } else {
        let mut file = file;
        file.seek(SeekFrom::Start(offset))
            .map_err(|error| error.to_string())?;
        let mut body = vec![0; length as usize];
        file.read_exact(&mut body)
            .map_err(|error| error.to_string())?;
        body
    };

    Ok(ResourceResponse {
        status: if range.is_some() {
            StatusCode::PARTIAL_CONTENT
        } else {
            StatusCode::OK
        },
        body,
        content_type: mime_guess::from_path(path)
            .first_or_octet_stream()
            .to_string(),
        content_length: if method == Method::HEAD { size } else { length },
        content_range,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infers_common_web_asset_types_and_falls_back_for_unknown_extensions() {
        for (name, expected) in [
            ("clip.mp4", "video/mp4"),
            ("clip.webm", "video/webm"),
            ("sound.mp3", "audio/mpeg"),
            ("captions.vtt", "text/vtt"),
            ("theme.css", "text/css"),
            ("data.json", "application/json"),
            ("font.woff2", "font/woff2"),
            ("module.wasm", "application/wasm"),
            ("data.unknown", "application/octet-stream"),
        ] {
            assert_eq!(
                mime_guess::from_path(name).first_or_octet_stream().as_ref(),
                expected,
                "{name}"
            );
        }
    }

    #[test]
    fn serves_full_and_ranged_media_with_expected_headers() {
        let file = tempfile::Builder::new().suffix(".mp4").tempfile().unwrap();
        std::fs::write(file.path(), b"0123456789").unwrap();

        let full = read(file.path(), &Method::GET, &HeaderMap::new()).unwrap();
        assert_eq!(full.status, StatusCode::OK);
        assert_eq!(full.content_type, "video/mp4");
        assert_eq!(full.content_length, 10);
        assert_eq!(full.body, b"0123456789");

        let mut headers = HeaderMap::new();
        headers.insert(header::RANGE, "bytes=2-5".parse().unwrap());
        let partial = read(file.path(), &Method::GET, &headers).unwrap();
        assert_eq!(partial.status, StatusCode::PARTIAL_CONTENT);
        assert_eq!(partial.content_range.as_deref(), Some("bytes 2-5/10"));
        assert_eq!(partial.content_length, 4);
        assert_eq!(partial.body, b"2345");
    }

    #[test]
    fn handles_unsatisfiable_multiple_head_and_if_range_requests() {
        let file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(file.path(), b"0123456789").unwrap();

        let mut headers = HeaderMap::new();
        headers.insert(header::RANGE, "bytes=12-".parse().unwrap());
        let invalid = read(file.path(), &Method::GET, &headers).unwrap();
        assert_eq!(invalid.status, StatusCode::RANGE_NOT_SATISFIABLE);
        assert_eq!(invalid.content_range.as_deref(), Some("bytes */10"));
        assert!(invalid.body.is_empty());

        headers.insert(header::RANGE, "bytes=0-1,4-5".parse().unwrap());
        let multiple = read(file.path(), &Method::GET, &headers).unwrap();
        assert_eq!(multiple.status, StatusCode::OK);
        assert_eq!(multiple.body, b"0123456789");

        let head = read(file.path(), &Method::HEAD, &headers).unwrap();
        assert_eq!(head.status, StatusCode::OK);
        assert_eq!(head.content_length, 10);
        assert!(head.body.is_empty());

        headers.insert(header::RANGE, "bytes=2-5".parse().unwrap());
        headers.insert(header::IF_RANGE, "\"stale\"".parse().unwrap());
        let stale = read(file.path(), &Method::GET, &headers).unwrap();
        assert_eq!(stale.status, StatusCode::OK);
        assert_eq!(stale.body, b"0123456789");
    }

    #[test]
    fn enforces_method_and_resource_size_limits() {
        let file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(file.path(), b"asset").unwrap();
        let method = read(file.path(), &Method::POST, &HeaderMap::new()).unwrap();
        assert_eq!(method.status, StatusCode::METHOD_NOT_ALLOWED);
        assert!(method.body.is_empty());

        assert_eq!(
            ResourceResponse::method_not_allowed().status,
            StatusCode::METHOD_NOT_ALLOWED
        );

        let file = tempfile::NamedTempFile::new().unwrap();
        file.as_file().set_len(MAX_RESOURCE_BYTES + 1).unwrap();
        assert!(read(file.path(), &Method::GET, &HeaderMap::new()).is_err());
    }
}
