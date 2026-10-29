//! Bounded, seekable streaming for document-provider file descriptors.
use axum::{
    body::{Body, Bytes},
    http::{header, HeaderMap, Method, StatusCode},
    response::Response,
};
use std::{fs::File, io, io::SeekFrom};
use tokio::io::{AsyncReadExt, AsyncSeekExt};

pub(super) async fn response(
    file: File,
    headers: &HeaderMap,
    method: &Method,
    path: &std::path::Path,
) -> Result<Response, StatusCode> {
    let mut file = tokio::fs::File::from_std(file);
    let size = file
        .seek(SeekFrom::End(0))
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
    // Without a matching validator, If-Range requests receive the full resource.
    let range = if method == Method::GET && !headers.contains_key(header::IF_RANGE) {
        headers
            .get(header::RANGE)
            .map(|value| http_range::HttpRange::parse_bytes(value.as_bytes(), size))
    } else {
        None
    };
    let range = match range {
        Some(Err(_)) => {
            return Response::builder()
                .status(StatusCode::RANGE_NOT_SATISFIABLE)
                .header(header::CONTENT_RANGE, format!("bytes */{size}"))
                .header(header::ACCEPT_RANGES, "bytes")
                .header(header::CONTENT_LENGTH, 0)
                .body(Body::empty())
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);
        }
        // A server may ignore multiple ranges and return the full representation.
        Some(Ok(ranges)) if ranges.len() == 1 && ranges[0].length > 0 => Some(ranges[0]),
        _ => None,
    };
    let (offset, length) = range.map_or((0, size), |range| (range.start, range.length));
    let mut response = Response::builder()
        .status(if range.is_some() {
            StatusCode::PARTIAL_CONTENT
        } else {
            StatusCode::OK
        })
        .header(header::ACCEPT_RANGES, "bytes")
        .header(
            header::CONTENT_TYPE,
            mime_guess::from_path(path).first_or_octet_stream().as_ref(),
        )
        .header(header::CONTENT_LENGTH, length);
    if range.is_some() {
        response = response.header(
            header::CONTENT_RANGE,
            format!("bytes {offset}-{}/{size}", offset + length - 1),
        );
    }
    let body = if method == Method::HEAD || length == 0 {
        Body::empty()
    } else {
        file.seek(SeekFrom::Start(offset))
            .await
            .map_err(|_| StatusCode::BAD_GATEWAY)?;
        let stream =
            futures_util::stream::try_unfold((file, length), |(mut file, remaining)| async move {
                if remaining == 0 {
                    return Ok::<_, io::Error>(None);
                }
                let mut bytes = vec![0; remaining.min(64 * 1024) as usize];
                let read = file.read(&mut bytes).await?;
                if read == 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "音乐文件读取提前结束",
                    ));
                }
                bytes.truncate(read);
                Ok(Some((Bytes::from(bytes), (file, remaining - read as u64))))
            });
        Body::from_stream(stream)
    };
    response
        .body(body)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn seekable_descriptor_serves_ranges_head_and_independent_offsets() {
        let file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(file.path(), b"0123456789").unwrap();
        for (method, range, if_range, status, expected, content_range) in [
            (Method::GET, None, false, 200, &b"0123456789"[..], None),
            (
                Method::GET,
                Some("bytes=2-5"),
                false,
                206,
                &b"2345"[..],
                Some("bytes 2-5/10"),
            ),
            (
                Method::GET,
                Some("bytes=-3"),
                false,
                206,
                &b"789"[..],
                Some("bytes 7-9/10"),
            ),
            (
                Method::GET,
                Some("bytes=8-"),
                false,
                206,
                &b"89"[..],
                Some("bytes 8-9/10"),
            ),
            (
                Method::GET,
                Some("bytes=12-"),
                false,
                416,
                &b""[..],
                Some("bytes */10"),
            ),
            (
                Method::GET,
                Some("bytes=oops"),
                false,
                416,
                &b""[..],
                Some("bytes */10"),
            ),
            (
                Method::GET,
                Some("bytes=0-1,4-5"),
                false,
                200,
                &b"0123456789"[..],
                None,
            ),
            (
                Method::GET,
                Some("bytes=2-5"),
                true,
                200,
                &b"0123456789"[..],
                None,
            ),
            (Method::HEAD, Some("bytes=2-5"), false, 200, &b""[..], None),
        ] {
            let mut headers = HeaderMap::new();
            if let Some(range) = range {
                headers.insert(header::RANGE, range.parse().unwrap());
            }
            if if_range {
                headers.insert(header::IF_RANGE, "\"old\"".parse().unwrap());
            }
            let result = response(
                File::open(file.path()).unwrap(),
                &headers,
                &method,
                std::path::Path::new("song.mp3"),
            )
            .await
            .unwrap();
            assert_eq!(result.status().as_u16(), status);
            assert_eq!(
                result
                    .headers()
                    .get(header::CONTENT_RANGE)
                    .map(|v| v.to_str().unwrap()),
                content_range
            );
            let length = result.headers()[header::CONTENT_LENGTH]
                .to_str()
                .unwrap()
                .parse::<usize>()
                .unwrap();
            assert_eq!(
                length,
                if method == Method::HEAD {
                    10
                } else {
                    expected.len()
                }
            );
            assert_eq!(
                axum::body::to_bytes(result.into_body(), 100)
                    .await
                    .unwrap()
                    .as_ref(),
                expected
            );
        }
        let read = |range: &'static str| {
            let file = File::open(file.path()).unwrap();
            async move {
                let mut headers = HeaderMap::new();
                headers.insert(header::RANGE, range.parse().unwrap());
                let response = response(
                    file,
                    &headers,
                    &Method::GET,
                    std::path::Path::new("song.flac"),
                )
                .await
                .unwrap();
                axum::body::to_bytes(response.into_body(), 100)
                    .await
                    .unwrap()
            }
        };
        let (first, second) = tokio::join!(read("bytes=1-3"), read("bytes=7-9"));
        assert_eq!(first.as_ref(), b"123");
        assert_eq!(second.as_ref(), b"789");
    }
}
