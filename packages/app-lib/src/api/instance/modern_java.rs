//! One-click modern-Java support for legacy Minecraft instances.
//!
//! Minecraft 1.7.10 is pinned to Java 8 by Mojang's manifest and by LWJGL 2.
//! [lwjgl3ify](https://github.com/GTNewHorizons/lwjgl3ify) lifts both limits,
//! and ships its launcher-side half as MultiMC/Prism version patches. This
//! module installs that half — the patches and the early-classpath jar — into
//! an instance so the [patch layer](crate::launcher::patches) can pick it up,
//! and installs the mod half through the normal content pipeline.

use crate::launcher::patches;
use crate::state::{ModLoader, State};
use crate::util::fetch::fetch_advanced;
use crate::util::io;
use modrinth_content_management::{ContentType, ResolutionPreferences};
use reqwest::Method;
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use std::path::{Path, PathBuf};

/// The Modrinth project the mod half of the support comes from. Its Modrinth
/// version numbers match the upstream release tags, which is what lets the
/// patches and the mod be installed at the same version.
const LWJGL3IFY_PROJECT_ID: &str = "eC4lt4Oy";

/// The only Minecraft version lwjgl3ify supports.
const SUPPORTED_GAME_VERSION: &str = "1.7.10";

/// The patch that identifies an installed setup, and carries its version.
const MARKER_PATCH: &str = "me.eigenraven.lwjgl3ify.forgepatches.json";

/// Patch components from the archive that the launcher does not want.
///
/// `net.minecraftforge` is a full copy of one specific Forge version's
/// component, which Prism needs because it has no other source of Forge
/// metadata. Noctrinth builds Forge from Daedalus instead, already with the
/// right libraries and the `FMLTweaker` entry, so taking the archive's copy
/// would pin the wrong Forge and register the tweaker twice. `mmc-pack.json`
/// only lists the components and has no counterpart here.
const SKIPPED_ENTRIES: &[&str] = &[BUNDLED_FORGE_COMPONENT, "mmc-pack.json"];

/// The Forge component bundled in the archive. Discarded, but its version says
/// which Forge the early-classpath jar was built to patch.
const BUNDLED_FORGE_COMPONENT: &str = "net.minecraftforge.json";

/// Records what was installed, so the state can still be judged after the
/// instance itself changes underneath it.
const MARKER_FILE: &str = ".noctrinth-modern-java.json";

/// The patch components this module writes, and therefore the only ones it
/// removes again.
const INSTALLED_PATCHES: &[&str] = &[
    MARKER_PATCH,
    "me.eigenraven.lwjgl3ify.launchargs.json",
    "net.minecraft.json",
    "org.lwjgl3.json",
];

/// Whether an instance can run on a modern JVM, and whether it already does.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModernJavaStatus {
    /// Whether the instance is eligible at all.
    pub supported: bool,
    /// Why it is not eligible, for the UI to show verbatim.
    pub unsupported_reason: Option<String>,
    /// Whether modern-Java support is currently installed.
    pub installed: bool,
    /// The installed lwjgl3ify version, if any.
    pub installed_version: Option<String>,
    /// The Java major version the instance will actually launch with.
    pub java_major: Option<u32>,
    /// Every Java major the installed patches declare support for, ascending.
    pub java_majors: Vec<u32>,
    /// Set when the instance's Forge is not the one the patches were built for.
    pub loader_warning: Option<String>,
}

/// What an install recorded about itself, kept beside the instance.
#[derive(Serialize, Deserialize, Debug, Default)]
struct InstallMarker {
    /// The Forge version the early-classpath jar patches.
    #[serde(default)]
    forge_version: Option<String>,
}

impl ModernJavaStatus {
    fn unsupported(reason: impl Into<String>) -> Self {
        Self {
            supported: false,
            unsupported_reason: Some(reason.into()),
            installed: false,
            installed_version: None,
            java_major: None,
            java_majors: Vec::new(),
            loader_warning: None,
        }
    }
}

/// Reports whether an instance can get, or already has, modern-Java support.
#[tracing::instrument]
pub async fn get_modern_java_status(
    instance_id: &str,
) -> crate::Result<ModernJavaStatus> {
    let state = State::get().await?;
    let metadata = super::get::get(instance_id).await?.ok_or_else(|| {
        crate::ErrorKind::InputError("Unknown instance".to_string())
    })?;

    let content_set = &metadata.applied_content_set;
    if content_set.game_version != SUPPORTED_GAME_VERSION {
        return Ok(ModernJavaStatus::unsupported(format!(
            "Modern Java support is only available for Minecraft {SUPPORTED_GAME_VERSION}, and this instance runs {}.",
            content_set.game_version
        )));
    }
    if content_set.loader != ModLoader::Forge {
        return Ok(ModernJavaStatus::unsupported(
            "Modern Java support requires the Forge loader.".to_string(),
        ));
    }

    let instance_path = instance_dir(&state, &metadata.instance.path);
    let mut status = read_installed_status(&instance_path)?;
    if !status.installed {
        return Ok(status);
    }

    status.java_majors =
        crate::api::jre::filter_installable_java_majors(&status.java_majors)
            .await;
    status.java_major = status.java_majors.first().copied();
    status.loader_warning =
        loader_warning(&instance_path, content_set.loader_version.as_deref());

    Ok(status)
}

/// Flags an instance whose Forge is no longer the one the early-classpath jar
/// was built to patch. It may still work, so this warns rather than blocks.
fn loader_warning(
    instance_path: &Path,
    loader_version: Option<&str>,
) -> Option<String> {
    let expected = read_marker(instance_path)?.forge_version?;
    let actual = loader_version?;

    (expected != actual).then(|| {
        format!(
            "These patches were built for Forge {expected}, but this instance runs Forge {actual}. Modern Java support may not work until the Forge version matches."
        )
    })
}

fn marker_path(instance_path: &Path) -> PathBuf {
    instance_path.join(MARKER_FILE)
}

fn read_marker(instance_path: &Path) -> Option<InstallMarker> {
    let contents = std::fs::read(marker_path(instance_path)).ok()?;
    serde_json::from_slice(&contents).ok()
}

/// Installs lwjgl3ify's launcher patches and mod into an instance.
#[tracing::instrument]
pub async fn install_modern_java(
    instance_id: &str,
) -> crate::Result<ModernJavaStatus> {
    let status = get_modern_java_status(instance_id).await?;
    if !status.supported {
        return Err(crate::ErrorKind::InputError(
            status
                .unsupported_reason
                .unwrap_or_else(|| "Instance is not supported".to_string()),
        )
        .into());
    }

    let state = State::get().await?;
    let metadata = super::get::get(instance_id).await?.ok_or_else(|| {
        crate::ErrorKind::InputError("Unknown instance".to_string())
    })?;
    let instance_path = instance_dir(&state, &metadata.instance.path);

    let version = latest_supported_version(&state).await?;
    tracing::info!(
        "Installing modern Java support {} into instance {instance_id}",
        version.version_number
    );

    let archive = fetch_advanced(
        Method::GET,
        &multimc_archive_url(&version.version_number),
        None,
        None,
        None,
        None,
        None,
        None,
        &state.fetch_semaphore,
        &state.pool,
    )
    .await?;

    let forge_version = extract_archive(&archive, &instance_path).await?;
    io::write(
        &marker_path(&instance_path),
        &serde_json::to_vec(&InstallMarker { forge_version })?,
    )
    .await?;

    // The mod half, with UniMixins pulled in as a declared dependency.
    super::projects::install_project_with_dependencies(
        instance_id,
        super::projects::InstallProjectWithDependenciesRequest {
            project_id: LWJGL3IFY_PROJECT_ID.to_string(),
            version_id: Some(version.id),
            content_type: ContentType::Mod,
            selected: ResolutionPreferences::default(),
        },
    )
    .await?;

    let status = get_modern_java_status(instance_id).await?;
    if let Some(java_major) = status.java_major {
        crate::api::jre::auto_install_java(java_major).await?;
    }

    // The patches swap LWJGL 2 for LWJGL 3 and add the early-classpath jar, so
    // the instance now depends on libraries it was never told to fetch. Without
    // this the next launch dies on the first missing native.
    crate::launcher::install_minecraft_for_instance_id_with_reporter(
        instance_id,
        false,
        None,
    )
    .await?;

    Ok(status)
}

/// Removes the launcher-side half again, putting the instance back on Java 8.
///
/// The mods stay: they are ordinary content and belong to the instance's mod
/// list, which is where the user manages them.
#[tracing::instrument]
pub async fn remove_modern_java(
    instance_id: &str,
) -> crate::Result<ModernJavaStatus> {
    let state = State::get().await?;
    let metadata = super::get::get(instance_id).await?.ok_or_else(|| {
        crate::ErrorKind::InputError("Unknown instance".to_string())
    })?;
    let instance_path = instance_dir(&state, &metadata.instance.path);

    // Only the files this module installs are removed: an instance may carry
    // hand-written patches or other local jars that are none of our business.
    let libraries_dir = patches::local_libraries_dir(&instance_path);
    for library in installed_local_libraries(&instance_path)? {
        let path = libraries_dir.join(patches::maven_file_name(&library));
        if path.is_file() {
            io::remove_file(&path).await?;
        }
    }

    let patches_dir = instance_path.join(patches::PATCHES_DIR);
    for component in INSTALLED_PATCHES {
        let path = patches_dir.join(component);
        if path.is_file() {
            io::remove_file(&path).await?;
        }
    }

    let marker = marker_path(&instance_path);
    if marker.is_file() {
        io::remove_file(&marker).await?;
    }

    remove_dir_if_empty(&patches_dir).await?;
    remove_dir_if_empty(&libraries_dir).await?;

    get_modern_java_status(instance_id).await
}

/// The maven coordinates of the instance-local jars the installed patches
/// reference, so removal can find them by the same name the classpath does.
fn installed_local_libraries(
    instance_path: &Path,
) -> crate::Result<Vec<String>> {
    Ok(patches::load_instance_patches(instance_path)?
        .iter()
        .flat_map(|patch| patch.libraries.iter().chain(&patch.added_libraries))
        .filter(|library| library.is_local())
        .map(|library| library.name.clone())
        .collect())
}

async fn remove_dir_if_empty(path: &Path) -> crate::Result<()> {
    if path.is_dir() && std::fs::read_dir(path)?.next().is_none() {
        io::remove_dir(path).await?;
    }

    Ok(())
}

fn instance_dir(state: &State, instance_path: &str) -> PathBuf {
    state.directories.instances_dir().join(instance_path)
}

/// Reads the installed state straight off disk, so a setup copied in by hand
/// from Prism is reported the same as one this module installed.
fn read_installed_status(
    instance_path: &Path,
) -> crate::Result<ModernJavaStatus> {
    let marker = instance_path.join(patches::PATCHES_DIR).join(MARKER_PATCH);
    if !marker.is_file() {
        return Ok(ModernJavaStatus {
            supported: true,
            unsupported_reason: None,
            installed: false,
            installed_version: None,
            java_major: None,
            java_majors: Vec::new(),
            loader_warning: None,
        });
    }

    let loaded = patches::load_instance_patches(instance_path)?;
    let installed_version = loaded
        .iter()
        .find(|patch| {
            patch.uid.as_deref() == Some("me.eigenraven.lwjgl3ify.forgepatches")
        })
        .and_then(|patch| patch.version.clone());
    let mut java_majors = loaded
        .iter()
        .filter_map(|patch| patch.compatible_java_majors.as_ref())
        .flatten()
        .copied()
        .collect::<Vec<_>>();
    java_majors.sort_unstable();
    java_majors.dedup();

    Ok(ModernJavaStatus {
        supported: true,
        unsupported_reason: None,
        installed: true,
        installed_version,
        // Without an explicit choice the launcher takes the lowest declared
        // runtime, which is the one the mod is tested against the most.
        java_major: java_majors.first().copied(),
        java_majors,
        // Filled in by the caller, which knows the instance's loader version.
        loader_warning: None,
    })
}

#[derive(Deserialize)]
struct ModrinthVersion {
    id: String,
    version_number: String,
}

/// Resolves the newest lwjgl3ify release that targets 1.7.10 on Forge.
async fn latest_supported_version(
    state: &State,
) -> crate::Result<ModrinthVersion> {
    let url = format!(
        "{}project/{LWJGL3IFY_PROJECT_ID}/version?game_versions=%5B%22{SUPPORTED_GAME_VERSION}%22%5D&loaders=%5B%22forge%22%5D",
        env!("MODRINTH_API_URL"),
    );

    let versions = crate::util::fetch::fetch_json::<Vec<ModrinthVersion>>(
        Method::GET,
        &url,
        None,
        None,
        None,
        &state.api_semaphore,
        &state.pool,
    )
    .await?;

    versions.into_iter().next().ok_or_else(|| {
        crate::ErrorKind::InputError(
            "No lwjgl3ify release is available for this instance".to_string(),
        )
        .into()
    })
}

/// The launcher-side archive lives on the upstream release, not on Modrinth,
/// but both are published under the same version number.
fn multimc_archive_url(version: &str) -> String {
    format!(
        "https://github.com/GTNewHorizons/lwjgl3ify/releases/download/{version}/lwjgl3ify-{version}-multimc.zip"
    )
}

/// Unpacks the `patches` and `libraries` folders of the archive into the
/// instance, dropping the entries the launcher supplies itself.
///
/// Returns the Forge version the archive was built against, read out of the
/// bundled Forge component on its way to the bin. The early-classpath jar
/// patches that exact Forge, so it is worth remembering even though the
/// component itself is discarded.
async fn extract_archive(
    archive: &[u8],
    instance_path: &Path,
) -> crate::Result<Option<String>> {
    let mut archive =
        zip::ZipArchive::new(Cursor::new(archive)).map_err(|error| {
            crate::ErrorKind::InputError(format!(
                "Could not read the modern Java archive: {error}"
            ))
        })?;

    let mut forge_version = None;

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|error| {
            crate::ErrorKind::InputError(format!(
                "Could not read the modern Java archive: {error}"
            ))
        })?;
        if entry.is_dir() {
            continue;
        }

        let Some(name) = entry.enclosed_name() else {
            continue;
        };

        let mut contents = Vec::with_capacity(entry.size() as usize);
        std::io::copy(&mut entry, &mut contents)?;

        if name.file_name().and_then(|name| name.to_str())
            == Some(BUNDLED_FORGE_COMPONENT)
        {
            forge_version =
                serde_json::from_slice::<BundledComponent>(&contents)
                    .ok()
                    .and_then(|component| component.version);
        }

        let Some(target) = archive_entry_target(&name) else {
            continue;
        };

        let path = instance_path.join(target);
        if let Some(parent) = path.parent() {
            io::create_dir_all(parent).await?;
        }
        io::write(&path, &contents).await?;
    }

    Ok(forge_version)
}

#[derive(Deserialize)]
struct BundledComponent {
    version: Option<String>,
}

/// Maps an archive entry to its path inside the instance, or `None` if it
/// should not be written at all.
fn archive_entry_target(name: &Path) -> Option<PathBuf> {
    let file_name = name.file_name()?.to_str()?;
    if SKIPPED_ENTRIES.contains(&file_name) {
        tracing::debug!("Skipping bundled component {file_name}");
        return None;
    }

    let mut components = name.components();
    let top_level = components.next()?.as_os_str().to_str()?;
    if top_level != patches::PATCHES_DIR
        && top_level != patches::LOCAL_LIBRARIES_DIR
    {
        return None;
    }

    Some(name.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_patches_and_libraries_are_extracted() {
        assert_eq!(
            archive_entry_target(Path::new("patches/org.lwjgl3.json")),
            Some(PathBuf::from("patches/org.lwjgl3.json"))
        );
        assert_eq!(
            archive_entry_target(Path::new(
                "libraries/lwjgl3ify-3.0.31-forgePatches.jar"
            )),
            Some(PathBuf::from("libraries/lwjgl3ify-3.0.31-forgePatches.jar"))
        );
        assert_eq!(archive_entry_target(Path::new("mmc-pack.json")), None);
        assert_eq!(archive_entry_target(Path::new("instance.cfg")), None);
    }

    #[test]
    fn the_bundled_forge_component_is_left_out() {
        assert_eq!(
            archive_entry_target(Path::new("patches/net.minecraftforge.json")),
            None
        );
        // The Minecraft component is kept: it carries compatibleJavaMajors.
        assert_eq!(
            archive_entry_target(Path::new("patches/net.minecraft.json")),
            Some(PathBuf::from("patches/net.minecraft.json"))
        );
    }

    fn instance_with_marker(forge_version: Option<&str>) -> tempfile::TempDir {
        let instance = tempfile::tempdir().unwrap();
        let marker = InstallMarker {
            forge_version: forge_version.map(str::to_string),
        };
        std::fs::write(
            marker_path(instance.path()),
            serde_json::to_vec(&marker).unwrap(),
        )
        .unwrap();
        instance
    }

    #[test]
    fn a_changed_forge_version_is_flagged() {
        let instance = instance_with_marker(Some("10.13.4.1614"));

        assert_eq!(loader_warning(instance.path(), Some("10.13.4.1614")), None);

        let warning = loader_warning(instance.path(), Some("10.13.4.1558"))
            .expect("a differing Forge version should warn");
        assert!(warning.contains("10.13.4.1614"));
        assert!(warning.contains("10.13.4.1558"));
    }

    #[test]
    fn nothing_is_flagged_without_something_to_compare() {
        // Hand-copied Prism setups have no marker.
        let instance = tempfile::tempdir().unwrap();
        assert_eq!(loader_warning(instance.path(), Some("10.13.4.1614")), None);

        // Neither does an archive that stopped bundling a Forge component.
        let instance = instance_with_marker(None);
        assert_eq!(loader_warning(instance.path(), Some("10.13.4.1614")), None);

        // Nor an instance with no loader version pinned.
        let instance = instance_with_marker(Some("10.13.4.1614"));
        assert_eq!(loader_warning(instance.path(), None), None);
    }

    #[test]
    fn archive_paths_cannot_escape_the_instance() {
        assert_eq!(archive_entry_target(Path::new("../../evil.json")), None);
    }
}
