use crate::ErrorKind;
use crate::state::content_store::adapters::downloads;
use crate::state::content_store::adapters::filesystem;
use crate::state::content_store::adapters::sqlite as catalog;
use crate::state::content_store::{
    ContentStore, GetFileResult, StoredFileHandle, StoredFileMetadata,
    StoredFileStatus, validate_digest,
};
use crate::util::content_hash::{
    ContentHasher, FileIntegrity, hash_file, temporary_file,
};
use crate::util::fetch::{self, DownloadMeta, FetchProgressFn, FetchSemaphore};
use anyhow::{Context, bail};
use std::path::Path;
use tokio::fs::File;

impl ContentStore {
    pub(crate) async fn get_or_download_file(
        &self,
        mirrors: &[&str],
        sha512: Option<&str>,
        size: Option<u64>,
        download_meta: Option<&DownloadMeta>,
        semaphore: &FetchSemaphore,
        progress: Option<&mut FetchProgressFn<'_>>,
    ) -> crate::Result<GetFileResult> {
        let sha512 = sha512.ok_or_else(|| {
            ErrorKind::ContentStoreError(
                "content download is missing a SHA-512 hash".into(),
            )
        })?;
        validate_digest(sha512, 128)?;
        let _acquisition = self.download_lock(sha512).lock().await;
        if let Some(stored_file) = self.lookup(Some(sha512), size).await? {
            return Ok(GetFileResult {
                stored_file,
                reused: true,
            });
        }
        let download = downloads::download(
            &self.staging,
            &self.pool,
            mirrors,
            download_meta,
            semaphore,
            progress,
        )
        .await?;
        if sha512 != download.sha512 {
            bail!(ErrorKind::ContentStoreError(format!(
                "downloaded content SHA-512 mismatch: expected {sha512}, got {}",
                download.sha512,
            )));
        }
        if let Some(expected_size) = size
            && expected_size != download.size
        {
            tracing::warn!(
                expected_size,
                actual_size = download.size,
                sha512 = %download.sha512,
                "Downloaded content matches SHA-512 but differs from the declared size; using its actual size"
            );
        }
        let sources = mirrors
            .iter()
            .filter_map(|source| {
                let url = url::Url::parse(source).ok()?;
                (url.scheme() == "https"
                    && url.username().is_empty()
                    && url.password().is_none()
                    && url.query().is_none())
                .then(|| source.to_string())
            })
            .collect::<Vec<_>>();
        let stored_file = self
            .save_staged_file(download.into_staged()?, &sources)
            .await?;
        Ok(GetFileResult {
            stored_file,
            reused: false,
        })
    }

    pub(crate) async fn get_or_download_file_by_checksum(
        &self,
        mirrors: &[&str],
        integrity: FileIntegrity<'_>,
        local_files: &[std::path::PathBuf],
        download_meta: Option<&DownloadMeta>,
        semaphore: &FetchSemaphore,
    ) -> crate::Result<Option<GetFileResult>> {
        let checksum = integrity.checksum();
        if let Some((algorithm, digest)) = &checksum {
            validate_digest(
                digest,
                if *algorithm == "sha1" { 40 } else { 32 },
            )?;
        }
        let key = checksum
            .as_ref()
            .map(|(_, digest)| digest.as_str())
            .or_else(|| mirrors.first().copied())
            .unwrap_or_default();
        let _acquisition = self.download_lock(key).lock().await;
        if let Some((algorithm, digest)) = &checksum {
            for sha512 in
                catalog::find_checksums(&self.pool, algorithm, digest).await?
            {
                if let Some(stored_file) =
                    self.lookup(Some(&sha512), integrity.size).await?
                    && integrity.matches_path(&stored_file.path).await?
                {
                    return Ok(Some(GetFileResult {
                        stored_file,
                        reused: true,
                    }));
                }
            }
            for path in local_files {
                match integrity.matches_path(path).await {
                    Ok(true) => {
                        let staged = self.stage_file(path).await?;
                        if !integrity.matches(&staged.hashes) {
                            bail!(ErrorKind::ContentStoreError(
                                "local content changed while being imported"
                                    .into()
                            ));
                        }
                        let stored_file =
                            self.save_staged_file(staged, &[]).await?;
                        return Ok(Some(GetFileResult {
                            stored_file,
                            reused: true,
                        }));
                    }
                    Ok(false) => {}
                    Err(error) => {
                        tracing::warn!(path = %path.display(), %error, "Could not verify local content")
                    }
                }
            }
        }
        if mirrors.is_empty() {
            return Ok(None);
        }
        if let Some(size) = integrity.size {
            self.require_staging_space(size)?;
        }
        let sha1 = integrity.sha1.map(str::to_ascii_lowercase);
        let download = fetch::fetch_file_mirrors_in(
            mirrors,
            sha1.as_deref(),
            download_meta,
            None,
            semaphore,
            &self.pool,
            None,
            Some(&self.staging),
        )
        .await?;
        let staged = download.into_staged()?;
        if !integrity.matches(&staged.hashes) {
            bail!(ErrorKind::ContentStoreError(
                "downloaded content failed its size or checksum check".into()
            ));
        }
        let sources = mirrors
            .iter()
            .filter_map(|source| {
                let url = url::Url::parse(source).ok()?;
                (url.scheme() == "https"
                    && url.username().is_empty()
                    && url.password().is_none()
                    && url.query().is_none())
                .then(|| source.to_string())
            })
            .collect::<Vec<_>>();
        let stored_file = self.save_staged_file(staged, &sources).await?;
        Ok(Some(GetFileResult {
            stored_file,
            reused: false,
        }))
    }

    pub(crate) async fn store_file(
        &self,
        source: &Path,
    ) -> crate::Result<StoredFileHandle> {
        if let Some(stored_file) = self.find_file_by_path(source).await? {
            return Ok(stored_file);
        }
        let hashes = hash_file(source).await?;
        if let Some(stored_file) =
            self.lookup(Some(&hashes.sha512), Some(hashes.size)).await?
        {
            return Ok(stored_file);
        }
        let staged = self.stage_file(source).await?;
        self.save_staged_file(staged, &[]).await
    }

    pub(in crate::state::content_store) async fn save_staged_file(
        &self,
        staged: fetch::StagedDownload,
        sources: &[String],
    ) -> crate::Result<StoredFileHandle> {
        if staged.path.parent() != Some(self.staging.as_path()) {
            bail!(ErrorKind::ContentStoreError(
                "content was not staged in the shared store".into()
            ));
        }
        let fetch::StagedDownload {
            path: temporary,
            hashes,
        } = staged;
        let size = hashes.size;
        let hash = hashes.sha512.clone();
        let guard = self.lease().await;
        let _publication = self.publish_lock(&hash).lock().await;
        if let Some(existing) = self
            .lookup_with_guard(Some(&hash), Some(size), guard.clone())
            .await?
        {
            catalog::save_checksums(&self.pool, &hashes).await?;
            return Ok(existing);
        }
        let destination = self
            .root
            .join(crate::state::content_store::object_relative_path(&hash)?);
        filesystem::publish_staged_file(
            &self.root,
            &destination,
            temporary,
            &hash,
        )
        .await?;
        let stored_file = StoredFileMetadata {
            sha512: hash,
            size: size.try_into().context(ErrorKind::ContentStoreError(
                "content file is too large".into(),
            ))?,
            status: StoredFileStatus::Ready,
            modified_as: filesystem::modified_at_ns(&destination).await?,
            last_used_at: chrono::Utc::now().timestamp(),
            sources: catalog::encode_sources(sources)?,
        };
        catalog::save_file(&self.pool, &stored_file).await?;
        catalog::save_checksums(&self.pool, &hashes).await?;
        Ok(StoredFileHandle {
            metadata: stored_file,
            path: destination,
            _guard: guard,
        })
    }

    pub(crate) async fn store_bytes(
        &self,
        bytes: &[u8],
    ) -> crate::Result<StoredFileHandle> {
        let (mut file, temporary) = self.temporary().await?;
        filesystem::write_staged_bytes(&mut file, bytes).await?;
        drop(file);
        let mut hasher = ContentHasher::default();
        hasher.update(bytes);
        self.save_staged_file(
            fetch::StagedDownload {
                path: temporary,
                hashes: hasher.finish(bytes.len() as u64),
            },
            &[],
        )
        .await
    }

    pub(crate) async fn temporary(
        &self,
    ) -> crate::Result<(File, tempfile::TempPath)> {
        temporary_file(Some(&self.staging)).await
    }
    pub(crate) fn require_staging_space(&self, size: u64) -> crate::Result<()> {
        filesystem::require_staging_space(&self.staging, size)
    }

    pub(in crate::state::content_store) async fn stage_file(
        &self,
        source: &Path,
    ) -> crate::Result<fetch::StagedDownload> {
        filesystem::stage_file(&self.staging, source).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::DirectoryInfo;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    async fn store() -> (tempfile::TempDir, DirectoryInfo, ContentStore) {
        crate::install_crypto_provider();
        let directory = tempfile::tempdir().unwrap();
        let dirs = DirectoryInfo {
            settings_dir: directory.path().to_path_buf(),
            config_dir: directory.path().to_path_buf(),
            app_identifier: "checksum-tests".into(),
        };
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                sqlx::sqlite::SqliteConnectOptions::new()
                    .filename(directory.path().join("test.db"))
                    .create_if_missing(true),
            )
            .await
            .unwrap();
        sqlx::migrate!().run(&pool).await.unwrap();
        let lock = ContentStore::lock_process(&dirs.settings_dir)
            .await
            .unwrap();
        let store = ContentStore::new(&dirs, pool, lock).await.unwrap();
        (directory, dirs, store)
    }

    async fn server(
        body: &'static [u8],
    ) -> (String, Arc<AtomicUsize>, tokio::task::JoinHandle<()>) {
        let listener =
            tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/file.jar", listener.local_addr().unwrap());
        let requests = Arc::new(AtomicUsize::new(0));
        let count = requests.clone();
        let task = tokio::spawn(async move {
            loop {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut request = Vec::new();
                loop {
                    assert!(stream.read_buf(&mut request).await.unwrap() > 0);
                    if request.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
                        break;
                    }
                    assert!(request.len() < 8192);
                }
                count.fetch_add(1, Ordering::SeqCst);
                stream.write_all(format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).as_bytes()).await.unwrap();
                for chunk in body.chunks(4096) {
                    stream.write_all(chunk).await.unwrap();
                }
            }
        });
        (url, requests, task)
    }

    async fn with_local_downloads(
        future: impl std::future::Future<Output = ()>,
    ) {
        crate::install_crypto_provider();
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        fetch::TEST_FILE_DOWNLOAD_CLIENT.scope(client, future).await;
    }

    #[tokio::test]
    async fn migration_indexes_existing_files_and_cascades_cache_cleanup() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::raw_sql("CREATE TABLE curseforge_pending_files (sha1 TEXT); CREATE TABLE store_blobs (sha512 TEXT PRIMARY KEY); CREATE TABLE instance_files (id TEXT, sha1 TEXT); CREATE TABLE store_instance_files (file_id TEXT, blob_sha512 TEXT);")
            .execute(&pool).await.unwrap();
        let sha1 = "A".repeat(40);
        let sha512 = "b".repeat(128);
        sqlx::query("INSERT INTO store_blobs VALUES (?)")
            .bind(&sha512)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO instance_files VALUES ('existing', ?)")
            .bind(&sha1)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO store_instance_files VALUES ('existing', ?)")
            .bind(&sha512)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::raw_sql(include_str!(
            "../../../../migrations/20261115121500_store-content-checksums.sql"
        ))
        .execute(&pool)
        .await
        .unwrap();
        assert_eq!(
            catalog::find_checksums(&pool, "sha1", &sha1.to_ascii_lowercase())
                .await
                .unwrap(),
            vec![sha512.clone()]
        );
        sqlx::query("DELETE FROM store_blobs WHERE sha512 = ?")
            .bind(&sha512)
            .execute(&pool)
            .await
            .unwrap();
        assert!(
            catalog::find_checksums(&pool, "sha1", &sha1.to_ascii_lowercase())
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn streamed_download_is_shared_persistent_and_deduplicated() {
        with_local_downloads(async {
            let (_directory, dirs, store) = store().await;
            static BODY: [u8; 256 * 1024] = [42; 256 * 1024];
            let body = BODY.as_slice();
            let sha1 = sha1_smol::Sha1::from(body).hexdigest();
            let integrity = FileIntegrity {
                sha1: Some(&sha1),
                size: Some(body.len() as u64),
                ..Default::default()
            };
            let semaphore = FetchSemaphore(tokio::sync::Semaphore::new(4));
            let (url, requests, task) = server(body).await;
            let mirrors = [url.as_str()];
            let (first, second) = tokio::join!(
                store.get_or_download_file_by_checksum(
                    &mirrors,
                    integrity,
                    &[],
                    None,
                    &semaphore
                ),
                store.get_or_download_file_by_checksum(
                    &mirrors,
                    integrity,
                    &[],
                    None,
                    &semaphore
                ),
            );
            let first = first.unwrap().unwrap();
            let second = second.unwrap().unwrap();
            assert_ne!(first.reused, second.reused);
            assert_eq!(requests.load(Ordering::SeqCst), 1);
            assert_eq!(first.stored_file.path, second.stored_file.path);
            assert_eq!(
                tokio::fs::read(&first.stored_file.path).await.unwrap(),
                body
            );
            let md5 = hash_file(&first.stored_file.path).await.unwrap().md5;
            let pool = store.pool.clone();
            drop((first, second, store));
            pool.close().await;
            let pool = sqlx::sqlite::SqlitePoolOptions::new()
                .max_connections(1)
                .connect_with(
                    sqlx::sqlite::SqliteConnectOptions::new()
                        .filename(dirs.settings_dir.join("test.db")),
                )
                .await
                .unwrap();
            let lock = ContentStore::lock_process(&dirs.settings_dir)
                .await
                .unwrap();
            let store = ContentStore::new(&dirs, pool, lock).await.unwrap();
            let cached = store
                .get_or_download_file_by_checksum(
                    &[],
                    FileIntegrity {
                        md5: Some(&md5),
                        size: integrity.size,
                        ..Default::default()
                    },
                    &[],
                    None,
                    &semaphore,
                )
                .await
                .unwrap()
                .unwrap();
            assert!(cached.reused);
            assert_eq!(requests.load(Ordering::SeqCst), 1);
            assert!(
                tokio::fs::read_dir(&store.staging)
                    .await
                    .unwrap()
                    .next_entry()
                    .await
                    .unwrap()
                    .is_none()
            );
            task.abort();
        })
        .await;
    }

    #[tokio::test]
    async fn bundled_restricted_files_require_a_matching_checksum() {
        let (directory, _dirs, store) = store().await;
        let path = directory.path().join("bundled.jar");
        tokio::fs::write(&path, b"abc").await.unwrap();
        let sha1 = sha1_smol::Sha1::from(b"abc")
            .hexdigest()
            .to_ascii_uppercase();
        let integrity = FileIntegrity {
            sha1: Some(&sha1),
            size: Some(3),
            ..Default::default()
        };
        let semaphore = FetchSemaphore(tokio::sync::Semaphore::new(1));
        let files = [path.clone()];
        assert!(
            store
                .get_or_download_file_by_checksum(
                    &[],
                    FileIntegrity {
                        sha1: None,
                        ..integrity
                    },
                    &files,
                    None,
                    &semaphore
                )
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            store
                .get_or_download_file_by_checksum(
                    &[],
                    FileIntegrity {
                        sha1: Some("0000000000000000000000000000000000000000"),
                        ..integrity
                    },
                    &files,
                    None,
                    &semaphore
                )
                .await
                .unwrap()
                .is_none()
        );
        let found = store
            .get_or_download_file_by_checksum(
                &[],
                integrity,
                &files,
                None,
                &semaphore,
            )
            .await
            .unwrap()
            .unwrap();
        assert!(found.reused);
        assert_eq!(
            tokio::fs::read(found.stored_file.path).await.unwrap(),
            b"abc"
        );
    }

    #[tokio::test]
    async fn wrong_local_content_is_downloaded_and_bad_downloads_are_not_cached()
     {
        with_local_downloads(async {
            let (directory, _dirs, store) = store().await;
            let path = directory.path().join("mod.jar");
            tokio::fs::write(&path, b"bad").await.unwrap();
            let sha1 = sha1_smol::Sha1::from(b"abc").hexdigest();
            let integrity = FileIntegrity {
                sha1: Some(&sha1),
                size: Some(3),
                ..Default::default()
            };
            let semaphore = FetchSemaphore(tokio::sync::Semaphore::new(1));
            let (url, requests, task) = server(b"abc").await;
            let acquired = store
                .get_or_download_file_by_checksum(
                    &[&url],
                    integrity,
                    &[directory.path().join("missing.jar"), path],
                    None,
                    &semaphore,
                )
                .await
                .unwrap()
                .unwrap();
            assert!(!acquired.reused);
            assert_eq!(requests.load(Ordering::SeqCst), 1);
            assert!(
                store
                    .get_or_download_file_by_checksum(
                        &[&url],
                        FileIntegrity {
                            size: Some(4),
                            ..integrity
                        },
                        &[],
                        None,
                        &semaphore
                    )
                    .await
                    .is_err()
            );
            assert!(
                store
                    .get_or_download_file_by_checksum(
                        &[&url],
                        FileIntegrity {
                            sha1: None,
                            md5: Some("00000000000000000000000000000000"),
                            size: Some(3)
                        },
                        &[],
                        None,
                        &semaphore
                    )
                    .await
                    .is_err()
            );
            assert!(
                store
                    .get_or_download_file_by_checksum(
                        &[],
                        FileIntegrity {
                            sha1: None,
                            md5: Some("00000000000000000000000000000000"),
                            size: Some(3)
                        },
                        &[],
                        None,
                        &semaphore
                    )
                    .await
                    .unwrap()
                    .is_none()
            );
            assert!(
                tokio::fs::read_dir(&store.staging)
                    .await
                    .unwrap()
                    .next_entry()
                    .await
                    .unwrap()
                    .is_none()
            );
            task.abort();
        })
        .await;
    }
}
