//! CurseForge files their author does not let launchers download.
//!
//! CurseForge answers such a file with no download URL. Earlier versions of
//! the fork went around that by building a CDN URL from the file id; that is
//! exactly what CurseForge's API terms forbid, and the author's choice is the
//! author's to make. What the launcher can do instead is notice that the file
//! very often already exists on this machine:
//!
//!   1. the Downloads folder, where a browser puts it,
//!   2. another launcher's instances — the CurseForge app's above all, since
//!      it is allowed to download these and very likely already has,
//!   3. the launcher's own instances.
//!
//! A file is recognised by its exact size and SHA1 (or, when CurseForge has
//! no hash for it, by size and name), so nothing is ever installed that is not
//! byte for byte the file CurseForge describes. Whatever cannot be found waits
//! in `curseforge_pending_files` until the user downloads it from CurseForge's
//! site, and the Downloads folder is checked again.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::api::pack::import::ImportLauncherType;
use crate::state::{ContentSourceKind, State};
use crate::util::io::sanitize_filename;

/// How deep below a launcher's folder a content file is looked for:
/// `instances/<name>/.minecraft/mods/<file>` is five levels.
const MAX_LAUNCHER_DEPTH: usize = 6;

/// Folders that hold content files. Nothing outside one is a candidate, which
/// is what keeps a walk over another launcher's whole folder cheap.
const CONTENT_FOLDERS: [&str; 4] =
    ["mods", "resourcepacks", "shaderpacks", "datapacks"];

/// Folders that are never worth descending into, however deep the limit.
const SKIPPED_FOLDERS: [&str; 12] = [
    "assets",
    "libraries",
    "versions",
    "saves",
    "logs",
    "crash-reports",
    "screenshots",
    "natives",
    "java",
    "runtime",
    "meta",
    ".git",
];

/// A CurseForge file the API will not hand out, and how it is to be installed
/// once found.
#[derive(Clone, Debug)]
pub struct BlockedFile {
    pub project_id: i64,
    pub file_id: i64,
    pub file_name: String,
    pub size: Option<u64>,
    pub sha1: Option<String>,
    pub source_kind: ContentSourceKind,
    pub disabled: bool,
}

/// A file still waiting, as the frontend draws it.
#[derive(Serialize, Clone, Debug)]
pub struct PendingFile {
    pub project_id: String,
    pub file_id: String,
    pub file_name: String,
    /// Filled in by [`describe`] where CurseForge could be asked.
    pub project_name: Option<String>,
    pub icon_url: Option<String>,
    /// The file's own download page on CurseForge's site.
    pub download_page_url: Option<String>,
}

/// Name, picture and download page for each pending file, from the same
/// cache the content listing uses. Files CurseForge could not be asked about
/// are returned as they were.
pub async fn describe(
    mut files: Vec<PendingFile>,
    state: &State,
) -> Vec<PendingFile> {
    let project_ids = files
        .iter()
        .map(|file| file.project_id.clone())
        .collect::<HashSet<_>>();
    let metadata = crate::api::curseforge_metadata::resolve(
        &project_ids,
        &HashSet::new(),
        state,
    )
    .await;
    for file in &mut files {
        if let Some(project) = metadata.projects.get(&file.project_id) {
            file.project_name = Some(project.name.clone());
            file.icon_url = project.icon_url.clone();
            file.download_page_url = project.website_url.as_ref().map(|url| {
                format!(
                    "{}/download/{}",
                    url.trim_end_matches('/'),
                    file.file_id
                )
            });
        }
    }
    files
}

/// Install every blocked file that can be found on disk; register the rest as
/// pending for the instance. Returns what is still pending.
///
/// Never fails: a file that could not be found or installed simply stays
/// pending, which is the state the user is then shown.
pub async fn settle(
    instance_id: &str,
    files: Vec<BlockedFile>,
    state: &State,
) -> Vec<PendingFile> {
    if files.is_empty() {
        return Vec::new();
    }

    let mut roots = vec![SearchRoot::downloads()];
    roots.extend(SearchRoot::launchers(state));
    let found = find_on_disk(&files, roots).await;

    let mut pending = Vec::new();
    for file in files {
        let installed = match found.get(&file.file_id) {
            Some(path) => install_from_path(instance_id, &file, path, state)
                .await
                .inspect_err(|error| {
                    tracing::warn!(
                        "Found {} at {} but could not install it: {error}",
                        file.file_name,
                        path.display()
                    );
                })
                .is_ok(),
            None => false,
        };
        if installed {
            continue;
        }
        if let Err(error) = write_pending(instance_id, &file, state).await {
            tracing::warn!(
                "Could not remember {} as pending: {error}",
                file.file_name
            );
        }
        pending.push(PendingFile::from(&file));
    }
    pending
}

/// What the instance is still waiting on.
pub async fn list(
    instance_id: &str,
    state: &State,
) -> crate::Result<Vec<PendingFile>> {
    Ok(read_pending(instance_id, state)
        .await?
        .iter()
        .map(PendingFile::from)
        .collect())
}

/// Look in the Downloads folder for the files the instance is waiting on and
/// install whatever has arrived. Returns what is still pending.
///
/// Cheap enough to be called every couple of seconds while the user is
/// downloading: only the top of the Downloads folder is read, and only files
/// of a size something is waiting for are hashed.
pub async fn check_downloads(
    instance_id: &str,
    state: &State,
) -> crate::Result<Vec<PendingFile>> {
    let files = read_pending(instance_id, state).await?;
    if files.is_empty() {
        return Ok(Vec::new());
    }
    let found = find_on_disk(&files, vec![SearchRoot::downloads()]).await;

    let mut pending = Vec::new();
    for file in files {
        let installed = match found.get(&file.file_id) {
            Some(path) => {
                install_from_path(instance_id, &file, path, state).await?;
                true
            }
            None => false,
        };
        if installed {
            delete_pending(instance_id, &file.file_id.to_string(), state)
                .await?;
        } else {
            pending.push(PendingFile::from(&file));
        }
    }
    Ok(pending)
}

/// Stop waiting for a file: the user has decided to do without it.
pub async fn dismiss(
    instance_id: &str,
    file_id: &str,
    state: &State,
) -> crate::Result<()> {
    delete_pending(instance_id, file_id, state).await
}

impl From<&BlockedFile> for PendingFile {
    fn from(file: &BlockedFile) -> Self {
        Self {
            project_id: file.project_id.to_string(),
            file_id: file.file_id.to_string(),
            file_name: file.file_name.clone(),
            project_name: None,
            icon_url: None,
            download_page_url: None,
        }
    }
}

async fn install_from_path(
    instance_id: &str,
    file: &BlockedFile,
    path: &Path,
    state: &State,
) -> crate::Result<()> {
    let bytes = bytes::Bytes::from(
        tokio::fs::read(path)
            .await
            .map_err(|e| crate::util::io::IOError::with_path(e, path))?,
    );
    let project_id = file.project_id.to_string();
    let file_id = file.file_id.to_string();
    let relative_path = {
        let _permit = state.installs.db_semaphore.acquire().await?;
        crate::state::instances::commands::add_project_bytes(
            instance_id,
            &sanitize_filename(&file.file_name),
            bytes,
            file.sha1.as_deref(),
            None,
            file.source_kind,
            Some(&project_id),
            Some(&file_id),
            state,
        )
        .await?
    };
    if file.disabled {
        crate::state::instances::commands::toggle_disable_project(
            instance_id,
            &relative_path,
            Some(false),
            state,
        )
        .await?;
    }
    tracing::info!(
        "Installed {} from {} instead of downloading it",
        file.file_name,
        path.display()
    );
    Ok(())
}

/// A folder to look for files in, and how far down.
struct SearchRoot {
    path: PathBuf,
    depth: usize,
    /// Whether a file must sit in one of `CONTENT_FOLDERS` to be considered.
    /// True for launcher folders, false for Downloads.
    content_folders_only: bool,
}

impl SearchRoot {
    fn downloads() -> Option<Self> {
        Some(Self {
            path: dirs::download_dir()?,
            depth: 1,
            content_folders_only: false,
        })
    }

    fn launchers(state: &State) -> Vec<Option<Self>> {
        let mut seen = HashSet::new();
        let others = if crate::api::curseforge_metadata::options()
            .search_other_launchers
        {
            vec![
                ImportLauncherType::Curseforge,
                ImportLauncherType::PrismLauncher,
                ImportLauncherType::MultiMC,
                ImportLauncherType::ATLauncher,
                ImportLauncherType::GDLauncher,
                ImportLauncherType::ModrinthApp,
            ]
        } else {
            Vec::new()
        };
        others
            .into_iter()
            .flat_map(crate::api::launcher_search::find_all)
            .chain(std::iter::once(state.directories.instances_dir()))
            .filter(|path| seen.insert(path.clone()))
            .map(|path| {
                Some(Self {
                    path,
                    depth: MAX_LAUNCHER_DEPTH,
                    content_folders_only: true,
                })
            })
            .collect()
    }
}

/// Where each wanted file is, keyed by file id, for the ones that were found.
async fn find_on_disk(
    files: &[BlockedFile],
    roots: Vec<Option<SearchRoot>>,
) -> HashMap<i64, PathBuf> {
    let wanted = files.to_vec();
    tokio::task::spawn_blocking(move || {
        let roots = roots.into_iter().flatten().collect::<Vec<_>>();
        find_on_disk_blocking(&wanted, &roots)
    })
    .await
    .unwrap_or_default()
}

fn find_on_disk_blocking(
    files: &[BlockedFile],
    roots: &[SearchRoot],
) -> HashMap<i64, PathBuf> {
    // Only files whose exact size something is waiting for get hashed, which
    // is what makes looking through whole launcher folders affordable.
    let mut by_size: HashMap<u64, Vec<&BlockedFile>> = HashMap::new();
    let mut by_name: HashMap<String, Vec<&BlockedFile>> = HashMap::new();
    for file in files {
        match file.size {
            Some(size) => by_size.entry(size).or_default().push(file),
            None => by_name
                .entry(file.file_name.to_lowercase())
                .or_default()
                .push(file),
        }
    }

    let mut found = HashMap::new();
    for root in roots {
        walk(&root.path, root.depth, &mut |path, size| {
            let sized = by_size.get(&size).into_iter().flatten();
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.trim_end_matches(".disabled").to_lowercase());
            let named = name
                .as_ref()
                .and_then(|name| by_name.get(name))
                .into_iter()
                .flatten();
            let candidates = sized
                .chain(named)
                .filter(|file| !found.contains_key(&file.file_id))
                .copied()
                .collect::<Vec<_>>();
            if candidates.is_empty() {
                return;
            }
            if root.content_folders_only && !in_content_folder(path) {
                return;
            }

            let Ok(bytes) = std::fs::read(path) else {
                return;
            };
            let sha1 = sha1_smol::Sha1::from(&bytes).hexdigest();
            for file in candidates {
                if matches_file(file, name.as_deref(), size, &sha1) {
                    found.insert(file.file_id, path.to_path_buf());
                }
            }
        });
        if found.len() == files.len() {
            break;
        }
    }
    found
}

/// Whether a file on disk is the file CurseForge describes. A hash, when
/// there is one, is the only thing that counts; a size and a name are trusted
/// only when CurseForge gave no hash at all.
fn matches_file(
    file: &BlockedFile,
    name_on_disk: Option<&str>,
    size: u64,
    sha1: &str,
) -> bool {
    match &file.sha1 {
        Some(expected) => expected.eq_ignore_ascii_case(sha1),
        None => {
            file.size.is_none_or(|expected| expected == size)
                && name_on_disk == Some(file.file_name.to_lowercase().as_str())
        }
    }
}

fn in_content_folder(path: &Path) -> bool {
    path.parent()
        .and_then(|parent| parent.file_name())
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            CONTENT_FOLDERS
                .iter()
                .any(|folder| folder.eq_ignore_ascii_case(name))
        })
}

/// Every regular file below `dir`, at most `depth` levels down, with its size.
/// Unreadable folders are skipped rather than failing the search.
fn walk(dir: &Path, depth: usize, visit: &mut dyn FnMut(&Path, u64)) {
    if depth == 0 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        let path = entry.path();
        if file_type.is_dir() {
            let skipped = path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| {
                    SKIPPED_FOLDERS
                        .iter()
                        .any(|skip| skip.eq_ignore_ascii_case(name))
                });
            if !skipped {
                walk(&path, depth - 1, visit);
            }
        } else if file_type.is_file()
            && let Ok(metadata) = entry.metadata()
        {
            visit(&path, metadata.len());
        }
    }
}

async fn write_pending(
    instance_id: &str,
    file: &BlockedFile,
    state: &State,
) -> crate::Result<()> {
    let file_id = file.file_id.to_string();
    let project_id = file.project_id.to_string();
    let size = file.size.map(|size| size as i64);
    let source_kind = file.source_kind.as_str();
    let added = chrono::Utc::now().timestamp();
    sqlx::query!(
        "
        INSERT INTO curseforge_pending_files (
            instance_id, file_id, project_id, file_name, size, sha1,
            source_kind, disabled, added
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        ON CONFLICT(instance_id, file_id) DO UPDATE SET
            project_id = excluded.project_id,
            file_name = excluded.file_name,
            size = excluded.size,
            sha1 = excluded.sha1,
            source_kind = excluded.source_kind,
            disabled = excluded.disabled
        ",
        instance_id,
        file_id,
        project_id,
        file.file_name,
        size,
        file.sha1,
        source_kind,
        file.disabled,
        added,
    )
    .execute(&state.pool)
    .await?;
    Ok(())
}

async fn read_pending(
    instance_id: &str,
    state: &State,
) -> crate::Result<Vec<BlockedFile>> {
    let rows = sqlx::query!(
        "
        SELECT file_id, project_id, file_name, size, sha1, source_kind,
               disabled
        FROM curseforge_pending_files
        WHERE instance_id = $1
        ORDER BY added, file_name
        ",
        instance_id
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(rows
        .into_iter()
        .filter_map(|row| {
            Some(BlockedFile {
                project_id: row.project_id.parse().ok()?,
                file_id: row.file_id.parse().ok()?,
                file_name: row.file_name,
                size: row.size.and_then(|size| u64::try_from(size).ok()),
                sha1: row.sha1,
                source_kind: ContentSourceKind::from_str(&row.source_kind)
                    .ok()?,
                disabled: row.disabled != 0,
            })
        })
        .collect())
}

async fn delete_pending(
    instance_id: &str,
    file_id: &str,
    state: &State,
) -> crate::Result<()> {
    sqlx::query!(
        "
        DELETE FROM curseforge_pending_files
        WHERE instance_id = $1 AND file_id = $2
        ",
        instance_id,
        file_id
    )
    .execute(&state.pool)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blocked(size: Option<u64>, sha1: Option<&str>) -> BlockedFile {
        BlockedFile {
            project_id: 1,
            file_id: 2,
            file_name: "Thing-1.0.jar".to_string(),
            size,
            sha1: sha1.map(str::to_string),
            source_kind: ContentSourceKind::CurseForge,
            disabled: false,
        }
    }

    #[test]
    fn a_hash_decides_on_its_own() {
        let file = blocked(Some(3), Some("ABC"));
        assert!(matches_file(&file, Some("renamed (1).jar"), 3, "abc"));
        assert!(!matches_file(&file, Some("thing-1.0.jar"), 3, "abd"));
    }

    #[test]
    fn without_a_hash_size_and_name_both_have_to_agree() {
        let file = blocked(Some(3), None);
        assert!(matches_file(&file, Some("thing-1.0.jar"), 3, "x"));
        assert!(!matches_file(&file, Some("thing-1.0.jar"), 4, "x"));
        assert!(!matches_file(&file, Some("other.jar"), 3, "x"));
    }

    #[test]
    fn a_file_is_found_by_content_wherever_it_sits() {
        let root = tempfile::tempdir().unwrap();
        let mods = root.path().join("Instances").join("Pack").join("mods");
        std::fs::create_dir_all(&mods).unwrap();
        // Saved under another name, as a browser does with a duplicate.
        std::fs::write(mods.join("Thing-1.0 (1).jar"), b"abc").unwrap();
        // Same size, different bytes, outside a content folder.
        std::fs::write(root.path().join("decoy.jar"), b"abd").unwrap();

        let sha1 = sha1_smol::Sha1::from(b"abc").hexdigest();
        let file = blocked(Some(3), Some(&sha1));
        let found = find_on_disk_blocking(
            std::slice::from_ref(&file),
            &[SearchRoot {
                path: root.path().to_path_buf(),
                depth: MAX_LAUNCHER_DEPTH,
                content_folders_only: true,
            }],
        );

        assert_eq!(found.get(&2), Some(&mods.join("Thing-1.0 (1).jar")));
    }

    #[test]
    fn a_launcher_folder_only_counts_content_folders() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("Thing-1.0.jar"), b"abc").unwrap();
        let sha1 = sha1_smol::Sha1::from(b"abc").hexdigest();
        let file = blocked(Some(3), Some(&sha1));

        let as_launcher = find_on_disk_blocking(
            std::slice::from_ref(&file),
            &[SearchRoot {
                path: root.path().to_path_buf(),
                depth: MAX_LAUNCHER_DEPTH,
                content_folders_only: true,
            }],
        );
        let as_downloads = find_on_disk_blocking(
            std::slice::from_ref(&file),
            &[SearchRoot {
                path: root.path().to_path_buf(),
                depth: 1,
                content_folders_only: false,
            }],
        );

        assert!(as_launcher.is_empty());
        assert_eq!(as_downloads.len(), 1);
    }
}
