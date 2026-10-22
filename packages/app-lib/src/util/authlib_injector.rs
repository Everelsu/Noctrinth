//! Downloads and caches the authlib-injector Java agent.
//!
//! authlib-injector is required to launch Minecraft with non-Microsoft
//! accounts (such as Ely.by): it patches the game's authentication so it
//! talks to the alternative auth server instead of Mojang's.

use std::path::PathBuf;
use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::state::DirectoryInfo;
use crate::util::fetch::REQWEST_CLIENT;
use crate::util::io;

/// Official authlib-injector distribution metadata endpoint.
const AUTHLIB_INJECTOR_LATEST_URL: &str =
    "https://authlib-injector.yushi.moe/artifact/latest.json";

/// How long a downloaded agent goes before the release list is looked at again.
const AGENT_CHECK_DAYS: i64 = 1;

#[derive(Deserialize)]
struct LatestArtifact {
    version: String,
    download_url: String,
    checksums: Checksums,
}

#[derive(Deserialize)]
struct Checksums {
    sha256: String,
}

/// Which release the jar on disk is, and when that was last checked.
#[derive(Serialize, Deserialize)]
struct AgentRecord {
    version: String,
    checked_at: DateTime<Utc>,
}

/// Ensures the authlib-injector jar is available locally and returns its path.
///
/// The jar is cached under `<caches>/authlib-injector/authlib-injector.jar`, so
/// launches keep working offline once it is there. Ely.by tells launchers to
/// use the latest release, and a new one is how support for a new Minecraft
/// version arrives, so a jar checked longer ago than [`AGENT_CHECK_DAYS`] is
/// compared with the release list behind the launch's back: the launch never
/// waits on it, and the next one gets the new jar.
pub async fn get_authlib_injector(
    directories: &DirectoryInfo,
) -> crate::Result<PathBuf> {
    let dir = directories.caches_dir().join("authlib-injector");
    io::create_dir_all(&dir).await?;

    let jar_path = dir.join("authlib-injector.jar");
    let record_path = dir.join("authlib-injector.json");

    if io::metadata(&jar_path).await.is_ok() {
        let record = io::read(&record_path).await.ok().and_then(|bytes| {
            serde_json::from_slice::<AgentRecord>(&bytes).ok()
        });
        let due = record.as_ref().is_none_or(|record| {
            (Utc::now() - record.checked_at).num_days() >= AGENT_CHECK_DAYS
        });
        if due {
            let known = record.map(|record| record.version);
            tokio::spawn(async move {
                if let Err(error) =
                    update_agent(&jar_path, &record_path, known.as_deref())
                        .await
                {
                    tracing::debug!(
                        "Could not check for a newer authlib-injector: {error}"
                    );
                }
            });
        }
        return Ok(dir.join("authlib-injector.jar"));
    }

    tracing::info!(
        "Downloading authlib-injector for alternative-account launch"
    );
    update_agent(&jar_path, &record_path, None).await?;
    Ok(jar_path)
}

/// Downloads the latest agent over `jar_path` unless it is already `known`.
///
/// The download is checked against the checksum the release list publishes
/// before it is put in place: a jar that arrived cut short or altered would
/// otherwise go straight into the game as a Java agent.
async fn update_agent(
    jar_path: &std::path::Path,
    record_path: &std::path::Path,
    known: Option<&str>,
) -> crate::Result<()> {
    use sha2::Digest;

    let latest = REQWEST_CLIENT
        .get(AUTHLIB_INJECTOR_LATEST_URL)
        .timeout(Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| {
            anyhow::anyhow!(format!(
                "Failed to reach the authlib-injector distribution: {e}"
            ))
        })?
        .json::<LatestArtifact>()
        .await
        .map_err(|e| {
            anyhow::anyhow!(format!(
                "Failed to parse authlib-injector metadata: {e}"
            ))
        })?;

    // A jar from before its release was written down is checked by its bytes,
    // so it is not downloaded again only to be the same file.
    let already_current = known == Some(latest.version.as_str())
        || io::read(jar_path).await.is_ok_and(|bytes| {
            format!("{:x}", sha2::Sha256::digest(&bytes))
                .eq_ignore_ascii_case(latest.checksums.sha256.trim())
        });

    if !already_current {
        let bytes = REQWEST_CLIENT
            .get(&latest.download_url)
            .send()
            .await
            .map_err(|e| {
                anyhow::anyhow!(format!(
                    "Failed to download authlib-injector: {e}"
                ))
            })?
            .bytes()
            .await
            .map_err(|e| {
                anyhow::anyhow!(format!(
                    "Failed to read the authlib-injector download: {e}"
                ))
            })?;

        let digest = format!("{:x}", sha2::Sha256::digest(&bytes));
        if !digest.eq_ignore_ascii_case(latest.checksums.sha256.trim()) {
            return Err(anyhow::anyhow!(format!(
                "The authlib-injector {} download does not match its checksum",
                latest.version
            )));
        }

        // Next to it first, so a game holding the old jar open on Windows
        // leaves that one in place rather than half of the new one.
        let partial = jar_path.with_extension("jar.part");
        io::write(&partial, &bytes).await?;
        io::rename_or_move(&partial, jar_path).await?;
        tracing::info!("Using authlib-injector {}", latest.version);
    }

    let record = AgentRecord {
        version: latest.version,
        checked_at: Utc::now(),
    };
    io::write(record_path, serde_json::to_vec(&record)?).await?;
    Ok(())
}

/// Where an account system's API lives, and what it says about itself.
///
/// The agent works this out for itself at startup, which costs the game two
/// requests before it has drawn anything: one to the address it was given, to
/// be told where the API really is, and one to read the metadata there. Handing
/// it both means it asks nobody — the launch stops waiting on somebody else's
/// server, and stops needing one at all.
pub struct InjectorProfile {
    /// What to write after `=` in the agent argument.
    pub api_root: String,
    /// The metadata, encoded for `authlibinjector.yggdrasil.prefetched`.
    ///
    /// `None` when it could not be had, and the agent is then left to do what
    /// it always did.
    pub prefetched: Option<String>,
}

/// The same, as it is kept on disk between runs.
#[derive(Serialize, Deserialize)]
struct CachedProfile {
    api_root: String,
    metadata: String,
    fetched_at: DateTime<Utc>,
}

/// How long a cached answer is current for.
///
/// Past this it is still handed over — a stale answer beats a launch that waits
/// on somebody else's server — and looked at again behind the launch's back, so
/// the next one has a fresh one.
const PROFILE_TTL_DAYS: i64 = 1;

/// Long enough for a slow server, short enough not to hold a launch hostage.
const PROFILE_TIMEOUT: Duration = Duration::from_secs(5);

/// Resolves what the agent needs to know about `server`, from cache where possible.
pub async fn get_injector_profile(
    directories: &DirectoryInfo,
    server: &str,
) -> InjectorProfile {
    let path = directories
        .caches_dir()
        .join("authlib-injector")
        .join(format!("{}.json", server.replace(['/', ':', '\\'], "_")));

    let cached = io::read(&path)
        .await
        .ok()
        .and_then(|bytes| serde_json::from_slice::<CachedProfile>(&bytes).ok());

    if let Some(cached) = &cached {
        if (Utc::now() - cached.fetched_at).num_days() >= PROFILE_TTL_DAYS {
            // Old enough to check again, not old enough to make anyone wait:
            // whatever comes back is for the launch after this one.
            let server = server.to_string();
            let path = path.clone();
            tokio::spawn(async move {
                if let Ok(fresh) = fetch_profile(&server).await {
                    let _ = store_profile(&path, &fresh).await;
                }
            });
        }

        return InjectorProfile {
            api_root: cached.api_root.clone(),
            prefetched: Some(BASE64.encode(&cached.metadata)),
        };
    }

    match fetch_profile(server).await {
        Ok(fresh) => {
            let encoded = BASE64.encode(&fresh.metadata);
            // Losing the cache costs a lookup next time, nothing more.
            let _ = store_profile(&path, &fresh).await;

            InjectorProfile {
                api_root: fresh.api_root,
                prefetched: Some(encoded),
            }
        }
        Err(err) => {
            tracing::warn!(
                "Could not resolve the {server} API ahead of launch: {err}"
            );

            // Whatever was kept, however old, beats making the game go and ask.
            cached.map_or(
                InjectorProfile {
                    api_root: server.to_string(),
                    prefetched: None,
                },
                |cached| InjectorProfile {
                    api_root: cached.api_root.clone(),
                    prefetched: Some(BASE64.encode(&cached.metadata)),
                },
            )
        }
    }
}

/// Writes a resolved profile down for the launches after this one.
async fn store_profile(
    path: &std::path::Path,
    profile: &CachedProfile,
) -> crate::Result<()> {
    if let Some(parent) = path.parent() {
        io::create_dir_all(parent).await?;
    }

    io::write(path, serde_json::to_vec(profile)?).await?;
    Ok(())
}

/// Asks the account system where its API is, and what is there.
///
/// The same two steps the agent takes: an address answers with an
/// `x-authlib-injector-api-location` header naming the real API root, which may
/// be relative to it, and that root answers with the metadata.
async fn fetch_profile(server: &str) -> crate::Result<CachedProfile> {
    let entry = if server.contains("://") {
        server.to_string()
    } else {
        format!("https://{server}")
    };

    let response = REQWEST_CLIENT
        .get(&entry)
        .timeout(PROFILE_TIMEOUT)
        .send()
        .await
        .map_err(|e| {
            anyhow::anyhow!(format!("Failed to reach {entry}: {e}"))
        })?;

    let located = response
        .headers()
        .get("x-authlib-injector-api-location")
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_string());

    let api_root = match located {
        Some(location) => response
            .url()
            .join(&location)
            .map_err(|e| {
                anyhow::anyhow!(format!(
                    "{entry} pointed at an API location that is not a URL: {e}"
                ))
            })?
            .to_string(),
        None => response.url().to_string(),
    };

    let metadata = REQWEST_CLIENT
        .get(&api_root)
        .timeout(PROFILE_TIMEOUT)
        .send()
        .await
        .map_err(|e| {
            anyhow::anyhow!(format!(
                "Failed to read the API metadata at {api_root}: {e}"
            ))
        })?
        .text()
        .await
        .map_err(|e| {
            anyhow::anyhow!(format!(
                "Failed to read the API metadata at {api_root}: {e}"
            ))
        })?;

    // Anything that is not the JSON object the agent expects would only make it
    // fail later, further from the cause.
    serde_json::from_str::<serde_json::Value>(&metadata)
        .ok()
        .filter(serde_json::Value::is_object)
        .ok_or_else(|| {
            anyhow::anyhow!(format!(
                "The API metadata at {api_root} is not a JSON object"
            ))
        })?;

    Ok(CachedProfile {
        api_root,
        metadata,
        fetched_at: Utc::now(),
    })
}
