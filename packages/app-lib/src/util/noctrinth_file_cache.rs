//! Keeping a downloaded content file under its own hash, so it is fetched once.
//!
//! Every content file the launcher downloads is checked against a SHA-1 it was
//! told in advance, which means that hash is enough to name the file: two
//! downloads sharing a hash are the same bytes, whichever pack, version or
//! instance asked for them. Storing the file under that name turns a
//! reinstall, a downgrade, and the second instance of a pack into local
//! copies.
//!
//! Only content is kept here. Minecraft's own libraries and assets already
//! have a shared store, and keeping them a second time would buy nothing for
//! the disk it costs — which is why an entry is only made for a download that
//! carries [`crate::util::fetch::DownloadMeta`], the mark of a file fetched
//! for an instance's content.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::SystemTime;

use crate::util::fetch::DownloadedFile;
use crate::util::io;

/// Unset until the launcher's directories are known, and while it is unset
/// nothing is cached — the handful of downloads made that early are metadata
/// nobody re-downloads anyway.
static CACHE_DIR: parking_lot::RwLock<Option<PathBuf>> =
    parking_lot::RwLock::new(None);

/// What the last scan found the cache holding, kept up to date as entries are
/// added so that growing past the budget is noticed without walking the
/// directory every time.
static CACHED_BYTES: AtomicU64 = AtomicU64::new(0);

/// Set while a prune is running, so a burst of stores starts one sweep rather
/// than one each.
static PRUNING: AtomicBool = AtomicBool::new(false);

/// How much the cache may hold before its least recently used entries are
/// dropped. Large enough for a few modpacks — the case it exists for — and
/// small enough not to quietly eat a disk.
const CACHE_BUDGET: u64 = 4 * 1024 * 1024 * 1024;

/// What a prune brings the cache down to. Below the budget, so that a prune
/// makes room for a while instead of running again on the next file.
const PRUNE_TARGET: u64 = CACHE_BUDGET / 10 * 8;

const HASH_BUFFER_SIZE: usize = 262144;

/// Points the cache at `dir` and takes stock of what an earlier run left there.
pub fn set_file_cache_dir(dir: PathBuf) {
    if let Err(error) = std::fs::create_dir_all(&dir) {
        tracing::warn!(
            "Could not create the download cache directory {}: {error} — \
             downloads will not be kept",
            dir.display()
        );
        return;
    }

    CACHED_BYTES.store(prune_to_budget(&dir), Ordering::Relaxed);
    *CACHE_DIR.write() = Some(dir);
}

/// Where a file with this hash would live: two characters of the hash for a
/// directory, the hash itself for the name, the same shape Minecraft's own
/// asset objects use.
///
/// `None` for anything that is not a SHA-1, which also keeps a hash that came
/// from somewhere unexpected from naming a path outside the cache.
fn entry_path(dir: &Path, sha1: &str) -> Option<PathBuf> {
    let sha1 = sha1.trim().to_ascii_lowercase();
    if sha1.len() != 40 || !sha1.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }

    Some(dir.join(&sha1[..2]).join(&sha1))
}

/// The cached file with this hash, if there is one and it is still intact.
pub(crate) async fn lookup(sha1: &str) -> Option<DownloadedFile> {
    let dir = CACHE_DIR.read().clone()?;
    let path = entry_path(&dir, sha1)?;

    let metadata = io::metadata(&path).await.ok()?;
    if !metadata.is_file() {
        return None;
    }

    // Named after its own hash, and read back to check it all the same. An
    // entry the disk damaged would otherwise be installed as though it were
    // the mod it is named after, and a game that crashes for that reason is
    // close to undiagnosable. The read is local, so it still costs a small
    // fraction of fetching the file again — and a bad entry repairs itself,
    // because dropping it here sends the caller to the network.
    match hash_file(&path).await {
        Ok(hash) if hash.eq_ignore_ascii_case(sha1) => {}
        Ok(_) => {
            tracing::warn!(
                "Cached download {} no longer matches the hash it is named \
                 after, so it is being dropped",
                path.display()
            );
            if io::remove_file(&path).await.is_ok() {
                let _ = CACHED_BYTES.fetch_update(
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                    |total| Some(total.saturating_sub(metadata.len())),
                );
            }
            return None;
        }
        Err(error) => {
            tracing::debug!(
                "Could not read the cached download {}: {error}",
                path.display()
            );
            return None;
        }
    }

    touch(&path).await;

    Some(DownloadedFile::from_cache(
        path,
        metadata.len(),
        sha1.to_ascii_lowercase(),
    ))
}

/// Keeps a freshly downloaded file, and hands back a handle to wherever it
/// ended up.
pub(crate) async fn store(file: DownloadedFile) -> DownloadedFile {
    let Some(dir) = CACHE_DIR.read().clone() else {
        return file;
    };
    let Some(path) = entry_path(&dir, &file.sha1) else {
        return file;
    };

    // Another download of the same file got there first; the bytes are the
    // same, so there is nothing to do but leave that entry alone.
    if io::metadata(&path).await.is_ok() {
        return file;
    }

    let size = file.size;
    match file.persist_into_cache(&path).await {
        Ok(file) => {
            let total = CACHED_BYTES.fetch_add(size, Ordering::Relaxed) + size;
            if total > CACHE_BUDGET {
                spawn_prune(dir);
            }
            file
        }
        Err(file) => file,
    }
}

/// Empties the cache. Everything in it can be downloaded again, so this only
/// ever costs time.
pub async fn purge_file_cache() -> crate::Result<()> {
    let Some(dir) = CACHE_DIR.read().clone() else {
        return Ok(());
    };

    if io::metadata(&dir).await.is_ok() {
        io::remove_dir_all(&dir).await?;
    }
    io::create_dir_all(&dir).await?;
    CACHED_BYTES.store(0, Ordering::Relaxed);

    Ok(())
}

/// What the cache currently holds, in bytes.
pub fn file_cache_size() -> u64 {
    CACHED_BYTES.load(Ordering::Relaxed)
}

async fn hash_file(path: &Path) -> crate::Result<String> {
    use tokio::io::AsyncReadExt;

    let mut file = tokio::fs::File::open(path)
        .await
        .map_err(|error| io::IOError::with_path(error, path))?;
    let mut hasher = sha1_smol::Sha1::new();
    let mut buffer = vec![0u8; HASH_BUFFER_SIZE];

    loop {
        let read = file
            .read(&mut buffer)
            .await
            .map_err(|error| io::IOError::with_path(error, path))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(hasher.digest().to_string())
}

/// Marks an entry as used, so that pruning drops what nobody has wanted for
/// longest rather than what happened to be downloaded first. Best effort: a
/// cache that cannot be written to still answers reads.
async fn touch(path: &Path) {
    let path = path.to_path_buf();
    let _ = tokio::task::spawn_blocking(move || {
        std::fs::OpenOptions::new()
            .write(true)
            .open(&path)
            .and_then(|file| file.set_modified(SystemTime::now()))
    })
    .await;
}

fn spawn_prune(dir: PathBuf) {
    if PRUNING.swap(true, Ordering::SeqCst) {
        return;
    }

    tokio::task::spawn_blocking(move || {
        CACHED_BYTES.store(prune_to_budget(&dir), Ordering::Relaxed);
        PRUNING.store(false, Ordering::SeqCst);
    });
}

/// Drops the least recently used entries until the cache is back under
/// [`PRUNE_TARGET`], and returns what it holds afterwards.
fn prune_to_budget(dir: &Path) -> u64 {
    let mut entries = Vec::new();
    let mut total = 0_u64;

    let Ok(buckets) = std::fs::read_dir(dir) else {
        return 0;
    };
    for bucket in buckets.flatten() {
        let Ok(files) = std::fs::read_dir(bucket.path()) else {
            continue;
        };
        for file in files.flatten() {
            let Ok(metadata) = file.metadata() else {
                continue;
            };
            if !metadata.is_file() {
                continue;
            }
            total += metadata.len();
            entries.push((
                file.path(),
                metadata.len(),
                metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
            ));
        }
    }

    if total <= CACHE_BUDGET {
        return total;
    }

    entries.sort_by_key(|(_, _, used_at)| *used_at);
    for (path, size, _) in entries {
        if total <= PRUNE_TARGET {
            break;
        }
        if std::fs::remove_file(&path).is_ok() {
            total = total.saturating_sub(size);
        }
    }

    total
}
