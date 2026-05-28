//! Top-level modpack install dispatcher.
//!
//! The two supported formats (Modrinth `.mrpack` and CurseForge zip) live
//! in their own modules and **don't know about each other**. This file is
//! the only place where the format is detected and the corresponding
//! installer is invoked.
//!
//! The dispatcher also owns the cleanup-on-error contract: if anything
//! fails — including format detection at the very start — the freshly-
//! created profile is removed so the user doesn't end up with an orphan
//! instance.

use crate::pack::install_curseforge::{
    install_curseforge_pack_from_zip, zip_has_curseforge_manifest,
    zip_has_modrinth_index,
};
use crate::pack::install_from::CreatePackLocation;
use crate::pack::install_mrpack::install_zipped_mrpack;
use crate::util::io;

/// Install a pack — pick the right installer based on the location.
///
/// Behaviour by `CreatePackLocation` variant:
///   - `FromVersionId`: always an `.mrpack` from Modrinth → mrpack installer.
///   - `FromFile`: peek at the zip and route:
///       - `modrinth.index.json` → mrpack installer (file path passed
///         through unchanged; the mrpack installer reads it itself).
///       - `manifest.json` + non-empty CF API key → CurseForge installer.
///       - `manifest.json` + missing key → fail early with a helpful message.
///       - Neither → fail with "not a modpack" before the modpack installer
///         hits a confusing manifest-not-found error.
///
/// On any failure the profile is removed.
pub async fn install_pack(
    location: CreatePackLocation,
    profile_path: String,
) -> crate::Result<String> {
    let result = dispatch(location, profile_path.clone()).await;
    match result {
        Ok(profile) => Ok(profile),
        Err(err) => {
            // Best-effort cleanup — keeps orphan profiles out of the user's
            // instances folder when an install fails for any reason.
            let _ = crate::api::profile::remove(&profile_path).await;
            Err(err)
        }
    }
}

async fn dispatch(
    location: CreatePackLocation,
    profile_path: String,
) -> crate::Result<String> {
    // Version-id installs are always Modrinth modpacks — no detection needed.
    let CreatePackLocation::FromFile {
        path,
        curseforge_api_key,
    } = &location
    else {
        return install_zipped_mrpack(location, profile_path).await;
    };

    // Clone the bits we need so the borrow on `location` ends before we
    // hand it off to a downstream installer that wants to consume it.
    let file_path = path.clone();
    let api_key_opt = curseforge_api_key.clone();
    let bytes = bytes::Bytes::from(io::read(&file_path).await?);
    let is_mrpack = zip_has_modrinth_index(&bytes).await;
    let is_cf = zip_has_curseforge_manifest(&bytes).await;

    if is_mrpack {
        // Hand off to the mrpack installer untouched — it re-reads the file
        // itself, so we don't pass `bytes` through.
        install_zipped_mrpack(location, profile_path).await
    } else if is_cf {
        let api_key = api_key_opt
            .as_deref()
            .map(str::trim)
            .filter(|k| !k.is_empty())
            .ok_or_else(|| {
                crate::ErrorKind::InputError(
                    "Cannot install a CurseForge modpack without a \
                     CurseForge API key."
                        .to_string(),
                )
            })?;
        install_curseforge_pack_from_zip(bytes, api_key, profile_path).await
    } else {
        Err(crate::ErrorKind::InputError(
            "The selected file isn't a recognised modpack — expected a \
             Modrinth .mrpack or a CurseForge modpack zip."
                .to_string(),
        )
        .into())
    }
}
