use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::State;
use crate::state::ReleaseChannel;
use crate::state::instances::adapters::sqlite;
use crate::state::instances::{Instance, InstanceLink};

pub(crate) const MANIFEST_FILE_NAME: &str = "noctrinth-instance.json";
const MANIFEST_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct InstanceManifest {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub path: String,
    pub game_version: Option<String>,
    pub loader: Option<String>,
    pub loader_version: Option<String>,
    pub update_channel: ReleaseChannel,
    pub icon_path: Option<String>,
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
    pub last_played: Option<DateTime<Utc>>,
    pub time_played: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<InstanceLink>,
}

/// Writes an optional portable `noctrinth-instance.json` sidecar inside the
/// instance directory. Best-effort: never fails instance creation or edits.
pub(crate) async fn write_instance_manifest(
    instance: &Instance,
    state: &State,
) {
    let instance_dir = state.directories.instances_dir().join(&instance.path);
    if !instance_dir.is_dir() {
        return;
    }

    let (content_set_res, link_res) = tokio::join!(
        sqlite::content_rows::get_applied_content_set(
            &instance.id,
            &state.pool,
        ),
        sqlite::instance_rows::get_instance_link(&instance.id, &state.pool),
    );
    let content_set = content_set_res.ok().flatten();
    let link = link_res.ok().filter(|l| !matches!(l, InstanceLink::Unmanaged));

    let manifest = InstanceManifest {
        schema_version: MANIFEST_SCHEMA_VERSION,
        id: instance.id.clone(),
        name: instance.name.clone(),
        path: instance.path.clone(),
        game_version: content_set.as_ref().map(|cs| cs.game_version.clone()),
        loader: content_set.as_ref().map(|cs| cs.loader.as_str().to_string()),
        loader_version: content_set.and_then(|cs| cs.loader_version),
        update_channel: instance.update_channel,
        icon_path: instance.icon_path.clone(),
        created: instance.created,
        modified: instance.modified,
        last_played: instance.last_played,
        time_played: instance.submitted_time_played
            + instance.recent_time_played,
        link,
    };

    let Ok(json) = serde_json::to_string_pretty(&manifest) else {
        return;
    };

    let target = instance_dir.join(MANIFEST_FILE_NAME);
    if let Err(error) = tokio::fs::write(&target, json).await {
        tracing::debug!(
            instance_id = %instance.id,
            error = %error,
            "Could not write noctrinth-instance.json sidecar"
        );
    }
}
