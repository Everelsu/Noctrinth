use crate::api::Result;

use theseus::{
    pack::{
        install::install_pack,
        install_curseforge::install_curseforge_pack,
        install_from::{CreatePackLocation, CreatePackProfile},
    },
    prelude::*,
};

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("pack")
        .invoke_handler(tauri::generate_handler![
            pack_install,
            pack_install_curseforge,
            pack_get_profile_from_pack,
        ])
        .build()
}

#[tauri::command]
pub async fn pack_install(
    location: CreatePackLocation,
    profile: String,
) -> Result<String> {
    // `install_pack` is the format-agnostic dispatcher — it inspects the
    // file (or version id) and routes to either the mrpack or CurseForge
    // installer, neither of which knows about the other any more.
    Ok(install_pack(location, profile).await?)
}

#[tauri::command]
pub async fn pack_install_curseforge(
    modpack_url: &str,
    curseforge_api_key: &str,
    profile: String,
) -> Result<String> {
    Ok(install_curseforge_pack(modpack_url, curseforge_api_key, profile)
        .await?)
}

#[tauri::command]
pub async fn pack_get_profile_from_pack(
    location: CreatePackLocation,
) -> Result<CreatePackProfile> {
    Ok(pack::install_from::get_profile_from_pack(location).await?)
}
