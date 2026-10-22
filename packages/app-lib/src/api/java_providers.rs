//! Noctrinth's own: where a Java runtime is downloaded from.
//!
//! Upstream only knows Azul, and Azul is not reachable everywhere — Java was
//! reported not installing at all from Russia. A runtime can also come from
//! Eclipse Temurin (through its own API, or straight from its GitHub releases,
//! which stay reachable where the API is not), from Mojang's own runtimes (the
//! ones the official launcher ships), or from those through the BMCLAPI
//! mirror. By default each is tried in that order until one works; the
//! settings can pin one.
//!
//! Every download is checked against the checksum its provider publishes:
//! SHA-256 for Azul and Temurin archives, SHA-1 for each of Mojang's files.

use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

use bytes::Bytes;
use futures::StreamExt;
use reqwest::Method;
use serde::Deserialize;

use crate::State;
use crate::util::fetch::{fetch_advanced, fetch_json};
use crate::util::io;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JavaProvider {
    Azul,
    Temurin,
    /// Temurin's releases on GitHub, without its API.
    TemurinGithub,
    Mojang,
    /// Mojang's runtimes, fetched through BMCLAPI.
    MojangMirror,
}

impl JavaProvider {
    /// The order tried when the settings leave the choice to the launcher.
    pub const AUTO_ORDER: [Self; 5] = [
        Self::Azul,
        Self::Temurin,
        Self::TemurinGithub,
        Self::Mojang,
        Self::MojangMirror,
    ];

    /// The value the settings store for it.
    pub fn key(self) -> &'static str {
        match self {
            Self::Azul => "azul",
            Self::Temurin => "temurin",
            Self::TemurinGithub => "temurin-github",
            Self::Mojang => "mojang",
            Self::MojangMirror => "mojang-mirror",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Azul => "Azul Zulu",
            Self::Temurin => "Eclipse Temurin",
            Self::TemurinGithub => "Eclipse Temurin (GitHub)",
            Self::Mojang => "Mojang",
            Self::MojangMirror => "Mojang (BMCLAPI)",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::AUTO_ORDER.into_iter().find(|p| p.key() == key)
    }

    /// The providers to try for a setting, in order: `auto`, or anything not
    /// known, is all of them.
    pub fn order_for_setting(setting: &str) -> Vec<Self> {
        Self::from_key(setting)
            .map_or_else(|| Self::AUTO_ORDER.to_vec(), |p| vec![p])
    }
}

/// How a runtime is fetched once it is known where it is.
#[derive(Debug, Clone)]
pub enum JavaDownload {
    /// One archive holding the runtime in a single top-level directory.
    Archive {
        url: String,
        sha256: Option<String>,
        tar_gz: bool,
    },
    /// One of Mojang's runtimes: a manifest listing every file, each fetched
    /// on its own and checked against its SHA-1.
    Files {
        manifest_url: String,
        manifest_sha1: String,
        /// The directory it goes into, named so the version can be read back
        /// off it: `mojang-java-runtime-delta-jre21.0.7`.
        directory: String,
        mirror: bool,
    },
}

impl JavaDownload {
    /// The address the first request goes to, for error reports.
    pub fn url(&self) -> &str {
        match self {
            Self::Archive { url, .. } => url,
            Self::Files { manifest_url, .. } => manifest_url,
        }
    }
}

const BMCLAPI: &str = "https://bmclapi2.bangbang93.com";
const MOJANG_RUNTIMES: &str = "https://launchermeta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json";

/// A Mojang URL as BMCLAPI serves it. BMCLAPI hands out Mojang's own lists
/// unchanged, Mojang's addresses and all, and expects clients to swap the
/// host themselves.
pub fn mirrored(url: &str) -> String {
    for host in [
        "https://piston-meta.mojang.com",
        "https://piston-data.mojang.com",
        "https://launchermeta.mojang.com",
        "https://launcher.mojang.com",
    ] {
        if let Some(rest) = url.strip_prefix(host) {
            return format!("{BMCLAPI}{rest}");
        }
    }
    url.to_string()
}

/// Where this provider has a runtime of this major version for this machine,
/// or `None` when it has none.
pub async fn resolve(
    provider: JavaProvider,
    java_version: u32,
    state: &State,
) -> crate::Result<Option<JavaDownload>> {
    match provider {
        JavaProvider::Azul => resolve_azul(java_version, state).await,
        JavaProvider::Temurin => resolve_temurin(java_version, state).await,
        JavaProvider::TemurinGithub => {
            resolve_temurin_github(java_version, state).await
        }
        JavaProvider::Mojang => {
            resolve_mojang(java_version, false, state).await
        }
        JavaProvider::MojangMirror => {
            resolve_mojang(java_version, true, state).await
        }
    }
}

/// The URL listing Azul's JRE packages for a major version here.
pub fn azul_packages_url(java_version: u32) -> String {
    format!(
        "https://api.azul.com/metadata/v1/zulu/packages?arch={}&java_version={}&os={}&archive_type=zip&javafx_bundled=false&java_package_type=jre&page_size=1&include_fields=sha256_hash",
        std::env::consts::ARCH,
        java_version,
        std::env::consts::OS
    )
}

async fn resolve_azul(
    java_version: u32,
    state: &State,
) -> crate::Result<Option<JavaDownload>> {
    #[derive(Deserialize)]
    struct Package {
        download_url: String,
        #[serde(default)]
        sha256_hash: Option<String>,
    }

    let packages = fetch_json::<Vec<Package>>(
        Method::GET,
        &azul_packages_url(java_version),
        None,
        None,
        None,
        &state.fetch_semaphore,
        &state.pool,
    )
    .await?;
    Ok(packages
        .into_iter()
        .next()
        .map(|package| JavaDownload::Archive {
            url: package.download_url,
            sha256: package.sha256_hash,
            tar_gz: false,
        }))
}

/// How Temurin names this machine: (os, architecture).
fn temurin_platform() -> Option<(&'static str, &'static str)> {
    let os = match std::env::consts::OS {
        "windows" => "windows",
        "linux" => "linux",
        "macos" => "mac",
        _ => return None,
    };
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x64",
        "x86" => "x86",
        "aarch64" => "aarch64",
        _ => return None,
    };
    Some((os, arch))
}

fn temurin_url(java_version: u32) -> Option<String> {
    let (os, arch) = temurin_platform()?;
    Some(format!(
        "https://api.adoptium.net/v3/assets/latest/{java_version}/hotspot?architecture={arch}&image_type=jre&os={os}&vendor=eclipse"
    ))
}

async fn resolve_temurin(
    java_version: u32,
    state: &State,
) -> crate::Result<Option<JavaDownload>> {
    #[derive(Deserialize)]
    struct Asset {
        binary: Binary,
    }
    #[derive(Deserialize)]
    struct Binary {
        package: Package,
    }
    #[derive(Deserialize)]
    struct Package {
        link: String,
        name: String,
        #[serde(default)]
        checksum: Option<String>,
    }

    let Some(url) = temurin_url(java_version) else {
        return Ok(None);
    };
    let assets = fetch_json::<Vec<Asset>>(
        Method::GET,
        &url,
        None,
        None,
        None,
        &state.fetch_semaphore,
        &state.pool,
    )
    .await?;
    Ok(assets.into_iter().find_map(|asset| {
        let package = asset.binary.package;
        let tar_gz = package.name.ends_with(".tar.gz");
        (tar_gz || package.name.ends_with(".zip")).then_some(
            JavaDownload::Archive {
                url: package.link,
                sha256: package.checksum,
                tar_gz,
            },
        )
    }))
}

/// Temurin's JRE archive for this machine among a release's assets, by its
/// name: `OpenJDK21U-jre_x64_windows_hotspot_21.0.5_11.zip`. Installers
/// (`.msi`, `.pkg`) and the files beside an archive are not it.
fn temurin_archive<'a>(
    names: impl IntoIterator<Item = &'a str>,
    java_version: u32,
    os: &str,
    arch: &str,
) -> Option<&'a str> {
    let prefix = format!("OpenJDK{java_version}U-jre_{arch}_{os}_hotspot_");
    names.into_iter().find(|name| {
        name.starts_with(&prefix)
            && (name.ends_with(".zip") || name.ends_with(".tar.gz"))
    })
}

async fn resolve_temurin_github(
    java_version: u32,
    state: &State,
) -> crate::Result<Option<JavaDownload>> {
    #[derive(Deserialize)]
    struct Release {
        assets: Vec<Asset>,
    }
    #[derive(Deserialize)]
    struct Asset {
        name: String,
        browser_download_url: String,
    }

    let Some((os, arch)) = temurin_platform() else {
        return Ok(None);
    };
    let release = fetch_json::<Release>(
        Method::GET,
        &format!(
            "https://api.github.com/repos/adoptium/temurin{java_version}-binaries/releases/latest"
        ),
        None,
        None,
        None,
        &state.fetch_semaphore,
        &state.pool,
    )
    .await?;
    let Some(name) = temurin_archive(
        release.assets.iter().map(|asset| asset.name.as_str()),
        java_version,
        os,
        arch,
    ) else {
        return Ok(None);
    };
    let url_of = |name: &str| {
        release
            .assets
            .iter()
            .find(|asset| asset.name == name)
            .map(|asset| asset.browser_download_url.clone())
    };
    let Some(url) = url_of(name) else {
        return Ok(None);
    };
    // The checksum is published beside the archive as `<name>.sha256.txt`:
    // the hash, then the file name.
    let sha256 = match url_of(&format!("{name}.sha256.txt")) {
        Some(checksum_url) => {
            let checksum = fetch_advanced(
                Method::GET,
                &checksum_url,
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
            String::from_utf8_lossy(&checksum)
                .split_whitespace()
                .next()
                .map(str::to_string)
        }
        None => None,
    };
    Ok(Some(JavaDownload::Archive {
        tar_gz: name.ends_with(".tar.gz"),
        url,
        sha256,
    }))
}

/// The platform Mojang files this machine's runtimes under.
fn mojang_platform() -> Option<&'static str> {
    Some(match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86_64") => "windows-x64",
        ("windows", "x86") => "windows-x86",
        ("windows", "aarch64") => "windows-arm64",
        ("linux", "x86_64") => "linux",
        ("linux", "x86") => "linux-i386",
        ("macos", "x86_64") => "mac-os",
        ("macos", "aarch64") => "mac-os-arm64",
        _ => return None,
    })
}

/// The major version a Mojang runtime's version name stands for: `21.0.7`
/// is 21, `8u51` is 8, and the old style `1.8.0_202` is 8 too.
fn major_of(version_name: &str) -> Option<u32> {
    let name = version_name.strip_prefix("1.").unwrap_or(version_name);
    let digits: String =
        name.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

#[derive(Deserialize)]
struct RuntimeEntry {
    manifest: RuntimeManifestRef,
    version: RuntimeVersion,
}

#[derive(Deserialize)]
struct RuntimeManifestRef {
    sha1: String,
    url: String,
}

#[derive(Deserialize)]
struct RuntimeVersion {
    name: String,
    #[serde(default)]
    released: String,
}

/// Mojang's newest runtime of this major for this platform, as
/// (component, entry), out of its list of every runtime.
fn pick_mojang_runtime(
    runtimes: HashMap<String, Vec<RuntimeEntry>>,
    java_version: u32,
) -> Option<(String, RuntimeEntry)> {
    runtimes
        .into_iter()
        .flat_map(|(component, entries)| {
            entries
                .into_iter()
                .map(move |entry| (component.clone(), entry))
        })
        .filter(|(_, entry)| {
            major_of(&entry.version.name) == Some(java_version)
        })
        .max_by(|(_, a), (_, b)| a.version.released.cmp(&b.version.released))
}

async fn resolve_mojang(
    java_version: u32,
    mirror: bool,
    state: &State,
) -> crate::Result<Option<JavaDownload>> {
    let Some(platform) = mojang_platform() else {
        return Ok(None);
    };
    let url = if mirror {
        mirrored(MOJANG_RUNTIMES)
    } else {
        MOJANG_RUNTIMES.to_string()
    };
    let mut all =
        fetch_json::<HashMap<String, HashMap<String, Vec<RuntimeEntry>>>>(
            Method::GET,
            &url,
            None,
            None,
            None,
            &state.fetch_semaphore,
            &state.pool,
        )
        .await?;
    let Some(runtimes) = all.remove(platform) else {
        return Ok(None);
    };
    Ok(
        pick_mojang_runtime(runtimes, java_version).map(
            |(component, entry)| {
                let manifest_url = if mirror {
                    mirrored(&entry.manifest.url)
                } else {
                    entry.manifest.url
                };
                JavaDownload::Files {
                    manifest_url,
                    manifest_sha1: entry.manifest.sha1,
                    directory: io::sanitize_filename(&format!(
                        "mojang-{component}-jre{}",
                        entry.version.name
                    )),
                    mirror,
                }
            },
        ),
    )
}

/// Checks an archive against the SHA-256 its provider published, when it
/// published one.
pub fn verify_sha256(
    bytes: &[u8],
    expected: Option<&str>,
) -> crate::Result<()> {
    use sha2::Digest;

    let Some(expected) = expected else {
        return Ok(());
    };
    let actual = format!("{:x}", sha2::Sha256::digest(bytes));
    if actual.eq_ignore_ascii_case(expected.trim()) {
        Ok(())
    } else {
        Err(crate::ErrorKind::InputError(format!(
            "The Java archive does not match its checksum (expected {expected}, got {actual})"
        ))
        .into())
    }
}

/// Unpacks a runtime archive into `root`, replacing an older copy of the same
/// runtime, and returns the runtime's own directory.
pub async fn unpack_archive(
    bytes: Bytes,
    tar_gz: bool,
    root: &Path,
) -> crate::Result<PathBuf> {
    let root = root.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let read_failed = |what: &str| {
            crate::Error::from(crate::ErrorKind::InputError(format!(
                "Failed to {what} the Java archive"
            )))
        };
        if tar_gz {
            let top = {
                let mut archive = tar::Archive::new(
                    flate2::read::GzDecoder::new(std::io::Cursor::new(&bytes)),
                );
                let mut entries =
                    archive.entries().map_err(|_| read_failed("read"))?;
                let first = entries
                    .next()
                    .ok_or_else(|| read_failed("read"))?
                    .map_err(|_| read_failed("read"))?;
                let path = first.path().map_err(|_| read_failed("read"))?;
                top_directory(&path).ok_or_else(|| read_failed("read"))?
            };
            let target = root.join(&top);
            if target.exists() {
                std::fs::remove_dir_all(&target)?;
            }
            tar::Archive::new(flate2::read::GzDecoder::new(
                std::io::Cursor::new(&bytes),
            ))
            .unpack(&root)
            .map_err(|_| read_failed("extract"))?;
            Ok(target)
        } else {
            let mut archive =
                zip::ZipArchive::new(std::io::Cursor::new(&bytes))
                    .map_err(|_| read_failed("read"))?;
            let top = archive
                .file_names()
                .next()
                .and_then(|name| top_directory(Path::new(name)))
                .ok_or_else(|| read_failed("read"))?;
            let target = root.join(&top);
            if target.exists() {
                std::fs::remove_dir_all(&target)?;
            }
            archive.extract(&root).map_err(|_| read_failed("extract"))?;
            Ok(target)
        }
    })
    .await
    .map_err(|e| {
        crate::Error::from(anyhow::anyhow!(format!(
            "Java extraction task failed: {e}"
        )))
    })?
}

/// The first component of an archive path, when it is a plain name.
fn top_directory(path: &Path) -> Option<String> {
    match path.components().next()? {
        Component::Normal(name) => Some(name.to_string_lossy().to_string()),
        _ => None,
    }
}

#[derive(Deserialize)]
struct RuntimeManifest {
    files: HashMap<String, RuntimeFile>,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum RuntimeFile {
    Directory,
    File {
        #[serde(default)]
        executable: bool,
        downloads: FileDownloads,
    },
    Link {
        #[cfg_attr(not(unix), allow(dead_code))]
        target: String,
    },
}

#[derive(Deserialize)]
struct FileDownloads {
    raw: RawDownload,
}

#[derive(Deserialize)]
struct RawDownload {
    sha1: String,
    size: u64,
    url: String,
}

/// A path out of a runtime manifest, made relative to the runtime's own
/// directory — or `None` when it would land anywhere else. The manifest may
/// come from a third-party mirror, so nothing in it is trusted to stay put.
fn runtime_relative_path(path: &str) -> Option<PathBuf> {
    // Mojang's macOS runtimes sit in a `jre.bundle`; the runtime's directory
    // is that bundle, as an archive's would be.
    let path = path.strip_prefix("jre.bundle/").unwrap_or(path);
    let relative = PathBuf::from(path);
    relative
        .components()
        .all(|component| matches!(component, Component::Normal(_)))
        .then_some(relative)
        .filter(|relative| relative.components().next().is_some())
}

/// How many of a runtime's files are fetched at once. The fetch semaphore
/// still bounds the connections; this only keeps the queue small.
const PARALLEL_FILES: usize = 16;

/// Downloads one of Mojang's runtimes file by file into `root`, replacing an
/// older copy of it, and returns its directory. `progress` hears the bytes
/// done and the bytes in all.
pub async fn install_files<F, Fut>(
    manifest_url: &str,
    manifest_sha1: &str,
    directory: &str,
    mirror: bool,
    root: &Path,
    state: &State,
    mut progress: F,
) -> crate::Result<PathBuf>
where
    F: FnMut(u64, u64) -> Fut,
    Fut: std::future::Future<Output = crate::Result<()>>,
{
    let manifest = fetch_advanced(
        Method::GET,
        manifest_url,
        Some(manifest_sha1),
        None,
        None,
        None,
        None,
        None,
        &state.fetch_semaphore,
        &state.pool,
    )
    .await?;
    let manifest: RuntimeManifest = serde_json::from_slice(&manifest)?;

    let target = root.join(directory);
    if target.exists() {
        io::remove_dir_all(&target).await?;
    }
    io::create_dir_all(&target).await?;

    let mut files = Vec::new();
    #[cfg_attr(not(unix), allow(unused_mut, unused_variables))]
    let mut links: Vec<(PathBuf, String)> = Vec::new();
    for (path, entry) in manifest.files {
        let Some(relative) = runtime_relative_path(&path) else {
            continue;
        };
        match entry {
            RuntimeFile::Directory => {
                io::create_dir_all(target.join(&relative)).await?;
            }
            RuntimeFile::File {
                executable,
                downloads,
            } => files.push((relative, executable, downloads.raw)),
            #[cfg(unix)]
            RuntimeFile::Link { target: link } => links.push((relative, link)),
            // Windows runtimes carry none, and a link there needs rights a
            // launcher should not ask for.
            #[cfg(not(unix))]
            RuntimeFile::Link { .. } => {}
        }
    }

    let total: u64 = files.iter().map(|(_, _, raw)| raw.size).sum();
    let mut done = 0;
    progress(done, total).await?;

    let mut downloads = futures::stream::iter(files.into_iter().map(
        |(relative, executable, raw)| {
            let path = target.join(relative);
            async move {
                let url = if mirror { mirrored(&raw.url) } else { raw.url };
                let bytes = fetch_advanced(
                    Method::GET,
                    &url,
                    Some(&raw.sha1),
                    None,
                    None,
                    None,
                    None,
                    None,
                    &state.fetch_semaphore,
                    &state.pool,
                )
                .await?;
                if let Some(parent) = path.parent() {
                    io::create_dir_all(parent).await?;
                }
                io::write(&path, &bytes).await?;
                #[cfg(unix)]
                if executable {
                    use std::os::unix::fs::PermissionsExt;
                    tokio::fs::set_permissions(
                        &path,
                        std::fs::Permissions::from_mode(0o755),
                    )
                    .await?;
                }
                #[cfg(not(unix))]
                let _ = executable;
                Ok::<_, crate::Error>(raw.size)
            }
        },
    ))
    .buffer_unordered(PARALLEL_FILES);

    while let Some(size) = downloads.next().await {
        done += size?;
        progress(done, total).await?;
    }
    drop(downloads);

    #[cfg(unix)]
    for (relative, link) in links {
        let path = target.join(&relative);
        if let Some(parent) = path.parent() {
            io::create_dir_all(parent).await?;
        }
        let _ = tokio::fs::remove_file(&path).await;
        tokio::fs::symlink(&link, &path).await?;
    }

    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_is_every_provider_in_order() {
        assert_eq!(
            JavaProvider::order_for_setting("auto"),
            JavaProvider::AUTO_ORDER.to_vec()
        );
        assert_eq!(
            JavaProvider::order_for_setting("something-else"),
            JavaProvider::AUTO_ORDER.to_vec()
        );
        assert_eq!(
            JavaProvider::order_for_setting("temurin"),
            vec![JavaProvider::Temurin]
        );
        for provider in JavaProvider::AUTO_ORDER {
            assert_eq!(JavaProvider::from_key(provider.key()), Some(provider));
        }
    }

    #[test]
    fn finds_temurins_jre_archive_among_release_assets() {
        let names = [
            "OpenJDK26U-jre_x64_windows_hotspot_26.0.2.1_1.msi",
            "OpenJDK26U-jre_x64_windows_hotspot_26.0.2.1_1.zip.sha256.txt",
            "OpenJDK26U-jdk_x64_windows_hotspot_26.0.2.1_1.zip",
            "OpenJDK26U-jre_x64_windows_hotspot_26.0.2.1_1.zip",
            "OpenJDK26U-jre_x64_linux_hotspot_26.0.2.1_1.tar.gz",
        ];
        assert_eq!(
            temurin_archive(names, 26, "windows", "x64"),
            Some("OpenJDK26U-jre_x64_windows_hotspot_26.0.2.1_1.zip")
        );
        assert_eq!(
            temurin_archive(names, 26, "linux", "x64"),
            Some("OpenJDK26U-jre_x64_linux_hotspot_26.0.2.1_1.tar.gz")
        );
        assert_eq!(temurin_archive(names, 26, "mac", "aarch64"), None);
        assert_eq!(
            temurin_archive(
                ["OpenJDK8U-jre_x64_windows_hotspot_8u482b08.zip"],
                8,
                "windows",
                "x64"
            ),
            Some("OpenJDK8U-jre_x64_windows_hotspot_8u482b08.zip")
        );
    }

    #[test]
    fn mirrors_only_mojangs_hosts() {
        assert_eq!(
            mirrored("https://piston-data.mojang.com/v1/objects/abc/java.exe"),
            "https://bmclapi2.bangbang93.com/v1/objects/abc/java.exe"
        );
        assert_eq!(
            mirrored(
                "https://piston-meta.mojang.com/v1/packages/x/manifest.json"
            ),
            "https://bmclapi2.bangbang93.com/v1/packages/x/manifest.json"
        );
        assert_eq!(
            mirrored("https://cdn.azul.com/zulu/bin/a.zip"),
            "https://cdn.azul.com/zulu/bin/a.zip"
        );
    }

    #[test]
    fn reads_majors_out_of_mojangs_version_names() {
        assert_eq!(major_of("21.0.7"), Some(21));
        assert_eq!(major_of("17.0.15"), Some(17));
        assert_eq!(major_of("8u51"), Some(8));
        assert_eq!(major_of("1.8.0_202"), Some(8));
        assert_eq!(major_of("snapshot"), None);
    }

    #[test]
    fn picks_the_newest_runtime_of_the_major() {
        let entry = |name: &str, released: &str| RuntimeEntry {
            manifest: RuntimeManifestRef {
                sha1: name.to_string(),
                url: String::new(),
            },
            version: RuntimeVersion {
                name: name.to_string(),
                released: released.to_string(),
            },
        };
        let runtimes = HashMap::from([
            (
                "java-runtime-beta".to_string(),
                vec![entry("17.0.1", "2021-10-01T00:00:00+00:00")],
            ),
            (
                "java-runtime-gamma".to_string(),
                vec![entry("17.0.8", "2023-08-01T00:00:00+00:00")],
            ),
            (
                "java-runtime-delta".to_string(),
                vec![entry("21.0.7", "2025-05-19T00:00:00+00:00")],
            ),
        ]);
        let (component, picked) = pick_mojang_runtime(runtimes, 17).unwrap();
        assert_eq!(component, "java-runtime-gamma");
        assert_eq!(picked.version.name, "17.0.8");
    }

    #[test]
    fn reads_mojangs_runtime_manifest() {
        // Entries as Mojang's manifests have them; links appear on macOS and
        // Linux only.
        let manifest: RuntimeManifest = serde_json::from_str(
            r#"{"files": {
                "bin": {"type": "directory"},
                "bin/java": {"type": "file", "executable": true, "downloads": {
                    "lzma": {"sha1": "b", "size": 1, "url": "https://piston-data.mojang.com/v1/objects/b/java"},
                    "raw": {"sha1": "a", "size": 2, "url": "https://piston-data.mojang.com/v1/objects/a/java"}}},
                "legal/LICENSE": {"type": "link", "target": "../LICENSE"}
            }}"#,
        )
        .unwrap();
        assert!(matches!(manifest.files["bin"], RuntimeFile::Directory));
        match &manifest.files["bin/java"] {
            RuntimeFile::File {
                executable,
                downloads,
            } => {
                assert!(executable);
                assert_eq!(downloads.raw.sha1, "a");
                assert_eq!(downloads.raw.size, 2);
            }
            _ => panic!("bin/java is a file"),
        }
        assert!(matches!(
            manifest.files["legal/LICENSE"],
            RuntimeFile::Link { .. }
        ));
    }

    #[test]
    fn keeps_manifest_paths_inside_the_runtime() {
        assert_eq!(
            runtime_relative_path("bin/java.exe"),
            Some(PathBuf::from("bin/java.exe"))
        );
        assert_eq!(
            runtime_relative_path("jre.bundle/Contents/Home/bin/java"),
            Some(PathBuf::from("Contents/Home/bin/java"))
        );
        assert_eq!(runtime_relative_path("../escape"), None);
        assert_eq!(runtime_relative_path("bin/../../escape"), None);
        assert_eq!(runtime_relative_path("/etc/passwd"), None);
        assert_eq!(runtime_relative_path(""), None);
    }

    #[test]
    fn checks_archives_against_their_sha256() {
        assert!(verify_sha256(b"anything", None).is_ok());
        assert!(verify_sha256(b"java", Some(&"0".repeat(64))).is_err());
        let actual = {
            use sha2::Digest;
            format!("{:x}", sha2::Sha256::digest(b"java"))
        };
        assert!(verify_sha256(b"java", Some(&actual.to_uppercase())).is_ok());
    }
}
