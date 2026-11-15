use super::{DownloadedFile, DownloadedFilePath, FetchProgressFn};
use crate::util::content_hash::{ContentHasher, temporary_file};
use futures::StreamExt;
use reqwest::header::{ACCEPT_ENCODING, CONTENT_RANGE, ETAG, IF_RANGE, RANGE};
use reqwest::{RequestBuilder, Response, StatusCode};
use std::path::Path;
use std::sync::Arc;
use tokio::io::{AsyncSeekExt, AsyncWriteExt};

// shortcut: partials survive automatic retries; cross-restart resume needs persistent storage and cleanup.
pub(super) struct PartialDownload {
    file: tokio::fs::File,
    path: tempfile::TempPath,
    hasher: ContentHasher,
    size: u64,
    total: Option<u64>,
    etag: Option<reqwest::header::HeaderValue>,
}

impl PartialDownload {
    pub(super) async fn new(staging: Option<&Path>) -> crate::Result<Self> {
        let (file, path) = temporary_file(staging).await?;
        Ok(Self {
            file,
            path,
            hasher: ContentHasher::default(),
            size: 0,
            total: None,
            etag: None,
        })
    }

    pub(super) fn headers(&self, request: RequestBuilder) -> RequestBuilder {
        let request = request.header(ACCEPT_ENCODING, "identity");
        if self.size > 0
            && let Some(etag) = &self.etag
        {
            request
                .header(RANGE, format!("bytes={}-", self.size))
                .header(IF_RANGE, etag)
        } else {
            request
        }
    }

    pub(super) fn has_prefix(&self) -> bool {
        self.size > 0
    }

    pub(super) async fn reset(&mut self) -> crate::Result<()> {
        self.file.set_len(0).await?;
        self.file.seek(std::io::SeekFrom::Start(0)).await?;
        self.hasher = ContentHasher::default();
        self.size = 0;
        self.total = None;
        self.etag = None;
        Ok(())
    }

    pub(super) async fn read_response(
        &mut self,
        response: Response,
        mut progress: Option<&mut FetchProgressFn<'_>>,
    ) -> crate::Result<()> {
        let etag = response.headers().get(ETAG).cloned().filter(|value| {
            value.to_str().is_ok_and(|value| {
                value.starts_with('"') && value.ends_with('"')
            })
        });
        let response_end;
        if response.status() == StatusCode::PARTIAL_CONTENT {
            let range = response
                .headers()
                .get(CONTENT_RANGE)
                .and_then(|value| value.to_str().ok())
                .and_then(content_range);
            let valid = self.size > 0
                && self.etag.is_some()
                && etag == self.etag
                && range.is_some_and(|(start, end, total)| {
                    start == self.size
                        && Some(total) == self.total
                        && response
                            .content_length()
                            .is_none_or(|length| length == end - start + 1)
                });
            if !valid {
                self.reset().await?;
                return Err(crate::ErrorKind::InputError(
                    "invalid resumed download response".into(),
                )
                .into());
            }
            response_end = range.map(|(_, end, _)| end + 1);
        } else {
            self.reset().await?;
            self.total = response.content_length();
            self.etag = etag.filter(|_| self.total.is_some());
            response_end = self.total;
        }
        let mut stream = response.bytes_stream();
        while let Some(chunk) =
            crate::install::control::download_step(stream.next()).await?
        {
            let chunk = match chunk {
                Ok(chunk) => chunk,
                Err(error) => {
                    self.file.flush().await?;
                    return Err(error.into());
                }
            };
            if response_end.is_some_and(|end| {
                self.size.saturating_add(chunk.len() as u64) > end
            }) {
                self.reset().await?;
                return Err(crate::ErrorKind::InputError(
                    "download exceeded its declared range".into(),
                )
                .into());
            }
            if let Err(error) = self.file.write_all(&chunk).await {
                self.reset().await?;
                return Err(error.into());
            }
            self.hasher.update(&chunk);
            self.size += chunk.len() as u64;
            if let Some(progress) = progress.as_mut() {
                progress(self.size, self.total.unwrap_or(0)).await?;
            }
        }
        self.file.flush().await?;
        if self.total.is_some_and(|total| total != self.size) {
            return Err(crate::ErrorKind::InputError(
                "download ended before the complete file arrived".into(),
            )
            .into());
        }
        Ok(())
    }

    pub(super) async fn finish(self) -> crate::Result<DownloadedFile> {
        self.file.sync_all().await?;
        drop(self.file);
        let hashes = self.hasher.finish(self.size);
        Ok(DownloadedFile {
            size: self.size,
            sha512: hashes.sha512.clone(),
            reused: false,
            path: DownloadedFilePath::Temporary(Arc::new(self.path), hashes),
        })
    }
}

fn content_range(value: &str) -> Option<(u64, u64, u64)> {
    let (range, total) = value.strip_prefix("bytes ")?.split_once('/')?;
    let (start, end) = range.split_once('-')?;
    let (start, end, total) =
        (start.parse().ok()?, end.parse().ok()?, total.parse().ok()?);
    (start <= end && end < total).then_some((start, end, total))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::fetch::{
        FetchSemaphore, TEST_FILE_DOWNLOAD_CLIENT, fetch_file_mirrors_in,
    };
    use std::time::Duration;
    use tokio::io::AsyncReadExt;

    fn response(
        status: &str,
        headers: &str,
        body: &[u8],
        length: usize,
    ) -> Vec<u8> {
        let mut bytes = format!("HTTP/1.1 {status}\r\nContent-Length: {length}\r\nConnection: close\r\n{headers}\r\n").into_bytes();
        bytes.extend_from_slice(body);
        bytes
    }

    async fn download(
        responses: Vec<Vec<u8>>,
        expected: &[u8],
    ) -> (crate::Result<Vec<u8>>, Vec<String>) {
        crate::install_crypto_provider();
        let directory = tempfile::tempdir().unwrap();
        let listener =
            tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/file", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let mut requests = Vec::new();
            for response in responses {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut request = Vec::new();
                loop {
                    assert!(stream.read_buf(&mut request).await.unwrap() > 0);
                    if request.windows(4).any(|value| value == b"\r\n\r\n") {
                        break;
                    }
                    assert!(request.len() < 8192);
                }
                requests
                    .push(String::from_utf8(request).unwrap().to_lowercase());
                stream.write_all(&response).await.unwrap();
                tokio::time::sleep(Duration::from_millis(30)).await;
            }
            requests
        });
        let client = reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(3))
            .build()
            .unwrap();
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect_lazy("sqlite::memory:")
            .unwrap();
        let semaphore = FetchSemaphore(tokio::sync::Semaphore::new(1));
        let sha1 = sha1_smol::Sha1::from(expected).hexdigest();
        let file = TEST_FILE_DOWNLOAD_CLIENT
            .scope(
                client,
                fetch_file_mirrors_in(
                    &[&url],
                    Some(&sha1),
                    None,
                    None,
                    &semaphore,
                    &pool,
                    None,
                    Some(directory.path()),
                ),
            )
            .await;
        let result = match file {
            Ok(file) => Ok(tokio::fs::read(file.path()).await.unwrap()),
            Err(error) => Err(error),
        };
        assert!(
            tokio::fs::read_dir(directory.path())
                .await
                .unwrap()
                .next_entry()
                .await
                .unwrap()
                .is_none()
        );
        let requests = tokio::time::timeout(Duration::from_secs(5), server)
            .await
            .unwrap()
            .unwrap();
        (result, requests)
    }

    #[tokio::test]
    async fn retries_resume_with_strong_etag_and_verified_range() {
        let (result, requests) = download(
            vec![
                response("200 OK", "ETag: \"v1\"\r\n", b"abcd", 12),
                response(
                    "206 Partial Content",
                    "ETag: \"v1\"\r\nContent-Range: bytes 4-11/12\r\n",
                    b"efghijkl",
                    8,
                ),
            ],
            b"abcdefghijkl",
        )
        .await;
        assert_eq!(result.unwrap(), b"abcdefghijkl");
        assert!(!requests[0].contains("\r\nrange:"));
        assert!(requests[1].contains("range: bytes=4-\r\n"));
        assert!(requests[1].contains("if-range: \"v1\"\r\n"));
        assert!(
            requests
                .iter()
                .all(|request| request.contains("accept-encoding: identity"))
        );
    }

    #[tokio::test]
    async fn chunked_partial_responses_are_supported() {
        let (result, requests) = download(vec![
            response("200 OK", "ETag: \"v1\"\r\n", b"abcd", 12),
            b"HTTP/1.1 206 Partial Content\r\nTransfer-Encoding: chunked\r\nConnection: close\r\nETag: \"v1\"\r\nContent-Range: bytes 4-11/12\r\n\r\n8\r\nefghijkl\r\n0\r\n\r\n".to_vec(),
        ], b"abcdefghijkl").await;
        assert_eq!(result.unwrap(), b"abcdefghijkl");
        assert!(requests[1].contains("range: bytes=4-"));
    }

    #[tokio::test]
    async fn a_full_response_replaces_the_prefix_instead_of_appending() {
        for tag in ["v1", "v2"] {
            let (result, requests) = download(
                vec![
                    response("200 OK", "ETag: \"v1\"\r\n", b"abcd", 12),
                    response(
                        "200 OK",
                        &format!("ETag: \"{tag}\"\r\n"),
                        b"ABCDEFGHIJKL",
                        12,
                    ),
                ],
                b"ABCDEFGHIJKL",
            )
            .await;
            assert_eq!(result.unwrap(), b"ABCDEFGHIJKL");
            assert!(requests[1].contains("range: bytes=4-"));
        }
    }

    #[tokio::test]
    async fn weak_or_missing_validators_restart_safely() {
        for headers in ["", "ETag: W/\"v1\"\r\n"] {
            let (result, requests) = download(
                vec![
                    response("200 OK", headers, b"abcd", 12),
                    response("200 OK", headers, b"abcdefghijkl", 12),
                ],
                b"abcdefghijkl",
            )
            .await;
            assert_eq!(result.unwrap(), b"abcdefghijkl");
            assert!(!requests[1].contains("\r\nrange:"));
        }
    }

    #[tokio::test]
    async fn wrong_range_or_changed_validator_discards_the_prefix() {
        for headers in [
            "ETag: \"v1\"\r\nContent-Range: bytes 5-11/12\r\n",
            "ETag: \"v2\"\r\nContent-Range: bytes 4-11/12\r\n",
        ] {
            let (result, requests) = download(
                vec![
                    response("200 OK", "ETag: \"v1\"\r\n", b"abcd", 12),
                    response("206 Partial Content", headers, b"xxxxxxxx", 8),
                    response("200 OK", "ETag: \"v2\"\r\n", b"abcdefghijkl", 12),
                ],
                b"abcdefghijkl",
            )
            .await;
            assert_eq!(result.unwrap(), b"abcdefghijkl");
            assert!(requests[1].contains("range: bytes=4-"));
            assert!(!requests[2].contains("\r\nrange:"));
        }
    }

    #[tokio::test]
    async fn unsatisfiable_ranges_restart_the_download() {
        let (result, requests) = download(
            vec![
                response("200 OK", "ETag: \"v1\"\r\n", b"abcd", 12),
                response(
                    "416 Range Not Satisfiable",
                    "Content-Range: bytes */12\r\n",
                    b"",
                    0,
                ),
                response("200 OK", "ETag: \"v1\"\r\n", b"abcdefghijkl", 12),
            ],
            b"abcdefghijkl",
        )
        .await;
        assert_eq!(result.unwrap(), b"abcdefghijkl");
        assert!(!requests[2].contains("\r\nrange:"));
    }

    #[tokio::test]
    async fn checksum_failures_do_not_resume_an_invalid_complete_file() {
        let (result, requests) = download(
            vec![
                response("200 OK", "ETag: \"v1\"\r\n", b"xxxxxxxxxxxx", 12),
                response("200 OK", "ETag: \"v1\"\r\n", b"abcdefghijkl", 12),
            ],
            b"abcdefghijkl",
        )
        .await;
        assert_eq!(result.unwrap(), b"abcdefghijkl");
        assert!(!requests[1].contains("\r\nrange:"));
    }

    #[tokio::test]
    async fn exhausted_retries_remove_the_unfinished_file() {
        let responses = (0..=crate::util::fetch::FETCH_ATTEMPTS)
            .map(|_| response("200 OK", "ETag: \"v1\"\r\n", b"abcd", 12))
            .collect();
        let (result, _) = download(responses, b"abcdefghijkl").await;
        assert!(result.is_err());
    }

    #[test]
    fn range_parser_rejects_inconsistent_or_unknown_bounds() {
        assert_eq!(content_range("bytes 4-11/12"), Some((4, 11, 12)));
        for value in [
            "bytes 4-3/12",
            "bytes 4-12/12",
            "bytes 4-11/*",
            "bytes */12",
            "items 4-11/12",
            "bytes -1-11/12",
        ] {
            assert!(content_range(value).is_none(), "{value}");
        }
    }
}
