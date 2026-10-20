//! Browsing and installing from CurseForge, in the shapes the Modrinth browse
//! page already draws.
//!
//! Everything goes through CurseForge's public API with the key the build was
//! configured with ([`curseforge_metadata::api_key`]). Search results come
//! back as Labrinth v3 search hits so the shared browse layout renders them
//! like any other; their ids carry a `cf-` prefix so a CurseForge project can
//! never be mistaken for a Modrinth one by anything that reads them later.
//!
//! A file whose author does not let launchers download it is never fetched
//! some other way: it is handed to [`curseforge_blocked`], which looks for it
//! on disk and otherwise leaves it for the user to download by hand.
//!
//! [`curseforge_metadata::api_key`]: crate::api::curseforge_metadata
//! [`curseforge_blocked`]: crate::api::curseforge_blocked

use std::collections::HashSet;
use std::path::PathBuf;

use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::api::curseforge_blocked::{self, BlockedFile, PendingFile};
use crate::state::{ContentSourceKind, ModLoader, State};
use crate::util::fetch::{fetch_advanced, fetch_mirrors};
use crate::util::io::sanitize_filename;

const API: &str = "https://api.curseforge.com/v1";
/// Minecraft's game id on CurseForge.
const MINECRAFT: i64 = 432;
/// The prefix a CurseForge project's id carries everywhere the frontend sees
/// it.
pub const ID_PREFIX: &str = "cf-";
/// CurseForge refuses a search whose `index + pageSize` exceeds this.
const MAX_SEARCH_WINDOW: u32 = 10_000;
const MAX_PAGE_SIZE: u32 = 50;
/// How far required dependencies are followed, so a cycle in CurseForge's
/// data cannot keep an install going forever.
const MAX_DEPENDENCY_DEPTH: usize = 8;

/// CurseForge's class ids, by the project type Modrinth calls them.
const CLASSES: [(&str, i64); 5] = [
    ("mod", 6),
    ("modpack", 4471),
    ("resourcepack", 12),
    ("shader", 6552),
    ("datapack", 6945),
];

/// CurseForge's mod loader ids, by the loader name Modrinth uses.
const LOADERS: [(&str, i64); 4] =
    [("forge", 1), ("fabric", 4), ("quilt", 5), ("neoforge", 6)];

fn class_id(project_type: &str) -> Option<i64> {
    CLASSES
        .iter()
        .find(|(name, _)| *name == project_type)
        .map(|(_, id)| *id)
}

fn project_type_of(class_id: Option<i64>) -> &'static str {
    CLASSES
        .iter()
        .find(|(_, id)| Some(*id) == class_id)
        .map_or("mod", |(name, _)| name)
}

fn loader_id(loader: &str) -> Option<i64> {
    LOADERS
        .iter()
        .find(|(name, _)| *name == loader)
        .map(|(_, id)| *id)
}

fn loader_name(id: i64) -> Option<&'static str> {
    LOADERS
        .iter()
        .find(|(_, loader)| *loader == id)
        .map(|(name, _)| *name)
}

/// The sort CurseForge calls by number for each one Modrinth calls by name.
/// Modrinth's "relevance" has no CurseForge equivalent; popularity is what
/// CurseForge's own site sorts by when nothing is chosen.
fn sort_field(sort: &str) -> i64 {
    match sort {
        "downloads" => 6,
        "newest" => 11,
        "updated" => 3,
        "follows" => 12,
        _ => 2,
    }
}

/// A CurseForge project id as the frontend sees it, back to CurseForge's own.
pub fn parse_project_id(id: &str) -> crate::Result<i64> {
    id.strip_prefix(ID_PREFIX)
        .unwrap_or(id)
        .parse()
        .map_err(|_| input(format!("Not a CurseForge project id: {id}")))
}

fn input(message: impl Into<String>) -> crate::Error {
    crate::ErrorKind::InputError(message.into()).into()
}

fn key() -> crate::Result<String> {
    crate::api::curseforge_metadata::api_key().ok_or_else(|| {
        input("This build has no CurseForge API key, so CurseForge is unavailable")
    })
}

async fn get<T: for<'de> Deserialize<'de>>(
    url: &str,
    state: &State,
) -> crate::Result<T> {
    let key = key()?;
    let body = fetch_advanced(
        Method::GET,
        url,
        None,
        None,
        Some(("x-api-key", &key)),
        None,
        None,
        None,
        &state.fetch_semaphore,
        &state.pool,
    )
    .await?;
    Ok(serde_json::from_slice(&body)?)
}

#[derive(Deserialize)]
struct Data<T> {
    data: T,
}

// ---------------------------------------------------------------------------
// CurseForge's payloads, reduced to what is used.

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ApiMod {
    id: i64,
    name: String,
    #[serde(default)]
    slug: Option<String>,
    #[serde(default)]
    summary: String,
    #[serde(default)]
    download_count: f64,
    #[serde(default)]
    thumbs_up_count: i64,
    #[serde(default)]
    class_id: Option<i64>,
    #[serde(default)]
    logo: Option<ApiImage>,
    #[serde(default)]
    screenshots: Vec<ApiImage>,
    #[serde(default)]
    authors: Vec<ApiAuthor>,
    #[serde(default)]
    categories: Vec<ApiCategory>,
    #[serde(default)]
    links: Option<ApiLinks>,
    #[serde(default)]
    date_created: Option<String>,
    #[serde(default)]
    date_modified: Option<String>,
    #[serde(default)]
    latest_files_indexes: Vec<ApiFileIndex>,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ApiImage {
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    thumbnail_url: Option<String>,
    #[serde(default)]
    url: Option<String>,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ApiAuthor {
    #[serde(default)]
    name: String,
    #[serde(default)]
    url: Option<String>,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ApiCategory {
    #[serde(default)]
    name: String,
    #[serde(default)]
    slug: String,
    #[serde(default)]
    icon_url: Option<String>,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ApiLinks {
    #[serde(default)]
    website_url: Option<String>,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ApiFileIndex {
    #[serde(default)]
    game_version: String,
    #[serde(default)]
    mod_loader: Option<i64>,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ApiFile {
    id: i64,
    mod_id: i64,
    #[serde(default)]
    display_name: String,
    #[serde(default)]
    file_name: String,
    /// 1 release, 2 beta, 3 alpha.
    #[serde(default)]
    release_type: i64,
    #[serde(default)]
    file_date: Option<String>,
    #[serde(default)]
    file_length: Option<u64>,
    #[serde(default)]
    download_count: f64,
    #[serde(default)]
    download_url: Option<String>,
    #[serde(default)]
    game_versions: Vec<String>,
    #[serde(default)]
    hashes: Vec<ApiHash>,
    #[serde(default)]
    dependencies: Vec<ApiDependency>,
}

#[derive(Deserialize, Clone)]
struct ApiHash {
    value: String,
    /// 1 SHA1, 2 MD5.
    algo: i64,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ApiDependency {
    mod_id: i64,
    /// 3 is a required dependency.
    relation_type: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiPagination {
    #[serde(default)]
    total_count: u32,
}

#[derive(Deserialize)]
struct ApiSearch {
    data: Vec<ApiMod>,
    pagination: ApiPagination,
}

impl ApiMod {
    fn icon_url(&self) -> Option<String> {
        self.logo
            .as_ref()
            .and_then(|logo| logo.thumbnail_url.clone().or(logo.url.clone()))
            .filter(|url| url.starts_with("https://"))
    }

    fn website_url(&self) -> Option<String> {
        self.links
            .as_ref()
            .and_then(|links| links.website_url.clone())
            .filter(|url| url.starts_with("https://"))
    }

    fn loaders(&self) -> Vec<String> {
        let mut loaders = self
            .latest_files_indexes
            .iter()
            .filter_map(|index| index.mod_loader.and_then(loader_name))
            .map(str::to_string)
            .collect::<Vec<_>>();
        loaders.sort();
        loaders.dedup();
        loaders
    }

    fn game_versions(&self) -> Vec<String> {
        let mut versions = self
            .latest_files_indexes
            .iter()
            .map(|index| index.game_version.clone())
            .filter(|version| !version.is_empty())
            .collect::<Vec<_>>();
        versions.sort();
        versions.dedup();
        versions
    }
}

impl ApiFile {
    fn sha1(&self) -> Option<&str> {
        self.hashes
            .iter()
            .find(|hash| hash.algo == 1)
            .map(|hash| hash.value.as_str())
    }

    fn as_blocked(&self) -> BlockedFile {
        BlockedFile {
            project_id: self.mod_id,
            file_id: self.id,
            file_name: self.file_name.clone(),
            size: self.file_length.filter(|length| *length > 0),
            sha1: self.sha1().map(str::to_string),
            source_kind: ContentSourceKind::CurseForge,
            disabled: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Search.

/// What the browse page is asking for, already taken apart into the pieces
/// CurseForge can filter on.
#[derive(Deserialize, Debug)]
pub struct SearchQuery {
    #[serde(default)]
    pub query: Option<String>,
    pub project_type: String,
    #[serde(default)]
    pub game_versions: Vec<String>,
    #[serde(default)]
    pub loaders: Vec<String>,
    #[serde(default)]
    pub sort: String,
    #[serde(default)]
    pub offset: u32,
    #[serde(default)]
    pub limit: u32,
}

/// A Labrinth v3 search hit, with the fields CurseForge can fill.
#[derive(Serialize, Debug)]
pub struct SearchHit {
    pub project_id: String,
    pub project_types: Vec<String>,
    pub all_project_types: Vec<String>,
    pub slug: Option<String>,
    pub author: String,
    pub author_id: Option<String>,
    pub organization: Option<String>,
    pub organization_id: Option<String>,
    pub name: String,
    pub summary: String,
    pub categories: Vec<String>,
    pub display_categories: Vec<String>,
    pub downloads: u64,
    pub follows: i64,
    pub icon_url: Option<String>,
    pub date_created: String,
    pub date_modified: String,
    pub license: String,
    pub gallery: Vec<String>,
    pub featured_gallery: Option<String>,
    pub color: Option<i64>,
    pub loaders: Vec<String>,
    pub disclosure_types: Vec<String>,
    /// Not part of Labrinth's shape; where the project lives on CurseForge.
    pub website_url: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct SearchResults {
    pub hits: Vec<SearchHit>,
    pub offset: u32,
    pub limit: u32,
    pub total_hits: u32,
    pub hits_per_page: u32,
}

fn search_url(query: &SearchQuery) -> crate::Result<String> {
    let class = class_id(&query.project_type).ok_or_else(|| {
        input(format!(
            "CurseForge has no {} projects to browse",
            query.project_type
        ))
    })?;
    let limit = query.limit.clamp(1, MAX_PAGE_SIZE);
    let offset = query.offset.min(MAX_SEARCH_WINDOW - limit);

    let mut url = url::Url::parse(&format!("{API}/mods/search"))
        .map_err(|error| input(error.to_string()))?;
    {
        let mut params = url.query_pairs_mut();
        params
            .append_pair("gameId", &MINECRAFT.to_string())
            .append_pair("classId", &class.to_string())
            .append_pair("sortField", &sort_field(&query.sort).to_string())
            .append_pair("sortOrder", "desc")
            .append_pair("index", &offset.to_string())
            .append_pair("pageSize", &limit.to_string());
        if let Some(text) = query.query.as_deref().map(str::trim)
            && !text.is_empty()
        {
            params.append_pair("searchFilter", text);
        }
        // CurseForge filters on one game version only.
        if let Some(version) = query.game_versions.first() {
            params.append_pair("gameVersion", version);
        }
        // Loaders mean nothing to resource packs, shaders and data packs.
        if class == 6 || class == 4471 {
            let loaders = query
                .loaders
                .iter()
                .filter_map(|loader| loader_id(loader))
                .collect::<Vec<_>>();
            if !loaders.is_empty() {
                params.append_pair(
                    "modLoaderTypes",
                    &serde_json::to_string(&loaders)?,
                );
            }
        }
    }
    Ok(url.to_string())
}

pub async fn search(query: SearchQuery) -> crate::Result<SearchResults> {
    let state = State::get().await?;
    let url = search_url(&query)?;
    let response: ApiSearch = get(&url, &state).await?;
    let limit = query.limit.clamp(1, MAX_PAGE_SIZE);

    Ok(SearchResults {
        hits: response
            .data
            .into_iter()
            .map(|project| search_hit(project, &query.project_type))
            .collect(),
        offset: query.offset,
        limit,
        // Past this CurseForge refuses to page, so it is all there is.
        total_hits: response.pagination.total_count.min(MAX_SEARCH_WINDOW),
        hits_per_page: limit,
    })
}

fn search_hit(project: ApiMod, project_type: &str) -> SearchHit {
    let icon_url = project.icon_url();
    let website_url = project.website_url();
    let loaders = project.loaders();
    let gallery = project
        .screenshots
        .iter()
        .filter_map(|image| image.url.clone())
        .collect::<Vec<_>>();
    let categories = project
        .categories
        .iter()
        .map(|category| category.name.clone())
        .collect::<Vec<_>>();

    SearchHit {
        project_id: format!("{ID_PREFIX}{}", project.id),
        project_types: vec![project_type.to_string()],
        all_project_types: vec![project_type.to_string()],
        slug: project.slug,
        author: project
            .authors
            .first()
            .map(|author| author.name.clone())
            .unwrap_or_default(),
        author_id: None,
        organization: None,
        organization_id: None,
        name: project.name,
        summary: project.summary,
        display_categories: categories.clone(),
        categories,
        downloads: project.download_count.max(0.0) as u64,
        follows: project.thumbs_up_count,
        icon_url,
        date_created: project.date_created.unwrap_or_default(),
        date_modified: project.date_modified.unwrap_or_default(),
        license: String::new(),
        featured_gallery: gallery.first().cloned(),
        gallery,
        color: None,
        loaders,
        disclosure_types: Vec::new(),
        website_url,
    }
}

// ---------------------------------------------------------------------------
// A project's page.

#[derive(Serialize, Debug)]
pub struct ProjectImage {
    pub url: String,
    pub thumbnail_url: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct ProjectCategory {
    pub name: String,
    pub slug: String,
    pub icon_url: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct ProjectAuthor {
    pub name: String,
    pub url: Option<String>,
}

/// Everything the CurseForge project page draws above its tabs.
#[derive(Serialize, Debug)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub slug: Option<String>,
    pub summary: String,
    pub project_type: String,
    pub icon_url: Option<String>,
    pub website_url: Option<String>,
    pub downloads: u64,
    pub follows: i64,
    pub date_created: Option<String>,
    pub date_modified: Option<String>,
    pub authors: Vec<ProjectAuthor>,
    pub categories: Vec<ProjectCategory>,
    pub gallery: Vec<ProjectImage>,
    pub loaders: Vec<String>,
    pub game_versions: Vec<String>,
}

pub async fn get_project(id: &str) -> crate::Result<Project> {
    let state = State::get().await?;
    let id = parse_project_id(id)?;
    let project: Data<ApiMod> = get(&format!("{API}/mods/{id}"), &state).await?;
    let project = project.data;

    Ok(Project {
        id: format!("{ID_PREFIX}{}", project.id),
        icon_url: project.icon_url(),
        website_url: project.website_url(),
        loaders: project.loaders(),
        game_versions: project.game_versions(),
        project_type: project_type_of(project.class_id).to_string(),
        name: project.name,
        slug: project.slug,
        summary: project.summary,
        downloads: project.download_count.max(0.0) as u64,
        follows: project.thumbs_up_count,
        date_created: project.date_created,
        date_modified: project.date_modified,
        authors: project
            .authors
            .into_iter()
            .map(|author| ProjectAuthor {
                name: author.name,
                url: author.url.filter(|url| url.starts_with("https://")),
            })
            .collect(),
        categories: project
            .categories
            .into_iter()
            .map(|category| ProjectCategory {
                name: category.name,
                slug: category.slug,
                icon_url: category.icon_url,
            })
            .collect(),
        gallery: project
            .screenshots
            .into_iter()
            .filter_map(|image| {
                Some(ProjectImage {
                    url: image.url.filter(|url| url.starts_with("https://"))?,
                    thumbnail_url: image.thumbnail_url,
                    title: image.title,
                    description: image.description,
                })
            })
            .collect(),
    })
}

/// The project's description, as the HTML CurseForge stores it. The frontend
/// renders it through the same sanitiser as any other project body.
pub async fn get_description(id: &str) -> crate::Result<String> {
    let state = State::get().await?;
    let id = parse_project_id(id)?;
    let description: Data<String> =
        get(&format!("{API}/mods/{id}/description"), &state).await?;
    Ok(description.data)
}

/// One file of a project, as the versions list draws it.
#[derive(Serialize, Debug)]
pub struct ProjectFile {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub file_name: String,
    /// `release`, `beta` or `alpha`, as Modrinth names them.
    pub version_type: &'static str,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub date_published: Option<String>,
    pub downloads: u64,
    pub size: Option<u64>,
    /// False when the author does not let launchers download the file; it can
    /// still be installed, by being downloaded from CurseForge's site.
    pub launcher_download: bool,
}

impl From<ApiFile> for ProjectFile {
    fn from(file: ApiFile) -> Self {
        // CurseForge lists loaders among the game versions, by display name.
        let (loaders, game_versions): (Vec<_>, Vec<_>) = file
            .game_versions
            .iter()
            .cloned()
            .partition(|version| loader_id(&version.to_lowercase()).is_some());
        Self {
            id: file.id.to_string(),
            project_id: format!("{ID_PREFIX}{}", file.mod_id),
            name: if file.display_name.is_empty() {
                file.file_name.clone()
            } else {
                file.display_name.clone()
            },
            file_name: file.file_name.clone(),
            version_type: match file.release_type {
                2 => "beta",
                3 => "alpha",
                _ => "release",
            },
            game_versions: game_versions
                .into_iter()
                .filter(|version| version.chars().next().is_some_and(|c| c.is_ascii_digit()))
                .collect(),
            loaders: loaders.into_iter().map(|l| l.to_lowercase()).collect(),
            date_published: file.file_date.clone(),
            downloads: file.download_count.max(0.0) as u64,
            size: file.file_length,
            launcher_download: file.download_url.is_some(),
        }
    }
}

async fn list_files(
    project_id: i64,
    game_version: Option<&str>,
    loader: Option<i64>,
    state: &State,
) -> crate::Result<Vec<ApiFile>> {
    let mut url = url::Url::parse(&format!("{API}/mods/{project_id}/files"))
        .map_err(|error| input(error.to_string()))?;
    {
        let mut params = url.query_pairs_mut();
        params.append_pair("pageSize", &MAX_PAGE_SIZE.to_string());
        if let Some(version) = game_version {
            params.append_pair("gameVersion", version);
        }
        if let Some(loader) = loader {
            params.append_pair("modLoaderType", &loader.to_string());
        }
    }
    let files: Data<Vec<ApiFile>> = get(url.as_str(), state).await?;
    Ok(files.data)
}

pub async fn get_files(
    id: &str,
    game_version: Option<String>,
    loader: Option<String>,
) -> crate::Result<Vec<ProjectFile>> {
    let state = State::get().await?;
    let id = parse_project_id(id)?;
    let files = list_files(
        id,
        game_version.as_deref(),
        loader.as_deref().and_then(loader_id),
        &state,
    )
    .await?;
    Ok(files.into_iter().map(ProjectFile::from).collect())
}

// ---------------------------------------------------------------------------
// Installing into an instance.

#[derive(Serialize, Debug, Default)]
pub struct InstallOutcome {
    /// Every project that was installed, the asked-for one and its
    /// dependencies, with their `cf-` ids.
    pub installed: Vec<String>,
    /// Files that have to be downloaded by hand before they are installed.
    pub pending: Vec<PendingFile>,
}

/// The newest file that fits the instance, preferring a release over a beta
/// over an alpha.
fn pick_file(files: Vec<ApiFile>) -> Option<ApiFile> {
    let mut files = files;
    files.sort_by(|a, b| b.file_date.cmp(&a.file_date));
    files
        .iter()
        .find(|file| file.release_type == 1)
        .or_else(|| files.iter().find(|file| file.release_type == 2))
        .or_else(|| files.first())
        .cloned()
}

/// Install a CurseForge project into an instance, with its required
/// dependencies. `file_id` picks a file; without one, the newest file for the
/// instance's game version and loader is used.
pub async fn install_project(
    instance_id: &str,
    project_id: &str,
    file_id: Option<String>,
) -> crate::Result<InstallOutcome> {
    let state = State::get().await?;
    let instance = crate::api::instance::get(instance_id)
        .await?
        .ok_or_else(|| input(format!("No instance {instance_id}")))?;
    let content_set = instance.applied_content_set;
    let project_id = parse_project_id(project_id)?;
    let file_id = file_id
        .map(|id| id.parse::<i64>())
        .transpose()
        .map_err(|_| input("Not a CurseForge file id"))?;

    let mut installed_ids: HashSet<String> =
        crate::api::instance::get_installed_project_ids(instance_id)
            .await?
            .into_iter()
            .collect();
    let mut outcome = InstallOutcome::default();
    let mut queue = vec![(project_id, file_id, 0usize)];
    let mut visited = HashSet::new();

    while let Some((project_id, file_id, depth)) = queue.pop() {
        if !visited.insert(project_id) || depth > MAX_DEPENDENCY_DEPTH {
            continue;
        }
        // A dependency already in the instance is left as it is.
        if depth > 0 && installed_ids.contains(&project_id.to_string()) {
            continue;
        }

        let file = match file_id {
            Some(file_id) => {
                let file: Data<ApiFile> = get(
                    &format!("{API}/mods/{project_id}/files/{file_id}"),
                    &state,
                )
                .await?;
                file.data
            }
            None => {
                let project: Data<ApiMod> =
                    get(&format!("{API}/mods/{project_id}"), &state).await?;
                let loader = match project_type_of(project.data.class_id) {
                    "mod" => match content_set.loader {
                        ModLoader::Vanilla => None,
                        loader => loader_id(loader.as_str()),
                    },
                    _ => None,
                };
                let files = list_files(
                    project_id,
                    Some(&content_set.game_version),
                    loader,
                    &state,
                )
                .await?;
                pick_file(files).ok_or_else(|| {
                    input(format!(
                        "{} has no file for Minecraft {}{}",
                        project.data.name,
                        content_set.game_version,
                        loader
                            .and_then(loader_name)
                            .map(|loader| format!(" with {loader}"))
                            .unwrap_or_default()
                    ))
                })?
            }
        };

        for dependency in &file.dependencies {
            if dependency.relation_type == 3 {
                queue.push((dependency.mod_id, None, depth + 1));
            }
        }

        match &file.download_url {
            Some(url) => {
                install_file(instance_id, &file, url, &state).await?;
                installed_ids.insert(file.mod_id.to_string());
                outcome.installed.push(format!("{ID_PREFIX}{}", file.mod_id));
            }
            None => {
                let pending = curseforge_blocked::settle(
                    instance_id,
                    vec![file.as_blocked()],
                    &state,
                )
                .await;
                if pending.is_empty() {
                    installed_ids.insert(file.mod_id.to_string());
                    outcome
                        .installed
                        .push(format!("{ID_PREFIX}{}", file.mod_id));
                }
                outcome.pending.extend(pending);
            }
        }
    }

    outcome.pending =
        curseforge_blocked::describe(outcome.pending, &state).await;
    Ok(outcome)
}

async fn install_file(
    instance_id: &str,
    file: &ApiFile,
    url: &str,
    state: &State,
) -> crate::Result<()> {
    let bytes = fetch_mirrors(
        &[url],
        file.sha1(),
        None,
        None,
        &state.fetch_semaphore,
        &state.pool,
    )
    .await?;
    let project_id = file.mod_id.to_string();
    let file_id = file.id.to_string();
    let _permit = state.install_db_semaphore.acquire().await?;
    crate::state::instances::commands::add_project_bytes(
        instance_id,
        &sanitize_filename(&file.file_name),
        bytes,
        file.sha1(),
        None,
        ContentSourceKind::CurseForge,
        Some(&project_id),
        Some(&file_id),
        state,
    )
    .await?;
    Ok(())
}

/// Download a modpack's zip, for the regular CurseForge pack installer to
/// install from. Returns where it was saved.
pub async fn download_modpack(
    project_id: &str,
    file_id: Option<String>,
) -> crate::Result<PathBuf> {
    let state = State::get().await?;
    let project_id = parse_project_id(project_id)?;
    let file = match file_id {
        Some(file_id) => {
            let file: Data<ApiFile> = get(
                &format!("{API}/mods/{project_id}/files/{file_id}"),
                &state,
            )
            .await?;
            file.data
        }
        None => pick_file(list_files(project_id, None, None, &state).await?)
            .ok_or_else(|| input("This modpack has no files"))?,
    };
    let Some(url) = file.download_url.as_deref() else {
        return Err(input(format!(
            "The author of {} does not allow launchers to download it; \
             download the zip from CurseForge and drop it on the launcher",
            file.file_name
        )));
    };

    let bytes = fetch_mirrors(
        &[url],
        file.sha1(),
        None,
        None,
        &state.fetch_semaphore,
        &state.pool,
    )
    .await?;
    let folder = state.directories.caches_dir().join("curseforge-packs");
    tokio::fs::create_dir_all(&folder)
        .await
        .map_err(|e| crate::util::io::IOError::with_path(e, &folder))?;
    let path = folder.join(format!("{}.zip", file.id));
    tokio::fs::write(&path, &bytes)
        .await
        .map_err(|e| crate::util::io::IOError::with_path(e, &path))?;
    Ok(path)
}

/// What an instance is still waiting on, described for the frontend.
pub async fn pending_files(
    instance_id: &str,
) -> crate::Result<Vec<PendingFile>> {
    let state = State::get().await?;
    let files = curseforge_blocked::list(instance_id, &state).await?;
    Ok(curseforge_blocked::describe(files, &state).await)
}

/// Install whatever of the pending files has arrived in Downloads.
pub async fn check_pending_downloads(
    instance_id: &str,
) -> crate::Result<Vec<PendingFile>> {
    let state = State::get().await?;
    let files = curseforge_blocked::check_downloads(instance_id, &state).await?;
    Ok(curseforge_blocked::describe(files, &state).await)
}

pub async fn dismiss_pending_file(
    instance_id: &str,
    file_id: &str,
) -> crate::Result<()> {
    let state = State::get().await?;
    curseforge_blocked::dismiss(instance_id, file_id, &state).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn query(project_type: &str) -> SearchQuery {
        SearchQuery {
            query: Some(" jei ".to_string()),
            project_type: project_type.to_string(),
            game_versions: vec!["1.20.1".to_string(), "1.20".to_string()],
            loaders: vec!["fabric".to_string(), "iris".to_string()],
            sort: "downloads".to_string(),
            offset: 40,
            limit: 20,
        }
    }

    fn params(url: &str) -> Vec<(String, String)> {
        url::Url::parse(url)
            .unwrap()
            .query_pairs()
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect()
    }

    #[test]
    fn a_mod_search_carries_every_filter_curseforge_has() {
        let params = params(&search_url(&query("mod")).unwrap());
        let get = |key: &str| {
            params
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v.as_str())
        };

        assert_eq!(get("gameId"), Some("432"));
        assert_eq!(get("classId"), Some("6"));
        assert_eq!(get("searchFilter"), Some("jei"));
        assert_eq!(get("gameVersion"), Some("1.20.1"));
        // Only loaders CurseForge knows are sent.
        assert_eq!(get("modLoaderTypes"), Some("[4]"));
        assert_eq!(get("sortField"), Some("6"));
        assert_eq!(get("index"), Some("40"));
        assert_eq!(get("pageSize"), Some("20"));
    }

    #[test]
    fn a_resource_pack_search_ignores_loaders() {
        let url = search_url(&query("resourcepack")).unwrap();
        assert!(!url.contains("modLoaderTypes"));
        assert!(url.contains("classId=12"));
    }

    #[test]
    fn a_search_never_pages_past_what_curseforge_allows() {
        let mut q = query("mod");
        q.offset = 20_000;
        q.limit = 500;
        let params = params(&search_url(&q).unwrap());
        assert!(params.contains(&("pageSize".to_string(), "50".to_string())));
        assert!(params.contains(&("index".to_string(), "9950".to_string())));
    }

    #[test]
    fn a_project_type_curseforge_lacks_is_refused() {
        assert!(search_url(&query("plugin")).is_err());
    }

    #[test]
    fn ids_round_trip_through_the_prefix() {
        assert_eq!(parse_project_id("cf-238222").unwrap(), 238222);
        assert_eq!(parse_project_id("238222").unwrap(), 238222);
        assert!(parse_project_id("AANobbMI").is_err());
    }

    #[test]
    fn the_newest_release_wins_over_a_newer_beta() {
        let file = |id, release_type, date: &str| ApiFile {
            id,
            mod_id: 1,
            display_name: String::new(),
            file_name: format!("{id}.jar"),
            release_type,
            file_date: Some(date.to_string()),
            file_length: None,
            download_count: 0.0,
            download_url: None,
            game_versions: Vec::new(),
            hashes: Vec::new(),
            dependencies: Vec::new(),
        };
        let picked = pick_file(vec![
            file(1, 1, "2024-01-01"),
            file(2, 2, "2024-03-01"),
            file(3, 1, "2024-02-01"),
        ])
        .unwrap();
        assert_eq!(picked.id, 3);
    }

    #[test]
    fn a_file_splits_loaders_out_of_its_game_versions() {
        let file: ApiFile = serde_json::from_str(
            r#"{
                "id": 5, "modId": 7, "fileName": "x.jar", "releaseType": 2,
                "gameVersions": ["1.20.1", "Fabric", "NeoForge", "Client"],
                "downloadUrl": null
            }"#,
        )
        .unwrap();
        let file = ProjectFile::from(file);
        assert_eq!(file.game_versions, vec!["1.20.1"]);
        assert_eq!(file.loaders, vec!["fabric", "neoforge"]);
        assert_eq!(file.version_type, "beta");
        assert!(!file.launcher_download);
        assert_eq!(file.project_id, "cf-7");
    }
}
