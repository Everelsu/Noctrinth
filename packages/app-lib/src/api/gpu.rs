//! Which GPU the game gets on a laptop with two of them.
//!
//! Minecraft is whatever GPU the *Java binary* is given, and on Windows that is
//! decided per executable path by `HKCU\Software\Microsoft\DirectX\
//! UserGpuPreferences` — the same store the system's "Graphics settings" screen
//! writes to.
//!
//! Keying on the path is what makes this worth automating. The launcher
//! installs each runtime into a directory named after its exact version, so
//! every Java update produces a path Windows has never seen, silently dropping
//! back to the default adapter — which on a laptop is usually the integrated
//! one. Someone who set this up once in Windows finds it undone by an update
//! they never noticed, with the old runtime's setting still in the registry.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// A display adapter present in the system.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct GpuAdapter {
    pub name: String,
    /// Best-effort guess, for ordering and wording only — never for a decision.
    pub likely_discrete: bool,
}

/// What Windows should do with a given executable.
///
/// These map onto the values the OS itself stores, and onto the wording of its
/// own settings screen. Which physical adapter "high performance" resolves to
/// is the driver's call, not ours.
#[derive(
    Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default,
)]
#[serde(rename_all = "snake_case")]
pub enum GpuPreference {
    /// No entry: Windows decides, which on a hybrid laptop usually means the
    /// integrated adapter.
    #[default]
    Auto,
    PowerSaving,
    HighPerformance,
}

impl GpuPreference {
    fn from_registry_value(value: &str) -> Self {
        // Stored as `GpuPreference=N;`, sometimes alongside other fields.
        for part in value.split(';') {
            if let Some(number) = part.trim().strip_prefix("GpuPreference=") {
                return match number.trim() {
                    "1" => Self::PowerSaving,
                    "2" => Self::HighPerformance,
                    _ => Self::Auto,
                };
            }
        }
        Self::Auto
    }

    fn to_registry_value(self) -> Option<String> {
        match self {
            // Windows treats a missing entry and `=0` alike; removing it keeps
            // the registry from filling up with runtimes that no longer exist.
            Self::Auto => None,
            Self::PowerSaving => Some("GpuPreference=1;".to_string()),
            Self::HighPerformance => Some("GpuPreference=2;".to_string()),
        }
    }
}

/// The GPU situation for one Java runtime.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JavaGpuStatus {
    /// Whether this platform can be told which GPU to use at all.
    pub supported: bool,
    /// Why not, for the UI to show verbatim.
    pub unsupported_reason: Option<String>,
    /// The executable the preference applies to.
    pub java_path: String,
    pub preference: GpuPreference,
    pub adapters: Vec<GpuAdapter>,
}

/// Names that mean "this is the chip inside the CPU".
///
/// Integrated parts are named for the CPU line rather than a card, and vendors
/// are consistent enough about it for a hint. It only ever changes wording, so
/// a wrong guess costs nothing.
fn looks_integrated(name: &str) -> bool {
    let name = name.to_lowercase();
    const INTEGRATED_MARKERS: [&str; 7] = [
        "uhd graphics",
        "hd graphics",
        "iris",
        "vega",
        "radeon(tm) graphics",
        "radeon graphics",
        "microsoft basic display",
    ];
    INTEGRATED_MARKERS
        .iter()
        .any(|marker| name.contains(marker))
}

#[cfg(target_os = "windows")]
const GPU_PREFERENCES_KEY: &str =
    r"Software\Microsoft\DirectX\UserGpuPreferences";

#[cfg(target_os = "windows")]
pub fn list_adapters() -> Vec<GpuAdapter> {
    use winreg::RegKey;
    use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ};

    // The display adapter device class. Enumerating it avoids pulling in WMI
    // or a graphics API just to read a handful of names.
    const DISPLAY_CLASS: &str = r"SYSTEM\CurrentControlSet\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}";

    let local_machine = RegKey::predef(HKEY_LOCAL_MACHINE);
    let Ok(class_key) =
        local_machine.open_subkey_with_flags(DISPLAY_CLASS, KEY_READ)
    else {
        return Vec::new();
    };

    let mut adapters: Vec<GpuAdapter> = Vec::new();

    for subkey_name in class_key.enum_keys().flatten() {
        // Only the numbered instance keys hold adapters; `Properties` and
        // friends sit alongside them.
        if !subkey_name.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }

        let Ok(subkey) =
            class_key.open_subkey_with_flags(&subkey_name, KEY_READ)
        else {
            continue;
        };
        let Ok(name) = subkey.get_value::<String, _>("DriverDesc") else {
            continue;
        };

        // The same adapter can appear more than once across driver revisions.
        if adapters.iter().any(|adapter| adapter.name == name) {
            continue;
        }

        adapters.push(GpuAdapter {
            likely_discrete: !looks_integrated(&name),
            name,
        });
    }

    // Discrete first: it is the one people are looking for.
    adapters.sort_by_key(|adapter| !adapter.likely_discrete);
    adapters
}

#[cfg(not(target_os = "windows"))]
pub fn list_adapters() -> Vec<GpuAdapter> {
    Vec::new()
}

#[cfg(target_os = "windows")]
pub fn get_preference(java_path: &Path) -> crate::Result<GpuPreference> {
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};

    let current_user = RegKey::predef(HKEY_CURRENT_USER);
    let Ok(key) =
        current_user.open_subkey_with_flags(GPU_PREFERENCES_KEY, KEY_READ)
    else {
        // No key at all simply means nothing has ever been configured.
        return Ok(GpuPreference::Auto);
    };

    let name = java_path.to_string_lossy().to_string();
    Ok(key
        .get_value::<String, _>(&name)
        .map(|value| GpuPreference::from_registry_value(&value))
        .unwrap_or_default())
}

#[cfg(not(target_os = "windows"))]
pub fn get_preference(_java_path: &Path) -> crate::Result<GpuPreference> {
    Ok(GpuPreference::Auto)
}

#[cfg(target_os = "windows")]
pub fn set_preference(
    java_path: &Path,
    preference: GpuPreference,
) -> crate::Result<()> {
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, KEY_ALL_ACCESS};

    let current_user = RegKey::predef(HKEY_CURRENT_USER);
    // Per-user, so this never needs administrator rights.
    let (key, _) = current_user
        .create_subkey_with_flags(GPU_PREFERENCES_KEY, KEY_ALL_ACCESS)?;

    let name = java_path.to_string_lossy().to_string();
    match preference.to_registry_value() {
        Some(value) => key.set_value(&name, &value)?,
        None => {
            // Deleting a value that was never there is not a failure.
            let _ = key.delete_value(&name);
        }
    }

    tracing::info!(
        java_path = %java_path.display(),
        ?preference,
        "Set GPU preference for Java runtime"
    );

    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn set_preference(
    _java_path: &Path,
    _preference: GpuPreference,
) -> crate::Result<()> {
    Err(crate::ErrorKind::OtherError(
        "Choosing a GPU per application is only supported on Windows"
            .to_string(),
    )
    .into())
}

/// The GPU situation for `java_path`, ready to put in front of someone.
pub fn status(java_path: &Path) -> crate::Result<JavaGpuStatus> {
    let supported = cfg!(target_os = "windows");

    Ok(JavaGpuStatus {
        supported,
        unsupported_reason: (!supported).then(|| {
            if cfg!(target_os = "macos") {
                "macOS decides this itself; there is nothing to choose."
                    .to_string()
            } else {
                "Choosing a GPU per application is only supported on Windows. \
                 On Linux, set the driver's own environment variables in the \
                 instance's Java settings."
                    .to_string()
            }
        }),
        java_path: java_path.to_string_lossy().to_string(),
        preference: get_preference(java_path)?,
        adapters: list_adapters(),
    })
}

/// Copies a preference onto a runtime that has none.
///
/// Called after installing Java so a freshly downloaded runtime inherits what
/// the previous one was set to, instead of quietly reverting to the default
/// adapter at the next launch. Never overwrites a preference already there.
pub fn inherit_preference(
    new_java_path: &Path,
    preference: GpuPreference,
) -> crate::Result<()> {
    if !cfg!(target_os = "windows") || preference == GpuPreference::Auto {
        return Ok(());
    }

    if get_preference(new_java_path)? != GpuPreference::Auto {
        return Ok(());
    }

    set_preference(new_java_path, preference)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_values_windows_writes() {
        assert_eq!(
            GpuPreference::from_registry_value("GpuPreference=2;"),
            GpuPreference::HighPerformance
        );
        assert_eq!(
            GpuPreference::from_registry_value("GpuPreference=1;"),
            GpuPreference::PowerSaving
        );
        assert_eq!(
            GpuPreference::from_registry_value("GpuPreference=0;"),
            GpuPreference::Auto
        );
    }

    #[test]
    fn ignores_fields_it_does_not_own() {
        // Windows stores other things under the same value, and an entry can
        // carry them without a preference at all.
        assert_eq!(
            GpuPreference::from_registry_value("AppStatus=4096;"),
            GpuPreference::Auto
        );
        assert_eq!(
            GpuPreference::from_registry_value(
                "AppStatus=4096;GpuPreference=2;"
            ),
            GpuPreference::HighPerformance
        );
    }

    #[test]
    fn survives_junk() {
        assert_eq!(GpuPreference::from_registry_value(""), GpuPreference::Auto);
        assert_eq!(
            GpuPreference::from_registry_value("GpuPreference=;"),
            GpuPreference::Auto
        );
        assert_eq!(
            GpuPreference::from_registry_value("nonsense"),
            GpuPreference::Auto
        );
    }

    #[test]
    fn round_trips_through_the_registry_format() {
        for preference in
            [GpuPreference::PowerSaving, GpuPreference::HighPerformance]
        {
            let value = preference.to_registry_value().unwrap();
            assert_eq!(GpuPreference::from_registry_value(&value), preference);
        }
        assert!(GpuPreference::Auto.to_registry_value().is_none());
    }

    #[test]
    fn recognises_integrated_adapters() {
        // Taken from real hybrid laptops, including the reporter's own.
        assert!(looks_integrated("AMD Radeon(TM) Graphics"));
        assert!(looks_integrated("Intel(R) UHD Graphics 630"));
        assert!(looks_integrated("Intel(R) Iris(R) Xe Graphics"));
        assert!(!looks_integrated("NVIDIA GeForce RTX 4050 Laptop GPU"));
        assert!(!looks_integrated("AMD Radeon RX 7600"));
    }
}
