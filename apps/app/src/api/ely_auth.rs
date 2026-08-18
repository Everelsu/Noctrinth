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
            ely_list_skins,
            ely_current_skin_url,
            ely_get_texture,
            ely_wear_skin,
            ely_upload_skin,
            ely_remove_skin,
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
/// Lists the skins the given Ely.by user has uploaded to the public catalogue.
#[tauri::command]
pub async fn ely_list_skins(
    username: &str,
) -> Result<Vec<theseus::ely_auth::ElyUploadedSkin>> {
    Ok(theseus::ely_auth::list_uploaded_skins(username).await?)
}

/// Fetches an Ely.by texture as raw bytes, for textures the frontend cannot
/// reach itself.
#[tauri::command]
pub async fn ely_get_texture(url: &str) -> Result<Vec<u8>> {
    Ok(theseus::ely_auth::get_texture_bytes(url).await?)
}

/// The storage URL of the skin the account currently wears, used to mark the
/// active entry in the grid.
#[tauri::command]
pub async fn ely_current_skin_url(username: &str) -> Result<Option<String>> {
    Ok(theseus::ely_auth::get_current_skin_url(username).await?)
}

/// The website call that puts a catalogue skin on the signed-in account.
///
/// `/skins/wear` wants nothing but the skin ID and the website session cookie —
/// there is no CSRF token — so it can be driven from inside the embedded
/// webview, which is the only place that session exists.
fn wear_skin_script(skin_id: u64) -> String {
    format!(
        "fetch('https://ely.by/skins/wear', {{             method: 'POST',             credentials: 'same-origin',             headers: {{                 'Content-Type': 'application/x-www-form-urlencoded; charset=UTF-8',                 'X-Requested-With': 'XMLHttpRequest'             }},             body: 'skinId={skin_id}'         }}).catch(function () {{}});"
    )
}

/// Deletes one of the account's skins from the Ely.by catalogue.
///
/// `POST /skins/remove/<id>` with an empty body and the site session; nothing
/// else is required.
#[tauri::command]
pub async fn ely_remove_skin<R: Runtime>(
    app: tauri::AppHandle<R>,
    skin_id: u64,
) -> Result<()> {
    run_in_skin_window(
        app,
        format!(
            "fetch('https://ely.by/skins/remove/{skin_id}', {{                 method: 'POST',                 credentials: 'same-origin',                 headers: {{ 'X-Requested-With': 'XMLHttpRequest' }}             }}).catch(function () {{}});"
        ),
    )
}

/// Uploads a PNG to the account's Ely.by catalogue and wears it.
///
/// Two website calls in sequence, both needing the site session. `/skins/upload`
/// takes a multipart body with a `file` field and answers with the new skin's
/// edit URL, which is where its ID comes from; `/skins/wear` then puts it on.
/// Chaining them inside one injected script avoids needing a channel back out
/// of the webview between the two.
fn upload_skin_script(data_url: &str) -> String {
    format!(
        "(async function () {{             try {{                 const blob = await (await fetch('{data_url}')).blob();                 const form = new FormData();                 form.append('file', new File([blob], 'skin.png', {{ type: 'image/png' }}));                 const uploaded = await fetch('https://ely.by/skins/upload', {{                     method: 'POST',                     credentials: 'same-origin',                     headers: {{ 'X-Requested-With': 'XMLHttpRequest' }},                     body: form                 }}).then(function (r) {{ return r.json(); }});                 const marker = '/skins/s';                 const at = String(uploaded.url || '').indexOf(marker);                 if (at < 0) return;                 const id = parseInt(String(uploaded.url).slice(at + marker.length), 10);                 if (!id) return;                 await fetch('https://ely.by/skins/wear', {{                     method: 'POST',                     credentials: 'same-origin',                     headers: {{                         'Content-Type': 'application/x-www-form-urlencoded; charset=UTF-8',                         'X-Requested-With': 'XMLHttpRequest'                     }},                     body: 'skinId=' + id                 }});             }} catch (e) {{}}         }})();"
    )
}

/// Uploads a skin file to Ely.by and wears it.
///
/// `data_url` is the PNG as a `data:` URL, which is what the injected script
/// can turn back into a file without touching the filesystem from the webview.
#[tauri::command]
pub async fn ely_upload_skin<R: Runtime>(
    app: tauri::AppHandle<R>,
    data_url: &str,
) -> Result<()> {
    run_in_skin_window(app, upload_skin_script(data_url))
}

/// Wears one of the account's uploaded skins.
///
/// The skin grid in the app is drawn from a public listing, but actually
/// putting one on needs the website session. That session lives in the
/// embedded skin window, so the request is issued from there. When the window
/// does not exist yet it is created hidden and the call is deferred until the
/// page has loaded.
///
/// Whether it took effect is not reported back: a webview on a remote origin
/// has no IPC channel to answer through. The caller re-reads the skin texture
/// instead, which is the same thing the user would look at.
#[tauri::command]
pub async fn ely_wear_skin<R: Runtime>(
    app: tauri::AppHandle<R>,
    skin_id: u64,
) -> Result<()> {
    run_in_skin_window(app, wear_skin_script(skin_id))
}

/// Runs a script in the embedded Ely.by window, creating it hidden if it is not
/// open yet and deferring until the page has loaded.
fn run_in_skin_window<R: Runtime>(
    app: tauri::AppHandle<R>,
    script: String,
) -> Result<()> {
    if let Some(existing) = app.get_webview_window(ELY_SKIN_WINDOW_LABEL) {
        existing.eval(&script).ok();
        return Ok(());
    }

    let url: tauri::Url = "https://ely.by/skins/add"
        .parse()
        .expect("static Ely.by URL must parse");

    tauri::WebviewWindowBuilder::new(
        &app,
        ELY_SKIN_WINDOW_LABEL,
        tauri::WebviewUrl::External(url),
    )
    .title("Ely.by — skin")
    .inner_size(1080.0, 800.0)
    .center()
    .visible(false)
    .on_page_load(move |window, _| {
        window.eval(&script).ok();
    })
    .build()?;

    Ok(())
}

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
