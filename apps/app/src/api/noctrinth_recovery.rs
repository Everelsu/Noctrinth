//! Instances that are on disk but not in the launcher.
//!
//! See `theseus::instance_recovery` for what is read out of a folder and why.

use crate::api::Result;
use std::path::PathBuf;
use theseus::instance_recovery::{self, OrphanedInstance};
use theseus::launcher_search;
use theseus::pack::import::ImportLauncherType;
use theseus::prelude::ModLoader;

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("noctrinth-recovery")
        .invoke_handler(tauri::generate_handler![
            recovery_scan_for_orphans,
            recovery_adopt_orphan,
            recovery_find_launchers,
        ])
        .build()
}

/// The folders under the instances directory the launcher has no row for.
#[tauri::command]
pub async fn recovery_scan_for_orphans() -> Result<Vec<OrphanedInstance>> {
    Ok(instance_recovery::scan_for_orphans().await?)
}

/// Puts one of them back, with its folder where it is.
#[tauri::command]
pub async fn recovery_adopt_orphan(
    folder: &str,
    name: &str,
    game_version: &str,
    loader: ModLoader,
    loader_version: Option<String>,
) -> Result<String> {
    Ok(instance_recovery::adopt_orphan(
        folder,
        name,
        game_version,
        loader,
        loader_version,
    )
    .await?)
}

/// Everywhere this launcher was found, rather than only where its installer
/// would have put it.
#[tauri::command]
pub fn recovery_find_launchers(
    launcher_type: ImportLauncherType,
) -> Vec<PathBuf> {
    launcher_search::find_all(launcher_type)
}
