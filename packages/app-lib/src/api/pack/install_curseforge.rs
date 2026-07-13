//! CurseForge modpack installer (install-job pipeline).
//!
//! A CurseForge modpack is a `.zip` containing a CurseForge-format
//! `manifest.json` (mods listed by `projectID`/`fileID`) and an `overrides/`
//! folder. Unlike the previous fork implementation, installs now run inside
//! the regular install-job pipeline (`crate::install`), which gives CurseForge
//! packs the same queueing, live progress, cancellation, retry, error
//! contexts, and rollback-on-failure behaviour as Modrinth packs:
//!   1. `read_curseforge_pack_meta` parses the manifest so the job (and the
//!      creation-modal preview) knows the pack name, Minecraft version and
//!      loader up-front,
//!   2. `install_curseforge_pack_files_with_reporter` resolves every mod file
//!      through the CurseForge API, downloads them concurrently (with a CDN
//!      fallback for author-restricted files), records each as
//!      `CurseForge`-sourced content, extracts `overrides/`, and installs
//!      Minecraft — reporting phase/progress through the job reporter.
//!
//! The pure zip-inspection helpers are also used by `install_from.rs` and the
//! job runner to route a dropped `.zip` to the right format.

use std::collections::HashSet;
use std::io::Cursor;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use async_zip::base::read::seek::ZipFileReader;
use futures::{StreamExt, TryStreamExt};
use path_util::SafeRelativeUtf8UnixPathBuf;
use reqwest::Method;
use serde::Deserialize;
use tokio::io::AsyncWriteExt;

use crate::install::{
    InstallErrorContext, InstallPhaseDetails, InstallPhaseId, InstallProgress,
    InstallProgressReporter,
};
use crate::state::{
    AppliedContentSetPatch, ContentSourceKind, EditInstance,
    InstanceInstallStage, InstanceLink, ModLoader, State,
};
use crate::util::fetch::{
    DownloadMeta, DownloadReason, fetch_advanced, fetch_mirrors,
};
use crate::util::io::sanitize_filename;

const CF_FILES_ENDPOINT: &str = "https://api.curseforge.com/v1/mods/files";

/// How many modpack content files are downloaded at the same time. Actual
/// network parallelism is additionally bounded by the global fetch semaphore.
const CF_CONTENT_DOWNLOAD_CONCURRENCY: usize = 8;

/// How many failed files are listed by name in the aggregate error message.
const CF_MAX_LISTED_FAILURES: usize = 8;

pub async fn zip_has_curseforge_manifest(pack_file: &bytes::Bytes) -> bool {
    zip_has_entry(pack_file, "manifest.json").await
}

pub async fn zip_has_modrinth_index(pack_file: &bytes::Bytes) -> bool {
    zip_has_entry(pack_file, "modrinth.index.json").await
}

async fn zip_has_entry(pack_file: &bytes::Bytes, name: &str) -> bool {
    match ZipFileReader::with_tokio(Cursor::new(pack_file)).await {
        Ok(zip) => zip
            .file()
            .entries()
            .iter()
            .any(|f| matches!(f.filename().as_str(), Ok(n) if n == name)),
        Err(_) => false,
    }
}

/// Check a zip on disk for an entry without loading the archive into memory.
/// Used by the job runner to route a local `.zip` between the CurseForge and
/// Modrinth installers while large `.mrpack` files stay file-backed.
pub async fn zip_file_has_entry(
    path: &std::path::Path,
    name: &str,
) -> crate::Result<bool> {
    let file = tokio::fs::File::open(path)
        .await
        .map_err(|e| crate::util::io::IOError::with_path(e, path))?;
    let mut reader = tokio::io::BufReader::new(file);
    match ZipFileReader::with_tokio(&mut reader).await {
        Ok(zip) => Ok(zip
            .file()
            .entries()
            .iter()
            .any(|f| matches!(f.filename().as_str(), Ok(n) if n == name))),
        Err(_) => Ok(false),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfManifest {
    minecraft: CfManifestMinecraft,
    name: String,
    /// Pack version, e.g. `1.4.2`. Optional in the wild.
    #[serde(default)]
    version: Option<String>,
    files: Vec<CfManifestFile>,
    /// Name of the overrides folder inside the zip. Defaults to `overrides`.
    #[serde(default)]
    overrides: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfManifestMinecraft {
    version: String,
    mod_loaders: Vec<CfManifestModLoader>,
}

#[derive(Deserialize)]
struct CfManifestModLoader {
    /// e.g. `fabric-0.16.5`, `forge-47.2.0`, `neoforge-21.1.0`.
    id: String,
    #[serde(default)]
    primary: bool,
}

#[derive(Deserialize)]
struct CfManifestFile {
    #[serde(rename = "projectID")]
    project_id: i64,
    #[serde(rename = "fileID")]
    file_id: i64,
}

#[derive(Deserialize)]
struct CfFilesResponse {
    data: Vec<CfFileData>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfFileData {
    id: i64,
    mod_id: i64,
    file_name: String,
    /// `None` when the author disabled third-party downloads.
    download_url: Option<String>,
    #[serde(default)]
    hashes: Vec<CfFileHash>,
}

#[derive(Deserialize)]
struct CfFileHash {
    value: String,
    /// 1 = SHA1, 2 = MD5.
    algo: i32,
}

impl CfFileData {
    fn sha1(&self) -> Option<&str> {
        self.hashes
            .iter()
            .find(|h| h.algo == 1)
            .map(|h| h.value.as_str())
    }

    /// Reconstruct the public CDN URL from the numeric file id — works even
    /// when the author disabled API distribution (which only nulls
    /// `download_url`). Same workaround MultiMC / Prism use.
    fn cdn_fallback_url(&self) -> Option<String> {
        if self.id < 1000 || self.file_name.is_empty() {
            return None;
        }
        let safe_name = self.file_name.replace(' ', "%20");
        Some(format!(
            "https://edge.forgecdn.net/files/{}/{}/{safe_name}",
            self.id / 1000,
            self.id % 1000
        ))
    }

    fn download_candidates(&self) -> Vec<String> {
        let mut urls = Vec::with_capacity(2);
        if let Some(url) = &self.download_url {
            urls.push(url.clone());
        }
        if let Some(url) = self.cdn_fallback_url()
            && !urls.contains(&url)
        {
            urls.push(url);
        }
        urls
    }
}

fn loader_from_manifest_id(id: &str) -> Option<(ModLoader, Option<String>)> {
    let (kind, version) = id.split_once('-')?;
    let loader = match kind {
        "forge" => ModLoader::Forge,
        "neoforge" => ModLoader::NeoForge,
        "fabric" => ModLoader::Fabric,
        "quilt" => ModLoader::Quilt,
        _ => return None,
    };
    Some((loader, Some(version.to_string())))
}

fn pick_loader(manifest: &CfManifest) -> (ModLoader, Option<String>) {
    manifest
        .minecraft
        .mod_loaders
        .iter()
        .find(|l| l.primary)
        .or_else(|| manifest.minecraft.mod_loaders.first())
        .and_then(|l| loader_from_manifest_id(&l.id))
        .unwrap_or((ModLoader::Vanilla, None))
}

async fn read_manifest(pack_file: &bytes::Bytes) -> crate::Result<CfManifest> {
    let mut zip_reader = ZipFileReader::with_tokio(Cursor::new(pack_file))
        .await
        .map_err(|_| {
            crate::Error::from(crate::ErrorKind::InputError(
                "Failed to read CurseForge modpack zip".to_string(),
            ))
        })?;

    let Some(manifest_idx) = zip_reader
        .file()
        .entries()
        .iter()
        .position(|f| matches!(f.filename().as_str(), Ok("manifest.json")))
    else {
        return Err(crate::ErrorKind::InputError(
            "No manifest.json found — not a CurseForge modpack".to_string(),
        )
        .into());
    };

    let mut manifest_str = String::new();
    {
        let mut reader = zip_reader.reader_with_entry(manifest_idx).await?;
        reader.read_to_string_checked(&mut manifest_str).await?;
    }
    Ok(serde_json::from_str(&manifest_str)?)
}

/// Pack metadata extracted from a CurseForge modpack zip — used by the
/// creation-modal preview and the job runner so the instance is created with
/// the pack's real name, Minecraft version and loader instead of placeholders.
pub struct CurseforgePackMeta {
    pub name: String,
    pub version: Option<String>,
    pub game_version: String,
    pub loader: ModLoader,
    pub loader_version: Option<String>,
}

pub async fn read_curseforge_pack_meta(
    pack_file: &bytes::Bytes,
) -> crate::Result<CurseforgePackMeta> {
    let manifest = read_manifest(pack_file).await?;
    let (loader, loader_version) = pick_loader(&manifest);
    Ok(CurseforgePackMeta {
        name: manifest.name,
        version: manifest.version,
        game_version: manifest.minecraft.version,
        loader,
        loader_version,
    })
}

/// Install CurseForge modpack contents into an already-created instance,
/// reporting phases and progress through the install-job reporter.
///
/// The full sequence: read the manifest, apply the pack's name / version /
/// loader to the instance, resolve every file through the CurseForge API,
/// download all files concurrently (aggregating failures into one error
/// instead of dying on the first), extract `overrides/`, and install
/// Minecraft. The job runner rolls the instance back if any step fails.
pub(crate) async fn install_curseforge_pack_files_with_reporter(
    pack_file: bytes::Bytes,
    curseforge_api_key: &str,
    instance_id: &str,
    reason: DownloadReason,
    reporter: InstallProgressReporter,
) -> crate::Result<String> {
    let state = State::get().await?;

    reporter
        .set_context(
            InstallErrorContext::new("read modpack manifest")
                .entry_path("manifest.json")
                .build(),
        )
        .await?;
    reporter
        .update(
            InstallPhaseId::ReadingPackManifest,
            None,
            InstallPhaseDetails::Empty,
        )
        .await?;
    let manifest = read_manifest(&pack_file).await?;
    let details = InstallPhaseDetails::Modpack {
        project_id: None,
        version_id: None,
        title: Some(manifest.name.clone()),
    };

    apply_manifest_to_instance(instance_id, &manifest).await?;

    // Resolve every file through the CurseForge API (one bulk request, then
    // individual re-fetches for any the bulk endpoint omits/dedups).
    reporter
        .update(InstallPhaseId::ResolvingPack, None, details.clone())
        .await?;
    reporter
        .set_context(
            InstallErrorContext::new("resolve CurseForge modpack files")
                .urls(vec![CF_FILES_ENDPOINT.to_string()])
                .build(),
        )
        .await?;
    let resolved =
        resolve_manifest_files(&state, &manifest, curseforge_api_key).await?;

    // Download every resolved mod file concurrently and record each as
    // CurseForge-sourced content. Failures are collected and reported
    // together so the user learns about ALL broken files at once.
    let total = resolved.len() as u64;
    reporter
        .update(
            InstallPhaseId::DownloadingContent,
            Some(InstallProgress {
                current: 0,
                total,
                secondary: None,
            }),
            details.clone(),
        )
        .await?;

    let download_meta = DownloadMeta {
        reason,
        game_version: manifest.minecraft.version.clone(),
        loader: pick_loader(&manifest).0.as_str().to_string(),
        dependent_on: None,
    };
    let completed = Arc::new(AtomicU64::new(0));
    let failures = Arc::new(tokio::sync::Mutex::new(Vec::<String>::new()));

    futures::stream::iter(resolved)
        .map(Ok::<CfFileData, crate::Error>)
        .map_ok(|cf_file| {
            let state = state.clone();
            let reporter = reporter.clone();
            let details = details.clone();
            let completed = completed.clone();
            let failures = failures.clone();
            let download_meta = download_meta.clone();
            let instance_id = instance_id.to_string();
            async move {
                if let Err(err) = download_cf_file(
                    &state,
                    &cf_file,
                    &download_meta,
                    &instance_id,
                )
                .await
                {
                    failures
                        .lock()
                        .await
                        .push(format!("{}: {}", cf_file.file_name, err));
                }
                let done = completed.fetch_add(1, Ordering::Relaxed) + 1;
                reporter
                    .update(
                        InstallPhaseId::DownloadingContent,
                        Some(InstallProgress {
                            current: done,
                            total,
                            secondary: None,
                        }),
                        details.clone(),
                    )
                    .await?;
                Ok::<(), crate::Error>(())
            }
        })
        .try_buffer_unordered(CF_CONTENT_DOWNLOAD_CONCURRENCY)
        .try_collect::<Vec<_>>()
        .await?;

    let failures = Arc::try_unwrap(failures)
        .map(|m| m.into_inner())
        .unwrap_or_default();
    if !failures.is_empty() {
        let listed = failures
            .iter()
            .take(CF_MAX_LISTED_FAILURES)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");
        let suffix = if failures.len() > CF_MAX_LISTED_FAILURES {
            format!("\n… and {} more", failures.len() - CF_MAX_LISTED_FAILURES)
        } else {
            String::new()
        };
        return Err(crate::ErrorKind::InputError(format!(
            "Failed to download {} of {} modpack files:\n{listed}{suffix}",
            failures.len(),
            total,
        ))
        .into());
    }

    extract_overrides(&pack_file, &manifest, instance_id, &reporter, &details)
        .await?;

    crate::launcher::install_minecraft_for_instance_id_with_reporter(
        instance_id,
        false,
        Some(reporter.clone()),
    )
    .await?;
    reporter.clear_context().await?;

    Ok(instance_id.to_string())
}

/// Apply the pack's name, version tag, Minecraft version, and loader to the
/// instance — the CurseForge analogue of `set_instance_information`.
async fn apply_manifest_to_instance(
    instance_id: &str,
    manifest: &CfManifest,
) -> crate::Result<()> {
    let (loader, manifest_loader_version) = pick_loader(manifest);
    let loader_version = if loader != ModLoader::Vanilla {
        crate::launcher::get_loader_version_from_profile(
            &manifest.minecraft.version,
            loader,
            manifest_loader_version.as_deref(),
        )
        .await?
    } else {
        None
    };

    crate::api::instance::edit(
        instance_id,
        EditInstance {
            install_stage: Some(InstanceInstallStage::PackInstalling),
            name: Some(manifest.name.clone()),
            link: Some(InstanceLink::ImportedModpack {
                project_id: None,
                version_id: None,
                name: Some(manifest.name.clone()),
                version_number: manifest.version.clone(),
                filename: None,
            }),
            content_set_patch: Some(AppliedContentSetPatch {
                source_kind: Some(ContentSourceKind::ImportedModpack),
                game_version: Some(manifest.minecraft.version.clone()),
                protocol_version: Some(None),
                loader: Some(loader),
                loader_version: Some(loader_version.map(|x| x.id)),
            }),
            ..EditInstance::default()
        },
    )
    .await?;
    Ok(())
}

async fn resolve_manifest_files(
    state: &State,
    manifest: &CfManifest,
    curseforge_api_key: &str,
) -> crate::Result<Vec<CfFileData>> {
    let file_ids: Vec<i64> = manifest.files.iter().map(|f| f.file_id).collect();
    if file_ids.is_empty() {
        return Ok(Vec::new());
    }

    let mut resolved: CfFilesResponse = {
        let body = fetch_advanced(
            Method::POST,
            CF_FILES_ENDPOINT,
            None,
            Some(serde_json::json!({ "fileIds": file_ids })),
            Some(("x-api-key", curseforge_api_key)),
            None,
            None,
            None,
            &state.fetch_semaphore,
            &state.pool,
        )
        .await?;
        serde_json::from_slice(&body)?
    };

    // The bulk endpoint silently drops some ids (and de-dups) — re-fetch the
    // missing ones individually, the same strategy XMCL uses.
    let returned: HashSet<i64> = resolved.data.iter().map(|f| f.id).collect();
    let missing: Vec<&CfManifestFile> = manifest
        .files
        .iter()
        .filter(|f| !returned.contains(&f.file_id))
        .collect();
    for file in missing {
        #[derive(Deserialize)]
        struct CfSingleFileResponse {
            data: CfFileData,
        }
        let body = fetch_advanced(
            Method::GET,
            &format!(
                "https://api.curseforge.com/v1/mods/{}/files/{}",
                file.project_id, file.file_id
            ),
            None,
            None,
            Some(("x-api-key", curseforge_api_key)),
            None,
            None,
            None,
            &state.fetch_semaphore,
            &state.pool,
        )
        .await?;
        let single: CfSingleFileResponse = serde_json::from_slice(&body)?;
        resolved.data.push(single.data);
    }

    Ok(resolved.data)
}

async fn download_cf_file(
    state: &State,
    cf_file: &CfFileData,
    download_meta: &DownloadMeta,
    instance_id: &str,
) -> crate::Result<()> {
    let candidates = cf_file.download_candidates();
    if candidates.is_empty() {
        return Err(crate::ErrorKind::InputError(format!(
            "no download URL could be determined (file id {})",
            cf_file.id
        ))
        .into());
    }
    let mirrors: Vec<&str> = candidates.iter().map(String::as_str).collect();
    let bytes = fetch_mirrors(
        &mirrors,
        cf_file.sha1(),
        Some(download_meta),
        None,
        &state.fetch_semaphore,
        &state.pool,
    )
    .await?;

    let safe_file_name = sanitize_filename(&cf_file.file_name);
    let cf_project_id = cf_file.mod_id.to_string();
    let cf_file_id = cf_file.id.to_string();
    // Serialize the DB-and-filesystem bookkeeping while the downloads above
    // still run in parallel — mirrors the mrpack installer's use of the
    // install DB semaphore.
    let _permit = state.install_db_semaphore.acquire().await?;
    crate::state::instances::commands::add_project_bytes(
        instance_id,
        &safe_file_name,
        bytes,
        cf_file.sha1(),
        None,
        ContentSourceKind::CurseForge,
        Some(&cf_project_id),
        Some(&cf_file_id),
        state,
    )
    .await?;
    Ok(())
}

/// Extract the modpack's overrides folder into the instance directory,
/// reporting extraction progress through the job reporter.
async fn extract_overrides(
    pack_file: &bytes::Bytes,
    manifest: &CfManifest,
    instance_id: &str,
    reporter: &InstallProgressReporter,
    details: &InstallPhaseDetails,
) -> crate::Result<()> {
    let instance_dir =
        crate::api::instance::get_full_path(instance_id).await?;
    let mut zip_reader = ZipFileReader::with_tokio(Cursor::new(pack_file))
        .await
        .map_err(|_| {
            crate::Error::from(crate::ErrorKind::InputError(
                "Failed to read CurseForge modpack zip".to_string(),
            ))
        })?;

    // The manifest can name a custom overrides folder; `overrides` and
    // `client-overrides` are always accepted as well.
    let mut override_prefixes = vec![
        "overrides".to_string(),
        "client-overrides".to_string(),
    ];
    if let Some(custom) = manifest
        .overrides
        .as_deref()
        .map(|s| s.trim_matches('/').to_string())
        .filter(|s| !s.is_empty() && !override_prefixes.contains(s))
    {
        override_prefixes.push(custom);
    }
    let matches_prefix = |name: &str| {
        override_prefixes
            .iter()
            .any(|p| name.starts_with(&format!("{p}/")))
    };

    let override_entries = zip_reader
        .file()
        .entries()
        .iter()
        .enumerate()
        .filter_map(|(index, file)| {
            let filename = file.filename().as_str().unwrap_or_default();
            (matches_prefix(filename) && !filename.ends_with('/'))
                .then_some(index)
        })
        .collect::<Vec<_>>();

    let total = override_entries.len() as u64;
    reporter
        .update(
            InstallPhaseId::ExtractingOverrides,
            Some(InstallProgress {
                current: 0,
                total,
                secondary: None,
            }),
            details.clone(),
        )
        .await?;

    for (done, index) in override_entries.into_iter().enumerate() {
        let entry_name = {
            let entries = zip_reader.file().entries();
            entries[index]
                .filename()
                .as_str()
                .unwrap_or_default()
                .to_string()
        };
        let raw_path = SafeRelativeUtf8UnixPathBuf::try_from(entry_name)?;
        let relative_path = override_prefixes
            .iter()
            .find_map(|p| raw_path.strip_prefix(p).ok())
            .ok_or_else(|| {
                crate::Error::from(crate::ErrorKind::OtherError(
                    "Failed to strip override prefix".to_string(),
                ))
            })?;
        let safe_relative_path =
            crate::util::io::sanitize_relative_path(relative_path.as_str());

        let mut file_bytes = vec![];
        {
            let mut reader = zip_reader.reader_with_entry(index).await?;
            reader.read_to_end_checked(&mut file_bytes).await?;
        }

        let dest = instance_dir.join(&safe_relative_path);
        if let Some(parent) = dest.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                crate::util::io::IOError::with_path(e, parent)
            })?;
        }
        let mut out = tokio::fs::File::create(&dest)
            .await
            .map_err(|e| crate::util::io::IOError::with_path(e, &dest))?;
        out.write_all(&file_bytes)
            .await
            .map_err(|e| crate::util::io::IOError::with_path(e, &dest))?;

        reporter
            .update(
                InstallPhaseId::ExtractingOverrides,
                Some(InstallProgress {
                    current: done as u64 + 1,
                    total,
                    secondary: None,
                }),
                details.clone(),
            )
            .await?;
    }

    Ok(())
}
