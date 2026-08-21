use crate::state::DirectoryInfo;
use sqlx::migrate::Migrator;
use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions,
};
use sqlx::{Pool, Sqlite};
use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

static MIGRATOR: Migrator = sqlx::migrate!();

pub(crate) async fn connect(
    app_identifier: &str,
) -> crate::Result<Pool<Sqlite>> {
    let settings_dir = DirectoryInfo::initial_settings_dir_path(app_identifier)
        .ok_or(crate::ErrorKind::FSError(
            "Could not find valid config dir".to_string(),
        ))?;

    crate::util::io::create_dir_all(&settings_dir).await?;

    let db_path = settings_dir.join("app.db");

    connect_app_db(&db_path).await
}

async fn connect_app_db(db_path: &Path) -> crate::Result<Pool<Sqlite>> {
    super::db_backup::maybe_backup_existing_app_db(db_path).await?;
    open_migrated_app_db(db_path).await
}

async fn open_migrated_app_db(db_path: &Path) -> crate::Result<Pool<Sqlite>> {
    let pool = open_app_db_pool(db_path).await?;

    if let Err(err) = stale_data_cleanup(&pool).await {
        tracing::warn!(
            "Failed to clean up stale data from state database before migrations: {err}"
        );
    }

    run_migrations(&pool).await?;
    record_current_app_version(&pool).await?;

    if let Err(err) = stale_data_cleanup(&pool).await {
        tracing::warn!(
            "Failed to clean up stale data from state database: {err}"
        );
    }

    Ok(pool)
}

/// Applies the embedded migrations, refusing to start on a set that cannot be
/// applied safely.
///
/// sqlx keys applied migrations by version alone, so two files sharing a
/// version make it compare a stored checksum against the other file's contents
/// and refuse with "previously applied but has been modified" — leaving a
/// launcher that will not open and an error naming a number rather than the two
/// files behind it. Noctrinth adds migrations alongside upstream's, which is
/// exactly how a version gets used twice; this checks before anything runs and
/// says which files collided.
async fn run_migrations(pool: &Pool<Sqlite>) -> crate::Result<()> {
    if let Some(problem) = duplicate_migration_versions(&MIGRATOR) {
        return Err(crate::ErrorKind::OtherError(format!(
            "Refusing to migrate the state database: {problem}. Renumber the \
             migration that has not shipped yet — the one already applied on \
             installs cannot move."
        ))
        .into());
    }

    MIGRATOR.run(pool).await.inspect_err(|err| {
        tracing::error!(
            "Failed to apply migrations to the state database: {err}. A copy of \
             the database from before this launch is kept next to it."
        );
    })?;

    Ok(())
}

/// The first version used by more than one migration, described by name.
fn duplicate_migration_versions(migrator: &Migrator) -> Option<String> {
    let mut by_version: BTreeMap<i64, Vec<&str>> = BTreeMap::new();
    for migration in migrator.iter() {
        by_version
            .entry(migration.version)
            .or_default()
            .push(migration.description.as_ref());
    }

    by_version.into_iter().find_map(|(version, names)| {
        (names.len() > 1).then(|| {
            format!("version {version} is used by {}", names.join(" and "))
        })
    })
}

async fn open_app_db_pool(db_path: &Path) -> crate::Result<Pool<Sqlite>> {
    let conn_options = SqliteConnectOptions::new()
        .filename(db_path)
        .busy_timeout(Duration::from_secs(30))
        .journal_mode(SqliteJournalMode::Wal)
        .optimize_on_close(true, None)
        .create_if_missing(true);

    Ok(SqlitePoolOptions::new()
        .max_connections(100)
        .connect_with(conn_options)
        .await?)
}

async fn record_current_app_version(pool: &Pool<Sqlite>) -> crate::Result<()> {
    sqlx::query!(
        "
		INSERT INTO app_metadata (key, value, updated_at)
		VALUES ('app_version', ?, unixepoch())
		ON CONFLICT(key) DO UPDATE SET
			value = excluded.value,
			updated_at = excluded.updated_at
		",
        env!("CARGO_PKG_VERSION"),
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Cleans up data from the database that is no longer referenced, but must be
/// kept around for a little while to allow users to recover from accidental
/// deletions.
async fn stale_data_cleanup(pool: &Pool<Sqlite>) -> crate::Result<()> {
    let mut tx = pool.begin().await?;

    let has_skin_tables = sqlx::query!(
		"SELECT COUNT(*) AS \"count!: i64\" FROM sqlite_master WHERE type = 'table' AND name IN ('custom_minecraft_skins', 'minecraft_users')",
	)
	.fetch_one(&mut *tx)
	.await?
	.count == 2;

    if has_skin_tables {
        sqlx::query!(
			"DELETE FROM custom_minecraft_skins WHERE minecraft_user_uuid NOT IN (SELECT uuid FROM minecraft_users)"
		)
		.execute(&mut *tx)
		.await?;
    }

    tx.commit().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A duplicate version is invisible until a launcher that already ran the
    /// first one meets the second, which is a shipped release later. Catching it
    /// here costs nothing and is the whole reason the check above exists.
    #[test]
    fn every_migration_has_its_own_version() {
        assert_eq!(duplicate_migration_versions(&MIGRATOR), None);
    }

    /// Version numbers are timestamps, and sqlx applies them in order. A new
    /// migration numbered below one that is already applied would simply never
    /// run on an existing install.
    #[test]
    fn migrations_are_ordered_by_version() {
        let versions: Vec<i64> =
            MIGRATOR.iter().map(|migration| migration.version).collect();
        let mut sorted = versions.clone();
        sorted.sort_unstable();
        assert_eq!(versions, sorted);
    }
}
