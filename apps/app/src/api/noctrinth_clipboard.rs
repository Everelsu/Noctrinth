//! Files on the clipboard, the way Explorer puts them there.
//!
//! The webview can hold one image at a time and nothing else, so copying five
//! screenshots used to copy the first. Windows keeps a list of files in the
//! clipboard as `CF_HDROP`, which is what Ctrl+C in Explorer does and what
//! Ctrl+V into a folder, a chat or a message reads.

use crate::api::Result;

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("noctrinth-clipboard")
        .invoke_handler(tauri::generate_handler![clipboard_copy_files])
        .build()
}

/// Puts `paths` on the clipboard as files. Fails where the platform has no
/// such thing, and the caller falls back to copying one image.
#[tauri::command]
pub async fn clipboard_copy_files(paths: Vec<String>) -> Result<()> {
    tauri::async_runtime::spawn_blocking(move || copy_files(&paths))
        .await
        .map_err(|error| {
            theseus::ErrorKind::OtherError(error.to_string()).as_error()
        })?
        .map_err(|error| theseus::ErrorKind::OtherError(error).as_error())?;
    Ok(())
}

#[cfg(windows)]
fn copy_files(paths: &[String]) -> std::result::Result<(), String> {
    use windows::Win32::Foundation::{GlobalFree, HANDLE, HGLOBAL};
    use windows::Win32::System::DataExchange::{
        CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData,
    };
    use windows::Win32::System::Memory::{
        GMEM_MOVEABLE, GMEM_ZEROINIT, GlobalAlloc, GlobalLock, GlobalUnlock,
    };
    use windows::Win32::System::Ole::CF_HDROP;
    use windows::Win32::UI::Shell::DROPFILES;

    if paths.is_empty() {
        return Ok(());
    }

    // `DROPFILES`, then every path as a NUL-terminated UTF-16 string, then
    // one more NUL to end the list.
    let mut names: Vec<u16> = Vec::new();
    for path in paths {
        names.extend(path.encode_utf16());
        names.push(0);
    }
    names.push(0);

    let header = std::mem::size_of::<DROPFILES>();
    let size = header + names.len() * std::mem::size_of::<u16>();

    unsafe {
        let memory: HGLOBAL = GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, size)
            .map_err(|error| error.to_string())?;
        let base = GlobalLock(memory).cast::<u8>();
        if base.is_null() {
            let _ = GlobalFree(Some(memory));
            return Err("Could not lock clipboard memory".to_string());
        }
        let drop_files = base.cast::<DROPFILES>();
        (*drop_files).pFiles = header as u32;
        (*drop_files).fWide = true.into();
        std::ptr::copy_nonoverlapping(
            names.as_ptr().cast::<u8>(),
            base.add(header),
            names.len() * std::mem::size_of::<u16>(),
        );
        let _ = GlobalUnlock(memory);

        // Another program can be holding the clipboard for a moment.
        let mut opened = OpenClipboard(None);
        for _ in 0..5 {
            if opened.is_ok() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
            opened = OpenClipboard(None);
        }
        if let Err(error) = opened {
            let _ = GlobalFree(Some(memory));
            return Err(error.to_string());
        }

        let result = EmptyClipboard().and_then(|()| {
            SetClipboardData(u32::from(CF_HDROP.0), Some(HANDLE(memory.0)))
        });
        let _ = CloseClipboard();

        // Once set, the memory is the clipboard's; only a failed hand-over is
        // still ours to free.
        if let Err(error) = result {
            let _ = GlobalFree(Some(memory));
            return Err(error.to_string());
        }
    }

    Ok(())
}

#[cfg(not(windows))]
fn copy_files(_paths: &[String]) -> std::result::Result<(), String> {
    Err(
        "Copying files to the clipboard is only supported on Windows"
            .to_string(),
    )
}
