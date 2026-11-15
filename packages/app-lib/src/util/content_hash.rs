use sha2::{Digest, Sha512};
use std::path::Path;
use tokio::fs::File;
use tokio::io::{AsyncRead, AsyncWrite, BufReader, copy_buf};
use tokio_util::io::InspectReader;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FileHashes {
    pub sha512: String,
    pub sha1: String,
    pub md5: String,
    pub size: u64,
}

#[derive(Default)]
pub(crate) struct ContentHasher {
    sha512: Sha512,
    sha1: sha1_smol::Sha1,
    md5: md5::Md5,
}

impl ContentHasher {
    pub(crate) fn update(&mut self, bytes: &[u8]) {
        self.sha512.update(bytes);
        self.sha1.update(bytes);
        md5::Digest::update(&mut self.md5, bytes);
    }

    pub(crate) fn finish(self, size: u64) -> FileHashes {
        FileHashes {
            sha512: encode_hex(self.sha512.finalize().as_slice()),
            sha1: self.sha1.hexdigest(),
            md5: encode_hex(md5::Digest::finalize(self.md5).as_slice()),
            size,
        }
    }
}

pub(crate) fn hash_reader(
    mut input: impl std::io::Read,
) -> std::io::Result<FileHashes> {
    let mut hasher = ContentHasher::default();
    let mut size = 0;
    let mut buffer = vec![0; 256 * 1024];
    loop {
        let read = input.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        size += read as u64;
    }
    Ok(hasher.finish(size))
}

pub(crate) async fn temporary_file(
    directory: Option<&Path>,
) -> crate::Result<(File, tempfile::TempPath)> {
    let directory = directory.map(Path::to_path_buf);
    let (file, path) = tokio::task::spawn_blocking(move || {
        match directory {
            Some(directory) => tempfile::NamedTempFile::new_in(directory),
            None => tempfile::NamedTempFile::new(),
        }
        .map(tempfile::NamedTempFile::into_parts)
    })
    .await??;
    Ok((File::from_std(file), path))
}

pub(crate) async fn hash_file(path: &Path) -> crate::Result<FileHashes> {
    hash_file_with_progress(path, &|_| {}).await
}

pub(crate) async fn hash_file_with_progress(
    path: &Path,
    on_read: &(dyn Fn(u64) + Send + Sync),
) -> crate::Result<FileHashes> {
    let file = File::open(path).await?;
    copy_and_hash(file, &mut tokio::io::sink(), on_read).await
}

pub(crate) async fn copy_and_hash(
    input: impl AsyncRead + Unpin,
    output: &mut (impl AsyncWrite + Unpin),
    on_read: &(dyn Fn(u64) + Send + Sync),
) -> crate::Result<FileHashes> {
    let mut hashes = ContentHasher::default();
    let size = {
        let reader = InspectReader::new(input, |bytes| {
            hashes.update(bytes);
            on_read(bytes.len() as u64);
        });
        let mut reader = BufReader::with_capacity(256 * 1024, reader);
        copy_buf(&mut reader, output).await?
    };
    Ok(hashes.finish(size))
}

pub(crate) fn encode_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[derive(Clone, Copy, Default)]
pub(crate) struct FileIntegrity<'a> {
    pub sha1: Option<&'a str>,
    pub md5: Option<&'a str>,
    pub size: Option<u64>,
}

impl FileIntegrity<'_> {
    pub(crate) fn checksum(self) -> Option<(&'static str, String)> {
        self.sha1
            .map(|value| ("sha1", value.to_ascii_lowercase()))
            .or_else(|| {
                self.md5.map(|value| ("md5", value.to_ascii_lowercase()))
            })
    }

    pub(crate) fn matches(self, hashes: &FileHashes) -> bool {
        self.size.is_none_or(|size| size == hashes.size)
            && match (self.sha1, self.md5) {
                (Some(expected), _) => {
                    expected.eq_ignore_ascii_case(&hashes.sha1)
                }
                (_, Some(expected)) => {
                    expected.eq_ignore_ascii_case(&hashes.md5)
                }
                _ => true,
            }
    }

    pub(crate) async fn matches_path(self, path: &Path) -> crate::Result<bool> {
        let metadata = tokio::fs::metadata(path).await?;
        if !metadata.is_file()
            || self.size.is_some_and(|size| size != metadata.len())
        {
            return Ok(false);
        }
        if self.checksum().is_none() {
            return Ok(metadata.len() > 0);
        }
        Ok(self.matches(&hash_file(path).await?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn streaming_hashes_and_integrity_checks() {
        let hashes =
            copy_and_hash(&b"abc"[..], &mut tokio::io::sink(), &|_| {})
                .await
                .unwrap();
        assert_eq!(hash_reader(&b"abc"[..]).unwrap(), hashes);
        assert_eq!(hashes.size, 3);
        assert_eq!(hashes.sha1, "a9993e364706816aba3e25717850c26c9cd0d89d");
        assert_eq!(hashes.md5, "900150983cd24fb0d6963f7d28e17f72");
        assert_eq!(
            hashes.sha512,
            "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f"
        );
        let upper = hashes.sha1.to_ascii_uppercase();
        let integrity = FileIntegrity {
            sha1: Some(&upper),
            md5: Some(&hashes.md5),
            size: Some(3),
        };
        assert!(integrity.matches(&hashes));
        assert!(
            !FileIntegrity {
                size: Some(4),
                ..integrity
            }
            .matches(&hashes)
        );
        assert!(
            !FileIntegrity {
                sha1: Some("bad"),
                ..integrity
            }
            .matches(&hashes)
        );
        assert!(
            FileIntegrity {
                sha1: None,
                ..integrity
            }
            .matches(&hashes)
        );
    }
}
