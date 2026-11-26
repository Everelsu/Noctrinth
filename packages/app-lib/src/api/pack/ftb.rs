use std::collections::{HashMap, HashSet};
use std::io::{Cursor, Write};

use reqwest::{Method, Url};
use serde::Deserialize;

use super::install_from::{
    CreatePack, CreatePackDescription, CreatePackFile, CreatePackInstance,
    PackDependency, PackFile, PackFileHash, PackFormat,
};
use crate::{
    ErrorKind, State,
    util::fetch::{REQWEST_CLIENT, fetch_advanced_with_client},
};

#[derive(Deserialize)]
struct Manifest {
    id: u64,
    parent: u64,
    name: String,
    private: bool,
    targets: Vec<Target>,
    files: Vec<File>,
}

#[derive(Deserialize)]
struct Target {
    name: String,
    version: String,
    #[serde(rename = "type")]
    kind: String,
}

#[derive(Deserialize)]
struct File {
    path: String,
    name: String,
    url: Option<String>,
    sha1: Option<String>,
    hashes: Option<FileHashes>,
    size: u32,
    #[serde(default)]
    serveronly: bool,
    #[serde(default)]
    optional: bool,
}

#[derive(Deserialize)]
struct FileHashes {
    sha512: Option<String>,
}

fn invalid(message: impl Into<String>) -> crate::Error {
    ErrorKind::InputError(message.into()).into()
}

fn dependencies(
    targets: &[Target],
) -> crate::Result<HashMap<PackDependency, String>> {
    let mut result = HashMap::new();
    let mut has_loader = false;
    for target in targets {
        let dependency = match (target.kind.as_str(), target.name.as_str()) {
            ("game", "minecraft") => PackDependency::Minecraft,
            ("modloader", name) => {
                if has_loader {
                    return Err(invalid(
                        "FTB manifest lists more than one mod loader",
                    ));
                }
                has_loader = true;
                match name {
                    "forge" => PackDependency::Forge,
                    "neoforge" => PackDependency::NeoForge,
                    "fabric" => PackDependency::FabricLoader,
                    "quilt" => PackDependency::QuiltLoader,
                    _ => {
                        return Err(invalid(format!(
                            "Unsupported FTB mod loader: {name}"
                        )));
                    }
                }
            }
            _ => continue,
        };
        if target.version.trim().is_empty()
            || result.insert(dependency, target.version.clone()).is_some()
        {
            return Err(invalid("Invalid FTB target version"));
        }
    }
    if !result.contains_key(&PackDependency::Minecraft) {
        return Err(invalid(
            "FTB manifest does not specify a Minecraft version",
        ));
    }
    Ok(result)
}

fn checked_path(directory: &str, name: &str) -> crate::Result<String> {
    if directory.starts_with('/') {
        return Err(invalid("FTB file directory must be relative"));
    }
    let directory = directory.strip_prefix("./").unwrap_or(directory);
    let directory = directory.trim_end_matches('/');
    let path = match directory {
        "" | "." => name.to_owned(),
        _ => format!("{directory}/{name}"),
    };
    if name.contains('/')
        || path.split('/').any(|part| {
            part.is_empty()
                || matches!(part, "." | "..")
                || part.ends_with([' ', '.'])
                || part
                    .chars()
                    .any(|c| c.is_control() || "\\:*?\"<>|".contains(c))
        })
    {
        return Err(invalid(format!("Invalid FTB file path: {path}")));
    }
    Ok(path)
}

fn pack_file(
    file: File,
    seen: &mut HashSet<String>,
) -> crate::Result<PackFile> {
    let path = checked_path(&file.path, &file.name)?;
    if !seen.insert(path.to_lowercase()) {
        return Err(invalid(format!("Duplicate FTB file path: {path}")));
    }
    let url =
        Url::parse(file.url.as_deref().unwrap_or_default()).map_err(|_| {
            invalid(format!("FTB did not provide a download for {path}"))
        })?;
    if url.scheme() != "https"
        || !matches!(
            url.host_str(),
            Some(
                "files.feed-the-beast.com"
                    | "cdn.feed-the-beast.com"
                    | "edge.forgecdn.net"
                    | "media.forgecdn.net"
            )
        )
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
    {
        return Err(invalid(format!(
            "Unsupported download URL for FTB file {path}"
        )));
    }
    let usable_hash = |hash: &str, length| {
        hash.len() == length
            && hash.bytes().all(|byte| byte.is_ascii_hexdigit())
    };
    let sha1 = file.sha1.filter(|hash| usable_hash(hash, 40));
    let sha512 = file
        .hashes
        .and_then(|hashes| hashes.sha512)
        .filter(|hash| usable_hash(hash, 128));
    let (Some(sha1), Some(sha512)) = (sha1, sha512) else {
        return Err(invalid(format!(
            "Missing or invalid checksum for FTB file {path}"
        )));
    };

    Ok(PackFile {
        path: path_util::SafeRelativeUtf8UnixPathBuf::try_from(path)
            .map_err(|error| invalid(error.to_string()))?,
        hashes: HashMap::from([
            (PackFileHash::Sha1, sha1.to_ascii_lowercase()),
            (PackFileHash::Sha512, sha512.to_ascii_lowercase()),
        ]),
        env: None,
        downloads: vec![url.into()],
        file_size: file.size,
    })
}

pub(super) fn preview(
    title: String,
    icon_url: Option<String>,
) -> CreatePackInstance {
    CreatePackInstance {
        name: title,
        icon_url,
        ..Default::default()
    }
}

pub(crate) async fn create_pack(
    pack_id: u64,
    version_id: u64,
    title: String,
    instance_id: String,
) -> crate::Result<CreatePack> {
    let state = State::get().await?;
    let bytes = fetch_advanced_with_client(
        Method::GET,
        &format!("https://api.feed-the-beast.com/v1/modpacks/public/modpack/{pack_id}/{version_id}"),
        None,
        None,
        Some(("User-Agent", "Noctrinth (+https://github.com/Everelsu/Noctrinth)")),
        None,
        None,
        None,
        &state.fetch_semaphore,
        &state.pool,
        &REQWEST_CLIENT,
    ).await?;
    let manifest: Manifest = serde_json::from_slice(&bytes)?;
    if manifest.id != version_id
        || manifest.parent != pack_id
        || manifest.private
    {
        return Err(invalid(
            "FTB returned a different or non-public pack version",
        ));
    }
    let dependencies = dependencies(&manifest.targets)?;
    let mut seen = HashSet::new();
    let files = manifest
        .files
        .into_iter()
        .filter(|file| !file.serveronly && !file.optional)
        .map(|file| pack_file(file, &mut seen))
        .collect::<crate::Result<Vec<_>>>()?;
    let index = PackFormat {
        game: "minecraft".into(),
        format_version: 1,
        version_id: manifest.name,
        name: title,
        summary: None,
        files,
        dependencies,
    };
    let mut archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
    archive.start_file(
        "modrinth.index.json",
        zip::write::SimpleFileOptions::default(),
    )?;
    archive.write_all(&serde_json::to_vec(&index)?)?;

    Ok(CreatePack {
        file: CreatePackFile::Bytes(archive.finish()?.into_inner().into()),
        description: CreatePackDescription {
            icon: None,
            override_title: None,
            project_id: None,
            version_id: None,
            instance_id,
            source_filename: Some(format!("ftb-{pack_id}-{version_id}.mrpack")),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restricts_manifest_file_paths() {
        assert_eq!(checked_path("./mods", "mod.jar").unwrap(), "mods/mod.jar");
        assert_eq!(checked_path("./", "options.txt").unwrap(), "options.txt");
        for (directory, name) in [
            ("../mods", "x.jar"),
            ("/mods", "x.jar"),
            ("/", "x.jar"),
            ("C:/mods", "x.jar"),
            (".", "../x.jar"),
            ("mods", "x\\y.jar"),
        ] {
            assert!(checked_path(directory, name).is_err());
        }
    }
}
