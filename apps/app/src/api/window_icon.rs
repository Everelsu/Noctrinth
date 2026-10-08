//! Keeping the taskbar's copy of the window icon in step with the window's.
//!
//! Windows keeps two icons per window: a small one, drawn in the window menu
//! and title bar, and a big one, which is what the taskbar button and Alt-Tab
//! read. `Window::set_icon` only ever sets the small one — tao sends
//! `WM_SETICON` with `ICON_SMALL` and offers the big one behind a separate,
//! Windows-only call that Tauri does not expose — so recolouring the icon left
//! the taskbar showing whatever was compiled into the executable.
//!
//! Rather than rasterise the mark a second time, this copies the icon the
//! window already has into the big slot: the copy is the app's own, so
//! replacing the small icon later cannot leave the taskbar pointing at a handle
//! that has been destroyed.
//!
//! None of that reaches an installed launcher pinned to the taskbar: its button
//! is the pinned shortcut's, and the shortcut's icon is what it shows, whatever
//! the window says. So the shortcuts that start this very executable — pinned,
//! in the Start menu, on the desktop — are given the accent's icon too, written
//! out as an `.ico` of their own.

use tauri::Runtime;

use crate::api::Result;

pub fn init<R: Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("window-icon")
        .invoke_handler(tauri::generate_handler![
            sync_taskbar_icon,
            paint_shortcut_icons
        ])
        .build()
}

/// Copies the window's icon into the slot the taskbar reads.
///
/// A no-op everywhere but Windows, which is the only platform that keeps the
/// two apart.
#[tauri::command]
pub async fn sync_taskbar_icon<R: Runtime>(
    window: tauri::Window<R>,
) -> Result<()> {
    #[cfg(windows)]
    {
        imp::sync(&window)?;
    }

    #[cfg(not(windows))]
    let _ = window;

    Ok(())
}

/// One size of the accent icon, as a PNG.
#[derive(serde::Deserialize)]
pub struct IconImage {
    size: u32,
    png: Vec<u8>,
}

/// Gives this executable's shortcuts the accent's icon, or their own back.
///
/// `key` names the icon file, so a new colour is a new path: the shell caches
/// icons by path and would keep drawing the old one from a file rewritten in
/// place. No images means the theme's own colour, which is the icon built into
/// the executable, so the shortcuts are pointed back at it.
///
/// Only shortcuts whose target is this executable are touched. A development
/// build is a different executable, so it leaves an installed launcher's
/// shortcuts alone.
#[tauri::command]
pub async fn paint_shortcut_icons<R: Runtime>(
    app: tauri::AppHandle<R>,
    key: String,
    images: Vec<IconImage>,
) -> Result<()> {
    #[cfg(windows)]
    {
        use tauri::Manager;

        let exe = std::env::current_exe()?;
        let icon_path = if images.is_empty() {
            None
        } else {
            let dir = app.path().app_data_dir()?.join("icons").join("accent");
            std::fs::create_dir_all(&dir)?;
            let safe_key: String = key
                .chars()
                .filter(|c| c.is_ascii_alphanumeric())
                .take(32)
                .collect();
            let path = dir.join(format!("noctrinth-{safe_key}.ico"));
            std::fs::write(&path, encode_ico(&images))?;
            Some(path)
        };

        let target = icon_path.clone();
        tokio::task::spawn_blocking(move || {
            imp::repoint_shortcuts(&exe, target.as_deref())
        })
        .await
        .map_err(|e| {
            theseus::ErrorKind::OtherError(format!(
                "Shortcut icon task failed: {e}"
            ))
            .as_error()
        })?;

        // The files of colours no shortcut uses any more.
        if let Some(current) = icon_path
            && let Some(dir) = current.parent()
            && let Ok(entries) = std::fs::read_dir(dir)
        {
            for entry in entries.flatten() {
                if entry.path() != current {
                    let _ = std::fs::remove_file(entry.path());
                }
            }
        }
    }

    #[cfg(not(windows))]
    let _ = (app, key, images);

    Ok(())
}

/// An `.ico` holding each size as the PNG it already is, which every Windows
/// since Vista reads.
fn encode_ico(images: &[IconImage]) -> Vec<u8> {
    let count = images.len() as u16;
    let mut out = Vec::new();
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&count.to_le_bytes());

    let mut offset = 6 + 16 * images.len() as u32;
    for image in images {
        // 256 is written as 0: the field is a byte.
        let side = if image.size >= 256 {
            0
        } else {
            image.size as u8
        };
        out.extend_from_slice(&[side, side, 0, 0]);
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&32u16.to_le_bytes());
        out.extend_from_slice(&(image.png.len() as u32).to_le_bytes());
        out.extend_from_slice(&offset.to_le_bytes());
        offset += image.png.len() as u32;
    }
    for image in images {
        out.extend_from_slice(&image.png);
    }
    out
}

#[cfg(windows)]
mod imp {
    use tauri::Runtime;
    use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        CopyIcon, DestroyIcon, HICON, ICON_BIG, ICON_SMALL, SendMessageW,
        WM_GETICON, WM_SETICON,
    };

    use crate::api::Result;

    /// Points every shortcut to `exe` at `icon`, or back at the executable's
    /// own icon when there is none, and tells the shell to redraw them.
    pub fn repoint_shortcuts(
        exe: &std::path::Path,
        icon: Option<&std::path::Path>,
    ) {
        use windows::Win32::System::Com::{
            CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance,
            CoInitializeEx, CoUninitialize, IPersistFile, STGM_READWRITE,
        };
        use windows::Win32::UI::Shell::{
            IShellLinkW, SHCNE_ASSOCCHANGED, SHCNF_IDLIST, SHChangeNotify,
            ShellLink,
        };
        use windows::core::{HSTRING, Interface};

        let exe_text = exe.to_string_lossy().to_lowercase();
        let (icon_file, icon_index) = match icon {
            Some(path) => (path.to_path_buf(), 0),
            None => (exe.to_path_buf(), 0),
        };
        let icon_text = icon_file.to_string_lossy().to_string();

        // SAFETY: COM is initialised for this blocking thread and released at
        // the end; every interface below is used only on it.
        unsafe {
            let initialised =
                CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_ok();
            let mut changed = 0usize;

            for path in shortcut_files() {
                let Ok(link) = CoCreateInstance::<_, IShellLinkW>(
                    &ShellLink,
                    None,
                    CLSCTX_INPROC_SERVER,
                ) else {
                    continue;
                };
                let Ok(file) = link.cast::<IPersistFile>() else {
                    continue;
                };
                let wide = HSTRING::from(path.as_os_str());
                if file.Load(&wide, STGM_READWRITE).is_err() {
                    continue;
                }

                let mut target = [0u16; 1024];
                if link.GetPath(&mut target, std::ptr::null_mut(), 0).is_err() {
                    continue;
                }
                let target = String::from_utf16_lossy(&target)
                    .trim_end_matches('\0')
                    .to_lowercase();
                if target != exe_text {
                    continue;
                }

                let mut current = [0u16; 1024];
                let mut index = 0;
                let _ = link.GetIconLocation(&mut current, &mut index);
                let current = String::from_utf16_lossy(&current)
                    .trim_end_matches('\0')
                    .to_string();
                if current.eq_ignore_ascii_case(&icon_text)
                    && index == icon_index
                {
                    continue;
                }

                if link
                    .SetIconLocation(
                        &HSTRING::from(icon_text.as_str()),
                        icon_index,
                    )
                    .is_ok()
                    && file.Save(None, true).is_ok()
                {
                    changed += 1;
                } else {
                    tracing::debug!(
                        "Could not repaint the shortcut at {}",
                        path.display()
                    );
                }
            }

            if changed > 0 {
                tracing::info!("Repainted {changed} launcher shortcut icon(s)");
                SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, None, None);
            }
            if initialised {
                CoUninitialize();
            }
        }
    }

    /// Every `.lnk` a launcher shortcut is likely to be: pinned to the taskbar
    /// or the Start menu, in the Start menu's programs, on the desktop.
    fn shortcut_files() -> Vec<std::path::PathBuf> {
        use windows::Win32::System::Com::CoTaskMemFree;
        use windows::Win32::UI::Shell::{
            FOLDERID_CommonPrograms, FOLDERID_Desktop, FOLDERID_Programs,
            FOLDERID_PublicDesktop, KF_FLAG_DEFAULT, SHGetKnownFolderPath,
        };

        let mut folders = Vec::new();
        for id in [
            FOLDERID_Programs,
            FOLDERID_CommonPrograms,
            FOLDERID_Desktop,
            FOLDERID_PublicDesktop,
        ] {
            // SAFETY: the returned string is read once and then freed.
            unsafe {
                if let Ok(path) =
                    SHGetKnownFolderPath(&id, KF_FLAG_DEFAULT, None)
                {
                    if let Ok(text) = path.to_string() {
                        folders.push(std::path::PathBuf::from(text));
                    }
                    CoTaskMemFree(Some(path.0 as *const _));
                }
            }
        }
        if let Some(app_data) = std::env::var_os("APPDATA") {
            let pinned = std::path::PathBuf::from(app_data)
                .join("Microsoft")
                .join("Internet Explorer")
                .join("Quick Launch")
                .join("User Pinned");
            folders.push(pinned.join("TaskBar"));
            folders.push(pinned.join("StartMenu"));
        }

        let mut files = Vec::new();
        for folder in folders {
            collect_links(&folder, 1, &mut files);
        }
        files
    }

    /// `.lnk` files in `dir`, and `depth` levels of folders below it, which is
    /// as deep as an installer puts a shortcut.
    fn collect_links(
        dir: &std::path::Path,
        depth: usize,
        files: &mut Vec<std::path::PathBuf>,
    ) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if depth > 0 {
                    collect_links(&path, depth - 1, files);
                }
            } else if path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("lnk"))
            {
                files.push(path);
            }
        }
    }

    /// The big icon this last installed, kept so it can be destroyed once the
    /// window has been given a newer one. Windows does not own these: whoever
    /// sets an icon is responsible for freeing the one it replaced.
    static PREVIOUS_BIG_ICON: std::sync::Mutex<Option<isize>> =
        std::sync::Mutex::new(None);

    pub fn sync<R: Runtime>(window: &tauri::Window<R>) -> Result<()> {
        let hwnd = HWND(window.hwnd()?.0);

        // SAFETY: the handle comes from the window Tauri is holding open, and
        // every call below is a plain message send or an icon copy against it.
        unsafe {
            let small = SendMessageW(
                hwnd,
                WM_GETICON,
                Some(WPARAM(ICON_SMALL as usize)),
                Some(LPARAM(0)),
            );
            if small.0 == 0 {
                // Nothing has set a small icon yet; the class icon is showing
                // and there is nothing to mirror.
                return Ok(());
            }

            let copy = match CopyIcon(HICON(small.0 as *mut _)) {
                Ok(copy) => copy,
                Err(error) => {
                    tracing::warn!(
                        "Failed to copy the window icon for the taskbar: {error}"
                    );
                    return Ok(());
                }
            };

            SendMessageW(
                hwnd,
                WM_SETICON,
                Some(WPARAM(ICON_BIG as usize)),
                Some(LPARAM(copy.0 as isize)),
            );

            // Free the copy this replaced, now that the window is no longer
            // drawing it.
            let mut previous =
                PREVIOUS_BIG_ICON.lock().unwrap_or_else(|error| {
                    PREVIOUS_BIG_ICON.clear_poison();
                    error.into_inner()
                });
            if let Some(handle) = previous.replace(copy.0 as isize) {
                let _ = DestroyIcon(HICON(handle as *mut _));
            }
        }

        Ok(())
    }
}
