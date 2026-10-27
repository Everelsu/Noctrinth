//! Noctrinth's CurseForge commands.
//!
//! `curseforge_set_api_key` hands the backend the key the frontend was built
//! with. The key is a build-time constant of the frontend's — vite bakes it
//! into the bundle — so the frontend says at startup what it is, and whether
//! CurseForge is on, and the backend keeps it. Passing `null` is meaningful: a
//! build configured without a key says so, and CurseForge lookups quietly do
//! nothing rather than failing anything — unless the user brought a key of
//! their own, which the backend keeps and the frontend never reads back.
//!
//! The rest are the browse page's: searching CurseForge, a project's page,
//! installing into an instance, and the files an instance is waiting on
//! because their author keeps them off launchers.

use crate::api::Result;
use theseus::curseforge_blocked::PendingFile;
use theseus::curseforge_browse::{
    self, Category, InstallOutcome, Project, ProjectFile, ProjectVersion,
    SearchQuery, SearchResults,
};
use theseus::curseforge_metadata;
use theseus::handler::CURSEFORGE_DEEP_LINK_SCHEME;

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("noctrinth-curseforge")
        .invoke_handler(tauri::generate_handler![
            curseforge_set_api_key,
            curseforge_has_user_key,
            curseforge_set_user_key,
            curseforge_check_key,
            curseforge_set_options,
            curseforge_clear_cache,
            curseforge_search,
            curseforge_get_categories,
            curseforge_get_project,
            curseforge_get_description,
            curseforge_get_files,
            curseforge_get_version,
            curseforge_install_project,
            curseforge_pending_files,
            curseforge_check_pending_downloads,
            curseforge_dismiss_pending_file,
            curseforge_download_shared_profile,
            curseforge_link_handler_get,
            curseforge_link_handler_set,
        ])
        .build()
}

#[tauri::command]
pub async fn curseforge_set_api_key(
    key: Option<String>,
    enabled: bool,
) -> Result<()> {
    curseforge_metadata::set_api_key(key, enabled);
    Ok(())
}

#[tauri::command]
pub async fn curseforge_has_user_key() -> Result<bool> {
    Ok(curseforge_metadata::has_user_key())
}

#[tauri::command]
pub async fn curseforge_set_user_key(key: Option<String>) -> Result<()> {
    Ok(curseforge_metadata::set_user_key(key).await?)
}

#[tauri::command]
pub async fn curseforge_check_key(key: Option<String>) -> Result<bool> {
    Ok(curseforge_metadata::check_key(key).await?)
}

#[tauri::command]
pub async fn curseforge_set_options(
    options: curseforge_metadata::CurseforgeOptions,
) -> Result<()> {
    curseforge_metadata::set_options(options);
    Ok(())
}

#[tauri::command]
pub async fn curseforge_clear_cache() -> Result<()> {
    curseforge_browse::clear_cache();
    Ok(())
}

#[tauri::command]
pub async fn curseforge_search(query: SearchQuery) -> Result<SearchResults> {
    Ok(curseforge_browse::search(query).await?)
}

#[tauri::command]
pub async fn curseforge_get_categories(
    project_type: String,
) -> Result<Vec<Category>> {
    Ok(curseforge_browse::get_categories(&project_type).await?)
}

#[tauri::command]
pub async fn curseforge_get_project(id: String) -> Result<Project> {
    Ok(curseforge_browse::get_project(&id).await?)
}

#[tauri::command]
pub async fn curseforge_get_description(id: String) -> Result<String> {
    Ok(curseforge_browse::get_description(&id).await?)
}

#[tauri::command]
pub async fn curseforge_get_files(
    id: String,
    game_version: Option<String>,
    loader: Option<String>,
) -> Result<Vec<ProjectFile>> {
    Ok(curseforge_browse::get_files(&id, game_version, loader).await?)
}

#[tauri::command]
pub async fn curseforge_get_version(
    id: String,
    file_id: String,
) -> Result<ProjectVersion> {
    Ok(curseforge_browse::get_version(&id, &file_id).await?)
}

#[tauri::command]
pub async fn curseforge_install_project(
    instance_id: String,
    project_id: String,
    file_id: Option<String>,
) -> Result<InstallOutcome> {
    Ok(
        curseforge_browse::install_project(&instance_id, &project_id, file_id)
            .await?,
    )
}

#[tauri::command]
pub async fn curseforge_pending_files(
    instance_id: String,
) -> Result<Vec<PendingFile>> {
    Ok(curseforge_browse::pending_files(&instance_id).await?)
}

#[tauri::command]
pub async fn curseforge_check_pending_downloads(
    instance_id: String,
) -> Result<Vec<PendingFile>> {
    Ok(curseforge_browse::check_pending_downloads(&instance_id).await?)
}

#[tauri::command]
pub async fn curseforge_dismiss_pending_file(
    instance_id: String,
    file_id: String,
) -> Result<()> {
    Ok(curseforge_browse::dismiss_pending_file(&instance_id, &file_id).await?)
}

/// Downloads the modpack behind a CurseForge profile code; the zip's path is
/// then imported like any modpack file.
#[tauri::command]
pub async fn curseforge_download_shared_profile(
    code: String,
) -> Result<std::path::PathBuf> {
    Ok(curseforge_browse::download_shared_profile(&code).await?)
}

/// Whether CurseForge's Install links open Noctrinth, or `None` where that
/// cannot be changed at runtime: macOS takes its schemes from the bundle.
#[tauri::command]
pub async fn curseforge_link_handler_get<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<Option<bool>> {
    use tauri_plugin_deep_link::DeepLinkExt;

    match app.deep_link().is_registered(CURSEFORGE_DEEP_LINK_SCHEME) {
        Ok(registered) => Ok(Some(registered)),
        Err(tauri_plugin_deep_link::Error::UnsupportedPlatform) => Ok(None),
        Err(error) => Err(anyhow::anyhow!(error.to_string()).into()),
    }
}

/// Makes CurseForge's Install links open Noctrinth, or stops them doing so.
///
/// The scheme is CurseForge's app's, so this is only ever done when asked, and
/// undoing it removes only what Noctrinth wrote: the plugin's own unregister
/// would also delete a machine-wide registration that may be the app's.
#[tauri::command]
pub async fn curseforge_link_handler_set<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    enabled: bool,
) -> Result<()> {
    use tauri_plugin_deep_link::DeepLinkExt;

    let deep_link = app.deep_link();
    let outcome = if enabled {
        deep_link.register(CURSEFORGE_DEEP_LINK_SCHEME)
    } else if deep_link
        .is_registered(CURSEFORGE_DEEP_LINK_SCHEME)
        .unwrap_or(false)
    {
        unregister_own_link_handler(&deep_link)
    } else {
        Ok(())
    };
    outcome.map_err(|error| anyhow::anyhow!(error.to_string()).into())
}

#[cfg(windows)]
fn unregister_own_link_handler<R: tauri::Runtime>(
    _: &tauri_plugin_deep_link::DeepLink<R>,
) -> tauri_plugin_deep_link::Result<()> {
    use windows::Win32::System::Registry::{HKEY_CURRENT_USER, RegDeleteTreeW};
    use windows::core::HSTRING;

    let key = HSTRING::from(format!(
        "Software\\Classes\\{CURSEFORGE_DEEP_LINK_SCHEME}"
    ));
    // SAFETY: a predefined root key and a valid, NUL-terminated wide string.
    let status = unsafe { RegDeleteTreeW(HKEY_CURRENT_USER, &key) };
    status.ok().map_err(|error| std::io::Error::other(error).into())
}

#[cfg(not(windows))]
fn unregister_own_link_handler<R: tauri::Runtime>(
    deep_link: &tauri_plugin_deep_link::DeepLink<R>,
) -> tauri_plugin_deep_link::Result<()> {
    deep_link.unregister(CURSEFORGE_DEEP_LINK_SCHEME)
}
