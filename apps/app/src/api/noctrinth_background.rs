//! A picture, an animated GIF or a looping video behind the launcher, chosen in
//! Settings → Appearance.
//!
//! The idea is Celestial Launcher's (github.com/Wemsur/Celestial-Launcher).
//! The picture is copied next to the app database rather than remembered by
//! path, so it survives the original being moved or deleted, and it is handed
//! to the interface as raw bytes: the webview cannot read files on disk.

use crate::api::Result;
use tauri::ipc::Response;

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("noctrinth-background")
        .invoke_handler(tauri::generate_handler![
            background_set,
            background_get,
            background_clear,
        ])
        .build()
}

/// Big enough for any wallpaper, small enough not to stall the window.
const MAX_BYTES: u64 = 32 * 1024 * 1024;

/// A video is handed over whole like a picture is, so it gets more room but
/// still a ceiling: a short loop, not a film.
const MAX_VIDEO_BYTES: u64 = 100 * 1024 * 1024;

const VIDEO_EXTENSIONS: [&str; 3] = ["mp4", "m4v", "webm"];

async fn stored_path() -> Result<std::path::PathBuf> {
    let state = theseus::State::get().await?;
    Ok(state.directories.settings_dir.join("background"))
}

/// Makes the picture at `path` the background.
#[tauri::command]
pub async fn background_set(path: String) -> Result<()> {
    let size = tokio::fs::metadata(&path)
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))?
        .len();
    let video = std::path::Path::new(&path)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            VIDEO_EXTENSIONS.contains(&extension.to_ascii_lowercase().as_str())
        });
    let max = if video { MAX_VIDEO_BYTES } else { MAX_BYTES };
    if size > max {
        return Err(anyhow::anyhow!(format!(
            "The {} is {} MB; the background takes up to {} MB",
            if video { "video" } else { "picture" },
            size / 1024 / 1024,
            max / 1024 / 1024
        ))
        .into());
    }

    tokio::fs::copy(&path, stored_path().await?)
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;
    Ok(())
}

/// The background's bytes, or none when there is no background.
#[tauri::command]
pub async fn background_get() -> Result<Response> {
    match tokio::fs::read(stored_path().await?).await {
        Ok(bytes) => Ok(Response::new(bytes)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(Response::new(Vec::new()))
        }
        Err(error) => Err(anyhow::anyhow!(error.to_string()).into()),
    }
}

#[tauri::command]
pub async fn background_clear() -> Result<()> {
    match tokio::fs::remove_file(stored_path().await?).await {
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
            Err(anyhow::anyhow!(error.to_string()).into())
        }
        _ => Ok(()),
    }
}
