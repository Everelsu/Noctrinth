use crate::State;
use crate::state::content_store::{
    FileContent, InstanceFileStatus, InstanceFileStorage, content_file_path,
    file_path_on_disk,
};
use crate::state::instances::adapters::{filesystem, sqlite};
use crate::state::instances::{Instance, InstanceFile};
use crate::state::{
    CachedEntry, InstanceInstallStage, ProjectType, file_hash_cache_key,
};
use chrono::Utc;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use super::content_mutation::{
    ContentChange, ContentChangeResult, InstanceContent,
};

pub(crate) async fn sync_content_files(
    instance_id: &str,
    state: &State,
) -> crate::Result<Vec<InstanceFile>> {
    let (files, _) = sync_content_files_with_changed(instance_id, state).await?;
    Ok(files)
}

pub(crate) async fn sync_content_files_with_changed(
    instance_id: &str,
    state: &State,
) -> crate::Result<(Vec<InstanceFile>, bool)> {
    let instance =
        sqlite::instance_rows::get_instance_by_id(instance_id, &state.pool)
            .await?
            .ok_or(crate::ErrorKind::InputError(
                "unknown instance".to_string(),
            ))?;

    sync_instance_content_scope(&instance, None, state).await
}

pub(crate) async fn sync_changed_content_files(
    instance: &Instance,
    paths: &HashSet<std::path::PathBuf>,
    state: &State,
) -> crate::Result<Vec<InstanceFile>> {
    let existing =
        sqlite::content_rows::get_instance_files(&instance.id, &state.pool)
            .await?;
    let bound_ids = crate::state::content_store::instance_storage(
        &state.pool,
        &instance.id,
    )
    .await?
    .into_iter()
    .map(|binding| binding.file_id)
    .collect::<HashSet<_>>();
    let scope = changed_content_scope(
        &state.directories.instances_dir(),
        &instance.path,
        paths,
        &existing,
        &bound_ids,
    )?;
    let (files, _) = sync_instance_content_scope(instance, scope.as_ref(), state).await?;
    Ok(files)
}

fn changed_content_scope(
    instances_dir: &std::path::Path,
    instance_path: &str,
    paths: &HashSet<std::path::PathBuf>,
    existing: &[InstanceFile],
    bound_ids: &HashSet<String>,
) -> crate::Result<Option<HashSet<String>>> {
    let root = instances_dir.join(instance_path);
    let mut scope = HashSet::new();
    for path in paths {
        let Ok(relative) = path.strip_prefix(&root) else {
            return Ok(None);
        };
        let parts = relative.components().collect::<Vec<_>>();
        if parts.len() != 2
            || parts
                .iter()
                .any(|part| !matches!(part, std::path::Component::Normal(_)))
            || path.is_dir()
        {
            return Ok(None);
        }
        let relative = relative.to_string_lossy().replace('\\', "/");
        if !crate::state::content_store::is_managed_content_path(&relative) {
            return Ok(None);
        }
        scope.insert(relative.trim_end_matches(".disabled").to_owned());
    }
    let scanned = filesystem::scan_content_paths(
        instances_dir,
        instance_path,
        Some(&scope),
    )?;
    for file in existing {
        let canonical = file.relative_path.trim_end_matches(".disabled");
        let touched = scope
            .iter()
            .any(|path| path.eq_ignore_ascii_case(canonical));
        if touched && !scope.contains(canonical) {
            return Ok(None);
        }
        let possible_rename = scanned.iter().any(|candidate| {
            candidate.size == file.size
                && candidate.enabled == file.enabled
                && filesystem::project_type_from_relative_path(
                    &candidate.relative_path,
                ) == project_type_for_file(file)
        });
        if bound_ids.contains(&file.id)
            && (touched || possible_rename)
            && !root.join(content_file_path(file)).try_exists()?
        {
            // Renames need the complete set to preserve bindings and resolve ambiguous matches.
            return Ok(None);
        }
    }
    Ok(Some(scope))
}

pub(crate) async fn sync_instance_content_files(
    instance: &Instance,
    state: &State,
) -> crate::Result<Vec<InstanceFile>> {
    let (files, _) = sync_instance_content_scope(instance, None, state).await?;
    Ok(files)
}

async fn sync_instance_content_scope(
    instance: &Instance,
    scope: Option<&HashSet<String>>,
    state: &State,
) -> crate::Result<(Vec<InstanceFile>, bool)> {
    let in_scope = |file: &InstanceFile| {
        scope.is_none_or(|paths| {
            paths.contains(file.relative_path.trim_end_matches(".disabled"))
        })
    };
    let (snapshot, known_files, known_bindings) = {
        let instance_content =
            InstanceContent::lock(&instance.id, state).await?;
        let instance = instance_content.instance();
        if matches!(
            instance.install_stage,
            InstanceInstallStage::MinecraftInstalling
                | InstanceInstallStage::PackInstalling
        ) {
            let files = sqlite::content_rows::get_instance_files(
                &instance.id,
                &state.pool,
            )
            .await?;
            return Ok((files, false));
        }
        let snapshot = filesystem::scan_content_paths(
            &state.directories.instances_dir(),
            &instance.path,
            scope,
        )?
        .into_iter()
        .map(|file| (file.relative_path.clone(), file))
        .collect::<HashMap<_, _>>();
        let known_files =
            sqlite::content_rows::get_instance_files(&instance.id, &state.pool)
                .await?;
        let known_bindings = crate::state::content_store::instance_storage(
            &state.pool,
            &instance.id,
        )
        .await?
        .into_iter()
        .map(|binding| (binding.file_id.clone(), binding))
        .collect::<HashMap<_, _>>();
        (snapshot, known_files, known_bindings)
    };

    // Warm hashes without the store lock; their stamps are checked again below.
    let mut changed_outside = HashSet::new();
    for file in known_files.iter().filter(|file| in_scope(file)) {
        if let Some(binding) = known_bindings.get(&file.id)
            && changed_outside_app(instance, file, binding, state)
                .await
                .unwrap_or(false)
        {
            changed_outside.insert(file.id.clone());
        }
    }

    let managed_paths = known_files
        .iter()
        .filter(|file| {
            known_bindings.contains_key(&file.id)
                && !changed_outside.contains(&file.id)
        })
        .map(|file| file.relative_path.trim_end_matches(".disabled"))
        .collect::<HashSet<_>>();
    let cache_keys = snapshot
        .values()
        .filter(|file| {
            !managed_paths
                .contains(file.relative_path.trim_end_matches(".disabled"))
        })
        .map(|file| file.hash_cache_key.as_str())
        .collect::<Vec<_>>();
    let hashes = CachedEntry::get_file_hash_many(
        &cache_keys,
        None,
        &state.pool,
        &state.api_semaphore,
    )
    .await?;
    let hashes_by_key = hashes
        .into_iter()
        .map(|hash| {
            (
                file_hash_cache_key(
                    hash.size,
                    hash.modified_at_ns,
                    hash.path.trim_end_matches(".disabled"),
                ),
                hash,
            )
        })
        .collect::<HashMap<_, _>>();

    let instance_content = InstanceContent::lock(&instance.id, state).await?;
    let instance = instance_content.instance();
    if matches!(
        instance.install_stage,
        InstanceInstallStage::MinecraftInstalling
            | InstanceInstallStage::PackInstalling
    ) {
        let files = sqlite::content_rows::get_instance_files(
            &instance.id,
            &state.pool,
        )
        .await?;
        return Ok((files, false));
    }
    let running =
        crate::state::instance_has_running_process(&instance.id, state).await?;
    let renamed = scope.is_none()
        && !running
        && reconcile_instance_renames(instance, state).await?;
    let mut existing =
        sqlite::content_rows::get_instance_files(&instance.id, &state.pool)
            .await?;
    let mut bindings = crate::state::content_store::instance_storage(
        &state.pool,
        &instance.id,
    )
    .await?
    .into_iter()
    .map(|binding| (binding.file_id.clone(), binding))
    .collect::<HashMap<_, _>>();
    let scanned = filesystem::scan_content_paths(
        &state.directories.instances_dir(),
        &instance.path,
        scope,
    )?;
    let normalized = normalize_legacy_content_files(
        instance, &existing, &bindings, &scanned, state,
    )
    .await?;
    if normalized {
        existing =
            sqlite::content_rows::get_instance_files(&instance.id, &state.pool)
                .await?;
    }
    let mut unbound = HashSet::new();
    for file in &existing {
        if changed_outside.contains(&file.id)
            && let Some(binding) = bindings.get(&file.id)
            && changed_outside_app(instance, file, binding, state).await?
        {
            unbound.insert(file.id.clone());
        }
    }
    bindings.retain(|file_id, _| !unbound.contains(file_id));
    let existing_by_path = existing
        .iter()
        .map(|file| (file.relative_path.as_str(), file))
        .collect::<HashMap<_, _>>();
    let existing_by_id = existing
        .iter()
        .map(|file| (file.id.as_str(), file))
        .collect::<HashMap<_, _>>();
    let managed_by_path = existing
        .iter()
        .filter(|file| bindings.contains_key(&file.id))
        .map(|file| (file.relative_path.trim_end_matches(".disabled"), file))
        .collect::<HashMap<_, _>>();
    let mut canonical_paths = HashSet::new();
    let mut duplicate_paths = HashSet::new();
    for file in &scanned {
        let canonical = file.relative_path.trim_end_matches(".disabled");
        if !canonical_paths.insert(canonical) {
            duplicate_paths.insert(canonical);
        }
    }
    let mut files = existing
        .iter()
        .filter(|file| !in_scope(file))
        .cloned()
        .collect::<Vec<_>>();
    let mut saved_files = HashSet::new();
    for previous in existing.iter().filter(|file| in_scope(file)) {
        let Some(binding) = bindings.get(&previous.id) else {
            continue;
        };
        let mut file = previous.clone();
        let content = state.content_store.file_content(previous).await?;
        let stored = matches!(content, FileContent::Stored { .. });
        let file_status = state
            .content_store
            .check_instance_file_cached(instance, previous, binding)
            .await?;
        file.missing = !stored || file_status != InstanceFileStatus::Healthy;
        if !file.enabled
            && file.missing
            && stored
            && file_status == InstanceFileStatus::Missing
            && !running
        {
            file = match instance_content
                .apply_change(ContentChange::Toggle {
                    project_path: &file.relative_path,
                    desired_enabled: Some(false),
                })
                .await?
            {
                ContentChangeResult::File(file) => file,
                _ => unreachable!("repair mutations return a content file"),
            };
            saved_files.insert(file.id.clone());
        }
        if file.missing != previous.missing {
            file.modified_at = Utc::now();
        }
        files.push(file);
    }
    for scanned in &scanned {
        let canonical_path =
            scanned.relative_path.trim_end_matches(".disabled");
        let previous = existing_by_path
            .get(scanned.relative_path.as_str())
            .or_else(|| {
                (!duplicate_paths.contains(canonical_path))
                    .then(|| existing_by_path.get(canonical_path))
                    .flatten()
            })
            .copied();
        if let Some(file) = managed_by_path.get(canonical_path) {
            if content_file_path(file) != scanned.relative_path {
                tracing::warn!(
                    instance_id = %instance.id,
                    path = %scanned.relative_path,
                    "Ignoring content at the inactive form of a managed path"
                );
            }
            continue;
        }
        let Some(hash) = hashes_by_key
            .get(scanned.hash_cache_key.trim_end_matches(".disabled"))
            .filter(|_| {
                snapshot
                    .get(&scanned.relative_path)
                    .is_some_and(|previous| {
                        previous.hash_cache_key == scanned.hash_cache_key
                            && previous.is_symlink == scanned.is_symlink
                            && previous.has_linked_parent
                                == scanned.has_linked_parent
                    })
            })
        else {
            if let Some(previous) = previous {
                let mut file = previous.clone();
                file.missing = false;
                files.push(file);
            }
            continue;
        };
        let mut file = InstanceFile {
            id: previous
                .map(|file| file.id.clone())
                .unwrap_or_else(instance_file_id),
            instance_id: instance.id.clone(),
            relative_path: scanned.relative_path.clone(),
            file_name: scanned.file_name.clone(),
            enabled: scanned.enabled,
            sha1: hash.hash.clone(),
            size: scanned.size,
            missing: false,
            added_at: previous
                .map(|file| file.added_at)
                .unwrap_or_else(Utc::now),
            modified_at: Utc::now(),
        };
        if let Some(previous) = previous {
            file.relative_path.clone_from(&previous.relative_path);
            file.file_name.clone_from(&previous.file_name);
            if is_content_unchanged(&file, previous) {
                file.modified_at = previous.modified_at;
            }
        } else if !duplicate_paths.contains(canonical_path) {
            file.relative_path = canonical_path.to_string();
            file.file_name =
                scanned.file_name.trim_end_matches(".disabled").to_string();
        }
        files.push(file);
    }
    let present_ids = files
        .iter()
        .map(|file| file.id.as_str())
        .collect::<HashSet<_>>();
    let missing = existing
        .iter()
        .filter(|file| !present_ids.contains(file.id.as_str()) && !file.missing)
        .collect::<Vec<_>>();
    let changed = renamed
        || normalized
        || !unbound.is_empty()
        || !missing.is_empty()
        || files.iter().any(|file| {
            existing_by_id.get(file.id.as_str()).is_none_or(|previous| {
                previous.missing != file.missing
                    || previous.enabled != file.enabled
                    || previous.sha1 != file.sha1
                    || previous.relative_path != file.relative_path
            })
        });
    let needs_upsert = files.iter().any(|file| {
        !(saved_files.contains(&file.id)
            || existing_by_id
                .get(file.id.as_str())
                .is_some_and(|previous| {
                    is_content_unchanged(file, previous)
                        && file.modified_at.timestamp()
                            == previous.modified_at.timestamp()
                }))
    });
    let stored = if unbound.is_empty() && missing.is_empty() && !needs_upsert {
        files
    } else {
        let mut tx = state.pool.begin_with("BEGIN IMMEDIATE").await?;
        for file_id in &unbound {
            crate::state::content_store::remove_file_storage(&mut tx, file_id)
                .await?;
        }
        for file in missing {
            sqlite::content_rows::set_instance_file_missing(
                &file.id, true, &mut tx,
            )
            .await?;
        }
        let mut stored = Vec::with_capacity(files.len());
        for file in files {
            if saved_files.contains(&file.id)
                || existing_by_id
                    .get(file.id.as_str())
                    .is_some_and(|previous| {
                        is_content_unchanged(&file, previous)
                            && file.modified_at.timestamp()
                                == previous.modified_at.timestamp()
                    })
            {
                stored.push(file);
            } else {
                stored.push(
                    sqlite::content_rows::upsert_instance_file(&file, &mut tx)
                        .await?,
                );
            }
        }
        tx.commit().await?;
        stored
    };
    if changed {
        super::mark_shared_instance_stale(&instance.id, &state.pool).await?;
        crate::api::instance::queue_game_locale_index();
    }
    Ok((stored, changed))
}

async fn changed_outside_app(
    instance: &Instance,
    file: &InstanceFile,
    binding: &InstanceFileStorage,
    state: &State,
) -> crate::Result<bool> {
    Ok(
        match state
            .content_store
            .check_instance_file_cached(instance, file, binding)
            .await?
        {
            InstanceFileStatus::Conflict => true,
            InstanceFileStatus::Missing => false,
            InstanceFileStatus::Healthy => !matches!(
                state
                    .content_store
                    .file_content_with_binding(file, binding.clone())
                    .await?,
                FileContent::Stored { .. }
            ),
        },
    )
}

/// Call only while holding both the instance content lock and the content store lock.
/// Minecraft must not be running for this instance.
pub(crate) async fn reconcile_instance_renames(
    instance: &Instance,
    state: &State,
) -> crate::Result<bool> {
    let files =
        sqlite::content_rows::get_instance_files(&instance.id, &state.pool)
            .await?;
    let bindings = crate::state::content_store::instance_storage(
        &state.pool,
        &instance.id,
    )
    .await?
    .into_iter()
    .map(|binding| (binding.file_id.clone(), binding))
    .collect::<HashMap<_, _>>();
    let mut missing = Vec::new();
    for file in &files {
        if let Some(binding) = bindings.get(&file.id) {
            let path = state
                .content_store
                .instance_path(&instance.path, &content_file_path(file))
                .await?;
            match tokio::fs::symlink_metadata(path).await {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    missing.push((file, binding));
                }
                Err(error) => return Err(error.into()),
                Ok(_) => {}
            }
        }
    }
    if missing.is_empty() {
        return Ok(false);
    }
    let scanned = filesystem::scan_content_files(
        &state.directories.instances_dir(),
        &instance.path,
    )?;
    let mut renames = Vec::new();
    for candidate in &scanned {
        if candidate.is_symlink
            || candidate.has_linked_parent
            || !crate::state::content_store::is_managed_content_path(
                &candidate.relative_path,
            )
        {
            continue;
        }
        let canonical = candidate.relative_path.trim_end_matches(".disabled");
        let target = files.iter().find(|file| {
            file.relative_path == canonical
                || file.relative_path == candidate.relative_path
        });
        if target.is_some_and(|file| file.relative_path != canonical) {
            continue;
        }
        let possible = missing
            .iter()
            .filter(|(file, _)| {
                file.size == candidate.size
                    && file.enabled == candidate.enabled
                    && project_type_for_file(file)
                        == filesystem::project_type_from_relative_path(
                            &candidate.relative_path,
                        )
            })
            .collect::<Vec<_>>();
        if possible.is_empty() {
            continue;
        }
        let path = state
            .content_store
            .instance_path(&instance.path, &candidate.relative_path)
            .await?;
        let hash = crate::state::content_store::hash_file(&path).await?.sha512;
        let matches = possible
            .into_iter()
            .filter(|(_, binding)| binding.blob_sha512 == hash)
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            continue;
        }
        let (source, binding) = *matches[0];
        if target.is_some_and(|target| bindings.contains_key(&target.id)) {
            continue;
        }
        renames.push((source, binding, candidate));
    }
    let mut changed = false;
    for (source, binding, candidate) in &renames {
        if renames
            .iter()
            .filter(|(file, _, _)| file.id == source.id)
            .count()
            != 1
        {
            continue;
        }
        let canonical = candidate.relative_path.trim_end_matches(".disabled");
        let mut renamed = (*source).clone();
        renamed.relative_path = canonical.to_string();
        if state
            .content_store
            .check_instance_file(instance, source, binding)
            .await?
            != InstanceFileStatus::Missing
            || state
                .content_store
                .check_instance_file(instance, &renamed, binding)
                .await?
                != InstanceFileStatus::Healthy
        {
            continue;
        }
        let mut tx = state.pool.begin_with("BEGIN IMMEDIATE").await?;
        sqlite::content_rows::rename_instance_file(
            &instance.id,
            &source.relative_path,
            canonical,
            candidate.file_name.trim_end_matches(".disabled"),
            source.enabled,
            &mut tx,
        )
        .await?;
        tx.commit().await?;
        changed = true;
    }
    if changed {
        super::mark_shared_instance_stale(&instance.id, &state.pool).await?;
    }
    Ok(changed)
}

/// Whether a rescanned file matches what is already stored, ignoring
/// `modified_at` — which records when the file last actually changed, not when
/// it was last looked at.
fn is_content_unchanged(
    candidate: &InstanceFile,
    previous: &InstanceFile,
) -> bool {
    candidate.relative_path == previous.relative_path
        && candidate.file_name == previous.file_name
        && candidate.enabled == previous.enabled
        && candidate.sha1 == previous.sha1
        && candidate.size == previous.size
        && candidate.missing == previous.missing
}

pub(super) async fn normalize_legacy_content_files(
    instance: &Instance,
    existing: &[InstanceFile],
    bindings: &HashMap<
        String,
        crate::state::content_store::InstanceFileStorage,
    >,
    scanned: &[filesystem::ScannedContentFile],
    state: &State,
) -> crate::Result<bool> {
    let mut renames = Vec::new();
    for scanned in scanned {
        let canonical = scanned.relative_path.trim_end_matches(".disabled");
        let disabled = format!("{canonical}.disabled");
        let source =
            existing.iter().find(|file| file.relative_path == disabled);
        let target =
            existing.iter().find(|file| file.relative_path == canonical);
        let Some(source) = source.or(target) else {
            continue;
        };
        if bindings.contains_key(&source.id)
            || target.is_some_and(|file| bindings.contains_key(&file.id))
            || source.relative_path == canonical
                && source.enabled == scanned.enabled
        {
            continue;
        }
        let opposite = file_path_on_disk(canonical, !scanned.enabled);
        let opposite = state
            .directories
            .instances_dir()
            .join(&instance.path)
            .join(opposite);
        match tokio::fs::symlink_metadata(opposite).await {
            Ok(_) => continue,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        renames.push((
            source,
            canonical,
            scanned.file_name.trim_end_matches(".disabled"),
            scanned.enabled,
        ));
    }
    if renames.is_empty() {
        return Ok(false);
    }
    let mut tx = state.pool.begin_with("BEGIN IMMEDIATE").await?;
    for (source, canonical, file_name, enabled) in renames {
        if let Some(file) = sqlite::content_rows::rename_instance_file(
            &instance.id,
            &source.relative_path,
            canonical,
            file_name,
            enabled,
            &mut tx,
        )
        .await?
            && let Some(content_set_id) = &instance.applied_content_set_id
        {
            sqlite::content_rows::set_content_entry_enabled_for_file(
                content_set_id,
                &file.id,
                enabled,
                &mut tx,
            )
            .await?;
        }
    }
    tx.commit().await?;
    Ok(true)
}

pub(crate) fn project_type_for_file(
    file: &InstanceFile,
) -> Option<ProjectType> {
    filesystem::project_type_from_relative_path(&file.relative_path)
}

pub(super) fn instance_file_id() -> String {
    format!("instance-file:{}", Uuid::new_v4())
}

#[cfg(test)]
mod incremental_tests {
    use super::*;

    #[test]
    fn additions_and_unmanaged_deletions_are_scoped_but_managed_renames_recover()
     {
        let dir = tempfile::tempdir().unwrap();
        let mods = dir.path().join("instance/mods");
        std::fs::create_dir_all(&mods).unwrap();
        std::fs::write(mods.join("new.jar"), b"new").unwrap();
        let file = InstanceFile {
            id: "old".into(),
            instance_id: "test".into(),
            relative_path: "mods/old.jar".into(),
            file_name: "old.jar".into(),
            enabled: true,
            sha1: "hash".into(),
            size: 3,
            missing: false,
            added_at: Utc::now(),
            modified_at: Utc::now(),
        };
        let paths = HashSet::from([mods.join("new.jar"), mods.join("old.jar")]);
        let scope = changed_content_scope(
            dir.path(),
            "instance",
            &paths,
            std::slice::from_ref(&file),
            &HashSet::new(),
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            scope,
            HashSet::from([
                "mods/new.jar".to_owned(),
                "mods/old.jar".to_owned()
            ])
        );
        let bound = HashSet::from([file.id.clone()]);
        assert!(
            changed_content_scope(
                dir.path(),
                "instance",
                &paths,
                std::slice::from_ref(&file),
                &bound
            )
            .unwrap()
            .is_none()
        );
        let only_new = HashSet::from([mods.join("new.jar")]);
        assert!(
            changed_content_scope(
                dir.path(),
                "instance",
                &only_new,
                &[file],
                &bound
            )
            .unwrap()
            .is_none(),
            "rename recovery must work even if the old path event is missing"
        );
        assert!(
            changed_content_scope(
                dir.path(),
                "instance",
                &HashSet::from([mods]),
                &[],
                &bound
            )
            .unwrap()
            .is_none()
        );
    }
}
