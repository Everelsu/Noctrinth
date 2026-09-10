//! Accounts that are only a name, for when nothing can be reached.

use crate::api::Result;
use theseus::offline_auth;
use theseus::prelude::OfflineCredentials;
use uuid::Uuid;

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("noctrinth-offline-auth")
        .invoke_handler(tauri::generate_handler![
            offline_add,
            offline_remove,
            offline_users,
            offline_get_default_user,
            offline_set_default_user,
            offline_preview_uuid,
        ])
        .build()
}

#[tauri::command]
pub async fn offline_add(username: &str) -> Result<OfflineCredentials> {
    Ok(offline_auth::add(username).await?)
}

#[tauri::command]
pub async fn offline_remove(uuid: Uuid) -> Result<()> {
    Ok(offline_auth::remove(uuid).await?)
}

#[tauri::command]
pub async fn offline_users() -> Result<Vec<OfflineCredentials>> {
    Ok(offline_auth::users().await?)
}

#[tauri::command]
pub async fn offline_get_default_user() -> Result<Option<Uuid>> {
    Ok(offline_auth::get_default_user().await?)
}

#[tauri::command]
pub async fn offline_set_default_user(uuid: Uuid) -> Result<()> {
    Ok(offline_auth::set_default_user(uuid).await?)
}

/// The UUID a name would play as, before anything is saved.
#[tauri::command]
pub fn offline_preview_uuid(username: &str) -> Option<Uuid> {
    offline_auth::preview_uuid(username)
}
