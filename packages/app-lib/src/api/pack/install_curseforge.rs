//! CurseForge modpack installer (0.15 content-management pipeline).
//!
//! A CurseForge modpack is a `.zip` containing a CurseForge-format
//! `manifest.json` (mods listed by `projectID`/`fileID`) and an `overrides/`
//! folder. This installer:
//!   1. parses the manifest to learn the Minecraft version + loader,
//!   2. creates the instance up-front with those (the 0.15 model fixes
//!      version/loader at creation, not after),
//!   3. resolves every mod file through the CurseForge API, downloads it
//!      (with CDN fallback for author-restricted files) and records it as a
//!      `CurseForge`-sourced content file,
//!   4. extracts `overrides/` into the instance,
//!   5. installs Minecraft (loader, libraries, assets).
//!
//! The pure zip-inspection helpers are also used by `install_from.rs` to route
//! a dropped `.zip` to the right format.

use std::collections::HashSet;
use std::io::Cursor;

use async_zip::base::read::seek::ZipFileReader;
use path_util::SafeRelativeUtf8UnixPathBuf;
use reqwest::Method;
use serde::Deserialize;
use tokio::io::AsyncWriteExt;

use crate::state::{ContentSourceKind, InstanceLink, ModLoader, State};
use crate::util::fetch::{
    DownloadMeta, DownloadReason, fetch, fetch_advanced, fetch_mirrors,
};
use crate::util::io::sanitize_filename;

const CF_FILES_ENDPOINT: &str = "https://api.curseforge.com/v1/mods/files";

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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfManifest {
    minecraft: CfManifestMinecraft,
    name: String,
    files: Vec<CfManifestFile>,
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

/// Install a CurseForge modpack from its download URL.
///
/// Creates the instance itself (from the manifest's version/loader) and returns
/// the new instance id. Cleans up the instance on failure.
#[tracing::instrument(skip(curseforge_api_key))]
pub async fn install_curseforge_pack(
    modpack_url: &str,
    curseforge_api_key: &str,
) -> crate::Result<String> {
    let state = State::get().await?;
    let pack_file = fetch(
        modpack_url,
        None,
        None,
        None,
        &state.fetch_semaphore,
        &state.pool,
    )
    .await?;
    install_curseforge_pack_from_zip(pack_file, curseforge_api_key).await
}

/// Install a CurseForge modpack from already-downloaded zip bytes.
#[tracing::instrument(skip(pack_file, curseforge_api_key))]
pub async fn install_curseforge_pack_from_zip(
    pack_file: bytes::Bytes,
    curseforge_api_key: &str,
) -> crate::Result<String> {
    let instance_id = match install_inner(&pack_file, curseforge_api_key).await
    {
        Ok(id) => id,
        Err(err) => return Err(err),
    };
    Ok(instance_id)
}

async fn install_inner(
    pack_file: &bytes::Bytes,
    curseforge_api_key: &str,
) -> crate::Result<String> {
    let state = State::get().await?;

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
    let manifest: CfManifest = serde_json::from_str(&manifest_str)?;

    let (loader, loader_version) = manifest
        .minecraft
        .mod_loaders
        .iter()
        .find(|l| l.primary)
        .or_else(|| manifest.minecraft.mod_loaders.first())
        .and_then(|l| loader_from_manifest_id(&l.id))
        .unwrap_or((ModLoader::Vanilla, None));

    // Create the instance up-front with the manifest's version + loader.
    let instance = crate::api::instance::create(
        manifest.name.clone(),
        manifest.minecraft.version.clone(),
        loader,
        loader_version,
        None,
        InstanceLink::ImportedModpack {
            project_id: None,
            version_id: None,
            name: Some(manifest.name.clone()),
            version_number: None,
            filename: None,
        },
    )
    .await?;
    let instance_id = instance.instance.id.clone();

    // From here, clean up the instance if anything fails.
    let result = populate_instance(
        &state,
        pack_file,
        &manifest,
        &instance_id,
        curseforge_api_key,
    )
    .await;
    if let Err(err) = result {
        let _ = crate::state::remove_instance(&instance_id, &state).await;
        return Err(err);
    }

    crate::launcher::install_minecraft_for_instance_id_with_reporter(
        &instance_id,
        false,
        None,
    )
    .await?;

    Ok(instance_id)
}

async fn populate_instance(
    state: &State,
    pack_file: &bytes::Bytes,
    manifest: &CfManifest,
    instance_id: &str,
    curseforge_api_key: &str,
) -> crate::Result<()> {
    // Resolve every file through the CurseForge API (one bulk request, then
    // individual re-fetches for any the bulk endpoint omits/dedups).
    let file_ids: Vec<i64> = manifest.files.iter().map(|f| f.file_id).collect();
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

    let download_meta = DownloadMeta {
        reason: DownloadReason::Modpack,
        game_version: manifest.minecraft.version.clone(),
        loader: String::new(),
        dependent_on: None,
    };

    // Download every resolved mod file and record it as CurseForge-sourced.
    for cf_file in &resolved.data {
        let candidates = cf_file.download_candidates();
        if candidates.is_empty() {
            return Err(crate::ErrorKind::InputError(format!(
                "No download URL could be determined for \"{}\" (file id {})",
                cf_file.file_name, cf_file.id
            ))
            .into());
        }
        let mirrors: Vec<&str> =
            candidates.iter().map(String::as_str).collect();
        let bytes = fetch_mirrors(
            &mirrors,
            cf_file.sha1(),
            Some(&download_meta),
            None,
            &state.fetch_semaphore,
            &state.pool,
        )
        .await?;

        let safe_file_name = sanitize_filename(&cf_file.file_name);
        let cf_project_id = cf_file.mod_id.to_string();
        let cf_file_id = cf_file.id.to_string();
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
    }

    // Extract the modpack's overrides/ folder into the instance directory.
    let instance_dir =
        crate::api::instance::get_full_path(instance_id).await?;
    let mut zip_reader = ZipFileReader::with_tokio(Cursor::new(pack_file))
        .await
        .map_err(|_| {
            crate::Error::from(crate::ErrorKind::InputError(
                "Failed to read CurseForge modpack zip".to_string(),
            ))
        })?;
    let override_entries = zip_reader
        .file()
        .entries()
        .iter()
        .enumerate()
        .filter_map(|(index, file)| {
            let filename = file.filename().as_str().unwrap_or_default();
            ((filename.starts_with("overrides/")
                || filename.starts_with("client-overrides/"))
                && !filename.ends_with('/'))
            .then_some(index)
        })
        .collect::<Vec<_>>();

    for index in override_entries {
        let entry_name = {
            let entries = zip_reader.file().entries();
            entries[index]
                .filename()
                .as_str()
                .unwrap_or_default()
                .to_string()
        };
        let raw_path = SafeRelativeUtf8UnixPathBuf::try_from(entry_name)?;
        let relative_path = raw_path
            .strip_prefix("overrides")
            .or_else(|_| raw_path.strip_prefix("client-overrides"))
            .map_err(|_| {
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
    }

    Ok(())
}
