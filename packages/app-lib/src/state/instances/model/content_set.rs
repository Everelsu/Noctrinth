use crate::state::ModLoader;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::unknown_value;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentSourceKind {
    Local,
    ModrinthModpack,
    ServerProject,
    ModrinthHosting,
    ImportedModpack,
    SharedInstance,
    /// A file installed from CurseForge on its own — the user added this one.
    /// The owning content entry stores the CurseForge numeric project id and
    /// file id in its `project_id` / `version_id` columns (as strings), since
    /// CurseForge content has no Modrinth hash mapping.
    CurseForge,
    /// A file a CurseForge modpack's own manifest listed, stored the same way.
    ///
    /// Separate from `CurseForge` because the two answer different questions.
    /// Both say the ids are CurseForge's rather than Modrinth's, which is what
    /// keeps them out of the Modrinth hash cache; only this one says the file
    /// is part of the pack the instance was installed from, which is what
    /// decides whether it is listed as the pack's content or as something the
    /// user added next to it. Stamping the pack's files `CurseForge` left the
    /// pack's content list empty and filed all several hundred mods under
    /// "Additional content".
    CurseForgeModpack,
}

impl ContentSourceKind {
    pub fn is_shared_instance_managed(self) -> bool {
        matches!(
            self,
            Self::SharedInstance
                | Self::ModrinthModpack
                | Self::ImportedModpack
                | Self::CurseForgeModpack
        )
    }

    /// Noctrinth's own: whether the ids this entry carries are CurseForge's
    /// numeric ones rather than Modrinth's, which is what decides whether they
    /// can go into the Modrinth hash cache.
    pub fn has_curseforge_ids(self) -> bool {
        matches!(self, Self::CurseForge | Self::CurseForgeModpack)
    }

    /// Noctrinth's own: whether an entry of this kind belongs to a pack that
    /// is being listed as `filter`.
    ///
    /// A pack installed from a CurseForge zip is linked as an imported modpack
    /// like any other, but its files are stamped with where they really came
    /// from, so the plain equality upstream uses would never match them.
    pub fn counts_as(self, filter: Self) -> bool {
        self == filter
            || (filter == Self::ImportedModpack
                && self == Self::CurseForgeModpack)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::ModrinthModpack => "modrinth_modpack",
            Self::ServerProject => "server_project",
            Self::ModrinthHosting => "modrinth_hosting",
            Self::ImportedModpack => "imported_modpack",
            Self::SharedInstance => "shared_instance",
            Self::CurseForge => "curseforge",
            Self::CurseForgeModpack => "curseforge_modpack",
        }
    }

    pub fn from_str(value: &str) -> crate::Result<Self> {
        match value {
            "local" => Ok(Self::Local),
            "modrinth_modpack" => Ok(Self::ModrinthModpack),
            "server_project" => Ok(Self::ServerProject),
            "modrinth_hosting" => Ok(Self::ModrinthHosting),
            "imported_modpack" => Ok(Self::ImportedModpack),
            "shared_instance" => Ok(Self::SharedInstance),
            "curseforge" => Ok(Self::CurseForge),
            "curseforge_modpack" => Ok(Self::CurseForgeModpack),
            other => Err(unknown_value("content source kind", other)),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentSetStatus {
    Available,
    Installing,
    Stale,
    MissingFiles,
}

impl ContentSetStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Installing => "installing",
            Self::Stale => "stale",
            Self::MissingFiles => "missing_files",
        }
    }

    pub fn from_str(value: &str) -> crate::Result<Self> {
        match value {
            "available" => Ok(Self::Available),
            "installing" => Ok(Self::Installing),
            "stale" => Ok(Self::Stale),
            "missing_files" => Ok(Self::MissingFiles),
            other => Err(unknown_value("content set status", other)),
        }
    }
}

/// Represents a playable setup slot for an instance.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContentSet {
    pub id: String,
    pub instance_id: String,
    pub name: String,
    pub source_kind: ContentSourceKind,
    pub status: ContentSetStatus,
    pub game_version: String,
    pub protocol_version: Option<u32>,
    pub loader: ModLoader,
    pub loader_version: Option<String>,
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::ContentSourceKind;

    #[test]
    fn a_curseforge_pack_s_files_are_the_pack_s_content() {
        assert!(
            ContentSourceKind::CurseForgeModpack
                .counts_as(ContentSourceKind::ImportedModpack)
        );
    }

    #[test]
    fn a_curseforge_mod_added_by_hand_is_not() {
        assert!(
            !ContentSourceKind::CurseForge
                .counts_as(ContentSourceKind::ImportedModpack)
        );
        assert!(
            !ContentSourceKind::Local
                .counts_as(ContentSourceKind::ImportedModpack)
        );
    }

    #[test]
    fn nothing_else_is_widened() {
        // Only the imported-modpack listing takes the wider reading; asking
        // for shared-instance or server content still means exactly that.
        assert!(
            !ContentSourceKind::CurseForgeModpack
                .counts_as(ContentSourceKind::SharedInstance)
        );
        assert!(
            ContentSourceKind::ServerProject
                .counts_as(ContentSourceKind::ServerProject)
        );
    }

    #[test]
    fn both_curseforge_kinds_keep_their_ids_out_of_the_modrinth_cache() {
        assert!(ContentSourceKind::CurseForge.has_curseforge_ids());
        assert!(ContentSourceKind::CurseForgeModpack.has_curseforge_ids());
        assert!(!ContentSourceKind::ImportedModpack.has_curseforge_ids());
    }

    #[test]
    fn the_new_kind_survives_a_round_trip_through_the_database() {
        let kind = ContentSourceKind::CurseForgeModpack;
        assert_eq!(kind.as_str(), "curseforge_modpack");
        assert_eq!(ContentSourceKind::from_str(kind.as_str()).unwrap(), kind);
    }
}
