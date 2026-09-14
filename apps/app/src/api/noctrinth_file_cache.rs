//! The store of downloaded content files, kept by hash.
//!
//! See `theseus::file_cache` for what goes in it and why. These commands are
//! what the Resource management settings need to show its size and empty it.

use crate::api::Result;

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("noctrinth-file-cache")
        .invoke_handler(tauri::generate_handler![
            file_cache_size,
            file_cache_purge,
        ])
        .build()
}

/// How many bytes of downloaded files are being kept.
#[tauri::command]
pub fn file_cache_size() -> u64 {
    theseus::file_cache::file_cache_size()
}

/// Empties the store. Everything in it can be downloaded again.
#[tauri::command]
pub async fn file_cache_purge() -> Result<()> {
    Ok(theseus::file_cache::purge_file_cache().await?)
}
