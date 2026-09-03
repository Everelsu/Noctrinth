//! What a newly created instance takes from the launcher's shared settings.
//!
//! An instance installed from a modpack arrives with the `options.txt`,
//! `servers.dat` and creative hotbars its author put in it, and those are as
//! much a part of the pack as its mods are. The shared settings — the fork's
//! options profile and upstream's synced options — would write over exactly
//! those the first time it is launched, so a pack starts with all of them
//! switched off. An instance built here is the player's own from the first
//! moment and keeps the defaults it always had.
//!
//! Only ever what an instance starts with: every one of these is a switch in
//! the instance's own settings afterwards, and nothing here touches an instance
//! that already exists.

use crate::state::instances::InstanceLink;
use crate::state::{Settings, State};

/// Whether an instance's contents are somebody else's work rather than
/// something put together here: a modpack, a server project, or a shared
/// instance.
pub(crate) fn is_managed(link: &InstanceLink) -> bool {
    !matches!(link, InstanceLink::Unmanaged)
}

/// Keeps the shared settings out of a newly created managed instance.
///
/// Runs before the synced options are reconciled for the first time, so that
/// nothing has been seeded out of the instance or written into it yet.
pub(crate) async fn keep_shared_settings_out(
    instance_id: &str,
    state: &State,
) -> crate::Result<()> {
    // Upstream's per-instance switches. A row per feature is there already,
    // taken from the global defaults; this is that same list, turned off.
    sqlx::query(
        "
        UPDATE instance_sync_preferences
        SET enabled = 0
        WHERE instance_id = ?
        ",
    )
    .bind(instance_id)
    .execute(&state.pool)
    .await?;

    // The fork's options profile, which keeps the instances it skips in its own
    // settings row. Read here rather than handed in, so that a profile edited
    // while the install was running is not rolled back to what it was when the
    // install started.
    let mut settings = Settings::get(&state.pool).await?;
    let excluded = &mut settings.shared_game_options.excluded_instances;
    if excluded.iter().all(|instance| instance != instance_id) {
        excluded.push(instance_id.to_string());
        settings.update(&state.pool).await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::is_managed;
    use crate::state::instances::InstanceLink;

    #[test]
    fn an_instance_built_here_is_the_players_own() {
        assert!(!is_managed(&InstanceLink::Unmanaged));
    }

    #[test]
    fn anything_installed_from_a_pack_is_not() {
        assert!(is_managed(&InstanceLink::ModrinthModpack {
            project_id: "project".to_string(),
            version_id: "version".to_string(),
        }));
        assert!(is_managed(&InstanceLink::ImportedModpack {
            project_id: None,
            version_id: None,
            name: None,
            version_number: None,
            filename: None,
        }));
        assert!(is_managed(&InstanceLink::SharedInstance {
            modpack_project_id: None,
            modpack_version_id: None,
        }));
    }
}
