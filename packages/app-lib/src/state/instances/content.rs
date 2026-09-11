use super::ContentSourceKind;
use crate::state::{
    License, Project, ProjectType, Version, VersionEnvironment,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContentItem {
    pub file_name: String,
    pub file_path: String,
    pub id: String,
    pub size: u64,
    pub enabled: bool,
    pub locked: bool,
    pub project_type: ProjectType,
    pub project: Option<ContentItemProject>,
    pub version: Option<ContentItemVersion>,
    pub environment: Option<VersionEnvironment>,
    pub owner: Option<ContentItemOwner>,
    pub has_update: bool,
    pub update_version_id: Option<String>,
    pub date_added: Option<String>,
    pub source_kind: Option<ContentSourceKind>,
    pub embedded_metadata: Option<EmbeddedContentMetadata>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub synced_pack: Option<SyncedPackInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SyncedPackInfo {
    pub id: String,
    pub instance_ids: Vec<String>,
    pub update_pending: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct EmbeddedContentMetadata {
    pub name: Option<String>,
    pub version: Option<String>,
    pub icon_path: Option<String>,
}

impl EmbeddedContentMetadata {
    pub fn is_empty(&self) -> bool {
        self.name.is_none()
            && self.version.is_none()
            && self.icon_path.is_none()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContentItemProject {
    pub id: String,
    pub slug: Option<String>,
    pub title: String,
    pub icon_url: Option<String>,
    pub license: License,
    pub categories: Vec<String>,
    pub additional_categories: Vec<String>,
    /// Noctrinth's own: where this project lives when it does not live on
    /// Modrinth. The frontend routes a project by its id, which only works for
    /// a Modrinth id — a CurseForge project has to be opened on the web
    /// instead, so it says so here rather than being linked into a page that
    /// cannot exist.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContentItemVersion {
    pub id: String,
    pub version_number: String,
    pub file_name: String,
    pub date_published: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContentItemOwner {
    pub id: String,
    pub name: String,
    pub avatar_url: Option<String>,
    #[serde(rename = "type")]
    pub owner_type: OwnerType,
    /// Noctrinth's own, for the same reason as on the project: an author who
    /// is not a Modrinth user has no Modrinth page to be sent to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OwnerType {
    User,
    Organization,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LinkedModpackInfo {
    pub project: Project,
    pub version: Option<Version>,
    pub owner: Option<ContentItemOwner>,
    pub has_update: bool,
    pub update_version_id: Option<String>,
    pub update_version: Option<Version>,
}
