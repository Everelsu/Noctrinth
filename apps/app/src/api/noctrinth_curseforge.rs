//! Handing the backend the CurseForge key the frontend was built with.
//!
//! The key is a build-time constant of the frontend's — vite bakes it into the
//! bundle — and until now it only ever travelled as an argument on a modpack
//! install. Naming what a CurseForge file is needs it too, from deep inside
//! the content listing where no such argument reaches, so the frontend says
//! once at startup what the key is and the backend keeps it.
//!
//! Passing `null` is meaningful: a build configured without a key says so, and
//! CurseForge lookups quietly do nothing rather than failing anything.

use crate::api::Result;
use theseus::curseforge_metadata;

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("noctrinth-curseforge")
        .invoke_handler(tauri::generate_handler![curseforge_set_api_key])
        .build()
}

#[tauri::command]
pub async fn curseforge_set_api_key(key: Option<String>) -> Result<()> {
    curseforge_metadata::set_api_key(key);
    Ok(())
}
