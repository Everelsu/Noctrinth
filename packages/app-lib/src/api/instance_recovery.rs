//! Instances that are on disk but not in the launcher.
//!
//! The database is the only place an instance is written down: the folder holds
//! the mods, the worlds and the settings, and nothing in it says which
//! Minecraft it is for. So a database that was lost — deleted to fix something
//! else, left behind by a reinstall, restored from a backup made before the
//! instance existed — takes every instance with it while every world is still
//! sitting there.
//!
//! This reads the folders instead. What a run leaves behind says which version
//! and loader it was: the log names both on its first line, and a crash report
//! names them again. That is enough to put the instance back with its folder
//! where it was, and the launcher fetches the version files the next time it is
//! launched.

use std::collections::HashSet;
use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::State;
use crate::state::{InstanceLink, ModLoader};

/// How much of a log is worth reading to find the version.
///
/// Both the loader and the game name themselves in the first few lines.
const LOG_HEAD_BYTES: u64 = 16 * 1024;

/// An instance folder the launcher has no record of.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OrphanedInstance {
    /// The folder's name, which is also the path an instance would take.
    pub folder: String,
    /// What to call it: the folder's name, which is what the launcher named it.
    pub name: String,
    /// The Minecraft version its last run was, when that could be read.
    pub game_version: Option<String>,
    pub loader: Option<ModLoader>,
    pub loader_version: Option<String>,
    /// What is in there, so the player can tell one folder from another.
    pub mods: usize,
    pub worlds: usize,
    /// When anything in the folder was last written, in seconds since the epoch.
    pub modified: u64,
}

static VERSION_PATTERNS: LazyLock<Vec<(Regex, Option<ModLoader>)>> =
    LazyLock::new(|| {
        vec![
            // Fabric and Quilt say both on the line they start with.
            (
                Regex::new(
                    r"Loading Minecraft (?P<version>[\w.\-]+) with Fabric Loader (?P<loader>[\w.\-]+)",
                )
                .unwrap(),
                Some(ModLoader::Fabric),
            ),
            (
                Regex::new(
                    r"Loading Minecraft (?P<version>[\w.\-]+) with Quilt Loader (?P<loader>[\w.\-]+)",
                )
                .unwrap(),
                Some(ModLoader::Quilt),
            ),
            // Forge and NeoForge pass theirs on the command line, which the log
            // repeats back.
            (
                Regex::new(
                    r"--fml\.neoForgeVersion,? (?P<loader>[\w.\-]+)",
                )
                .unwrap(),
                Some(ModLoader::NeoForge),
            ),
            (
                Regex::new(r"--fml\.forgeVersion,? (?P<loader>[\w.\-]+)")
                    .unwrap(),
                Some(ModLoader::Forge),
            ),
            // A crash report says the version outright, whatever loaded it.
            (
                Regex::new(r"(?m)^\s*Minecraft Version: (?P<version>[\w.\-]+)")
                    .unwrap(),
                None,
            ),
            (
                Regex::new(r"--fml\.mcVersion,? (?P<version>[\w.\-]+)").unwrap(),
                None,
            ),
            // Vanilla, and anything else that only names the game.
            (
                Regex::new(
                    r"Loading for game Minecraft (?P<version>[\w.\-]+)",
                )
                .unwrap(),
                None,
            ),
        ]
    });

/// The folders under the launcher's instances directory that it has no row for.
#[tracing::instrument]
pub async fn scan_for_orphans() -> crate::Result<Vec<OrphanedInstance>> {
    let state = State::get().await?;

    let known: HashSet<String> =
        sqlx::query_scalar::<_, String>("SELECT path FROM instances")
            .fetch_all(&state.pool)
            .await?
            .into_iter()
            .collect();

    let instances_dir = state.directories.instances_dir();
    let Ok(mut entries) = tokio::fs::read_dir(&instances_dir).await else {
        return Ok(Vec::new());
    };

    let mut orphans = Vec::new();
    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let Some(folder) = path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
        else {
            continue;
        };

        if known.contains(&folder) || !looks_like_an_instance(&path) {
            continue;
        }

        orphans.push(describe(&path, folder).await);
    }

    orphans.sort_by(|a, b| b.modified.cmp(&a.modified));
    Ok(orphans)
}

/// Whether there is a game in this folder rather than something else.
///
/// A folder the launcher made always has at least one of these, and a folder
/// that has none of them is not worth offering: an empty directory, or
/// something the player keeps beside their instances.
fn looks_like_an_instance(path: &Path) -> bool {
    const MARKERS: &[&str] = &[
        "mods",
        "config",
        "saves",
        "resourcepacks",
        "logs",
        "crash-reports",
        "options.txt",
        "servers.dat",
        "versions",
        "shaderpacks",
    ];

    MARKERS.iter().any(|marker| path.join(marker).exists())
}

async fn describe(path: &Path, folder: String) -> OrphanedInstance {
    let (game_version, loader, loader_version) = read_version(path).await;

    OrphanedInstance {
        name: folder.clone(),
        folder,
        game_version,
        loader,
        loader_version,
        mods: count_in(&path.join("mods"), false).await,
        worlds: count_in(&path.join("saves"), true).await,
        modified: newest_write(path).await,
    }
}

/// What the last run in this folder was, as far as it wrote down.
async fn read_version(
    path: &Path,
) -> (Option<String>, Option<ModLoader>, Option<String>) {
    let mut version = None;
    let mut loader = None;
    let mut loader_version = None;

    for source in sources_to_read(path).await {
        let Ok(text) = read_head(&source).await else {
            continue;
        };

        for (pattern, kind) in VERSION_PATTERNS.iter() {
            let Some(captures) = pattern.captures(&text) else {
                continue;
            };

            if version.is_none()
                && let Some(found) = captures.name("version")
            {
                version = Some(found.as_str().to_string());
            }

            if loader.is_none()
                && let Some(kind) = kind
            {
                loader = Some(*kind);
                loader_version = captures
                    .name("loader")
                    .map(|found| found.as_str().to_string());
            }
        }

        if version.is_some() && loader.is_some() {
            break;
        }
    }

    // A folder with mods and no loader named is still a modded folder, but
    // which loader is a guess — and a wrong guess installs the wrong game.
    (version, loader, loader_version)
}

/// The files worth reading, newest first: the last log, then the last crash.
async fn sources_to_read(path: &Path) -> Vec<std::path::PathBuf> {
    let mut sources = Vec::new();

    let latest = path.join("logs").join("latest.log");
    if latest.is_file() {
        sources.push(latest);
    }

    if let Some(report) = newest_matching(&path.join("crash-reports"), |name| {
        name.starts_with("crash-") && name.ends_with(".txt")
    })
    .await
    {
        sources.push(report);
    }

    if let Some(log) = newest_matching(&path.join("logs"), |name| {
        name.ends_with(".log") && name != "latest.log"
    })
    .await
    {
        sources.push(log);
    }

    sources
}

async fn read_head(path: &Path) -> crate::Result<String> {
    use tokio::io::AsyncReadExt;

    let file = tokio::fs::File::open(path)
        .await
        .map_err(|e| crate::util::io::IOError::with_path(e, path))?;

    let mut bytes = Vec::new();
    file.take(LOG_HEAD_BYTES)
        .read_to_end(&mut bytes)
        .await
        .map_err(|e| crate::util::io::IOError::with_path(e, path))?;

    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

async fn count_in(path: &Path, directories: bool) -> usize {
    let Ok(mut entries) = tokio::fs::read_dir(path).await else {
        return 0;
    };

    let mut count = 0;
    while let Ok(Some(entry)) = entries.next_entry().await {
        let is_dir = entry.path().is_dir();
        if is_dir == directories {
            count += 1;
        }
    }

    count
}

/// When anything directly in the folder was last written.
async fn newest_write(path: &Path) -> u64 {
    let Ok(mut entries) = tokio::fs::read_dir(path).await else {
        return 0;
    };

    let mut newest = 0;
    while let Ok(Some(entry)) = entries.next_entry().await {
        if let Ok(metadata) = entry.metadata().await
            && let Ok(modified) = metadata.modified()
            && let Ok(since) = modified.duration_since(std::time::UNIX_EPOCH)
        {
            newest = newest.max(since.as_secs());
        }
    }

    newest
}

async fn newest_matching(
    dir: &Path,
    accept: impl Fn(&str) -> bool,
) -> Option<std::path::PathBuf> {
    let mut entries = tokio::fs::read_dir(dir).await.ok()?;
    let mut newest: Option<(std::time::SystemTime, std::path::PathBuf)> = None;

    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        let Some(name) = path.file_name().map(|name| name.to_string_lossy())
        else {
            continue;
        };
        if !accept(name.as_ref()) {
            continue;
        }

        let Ok(modified) = entry
            .metadata()
            .await
            .and_then(|metadata| metadata.modified())
        else {
            continue;
        };

        if newest.as_ref().is_none_or(|(at, _)| modified >= *at) {
            newest = Some((modified, path));
        }
    }

    newest.map(|(_, path)| path)
}

/// Puts an orphaned folder back in the launcher, as it was.
///
/// The instance is created the way any other is and then pointed at the folder
/// that is already there, rather than the empty one it was given: everything
/// the launcher knows how to set up — the content set, the sync preferences,
/// the launch overrides — is set up by the same code as for a new instance, and
/// only the folder is different.
///
/// What it does not do is install anything. The instance comes back as not yet
/// installed, and the next launch fetches the version files it needs, which is
/// the one thing that does need a connection.
#[tracing::instrument]
pub async fn adopt_orphan(
    folder: &str,
    name: &str,
    game_version: &str,
    loader: ModLoader,
    loader_version: Option<String>,
) -> crate::Result<String> {
    let state = State::get().await?;

    let full_path = state.directories.instances_dir().join(folder);
    if !full_path.is_dir() {
        return Err(crate::ErrorKind::InputError(format!(
            "There is no folder called {folder} to import"
        ))
        .into());
    }

    let taken: Option<String> =
        sqlx::query_scalar("SELECT id FROM instances WHERE path = ?")
            .bind(folder)
            .fetch_optional(&state.pool)
            .await?;
    if let Some(id) = taken {
        return Err(crate::ErrorKind::InputError(format!(
            "That folder is already the instance {id}"
        ))
        .into());
    }

    let instance = crate::api::instance::create(
        name.to_string(),
        game_version.to_string(),
        loader,
        loader_version,
        None,
        None,
        InstanceLink::Unmanaged,
    )
    .await?
    .instance;

    // The folder it was given is empty and is not the one we want.
    let created = state.directories.instances_dir().join(&instance.path);
    if created != full_path {
        let _ = tokio::fs::remove_dir_all(&created).await;

        sqlx::query("UPDATE instances SET path = ? WHERE id = ?")
            .bind(folder)
            .bind(&instance.id)
            .execute(&state.pool)
            .await?;

        crate::state::instances::watcher::watch_instance_folder(
            &instance.id,
            folder,
            &state.file_watcher,
            &state.directories,
        )
        .await;
    }

    Ok(instance.id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(text: &str) -> (Option<String>, Option<ModLoader>, Option<String>) {
        let mut version = None;
        let mut loader = None;
        let mut loader_version = None;

        for (pattern, kind) in VERSION_PATTERNS.iter() {
            let Some(captures) = pattern.captures(text) else {
                continue;
            };
            if version.is_none()
                && let Some(found) = captures.name("version")
            {
                version = Some(found.as_str().to_string());
            }
            if loader.is_none()
                && let Some(kind) = kind
            {
                loader = Some(*kind);
                loader_version = captures
                    .name("loader")
                    .map(|found| found.as_str().to_string());
            }
        }

        (version, loader, loader_version)
    }

    #[test]
    fn a_fabric_log_names_the_game_and_the_loader() {
        let (version, loader, loader_version) = read(
            "[12:00:00] [main/INFO]: Loading Minecraft 1.20.1 with Fabric Loader 0.15.7",
        );

        assert_eq!(version.as_deref(), Some("1.20.1"));
        assert_eq!(loader, Some(ModLoader::Fabric));
        assert_eq!(loader_version.as_deref(), Some("0.15.7"));
    }

    #[test]
    fn a_quilt_log_is_not_mistaken_for_fabric() {
        let (version, loader, _) = read(
            "[12:00:00] [main/INFO]: Loading Minecraft 1.20.1 with Quilt Loader 0.21.0",
        );

        assert_eq!(version.as_deref(), Some("1.20.1"));
        assert_eq!(loader, Some(ModLoader::Quilt));
    }

    #[test]
    fn forge_is_read_from_the_arguments_it_was_given() {
        let (version, loader, loader_version) = read(
            "ModLauncher running: args [--gameDir, ., --fml.forgeVersion, 47.2.0, --fml.mcVersion, 1.20.1]",
        );

        assert_eq!(version.as_deref(), Some("1.20.1"));
        assert_eq!(loader, Some(ModLoader::Forge));
        assert_eq!(loader_version.as_deref(), Some("47.2.0"));
    }

    #[test]
    fn neoforge_is_not_read_as_forge() {
        let (version, loader, loader_version) = read(
            "ModLauncher running: args [--fml.neoForgeVersion, 21.1.65, --fml.mcVersion, 1.21.1]",
        );

        assert_eq!(version.as_deref(), Some("1.21.1"));
        assert_eq!(loader, Some(ModLoader::NeoForge));
        assert_eq!(loader_version.as_deref(), Some("21.1.65"));
    }

    #[test]
    fn a_crash_report_is_enough_for_the_version_alone() {
        let (version, loader, _) = read(
            "-- System Details --\n\tMinecraft Version: 1.8.9\n\tOperating System: Windows 10",
        );

        assert_eq!(version.as_deref(), Some("1.8.9"));
        assert_eq!(loader, None);
    }

    #[test]
    fn a_log_that_says_nothing_is_left_alone() {
        let (version, loader, loader_version) =
            read("[12:00:00] [main/INFO]: Stopping!");

        assert!(version.is_none());
        assert!(loader.is_none());
        assert!(loader_version.is_none());
    }
}
