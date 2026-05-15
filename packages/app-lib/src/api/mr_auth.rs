use crate::state::ModrinthCredentials;

#[tracing::instrument]
pub fn build_auth_url(redirect_uri: &str) -> String {
    crate::state::build_login_url(redirect_uri)
}

#[tracing::instrument]
pub async fn exchange_code(
    code: &str,
    redirect_uri: &str,
) -> crate::Result<ModrinthCredentials> {
    let state = crate::State::get().await?;

    let creds = crate::state::exchange_code_for_token(
        code,
        redirect_uri,
        &state.api_semaphore,
        &state.pool,
    )
    .await?;

    creds.upsert(&state.pool).await?;
    if let Err(e) = state
        .friends_socket
        .connect(&state.pool, &state.api_semaphore, &state.process_manager)
        .await
    {
        tracing::warn!("Failed to connect to friends socket: {e}");
    }

    Ok(creds)
}

#[tracing::instrument]
pub async fn logout() -> crate::Result<()> {
    let state = crate::State::get().await?;
    let current = ModrinthCredentials::get_active(&state.pool).await?;

    if let Some(current) = current {
        ModrinthCredentials::remove(&current.user_id, &state.pool).await?;
        state.friends_socket.disconnect().await?;
    }

    Ok(())
}

#[tracing::instrument]
pub async fn get_credentials() -> crate::Result<Option<ModrinthCredentials>> {
    let state = crate::State::get().await?;
    let current =
        ModrinthCredentials::get_and_refresh(&state.pool, &state.api_semaphore)
            .await?;

    Ok(current)
}
