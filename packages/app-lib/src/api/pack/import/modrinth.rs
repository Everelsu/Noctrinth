//! Import instances from an existing Modrinth App installation.
//!
//! Noctrinth is a fork of the Modrinth App, so the two share an on-disk
//! layout: instances live in `<data dir>/profiles/<folder>` and are plain
//! `.minecraft` directories, while their metadata (name, icon, game version,
//! loader) lives in the launcher's `app.db`. That makes the folder alone
//! useless for an import — unlike MultiMC or GDLauncher there is no per
//! instance config file — so this module reads the metadata straight out of
//! the other launcher's database.
//!
//! The database is opened read-only and queried with the runtime API rather
//! than the `sqlx::query!` macros: the schema belongs to whatever Modrinth App
//! version the user happens to have installed, so it must not be validated
//! against ours at compile time. Both the current `instances` schema and the
//! legacy `profiles` one are understood.

use std::path::{Path, PathBuf};

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Row, Sqlite, sqlite::SqliteRow};

use super::{finish_import, recache_icon};
use crate::{
    State,
    install::{InstallPhaseDetails, InstallProgressReporter},
    prelude::ModLoader,
    state::{AppliedContentSetPatch, EditInstance, InstanceInstallStage},
};

pub const INSTANCES_SUBFOLDER: &str = "profiles";

const APP_DB_FILE_NAME: &str = "app.db";

/// One importable instance as described by the source launcher's database.
struct ModrinthInstance {
    folder: String,
    name: String,
    icon_path: Option<String>,
    game_version: String,
    loader: ModLoader,
    loader_version: Option<String>,
}

impl ModrinthInstance {
    fn from_row(row: &SqliteRow) -> Option<Self> {
        let folder: String = row.try_get("path").ok()?;
        Some(Self {
            name: row
                .try_get::<String, _>("name")
                .unwrap_or_else(|_| folder.clone()),
            icon_path: row.try_get("icon_path").ok(),
            game_version: row.try_get("game_version").ok()?,
            loader: row
                .try_get::<String, _>("loader")
                .map(|loader| ModLoader::from_string(&loader))
                .unwrap_or(ModLoader::Vanilla),
            loader_version: row.try_get("loader_version").ok(),
            folder,
        })
    }
}

/// A Modrinth App data directory is identified by its database — the
/// `profiles` folder alone is indistinguishable from any other launcher's.
pub async fn is_valid_modrinth_base(base_path: &Path) -> bool {
    tokio::fs::try_exists(base_path.join(APP_DB_FILE_NAME))
        .await
        .unwrap_or(false)
        && tokio::fs::try_exists(base_path.join(INSTANCES_SUBFOLDER))
            .await
            .unwrap_or(false)
}

/// An instance folder is valid when its launcher's database sits two levels up
/// — checked without opening the database so the per-folder scan stays cheap.
pub async fn is_valid_modrinth(instance_path: PathBuf) -> bool {
    if !instance_path.is_dir() {
        return false;
    }
    let Some(base_path) = instance_path.parent().and_then(Path::parent) else {
        return false;
    };
    is_valid_modrinth_base(base_path).await
}

/// Instances the source launcher knows about *and* that still exist on disk.
///
/// A database row without its folder is a leftover the other launcher hasn't
/// cleaned up yet; importing it would produce an empty instance.
pub async fn get_modrinth_instances(
    base_path: &Path,
) -> crate::Result<Vec<String>> {
    let instances = query_instances(base_path).await?;
    let instances_folder = base_path.join(INSTANCES_SUBFOLDER);

    let mut folders = Vec::new();
    for instance in instances {
        if tokio::fs::try_exists(instances_folder.join(&instance.folder))
            .await
            .unwrap_or(false)
        {
            folders.push(instance.folder);
        }
    }
    Ok(folders)
}

pub async fn import_modrinth(
    base_path: PathBuf,
    instance_folder: String,
    instance_id: &str,
    reporter: InstallProgressReporter,
    details: InstallPhaseDetails,
) -> crate::Result<()> {
    let source = query_instances(&base_path)
        .await?
        .into_iter()
        .find(|instance| instance.folder == instance_folder)
        .ok_or_else(|| {
            crate::ErrorKind::InputError(format!(
                "Modrinth App has no instance named '{instance_folder}'"
            ))
        })?;

    let icon = match source.icon_path.as_deref() {
        Some(icon_path) => recache_icon(PathBuf::from(icon_path)).await?,
        None => None,
    };

    let loader_version = if source.loader != ModLoader::Vanilla {
        crate::launcher::get_loader_version_from_profile(
            &source.game_version,
            source.loader,
            source.loader_version.as_deref(),
        )
        .await?
    } else {
        None
    };

    crate::api::instance::edit(
        instance_id,
        EditInstance {
            install_stage: Some(InstanceInstallStage::PackInstalling),
            name: Some(source.name),
            icon_path: Some(icon.map(|x| x.to_string_lossy().to_string())),
            content_set_patch: Some(AppliedContentSetPatch {
                source_kind: None,
                game_version: Some(source.game_version),
                protocol_version: Some(None),
                loader: Some(source.loader),
                loader_version: Some(loader_version.map(|x| x.id)),
            }),
            ..EditInstance::default()
        },
    )
    .await?;

    let state = State::get().await?;
    finish_import(
        instance_id,
        base_path.join(INSTANCES_SUBFOLDER).join(instance_folder),
        &state.io_semaphore,
        reporter,
        details,
    )
    .await?;

    Ok(())
}

/// Deletes an instance from the source launcher after a successful import.
///
/// Only the instance folder is removed — the row is left in the other
/// launcher's database on purpose, since writing to a database owned by a
/// different (possibly running) process risks corrupting it. Modrinth App
/// tolerates a missing folder and drops the instance on its next scan.
pub async fn remove_source_instance(
    base_path: &Path,
    instance_folder: &str,
) -> crate::Result<()> {
    // The folder name reaches us from the frontend, so it must be a single
    // plain component — anything else could walk out of the profiles folder
    // and delete an unrelated directory.
    if !path_util::is_safe_file_name(instance_folder) {
        return Err(crate::ErrorKind::InputError(format!(
            "Refusing to delete '{instance_folder}': not a valid instance folder name"
        ))
        .into());
    }

    let folder = base_path.join(INSTANCES_SUBFOLDER).join(instance_folder);
    if !folder.is_dir() {
        return Err(crate::ErrorKind::InputError(format!(
            "Refusing to delete '{}': not a Modrinth App instance folder",
            folder.display()
        ))
        .into());
    }
    crate::util::io::remove_dir_all(&folder).await?;
    Ok(())
}

async fn query_instances(
    base_path: &Path,
) -> crate::Result<Vec<ModrinthInstance>> {
    let db_path = base_path.join(APP_DB_FILE_NAME);
    if !tokio::fs::try_exists(&db_path).await.unwrap_or(false) {
        return Err(crate::ErrorKind::InputError(format!(
            "No Modrinth App database found at {}",
            db_path.display()
        ))
        .into());
    }

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(&db_path)
                .read_only(true)
                .create_if_missing(false),
        )
        .await?;

    // The instance/content-set split landed partway through 0.17; anything
    // older still keeps everything on a single `profiles` table.
    let rows = if table_exists(&pool, "instances").await? {
        sqlx::query(
            "
			SELECT i.path, i.name, i.icon_path,
				cs.game_version, cs.loader, cs.loader_version
			FROM instances i
			INNER JOIN instance_content_sets cs
				ON cs.id = i.applied_content_set_id
			",
        )
        .fetch_all(&pool)
        .await?
    } else {
        sqlx::query(
            "
			SELECT path, name, icon_path,
				game_version, mod_loader AS loader,
				mod_loader_version AS loader_version
			FROM profiles
			",
        )
        .fetch_all(&pool)
        .await?
    };

    pool.close().await;

    Ok(rows.iter().filter_map(ModrinthInstance::from_row).collect())
}

async fn table_exists(
    pool: &sqlx::Pool<Sqlite>,
    name: &str,
) -> crate::Result<bool> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?",
    )
    .bind(name)
    .fetch_one(pool)
    .await?;
    Ok(count > 0)
}
