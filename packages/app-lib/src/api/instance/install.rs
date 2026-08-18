use crate::state::{JavaVersion, State};

pub async fn get_optimal_jre_key(
    instance_id: &str,
) -> crate::Result<Option<JavaVersion>> {
    let state = State::get().await?;
    let context =
        crate::state::instances::commands::get_instance_launch_context(
            instance_id,
            &state.pool,
        )
        .await?
        .ok_or_else(|| {
            crate::ErrorKind::OtherError(format!(
                "Tried to resolve a nonexistent instance {instance_id}!"
            ))
        })?;
    let (minecraft, version_index) =
        crate::launcher::resolve_minecraft_manifest(
            &context.applied_content_set.game_version,
            &state,
        )
        .await?;
    let version = &minecraft.versions[version_index];
    let loader_version = crate::launcher::get_loader_version_from_profile(
        &context.applied_content_set.game_version,
        context.applied_content_set.loader,
        context.applied_content_set.loader_version.as_deref(),
    )
    .await?;
    let mut version_info = crate::launcher::download::download_version_info(
        &state,
        version,
        loader_version.as_ref(),
        crate::launcher::patches::loader_component(
            context.applied_content_set.loader,
        ),
        None,
        None,
        None,
    )
    .await?;

    let instance_path = state
        .directories
        .instances_dir()
        .join(&context.instance.path);
    let applied_patches = crate::launcher::patches::patch_version_info(
        &instance_path,
        &mut version_info,
        &context.applied_content_set.game_version,
    )?;

    crate::launcher::get_java_version_from_launch_context(
        &context,
        crate::launcher::required_java_major(&version_info, &applied_patches),
    )
    .await
}
