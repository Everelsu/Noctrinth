//! Adding, choosing and removing accounts that are only a name.
//!
//! See [`crate::state::OfflineCredentials`] for what one of these is and what
//! it can and cannot do.

use uuid::Uuid;

use crate::State;
use crate::state::OfflineCredentials;

/// Adds an account for `username`, and makes it the one to play as.
#[tracing::instrument]
pub async fn add(username: &str) -> crate::Result<OfflineCredentials> {
    let state = State::get().await?;

    let account = OfflineCredentials::create(username, &state.pool).await?;

    // The checklist's "sign in to Minecraft" step is about having an account to
    // play with, and an offline one is exactly that. Neither this nor the
    // interface asks anything else, but the sidebar's "Playing as" card is only
    // shown once that step is done — so an account added here was written down
    // and then had nowhere to appear, which reads as the account never having
    // been added at all.
    if let Err(error) =
        crate::onboarding_checklist::mark_logged_into_minecraft().await
    {
        tracing::warn!(
            "Failed to mark offline account in onboarding checklist: {error}"
        );
    }

    Ok(account)
}

#[tracing::instrument]
pub async fn remove(uuid: Uuid) -> crate::Result<()> {
    let state = State::get().await?;

    OfflineCredentials::remove(uuid, &state.pool).await
}

#[tracing::instrument]
pub async fn users() -> crate::Result<Vec<OfflineCredentials>> {
    let state = State::get().await?;

    OfflineCredentials::get_all(&state.pool).await
}

#[tracing::instrument]
pub async fn get_default_user() -> crate::Result<Option<Uuid>> {
    let state = State::get().await?;

    Ok(OfflineCredentials::get_active(&state.pool)
        .await?
        .map(|account| account.uuid))
}

#[tracing::instrument]
pub async fn set_default_user(uuid: Uuid) -> crate::Result<()> {
    let state = State::get().await?;

    OfflineCredentials::set_active(uuid, &state.pool).await
}

/// The UUID a name would play as, without adding anything.
///
/// The interface shows it before the account is made, because it is the one
/// thing about an offline account that is not obvious and the one thing that
/// decides whether an existing world recognises the player.
pub fn preview_uuid(username: &str) -> Option<Uuid> {
    OfflineCredentials::is_valid_username(username.trim())
        .then(|| OfflineCredentials::uuid_for(username.trim()))
}
