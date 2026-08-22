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

use tauri::Runtime;

use crate::api::Result;

pub fn init<R: Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("window-icon")
        .invoke_handler(tauri::generate_handler![sync_taskbar_icon])
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

#[cfg(windows)]
mod imp {
    use tauri::Runtime;
    use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        CopyIcon, DestroyIcon, HICON, ICON_BIG, ICON_SMALL, SendMessageW,
        WM_GETICON, WM_SETICON,
    };

    use crate::api::Result;

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
