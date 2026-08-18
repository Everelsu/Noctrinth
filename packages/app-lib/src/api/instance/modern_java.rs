//! One-click modern-Java support for legacy Minecraft instances.
//!
//! Old Minecraft is pinned to Java 8 by Mojang's manifest and by LWJGL 2.
//! Projects that lift both limits ship their launcher-side half as
//! MultiMC/Prism version patches, which is what this module installs into an
//! instance for the [patch layer](crate::launcher::patches) to pick up.
//!
//! The two it knows about work in opposite ways, and [`PROVIDERS`] records the
//! difference. [lwjgl3ify](https://github.com/GTNewHorizons/lwjgl3ify) patches
//! the stock Forge at load time and needs its mod half installed alongside,
//! while [Cleanroom](https://github.com/CleanroomMC/Cleanroom) replaces Forge
//! outright and is nothing but patches.

use crate::launcher::patches;
use crate::state::{ModLoader, State};
use crate::util::fetch::fetch_advanced;
use crate::util::io;
use modrinth_content_management::{ContentType, ResolutionPreferences};
use reqwest::Method;
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use std::path::{Path, PathBuf};

/// Records what was installed, so the state can still be judged after the
/// instance itself changes underneath it.
const MARKER_FILE: &str = ".noctrinth-modern-java.json";

/// The Forge component bundled in an archive.
const FORGE_COMPONENT: &str = "net.minecraftforge.json";

/// Only lists the components; the launcher derives that from the patches.
const PACK_INDEX: &str = "mmc-pack.json";

/// Where the launcher-side archive and its version number come from.
#[derive(Debug, Clone, Copy)]
enum ArchiveSource {
    /// The mod is on Modrinth and publishes its launcher archive on GitHub
    /// under the same version number, so the two halves stay in step.
    ModrinthMod {
        project_id: &'static str,
        repo: &'static str,
        /// `{version}` is replaced with the resolved version.
        asset: &'static str,
    },
    /// The project only publishes GitHub releases.
    GitHubRelease {
        repo: &'static str,
        asset: &'static str,
    },
}

/// A way of running one legacy Minecraft version on a modern JVM.
#[derive(Debug, Clone, Copy)]
struct Provider {
    /// The Minecraft version this applies to.
    game_version: &'static str,
    /// Shown in the UI.
    name: &'static str,
    source: ArchiveSource,
    /// Whether the archive's Forge component *is* the loader.
    ///
    /// lwjgl3ify patches the stock Forge, so its bundled copy of one specific
    /// Forge version is dropped in favour of the one Daedalus builds — taking
    /// it would pin the wrong Forge and register `FMLTweaker` twice. Cleanroom
    /// is a Forge replacement, so its component has to be kept.
    forge_component_is_loader: bool,
}

/// Everything the launcher knows how to install, by Minecraft version.
const PROVIDERS: &[Provider] = &[
    Provider {
        game_version: "1.7.10",
        name: "lwjgl3ify",
        source: ArchiveSource::ModrinthMod {
            project_id: "eC4lt4Oy",
            repo: "GTNewHorizons/lwjgl3ify",
            asset: "lwjgl3ify-{version}-multimc.zip",
        },
        forge_component_is_loader: false,
    },
    Provider {
        game_version: "1.12.2",
        name: "Cleanroom",
        source: ArchiveSource::GitHubRelease {
            repo: "CleanroomMC/Cleanroom",
            asset: "cleanroom-{version}.zip",
        },
        forge_component_is_loader: true,
    },
];

fn provider_for(game_version: &str) -> Option<&'static Provider> {
    PROVIDERS
        .iter()
        .find(|provider| provider.game_version == game_version)
}

/// The Minecraft versions the launcher can do this for, for error messages.
fn supported_game_versions() -> String {
    PROVIDERS
        .iter()
        .map(|provider| provider.game_version)
        .collect::<Vec<_>>()
        .join(", ")
}

/// Whether an instance can run on a modern JVM, and whether it already does.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModernJavaStatus {
    /// Whether the instance is eligible at all.
    pub supported: bool,
    /// Why it is not eligible, for the UI to show verbatim.
    pub unsupported_reason: Option<String>,
    /// Whether modern-Java support is currently installed.
    pub installed: bool,
    /// What provides the support for this Minecraft version, e.g. `lwjgl3ify`
    /// or `Cleanroom`.
    pub provider_name: String,
    /// The installed version of that provider, if any.
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
    /// Which of the [`PROVIDERS`] was used.
    #[serde(default)]
    provider: Option<String>,
    /// The version of that provider.
    #[serde(default)]
    version: Option<String>,
    /// The Forge version the patches were built against, when the provider
    /// leaves the loader in place.
    #[serde(default)]
    forge_version: Option<String>,
    /// The patch files written, so removal takes back exactly those.
    #[serde(default)]
    patch_files: Vec<String>,
}

impl ModernJavaStatus {
    fn unsupported(reason: impl Into<String>) -> Self {
        Self {
            supported: false,
            unsupported_reason: Some(reason.into()),
            provider_name: String::new(),
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
    let Some(provider) = provider_for(&content_set.game_version) else {
        return Ok(ModernJavaStatus::unsupported(format!(
            "Modern Java support is available for Minecraft {}, and this instance runs {}.",
            supported_game_versions(),
            content_set.game_version
        )));
    };
    if content_set.loader != ModLoader::Forge {
        return Ok(ModernJavaStatus::unsupported(
            "Modern Java support requires the Forge loader.".to_string(),
        ));
    }

    let instance_path = instance_dir(&state, &metadata.instance.path);
    let mut status = read_installed_status(&instance_path)?;
    status.provider_name = provider.name.to_string();
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
    let provider = provider_for(&metadata.applied_content_set.game_version)
        .ok_or_else(|| {
            crate::ErrorKind::InputError(
                "Instance is not supported".to_string(),
            )
        })?;

    let release = resolve_release(&state, provider).await?;
    tracing::info!(
        "Installing {} {} into instance {instance_id}",
        provider.name,
        release.version
    );

    let archive = fetch_advanced(
        Method::GET,
        &release.archive_url,
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

    let extracted = extract_archive(&archive, &instance_path, provider).await?;
    io::write(
        &marker_path(&instance_path),
        &serde_json::to_vec(&InstallMarker {
            provider: Some(provider.name.to_string()),
            version: Some(release.version.clone()),
            forge_version: extracted.forge_version,
            patch_files: extracted.patch_files,
        })?,
    )
    .await?;

    // The mod half, where there is one. Cleanroom is the loader itself and has
    // nothing to install here; lwjgl3ify pulls UniMixins in as a declared
    // dependency.
    if let Some(mod_version_id) = release.mod_version_id {
        super::projects::install_project_with_dependencies(
            instance_id,
            super::projects::InstallProjectWithDependenciesRequest {
                project_id: release.mod_project_id.unwrap_or_default(),
                version_id: Some(mod_version_id),
                content_type: ContentType::Mod,
                selected: ResolutionPreferences::default(),
            },
        )
        .await?;
    }

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
    for component in read_marker(&instance_path)
        .map(|marker| marker.patch_files)
        .unwrap_or_default()
    {
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
///
/// A patch set counts as installed when it declares the Java versions it can
/// run on: that is the whole point of these archives, and it is the one signal
/// present whether the files came from this module or were dropped in by hand.
fn read_installed_status(
    instance_path: &Path,
) -> crate::Result<ModernJavaStatus> {
    let loaded = patches::load_instance_patches(instance_path)?;
    let mut java_majors = loaded
        .iter()
        .filter_map(|patch| patch.compatible_java_majors.as_ref())
        .flatten()
        .copied()
        .collect::<Vec<_>>();
    java_majors.sort_unstable();
    java_majors.dedup();

    if java_majors.is_empty() {
        return Ok(ModernJavaStatus {
            supported: true,
            unsupported_reason: None,
            // Filled in by the caller, which knows the instance's version.
            provider_name: String::new(),
            installed: false,
            installed_version: None,
            java_major: None,
            java_majors: Vec::new(),
            loader_warning: None,
        });
    }

    // The recorded version is the provider's own; a hand-copied set has none,
    // so fall back to the version of the component that carries the runtime
    // requirement.
    let installed_version = read_marker(instance_path)
        .and_then(|marker| marker.version)
        .or_else(|| {
            loaded
                .iter()
                .find(|patch| patch.compatible_java_majors.is_some())
                .and_then(|patch| patch.version.clone())
        });

    Ok(ModernJavaStatus {
        supported: true,
        unsupported_reason: None,
        provider_name: String::new(),
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

/// A resolved release: which version, where its archive is, and the mod half
/// to install alongside it if the provider has one.
struct Release {
    version: String,
    archive_url: String,
    mod_project_id: Option<String>,
    mod_version_id: Option<String>,
}

#[derive(Deserialize)]
struct ModrinthVersion {
    id: String,
    version_number: String,
}

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
}

/// Finds the newest release of a provider and where to fetch its archive.
async fn resolve_release(
    state: &State,
    provider: &Provider,
) -> crate::Result<Release> {
    match provider.source {
        ArchiveSource::ModrinthMod {
            project_id,
            repo,
            asset,
        } => {
            let url = format!(
                "{}project/{project_id}/version?game_versions=%5B%22{}%22%5D&loaders=%5B%22forge%22%5D",
                env!("MODRINTH_API_URL"),
                provider.game_version,
            );
            let versions =
                crate::util::fetch::fetch_json::<Vec<ModrinthVersion>>(
                    Method::GET,
                    &url,
                    None,
                    None,
                    None,
                    &state.api_semaphore,
                    &state.pool,
                )
                .await?;
            let version = versions.into_iter().next().ok_or_else(|| {
                crate::ErrorKind::InputError(format!(
                    "No {} release is available for this instance",
                    provider.name
                ))
            })?;

            Ok(Release {
                archive_url: release_asset_url(
                    repo,
                    &version.version_number,
                    asset,
                ),
                version: version.version_number,
                mod_project_id: Some(project_id.to_string()),
                mod_version_id: Some(version.id),
            })
        }
        ArchiveSource::GitHubRelease { repo, asset } => {
            let release = crate::util::fetch::fetch_json::<GitHubRelease>(
                Method::GET,
                &format!("https://api.github.com/repos/{repo}/releases/latest"),
                None,
                None,
                None,
                &state.api_semaphore,
                &state.pool,
            )
            .await?;

            Ok(Release {
                archive_url: release_asset_url(repo, &release.tag_name, asset),
                version: release.tag_name,
                mod_project_id: None,
                mod_version_id: None,
            })
        }
    }
}

/// The launcher-side archive lives on the upstream GitHub release even when the
/// mod itself is distributed through Modrinth.
fn release_asset_url(repo: &str, version: &str, asset: &str) -> String {
    format!(
        "https://github.com/{repo}/releases/download/{version}/{}",
        asset.replace("{version}", version)
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
    provider: &Provider,
) -> crate::Result<Extracted> {
    let mut archive =
        zip::ZipArchive::new(Cursor::new(archive)).map_err(|error| {
            crate::ErrorKind::InputError(format!(
                "Could not read the modern Java archive: {error}"
            ))
        })?;

    let mut extracted = Extracted::default();

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

        let file_name = name.file_name().and_then(|name| name.to_str());
        if file_name == Some(FORGE_COMPONENT)
            && !provider.forge_component_is_loader
        {
            extracted.forge_version =
                serde_json::from_slice::<BundledComponent>(&contents)
                    .ok()
                    .and_then(|component| component.version);
        }

        let Some(target) = archive_entry_target(&name, provider) else {
            continue;
        };

        if target.starts_with(patches::PATCHES_DIR)
            && let Some(file_name) = file_name
        {
            extracted.patch_files.push(file_name.to_string());
        }

        let path = instance_path.join(target);
        if let Some(parent) = path.parent() {
            io::create_dir_all(parent).await?;
        }
        io::write(&path, &contents).await?;
    }

    Ok(extracted)
}

/// What unpacking an archive produced, beyond the files themselves.
#[derive(Default)]
struct Extracted {
    /// The Forge version the patches were built against, when the provider
    /// leaves the loader in place.
    forge_version: Option<String>,
    /// The patch file names written.
    patch_files: Vec<String>,
}

#[derive(Deserialize)]
struct BundledComponent {
    version: Option<String>,
}

/// Maps an archive entry to its path inside the instance, or `None` if it
/// should not be written at all.
fn archive_entry_target(name: &Path, provider: &Provider) -> Option<PathBuf> {
    let file_name = name.file_name()?.to_str()?;
    if file_name == PACK_INDEX
        || (file_name == FORGE_COMPONENT && !provider.forge_component_is_loader)
    {
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

    fn provider(game_version: &str) -> &'static Provider {
        provider_for(game_version).unwrap()
    }

    #[test]
    fn only_patches_and_libraries_are_extracted() {
        let lwjgl3ify = provider("1.7.10");

        assert_eq!(
            archive_entry_target(
                Path::new("patches/org.lwjgl3.json"),
                lwjgl3ify
            ),
            Some(PathBuf::from("patches/org.lwjgl3.json"))
        );
        assert_eq!(
            archive_entry_target(
                Path::new("libraries/lwjgl3ify-3.0.31-forgePatches.jar"),
                lwjgl3ify
            ),
            Some(PathBuf::from("libraries/lwjgl3ify-3.0.31-forgePatches.jar"))
        );
        // Cleanroom's archive is a whole instance; the rest of it is not ours.
        let cleanroom = provider("1.12.2");
        assert_eq!(
            archive_entry_target(Path::new("mmc-pack.json"), cleanroom),
            None
        );
        assert_eq!(
            archive_entry_target(Path::new("instance.cfg"), cleanroom),
            None
        );
        assert_eq!(
            archive_entry_target(Path::new("cleanroom.png"), cleanroom),
            None
        );
    }

    #[test]
    fn the_forge_component_is_kept_only_when_it_is_the_loader() {
        // lwjgl3ify patches the stock Forge, so its bundled copy is dropped.
        assert_eq!(
            archive_entry_target(
                Path::new("patches/net.minecraftforge.json"),
                provider("1.7.10")
            ),
            None
        );
        // Cleanroom *is* the loader under that component ID, so it stays.
        assert_eq!(
            archive_entry_target(
                Path::new("patches/net.minecraftforge.json"),
                provider("1.12.2")
            ),
            Some(PathBuf::from("patches/net.minecraftforge.json"))
        );
        // The Minecraft component is kept either way: it carries
        // compatibleJavaMajors.
        assert_eq!(
            archive_entry_target(
                Path::new("patches/net.minecraft.json"),
                provider("1.7.10")
            ),
            Some(PathBuf::from("patches/net.minecraft.json"))
        );
    }

    fn instance_with_marker(forge_version: Option<&str>) -> tempfile::TempDir {
        let instance = tempfile::tempdir().unwrap();
        let marker = InstallMarker {
            forge_version: forge_version.map(str::to_string),
            ..Default::default()
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
        assert_eq!(
            archive_entry_target(
                Path::new("../../evil.json"),
                provider("1.7.10")
            ),
            None
        );
    }
}
