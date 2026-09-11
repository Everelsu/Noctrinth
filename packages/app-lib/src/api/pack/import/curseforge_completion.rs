//! The parts of a CurseForge instance that copying its folder does not bring.
//!
//! Importing a CurseForge instance is a file copy: whatever is in the folder is
//! what the new instance gets. That is fine right up until the folder is not
//! the whole instance, which is the ordinary case rather than the odd one:
//!
//!   * CurseForge writes a mod into `minecraftinstance.json` when it is added
//!     to the pack and downloads the jar separately. An instance that was never
//!     opened after an update, that was interrupted mid-download, or that was
//!     copied off another machine without its `mods` folder lists mods it does
//!     not have. Copying it faithfully reproduces a pack with mods missing, and
//!     the launcher has no way to know anything is gone — the only symptom is
//!     the game crashing, or quietly playing without them.
//!
//!   * The instance's picture is not in the folder at all. `profileImagePath`
//!     is an absolute path into the CurseForge installation, so it is dead the
//!     moment the folder is moved, the drive letter changes, or CurseForge is
//!     uninstalled — and upstream then skips the modpack thumbnail it could
//!     have fallen back on, because the two are written as one `if`/`else`.
//!
//! So the `minecraftinstance.json` upstream reads for the name and the loader
//! is read again here, for the two other things it knows: every file the pack
//! is supposed to contain, and where its picture came from. Anything the copy
//! did not bring is fetched from CurseForge's CDN — the URLs are in the file,
//! so no API key is involved — and recorded as CurseForge content, which is
//! how a mod that arrived this way can later be updated like any other.
//!
//! Nothing here is allowed to fail the import. An instance that came across
//! with nine mods out of ten is still worth having, and the user is told which
//! one is missing rather than being handed an error and no instance at all.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::event::emit::emit_warning;
use crate::install::{
    InstallPhaseDetails, InstallPhaseId, InstallProgress,
    InstallProgressReporter,
};
use crate::state::{ContentSourceKind, State};
use crate::util::{fetch, io};

use super::get_all_subfiles;

/// CurseForge's per-instance metadata file, in the instance's own folder.
const INSTANCE_FILE: &str = "minecraftinstance.json";

/// How many unrecoverable files are named in the warning, so a pack that lost
/// its whole `mods` folder does not produce an unreadable wall of text.
const MAX_LISTED_MISSING: usize = 8;

/// Image files to look for when nothing points at one — the names CurseForge
/// and the packs themselves use, in the order they are preferred.
const ICON_FILE_STEMS: [&str; 6] = [
    "profileImage",
    "icon",
    "instance-icon",
    "modpack-icon",
    "pack",
    "logo",
];

/// Extensions accepted for those, matched case-insensitively. SVG is left out
/// deliberately: instance icons are rasterised, and `cache_icon_from_path`
/// refuses it.
const ICON_EXTENSIONS: [&str; 6] = ["png", "jpg", "jpeg", "webp", "gif", "bmp"];

/// Subfolders of an instance that CurseForge has kept its own files in.
const ICON_SUBFOLDERS: [&str; 2] = [".curseforge", ".cf"];

#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
struct InstanceManifest {
    #[serde(default)]
    installed_addons: Vec<InstalledAddon>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct InstalledAddon {
    // Written `addonID`, which no rename rule produces on its own.
    #[serde(default, alias = "addonID", alias = "AddonID")]
    addon_id: Option<i64>,
    #[serde(default, alias = "InstalledFile")]
    installed_file: Option<InstalledFile>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct InstalledFile {
    #[serde(default)]
    id: Option<i64>,
    /// What the file is called on disk, which is not always what it is called
    /// on CurseForge — the app renames collisions.
    #[serde(default, alias = "FileNameOnDisk")]
    file_name_on_disk: Option<String>,
    #[serde(default, alias = "FileName")]
    file_name: Option<String>,
    /// `None` when the author opted out of third-party distribution.
    #[serde(default, alias = "DownloadUrl")]
    download_url: Option<String>,
    #[serde(default, alias = "FileLength")]
    file_length: Option<u64>,
    #[serde(default, alias = "Hashes")]
    hashes: Vec<FileHash>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct FileHash {
    #[serde(default)]
    value: String,
    /// 1 = SHA1, 2 = MD5.
    #[serde(default)]
    algo: i32,
}

/// One file the pack says it has, reduced to what is needed to find it on disk
/// and fetch it again if it is not there.
struct ExpectedFile {
    file_name: String,
    project_id: Option<i64>,
    file_id: Option<i64>,
    download_url: Option<String>,
    length: Option<u64>,
    sha1: Option<String>,
}

impl ExpectedFile {
    fn from_addon(addon: InstalledAddon) -> Option<Self> {
        let file = addon.installed_file?;
        let file_name = file
            .file_name_on_disk
            .or(file.file_name)
            .map(|name| name.trim().to_string())
            .filter(|name| !name.is_empty())?;

        Some(Self {
            file_name,
            project_id: addon.addon_id,
            file_id: file.id,
            // Only ever fetched over TLS, and only from a URL the file itself
            // carried — never one assembled out of anything else it says.
            download_url: file
                .download_url
                .filter(|url| url.starts_with("https://")),
            length: file.file_length.filter(|length| *length > 0),
            sha1: file
                .hashes
                .into_iter()
                .find(|hash| hash.algo == 1 && !hash.value.is_empty())
                .map(|hash| hash.value),
        })
    }

    /// Rebuilds the public CDN URL from the numeric file id. CurseForge only
    /// nulls `downloadUrl` for files whose author opted out of the *API*; the
    /// CDN path stays reachable, which is the same fallback the fork's own
    /// modpack installer already uses.
    fn cdn_fallback_url(&self) -> Option<String> {
        let id = self.file_id?;
        if id < 1000 {
            return None;
        }
        Some(format!(
            "https://edge.forgecdn.net/files/{}/{}/{}",
            id / 1000,
            id % 1000,
            self.file_name.replace(' ', "%20")
        ))
    }

    fn download_candidates(&self) -> Vec<String> {
        let mut urls = Vec::with_capacity(2);
        if let Some(url) = &self.download_url {
            urls.push(url.clone());
        }
        if let Some(url) = self.cdn_fallback_url()
            && !urls.contains(&url)
        {
            urls.push(url);
        }
        urls
    }
}

/// A file of the pack's as it actually sits in the imported instance.
struct PresentFile {
    path: PathBuf,
    size: u64,
    /// CurseForge stores a mod the user turned off as `name.jar.disabled`, and
    /// so does this launcher — a file that comes back has to go back off.
    disabled: bool,
}

/// Reads the instance's manifest, ignoring a missing or unreadable one: this
/// runs after the import has already succeeded on upstream's terms, and a
/// manifest that cannot be parsed is no reason to undo that.
async fn read_manifest(folder: &Path) -> Option<InstanceManifest> {
    let path = folder.join(INSTANCE_FILE);
    let (text, _) = io::read_any_encoding_to_string(&path).await.ok()?;
    match serde_json::from_str::<InstanceManifest>(&text) {
        Ok(manifest) => Some(manifest),
        Err(error) => {
            tracing::warn!(
                "Could not read {} while importing: {error}",
                path.display()
            );
            None
        }
    }
}

/// Every file in the instance, keyed by lowercased name with any `.disabled`
/// taken off — which is how the manifest names them, and how the same mod is
/// found whether the user left it switched on or off.
async fn index_instance_files(
    instance_path: &Path,
) -> crate::Result<HashMap<String, PresentFile>> {
    let mut present: HashMap<String, PresentFile> = HashMap::new();

    for path in get_all_subfiles(instance_path, false).await? {
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let disabled = name.ends_with(".disabled");
        let key = name.trim_end_matches(".disabled").to_lowercase();
        let size = tokio::fs::metadata(&path)
            .await
            .map(|metadata| metadata.len())
            .unwrap_or(0);

        // A pack can carry the same jar in two places. Whichever copy is whole
        // wins, so a stray zero-byte duplicate does not cause a perfectly good
        // mod to be downloaded all over again.
        match present.get_mut(&key) {
            Some(existing) if size > existing.size => {
                existing.path = path;
                existing.size = size;
                existing.disabled = disabled;
            }
            Some(_) => {}
            None => {
                present.insert(
                    key,
                    PresentFile {
                        path,
                        size,
                        disabled,
                    },
                );
            }
        }
    }

    Ok(present)
}

/// Whether the copy of this file that is on disk can be taken at its word.
///
/// The manifest records the exact byte length of every file it lists, so a
/// copy that was cut short — an interrupted download in CurseForge, a drive
/// that filled up mid-import — is recognisable without hashing anything.
fn is_intact(expected: &ExpectedFile, present: &PresentFile) -> bool {
    if present.size == 0 {
        return false;
    }
    match expected.length {
        Some(length) => present.size == length,
        None => true,
    }
}

/// Downloads one file the pack is missing and records it as CurseForge
/// content, so it carries its project and can be updated like any other mod.
async fn restore_one(
    state: &State,
    instance_id: &str,
    expected: &ExpectedFile,
    disabled: bool,
) -> crate::Result<()> {
    let candidates = expected.download_candidates();
    if candidates.is_empty() {
        return Err(crate::ErrorKind::InputError(
            "CurseForge gives no way to download it".to_string(),
        )
        .into());
    }

    let mirrors = candidates.iter().map(String::as_str).collect::<Vec<_>>();
    let bytes = fetch::fetch_mirrors(
        &mirrors,
        expected.sha1.as_deref(),
        None,
        None,
        &state.fetch_semaphore,
        &state.pool,
    )
    .await?;

    let file_name = io::sanitize_filename(&expected.file_name);
    let project_id = expected.project_id.map(|id| id.to_string());
    let file_id = expected.file_id.map(|id| id.to_string());
    let relative_path = {
        // Serialised against the rest of the install bookkeeping, exactly as
        // the modpack installer does when it writes content of its own.
        let _permit = state.install_db_semaphore.acquire().await?;
        crate::state::instances::commands::add_project_bytes(
            instance_id,
            &file_name,
            bytes,
            expected.sha1.as_deref(),
            // Left to the installer to infer from the archive itself, which
            // tells a resource pack from a mod far better than an extension.
            None,
            ContentSourceKind::CurseForge,
            project_id.as_deref(),
            file_id.as_deref(),
            state,
        )
        .await?
    };

    if disabled {
        crate::state::instances::commands::toggle_disable_project(
            instance_id,
            &relative_path,
            Some(false),
            state,
        )
        .await?;
    }

    Ok(())
}

/// Puts back every file `minecraftinstance.json` lists that the copy did not
/// bring across, and says which ones could not be put back.
pub(super) async fn restore_missing_addons(
    instance_id: &str,
    curseforge_instance_folder: &Path,
    reporter: &InstallProgressReporter,
    details: &InstallPhaseDetails,
) -> crate::Result<()> {
    let Some(manifest) = read_manifest(curseforge_instance_folder).await else {
        return Ok(());
    };
    let expected = manifest
        .installed_addons
        .into_iter()
        .filter_map(ExpectedFile::from_addon)
        .collect::<Vec<_>>();
    if expected.is_empty() {
        return Ok(());
    }

    let state = State::get().await?;
    let instance_path =
        crate::api::instance::get_full_path(instance_id).await?;
    let present = index_instance_files(&instance_path).await?;

    // `truncated` carries the path of a copy that is there but the wrong size,
    // which has to go before its replacement is written: the replacement lands
    // under the pack's own name for the file, which need not be that one.
    let missing = expected
        .into_iter()
        .filter_map(|expected| {
            match present.get(&expected.file_name.to_lowercase()) {
                Some(file) if is_intact(&expected, file) => None,
                Some(file) => {
                    Some((expected, file.disabled, Some(file.path.clone())))
                }
                None => Some((expected, false, None)),
            }
        })
        .collect::<Vec<_>>();
    if missing.is_empty() {
        return Ok(());
    }

    tracing::info!(
        "{} file(s) the CurseForge instance lists are not in the copy; \
         fetching them",
        missing.len()
    );

    let total = missing.len() as u64;
    let mut failures = Vec::new();

    for (index, (expected, disabled, truncated)) in
        missing.into_iter().enumerate()
    {
        reporter
            .update(
                InstallPhaseId::DownloadingContent,
                Some(InstallProgress {
                    current: index as u64,
                    total,
                    secondary: None,
                }),
                details.clone(),
            )
            .await?;

        if let Some(path) = truncated
            && let Err(error) = io::remove_file(&path).await
        {
            tracing::warn!(
                "Could not remove the incomplete {}: {error}",
                path.display()
            );
        }

        if let Err(error) =
            restore_one(&state, instance_id, &expected, disabled).await
        {
            failures.push(format!("{} ({error})", expected.file_name));
        }
    }

    reporter
        .update(
            InstallPhaseId::DownloadingContent,
            Some(InstallProgress {
                current: total,
                total,
                secondary: None,
            }),
            details.clone(),
        )
        .await?;

    if !failures.is_empty() {
        let listed = failures
            .iter()
            .take(MAX_LISTED_MISSING)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        let suffix = if failures.len() > MAX_LISTED_MISSING {
            format!(" and {} more", failures.len() - MAX_LISTED_MISSING)
        } else {
            String::new()
        };
        emit_warning(&format!(
            "The CurseForge instance lists {} file(s) that were neither in \
             its folder nor downloadable: {listed}{suffix}. It was imported \
             without them.",
            failures.len()
        ))
        .await?;
    }

    Ok(())
}

/// Whether this is a file an instance icon could be read out of.
fn looks_like_image(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            ICON_EXTENSIONS
                .iter()
                .any(|candidate| extension.eq_ignore_ascii_case(candidate))
        })
}

async fn first_existing(candidates: Vec<PathBuf>) -> Option<PathBuf> {
    for candidate in candidates {
        if tokio::fs::try_exists(&candidate).await.unwrap_or(false) {
            return Some(candidate);
        }
    }
    None
}

/// Finds the picture a CurseForge instance was shown under.
///
/// `declared` is `profileImagePath` out of the manifest, which is an absolute
/// path into the CurseForge installation and so is the first thing to break:
/// it points at a drive that is no longer there, at a user folder from another
/// machine, or at an app that has since been uninstalled. When it is dead, the
/// file it names is looked for inside the instance folder itself — CurseForge
/// keeps a copy there — and failing that, the names packs conventionally use.
///
/// Returns `None` when the instance has no picture of its own, which leaves
/// the caller free to fall back to the modpack's thumbnail.
pub(super) async fn find_instance_image(
    curseforge_instance_folder: &Path,
    declared: Option<&Path>,
) -> Option<PathBuf> {
    if let Some(declared) = declared {
        if tokio::fs::try_exists(declared).await.unwrap_or(false) {
            return Some(declared.to_path_buf());
        }

        // A relative `profileImagePath`, or an absolute one whose file was
        // left behind in the folder that is being imported.
        if let Some(name) = declared.file_name() {
            let mut candidates = vec![curseforge_instance_folder.join(name)];
            candidates.extend(ICON_SUBFOLDERS.iter().map(|folder| {
                curseforge_instance_folder.join(folder).join(name)
            }));
            if let Some(found) = first_existing(candidates).await {
                return Some(found);
            }
        }
    }

    let mut folders = vec![curseforge_instance_folder.to_path_buf()];
    folders.extend(
        ICON_SUBFOLDERS
            .iter()
            .map(|folder| curseforge_instance_folder.join(folder)),
    );

    let mut candidates = Vec::new();
    for stem in ICON_FILE_STEMS {
        for extension in ICON_EXTENSIONS {
            for folder in &folders {
                candidates.push(folder.join(format!("{stem}.{extension}")));
            }
        }
    }
    if let Some(found) = first_existing(candidates).await {
        return Some(found);
    }

    // Nothing is named the way it usually is, so take any image sitting at the
    // top of the instance folder — a pack that ships one rarely buries it.
    let mut dir = io::read_dir(curseforge_instance_folder).await.ok()?;
    while let Ok(Some(entry)) = dir.next_entry().await {
        let path = entry.path();
        if path.is_file() && looks_like_image(&path) {
            return Some(path);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addon(json: &str) -> Option<ExpectedFile> {
        ExpectedFile::from_addon(serde_json::from_str(json).unwrap())
    }

    #[test]
    fn reads_the_shape_curseforge_writes() {
        let file = addon(
            r#"{
                "addonID": 238222,
                "installedFile": {
                    "id": 4712345,
                    "fileName": "jei-1.20.1.jar",
                    "fileNameOnDisk": "jei-1.20.1 (1).jar",
                    "downloadUrl": "https://edge.forgecdn.net/files/4712/345/jei.jar",
                    "fileLength": 1234,
                    "hashes": [
                        { "value": "deadbeef", "algo": 1 },
                        { "value": "ignored", "algo": 2 }
                    ]
                }
            }"#,
        )
        .expect("an addon with a file is expected");

        // The name on disk wins: it is the one the copy would have brought.
        assert_eq!(file.file_name, "jei-1.20.1 (1).jar");
        assert_eq!(file.project_id, Some(238222));
        assert_eq!(file.length, Some(1234));
        assert_eq!(file.sha1.as_deref(), Some("deadbeef"));
    }

    #[test]
    fn reads_the_shape_older_curseforge_wrote() {
        let file = addon(
            r#"{
                "AddonID": 306612,
                "InstalledFile": {
                    "id": 3900000,
                    "FileName": "fabric-api.jar",
                    "DownloadUrl": null,
                    "FileLength": 0
                }
            }"#,
        )
        .expect("an addon with a file is expected");

        assert_eq!(file.file_name, "fabric-api.jar");
        assert_eq!(file.download_url, None);
        // A length of zero is no length at all, and must not read as a file
        // that is supposed to be empty.
        assert_eq!(file.length, None);
    }

    #[test]
    fn an_addon_without_a_file_is_not_expected_on_disk() {
        assert!(addon(r#"{ "addonID": 1 }"#).is_none());
        assert!(
            addon(r#"{ "addonID": 1, "installedFile": { "id": 2 } }"#)
                .is_none()
        );
    }

    #[test]
    fn a_file_the_author_unlisted_is_still_on_the_cdn() {
        let file = addon(
            r#"{
                "addonID": 1,
                "installedFile": { "id": 4712345, "fileName": "some mod.jar" }
            }"#,
        )
        .unwrap();

        assert_eq!(
            file.download_candidates(),
            vec![
                "https://edge.forgecdn.net/files/4712/345/some%20mod.jar"
                    .to_string()
            ]
        );
    }

    #[test]
    fn a_url_that_is_not_https_is_not_followed() {
        let file = addon(
            r#"{
                "addonID": 1,
                "installedFile": {
                    "id": 1,
                    "fileName": "mod.jar",
                    "downloadUrl": "http://example.invalid/mod.jar"
                }
            }"#,
        )
        .unwrap();

        // Nor is a file id too small to be a real one turned into a CDN path.
        assert!(file.download_candidates().is_empty());
    }

    #[test]
    fn a_copy_is_only_trusted_at_the_length_the_pack_recorded() {
        let expected = addon(
            r#"{
                "addonID": 1,
                "installedFile": {
                    "id": 1,
                    "fileName": "mod.jar",
                    "fileLength": 100
                }
            }"#,
        )
        .unwrap();
        let present = |size| PresentFile {
            path: PathBuf::from("mods/mod.jar"),
            size,
            disabled: false,
        };

        assert!(is_intact(&expected, &present(100)));
        assert!(!is_intact(&expected, &present(99)));
        assert!(!is_intact(&expected, &present(0)));
    }

    #[test]
    fn a_pack_that_records_no_length_is_taken_at_its_word() {
        let expected = addon(
            r#"{
                "addonID": 1,
                "installedFile": { "id": 1, "fileName": "mod.jar" }
            }"#,
        )
        .unwrap();

        assert!(is_intact(
            &expected,
            &PresentFile {
                path: PathBuf::from("mods/mod.jar"),
                size: 7,
                disabled: false,
            }
        ));
    }

    #[tokio::test]
    async fn a_mod_that_is_switched_off_is_still_a_mod_that_is_there() {
        let folder = tempfile::tempdir().unwrap();
        let mods = folder.path().join("mods");
        std::fs::create_dir_all(&mods).unwrap();
        std::fs::write(mods.join("mod.jar.disabled"), b"jar").unwrap();

        let present = index_instance_files(folder.path()).await.unwrap();
        let file = present.get("mod.jar").expect("found under its own name");
        assert!(file.disabled);
        assert_eq!(file.size, 3);
    }

    #[tokio::test]
    async fn the_whole_copy_of_a_file_wins_over_the_empty_one() {
        let folder = tempfile::tempdir().unwrap();
        let mods = folder.path().join("mods");
        let disabled = folder.path().join("mods-disabled");
        std::fs::create_dir_all(&mods).unwrap();
        std::fs::create_dir_all(&disabled).unwrap();
        std::fs::write(mods.join("mod.jar"), b"jar").unwrap();
        std::fs::write(disabled.join("mod.jar"), b"").unwrap();

        let present = index_instance_files(folder.path()).await.unwrap();
        assert_eq!(present["mod.jar"].size, 3);
    }

    #[tokio::test]
    async fn a_dead_profile_image_path_still_finds_the_image() {
        let folder = tempfile::tempdir().unwrap();
        std::fs::write(folder.path().join("profileImage.png"), b"png").unwrap();

        let declared = PathBuf::from("D:/gone/curseforge/profileImage.png");
        let found = find_instance_image(folder.path(), Some(&declared)).await;
        assert_eq!(found, Some(folder.path().join("profileImage.png")));
    }

    #[tokio::test]
    async fn an_image_under_any_name_beats_no_image_at_all() {
        let folder = tempfile::tempdir().unwrap();
        std::fs::write(folder.path().join("thumbnail.webp"), b"webp").unwrap();
        std::fs::write(folder.path().join("minecraftinstance.json"), b"{}")
            .unwrap();

        let found = find_instance_image(folder.path(), None).await;
        assert_eq!(found, Some(folder.path().join("thumbnail.webp")));
    }

    #[tokio::test]
    async fn an_instance_with_no_image_says_so() {
        let folder = tempfile::tempdir().unwrap();
        std::fs::write(folder.path().join("minecraftinstance.json"), b"{}")
            .unwrap();

        assert_eq!(find_instance_image(folder.path(), None).await, None);
    }
}
