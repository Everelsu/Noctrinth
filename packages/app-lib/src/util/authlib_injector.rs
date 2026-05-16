//! Downloads and caches the authlib-injector Java agent.
//!
//! authlib-injector is required to launch Minecraft with non-Microsoft
//! accounts (such as Ely.by): it patches the game's authentication so it
//! talks to the alternative auth server instead of Mojang's.

use std::path::PathBuf;

use serde::Deserialize;

use crate::state::DirectoryInfo;
use crate::util::fetch::REQWEST_CLIENT;
use crate::util::io;

/// Official authlib-injector distribution metadata endpoint.
const AUTHLIB_INJECTOR_LATEST_URL: &str =
    "https://authlib-injector.yushi.moe/artifact/latest.json";

#[derive(Deserialize)]
struct LatestArtifact {
    download_url: String,
}

/// Ensures the authlib-injector jar is available locally and returns its path.
///
/// The jar is cached under `<caches>/authlib-injector/authlib-injector.jar`.
/// If a cached copy already exists it is reused as-is (the agent is stable
/// across versions), so launches keep working offline once it's downloaded.
pub async fn get_authlib_injector(
    directories: &DirectoryInfo,
) -> crate::Result<PathBuf> {
    let dir = directories.caches_dir().join("authlib-injector");
    io::create_dir_all(&dir).await?;

    let jar_path = dir.join("authlib-injector.jar");

    // Reuse the cached jar if present.
    if io::metadata(&jar_path).await.is_ok() {
        return Ok(jar_path);
    }

    tracing::info!("Downloading authlib-injector for alternative-account launch");

    // Resolve the latest released artifact.
    let latest = REQWEST_CLIENT
        .get(AUTHLIB_INJECTOR_LATEST_URL)
        .send()
        .await
        .map_err(|e| {
            crate::ErrorKind::OtherError(format!(
                "Failed to reach the authlib-injector distribution: {e}"
            ))
        })?
        .json::<LatestArtifact>()
        .await
        .map_err(|e| {
            crate::ErrorKind::OtherError(format!(
                "Failed to parse authlib-injector metadata: {e}"
            ))
        })?;

    // Download the jar.
    let bytes = REQWEST_CLIENT
        .get(&latest.download_url)
        .send()
        .await
        .map_err(|e| {
            crate::ErrorKind::OtherError(format!(
                "Failed to download authlib-injector: {e}"
            ))
        })?
        .bytes()
        .await
        .map_err(|e| {
            crate::ErrorKind::OtherError(format!(
                "Failed to read the authlib-injector download: {e}"
            ))
        })?;

    io::write(&jar_path, &bytes).await?;

    Ok(jar_path)
}
