//! Noctrinth's CurseForge commands.
//!
//! `curseforge_set_api_key` hands the backend the key the frontend was built
//! with. The key is a build-time constant of the frontend's — vite bakes it
//! into the bundle — so the frontend says once at startup what it is and the
//! backend keeps it. Passing `null` is meaningful: a build configured without
//! a key says so, and CurseForge lookups quietly do nothing rather than
//! failing anything.
//!
//! The rest are the browse page's: searching CurseForge, a project's page,
//! installing into an instance, and the files an instance is waiting on
//! because their author keeps them off launchers.

use std::path::PathBuf;

use crate::api::Result;
use theseus::curseforge_blocked::PendingFile;
use theseus::curseforge_browse::{
    self, Category, InstallOutcome, Project, ProjectFile, SearchQuery,
    SearchResults,
};
use theseus::curseforge_metadata;

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("noctrinth-curseforge")
        .invoke_handler(tauri::generate_handler![
            curseforge_set_api_key,
            curseforge_search,
            curseforge_get_categories,
            curseforge_get_project,
            curseforge_get_description,
            curseforge_get_files,
            curseforge_install_project,
            curseforge_download_modpack,
            curseforge_pending_files,
            curseforge_check_pending_downloads,
            curseforge_dismiss_pending_file,
        ])
        .build()
}

#[tauri::command]
pub async fn curseforge_set_api_key(key: Option<String>) -> Result<()> {
    curseforge_metadata::set_api_key(key);
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
pub async fn curseforge_download_modpack(
    project_id: String,
    file_id: Option<String>,
) -> Result<PathBuf> {
    Ok(curseforge_browse::download_modpack(&project_id, file_id).await?)
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
