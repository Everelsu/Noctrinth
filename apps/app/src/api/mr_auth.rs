use crate::api::Result;
use crate::api::TheseusSerializableError;
use crate::api::oauth_utils;
use tauri::Manager;
use tauri::Runtime;
use tauri::plugin::TauriPlugin;
use tauri_plugin_opener::OpenerExt;
use theseus::prelude::*;
use tokio::sync::oneshot;

/// Fixed loopback port for the Modrinth OAuth redirect URI.
/// Register `http://127.0.0.1:9090` as a redirect URI in your Modrinth OAuth app settings
/// at https://modrinth.com/settings/applications
const MODRINTH_OAUTH_REDIRECT_PORT: u16 = 9090;

pub fn init<R: tauri::Runtime>() -> TauriPlugin<R> {
    tauri::plugin::Builder::new("mr-auth")
        .invoke_handler(tauri::generate_handler![
            modrinth_login,
            logout,
            get,
            cancel_modrinth_login,
        ])
        .build()
}

#[tauri::command]
pub async fn modrinth_login<R: Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<ModrinthCredentials> {
    let redirect_uri =
        format!("http://127.0.0.1:{MODRINTH_OAUTH_REDIRECT_PORT}");

    let (socket_tx, socket_rx) = oneshot::channel();
    let auth_code_task = tokio::spawn(
        oauth_utils::auth_code_reply::listen_on_port(
            MODRINTH_OAUTH_REDIRECT_PORT,
            socket_tx,
        ),
    );

    // Wait until the local listener is ready before opening the browser
    socket_rx.await.unwrap()?;

    let auth_request_uri = mr_auth::build_auth_url(&redirect_uri);

    app.opener()
        .open_url(auth_request_uri, None::<&str>)
        .map_err(|e| {
            TheseusSerializableError::Theseus(
                theseus::ErrorKind::OtherError(format!(
                    "Failed to open Modrinth auth URL: {e}"
                ))
                .into(),
            )
        })?;

    let Some(auth_code) = auth_code_task.await.unwrap()? else {
        return Err(TheseusSerializableError::Theseus(
            theseus::ErrorKind::OtherError("Login canceled".into()).into(),
        ));
    };

    let credentials =
        mr_auth::exchange_code(&auth_code, &redirect_uri).await?;

    if let Some(main_window) = app.get_window("main") {
        main_window.set_focus().ok();
    }

    Ok(credentials)
}

#[tauri::command]
pub async fn logout() -> Result<()> {
    Ok(theseus::mr_auth::logout().await?)
}

#[tauri::command]
pub async fn get() -> Result<Option<ModrinthCredentials>> {
    Ok(theseus::mr_auth::get_credentials().await?)
}

#[tauri::command]
pub fn cancel_modrinth_login() {
    oauth_utils::auth_code_reply::stop_listeners();
}
