use crate::state::ElyCredentials;
use uuid::Uuid;

pub async fn login(username: &str, password: &str) -> crate::Result<ElyCredentials> {
    let state = crate::State::get().await?;
    let creds = ElyCredentials::authenticate(username, password).await?;
    creds.upsert(&state.pool).await?;
    Ok(creds)
}

pub async fn logout(uuid: Uuid) -> crate::Result<()> {
    let state = crate::State::get().await?;
    ElyCredentials::remove(uuid, &state.pool).await
}

pub async fn get_default_user() -> crate::Result<Option<Uuid>> {
    let state = crate::State::get().await?;
    let user = ElyCredentials::get_active(&state.pool).await?;
    Ok(user.map(|u| u.uuid))
}

pub async fn set_default_user(uuid: Uuid) -> crate::Result<()> {
    let state = crate::State::get().await?;
    let all = ElyCredentials::get_all(&state.pool).await?;

    // Deactivate all Ely.by accounts.
    sqlx::query!("UPDATE ely_users SET active = FALSE")
        .execute(&state.pool)
        .await
        .map_err(crate::Error::from)?;

    // Deactivate all Microsoft accounts too: there is exactly one active
    // account across both providers, so picking an Ely.by account must
    // unselect any Microsoft account (and vice versa).
    sqlx::query!("UPDATE minecraft_users SET active = FALSE")
        .execute(&state.pool)
        .await
        .map_err(crate::Error::from)?;

    // Activate the selected one
    if let Some(mut user) = all.into_iter().find(|u| u.uuid == uuid) {
        user.active = true;
        user.upsert(&state.pool).await?;
    }

    Ok(())
}

pub async fn users() -> crate::Result<Vec<ElyCredentials>> {
    let state = crate::State::get().await?;
    ElyCredentials::get_all(&state.pool).await
}

/// Fetches the raw PNG skin texture for an Ely.by user from the public
/// skin system. Returns the PNG bytes, or an error if the user has no
/// custom skin (HTTP 404) or the request fails.
pub async fn get_skin_texture(username: &str) -> crate::Result<Vec<u8>> {
    use crate::util::fetch::INSECURE_REQWEST_CLIENT;

    let url = format!(
        "https://skinsystem.ely.by/skins/{}.png",
        urlencoding::encode(username)
    );

    let resp = INSECURE_REQWEST_CLIENT
        .get(&url)
        .send()
        .await
        .map_err(|e| {
            crate::ErrorKind::OtherError(format!(
                "Failed to fetch Ely.by skin: {e}"
            ))
        })?;

    if !resp.status().is_success() {
        return Err(crate::ErrorKind::OtherError(format!(
            "Ely.by skin not available (HTTP {})",
            resp.status()
        ))
        .into());
    }

    let bytes = resp.bytes().await.map_err(|e| {
        crate::ErrorKind::OtherError(format!(
            "Failed to read Ely.by skin data: {e}"
        ))
    })?;

    Ok(bytes.to_vec())
}
