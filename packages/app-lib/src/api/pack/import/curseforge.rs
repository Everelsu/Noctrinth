use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{
    State,
    install::{InstallPhaseDetails, InstallProgressReporter},
    prelude::ModLoader,
    state::{AppliedContentSetPatch, EditInstance, InstanceInstallStage},
    util::{fetch::fetch, io},
};

use super::{
    copy_dotminecraft_with_reporter, curseforge_completion, recache_icon,
};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MinecraftInstance {
    pub name: Option<String>,
    pub base_mod_loader: Option<MinecraftInstanceModLoader>,
    pub profile_image_path: Option<PathBuf>,
    pub installed_modpack: Option<InstalledModpack>,
    pub game_version: String, // Minecraft game version. Non-prioritized, use this if Vanilla
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MinecraftInstanceModLoader {
    pub name: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InstalledModpack {
    pub thumbnail_url: Option<String>,
}

// Check if folder has a minecraftinstance.json that parses
pub async fn is_valid_curseforge(instance_folder: PathBuf) -> bool {
    let minecraft_instance = serde_json::from_str::<MinecraftInstance>(
        &io::read_any_encoding_to_string(
            &instance_folder.join("minecraftinstance.json"),
        )
        .await
        .unwrap_or(("".into(), encoding_rs::UTF_8))
        .0,
    );
    minecraft_instance.is_ok()
}

pub async fn import_curseforge(
    curseforge_instance_folder: PathBuf, // instance's folder
    instance_id: &str,
    reporter: InstallProgressReporter,
    details: InstallPhaseDetails,
) -> crate::Result<()> {
    // Load minecraftinstance.json
    let minecraft_instance = serde_json::from_str::<MinecraftInstance>(
        &io::read_any_encoding_to_string(
            &curseforge_instance_folder.join("minecraftinstance.json"),
        )
        .await
        .unwrap_or(("".into(), encoding_rs::UTF_8))
        .0,
    )?;
    let override_title = minecraft_instance.name;
    let backup_name = format!(
        "Curseforge-{}",
        curseforge_instance_folder
            .file_name()
            .map_or("Unknown".to_string(), |a| a.to_string_lossy().to_string())
    );

    let state = State::get().await?;
    // Recache Curseforge Icon if it exists
    let mut icon = None;

    // Noctrinth's own: `profileImagePath` is an absolute path into the
    // CurseForge installation, so it is routinely dead by the time an instance
    // is imported. The picture is looked for in the folder as well, and a dead
    // path now falls through to the thumbnail instead of ruling it out.
    if let Some(icon_path) = curseforge_completion::find_instance_image(
        &curseforge_instance_folder,
        minecraft_instance.profile_image_path.as_deref(),
    )
    .await
    {
        icon = recache_icon(icon_path).await?;
    }

    if icon.is_none()
        && let Some(InstalledModpack {
            thumbnail_url: Some(thumbnail_url),
        }) = minecraft_instance.installed_modpack.clone()
    {
        let icon_bytes = fetch(
            &thumbnail_url,
            None,
            None,
            None,
            &state.fetch_semaphore,
            &state.pool,
        )
        .await?;
        icon =
            Some(crate::api::instance::cache_icon(icon_bytes, &state).await?);
    }

    // base mod loader is always None for vanilla
    if let Some(instance_mod_loader) = minecraft_instance.base_mod_loader {
        let game_version = minecraft_instance.game_version;

        // CF allows Forge, Fabric, and Vanilla
        let mut mod_loader = None;
        let mut loader_version = None;

        match instance_mod_loader.name.split('-').collect::<Vec<&str>>()[..] {
            ["forge", version] => {
                mod_loader = Some(ModLoader::Forge);
                loader_version = Some(version.to_string());
            }
            ["fabric", version, _game_version] => {
                mod_loader = Some(ModLoader::Fabric);
                loader_version = Some(version.to_string());
            }
            _ => {}
        }

        let mod_loader = mod_loader.unwrap_or(ModLoader::Vanilla);

        let loader_version = if mod_loader != ModLoader::Vanilla {
            crate::launcher::get_loader_version_from_profile(
                &game_version,
                mod_loader,
                loader_version.as_deref(),
            )
            .await?
        } else {
            None
        };

        crate::api::instance::edit(
            instance_id,
            EditInstance {
                install_stage: Some(InstanceInstallStage::PackInstalling),
                name: Some(
                    override_title
                        .clone()
                        .unwrap_or_else(|| backup_name.to_string()),
                ),
                icon_path: Some(
                    icon.clone().map(|x| x.to_string_lossy().to_string()),
                ),
                content_set_patch: Some(AppliedContentSetPatch {
                    source_kind: None,
                    game_version: Some(game_version.clone()),
                    protocol_version: Some(None),
                    loader: Some(mod_loader),
                    loader_version: Some(loader_version.clone().map(|x| x.id)),
                }),
                ..EditInstance::default()
            },
        )
        .await?;
    } else {
        crate::api::instance::edit(
            instance_id,
            EditInstance {
                name: Some(
                    override_title
                        .clone()
                        .unwrap_or_else(|| backup_name.to_string()),
                ),
                icon_path: Some(
                    icon.clone().map(|x| x.to_string_lossy().to_string()),
                ),
                content_set_patch: Some(AppliedContentSetPatch {
                    source_kind: None,
                    game_version: Some(minecraft_instance.game_version.clone()),
                    protocol_version: Some(None),
                    loader: Some(ModLoader::Vanilla),
                    loader_version: Some(None),
                }),
                ..EditInstance::default()
            },
        )
        .await?;
    }

    // Copy in contained folders as overrides
    let state = State::get().await?;
    copy_dotminecraft_with_reporter(
        instance_id,
        curseforge_instance_folder.clone(),
        &state.io_semaphore,
        reporter.clone(),
        details.clone(),
    )
    .await?;

    // Noctrinth's own, and the reason this does not call `finish_import`: the
    // copy is not the whole pack. Whatever the instance says it has but did
    // not bring is fetched before Minecraft is installed, so the instance is
    // whole by the time the job finishes rather than a mod short.
    curseforge_completion::restore_missing_addons(
        instance_id,
        &curseforge_instance_folder,
        &reporter,
        &details,
    )
    .await?;

    crate::launcher::install_minecraft_for_instance_id_with_reporter(
        instance_id,
        false,
        Some(reporter),
    )
    .await?;

    Ok(())
}
