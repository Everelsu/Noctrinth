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
//!      through the CurseForge API, extracts bundled content, reuses verified
//!      files and downloads only missing ones, records their CurseForge source,
//!      and installs Minecraft — reporting phase/progress through the job reporter.
//!      Files whose author does not allow launcher downloads are left to
//!      `curseforge_blocked`, which looks for them on disk and otherwise
//!      leaves them for the user to download by hand.
//!
//! The pure zip-inspection helpers are also used by `install_from.rs` and the
//! job runner to route a dropped `.zip` to the right format.

use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::io::Cursor;
use std::path::PathBuf;
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
    /// Noctrinth's own: where the pack's picture lives, when the manifest says
    /// so at all. CurseForge's own format has no such field, but the exporters
    /// people actually use write one — as a URL on CurseForge's avatar CDN,
    /// which is the only copy of the picture a pack that ships no image file
    /// has anywhere.
    #[serde(
        default,
        alias = "iconUrl",
        alias = "thumbnailUrl",
        alias = "logo"
    )]
    image: Option<String>,
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
    file_length: Option<u64>,
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

    fn matches_bytes(&self, bytes: &[u8]) -> bool {
        if self
            .file_length
            .is_some_and(|length| length > 0 && length != bytes.len() as u64)
        {
            return false;
        }
        if let Some(expected) = self.sha1() {
            return expected.eq_ignore_ascii_case(
                &sha1_smol::Sha1::from(bytes).hexdigest(),
            );
        }
        if let Some(hash) = self.hashes.iter().find(|hash| hash.algo == 2) {
            use md5::Digest;
            return hash.value.eq_ignore_ascii_case(&format!(
                "{:x}",
                md5::Md5::digest(bytes)
            ));
        }
        false
    }

    /// The file as one its author does not let launchers download, to be
    /// looked for on disk instead.
    fn as_blocked(&self) -> crate::api::curseforge_blocked::BlockedFile {
        crate::api::curseforge_blocked::BlockedFile {
            project_id: self.mod_id,
            file_id: self.id,
            file_name: self.file_name.clone(),
            size: self.file_length.filter(|length| *length > 0),
            sha1: self.sha1().map(str::to_string),
            source_kind: ContentSourceKind::CurseForgeModpack,
            disabled: false,
        }
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
/// extract bundled files, reuse verified content or download missing files
/// concurrently (aggregating failures into one error), and install
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
    let bundled = Arc::new(
        extract_overrides(
            &pack_file,
            &manifest,
            instance_id,
            &reporter,
            &details,
        )
        .await?,
    );

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
    let blocked = Arc::new(tokio::sync::Mutex::new(Vec::<CfFileData>::new()));
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
            let blocked = blocked.clone();
            let bundled = bundled.clone();
            let download_meta = download_meta.clone();
            let instance_id = instance_id.to_string();
            async move {
                match install_cf_file(
                    &state,
                    &cf_file,
                    &download_meta,
                    &instance_id,
                    &bundled,
                )
                .await
                {
                    Ok(true) => {}
                    Ok(false) => blocked.lock().await.push(cf_file),
                    Err(err) => failures
                        .lock()
                        .await
                        .push(format!("{}: {}", cf_file.file_name, err)),
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

    // Noctrinth's own: the files CurseForge will not hand a launcher are
    // looked for on disk, and whatever is not there waits for the user. The
    // pack still installs; it is the content page that says what is missing.
    let blocked = Arc::try_unwrap(blocked)
        .map(|mutex| mutex.into_inner())
        .unwrap_or_default();
    if !blocked.is_empty() {
        let pending = crate::api::curseforge_blocked::settle(
            instance_id,
            blocked.iter().map(CfFileData::as_blocked).collect(),
            &state,
        )
        .await;
        if !pending.is_empty() {
            tracing::info!(
                "{} file(s) of {} have to be downloaded by hand",
                pending.len(),
                manifest.name
            );
        }
    }

    // Noctrinth's own: a manifest has no icon field, so a pack's instance used
    // to come out blank even when the zip was carrying the pack's picture all
    // along. Never worth failing an install over.
    if let Err(error) =
        apply_pack_icon(&pack_file, &manifest, instance_id, &state).await
    {
        tracing::warn!("Could not read the pack's own icon: {error}");
    }

    crate::launcher::install_minecraft_for_instance_id_with_reporter(
        instance_id,
        false,
        Some(reporter.clone()),
        &state,
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
        let state = crate::State::get().await?;
        crate::launcher::get_loader_version_from_profile(
            &manifest.minecraft.version,
            loader,
            manifest_loader_version.as_deref(),
            &state.directories,
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

async fn cf_file_bytes<'a, F, Fut>(
    file: &'a CfFileData,
    bundled: &[PathBuf],
    download: F,
) -> crate::Result<Option<bytes::Bytes>>
where
    F: FnOnce(&'a str) -> Fut,
    Fut: Future<Output = crate::Result<bytes::Bytes>>,
{
    for path in bundled {
        if let Some(length) = file.file_length.filter(|length| *length > 0)
            && !tokio::fs::metadata(path)
                .await
                .is_ok_and(|meta| meta.len() == length)
        {
            continue;
        }
        match tokio::fs::read(path).await {
            Ok(bytes) if file.matches_bytes(&bytes) => {
                tracing::info!(
                    "Reusing bundled CurseForge file {} from {}",
                    file.id,
                    path.display()
                );
                return Ok(Some(bytes.into()));
            }
            Ok(_) => {}
            Err(error) => tracing::warn!(
                "Could not read bundled file {}: {error}",
                path.display()
            ),
        }
    }
    let Some(url) = file.download_url.as_deref() else {
        return Ok(None);
    };
    let bytes = download(url).await?;
    if file
        .file_length
        .is_some_and(|length| length > 0 && length != bytes.len() as u64)
        || (file.hashes.iter().any(|hash| matches!(hash.algo, 1 | 2))
            && !file.matches_bytes(&bytes))
    {
        return Err(crate::ErrorKind::InputError(format!(
            "CurseForge file {} failed its size or checksum check",
            file.id
        ))
        .into());
    }
    Ok(Some(bytes))
}

async fn install_cf_file(
    state: &State,
    cf_file: &CfFileData,
    download_meta: &DownloadMeta,
    instance_id: &str,
    bundled: &HashMap<String, Vec<PathBuf>>,
) -> crate::Result<bool> {
    let safe_file_name = sanitize_filename(&cf_file.file_name);
    let sha1 = cf_file.sha1().map(str::to_ascii_lowercase);
    let expected_hash = sha1.as_deref();
    let Some(bytes) = cf_file_bytes(
        cf_file,
        bundled
            .get(&safe_file_name)
            .map(Vec::as_slice)
            .unwrap_or_default(),
        |url| async move {
            fetch_mirrors(
                &[url],
                expected_hash,
                Some(download_meta),
                None,
                &state.fetch_semaphore,
                &state.pool,
            )
            .await
        },
    )
    .await?
    else {
        return Ok(false);
    };

    let cf_project_id = cf_file.mod_id.to_string();
    let cf_file_id = cf_file.id.to_string();
    // Serialize the DB-and-filesystem bookkeeping while the downloads above
    // still run in parallel — mirrors the mrpack installer's use of the
    // install DB semaphore.
    let _permit = state.installs.db_semaphore.acquire().await?;
    crate::state::instances::commands::add_project_bytes(
        instance_id,
        &safe_file_name,
        bytes,
        expected_hash,
        None,
        // The pack's own, as against a CurseForge mod the user adds later —
        // which is what tells the pack's content list from everything beside
        // it. Both carry CurseForge ids; only this one is part of the pack.
        ContentSourceKind::CurseForgeModpack,
        Some(&cf_project_id),
        Some(&cf_file_id),
        state,
    )
    .await?;
    Ok(true)
}

/// Noctrinth's own: names a CurseForge pack ships its own picture under, in
/// the order they are preferred.
const PACK_ICON_STEMS: [&str; 5] =
    ["icon", "pack", "modpack-icon", "logo", "thumbnail"];

/// Extensions accepted for those, matched case-insensitively. SVG is left out
/// deliberately: instance icons are rasterised, and `cache_icon` refuses it.
const PACK_ICON_EXTENSIONS: [&str; 6] =
    ["png", "jpg", "jpeg", "webp", "gif", "bmp"];

/// How well a zip entry does as the pack's picture — lower is better, `None`
/// means it is not one.
///
/// Only the zip's root and the root of a folder inside it are considered: a
/// pack's own picture sits beside `manifest.json` or at the top of
/// `overrides/`, while `overrides/config/.../logo.png` belongs to a mod and
/// has nothing to do with the pack.
fn pack_icon_rank(entry_name: &str) -> Option<(usize, usize, usize)> {
    let depth = entry_name.matches('/').count();
    if depth > 1 {
        return None;
    }

    let file_name = entry_name.rsplit('/').next()?;
    let (stem, extension) = file_name.rsplit_once('.')?;
    let stem_rank = PACK_ICON_STEMS
        .iter()
        .position(|candidate| stem.eq_ignore_ascii_case(candidate))?;
    let extension_rank = PACK_ICON_EXTENSIONS
        .iter()
        .position(|candidate| extension.eq_ignore_ascii_case(candidate))?;

    Some((depth, stem_rank, extension_rank))
}

/// The picture a pack ships inside its own zip, if it ships one.
async fn pack_icon_from_zip(
    pack_file: &bytes::Bytes,
) -> crate::Result<Option<Vec<u8>>> {
    let mut zip_reader = ZipFileReader::with_tokio(Cursor::new(pack_file))
        .await
        .map_err(|_| {
            crate::Error::from(crate::ErrorKind::InputError(
                "Failed to read CurseForge modpack zip".to_string(),
            ))
        })?;

    let Some((_, index)) = zip_reader
        .file()
        .entries()
        .iter()
        .enumerate()
        .filter_map(|(index, file)| {
            let name = file.filename().as_str().ok()?;
            Some((pack_icon_rank(name)?, index))
        })
        .min()
    else {
        return Ok(None);
    };

    let mut icon_bytes = Vec::new();
    {
        let mut reader = zip_reader.reader_with_entry(index).await?;
        reader.read_to_end_checked(&mut icon_bytes).await?;
    }

    Ok(Some(icon_bytes))
}

/// Gives the instance the pack's picture, if the pack has one and the instance
/// has not been given one already — a reinstall over an instance the user has
/// since chosen an icon for leaves that choice alone.
///
/// A pack carries its picture one of two ways, and most carry it only the
/// second: as an image file in the zip, or as a URL in the manifest pointing
/// at CurseForge's avatar CDN. A zip holding several hundred mods and no
/// picture at all is the common case, so the URL is not a fallback worth
/// skipping — without it such a pack installs blank.
async fn apply_pack_icon(
    pack_file: &bytes::Bytes,
    manifest: &CfManifest,
    instance_id: &str,
    state: &State,
) -> crate::Result<()> {
    let already_has_icon = crate::api::instance::get(instance_id)
        .await?
        .is_some_and(|metadata| metadata.instance.icon_path.is_some());
    if already_has_icon {
        return Ok(());
    }

    let icon_bytes = match pack_icon_from_zip(pack_file).await? {
        Some(bytes) => bytes::Bytes::from(bytes),
        None => {
            // Fetched only over TLS, and only from the manifest's own field —
            // the same footing as the mod download URLs beside it.
            let Some(url) = manifest
                .image
                .as_deref()
                .map(str::trim)
                .filter(|url| url.starts_with("https://"))
            else {
                return Ok(());
            };
            fetch_advanced(
                Method::GET,
                url,
                None,
                None,
                None,
                None,
                None,
                None,
                &state.fetch_semaphore,
                &state.pool,
            )
            .await?
        }
    };

    let icon_path = crate::api::instance::cache_icon(icon_bytes, state).await?;
    crate::api::instance::edit(
        instance_id,
        EditInstance {
            icon_path: Some(Some(icon_path.to_string_lossy().to_string())),
            ..EditInstance::default()
        },
    )
    .await?;

    Ok(())
}

fn pack_entry_path(
    name: &str,
    prefixes: &[String],
) -> crate::Result<Option<String>> {
    if name.ends_with('/') {
        return Ok(None);
    }
    let relative = prefixes
        .iter()
        .find_map(|prefix| name.strip_prefix(&format!("{prefix}/")))
        .or_else(|| {
            crate::state::content_store::is_managed_content_path(name)
                .then_some(name)
        });
    let Some(relative) = relative else {
        return Ok(None);
    };
    SafeRelativeUtf8UnixPathBuf::try_from(name.to_string())?;
    SafeRelativeUtf8UnixPathBuf::try_from(relative.to_string())?;
    Ok(Some(crate::util::io::sanitize_relative_path(relative)))
}

/// Extract overrides and root content into the instance directory, returning
/// candidate files for verified reuse while reporting preparation progress.
async fn extract_overrides(
    pack_file: &bytes::Bytes,
    manifest: &CfManifest,
    instance_id: &str,
    reporter: &InstallProgressReporter,
    details: &InstallPhaseDetails,
) -> crate::Result<HashMap<String, Vec<PathBuf>>> {
    let instance_dir = crate::api::instance::get_full_path(instance_id).await?;
    let mut zip_reader = ZipFileReader::with_tokio(Cursor::new(pack_file))
        .await
        .map_err(|_| {
            crate::Error::from(crate::ErrorKind::InputError(
                "Failed to read CurseForge modpack zip".to_string(),
            ))
        })?;

    // The manifest can name a custom overrides folder; `overrides` and
    // `client-overrides` are always accepted as well.
    let mut override_prefixes =
        vec!["overrides".to_string(), "client-overrides".to_string()];
    if let Some(custom) = manifest
        .overrides
        .as_deref()
        .map(|s| s.trim_matches('/').to_string())
        .filter(|s| !s.is_empty() && !override_prefixes.contains(s))
    {
        override_prefixes.push(custom);
    }
    let mut override_entries = Vec::new();
    for (index, file) in zip_reader.file().entries().iter().enumerate() {
        if let Some(relative) = pack_entry_path(
            file.filename().as_str().unwrap_or_default(),
            &override_prefixes,
        )? {
            override_entries.push((index, relative));
        }
    }
    let mut bundled: HashMap<String, Vec<PathBuf>> = HashMap::new();

    // Bundled content is prepared before the download phase starts.
    let total = override_entries.len() as u64;
    reporter
        .update(
            InstallPhaseId::ReadingPackManifest,
            Some(InstallProgress {
                current: 0,
                total,
                secondary: None,
            }),
            details.clone(),
        )
        .await?;

    for (done, (index, safe_relative_path)) in
        override_entries.into_iter().enumerate()
    {
        let mut file_bytes = vec![];
        {
            let mut reader = zip_reader.reader_with_entry(index).await?;
            reader.read_to_end_checked(&mut file_bytes).await?;
        }

        let dest = instance_dir.join(&safe_relative_path);
        if let Some(parent) = dest.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| crate::util::io::IOError::with_path(e, parent))?;
        }
        let mut out = tokio::fs::File::create(&dest)
            .await
            .map_err(|e| crate::util::io::IOError::with_path(e, &dest))?;
        out.write_all(&file_bytes)
            .await
            .map_err(|e| crate::util::io::IOError::with_path(e, &dest))?;

        out.flush()
            .await
            .map_err(|e| crate::util::io::IOError::with_path(e, &dest))?;
        drop(out);

        if crate::state::content_store::is_managed_content_path(
            &safe_relative_path,
        ) {
            let file_name = dest
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default()
                .to_string();
            let paths = bundled.entry(file_name).or_default();
            if !paths.contains(&dest) {
                paths.push(dest);
            }
        }

        reporter
            .update(
                InstallPhaseId::ReadingPackManifest,
                Some(InstallProgress {
                    current: done as u64 + 1,
                    total,
                    secondary: None,
                }),
                details.clone(),
            )
            .await?;
    }

    Ok(bundled)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mod_file(bytes: &[u8]) -> CfFileData {
        CfFileData {
            id: 123,
            mod_id: 456,
            file_name: "mod.jar".into(),
            download_url: Some("https://example.invalid/mod.jar".into()),
            file_length: Some(bytes.len() as u64),
            hashes: vec![CfFileHash {
                algo: 1,
                value: sha1_smol::Sha1::from(bytes).hexdigest(),
            }],
        }
    }

    #[tokio::test]
    async fn bundled_mod_skips_the_download() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("mod.jar");
        tokio::fs::write(&path, b"correct mod").await.unwrap();
        let mut file = mod_file(b"correct mod");
        file.hashes[0].value.make_ascii_uppercase();
        let bytes = cf_file_bytes(&file, &[path], |_| async {
            panic!("bundled mod must not be downloaded")
        })
        .await
        .unwrap()
        .unwrap();
        assert_eq!(bytes.as_ref(), b"correct mod");
    }

    #[tokio::test]
    async fn same_name_with_wrong_contents_or_size_is_downloaded() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("mod.jar");
        let file = mod_file(b"correct mod");
        for contents in [b"invalid mod".as_slice(), b"short".as_slice()] {
            tokio::fs::write(&path, contents).await.unwrap();
            let downloaded = std::sync::atomic::AtomicBool::new(false);
            let bytes =
                cf_file_bytes(&file, std::slice::from_ref(&path), |url| {
                    assert_eq!(url, "https://example.invalid/mod.jar");
                    downloaded.store(true, Ordering::Relaxed);
                    std::future::ready(Ok(bytes::Bytes::from_static(
                        b"correct mod",
                    )))
                })
                .await
                .unwrap()
                .unwrap();
            assert!(downloaded.load(Ordering::Relaxed));
            assert_eq!(bytes.as_ref(), b"correct mod");
        }
    }

    #[tokio::test]
    async fn unavailable_download_is_satisfied_only_by_verified_local_bytes() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("mod.jar");
        tokio::fs::write(&path, b"correct mod").await.unwrap();
        let mut file = mod_file(b"correct mod");
        file.download_url = None;
        assert!(
            cf_file_bytes(&file, &[path], |_| async {
                panic!("restricted file must not be fetched")
            })
            .await
            .unwrap()
            .is_some()
        );
        assert!(
            cf_file_bytes(&file, &[], |_| async {
                panic!("restricted file must remain pending")
            })
            .await
            .unwrap()
            .is_none()
        );
    }

    #[tokio::test]
    async fn unverified_or_missing_local_files_fall_back_to_download() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("mod.jar");
        let mut file = mod_file(b"correct mod");
        let downloaded = std::sync::atomic::AtomicU64::new(0);
        for write in [false, true] {
            if write {
                tokio::fs::write(&path, b"correct mod").await.unwrap();
                file.hashes.clear();
            }
            cf_file_bytes(&file, std::slice::from_ref(&path), |_| {
                downloaded.fetch_add(1, Ordering::Relaxed);
                std::future::ready(Ok(bytes::Bytes::from_static(
                    b"correct mod",
                )))
            })
            .await
            .unwrap()
            .unwrap();
        }
        assert_eq!(downloaded.load(Ordering::Relaxed), 2);
    }

    #[tokio::test]
    async fn another_bundled_copy_can_satisfy_the_file() {
        let directory = tempfile::tempdir().unwrap();
        let bad = directory.path().join("bad.jar");
        let good = directory.path().join("good.jar");
        tokio::fs::write(&bad, b"invalid mod").await.unwrap();
        tokio::fs::write(&good, b"correct mod").await.unwrap();
        assert!(
            cf_file_bytes(&mod_file(b"correct mod"), &[bad, good], |_| async {
                panic!("valid bundled copy exists")
            })
            .await
            .unwrap()
            .is_some()
        );
    }

    #[test]
    fn md5_is_supported_but_cannot_override_a_mismatched_sha1() {
        use md5::Digest;
        let mut file = mod_file(b"correct mod");
        let md5 = CfFileHash {
            algo: 2,
            value: format!("{:x}", md5::Md5::digest(b"correct mod")),
        };
        file.hashes = vec![md5];
        assert!(file.matches_bytes(b"correct mod"));
        assert!(!file.matches_bytes(b"invalid mod"));
        file.hashes.push(CfFileHash {
            algo: 1,
            value: "0".repeat(40),
        });
        assert!(!file.matches_bytes(b"correct mod"));
    }

    #[tokio::test]
    async fn corrupt_download_is_rejected() {
        assert!(
            cf_file_bytes(&mod_file(b"correct mod"), &[], |_| async {
                Ok(bytes::Bytes::from_static(b"invalid mod"))
            })
            .await
            .is_err()
        );
    }

    #[test]
    fn extraction_accepts_pack_content_and_rejects_escape_paths() {
        let prefixes = vec![
            "overrides".into(),
            "client-overrides".into(),
            "custom/files".into(),
        ];
        for (source, target) in [
            ("overrides/mods/mod.jar", "mods/mod.jar"),
            ("client-overrides/mods/mod.jar", "mods/mod.jar"),
            ("custom/files/config/options.txt", "config/options.txt"),
            ("mods/mod.jar", "mods/mod.jar"),
            ("resourcepacks/pack.zip", "resourcepacks/pack.zip"),
            ("overrides/config/options.txt", "config/options.txt"),
        ] {
            assert_eq!(
                pack_entry_path(source, &prefixes).unwrap().as_deref(),
                Some(target)
            );
        }
        for source in [
            "manifest.json",
            "unrelated/mod.jar",
            "override/mod.jar",
            "overrides/",
            "mods/nested/mod.jar",
        ] {
            assert!(pack_entry_path(source, &prefixes).unwrap().is_none());
        }
        assert!(
            pack_entry_path("overrides/../../escape.jar", &prefixes).is_err()
        );
        assert!(pack_entry_path("overrides/C:/escape.jar", &prefixes).is_err());
    }

    #[test]
    fn a_pack_icon_is_only_looked_for_where_a_pack_puts_one() {
        assert!(pack_icon_rank("icon.png").is_some());
        assert!(pack_icon_rank("overrides/pack.png").is_some());
        // A mod's own artwork, buried in the config it came with.
        assert!(pack_icon_rank("overrides/config/mod/logo.png").is_none());
        assert!(pack_icon_rank("manifest.json").is_none());
        assert!(pack_icon_rank("overrides/icon.svg").is_none());
    }

    #[test]
    fn a_manifest_can_name_the_pack_s_picture() {
        // Shape taken from a real export: no icon file anywhere in the zip,
        // the picture only reachable through the manifest's own field.
        let manifest: CfManifest = serde_json::from_str(
            r#"{
                "minecraft": {
                    "version": "1.20.1",
                    "modLoaders": [{ "id": "forge-47.4.10", "primary": true }]
                },
                "name": "Zombie REAP",
                "version": "3.6",
                "files": [],
                "overrides": "overrides",
                "image": "https://media.forgecdn.net/avatars/1886/328/x.gif"
            }"#,
        )
        .unwrap();

        assert_eq!(
            manifest.image.as_deref(),
            Some("https://media.forgecdn.net/avatars/1886/328/x.gif")
        );
    }

    #[test]
    fn a_manifest_without_a_picture_is_still_a_manifest() {
        let manifest: CfManifest = serde_json::from_str(
            r#"{
                "minecraft": {
                    "version": "1.20.1",
                    "modLoaders": [{ "id": "forge-47.4.10", "primary": true }]
                },
                "name": "Plain",
                "files": []
            }"#,
        )
        .unwrap();

        assert!(manifest.image.is_none());
    }

    #[test]
    fn the_pack_icon_closest_to_the_manifest_wins() {
        let mut entries = [
            "overrides/thumbnail.jpg",
            "overrides/icon.png",
            "icon.jpg",
            "pack.png",
        ];
        entries.sort_by_key(|name| pack_icon_rank(name).unwrap());

        // Root before overrides, then `icon` before `pack`, then png before
        // jpg — so a pack that ships several is read the way a person would.
        assert_eq!(entries[0], "icon.jpg");
        assert_eq!(entries[1], "pack.png");
        assert_eq!(entries[2], "overrides/icon.png");
    }
}
