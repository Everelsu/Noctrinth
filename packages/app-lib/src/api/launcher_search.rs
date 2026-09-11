//! Where else a launcher might be.
//!
//! Importing from another launcher starts by finding it, and until now that was
//! one guess per launcher: the folder its installer uses by default. Anybody
//! who keeps their games on another drive, unpacked a portable build into
//! Downloads, or installed through Scoop had to find the path themselves and
//! type it in — which is the sort of thing people give up on.
//!
//! So each launcher gets a list of places to look instead of one, and a file
//! that proves it is really there: a folder named `MultiMC` with nothing of
//! MultiMC's in it is not a MultiMC, and offering it would only produce an
//! error later.

use std::path::{Path, PathBuf};

use crate::api::pack::import::ImportLauncherType;

/// A file that is only there if this launcher is.
///
/// `None` means the folder existing is as much proof as there is — the Modrinth
/// App keeps everything in a database whose name is not worth pinning here.
fn marker_of(launcher: ImportLauncherType) -> Option<&'static [&'static str]> {
    match launcher {
        ImportLauncherType::MultiMC => Some(&["multimc.cfg"]),
        ImportLauncherType::PrismLauncher => {
            Some(&["prismlauncher.cfg", "polymc.cfg"])
        }
        ImportLauncherType::ATLauncher => {
            Some(&["configs", "instances", "ATLauncher.json"])
        }
        ImportLauncherType::GDLauncher => Some(&["instances"]),
        ImportLauncherType::Curseforge => Some(&["Instances"]),
        ImportLauncherType::ModrinthApp => Some(&["profiles", "app.db"]),
        ImportLauncherType::Unknown => None,
    }
}

/// The folder names this launcher is installed under, in the wild.
fn names_of(launcher: ImportLauncherType) -> &'static [&'static str] {
    match launcher {
        ImportLauncherType::MultiMC => &["MultiMC", "multimc"],
        ImportLauncherType::PrismLauncher => {
            &["PrismLauncher", "prismlauncher", "PolyMC", "polymc"]
        }
        ImportLauncherType::ATLauncher => &["ATLauncher", "atlauncher"],
        ImportLauncherType::GDLauncher => {
            &["gdlauncher_next", "GDLauncher", "gdlauncher_carbon"]
        }
        ImportLauncherType::Curseforge => &["curseforge", "CurseForge"],
        ImportLauncherType::ModrinthApp => {
            &["ModrinthApp", "com.modrinth.theseus", "Noctrinth"]
        }
        ImportLauncherType::Unknown => &[],
    }
}

/// Every folder a launcher is worth looking for in.
///
/// Roots only: each is joined with the launcher's own names afterwards, so a
/// root that means nothing for one launcher costs nothing for it.
fn roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    for root in [
        dirs::data_dir(),
        dirs::data_local_dir(),
        dirs::config_dir(),
        dirs::home_dir(),
        dirs::document_dir(),
        dirs::desktop_dir(),
        dirs::download_dir(),
    ]
    .into_iter()
    .flatten()
    {
        roots.push(root);
    }

    // A launcher kept beside the games rather than with the settings.
    if let Some(home) = dirs::home_dir() {
        roots.push(home.join("Games"));
        roots.push(home.join("Minecraft"));
        roots.push(home.join("scoop").join("apps"));
    }

    #[cfg(target_os = "windows")]
    {
        for variable in ["ProgramFiles", "ProgramFiles(x86)", "LOCALAPPDATA"] {
            if let Some(path) = std::env::var_os(variable) {
                roots.push(PathBuf::from(path));
            }
        }

        // Portable builds live wherever they were unpacked, and a second drive
        // is where they usually are. Only the roots that exist are kept, so
        // this costs one stat per letter.
        for letter in 'A'..='Z' {
            let drive = PathBuf::from(format!("{letter}:\\"));
            if !drive.is_dir() {
                continue;
            }

            roots.push(drive.join("Games"));
            roots.push(drive.join("Minecraft"));
            roots.push(drive);
        }
    }

    #[cfg(target_os = "macos")]
    {
        roots.push(PathBuf::from("/Applications"));
        if let Some(home) = dirs::home_dir() {
            roots.push(home.join("Applications"));
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(home) = dirs::home_dir() {
            // Flatpak keeps each application's data under its own id.
            roots.push(home.join(".var").join("app"));
            roots.push(home.join(".local").join("share"));
        }
    }

    roots.sort();
    roots.dedup();
    roots
}

/// Whether this folder really is the launcher it is named after.
fn is_really(launcher: ImportLauncherType, path: &Path) -> bool {
    if !path.is_dir() {
        return false;
    }

    match marker_of(launcher) {
        Some(markers) => {
            markers.iter().any(|marker| path.join(marker).exists())
        }
        None => true,
    }
}

/// Everywhere this launcher was found, most likely first.
pub fn find_all(launcher: ImportLauncherType) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut push = |path: PathBuf| {
        if is_really(launcher, &path) && !found.contains(&path) {
            found.push(path);
        }
    };

    // The places the launcher's own installer puts it come first, because they
    // are what a normal install looks like.
    for path in installer_defaults(launcher) {
        push(path);
    }

    for root in roots() {
        for name in names_of(launcher) {
            push(root.join(name));
        }

        // macOS bundles keep their data inside the application.
        #[cfg(target_os = "macos")]
        for name in names_of(launcher) {
            push(root.join(format!("{name}.app")).join("Data"));
        }
    }

    found
}

/// What the launcher's installer would have used, per platform.
fn installer_defaults(launcher: ImportLauncherType) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    match launcher {
        ImportLauncherType::Curseforge => {
            // The CurseForge app asks where to keep its games and suggests
            // these two, in this order.
            if let Some(home) = dirs::home_dir() {
                paths.push(home.join("curseforge").join("minecraft"));
            }
            if let Some(documents) = dirs::document_dir() {
                paths.push(documents.join("curseforge").join("minecraft"));
            }
        }
        ImportLauncherType::GDLauncher => {
            if let Some(data) = dirs::data_dir() {
                paths.push(data.join("gdlauncher_next"));
                paths.push(data.join("gdlauncher_carbon"));
            }
        }
        _ => {
            if let Some(data) = dirs::data_dir() {
                for name in names_of(launcher) {
                    paths.push(data.join(name));
                }
            }
        }
    }

    paths
}

/// The one to offer, of everywhere it was found.
pub fn find(launcher: ImportLauncherType) -> Option<PathBuf> {
    find_all(launcher).into_iter().next()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_folder_with_the_right_name_and_nothing_in_it_is_not_a_launcher() {
        let empty = std::env::temp_dir().join("noctrinth-not-a-multimc");
        let _ = std::fs::create_dir_all(&empty);

        assert!(!is_really(ImportLauncherType::MultiMC, &empty));

        let _ = std::fs::write(empty.join("multimc.cfg"), "");
        assert!(is_really(ImportLauncherType::MultiMC, &empty));

        let _ = std::fs::remove_dir_all(&empty);
    }

    #[test]
    fn a_path_that_is_not_there_is_never_offered() {
        assert!(!is_really(
            ImportLauncherType::PrismLauncher,
            Path::new("/noctrinth/nowhere/at/all"),
        ));
    }

    #[test]
    fn polymc_counts_as_a_prism_launcher() {
        let folder = std::env::temp_dir().join("noctrinth-polymc");
        let _ = std::fs::create_dir_all(&folder);
        let _ = std::fs::write(folder.join("polymc.cfg"), "");

        assert!(is_really(ImportLauncherType::PrismLauncher, &folder));

        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn there_is_somewhere_to_look() {
        assert!(!roots().is_empty());
    }
}
