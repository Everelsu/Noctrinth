//! Noctrinth plugin manager backend.
//!
//! Stores installed plugins under `<settings_dir>/plugins/<id>/` with an
//! accompanying `state.json`. Remote installs from the Millennium-style
//! `Noctrinth-Plugins` catalog strictly verify SHA-256 before extraction.

use std::collections::HashMap;
use std::io::{Cursor, Read};
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine as _;
use crate::api::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::Emitter;

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("noctrinth-plugins")
        .invoke_handler(tauri::generate_handler![
            plugins_list,
            plugins_install_zip,
            plugins_install_remote,
            plugins_set_enabled,
            plugins_remove,
            plugins_read_asset,
            plugins_read_asset_base64,
            plugins_http_fetch,
            plugins_http_fetch_stream,
            plugins_fetch_catalog,
            plugins_get_dir,
            plugins_is_safe_mode,
            plugins_instance_list_files,
            plugins_instance_read_file,
            plugins_instance_write_file,
        ])
        .build()
}

const SUPPORTED_API_VERSION: u32 = 1;
const MAX_ARCHIVE_BYTES: usize = 25 * 1024 * 1024;
const MAX_ASSET_BYTES: usize = 2 * 1024 * 1024;
const MAX_BINARY_ASSET_BYTES: usize = 4 * 1024 * 1024;
const MAX_INSTANCE_FILE_BYTES: usize = 1024 * 1024;
const MAX_RESPONSE_BYTES: usize = 4 * 1024 * 1024;
const MAX_STREAM_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginPermissions {
    #[serde(default)]
    pub domains: Vec<String>,
    #[serde(default)]
    pub events: Vec<String>,
    #[serde(default)]
    pub instances: Vec<String>,
    #[serde(default)]
    pub ui: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub api_version: u32,
    #[serde(rename = "type")]
    pub plugin_type: String,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub styles: Vec<String>,
    #[serde(default)]
    pub entry: Option<String>,
    #[serde(default)]
    pub permissions: PluginPermissions,
    #[serde(default)]
    pub settings: Vec<serde_json::Value>,
    #[serde(default)]
    pub presets: Vec<serde_json::Value>,
    #[serde(default)]
    pub pages: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PluginStateEntry {
    enabled: bool,
    source: String,
    sha256: String,
    installed_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct PluginsStateFile {
    #[serde(default)]
    plugins: HashMap<String, PluginStateEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InstalledPlugin {
    pub manifest: PluginManifest,
    pub enabled: bool,
    pub source: String,
    pub sha256: String,
    pub installed_at: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PluginHttpResponse {
    pub status: u16,
    pub ok: bool,
    pub headers: HashMap<String, String>,
    pub body: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PluginStreamHead {
    pub status: u16,
    pub ok: bool,
    pub headers: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PluginStreamChunkEvent {
    plugin_id: String,
    stream_id: String,
    chunk: Option<String>,
    done: bool,
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PluginBinaryAsset {
    pub mime: String,
    pub base64: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PluginInstanceFileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
}

async fn plugins_dir() -> Result<PathBuf> {
    let state = theseus::State::get().await?;
    let dir = state.directories.settings_dir.join("plugins");
    tokio::fs::create_dir_all(&dir).await?;
    Ok(dir)
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write;
        let _ = write!(&mut out, "{byte:02x}");
    }
    out
}

fn is_valid_plugin_id(id: &str) -> bool {
    let bytes = id.as_bytes();
    if bytes.len() < 3 || bytes.len() > 64 {
        return false;
    }
    if bytes[0] == b'-' || bytes[bytes.len() - 1] == b'-' {
        return false;
    }
    bytes
        .iter()
        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'-')
}

fn validate_manifest(manifest: &PluginManifest) -> Result<()> {
    if !is_valid_plugin_id(&manifest.id) {
        return Err(theseus::ErrorKind::InputError(
            "Invalid plugin id in plugin.json".to_string(),
        )
        .into());
    }
    if manifest.api_version != SUPPORTED_API_VERSION {
        return Err(theseus::ErrorKind::InputError(format!(
            "Unsupported plugin apiVersion {} (expected {SUPPORTED_API_VERSION})",
            manifest.api_version
        ))
        .into());
    }
    if !matches!(
        manifest.plugin_type.as_str(),
        "theme" | "content-source" | "extension"
    ) {
        return Err(theseus::ErrorKind::InputError(format!(
            "Unsupported plugin type '{}'",
            manifest.plugin_type
        ))
        .into());
    }
    if manifest.name.trim().is_empty() || manifest.version.trim().is_empty() {
        return Err(theseus::ErrorKind::InputError(
            "Plugin manifest must include name and version".to_string(),
        )
        .into());
    }
    Ok(())
}

fn sanitize_relative_path(raw: &str) -> Option<PathBuf> {
    let path = Path::new(raw);
    let mut clean = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => clean.push(part),
            Component::CurDir => {}
            _ => return None,
        }
    }
    if clean.as_os_str().is_empty() {
        None
    } else {
        Some(clean)
    }
}

async fn load_state(dir: &Path) -> PluginsStateFile {
    let path = dir.join("state.json");
    let Ok(raw) = tokio::fs::read_to_string(&path).await else {
        return PluginsStateFile::default();
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

async fn save_state(dir: &Path, state: &PluginsStateFile) -> Result<()> {
    let path = dir.join("state.json");
    let json = serde_json::to_string_pretty(state)?;
    tokio::fs::write(&path, json).await?;
    Ok(())
}

fn extract_plugin_archive(
    archive_bytes: Vec<u8>,
    plugins_root: PathBuf,
) -> Result<PluginManifest> {
    if archive_bytes.len() > MAX_ARCHIVE_BYTES {
        return Err(theseus::ErrorKind::InputError(
            "Plugin archive exceeds 25 MB limit".to_string(),
        )
        .into());
    }

    let cursor = Cursor::new(archive_bytes);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|e| {
        theseus::ErrorKind::InputError(format!("Invalid plugin zip: {e}"))
    })?;

    // Locate plugin.json at archive root or inside a single top-level directory.
    let mut manifest_prefix = String::new();
    let mut manifest_raw: Option<String> = None;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| {
            theseus::ErrorKind::InputError(format!("Zip read error: {e}"))
        })?;
        let name = entry
            .name()
            .map_err(|e| theseus::ErrorKind::InputError(format!("Zip name error: {e}")))?
            .replace('\\', "/");
        if name == "plugin.json" {
            let mut buf = String::new();
            entry.read_to_string(&mut buf)?;
            manifest_prefix.clear();
            manifest_raw = Some(buf);
            break;
        }
        if let Some(prefix) = name.strip_suffix("/plugin.json")
            && !prefix.contains('/')
            && manifest_raw.is_none()
        {
            let mut buf = String::new();
            entry.read_to_string(&mut buf)?;
            manifest_prefix = format!("{prefix}/");
            manifest_raw = Some(buf);
        }
    }

    let manifest_text = manifest_raw.ok_or_else(|| {
        theseus::ErrorKind::InputError(
            "Archive does not contain plugin.json".to_string(),
        )
    })?;
    let manifest: PluginManifest =
        serde_json::from_str(&manifest_text).map_err(|e| {
            theseus::ErrorKind::InputError(format!("Invalid plugin.json: {e}"))
        })?;
    validate_manifest(&manifest)?;

    let target_dir = plugins_root.join(&manifest.id);
    if target_dir.exists() {
        std::fs::remove_dir_all(&target_dir)?;
    }
    std::fs::create_dir_all(&target_dir)?;

    let mut total_written: usize = 0;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| {
            theseus::ErrorKind::InputError(format!("Zip read error: {e}"))
        })?;
        let raw_name = entry
            .name()
            .map_err(|e| theseus::ErrorKind::InputError(format!("Zip name error: {e}")))?
            .replace('\\', "/");
        let stripped = if !manifest_prefix.is_empty() {
            match raw_name.strip_prefix(&manifest_prefix) {
                Some(rest) => rest,
                None => continue,
            }
        } else {
            raw_name.as_str()
        };
        if stripped.is_empty() || stripped.ends_with('/') {
            continue;
        }
        let Some(rel) = sanitize_relative_path(stripped) else {
            return Err(theseus::ErrorKind::InputError(
                "Plugin archive contains unsafe path".to_string(),
            )
            .into());
        };
        let out_path = target_dir.join(&rel);
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut data = Vec::new();
        entry.read_to_end(&mut data)?;
        total_written = total_written.saturating_add(data.len());
        if total_written > MAX_ARCHIVE_BYTES * 2 {
            let _ = std::fs::remove_dir_all(&target_dir);
            return Err(theseus::ErrorKind::InputError(
                "Extracted plugin exceeds size limit".to_string(),
            )
            .into());
        }
        std::fs::write(&out_path, &data)?;
    }

    Ok(manifest)
}

async fn install_archive_bytes(
    bytes: Vec<u8>,
    source: &str,
    expected_sha256: Option<&str>,
) -> Result<InstalledPlugin> {
    let actual_sha = sha256_hex(&bytes);
    if let Some(expected) = expected_sha256 {
        let expected_clean = expected.trim().to_ascii_lowercase();
        if expected_clean.is_empty() || actual_sha != expected_clean {
            return Err(theseus::ErrorKind::InputError(format!(
                "SHA-256 checksum mismatch (expected {expected_clean}, got {actual_sha})"
            ))
            .into());
        }
    }

    let root = plugins_dir().await?;
    let root_for_task = root.clone();
    let manifest = tokio::task::spawn_blocking(move || {
        extract_plugin_archive(bytes, root_for_task)
    })
    .await
    .map_err(|e| {
        theseus::ErrorKind::InputError(format!("Extract task failed: {e}"))
    })??;

    let mut state = load_state(&root).await;
    let installed_at = now_secs();
    let entry = PluginStateEntry {
        enabled: true,
        source: source.to_string(),
        sha256: actual_sha.clone(),
        installed_at,
    };
    state.plugins.insert(manifest.id.clone(), entry);
    save_state(&root, &state).await?;

    Ok(InstalledPlugin {
        manifest,
        enabled: true,
        source: source.to_string(),
        sha256: actual_sha,
        installed_at,
    })
}

#[tauri::command]
pub async fn plugins_list() -> Result<Vec<InstalledPlugin>> {
    let root = plugins_dir().await?;
    let state = load_state(&root).await;

    let mut entries = tokio::fs::read_dir(&root).await?;
    let mut installed = Vec::new();
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let manifest_path = path.join("plugin.json");
        let Ok(raw) = tokio::fs::read_to_string(&manifest_path).await else {
            continue;
        };
        let Ok(manifest) = serde_json::from_str::<PluginManifest>(&raw) else {
            continue;
        };
        if validate_manifest(&manifest).is_err() {
            continue;
        }
        let meta = state.plugins.get(&manifest.id);
        installed.push(InstalledPlugin {
            enabled: meta.map_or(false, |m| m.enabled),
            source: meta.map_or_else(|| "local".to_string(), |m| m.source.clone()),
            sha256: meta.map_or_else(String::new, |m| m.sha256.clone()),
            installed_at: meta.map_or(0, |m| m.installed_at),
            manifest,
        });
    }
    installed.sort_by(|a, b| a.manifest.name.cmp(&b.manifest.name));
    Ok(installed)
}

#[tauri::command]
pub async fn plugins_install_zip(zip_path: String) -> Result<InstalledPlugin> {
    let bytes = tokio::fs::read(&zip_path).await?;
    install_archive_bytes(bytes, "local", None).await
}

#[tauri::command]
pub async fn plugins_install_remote(
    url: String,
    expected_sha256: String,
) -> Result<InstalledPlugin> {
    let parsed = url::Url::parse(&url).map_err(|e| {
        theseus::ErrorKind::InputError(format!("Invalid plugin URL: {e}"))
    })?;
    if parsed.scheme() != "https" {
        return Err(theseus::ErrorKind::InputError(
            "Catalog plugins must be downloaded over HTTPS".to_string(),
        )
        .into());
    }

    let client = tauri_plugin_http::reqwest::Client::builder()
        .user_agent("Noctrinth-Plugins/1")
        .build()
        .map_err(|e| theseus::ErrorKind::InputError(e.to_string()))?;
    let resp = client
        .get(parsed)
        .send()
        .await
        .map_err(|e| theseus::ErrorKind::InputError(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(theseus::ErrorKind::InputError(format!(
            "Failed to download plugin: HTTP {}",
            resp.status()
        ))
        .into());
    }
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| theseus::ErrorKind::InputError(e.to_string()))?
        .to_vec();

    install_archive_bytes(bytes, "catalog", Some(&expected_sha256)).await
}

#[tauri::command]
pub async fn plugins_set_enabled(id: String, enabled: bool) -> Result<()> {
    if !is_valid_plugin_id(&id) {
        return Err(theseus::ErrorKind::InputError("Invalid plugin id".into()).into());
    }
    let root = plugins_dir().await?;
    let mut state = load_state(&root).await;
    if let Some(entry) = state.plugins.get_mut(&id) {
        entry.enabled = enabled;
    } else {
        state.plugins.insert(
            id,
            PluginStateEntry {
                enabled,
                source: "local".to_string(),
                sha256: String::new(),
                installed_at: now_secs(),
            },
        );
    }
    save_state(&root, &state).await
}

#[tauri::command]
pub async fn plugins_remove(id: String) -> Result<()> {
    if !is_valid_plugin_id(&id) {
        return Err(theseus::ErrorKind::InputError("Invalid plugin id".into()).into());
    }
    let root = plugins_dir().await?;
    let target = root.join(&id);
    if target.is_dir() {
        tokio::fs::remove_dir_all(&target).await?;
    }
    let mut state = load_state(&root).await;
    state.plugins.remove(&id);
    save_state(&root, &state).await
}

#[tauri::command]
pub async fn plugins_read_asset(id: String, rel_path: String) -> Result<String> {
    if !is_valid_plugin_id(&id) {
        return Err(theseus::ErrorKind::InputError("Invalid plugin id".into()).into());
    }
    let Some(clean_rel) = sanitize_relative_path(&rel_path) else {
        return Err(theseus::ErrorKind::InputError("Unsafe asset path".into()).into());
    };
    let root = plugins_dir().await?;
    let plugin_root = root.join(&id);
    let target = plugin_root.join(&clean_rel);

    let canon_root = tokio::fs::canonicalize(&plugin_root).await?;
    let canon_target = tokio::fs::canonicalize(&target).await?;
    if !canon_target.starts_with(&canon_root) {
        return Err(theseus::ErrorKind::InputError("Asset outside plugin dir".into()).into());
    }
    let meta = tokio::fs::metadata(&canon_target).await?;
    if meta.len() as usize > MAX_ASSET_BYTES {
        return Err(theseus::ErrorKind::InputError("Asset exceeds size limit".into()).into());
    }
    Ok(tokio::fs::read_to_string(&canon_target).await?)
}

fn domain_allowed(host: &str, allowed: &[String]) -> bool {
    let host_lc = host.to_ascii_lowercase();
    allowed.iter().any(|pattern| {
        let pat = pattern.trim().to_ascii_lowercase();
        if let Some(suffix) = pat.strip_prefix("*.") {
            host_lc == suffix || host_lc.ends_with(&format!(".{suffix}"))
        } else {
            host_lc == pat
        }
    })
}

#[tauri::command]
pub async fn plugins_http_fetch(
    id: String,
    url: String,
    method: Option<String>,
    headers: Option<HashMap<String, String>>,
    body: Option<String>,
) -> Result<PluginHttpResponse> {
    if !is_valid_plugin_id(&id) {
        return Err(theseus::ErrorKind::InputError("Invalid plugin id".into()).into());
    }
    let root = plugins_dir().await?;
    let state = load_state(&root).await;
    if !state.plugins.get(&id).is_some_and(|e| e.enabled) {
        return Err(theseus::ErrorKind::InputError("Plugin is not enabled".into()).into());
    }
    let manifest_raw = tokio::fs::read_to_string(root.join(&id).join("plugin.json")).await?;
    let manifest: PluginManifest = serde_json::from_str(&manifest_raw)?;

    let parsed = url::Url::parse(&url).map_err(|e| {
        theseus::ErrorKind::InputError(format!("Invalid URL: {e}"))
    })?;
    let host = parsed.host_str().unwrap_or_default();
    let is_loopback = matches!(
        host.to_ascii_lowercase().as_str(),
        "localhost" | "127.0.0.1" | "::1" | "[::1]"
    );
    if parsed.scheme() != "https" && !(parsed.scheme() == "http" && is_loopback) {
        return Err(theseus::ErrorKind::InputError(
            "Plugin fetch only supports HTTPS URLs (or HTTP on localhost)".into(),
        )
        .into());
    }
    if !domain_allowed(host, &manifest.permissions.domains) {
        return Err(theseus::ErrorKind::InputError(format!(
            "Plugin '{id}' does not have permission to access '{host}'"
        ))
        .into());
    }

    let method = match method.as_deref().unwrap_or("GET").to_ascii_uppercase().as_str() {
        "GET" => tauri_plugin_http::reqwest::Method::GET,
        "POST" => tauri_plugin_http::reqwest::Method::POST,
        "PUT" => tauri_plugin_http::reqwest::Method::PUT,
        "DELETE" => tauri_plugin_http::reqwest::Method::DELETE,
        "HEAD" => tauri_plugin_http::reqwest::Method::HEAD,
        other => {
            return Err(theseus::ErrorKind::InputError(format!(
                "Unsupported HTTP method '{other}'"
            ))
            .into());
        }
    };

    let client = tauri_plugin_http::reqwest::Client::builder()
        .user_agent(format!("Noctrinth-Plugin/{}", manifest.id))
        .build()
        .map_err(|e| theseus::ErrorKind::InputError(e.to_string()))?;
    let mut req = client.request(method, parsed);
    if let Some(hdrs) = headers {
        for (k, v) in hdrs {
            req = req.header(k, v);
        }
    }
    if let Some(b) = body {
        req = req.body(b);
    }
    let resp = req
        .send()
        .await
        .map_err(|e| theseus::ErrorKind::InputError(e.to_string()))?;
    let status = resp.status().as_u16();
    let ok = resp.status().is_success();
    let mut resp_headers = HashMap::new();
    for (k, v) in resp.headers() {
        if let Ok(val) = v.to_str() {
            resp_headers.insert(k.as_str().to_string(), val.to_string());
        }
    }
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| theseus::ErrorKind::InputError(e.to_string()))?;
    if bytes.len() > MAX_RESPONSE_BYTES {
        return Err(theseus::ErrorKind::InputError(
            "Plugin HTTP response exceeds 4 MB limit".into(),
        )
        .into());
    }
    Ok(PluginHttpResponse {
        status,
        ok,
        headers: resp_headers,
        body: String::from_utf8_lossy(&bytes).into_owned(),
    })
}

#[tauri::command]
pub async fn plugins_http_fetch_stream<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    id: String,
    stream_id: String,
    url: String,
    method: Option<String>,
    headers: Option<HashMap<String, String>>,
    body: Option<String>,
) -> Result<PluginStreamHead> {
    if !is_valid_plugin_id(&id) {
        return Err(theseus::ErrorKind::InputError("Invalid plugin id".into()).into());
    }
    let root = plugins_dir().await?;
    let state = load_state(&root).await;
    if !state.plugins.get(&id).is_some_and(|e| e.enabled) {
        return Err(theseus::ErrorKind::InputError("Plugin is not enabled".into()).into());
    }
    let manifest_raw = tokio::fs::read_to_string(root.join(&id).join("plugin.json")).await?;
    let manifest: PluginManifest = serde_json::from_str(&manifest_raw)?;

    let parsed = url::Url::parse(&url).map_err(|e| {
        theseus::ErrorKind::InputError(format!("Invalid URL: {e}"))
    })?;
    let host = parsed.host_str().unwrap_or_default();
    let is_loopback = matches!(
        host.to_ascii_lowercase().as_str(),
        "localhost" | "127.0.0.1" | "::1" | "[::1]"
    );
    if parsed.scheme() != "https" && !(parsed.scheme() == "http" && is_loopback) {
        return Err(theseus::ErrorKind::InputError(
            "Plugin fetch only supports HTTPS URLs (or HTTP on localhost)".into(),
        )
        .into());
    }
    if !domain_allowed(host, &manifest.permissions.domains) {
        return Err(theseus::ErrorKind::InputError(format!(
            "Plugin '{id}' does not have permission to access '{host}'"
        ))
        .into());
    }

    let method = match method.as_deref().unwrap_or("GET").to_ascii_uppercase().as_str() {
        "GET" => tauri_plugin_http::reqwest::Method::GET,
        "POST" => tauri_plugin_http::reqwest::Method::POST,
        "PUT" => tauri_plugin_http::reqwest::Method::PUT,
        "DELETE" => tauri_plugin_http::reqwest::Method::DELETE,
        "HEAD" => tauri_plugin_http::reqwest::Method::HEAD,
        other => {
            return Err(theseus::ErrorKind::InputError(format!(
                "Unsupported HTTP method '{other}'"
            ))
            .into());
        }
    };

    let client = tauri_plugin_http::reqwest::Client::builder()
        .user_agent(format!("Noctrinth-Plugin/{}", manifest.id))
        .build()
        .map_err(|e| theseus::ErrorKind::InputError(e.to_string()))?;
    let mut req = client.request(method, parsed);
    if let Some(hdrs) = headers {
        for (k, v) in hdrs {
            req = req.header(k, v);
        }
    }
    if let Some(b) = body {
        req = req.body(b);
    }
    let mut resp = req
        .send()
        .await
        .map_err(|e| theseus::ErrorKind::InputError(e.to_string()))?;

    let status = resp.status().as_u16();
    let ok = resp.status().is_success();
    let mut resp_headers = HashMap::new();
    for (k, v) in resp.headers() {
        if let Ok(val) = v.to_str() {
            resp_headers.insert(k.as_str().to_string(), val.to_string());
        }
    }

    let plugin_id = id.clone();
    tokio::spawn(async move {
        let mut total_bytes: usize = 0;
        loop {
            match resp.chunk().await {
                Ok(Some(bytes)) => {
                    total_bytes = total_bytes.saturating_add(bytes.len());
                    if total_bytes > MAX_STREAM_BYTES {
                        let _ = app.emit(
                            "noctrinth-plugin-stream",
                            PluginStreamChunkEvent {
                                plugin_id: plugin_id.clone(),
                                stream_id: stream_id.clone(),
                                chunk: None,
                                done: true,
                                error: Some("Plugin HTTP stream exceeds 8 MB limit".into()),
                            },
                        );
                        break;
                    }
                    let text = String::from_utf8_lossy(&bytes).into_owned();
                    let _ = app.emit(
                        "noctrinth-plugin-stream",
                        PluginStreamChunkEvent {
                            plugin_id: plugin_id.clone(),
                            stream_id: stream_id.clone(),
                            chunk: Some(text),
                            done: false,
                            error: None,
                        },
                    );
                }
                Ok(None) => {
                    let _ = app.emit(
                        "noctrinth-plugin-stream",
                        PluginStreamChunkEvent {
                            plugin_id: plugin_id.clone(),
                            stream_id: stream_id.clone(),
                            chunk: None,
                            done: true,
                            error: None,
                        },
                    );
                    break;
                }
                Err(err) => {
                    let _ = app.emit(
                        "noctrinth-plugin-stream",
                        PluginStreamChunkEvent {
                            plugin_id: plugin_id.clone(),
                            stream_id: stream_id.clone(),
                            chunk: None,
                            done: true,
                            error: Some(err.to_string()),
                        },
                    );
                    break;
                }
            }
        }
    });

    Ok(PluginStreamHead {
        status,
        ok,
        headers: resp_headers,
    })
}

#[tauri::command]
pub async fn plugins_fetch_catalog(catalog_url: String) -> Result<String> {
    let parsed = url::Url::parse(&catalog_url).map_err(|e| {
        theseus::ErrorKind::InputError(format!("Invalid catalog URL: {e}"))
    })?;
    if parsed.scheme() != "https" {
        return Err(theseus::ErrorKind::InputError(
            "Catalog URL must use HTTPS".into(),
        )
        .into());
    }
    let client = tauri_plugin_http::reqwest::Client::builder()
        .user_agent("Noctrinth-Plugins/1")
        .build()
        .map_err(|e| theseus::ErrorKind::InputError(e.to_string()))?;
    let resp = client
        .get(parsed)
        .send()
        .await
        .map_err(|e| theseus::ErrorKind::InputError(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(theseus::ErrorKind::InputError(format!(
            "Failed to load catalog: HTTP {}",
            resp.status()
        ))
        .into());
    }
    let text = resp
        .text()
        .await
        .map_err(|e| theseus::ErrorKind::InputError(e.to_string()))?;
    Ok(text)
}

#[tauri::command]
pub async fn plugins_get_dir() -> Result<String> {
    let dir = plugins_dir().await?;
    Ok(dir.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn plugins_is_safe_mode() -> bool {
    std::env::args().any(|arg| arg == "--safe-mode" || arg == "--no-plugins")
}

fn asset_mime_for_path(path: &Path) -> Option<&'static str> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    match ext.as_str() {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "webp" => Some("image/webp"),
        "gif" => Some("image/gif"),
        "avif" => Some("image/avif"),
        "svg" => Some("image/svg+xml"),
        "woff2" => Some("font/woff2"),
        "woff" => Some("font/woff"),
        "ttf" => Some("font/ttf"),
        "otf" => Some("font/otf"),
        _ => None,
    }
}

#[tauri::command]
pub async fn plugins_read_asset_base64(
    id: String,
    rel_path: String,
) -> Result<PluginBinaryAsset> {
    if !is_valid_plugin_id(&id) {
        return Err(theseus::ErrorKind::InputError("Invalid plugin id".into()).into());
    }
    let Some(clean_rel) = sanitize_relative_path(&rel_path) else {
        return Err(theseus::ErrorKind::InputError("Unsafe asset path".into()).into());
    };
    let Some(mime) = asset_mime_for_path(&clean_rel) else {
        return Err(theseus::ErrorKind::InputError("Unsupported binary asset type".into()).into());
    };
    let root = plugins_dir().await?;
    let plugin_root = root.join(&id);
    let target = plugin_root.join(&clean_rel);

    let canon_root = tokio::fs::canonicalize(&plugin_root).await?;
    let canon_target = tokio::fs::canonicalize(&target).await?;
    if !canon_target.starts_with(&canon_root) {
        return Err(theseus::ErrorKind::InputError("Asset outside plugin dir".into()).into());
    }
    let meta = tokio::fs::metadata(&canon_target).await?;
    if meta.len() as usize > MAX_BINARY_ASSET_BYTES {
        return Err(
            theseus::ErrorKind::InputError("Binary asset exceeds 4 MB limit".into()).into(),
        );
    }
    let bytes = tokio::fs::read(&canon_target).await?;
    Ok(PluginBinaryAsset {
        mime: mime.to_string(),
        base64: base64::engine::general_purpose::STANDARD.encode(bytes),
    })
}

async fn require_enabled_plugin_instance_perm(
    id: &str,
    required_perm: &str,
) -> Result<()> {
    if !is_valid_plugin_id(id) {
        return Err(theseus::ErrorKind::InputError("Invalid plugin id".into()).into());
    }
    let root = plugins_dir().await?;
    let state = load_state(&root).await;
    if !state.plugins.get(id).is_some_and(|e| e.enabled) {
        return Err(theseus::ErrorKind::InputError("Plugin is not enabled".into()).into());
    }
    let manifest_raw = tokio::fs::read_to_string(root.join(id).join("plugin.json")).await?;
    let manifest: PluginManifest = serde_json::from_str(&manifest_raw)?;
    if !manifest
        .permissions
        .instances
        .iter()
        .any(|p| p == required_perm)
    {
        return Err(theseus::ErrorKind::InputError(format!(
            "Plugin '{id}' lacks permissions.instances.{required_perm}"
        ))
        .into());
    }
    Ok(())
}

fn is_forbidden_write_extension(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
        return false;
    };
    matches!(
        ext.to_ascii_lowercase().as_str(),
        "exe"
            | "bat"
            | "cmd"
            | "ps1"
            | "sh"
            | "dll"
            | "so"
            | "dylib"
            | "jar"
            | "class"
            | "vbs"
            | "msi"
            | "appimage"
            | "com"
            | "scr"
    )
}

#[tauri::command]
pub async fn plugins_instance_list_files(
    id: String,
    instance_id: String,
    rel_dir: Option<String>,
) -> Result<Vec<PluginInstanceFileEntry>> {
    require_enabled_plugin_instance_perm(&id, "read").await?;
    let instance_root = theseus::instance::get_full_path(&instance_id).await?;
    let canon_root = tokio::fs::canonicalize(&instance_root).await?;

    let (target_dir, prefix) = match rel_dir.as_deref().map(str::trim) {
        None | Some("") | Some(".") => (canon_root.clone(), String::new()),
        Some(raw) => {
            let Some(clean) = sanitize_relative_path(raw) else {
                return Err(
                    theseus::ErrorKind::InputError("Unsafe instance directory path".into()).into(),
                );
            };
            let candidate = canon_root.join(&clean);
            let canon_target = tokio::fs::canonicalize(&candidate).await?;
            if !canon_target.starts_with(&canon_root) {
                return Err(
                    theseus::ErrorKind::InputError("Directory outside instance folder".into())
                        .into(),
                );
            }
            (
                canon_target,
                format!("{}/", clean.to_string_lossy().replace('\\', "/")),
            )
        }
    };

    let mut entries = tokio::fs::read_dir(&target_dir).await?;
    let mut out = Vec::new();
    while let Some(entry) = entries.next_entry().await? {
        let name = entry.file_name().to_string_lossy().into_owned();
        let meta = entry.metadata().await?;
        out.push(PluginInstanceFileEntry {
            path: format!("{prefix}{name}"),
            name,
            is_dir: meta.is_dir(),
            size: if meta.is_dir() { 0 } else { meta.len() },
        });
    }
    out.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then_with(|| a.name.cmp(&b.name)));
    Ok(out)
}

#[tauri::command]
pub async fn plugins_instance_read_file(
    id: String,
    instance_id: String,
    rel_path: String,
) -> Result<String> {
    require_enabled_plugin_instance_perm(&id, "read").await?;
    let Some(clean) = sanitize_relative_path(&rel_path) else {
        return Err(theseus::ErrorKind::InputError("Unsafe instance file path".into()).into());
    };
    let instance_root = theseus::instance::get_full_path(&instance_id).await?;
    let canon_root = tokio::fs::canonicalize(&instance_root).await?;
    let canon_target = tokio::fs::canonicalize(canon_root.join(&clean)).await?;
    if !canon_target.starts_with(&canon_root) {
        return Err(theseus::ErrorKind::InputError("File outside instance folder".into()).into());
    }
    let meta = tokio::fs::metadata(&canon_target).await?;
    if meta.len() as usize > MAX_INSTANCE_FILE_BYTES {
        return Err(
            theseus::ErrorKind::InputError("Instance file exceeds 1 MB read limit".into()).into(),
        );
    }
    Ok(tokio::fs::read_to_string(&canon_target).await?)
}

#[tauri::command]
pub async fn plugins_instance_write_file(
    id: String,
    instance_id: String,
    rel_path: String,
    content: String,
) -> Result<()> {
    require_enabled_plugin_instance_perm(&id, "write").await?;
    if content.len() > MAX_INSTANCE_FILE_BYTES {
        return Err(
            theseus::ErrorKind::InputError("Instance file exceeds 1 MB write limit".into()).into(),
        );
    }
    let Some(clean) = sanitize_relative_path(&rel_path) else {
        return Err(theseus::ErrorKind::InputError("Unsafe instance file path".into()).into());
    };
    if is_forbidden_write_extension(&clean) {
        return Err(theseus::ErrorKind::InputError(
            "Writing executable or archive files is not allowed for plugins".into(),
        )
        .into());
    }
    let instance_root = theseus::instance::get_full_path(&instance_id).await?;
    let canon_root = tokio::fs::canonicalize(&instance_root).await?;
    let target = canon_root.join(&clean);
    if let Some(parent) = target.parent() {
        tokio::fs::create_dir_all(parent).await?;
        let canon_parent = tokio::fs::canonicalize(parent).await?;
        if !canon_parent.starts_with(&canon_root) {
            return Err(
                theseus::ErrorKind::InputError("Target outside instance folder".into()).into(),
            );
        }
    }
    tokio::fs::write(&target, content).await?;
    Ok(())
}
