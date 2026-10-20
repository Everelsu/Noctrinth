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

use std::collections::{HashMap, HashSet};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

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

/// How long a CurseForge answer is reused. Long enough that going back to
/// the results, opening a project twice or paging back costs nothing; short
/// enough that a new upload shows up within minutes.
const CACHE_TTL: Duration = Duration::from_secs(5 * 60);
/// Past this many answers the cache is emptied rather than tracked further.
const CACHE_MAX_ENTRIES: usize = 512;

static CACHE: LazyLock<Mutex<HashMap<String, (Instant, bytes::Bytes)>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn cached(url: &str) -> Option<bytes::Bytes> {
    let cache = CACHE.lock().ok()?;
    let (at, body) = cache.get(url)?;
    (at.elapsed() < CACHE_TTL).then(|| body.clone())
}

fn remember(url: &str, body: &bytes::Bytes) {
    if let Ok(mut cache) = CACHE.lock() {
        if cache.len() >= CACHE_MAX_ENTRIES {
            // ponytail: drops everything at once; an LRU if this ever thrashes.
            cache.clear();
        }
        cache.insert(url.to_string(), (Instant::now(), body.clone()));
    }
}

/// A GET against CurseForge's API, answered from the cache when it can be.
///
/// Goes through the API semaphore rather than the download one, so browsing
/// stays quick while a modpack is downloading hundreds of files.
async fn get<T: for<'de> Deserialize<'de>>(
    url: &str,
    state: &State,
) -> crate::Result<T> {
    request(Method::GET, url, None, state).await
}

/// A POST of `body`, cached like a GET: CurseForge's batch lookups only read.
async fn post<T: for<'de> Deserialize<'de>>(
    url: &str,
    body: serde_json::Value,
    state: &State,
) -> crate::Result<T> {
    request(Method::POST, url, Some(body), state).await
}

async fn request<T: for<'de> Deserialize<'de>>(
    method: Method,
    url: &str,
    body: Option<serde_json::Value>,
    state: &State,
) -> crate::Result<T> {
    let cache_key = match &body {
        Some(body) => format!("{url} {body}"),
        None => url.to_string(),
    };
    if let Some(cached) = cached(&cache_key) {
        return Ok(serde_json::from_slice(&cached)?);
    }
    let key = key()?;
    let response = fetch_advanced(
        method,
        url,
        None,
        body,
        Some(("x-api-key", &key)),
        None,
        None,
        None,
        &state.api_semaphore,
        &state.pool,
    )
    .await?;
    let parsed = serde_json::from_slice(&response)?;
    remember(&cache_key, &response);
    Ok(parsed)
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
    id: Option<i64>,
    #[serde(default)]
    name: String,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    avatar_url: Option<String>,
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
    #[serde(default)]
    wiki_url: Option<String>,
    #[serde(default)]
    issues_url: Option<String>,
    #[serde(default)]
    source_url: Option<String>,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ApiFileIndex {
    #[serde(default)]
    game_version: String,
    #[serde(default)]
    mod_loader: Option<i64>,
    #[serde(default)]
    file_id: i64,
    /// 1 release, 2 beta, 3 alpha.
    #[serde(default)]
    release_type: i64,
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
    /// A modpack's server files, listed alongside its client files.
    #[serde(default)]
    is_server_pack: Option<bool>,
    /// False for a file CurseForge has pulled or not yet approved.
    #[serde(default)]
    is_available: Option<bool>,
}

impl ApiFile {
    /// Whether this is a file a launcher should ever offer: CurseForge's own
    /// app never installs a server pack or an unavailable file into a client.
    fn is_installable(&self) -> bool {
        self.is_server_pack != Some(true) && self.is_available != Some(false)
    }

    /// Whether the file names no loader at all among its game versions, as
    /// files from before CurseForge tagged loaders do.
    fn has_no_loader_tag(&self) -> bool {
        !self
            .game_versions
            .iter()
            .any(|version| loader_id(&version.to_lowercase()).is_some())
    }
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
    /// CurseForge category ids, as listed by [`get_categories`].
    #[serde(default)]
    pub category_ids: Vec<i64>,
    #[serde(default)]
    pub sort: String,
    #[serde(default)]
    pub offset: u32,
    #[serde(default)]
    pub limit: u32,
}

/// CurseForge filters on at most this many game versions, loaders and
/// categories at once.
const MAX_GAME_VERSIONS: usize = 4;
const MAX_CATEGORIES: usize = 10;

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
    let text = query
        .query
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty());
    {
        let mut params = url.query_pairs_mut();
        params
            .append_pair("gameId", &MINECRAFT.to_string())
            .append_pair("classId", &class.to_string())
            .append_pair("index", &offset.to_string())
            .append_pair("pageSize", &limit.to_string());
        // CurseForge's public search has no relevance order; left without a
        // sort it returns matches in an order that reads as random, so
        // "relevance" is popularity among the matches.
        params
            .append_pair("sortField", &sort_field(&query.sort).to_string())
            .append_pair("sortOrder", "desc");
        if let Some(text) = text {
            params.append_pair("searchFilter", text);
        }
        let versions = query
            .game_versions
            .iter()
            .take(MAX_GAME_VERSIONS)
            .collect::<Vec<_>>();
        if !versions.is_empty() {
            params.append_pair(
                "gameVersions",
                &serde_json::to_string(&versions)?,
            );
        }
        let categories = query
            .category_ids
            .iter()
            .take(MAX_CATEGORIES)
            .collect::<Vec<_>>();
        if !categories.is_empty() {
            params.append_pair(
                "categoryIds",
                &serde_json::to_string(&categories)?,
            );
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

/// A CurseForge category, to filter a search by.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    pub id: i64,
    pub name: String,
    #[serde(default)]
    pub slug: String,
    #[serde(default)]
    pub icon_url: Option<String>,
    #[serde(default)]
    pub parent_category_id: Option<i64>,
    #[serde(default, skip_serializing)]
    is_class: Option<bool>,
}

/// The categories CurseForge files projects of this type under, as its own
/// app lists them in its filters.
pub async fn get_categories(
    project_type: &str,
) -> crate::Result<Vec<Category>> {
    let state = State::get().await?;
    let class = class_id(project_type).ok_or_else(|| {
        input(format!("CurseForge has no {project_type} projects"))
    })?;
    let categories: Data<Vec<Category>> = get(
        &format!("{API}/categories?gameId={MINECRAFT}&classId={class}"),
        &state,
    )
    .await?;
    // Only the top level, as CurseForge's own app lists them: below it are
    // hundreds of per-mod subcategories ("Applied Energistics 2", ...).
    let mut categories = categories
        .data
        .into_iter()
        .filter(|category| {
            category.is_class != Some(true)
                && category.parent_category_id == Some(class)
        })
        .collect::<Vec<_>>();
    categories.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(categories)
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
    pub id: Option<String>,
    pub name: String,
    pub url: Option<String>,
    pub avatar_url: Option<String>,
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
    pub wiki_url: Option<String>,
    pub issues_url: Option<String>,
    pub source_url: Option<String>,
    pub authors: Vec<ProjectAuthor>,
    pub categories: Vec<ProjectCategory>,
    pub gallery: Vec<ProjectImage>,
    pub loaders: Vec<String>,
    pub game_versions: Vec<String>,
}

/// A URL from CurseForge's data, kept only when it is one to follow.
fn https_only(url: Option<String>) -> Option<String> {
    url.filter(|url| url.starts_with("https://"))
}

pub async fn get_project(id: &str) -> crate::Result<Project> {
    let state = State::get().await?;
    let id = parse_project_id(id)?;
    let project: Data<ApiMod> =
        get(&format!("{API}/mods/{id}"), &state).await?;
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
        wiki_url: https_only(
            project.links.as_ref().and_then(|l| l.wiki_url.clone()),
        ),
        issues_url: https_only(
            project.links.as_ref().and_then(|l| l.issues_url.clone()),
        ),
        source_url: https_only(
            project.links.as_ref().and_then(|l| l.source_url.clone()),
        ),
        authors: project
            .authors
            .into_iter()
            .map(|author| ProjectAuthor {
                id: author.id.map(|id| id.to_string()),
                name: author.name,
                url: https_only(author.url),
                avatar_url: https_only(author.avatar_url),
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
                // CurseForge often repeats the title as the description,
                // and the file name as both; the repeat is not drawn twice.
                let description = image.description.filter(|d| {
                    !d.trim().is_empty() && Some(d) != image.title.as_ref()
                });
                Some(ProjectImage {
                    url: image.url.filter(|url| url.starts_with("https://"))?,
                    thumbnail_url: image.thumbnail_url,
                    title: image.title,
                    description,
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
        let (loaders, game_versions): (Vec<_>, Vec<_>) =
            file.game_versions.iter().cloned().partition(|version| {
                loader_id(&version.to_lowercase()).is_some()
            });
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
                .filter(|version| {
                    version.chars().next().is_some_and(|c| c.is_ascii_digit())
                })
                .collect(),
            loaders: loaders.into_iter().map(|l| l.to_lowercase()).collect(),
            date_published: file.file_date.clone(),
            downloads: file.download_count.max(0.0) as u64,
            size: file.file_length,
            launcher_download: file.download_url.is_some(),
        }
    }
}

#[derive(Deserialize)]
struct FilesPage {
    data: Vec<ApiFile>,
    #[serde(default)]
    pagination: Option<ApiPagination>,
}

fn files_url(
    project_id: i64,
    game_version: Option<&str>,
    loader: Option<i64>,
    index: u32,
) -> crate::Result<String> {
    let mut url = url::Url::parse(&format!("{API}/mods/{project_id}/files"))
        .map_err(|error| input(error.to_string()))?;
    {
        let mut params = url.query_pairs_mut();
        params
            .append_pair("pageSize", &MAX_PAGE_SIZE.to_string())
            .append_pair("index", &index.to_string());
        if let Some(version) = game_version {
            params.append_pair("gameVersion", version);
        }
        if let Some(loader) = loader {
            params.append_pair("modLoaderType", &loader.to_string());
        }
    }
    Ok(url.to_string())
}

/// The newest page of a project's files, which is what picking one for an
/// instance needs.
async fn list_files(
    project_id: i64,
    game_version: Option<&str>,
    loader: Option<i64>,
    state: &State,
) -> crate::Result<Vec<ApiFile>> {
    let page: FilesPage =
        get(&files_url(project_id, game_version, loader, 0)?, state).await?;
    Ok(page.data)
}

/// The most files the versions tab lists; a project with more shows its
/// newest this many.
const MAX_LISTED_FILES: u32 = 250;

/// Every file of a project, page by page, as the versions tab lists them.
async fn list_all_files(
    project_id: i64,
    game_version: Option<&str>,
    loader: Option<i64>,
    state: &State,
) -> crate::Result<Vec<ApiFile>> {
    let first: FilesPage =
        get(&files_url(project_id, game_version, loader, 0)?, state).await?;
    let total = first
        .pagination
        .map_or(0, |p| p.total_count)
        .min(MAX_LISTED_FILES);
    let mut files = first.data;
    // ponytail: the remaining pages are fetched together, bounded by the API
    // semaphore; a project at the cap is ~20 requests, cached afterwards.
    let rest = futures::future::try_join_all(
        (MAX_PAGE_SIZE..total).step_by(MAX_PAGE_SIZE as usize).map(
            |index| async move {
                let url = files_url(project_id, game_version, loader, index)?;
                let page: FilesPage = get(&url, state).await?;
                Ok::<_, crate::Error>(page.data)
            },
        ),
    )
    .await?;
    files.extend(rest.into_iter().flatten());
    Ok(files)
}

pub async fn get_files(
    id: &str,
    game_version: Option<String>,
    loader: Option<String>,
) -> crate::Result<Vec<ProjectFile>> {
    let state = State::get().await?;
    let id = parse_project_id(id)?;
    let files = list_all_files(
        id,
        game_version.as_deref(),
        loader.as_deref().and_then(loader_id),
        &state,
    )
    .await?;
    Ok(files
        .into_iter()
        .filter(ApiFile::is_installable)
        .map(ProjectFile::from)
        .collect())
}

/// A project another file needs, or cannot run beside, as a version's page
/// lists it.
#[derive(Serialize, Debug)]
pub struct VersionDependency {
    pub project_id: String,
    /// `required`, `optional`, `incompatible` or `embedded`, as Modrinth
    /// names them.
    pub dependency_type: &'static str,
    pub name: Option<String>,
    pub slug: Option<String>,
    pub icon_url: Option<String>,
    pub project_type: &'static str,
}

/// One file of a project with what its own page shows besides the list's
/// columns.
#[derive(Serialize, Debug)]
pub struct ProjectVersion {
    #[serde(flatten)]
    pub file: ProjectFile,
    /// HTML, as CurseForge stores it.
    pub changelog: String,
    pub dependencies: Vec<VersionDependency>,
}

/// CurseForge's relation type as Modrinth names it; tools are not listed.
fn dependency_type(relation_type: i64) -> Option<&'static str> {
    match relation_type {
        1 | 6 => Some("embedded"),
        2 => Some("optional"),
        RELATION_REQUIRED => Some("required"),
        RELATION_INCOMPATIBLE => Some("incompatible"),
        _ => None,
    }
}

pub async fn get_version(
    id: &str,
    file_id: &str,
) -> crate::Result<ProjectVersion> {
    let state = State::get().await?;
    let id = parse_project_id(id)?;
    let file_id: i64 = file_id
        .parse()
        .map_err(|_| input(format!("Not a CurseForge file id: {file_id}")))?;
    let file_url = format!("{API}/mods/{id}/files/{file_id}");
    let changelog_url = format!("{file_url}/changelog");
    let (file, changelog) = futures::try_join!(
        get::<Data<ApiFile>>(&file_url, &state),
        get::<Data<String>>(&changelog_url, &state),
    )?;
    let file = file.data;

    // A dependency whose project is gone is still listed, by its id.
    let dependencies = futures::future::join_all(
        file.dependencies
            .iter()
            .filter_map(|d| Some((d.mod_id, dependency_type(d.relation_type)?)))
            .map(|(mod_id, dependency_type)| {
                let state = &state;
                async move {
                    let project = get::<Data<ApiMod>>(
                        &format!("{API}/mods/{mod_id}"),
                        state,
                    )
                    .await
                    .ok()
                    .map(|project| project.data);
                    VersionDependency {
                        project_id: format!("{ID_PREFIX}{mod_id}"),
                        dependency_type,
                        icon_url: project.as_ref().and_then(ApiMod::icon_url),
                        project_type: project_type_of(
                            project.as_ref().and_then(|p| p.class_id),
                        ),
                        slug: project.as_ref().and_then(|p| p.slug.clone()),
                        name: project.map(|p| p.name),
                    }
                }
            }),
    )
    .await;

    Ok(ProjectVersion {
        file: file.into(),
        changelog: changelog.data,
        dependencies,
    })
}

/// The game version and loader a CurseForge modpack's file says it runs on,
/// for the instance drawn while the pack downloads; the pack's own manifest
/// has the last word once it is in.
pub async fn modpack_target(
    project_id: &str,
    file_id: Option<&str>,
) -> crate::Result<(Option<String>, Option<ModLoader>)> {
    let state = State::get().await?;
    let id = parse_project_id(project_id)?;
    let file = match file_id {
        Some(file_id) => Some(
            get::<Data<ApiFile>>(
                &format!("{API}/mods/{id}/files/{file_id}"),
                &state,
            )
            .await?
            .data,
        ),
        None => pick_file(list_files(id, None, None, &state).await?),
    };
    let Some(file) = file.map(ProjectFile::from) else {
        return Ok((None, None));
    };
    Ok((
        file.game_versions.into_iter().next(),
        file.loaders
            .first()
            .map(|loader| ModLoader::from_string(loader)),
    ))
}

/// The most projects one batch lookup asks CurseForge about.
const MAX_IDS_PER_REQUEST: usize = 500;

/// Of the given installed CurseForge files, as (project id, file id) without
/// the `cf-` prefix, the ones with a newer file for this game version and
/// loader, mapped to that file's id.
///
/// CurseForge's own app checks the same way: one lookup of every project, and
/// the newest file each lists for the instance's version and loader. A
/// release is preferred; a beta or alpha only where the version has no
/// release at all. CurseForge's file ids only grow, so a larger one is newer.
pub async fn available_updates(
    installed: &[(String, String)],
    game_version: &str,
    loader: &str,
) -> crate::Result<HashMap<String, String>> {
    let installed = installed
        .iter()
        .filter_map(|(project, file)| {
            Some((project.parse::<i64>().ok()?, file.parse::<i64>().ok()?))
        })
        .collect::<HashMap<_, _>>();
    if installed.is_empty() {
        return Ok(HashMap::new());
    }
    let options = crate::api::curseforge_metadata::options();
    if !options.check_updates {
        return Ok(HashMap::new());
    }
    let state = State::get().await?;
    let wanted_loader = loader_id(loader);
    let mut ids = installed.keys().copied().collect::<Vec<_>>();
    ids.sort_unstable();

    let mut updates = HashMap::new();
    for chunk in ids.chunks(MAX_IDS_PER_REQUEST) {
        let mods: Data<Vec<ApiMod>> = post(
            &format!("{API}/mods"),
            serde_json::json!({ "modIds": chunk }),
            &state,
        )
        .await?;
        for project in mods.data {
            if let Some(newest) = newest_file(
                &project.latest_files_indexes,
                game_version,
                wanted_loader,
                options.update_release_type,
            ) && installed
                .get(&project.id)
                .is_some_and(|&file| newest > file)
            {
                updates.insert(project.id.to_string(), newest.to_string());
            }
        }
    }
    Ok(updates)
}

/// The newest file a project lists for this game version and loader that is
/// at least as stable as `release_type` (1 release, 2 beta, 3 alpha). A file
/// naming no loader (a pack, or a mod from before loaders were tagged) fits
/// any.
fn newest_file(
    indexes: &[ApiFileIndex],
    game_version: &str,
    loader: Option<i64>,
    release_type: i64,
) -> Option<i64> {
    indexes
        .iter()
        .filter(|index| {
            index.game_version == game_version
                && index.mod_loader.is_none_or(|l| Some(l) == loader)
                && index.release_type <= release_type
        })
        .map(|index| index.file_id)
        .max()
}

/// Forgets every CurseForge answer kept in memory.
pub fn clear_cache() {
    if let Ok(mut cache) = CACHE.lock() {
        cache.clear();
    }
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
    /// Required dependencies with no file for this instance, which have to
    /// be found some other way — CurseForge's own app reports them the same.
    pub missing_dependencies: Vec<MissingDependency>,
}

#[derive(Serialize, Debug)]
pub struct MissingDependency {
    pub project_id: String,
    pub name: Option<String>,
    pub website_url: Option<String>,
}

/// The newest file that fits the instance, preferring a release over a beta
/// over an alpha.
fn pick_file(files: Vec<ApiFile>) -> Option<ApiFile> {
    let mut files = files
        .into_iter()
        .filter(ApiFile::is_installable)
        .collect::<Vec<_>>();
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
        // A dependency already in the instance is left as it is, as
        // CurseForge's own installer does.
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
                let mut picked = pick_file(files);
                // Files from before CurseForge tagged loaders carry none, and
                // CurseForge's own app takes such a file for a Forge one.
                if picked.is_none() && loader == loader_id("forge") {
                    let untagged = list_files(
                        project_id,
                        Some(&content_set.game_version),
                        None,
                        &state,
                    )
                    .await?
                    .into_iter()
                    .filter(ApiFile::has_no_loader_tag)
                    .collect();
                    picked = pick_file(untagged);
                }
                match picked {
                    Some(file) => file,
                    // A dependency with nothing for this instance does not
                    // stop the install; it is reported, with where to look.
                    None if depth > 0 => {
                        outcome.missing_dependencies.push(MissingDependency {
                            project_id: format!("{ID_PREFIX}{project_id}"),
                            website_url: project.data.website_url(),
                            name: Some(project.data.name),
                        });
                        continue;
                    }
                    None => {
                        return Err(input(format!(
                            "{} has no file for Minecraft {}{}",
                            project.data.name,
                            content_set.game_version,
                            loader
                                .and_then(loader_name)
                                .map(|loader| format!(" with {loader}"))
                                .unwrap_or_default()
                        )));
                    }
                }
            }
        };

        if depth == 0 {
            refuse_incompatible(&file, &installed_ids, &state).await?;
            // Installing another version of something already there replaces
            // it, rather than leaving two copies of the same mod side by side.
            if installed_ids.contains(&project_id.to_string()) {
                remove_installed(instance_id, project_id).await?;
            }
        }

        for dependency in &file.dependencies {
            if dependency.relation_type == RELATION_REQUIRED {
                queue.push((dependency.mod_id, None, depth + 1));
            }
        }

        match &file.download_url {
            Some(url) => {
                install_file(instance_id, &file, url, &state).await?;
                installed_ids.insert(file.mod_id.to_string());
                outcome
                    .installed
                    .push(format!("{ID_PREFIX}{}", file.mod_id));
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

/// CurseForge's relation types, as a file lists its dependencies.
const RELATION_REQUIRED: i64 = 3;
const RELATION_INCOMPATIBLE: i64 = 5;

/// Refuse a file that says it cannot run beside something already in the
/// instance, naming what is in the way.
async fn refuse_incompatible(
    file: &ApiFile,
    installed_ids: &HashSet<String>,
    state: &State,
) -> crate::Result<()> {
    let clashing = file
        .dependencies
        .iter()
        .filter(|d| d.relation_type == RELATION_INCOMPATIBLE)
        .map(|d| d.mod_id.to_string())
        .filter(|id| installed_ids.contains(id))
        .collect::<HashSet<_>>();
    if clashing.is_empty() {
        return Ok(());
    }
    let names = crate::api::curseforge_metadata::resolve(
        &clashing,
        &HashSet::new(),
        state,
    )
    .await
    .projects;
    let mut listed = clashing
        .iter()
        .map(|id| names.get(id).map_or(id.clone(), |p| p.name.clone()))
        .collect::<Vec<_>>();
    listed.sort();
    Err(input(format!(
        "{} is incompatible with {}, which is already installed",
        file.display_name,
        listed.join(", ")
    )))
}

/// Remove every file of a project from the instance, ahead of installing
/// another version of it.
async fn remove_installed(
    instance_id: &str,
    project_id: i64,
) -> crate::Result<()> {
    let wanted = project_id.to_string();
    let paths = crate::api::instance::get_projects(instance_id, None)
        .await?
        .iter()
        .filter(|entry| {
            entry
                .value()
                .metadata
                .as_ref()
                .is_some_and(|metadata| metadata.project_id == wanted)
        })
        .map(|entry| entry.key().clone())
        .collect::<Vec<_>>();
    for path in paths {
        crate::api::instance::remove_project(instance_id, &path).await?;
    }
    Ok(())
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
/// install from. Called from inside the install job, so the download is part
/// of the task the user sees.
pub async fn download_modpack_bytes(
    project_id: &str,
    file_id: Option<String>,
) -> crate::Result<bytes::Bytes> {
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
    Ok(bytes)
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
    let files =
        curseforge_blocked::check_downloads(instance_id, &state).await?;
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
            category_ids: vec![406, 420],
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
        assert_eq!(get("gameVersions"), Some(r#"["1.20.1","1.20"]"#));
        assert_eq!(get("categoryIds"), Some("[406,420]"));
        // Only loaders CurseForge knows are sent.
        assert_eq!(get("modLoaderTypes"), Some("[4]"));
        assert_eq!(get("sortField"), Some("6"));
        assert_eq!(get("index"), Some("40"));
        assert_eq!(get("pageSize"), Some("20"));
    }

    #[test]
    fn relevance_is_popularity_among_the_matches() {
        let mut q = query("mod");
        q.sort = "relevance".to_string();
        let params = params(&search_url(&q).unwrap());
        assert!(params.contains(&("sortField".to_string(), "2".to_string())));
        assert!(
            params.contains(&("searchFilter".to_string(), "jei".to_string()))
        );
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

    fn file(id: i64, release_type: i64, date: &str) -> ApiFile {
        ApiFile {
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
            is_server_pack: None,
            is_available: None,
        }
    }

    #[test]
    fn the_newest_release_wins_over_a_newer_beta() {
        let picked = pick_file(vec![
            file(1, 1, "2024-01-01"),
            file(2, 2, "2024-03-01"),
            file(3, 1, "2024-02-01"),
        ])
        .unwrap();
        assert_eq!(picked.id, 3);
    }

    #[test]
    fn a_file_with_no_loader_tag_is_told_apart() {
        let mut legacy = file(1, 1, "2016-01-01");
        legacy.game_versions = vec!["1.12.2".to_string()];
        let mut tagged = file(2, 1, "2024-01-01");
        tagged.game_versions = vec!["1.20.1".to_string(), "Forge".to_string()];

        assert!(legacy.has_no_loader_tag());
        assert!(!tagged.has_no_loader_tag());
    }

    #[test]
    fn a_server_pack_or_pulled_file_is_never_picked() {
        let mut server = file(4, 1, "2024-05-01");
        server.is_server_pack = Some(true);
        let mut pulled = file(5, 1, "2024-06-01");
        pulled.is_available = Some(false);

        let picked =
            pick_file(vec![file(1, 1, "2024-01-01"), server, pulled]).unwrap();
        assert_eq!(picked.id, 1);
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

    #[test]
    fn newest_file_keeps_to_the_wanted_release_type() {
        let index = |game_version: &str, mod_loader, file_id, release_type| {
            ApiFileIndex {
                game_version: game_version.to_string(),
                mod_loader,
                file_id,
                release_type,
            }
        };
        let indexes = [
            index("1.21.1", Some(6), 10, 1),
            index("1.21.1", Some(6), 12, 2),
            index("1.21.1", Some(4), 20, 1),
            index("1.20.1", Some(6), 30, 1),
        ];
        assert_eq!(newest_file(&indexes, "1.21.1", Some(6), 1), Some(10));
        assert_eq!(newest_file(&indexes, "1.21.1", Some(6), 2), Some(12));
        assert_eq!(newest_file(&indexes, "1.21.1", Some(4), 1), Some(20));
        assert_eq!(newest_file(&indexes, "1.19.2", Some(6), 3), None);
        // Only a beta for the version: an update only if betas are wanted.
        let beta_only = [index("1.21.1", Some(6), 12, 2)];
        assert_eq!(newest_file(&beta_only, "1.21.1", Some(6), 1), None);
        assert_eq!(newest_file(&beta_only, "1.21.1", Some(6), 2), Some(12));
        // A pack names no loader and fits any instance.
        let pack = [index("1.21.1", None, 7, 1)];
        assert_eq!(newest_file(&pack, "1.21.1", None, 1), Some(7));
        assert_eq!(newest_file(&pack, "1.21.1", Some(1), 1), Some(7));
    }
}
