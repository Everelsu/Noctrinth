//! CurseForge modpack installer.
//!
//! A CurseForge modpack is a `.zip` containing a CurseForge-format
//! `manifest.json` (which lists mods by `projectID`/`fileID`) and an
//! `overrides/` folder. This module downloads the pack, resolves every mod
//! file through the CurseForge API, then installs everything into a profile —
//! reusing the same building blocks as the Modrinth `.mrpack` installer
//! (`set_profile_information`, `fetch_mirrors`, override extraction,
//! `install_minecraft`) so progress flows through the existing loading bar.

use std::collections::HashMap;
use std::io::Cursor;

use async_zip::base::read::seek::ZipFileReader;
use futures::StreamExt;
use path_util::SafeRelativeUtf8UnixPathBuf;
use reqwest::Method;
use serde::Deserialize;

use crate::data::ProjectType;
use crate::event::LoadingBarType;
use crate::event::emit::{
    emit_loading, init_loading, loading_try_for_each_concurrent,
};
use crate::pack::install_from::{
    CreatePackDescription, PackDependency, set_profile_information,
};
use crate::state::{Profile, cache_file_hash};
use crate::util::fetch::{
    DownloadMeta, DownloadReason, fetch, fetch_advanced, fetch_mirrors, write,
};
use crate::{State, profile};

const CF_FILES_ENDPOINT: &str = "https://api.curseforge.com/v1/mods/files";

// ─── CurseForge manifest.json ─────────────────────────────────────────────────

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
    #[serde(rename = "fileID")]
    file_id: i64,
}

// ─── CurseForge /mods/files response ──────────────────────────────────────────

#[derive(Deserialize)]
struct CfFilesResponse {
    data: Vec<CfFileData>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfFileData {
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
}

// ─── Installer ────────────────────────────────────────────────────────────────

/// Returns true if the zip looks like a CurseForge modpack — i.e. it contains
/// a `manifest.json`. Used to route file imports to the right installer.
pub async fn zip_has_curseforge_manifest(pack_file: &bytes::Bytes) -> bool {
    match ZipFileReader::with_tokio(Cursor::new(pack_file)).await {
        Ok(zip) => zip
            .file()
            .entries()
            .iter()
            .any(|f| matches!(f.filename().as_str(), Ok("manifest.json"))),
        Err(_) => false,
    }
}

/// Install a CurseForge modpack from its download URL.
#[tracing::instrument(skip(curseforge_api_key))]
pub async fn install_curseforge_pack(
    modpack_url: &str,
    curseforge_api_key: &str,
    profile_path: String,
) -> crate::Result<String> {
    let state = State::get().await?;
    let pack_file =
        fetch(modpack_url, None, None, &state.fetch_semaphore, &state.pool)
            .await?;
    install_curseforge_pack_from_zip(pack_file, curseforge_api_key, profile_path)
        .await
}

/// Install a CurseForge modpack from already-downloaded zip bytes.
///
/// On failure the profile is removed so a half-installed instance is never
/// left behind — mirroring `install_zipped_mrpack`.
#[tracing::instrument(skip(pack_file, curseforge_api_key))]
pub async fn install_curseforge_pack_from_zip(
    pack_file: bytes::Bytes,
    curseforge_api_key: &str,
    profile_path: String,
) -> crate::Result<String> {
    let result = install_curseforge_pack_inner(
        pack_file,
        curseforge_api_key,
        profile_path.clone(),
    )
    .await;

    match result {
        Ok(path) => Ok(path),
        Err(err) => {
            let _ = crate::api::profile::remove(&profile_path).await;
            Err(err)
        }
    }
}

async fn install_curseforge_pack_inner(
    pack_file: bytes::Bytes,
    curseforge_api_key: &str,
    profile_path: String,
) -> crate::Result<String> {
    let state = &State::get().await?;

    // Open the zip and read manifest.json.
    let mut zip_reader = ZipFileReader::with_tokio(Cursor::new(&pack_file))
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

    // 3. Resolve every file through the CurseForge API (one bulk request).
    let file_ids: Vec<i64> =
        manifest.files.iter().map(|f| f.file_id).collect();

    let resolved: CfFilesResponse = {
        let body = fetch_advanced(
            Method::POST,
            CF_FILES_ENDPOINT,
            None,
            Some(serde_json::json!({ "fileIds": file_ids })),
            Some(("x-api-key", curseforge_api_key)),
            None,
            None,
            &state.fetch_semaphore,
            &state.pool,
        )
        .await?;
        serde_json::from_slice(&body)?
    };

    // 4. Build profile dependencies (Minecraft version + loader).
    let mut dependencies: HashMap<PackDependency, String> = HashMap::new();
    dependencies.insert(
        PackDependency::Minecraft,
        manifest.minecraft.version.clone(),
    );

    let loader = manifest
        .minecraft
        .mod_loaders
        .iter()
        .find(|l| l.primary)
        .or_else(|| manifest.minecraft.mod_loaders.first());
    if let Some(loader) = loader
        && let Some((kind, version)) = loader.id.split_once('-')
    {
        let dependency = match kind {
            "forge" => Some(PackDependency::Forge),
            "neoforge" => Some(PackDependency::NeoForge),
            "fabric" => Some(PackDependency::FabricLoader),
            "quilt" => Some(PackDependency::QuiltLoader),
            _ => None,
        };
        if let Some(dependency) = dependency {
            dependencies.insert(dependency, version.to_string());
        }
    }

    // 5. Apply name / game version / loader to the profile.
    let description = CreatePackDescription {
        icon: None,
        override_title: Some(manifest.name.clone()),
        project_id: None,
        version_id: None,
        existing_loading_bar: None,
        profile_path: profile_path.clone(),
    };
    set_profile_information(
        profile_path.clone(),
        &description,
        &manifest.name,
        &dependencies,
        false,
    )
    .await?;

    // 6. Loading bar — drives the existing UI progress indicator.
    let loading_bar = init_loading(
        LoadingBarType::PackDownload {
            profile_path: profile_path.clone(),
            pack_name: manifest.name.clone(),
            icon: None,
            pack_id: None,
            pack_version: None,
        },
        100.0,
        "Downloading CurseForge modpack",
    )
    .await?;

    let profile = Profile::get(&profile_path, &state.pool)
        .await?
        .ok_or_else(|| {
            crate::ErrorKind::UnmanagedProfileError(profile_path.to_string())
                .as_error()
        })?;

    let download_meta = DownloadMeta {
        reason: DownloadReason::Modpack,
        game_version: profile.game_version.clone(),
        loader: profile.loader.as_str().to_string(),
    };

    // 7. Download every resolved mod file into the profile.
    let files: Vec<CfFileData> = resolved
        .data
        .into_iter()
        .filter(|f| f.download_url.is_some())
        .collect();
    let num_files = files.len();

    loading_try_for_each_concurrent(
        futures::stream::iter(files).map(Ok::<CfFileData, crate::Error>),
        None,
        Some(&loading_bar),
        70.0,
        num_files,
        None,
        |cf_file| {
            let profile_path = profile_path.clone();
            let download_meta = download_meta.clone();
            async move {
                let Some(download_url) = cf_file.download_url.as_deref()
                else {
                    return Ok(());
                };

                let bytes = fetch_mirrors(
                    &[download_url],
                    cf_file.sha1(),
                    Some(&download_meta),
                    &state.fetch_semaphore,
                    &state.pool,
                )
                .await?;

                let relative_path = format!("mods/{}", cf_file.file_name);
                let path = profile::get_full_path(&profile_path)
                    .await?
                    .join(&relative_path);

                cache_file_hash(
                    bytes.clone(),
                    &profile_path,
                    &relative_path,
                    cf_file.sha1(),
                    ProjectType::get_from_parent_folder(&path),
                    &state.pool,
                )
                .await?;

                write(&path, &bytes, &state.io_semaphore).await?;

                Ok(())
            }
        },
    )
    .await?;

    // 8. Extract the modpack's overrides/ folder.
    emit_loading(&loading_bar, 0.0, Some("Extracting overrides"))?;

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
            .then(|| (index, file.clone()))
        })
        .collect::<Vec<_>>();
    let override_count = override_entries.len();

    for (i, (index, file)) in override_entries.into_iter().enumerate() {
        let raw_path = SafeRelativeUtf8UnixPathBuf::try_from(
            file.filename().as_str().unwrap_or_default().to_string(),
        )?;
        let relative_path = raw_path
            .strip_prefix("overrides")
            .or_else(|_| raw_path.strip_prefix("client-overrides"))
            .map_err(|_| {
                crate::Error::from(crate::ErrorKind::OtherError(format!(
                    "Failed to strip override prefix: {raw_path}"
                )))
            })?;

        let mut file_bytes = vec![];
        {
            let mut reader = zip_reader.reader_with_entry(index).await?;
            reader.read_to_end_checked(&mut file_bytes).await?;
        }
        let file_bytes = bytes::Bytes::from(file_bytes);

        cache_file_hash(
            file_bytes.clone(),
            &profile_path,
            relative_path.as_str(),
            None,
            ProjectType::get_from_parent_folder(relative_path.as_str()),
            &state.pool,
        )
        .await?;

        write(
            &profile::get_full_path(&profile_path)
                .await?
                .join(relative_path.as_str()),
            &file_bytes,
            &state.io_semaphore,
        )
        .await?;

        emit_loading(
            &loading_bar,
            30.0 / override_count.max(1) as f64,
            Some(&format!(
                "Extracting override {}/{override_count}",
                i + 1
            )),
        )?;
    }

    // 9. Install Minecraft (loader, libraries, assets).
    if let Some(profile_val) = profile::get(&profile_path).await? {
        crate::launcher::install_minecraft(
            &profile_val,
            Some(loading_bar),
            false,
        )
        .await?;
    }

    Ok(profile_path)
}
