use crate::state::{ProjectType, file_hash_cache_key, file_modified_at_ns};
use crate::util::io::{self, IOError};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub(crate) struct ScannedContentFile {
    pub relative_path: String,
    pub file_name: String,
    pub enabled: bool,
    pub size: u64,
    pub hash_cache_key: String,
    pub is_symlink: bool,
    pub has_linked_parent: bool,
}

pub(crate) fn scan_content_files(
    instances_dir: &Path,
    instance_path: &str,
) -> crate::Result<Vec<ScannedContentFile>> {
    scan_content_paths(instances_dir, instance_path, None)
}

pub(crate) fn scan_content_paths(
    instances_dir: &Path,
    instance_path: &str,
    paths: Option<&std::collections::HashSet<String>>,
) -> crate::Result<Vec<ScannedContentFile>> {
    crate::state::content_store::validate_instance_path(instance_path)?;
    let instance_full_path = instances_dir.join(instance_path);
    let instance_dir = io::canonicalize(instance_full_path)?;
    let linked_instance =
        instance_dir != io::canonicalize(instances_dir)?.join(instance_path);
    let mut files = Vec::new();

    for project_type in ProjectType::iterator() {
        let folder = project_type.get_folder();
        if paths.is_some_and(|paths| {
            !paths
                .iter()
                .any(|path| path.starts_with(&format!("{folder}/")))
        }) {
            continue;
        }
        let folder_path = instance_dir.join(folder);
        let has_linked_parent = linked_instance
            || std::fs::symlink_metadata(&folder_path)
                .is_ok_and(|metadata| metadata.file_type().is_symlink());

        if !folder_path.exists() {
            continue;
        }

        let candidates = if let Some(paths) = paths {
            paths
                .iter()
                .filter(|path| path.starts_with(&format!("{folder}/")))
                .flat_map(|path| {
                    [
                        instance_dir.join(path),
                        instance_dir.join(format!("{path}.disabled")),
                    ]
                })
                .collect::<Vec<_>>()
        } else {
            std::fs::read_dir(&folder_path)
                .map_err(|err| IOError::with_path(err, &folder_path))?
                .map(|entry| {
                    entry.map(|entry| entry.path()).map_err(IOError::from)
                })
                .collect::<std::result::Result<Vec<_>, _>>()?
        };
        for path in candidates {
            if !path.is_file() {
                continue;
            }

            let Some(file_name) =
                path.file_name().and_then(|value| value.to_str())
            else {
                continue;
            };

            if !is_scannable_project_file(project_type, file_name) {
                continue;
            }

            let metadata = path.metadata().map_err(IOError::from)?;
            let size = metadata.len();
            let modified_at_ns =
                file_modified_at_ns(&metadata).map_err(IOError::from)?;
            let relative_path = format!("{folder}/{file_name}");
            let hash_cache_key = file_hash_cache_key(
                size,
                modified_at_ns,
                &format!("{instance_path}/{relative_path}"),
            );

            files.push(ScannedContentFile {
                has_linked_parent,
                relative_path,
                file_name: file_name.to_string(),
                enabled: !file_name.ends_with(".disabled"),
                size,
                hash_cache_key,
                is_symlink: path
                    .symlink_metadata()
                    .map_err(IOError::from)?
                    .file_type()
                    .is_symlink(),
            });
        }
    }

    Ok(files)
}

pub(crate) fn project_type_from_relative_path(
    relative_path: &str,
) -> Option<ProjectType> {
    ProjectType::get_from_parent_folder(PathBuf::from(relative_path))
}

fn is_scannable_project_file(
    project_type: ProjectType,
    file_name: &str,
) -> bool {
    let Some(extension) = Path::new(file_name.trim_end_matches(".disabled"))
        .extension()
        .and_then(|ext| ext.to_str())
    else {
        return false;
    };

    match project_type {
        ProjectType::Mod => extension.eq_ignore_ascii_case("jar"),
        ProjectType::DataPack
        | ProjectType::ResourcePack
        | ProjectType::ShaderPack => {
            extension.eq_ignore_ascii_case("zip")
                || extension.eq_ignore_ascii_case("jar")
        }
    }
}

#[cfg(test)]
mod incremental_tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn selected_paths_include_both_enabled_forms_without_scanning_other_folders()
     {
        let dir = tempfile::tempdir().unwrap();
        let instance = dir.path().join("example");
        std::fs::create_dir_all(instance.join("mods")).unwrap();
        std::fs::write(instance.join("mods/a.jar"), b"a").unwrap();
        std::fs::write(instance.join("mods/a.jar.disabled"), b"disabled")
            .unwrap();
        std::fs::write(instance.join("mods/b.jar"), b"unrelated").unwrap();
        std::fs::write(instance.join("resourcepacks"), b"not a directory")
            .unwrap();
        let paths = HashSet::from(["mods/a.jar".to_owned()]);
        let files =
            scan_content_paths(dir.path(), "example", Some(&paths)).unwrap();
        assert_eq!(files.len(), 2);
        assert!(
            files
                .iter()
                .all(|file| file.relative_path.starts_with("mods/a.jar"))
        );
        std::fs::remove_file(instance.join("mods/a.jar")).unwrap();
        assert_eq!(
            scan_content_paths(dir.path(), "example", Some(&paths))
                .unwrap()
                .len(),
            1
        );
    }
}
