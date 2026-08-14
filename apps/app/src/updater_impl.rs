use crate::api::Result;
use std::sync::{Arc, Mutex};
use tauri::http::HeaderValue;
use tauri::http::header::ACCEPT;
use tauri::{Manager, ResourceId, Runtime, Webview};
use tauri_plugin_http::reqwest;
use tauri_plugin_http::reqwest::ClientBuilder;
use tauri_plugin_updater::Error;
use tauri_plugin_updater::Update;
use theseus::{
    LoadingBarType, emit_loading, init_loading, launcher_user_agent,
};
use tokio::time::Instant;

#[derive(Default)]
pub struct PendingUpdateData(pub Mutex<Option<(Arc<Update>, Vec<u8>)>>);

/// Why this installation cannot replace itself, if it cannot.
///
/// On Linux the updater rewrites the app in place: it renames the running
/// executable aside and writes the new one over the old path, so it needs write
/// access to the *directory* holding it, not just the file. A `.deb` or `.rpm`
/// install puts that under `/usr` or `/opt`, owned by root, and the rename ends
/// as `Permission denied (os error 13)`.
///
/// Package-manager bundles have their own path through `pkexec`, so those are
/// left alone; the check only covers the rename branch, which is also the
/// fallback whenever the bundle type could not be determined at all.
///
/// Checked before downloading rather than after, so a doomed update doesn't
/// cost the user the whole download first.
// `cfg!` rather than `#[cfg]` so the whole body still type-checks on every
// platform, including from a Windows dev machine.
pub fn self_update_blocker() -> Option<String> {
    use tauri::utils::config::BundleType;
    use tauri::utils::platform::bundle_type;

    // Only the Linux updater rewrites the app in place. Windows re-runs its
    // installer and macOS swaps the bundle, so neither is answered by this.
    if !cfg!(target_os = "linux") {
        return None;
    }

    // These install through a privilege prompt instead of a rename.
    if matches!(bundle_type(), Some(BundleType::Deb | BundleType::Rpm)) {
        return None;
    }

    // Matches how the updater plugin resolves its target: the AppImage itself
    // when running as one, otherwise the executable.
    let target = std::env::var_os("APPIMAGE")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::current_exe().ok())?;
    let parent = target.parent()?;

    // Probing beats reading mode bits: it accounts for ownership, ACLs and
    // read-only mounts in one go. The rename needs write access to the
    // directory, not just to the file.
    let probe =
        parent.join(format!(".noctrinth-update-check-{}", std::process::id()));
    match std::fs::File::create(&probe) {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
            None
        }
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            Some(format!(
                "Noctrinth is installed in {}, which this user cannot write to, \
                 so it cannot replace itself there.\n\n\
                 If you installed the .deb, update through your package manager \
                 or install the new .deb from the releases page. If you are \
                 running the AppImage, move it somewhere you own — your home \
                 directory, for example — and update from there.",
                parent.display()
            ))
        }
        // Anything else (a full disk, a missing directory) is not ours to
        // diagnose here; let the real install surface it.
        Err(_) => None,
    }
}

// Reimplementation of Update::download mostly, minus the actual download part
#[tauri::command]
pub async fn get_update_size<R: Runtime>(
    webview: Webview<R>,
    rid: ResourceId,
) -> Result<Option<u64>> {
    let update = webview.resources_table().get::<Update>(rid)?;

    let mut headers = update.headers.clone();
    if !headers.contains_key(ACCEPT) {
        headers.insert(
            ACCEPT,
            HeaderValue::from_static("application/octet-stream"),
        );
    }

    let mut request = ClientBuilder::new().user_agent(launcher_user_agent());
    if let Some(timeout) = update.timeout {
        request = request.timeout(timeout);
    }
    if let Some(ref proxy) = update.proxy {
        let proxy = reqwest::Proxy::all(proxy.as_str())?;
        request = request.proxy(proxy);
    }
    let response = request
        .build()?
        .head(update.download_url.clone())
        .headers(headers)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(Error::Network(format!(
            "Download request failed with status: {}",
            response.status()
        ))
        .into());
    }

    let content_length = response
        .headers()
        .get("Content-Length")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse().ok());

    Ok(content_length)
}

#[tauri::command]
pub async fn enqueue_update_for_installation<R: Runtime>(
    webview: Webview<R>,
    rid: ResourceId,
) -> Result<()> {
    if let Some(reason) = self_update_blocker() {
        return Err(Error::Io(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            reason,
        ))
        .into());
    }

    let pending_data = webview.state::<PendingUpdateData>().inner();

    let update = webview.resources_table().get::<Update>(rid)?;

    let progress = init_loading(
        LoadingBarType::LauncherUpdate {
            version: update.version.clone(),
            current_version: update.current_version.clone(),
        },
        1.0,
        "Downloading update...",
    )
    .await?;

    let download_start = Instant::now();
    let update_data = update
        .download(
            |chunk_size, total_size| {
                let Some(total_size) = total_size else {
                    return;
                };
                if let Err(e) = emit_loading(
                    &progress,
                    chunk_size as f64 / total_size as f64,
                    None,
                ) {
                    tracing::error!(
                        "Failed to update download progress bar: {e}"
                    );
                }
            },
            || {},
        )
        .await?;
    let download_duration = download_start.elapsed();
    tracing::info!("Downloaded update in {download_duration:?}");

    pending_data
        .0
        .lock()
        .unwrap()
        .replace((update, update_data));

    Ok(())
}

#[tauri::command]
pub fn remove_enqueued_update<R: Runtime>(webview: Webview<R>) {
    let pending_data = webview.state::<PendingUpdateData>().inner();
    pending_data.0.lock().unwrap().take();
}
