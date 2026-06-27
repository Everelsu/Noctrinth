//! CurseForge-specific instance helpers.
//!
//! These back the fork's CurseForge integration: identifying already-installed
//! CurseForge files by fingerprint, and installing a CurseForge file straight
//! from its download mirrors. Kept in its own module so the upstream `instance`
//! API stays untouched and easy to re-sync.

use super::paths::get_full_path;
use crate::event::InstancePayloadType;
use crate::event::emit::emit_instance;
use crate::state::{ContentSourceKind, State};
use crate::util::fetch;

/// Compute CurseForge-style Murmur2 fingerprints for every CurseForge-
/// installable file in the instance.
///
/// Scans **all** content folders, not just `mods/`:
///   - `mods/` (.jar) — actual mods
///   - `resourcepacks/` (.zip) — resource packs
///   - `shaderpacks/` (.zip) — shader packs
///   - `datapacks/` (.zip) — globally-applied datapacks
///
/// CurseForge's `POST /v1/fingerprints/{gameId}` endpoint uses a Murmur2
/// (seed=1) over file bytes with whitespace (\t, \n, \r, space) stripped
/// first. The resulting `u32` uniquely identifies a CF file across all
/// projects — same trick xmcl-launcher uses.
///
/// Returns `(filename, fingerprint)` pairs. Files that can't be read are
/// silently skipped so a single bad zip doesn't break the whole scan.
#[tracing::instrument]
pub async fn compute_cf_fingerprints(
    path: &str,
) -> crate::Result<Vec<(String, u32)>> {
    use tokio::io::AsyncReadExt;

    let full = get_full_path(path).await?;

    // (folder, expected extension) — extension is matched case-insensitively.
    // .disabled is intentionally NOT supported; the user has disabled the
    // item explicitly and we don't want it counted as "installed".
    let scan_targets: &[(&str, &str)] = &[
        ("mods", "jar"),
        ("resourcepacks", "zip"),
        ("shaderpacks", "zip"),
        ("datapacks", "zip"),
    ];

    let mut out = Vec::new();
    for (folder, expected_ext) in scan_targets {
        let dir = full.join(folder);
        if !dir.is_dir() {
            continue;
        }
        let mut entries = match tokio::fs::read_dir(&dir).await {
            Ok(e) => e,
            Err(err) => {
                tracing::warn!(
                    "[cf-fingerprint] Could not read {}: {}",
                    dir.display(),
                    err
                );
                continue;
            }
        };
        while let Some(entry) = entries.next_entry().await? {
            let entry_path = entry.path();
            if !entry_path.is_file() {
                continue;
            }
            let ext = entry_path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if ext != *expected_ext {
                continue;
            }

            let filename = entry.file_name().to_string_lossy().into_owned();

            let mut file = match tokio::fs::File::open(&entry_path).await {
                Ok(f) => f,
                Err(err) => {
                    tracing::warn!(
                        "[cf-fingerprint] Could not open {}: {}",
                        entry_path.display(),
                        err
                    );
                    continue;
                }
            };
            let mut buf = Vec::with_capacity(1024 * 64);
            if let Err(err) = file.read_to_end(&mut buf).await {
                tracing::warn!(
                    "[cf-fingerprint] Could not read {}: {}",
                    entry_path.display(),
                    err
                );
                continue;
            }

            let fp = curseforge_fingerprint(&buf);
            out.push((filename, fp));
        }
    }

    tracing::debug!("[cf-fingerprint] {} hashed {} files", path, out.len());
    Ok(out)
}

/// CurseForge Murmur2 (seed=1, whitespace-stripped) fingerprint.
///
/// The whitespace strip is the canonical CF tweak: bytes 0x09 (tab), 0x0a
/// (LF), 0x0d (CR), and 0x20 (space) are removed before hashing, and the
/// `length` parameter inside the hash is the *stripped* length.
fn curseforge_fingerprint(data: &[u8]) -> u32 {
    let stripped: Vec<u8> = data
        .iter()
        .copied()
        .filter(|b| !matches!(b, 9 | 10 | 13 | 32))
        .collect();
    murmur2(&stripped, 1)
}

/// 32-bit MurmurHash2 (little-endian). Standard reference implementation.
fn murmur2(data: &[u8], seed: u32) -> u32 {
    const M: u32 = 0x5bd1e995;
    const R: u32 = 24;

    let mut h: u32 = seed ^ (data.len() as u32);
    let mut i = 0;
    while i + 4 <= data.len() {
        let mut k =
            u32::from_le_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]);
        k = k.wrapping_mul(M);
        k ^= k >> R;
        k = k.wrapping_mul(M);
        h = h.wrapping_mul(M);
        h ^= k;
        i += 4;
    }

    let tail = data.len() - i;
    if tail >= 3 {
        h ^= (data[i + 2] as u32) << 16;
    }
    if tail >= 2 {
        h ^= (data[i + 1] as u32) << 8;
    }
    if tail >= 1 {
        h ^= data[i] as u32;
        h = h.wrapping_mul(M);
    }

    h ^= h >> 13;
    h = h.wrapping_mul(M);
    h ^= h >> 15;
    h
}

/// Add a project from direct download URLs (e.g. a CurseForge file).
///
/// Unlike `add_project_from_version`, this does not resolve a Modrinth
/// version — it downloads the file from the first mirror that works and
/// stores it in the instance. When `sha1` is provided, the download is
/// integrity-checked and retried on mismatch.
///
/// Returns the relative path to the installed project.
#[tracing::instrument]
pub async fn add_project_from_curseforge(
    instance_id: &str,
    file_urls: &[String],
    file_name: &str,
    curseforge_project_id: i64,
    curseforge_file_id: i64,
    sha1: Option<&str>,
) -> crate::Result<String> {
    let state = State::get().await?;

    let mirrors: Vec<&str> = file_urls.iter().map(String::as_str).collect();
    let bytes = fetch::fetch_mirrors(
        &mirrors,
        sha1,
        None,
        None,
        &state.fetch_semaphore,
        &state.pool,
    )
    .await?;

    add_curseforge_bytes(
        instance_id,
        file_name,
        bytes,
        curseforge_project_id,
        curseforge_file_id,
        sha1,
    )
    .await
}

/// Record already-downloaded CurseForge file bytes as content in the instance.
///
/// Shared by the API-download path (`add_project_from_curseforge`) and the
/// manual WebView-download path (for files whose author disabled third-party
/// distribution). CurseForge has no Modrinth hash mapping, so the numeric
/// project/file ids are persisted in the content entry's project_id/version_id
/// columns (tagged by the CurseForge source kind) for later update checks.
pub async fn add_curseforge_bytes(
    instance_id: &str,
    file_name: &str,
    bytes: impl Into<bytes::Bytes>,
    curseforge_project_id: i64,
    curseforge_file_id: i64,
    sha1: Option<&str>,
) -> crate::Result<String> {
    let state = State::get().await?;

    let bytes = bytes.into();
    let cf_project_id = curseforge_project_id.to_string();
    let cf_file_id = curseforge_file_id.to_string();
    let project_path = crate::state::instances::commands::add_project_bytes(
        instance_id,
        file_name,
        bytes,
        sha1,
        None,
        ContentSourceKind::CurseForge,
        Some(&cf_project_id),
        Some(&cf_file_id),
        &state,
    )
    .await?;

    emit_instance(instance_id, InstancePayloadType::Edited).await?;

    Ok(project_path)
}

#[cfg(test)]
mod cf_fingerprint_tests {
    use super::{curseforge_fingerprint, murmur2};

    #[test]
    fn murmur2_empty_seed1() {
        assert_eq!(murmur2(&[], 1), 0x5BD1_5E36);
    }

    #[test]
    fn fingerprint_whitespace_only_equals_empty() {
        let only_ws: &[u8] = b"\t\n\r  \r\n\t  ";
        assert_eq!(curseforge_fingerprint(only_ws), curseforge_fingerprint(&[]));
        assert_eq!(curseforge_fingerprint(only_ws), 0x5BD1_5E36);
    }

    #[test]
    fn fingerprint_hello_world() {
        let stripped_ref = murmur2(b"HelloWorld!", 1);
        let with_space = curseforge_fingerprint(b"Hello World!");
        assert_eq!(with_space, stripped_ref);
    }
}
