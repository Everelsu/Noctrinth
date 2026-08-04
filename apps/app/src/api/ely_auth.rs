use crate::api::Result;
use tauri::plugin::TauriPlugin;
use tauri::{Emitter, Manager, Runtime};
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
            ely_open_skin_window,
        ])
        .build()
}

#[tauri::command]
pub async fn ely_login(
    username: String,
    password: String,
) -> Result<ElyCredentials> {
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

/// Label of the embedded Ely.by skin-management window. The frontend listens
/// for `ELY_SKIN_WINDOW_CLOSED_EVENT` to know when to refresh the skin preview.
const ELY_SKIN_WINDOW_LABEL: &str = "ely-skin-manager";
const ELY_SKIN_WINDOW_CLOSED_EVENT: &str = "ely-skin-window-closed";

/// Opens (or focuses) an embedded webview window with the Ely.by skin upload
/// page. Ely.by has no public skin-upload API, so the website is the only
/// supported way to change a skin — embedding it keeps the flow inside the
/// app. When the window is closed, an event is emitted so the Skins page can
/// drop its texture cache and re-render the preview.
///
/// Closing hides the window instead of destroying it. Ely.by cannot be logged
/// in from the launcher's own credentials — its OAuth2 scopes cover account
/// info and Minecraft sessions, not the website session, and no skin API
/// exists — so the sign-in inside this webview is the only one there is.
/// Keeping the webview alive keeps that sign-in, which means the page is
/// reached logged in on every subsequent open rather than only the first.
#[tauri::command]
pub async fn ely_open_skin_window<R: Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<()> {
    if let Some(existing) = app.get_webview_window(ELY_SKIN_WINDOW_LABEL) {
        // Send it back to the upload page: it was left wherever the user
        // navigated to before hiding it.
        existing
            .eval("window.location.assign('https://ely.by/skins/add')")
            .ok();
        existing.show().ok();
        existing.set_focus().ok();
        return Ok(());
    }

    let url: tauri::Url = "https://ely.by/skins/add"
        .parse()
        .expect("static Ely.by URL must parse");

    let window = tauri::WebviewWindowBuilder::new(
        &app,
        ELY_SKIN_WINDOW_LABEL,
        tauri::WebviewUrl::External(url),
    )
    .title("Ely.by — skin")
    .inner_size(1080.0, 800.0)
    .center()
    .build()?;

    let handle = app.clone();
    window.on_window_event(move |event| {
        match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                if let Some(window) =
                    handle.get_webview_window(ELY_SKIN_WINDOW_LABEL)
                {
                    window.hide().ok();
                }
                handle.emit(ELY_SKIN_WINDOW_CLOSED_EVENT, ()).ok();
            }
            // Still emitted on app shutdown, when the window really does go.
            tauri::WindowEvent::Destroyed => {
                handle.emit(ELY_SKIN_WINDOW_CLOSED_EVENT, ()).ok();
            }
            _ => {}
        }
    });

    Ok(())
}
