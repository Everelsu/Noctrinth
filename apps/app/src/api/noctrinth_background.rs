//! A picture, an animated GIF or a looping video behind the launcher, chosen in
//! Settings → Appearance.
//!
//! The idea is Celestial Launcher's (github.com/Wemsur/Celestial-Launcher).
//! The picture is copied next to the app database rather than remembered by
//! path, so it survives the original being moved or deleted, and it is streamed
//! to the webview through Tauri's asset protocol rather than buffered in RAM.

use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use crate::api::Result;
use serde::Serialize;
use tauri::Manager;

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

/// Looping videos are streamed from disk, so they can be larger than stills.
const MAX_VIDEO_BYTES: u64 = 100 * 1024 * 1024;

const IMAGE_EXTENSIONS: [&str; 6] = ["png", "jpg", "jpeg", "webp", "gif", "avif"];
const VIDEO_EXTENSIONS: [&str; 3] = ["mp4", "m4v", "webm"];

#[derive(Debug, Clone, Serialize)]
pub struct BackgroundInfo {
    pub path: String,
    pub is_video: bool,
    pub revision: u64,
}

async fn settings_dir() -> Result<PathBuf> {
    let state = theseus::State::get().await?;
    Ok(state.directories.settings_dir.clone())
}

fn is_video_extension(ext: &str) -> bool {
    VIDEO_EXTENSIONS.contains(&ext)
}

fn is_supported_extension(ext: &str) -> bool {
    IMAGE_EXTENSIONS.contains(&ext) || is_video_extension(ext)
}

fn normalized_extension(path: &Path) -> Option<String> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    is_supported_extension(&ext).then_some(ext)
}

fn detect_extension_from_header(bytes: &[u8]) -> &'static str {
    if bytes.len() >= 12 && &bytes[4..8] == b"ftyp" {
        if &bytes[8..11] == b"avi" {
            return "avif";
        }
        return "mp4";
    }
    if bytes.starts_with(&[0x1a, 0x45, 0xdf, 0xa3]) {
        return "webm";
    }
    if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        return "png";
    }
    if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        return "jpg";
    }
    if bytes.starts_with(b"GIF8") {
        return "gif";
    }
    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP"
    {
        return "webp";
    }
    "png"
}

async fn remove_existing_backgrounds(dir: &Path) -> Result<()> {
    let mut entries = tokio::fs::read_dir(dir)
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;
    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))?
    {
        let file_name = entry.file_name();
        let Some(name) = file_name.to_str() else {
            continue;
        };
        if name == "background" || name.starts_with("background.") {
            let _ = tokio::fs::remove_file(entry.path()).await;
        }
    }
    Ok(())
}

async fn find_stored_background(dir: &Path) -> Result<Option<(PathBuf, bool)>> {
    for ext in IMAGE_EXTENSIONS.iter().chain(VIDEO_EXTENSIONS.iter()) {
        let candidate = dir.join(format!("background.{ext}"));
        if tokio::fs::try_exists(&candidate)
            .await
            .unwrap_or(false)
        {
            return Ok(Some((candidate, is_video_extension(ext))));
        }
    }

    // Migrate legacy extensionless `background` file so the asset protocol serves
    // the right MIME type and Range headers for video playback.
    let legacy = dir.join("background");
    if tokio::fs::try_exists(&legacy).await.unwrap_or(false) {
        let mut header = [0u8; 16];
        if let Ok(mut file) = tokio::fs::File::open(&legacy).await {
            use tokio::io::AsyncReadExt;
            let read = file.read(&mut header).await.unwrap_or(0);
            let ext = detect_extension_from_header(&header[..read]);
            let target = dir.join(format!("background.{ext}"));
            if tokio::fs::rename(&legacy, &target).await.is_ok() {
                return Ok(Some((target, is_video_extension(ext))));
            }
            return Ok(Some((legacy, is_video_extension(ext))));
        }
    }

    Ok(None)
}

/// Makes the picture at `path` the background.
#[tauri::command]
pub async fn background_set(path: String) -> Result<()> {
    let source = Path::new(&path);
    let size = tokio::fs::metadata(source)
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))?
        .len();
    let ext = normalized_extension(source).unwrap_or_else(|| "png".to_string());
    let video = is_video_extension(&ext);
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

    let dir = settings_dir().await?;
    let target = dir.join(format!("background.{ext}"));
    if source != target {
        remove_existing_backgrounds(&dir).await?;
        tokio::fs::copy(source, &target)
            .await
            .map_err(|error| anyhow::anyhow!(error.to_string()))?;
    }
    Ok(())
}

/// Metadata for the stored background, or none when there is no background.
#[tauri::command]
pub async fn background_get<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<Option<BackgroundInfo>> {
    let dir = settings_dir().await?;
    let Some((path, is_video)) = find_stored_background(&dir).await? else {
        return Ok(None);
    };

    let revision = tokio::fs::metadata(&path)
        .await
        .ok()
        .and_then(|meta| meta.modified().ok())
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0);

    app.asset_protocol_scope()
        .allow_file(&path)
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;

    Ok(Some(BackgroundInfo {
        path: path.to_string_lossy().into_owned(),
        is_video,
        revision,
    }))
}

#[tauri::command]
pub async fn background_clear() -> Result<()> {
    let dir = settings_dir().await?;
    remove_existing_backgrounds(&dir).await
}
