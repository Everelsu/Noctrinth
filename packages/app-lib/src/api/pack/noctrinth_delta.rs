//! Working out what a modpack update does not have to download again.
//!
//! An update is a new manifest, not a new pack: most of the jars the incoming
//! version lists are byte-for-byte the ones already sitting in the instance.
//! Deciding which ones *before* the old pack is torn off disk is what lets the
//! install leave them where they are, instead of deleting a file only to fetch
//! the same bytes back over the wire.
//!
//! Only the manifest's own files are considered. Overrides ride along inside
//! the `.mrpack` that has already been downloaded, so re-extracting them costs
//! no bandwidth and buys the certainty that configs end up as the pack author
//! intended.

use std::collections::{HashMap, HashSet};

use crate::State;
use crate::pack::install_from::{CreatePackFile, EnvType, PackFileHash};
use crate::state::{
    CachedEntry, SideType, file_hash_cache_key, file_modified_at_ns,
};
use crate::util::io;

/// The files an incoming pack shares, unchanged, with what is already on disk.
///
/// Paths are instance-relative, in the same shape the pack manifest writes
/// them, which is also the shape the install and removal passes look them up
/// by.
#[derive(Debug, Default)]
pub struct PackDelta {
    kept: HashSet<String>,
    kept_bytes: u64,
}

impl PackDelta {
    /// Whether this path is already installed as the incoming pack wants it.
    pub fn keeps(&self, relative_path: &str) -> bool {
        self.kept.contains(relative_path)
    }

    pub fn kept_files(&self) -> usize {
        self.kept.len()
    }

    pub fn kept_bytes(&self) -> u64 {
        self.kept_bytes
    }

    pub fn is_empty(&self) -> bool {
        self.kept.is_empty()
    }
}

/// Compares an incoming `.mrpack` against what the instance already holds.
///
/// A file is kept only when the manifest's size, the manifest's SHA-1 and the
/// bytes actually on disk all agree, so anything the user edited by hand is
/// fetched again rather than silently passed off as the pack's own copy.
///
/// This never fails an update: anything that cannot be established — an
/// unreadable archive, a manifest for another game, a missing instance —
/// leaves the plan empty, and an empty plan is exactly the download-everything
/// behaviour an update had before.
pub(crate) async fn plan_pack_delta(
    instance_id: &str,
    incoming: &CreatePackFile,
    state: &State,
) -> PackDelta {
    match build_pack_delta(instance_id, incoming, state).await {
        Ok(delta) => delta,
        Err(error) => {
            tracing::warn!(
                "Could not work out what instance {instance_id} already has \
                 installed, so the update will download everything: {error}"
            );
            PackDelta::default()
        }
    }
}

struct KeepCandidate {
    relative_path: String,
    expected_hash: String,
    cache_key: String,
    size: u64,
}

async fn build_pack_delta(
    instance_id: &str,
    incoming: &CreatePackFile,
    state: &State,
) -> crate::Result<PackDelta> {
    let pack = super::install_mrpack::read_pack_manifest(incoming).await?;
    if &*pack.game != "minecraft" {
        return Ok(PackDelta::default());
    }

    let Some(metadata) =
        crate::state::instances::commands::get_instance_metadata(
            instance_id,
            &state.pool,
        )
        .await?
    else {
        return Ok(PackDelta::default());
    };
    let instance_path = metadata.instance.path;
    let instance_dir = state.directories.instances_dir().join(&instance_path);

    let mut candidates = Vec::new();
    for file in &pack.files {
        // Never installed on a client, so there is nothing on disk to keep.
        if file.env.as_ref().is_some_and(|env| {
            env.get(&EnvType::Client) == Some(&SideType::Unsupported)
        }) {
            continue;
        }
        let Some(expected_hash) = file.hashes.get(&PackFileHash::Sha1) else {
            continue;
        };

        let relative_path = file.path.as_str().to_string();
        let Ok(file_metadata) =
            io::metadata(instance_dir.join(&relative_path)).await
        else {
            continue;
        };
        if !file_metadata.is_file() {
            continue;
        }
        let size = file_metadata.len();
        if size != u64::from(file.file_size) {
            continue;
        }
        let Ok(modified_at_ns) = file_modified_at_ns(&file_metadata) else {
            continue;
        };

        candidates.push(KeepCandidate {
            cache_key: file_hash_cache_key(
                size,
                modified_at_ns,
                &format!("{instance_path}/{relative_path}"),
            ),
            relative_path,
            expected_hash: expected_hash.clone(),
            size,
        });
    }

    if candidates.is_empty() {
        return Ok(PackDelta::default());
    }

    // Keyed by size and modification time, so a file the launcher installed and
    // nobody has touched since answers out of the cache; anything else is read
    // off disk and hashed for real.
    let cache_keys = candidates
        .iter()
        .map(|candidate| candidate.cache_key.as_str())
        .collect::<Vec<_>>();
    let hashes = CachedEntry::get_file_hash_many(
        &cache_keys,
        None,
        &state.pool,
        &state.api_semaphore,
    )
    .await?;
    let hashes_by_key = hashes
        .into_iter()
        .map(|hash| {
            (
                file_hash_cache_key(hash.size, hash.modified_at_ns, &hash.path),
                hash.hash,
            )
        })
        .collect::<HashMap<_, _>>();

    let mut delta = PackDelta::default();
    for candidate in candidates {
        let matches =
            hashes_by_key.get(&candidate.cache_key).is_some_and(|hash| {
                hash.eq_ignore_ascii_case(&candidate.expected_hash)
            });
        if matches {
            delta.kept_bytes += candidate.size;
            delta.kept.insert(candidate.relative_path);
        }
    }

    Ok(delta)
}
