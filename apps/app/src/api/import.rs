use std::path::PathBuf;

use crate::api::Result;
use theseus::pack::import::ImportLauncherType;

use theseus::pack::import;

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("import")
        .invoke_handler(tauri::generate_handler![
            get_importable_instances,
            is_valid_importable_instance,
            get_default_launcher_path,
            remove_source_instance,
        ])
        .build()
}

/// Gets a list of importable instances from a launcher type and base path
/// eg: get_importable_instances(ImportLauncherType::MultiMC, PathBuf::from("C:/MultiMC"))
/// returns ["Instance 1", "Instance 2"]
#[tauri::command]
pub async fn get_importable_instances(
    launcher_type: ImportLauncherType,
    base_path: PathBuf,
) -> Result<Vec<String>> {
    Ok(import::get_importable_instances(launcher_type, base_path).await?)
}

/// Checks if this instance is valid for importing, given a certain launcher type
/// eg: is_valid_importable_instance(PathBuf::from("C:/MultiMC/Instance 1"), ImportLauncherType::MultiMC)
#[tauri::command]
pub async fn is_valid_importable_instance(
    instance_folder: PathBuf,
    launcher_type: ImportLauncherType,
) -> Result<bool> {
    Ok(
        import::is_valid_importable_instance(instance_folder, launcher_type)
            .await,
    )
}

/// Deletes an instance from the launcher it was imported from, once the import
/// has finished. Only supported for Modrinth App.
#[tauri::command]
pub async fn remove_source_instance(
    launcher_type: ImportLauncherType,
    base_path: PathBuf,
    instance_folder: String,
) -> Result<()> {
    Ok(import::remove_source_instance(
        launcher_type,
        base_path,
        instance_folder,
    )
    .await?)
}

/// Returns the default path for the given launcher type
/// None if it can't be found or doesn't exist
#[tauri::command]
pub async fn get_default_launcher_path(
    launcher_type: ImportLauncherType,
) -> Result<Option<PathBuf>> {
    Ok(import::get_default_launcher_path(launcher_type))
}
