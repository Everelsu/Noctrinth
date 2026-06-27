//! CurseForge modpack installer.
//!
//! NOTE (0.15 sync): the full installer was written against the pre-0.15
//! profile/pack pipeline (`set_profile_information`, `Profile::get`,
//! `cache_file_hash`, the old loading-bar plumbing), all of which were
//! rewritten upstream. CurseForge modpack installation is being reworked
//! against the new content-management model, so the install entry points are
//! temporarily stubbed. The lightweight, already-ported CurseForge paths
//! (single-file install via `instance::add_project_from_curseforge` and the
//! fingerprint scan) are unaffected.
//!
//! The pure zip-inspection helpers below carry no profile-pipeline dependency
//! and are kept because the file-import detection in `install_from.rs` relies
//! on them to route a dropped `.zip` to the right format.

use std::io::Cursor;

use async_zip::base::read::seek::ZipFileReader;

/// Returns true if the zip looks like a CurseForge modpack — i.e. it contains
/// a `manifest.json`. Used to route file imports to the right installer.
pub async fn zip_has_curseforge_manifest(pack_file: &bytes::Bytes) -> bool {
    zip_has_entry(pack_file, "manifest.json").await
}

/// Returns true if the zip is a Modrinth `.mrpack` — i.e. it contains
/// `modrinth.index.json`. Sibling of `zip_has_curseforge_manifest` so the
/// importer can route between formats without re-parsing the file twice.
pub async fn zip_has_modrinth_index(pack_file: &bytes::Bytes) -> bool {
    zip_has_entry(pack_file, "modrinth.index.json").await
}

async fn zip_has_entry(pack_file: &bytes::Bytes, name: &str) -> bool {
    match ZipFileReader::with_tokio(Cursor::new(pack_file)).await {
        Ok(zip) => zip
            .file()
            .entries()
            .iter()
            .any(|f| matches!(f.filename().as_str(), Ok(n) if n == name)),
        Err(_) => false,
    }
}

fn rework_error() -> crate::Error {
    crate::ErrorKind::InputError(
        "CurseForge modpack installation is being reworked for the 0.15 \
         content-management pipeline and is temporarily unavailable."
            .to_string(),
    )
    .into()
}

/// Install a CurseForge modpack from its download URL.
#[tracing::instrument(skip(_curseforge_api_key))]
pub async fn install_curseforge_pack(
    _modpack_url: &str,
    _curseforge_api_key: &str,
    _profile_path: String,
) -> crate::Result<String> {
    Err(rework_error())
}

/// Install a CurseForge modpack from already-downloaded zip bytes.
#[tracing::instrument(skip(_pack_file, _curseforge_api_key))]
pub async fn install_curseforge_pack_from_zip(
    _pack_file: bytes::Bytes,
    _curseforge_api_key: &str,
    _profile_path: String,
) -> crate::Result<String> {
    Err(rework_error())
}
