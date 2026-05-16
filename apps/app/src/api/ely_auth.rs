use crate::api::Result;
use tauri::Runtime;
use tauri::plugin::TauriPlugin;
use theseus::ely_auth;
use theseus::prelude::ElyCredentials;

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    tauri::plugin::Builder::<R>::new("ely-auth")
        .invoke_handler(tauri::generate_handler![
            ely_login,
            ely_logout,
            ely_get_users,
            ely_get_default_user,
            ely_set_default_user,
            ely_get_skin_texture,
        ])
        .build()
}

#[tauri::command]
pub async fn ely_login(username: String, password: String) -> Result<ElyCredentials> {
    Ok(ely_auth::login(&username, &password).await?)
}

#[tauri::command]
pub async fn ely_logout(user: uuid::Uuid) -> Result<()> {
    Ok(ely_auth::logout(user).await?)
}

#[tauri::command]
pub async fn ely_get_users() -> Result<Vec<ElyCredentials>> {
    Ok(ely_auth::users().await?)
}

#[tauri::command]
pub async fn ely_get_default_user() -> Result<Option<uuid::Uuid>> {
    Ok(ely_auth::get_default_user().await?)
}

#[tauri::command]
pub async fn ely_set_default_user(user: uuid::Uuid) -> Result<()> {
    Ok(ely_auth::set_default_user(user).await?)
}

#[tauri::command]
pub async fn ely_get_skin_texture(username: String) -> Result<Vec<u8>> {
    Ok(ely_auth::get_skin_texture(&username).await?)
}
