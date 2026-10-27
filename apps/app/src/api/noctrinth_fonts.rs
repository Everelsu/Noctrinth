//! The fonts installed on this computer, for the interface font picker in
//! Settings → Appearance.
//!
//! The webview cannot list them — Chromium's `queryLocalFonts` asks for a
//! permission WebView2 never grants, and WebKit has nothing of the kind — so
//! the system's font folders are read here instead.

use crate::api::Result;

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("noctrinth-fonts")
        .invoke_handler(tauri::generate_handler![fonts_installed_families])
        .build()
}

/// Every installed font family, by the name CSS knows it by, sorted and once
/// each.
#[tauri::command]
pub async fn fonts_installed_families() -> Result<Vec<String>> {
    // Reading every font file's names takes a moment; off the async workers.
    let families = tauri::async_runtime::spawn_blocking(|| {
        let mut database = fontdb::Database::new();
        database.load_system_fonts();
        families_of(&database)
    })
    .await
    .map_err(|error| anyhow::anyhow!(error.to_string()))?;
    Ok(families)
}

fn families_of(database: &fontdb::Database) -> Vec<String> {
    let mut families: Vec<String> = database
        .faces()
        // The first name is the English one, which is what CSS matches on.
        .filter_map(|face| face.families.first().map(|(name, _)| name.clone()))
        // Names starting with a dot are the system's own private faces on
        // macOS, which a page cannot ask for.
        .filter(|name| !name.is_empty() && !name.starts_with('.'))
        .collect();
    families.sort_by_key(|name| name.to_lowercase());
    families.dedup();
    families
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_installed_families_once_each() {
        let mut database = fontdb::Database::new();
        database.load_system_fonts();
        let families = families_of(&database);
        let mut unique = families.clone();
        unique.dedup();
        assert_eq!(families, unique);
        assert!(families.iter().all(|name| !name.starts_with('.')));
    }
}
