//! Authentication flow interface
use crate::event::emit::{emit_loading, init_loading};
use crate::install::{
    InstallErrorContext, InstallJavaStep, InstallPhaseDetails, InstallPhaseId,
    InstallProgress, InstallProgressReporter,
};
use crate::state::JavaVersion;
use crate::util::fetch::{
    FetchProgressFn, fetch_advanced, fetch_advanced_with_progress, fetch_json,
};
use dashmap::DashMap;
use reqwest::Method;
use serde::Deserialize;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use sysinfo::{MemoryRefreshKind, RefreshKind};

use crate::util::io;
use crate::util::jre::extract_java_version;
use crate::{
    LoadingBarType, State,
    util::jre::{self},
};

pub async fn get_java_versions() -> crate::Result<DashMap<u32, JavaVersion>> {
    let state = State::get().await?;

    JavaVersion::get_all(&state.pool).await
}

pub async fn set_java_version(java_version: JavaVersion) -> crate::Result<()> {
    let state = State::get().await?;
    java_version.upsert(&state.pool).await?;
    Ok(())
}

// Searches for jres on the system given a java version (ex: 1.8, 1.17, 1.18)
// Allow higher allows for versions higher than the given version to be returned ('at least')
pub async fn find_filtered_jres(
    java_version: Option<u32>,
) -> crate::Result<Vec<JavaVersion>> {
    let jres = jre::get_all_jre().await?;

    // Filter out JREs that are not 1.17 or higher
    Ok(if let Some(java_version) = java_version {
        jres.into_iter()
            .filter(|jre| {
                let jre_version = extract_java_version(&jre.version);
                if let Ok(jre_version) = jre_version {
                    jre_version == java_version
                } else {
                    false
                }
            })
            .collect()
    } else {
        jres
    })
}

pub async fn auto_install_java(java_version: u32) -> crate::Result<PathBuf> {
    auto_install_java_with_loading(java_version, true).await
}

/// The URL listing the JRE packages available for a major version here.
fn java_packages_url(java_version: u32) -> String {
    format!(
        "https://api.azul.com/metadata/v1/zulu/packages?arch={}&java_version={}&os={}&archive_type=zip&javafx_bundled=false&java_package_type=jre&page_size=1",
        std::env::consts::ARCH,
        java_version,
        std::env::consts::OS
    )
}

/// Narrows a list of Java major versions to the ones that can actually be
/// installed on this platform.
///
/// A major that cannot be resolved is dropped, but one that could not be
/// checked at all is kept: a failed request means the network is down, not that
/// the runtime does not exist, and hiding choices over that would be worse than
/// letting the install report the problem.
pub async fn filter_installable_java_majors(candidates: &[u32]) -> Vec<u32> {
    #[derive(Deserialize)]
    struct Package {}

    let Ok(state) = State::get().await else {
        return candidates.to_vec();
    };

    let checks = candidates.iter().map(|&java_version| {
        let state = state.clone();
        async move {
            let packages = fetch_json::<Vec<Package>>(
                Method::GET,
                &java_packages_url(java_version),
                None,
                None,
                None,
                &state.fetch_semaphore,
                &state.pool,
            )
            .await;

            match packages {
                Ok(packages) => (java_version, !packages.is_empty()),
                Err(error) => {
                    tracing::debug!(
                        "Could not check whether Java {java_version} is available: {error}"
                    );
                    (java_version, true)
                }
            }
        }
    });

    futures::future::join_all(checks)
        .await
        .into_iter()
        .filter_map(|(java_version, available)| {
            available.then_some(java_version)
        })
        .collect()
}

pub async fn auto_install_java_with_loading(
    java_version: u32,
    show_loading: bool,
) -> crate::Result<PathBuf> {
    auto_install_java_inner(java_version, show_loading, None).await
}

pub async fn auto_install_java_with_reporter(
    java_version: u32,
    reporter: InstallProgressReporter,
) -> crate::Result<PathBuf> {
    auto_install_java_inner(java_version, false, Some(reporter)).await
}

const JAVA_INSTALL_STEPS: u64 = 4;
const JAVA_DOWNLOAD_PROGRESS_MIN_BYTES: u64 = 256 * 1024;

async fn update_java_install_progress(
    reporter: Option<&InstallProgressReporter>,
    java_version: u32,
    step: InstallJavaStep,
    progress: Option<InstallProgress>,
) -> crate::Result<()> {
    if let Some(reporter) = reporter {
        reporter
            .update(
                InstallPhaseId::PreparingJava,
                progress,
                InstallPhaseDetails::Java {
                    major_version: java_version,
                    step,
                },
            )
            .await?;
    }

    Ok(())
}

fn java_step_progress(current: u64) -> InstallProgress {
    InstallProgress {
        current,
        total: JAVA_INSTALL_STEPS,
        secondary: None,
    }
}

async fn auto_install_java_inner(
    java_version: u32,
    show_loading: bool,
    reporter: Option<InstallProgressReporter>,
) -> crate::Result<PathBuf> {
    let state = State::get().await?;

    let loading_bar = if show_loading {
        Some(
            init_loading(
                LoadingBarType::JavaDownload {
                    version: java_version,
                },
                100.0,
                "Downloading java version",
            )
            .await?,
        )
    } else {
        None
    };

    #[derive(Deserialize)]
    struct Package {
        pub download_url: String,
        pub name: PathBuf,
    }

    if let Some(loading_bar) = &loading_bar {
        emit_loading(loading_bar, 0.0, Some("Fetching java version"))?;
    }
    update_java_install_progress(
        reporter.as_ref(),
        java_version,
        InstallJavaStep::FetchingMetadata,
        Some(java_step_progress(1)),
    )
    .await?;
    let metadata_url = java_packages_url(java_version);
    if let Some(reporter) = &reporter {
        reporter
            .set_context(
                InstallErrorContext::new("fetch Java package metadata")
                    .urls(vec![metadata_url.clone()])
                    .java_version(java_version)
                    .os(std::env::consts::OS)
                    .arch(std::env::consts::ARCH)
                    .build(),
            )
            .await?;
    }
    let packages = fetch_json::<Vec<Package>>(
        Method::GET,
        &metadata_url,
        None,
        None,
        None,
        &state.fetch_semaphore,
        &state.pool,
    )
    .await?;
    if let Some(loading_bar) = &loading_bar {
        emit_loading(loading_bar, 10.0, Some("Downloading java version"))?;
    }

    if let Some(download) = packages.first() {
        if let Some(reporter) = &reporter {
            reporter
                .set_context(
                    InstallErrorContext::new("download Java archive")
                        .urls(vec![download.download_url.clone()])
                        .file_path(download.name.display().to_string())
                        .java_version(java_version)
                        .os(std::env::consts::OS)
                        .arch(std::env::consts::ARCH)
                        .build(),
                )
                .await?;
        }
        update_java_install_progress(
            reporter.as_ref(),
            java_version,
            InstallJavaStep::Downloading,
            None,
        )
        .await?;
        let file = if reporter.is_some() {
            let mut last_reported_bytes = 0_u64;
            let download_reporter = reporter.clone();
            let mut progress = move |current: u64,
                                     total: u64|
                  -> Pin<
                Box<dyn Future<Output = crate::Result<()>> + Send>,
            > {
                let min_delta =
                    (total / 200).max(JAVA_DOWNLOAD_PROGRESS_MIN_BYTES);
                if current < total
                    && current.saturating_sub(last_reported_bytes) < min_delta
                {
                    return Box::pin(async { Ok(()) });
                }

                last_reported_bytes = current;
                let reporter = download_reporter.clone();
                Box::pin(async move {
                    update_java_install_progress(
                        reporter.as_ref(),
                        java_version,
                        InstallJavaStep::Downloading,
                        Some(InstallProgress {
                            current,
                            total,
                            secondary: None,
                        }),
                    )
                    .await
                })
            };

            fetch_advanced_with_progress(
                Method::GET,
                &download.download_url,
                None,
                None,
                None,
                None,
                loading_bar.as_ref().map(|loading_bar| (loading_bar, 80.0)),
                None,
                &state.fetch_semaphore,
                &state.pool,
                Some(&mut progress as &mut FetchProgressFn<'_>),
            )
            .await?
        } else {
            fetch_advanced(
                Method::GET,
                &download.download_url,
                None,
                None,
                None,
                None,
                loading_bar.as_ref().map(|loading_bar| (loading_bar, 80.0)),
                None,
                &state.fetch_semaphore,
                &state.pool,
            )
            .await?
        };

        let path = state.directories.java_versions_dir();

        if let Some(reporter) = &reporter {
            reporter
                .set_context(
                    InstallErrorContext::new("read Java archive")
                        .urls(vec![download.download_url.clone()])
                        .file_path(download.name.display().to_string())
                        .target_path(path.display().to_string())
                        .java_version(java_version)
                        .os(std::env::consts::OS)
                        .arch(std::env::consts::ARCH)
                        .build(),
                )
                .await?;
        }
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(file))
            .map_err(|_| {
                crate::Error::from(crate::ErrorKind::InputError(
                    "Failed to read java zip".to_string(),
                ))
            })?;

        // removes the old installation of java
        if let Some(file) = archive.file_names().next()
            && let Some(dir) = file.split('/').next()
        {
            let path = path.join(dir);

            if path.exists() {
                io::remove_dir_all(path).await?;
            }
        }

        if let Some(loading_bar) = &loading_bar {
            emit_loading(loading_bar, 0.0, Some("Extracting java"))?;
        }
        update_java_install_progress(
            reporter.as_ref(),
            java_version,
            InstallJavaStep::Extracting,
            Some(java_step_progress(3)),
        )
        .await?;
        if let Some(reporter) = &reporter {
            reporter
                .set_context(
                    InstallErrorContext::new("extract Java archive")
                        .urls(vec![download.download_url.clone()])
                        .file_path(download.name.display().to_string())
                        .target_path(path.display().to_string())
                        .java_version(java_version)
                        .os(std::env::consts::OS)
                        .arch(std::env::consts::ARCH)
                        .build(),
                )
                .await?;
        }
        archive.extract(&path).map_err(|_| {
            crate::Error::from(crate::ErrorKind::InputError(
                "Failed to extract java zip".to_string(),
            ))
        })?;
        if let Some(loading_bar) = &loading_bar {
            emit_loading(loading_bar, 10.0, Some("Done extracting java"))?;
        }
        let mut base_path = path.join(
            download
                .name
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
        );

        #[cfg(target_os = "macos")]
        {
            base_path = base_path
                .join("Contents")
                .join("Home")
                .join("bin")
                .join("java")
        }

        #[cfg(not(target_os = "macos"))]
        {
            base_path = base_path.join("bin").join(jre::JAVA_BIN)
        }

        carry_gpu_preference_forward(java_version, &base_path).await;

        Ok(base_path)
    } else {
        Err(crate::ErrorKind::LauncherError(format!(
                    "No Java Version found for Java version {}, OS {}, and Architecture {}",
                    java_version, std::env::consts::OS, std::env::consts::ARCH,
                )).into())
    }
}

/// Gives a freshly installed runtime the GPU its predecessor was set to.
///
/// Windows keys the choice on the executable's full path, and every Java update
/// lands in a directory named after the new version — so without this, updating
/// Java silently hands the game back to the integrated GPU, having only ever
/// been configured once. Best-effort throughout: a launcher that cannot copy a
/// display preference has no business failing a Java install over it.
async fn carry_gpu_preference_forward(java_version: u32, new_java_path: &Path) {
    if !cfg!(target_os = "windows") {
        return;
    }

    let Ok(state) = State::get().await else {
        return;
    };
    let Ok(previous) = JavaVersion::get(java_version, &state.pool).await else {
        return;
    };
    let Some(previous) = previous else { return };

    // The same path means nothing moved, so there is nothing to carry.
    let previous_path = PathBuf::from(&previous.path);
    if previous_path == new_java_path {
        return;
    }

    let preference = match crate::api::gpu::get_preference(&previous_path) {
        Ok(preference) => preference,
        Err(error) => {
            tracing::warn!(
                "Could not read the previous GPU preference: {error}"
            );
            return;
        }
    };

    if let Err(error) =
        crate::api::gpu::inherit_preference(new_java_path, preference)
    {
        tracing::warn!("Could not carry the GPU preference forward: {error}");
    }
}

// Validates JRE at a given at a given path
pub async fn check_jre(path: PathBuf) -> crate::Result<JavaVersion> {
    jre::check_java_at_filepath(&path).await
}

// Test JRE at a given path
pub async fn test_jre(
    path: PathBuf,
    major_version: u32,
) -> crate::Result<bool> {
    let jre = match jre::check_java_at_filepath(&path).await {
        Ok(jre) => jre,
        Err(e) => {
            tracing::warn!("Invalid Java at {}: {e}", path.display());
            return Ok(false);
        }
    };
    let version = extract_java_version(&jre.version)?;
    tracing::info!(
        "Expected Java version {major_version}, and found {version} at {}",
        path.display()
    );
    Ok(version == major_version)
}

fn system_memory_bytes() -> u64 {
    sysinfo::System::new_with_specifics(
        RefreshKind::nothing()
            .with_memory(MemoryRefreshKind::nothing().with_ram()),
    )
    .total_memory()
}

/// Recommended default max heap (MiB) for new instances based on system RAM.
pub fn default_memory_max_mb() -> u32 {
    const BYTES_PER_GIB: u64 = 1024 * 1024 * 1024;
    let system_gib = system_memory_bytes() / BYTES_PER_GIB;

    if system_gib < 8 {
        1024 * 2
    } else if system_gib >= 24 {
        1024 * 6
    } else {
        1024 * 4
    }
}

// Gets maximum memory in KiB.
pub async fn get_max_memory() -> crate::Result<u64> {
    Ok(system_memory_bytes() / 1024)
}

/// A Java runtime the launcher downloaded, as it sits on disk.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct InstalledRuntime {
    /// The runtime's own directory, and what removal takes.
    pub path: String,
    /// The executable inside it, which is what settings point at.
    pub java_path: String,
    /// The directory name, e.g. `zulu25.36.15-ca-jre25.0.4-win_x64`.
    pub name: String,
    /// Parsed out of the name; `None` when it doesn't follow the usual shape.
    pub major_version: Option<u32>,
    pub size_bytes: u64,
    /// Whether a Java setting currently points at this runtime.
    pub in_use: bool,
}

/// Reads the major version out of an Azul directory name.
///
/// These arrive as `zulu25.36.15-ca-jre25.0.4-win_x64`, where the part after
/// `-jre` is the Java version proper — the leading `zulu25.36.15` is Azul's own
/// build number and only coincidentally starts the same way. Anything that
/// doesn't match is left unlabelled rather than guessed at.
fn major_version_from_runtime_name(name: &str) -> Option<u32> {
    let after_marker = name.split("-jre").nth(1)?;
    let digits: String = after_marker
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    digits.parse().ok()
}

async fn directory_size(path: &Path) -> u64 {
    let mut total = 0;
    let mut stack = vec![path.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let Ok(mut entries) = tokio::fs::read_dir(&dir).await else {
            continue;
        };
        while let Ok(Some(entry)) = entries.next_entry().await {
            let Ok(metadata) = entry.metadata().await else {
                continue;
            };
            if metadata.is_dir() {
                stack.push(entry.path());
            } else {
                total += metadata.len();
            }
        }
    }

    total
}

/// Every runtime under the launcher's own Java directory.
///
/// Nothing prunes these: each update unpacks into a directory named after the
/// new version and the previous one stays behind forever, so a long-lived
/// install accumulates a copy of every Java it has ever downloaded.
pub async fn list_installed_runtimes() -> crate::Result<Vec<InstalledRuntime>> {
    let state = State::get().await?;
    let root = state.directories.java_versions_dir();

    let configured = JavaVersion::get_all(&state.pool).await?;
    let configured_paths: Vec<PathBuf> = configured
        .iter()
        .map(|entry| PathBuf::from(&entry.value().path))
        .collect();

    let mut runtimes = Vec::new();
    let Ok(mut entries) = tokio::fs::read_dir(&root).await else {
        return Ok(runtimes);
    };

    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if !entry.metadata().await.map(|m| m.is_dir()).unwrap_or(false) {
            continue;
        }

        let name = entry.file_name().to_string_lossy().to_string();
        let java_path = runtime_executable(&path);

        // A configured path can be the executable anywhere inside this
        // directory, so compare by containment rather than by equality.
        let in_use = configured_paths
            .iter()
            .any(|configured| configured.starts_with(&path));

        runtimes.push(InstalledRuntime {
            major_version: major_version_from_runtime_name(&name),
            size_bytes: directory_size(&path).await,
            java_path: java_path.to_string_lossy().to_string(),
            path: path.to_string_lossy().to_string(),
            name,
            in_use,
        });
    }

    // Biggest first: the point of this list is reclaiming space.
    runtimes.sort_by_key(|runtime| std::cmp::Reverse(runtime.size_bytes));
    Ok(runtimes)
}

/// Where the executable lives inside a runtime directory.
fn runtime_executable(runtime_dir: &Path) -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        runtime_dir
            .join("Contents")
            .join("Home")
            .join("bin")
            .join("java")
    }
    #[cfg(not(target_os = "macos"))]
    {
        runtime_dir.join("bin").join(jre::JAVA_BIN)
    }
}

/// Deletes a downloaded runtime.
///
/// Guarded twice over, because this removes a directory tree: the path must sit
/// inside the launcher's own Java directory, and it must not be one a Java
/// setting still points at. A caller is free to pass anything, and neither the
/// user's own JDKs nor a runtime in use should ever be reachable from here.
pub async fn remove_installed_runtime(path: PathBuf) -> crate::Result<()> {
    let state = State::get().await?;
    let root = state.directories.java_versions_dir();

    // Resolve both sides before comparing, so `..` cannot walk out.
    let canonical_root = io::canonicalize(&root)?;
    let canonical_target = io::canonicalize(&path)?;

    if !canonical_target.starts_with(&canonical_root)
        || canonical_target == canonical_root
    {
        return Err(crate::ErrorKind::InputError(format!(
            "Refusing to remove {} — it is not a runtime this launcher installed",
            path.display()
        ))
        .into());
    }

    let configured = JavaVersion::get_all(&state.pool).await?;
    for entry in &configured {
        let configured_path = PathBuf::from(&entry.value().path);
        if io::canonicalize(&configured_path)
            .map(|resolved| resolved.starts_with(&canonical_target))
            .unwrap_or(false)
        {
            return Err(crate::ErrorKind::InputError(format!(
                "Java {} is set to use this runtime; point it elsewhere first",
                entry.key()
            ))
            .into());
        }
    }

    // Drop the GPU preference too, or the registry keeps an entry for an
    // executable that no longer exists.
    let _ = crate::api::gpu::set_preference(
        &runtime_executable(&canonical_target),
        crate::api::gpu::GpuPreference::Auto,
    );

    io::remove_dir_all(&canonical_target).await?;
    tracing::info!("Removed Java runtime at {}", canonical_target.display());

    Ok(())
}

/// Deletes every runtime nothing points at, returning the bytes reclaimed.
pub async fn remove_unused_runtimes() -> crate::Result<u64> {
    let mut freed = 0;

    for runtime in list_installed_runtimes().await? {
        if runtime.in_use {
            continue;
        }
        match remove_installed_runtime(PathBuf::from(&runtime.path)).await {
            Ok(()) => freed += runtime.size_bytes,
            // One stubborn directory — a file held open, say — should not stop
            // the rest from being cleared.
            Err(error) => tracing::warn!(
                "Could not remove unused runtime {}: {error}",
                runtime.name
            ),
        }
    }

    Ok(freed)
}

#[cfg(test)]
mod runtime_name_tests {
    use super::major_version_from_runtime_name;

    #[test]
    fn reads_the_java_version_not_azuls_build_number() {
        // The leading number is Azul's build; the Java version follows `-jre`.
        // These two agree for 25 and disagree for 8, which is the whole point.
        assert_eq!(
            major_version_from_runtime_name(
                "zulu25.36.15-ca-jre25.0.4-win_x64"
            ),
            Some(25)
        );
        assert_eq!(
            major_version_from_runtime_name(
                "zulu8.94.0.17-ca-jre8.0.492-win_x64"
            ),
            Some(8)
        );
        assert_eq!(
            major_version_from_runtime_name(
                "zulu17.68.17-ca-jre17.0.20-win_x64"
            ),
            Some(17)
        );
        assert_eq!(
            major_version_from_runtime_name(
                "zulu26.32.13-ca-jre26.0.2-win_x64"
            ),
            Some(26)
        );
    }

    #[test]
    fn leaves_unfamiliar_names_unlabelled() {
        assert_eq!(major_version_from_runtime_name("some-custom-jdk"), None);
        assert_eq!(major_version_from_runtime_name(""), None);
        // A JDK rather than a JRE does not carry the `-jre` marker.
        assert_eq!(
            major_version_from_runtime_name(
                "zulu21.50.19-ca-jdk21.0.11-win_x64"
            ),
            None
        );
    }
}
