//! Names, pictures and authors for content that came from CurseForge.
//!
//! Everything the launcher knows about an installed file it learns from
//! Modrinth, by hashing the file and asking what it is. A jar from CurseForge
//! is not on Modrinth, so that question comes back empty and the file is drawn
//! as an anonymous upload: the filename where the title should be, "Unknown"
//! where the version should be, and no author, no icon and no link. A pack of
//! several hundred mods installed from a CurseForge zip was several hundred
//! rows of that.
//!
//! The ids are not actually missing, though. A CurseForge file's content entry
//! carries the numeric project and file ids it was installed with — that is
//! what `ContentSourceKind::has_curseforge_ids` marks — so the same two bulk
//! endpoints the modpack installer already uses can say what they are.
//!
//! What comes back is kept in tables of its own rather than in the shared
//! cache. Two reasons: the shared cache is keyed by Modrinth ids and shaped
//! around Modrinth's payloads, so CurseForge rows in it would be a standing
//! trap for anything that reads it by id; and a fork-only table is a fork-only
//! file, which is the difference between a sync that merges and a sync that
//! does not.
//!
//! Failure here is never fatal. A missing key, a rate limit, no network — the
//! listing is still drawn, with whatever was cached last time and filenames
//! for the rest, exactly as it was before any of this existed.

use std::collections::{HashMap, HashSet};
use std::io::{Read, Seek};
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use chrono::{DateTime, Duration, Utc};
use reqwest::Method;
use serde::Deserialize;

use crate::state::{FileMetadata, State};
use crate::util::fetch::fetch_advanced;

const MODS_ENDPOINT: &str = "https://api.curseforge.com/v1/mods";
const FILES_ENDPOINT: &str = "https://api.curseforge.com/v1/mods/files";
/// Minecraft's game id on CurseForge.
const FINGERPRINTS_ENDPOINT: &str =
    "https://api.curseforge.com/v1/fingerprints/432";

/// How long a cached answer is trusted. A mod's name, author and icon change
/// about as often as anything on Modrinth does, and the alternative to a stale
/// title is no title.
const CACHE_DAYS: i64 = 7;

/// The most ids CurseForge accepts in one bulk request.
const MAX_IDS_PER_REQUEST: usize = 500;

/// The key the frontend was built with, handed over once at startup.
///
/// It lives here rather than in `State` because it is not part of the
/// launcher's own state: it is a build-time constant of the frontend's, and
/// the backend is simply told what it is. `None` means CurseForge cannot be
/// asked anything, which is a perfectly ordinary way for a build to be
/// configured and must stay silent rather than erroring.
static API_KEY: RwLock<Option<String>> = RwLock::new(None);

/// Called once from the frontend at startup.
pub fn set_api_key(key: Option<String>) {
    let key = key
        .map(|key| key.trim().to_string())
        .filter(|k| !k.is_empty());
    let had_key = api_key().is_some();
    if let Ok(mut slot) = API_KEY.write() {
        *slot = key;
    }
    if !had_key && api_key().is_some() {
        tracing::debug!("CurseForge metadata lookups are available");
    }
}

pub(crate) fn api_key() -> Option<String> {
    API_KEY.read().ok().and_then(|slot| slot.clone())
}

/// A CurseForge project, reduced to what a content row draws.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CurseforgeProject {
    pub id: String,
    pub name: String,
    pub slug: Option<String>,
    pub summary: Option<String>,
    pub icon_url: Option<String>,
    pub website_url: Option<String>,
    pub author_id: Option<String>,
    pub author_name: Option<String>,
    pub author_url: Option<String>,
    pub author_avatar_url: Option<String>,
}

/// One file of a CurseForge project, likewise.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CurseforgeFile {
    pub id: String,
    pub project_id: String,
    /// What CurseForge shows as the version, e.g. `JEI 1.20.1-15.3.0.4`.
    pub display_name: String,
    pub file_date: Option<String>,
}

#[derive(Default)]
pub struct CurseforgeMetadata {
    pub projects: HashMap<String, CurseforgeProject>,
    pub files: HashMap<String, CurseforgeFile>,
}

#[derive(Deserialize)]
struct ModsResponse {
    #[serde(default)]
    data: Vec<ApiMod>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiMod {
    id: i64,
    #[serde(default)]
    name: String,
    #[serde(default)]
    slug: Option<String>,
    #[serde(default)]
    summary: Option<String>,
    #[serde(default)]
    logo: Option<ApiLogo>,
    #[serde(default)]
    links: Option<ApiLinks>,
    #[serde(default)]
    authors: Vec<ApiAuthor>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiLogo {
    #[serde(default)]
    thumbnail_url: Option<String>,
    #[serde(default)]
    url: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiLinks {
    #[serde(default)]
    website_url: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiAuthor {
    #[serde(default)]
    id: Option<i64>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    avatar_url: Option<String>,
}

#[derive(Deserialize)]
struct FilesResponse {
    #[serde(default)]
    data: Vec<ApiFile>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiFile {
    id: i64,
    mod_id: i64,
    #[serde(default)]
    display_name: Option<String>,
    #[serde(default)]
    file_name: Option<String>,
    #[serde(default)]
    file_date: Option<String>,
    #[serde(default)]
    file_fingerprint: Option<u32>,
}

#[derive(Deserialize)]
struct FingerprintsResponse {
    data: FingerprintMatches,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FingerprintMatches {
    #[serde(default)]
    exact_matches: Vec<FingerprintMatch>,
}

#[derive(Deserialize)]
struct FingerprintMatch {
    file: ApiFile,
}

impl From<ApiMod> for CurseforgeProject {
    fn from(value: ApiMod) -> Self {
        // The thumbnail is what a 40px row wants; the full-size logo is the
        // fallback for a project whose thumbnail CurseForge never generated.
        let icon_url = value
            .logo
            .and_then(|logo| logo.thumbnail_url.or(logo.url))
            .filter(|url| url.starts_with("https://"));
        let author = value.authors.into_iter().next();

        Self {
            id: value.id.to_string(),
            name: value.name,
            slug: value.slug,
            summary: value.summary,
            icon_url,
            website_url: value
                .links
                .and_then(|links| links.website_url)
                .filter(|url| url.starts_with("https://")),
            author_id: author
                .as_ref()
                .and_then(|a| a.id)
                .map(|id| id.to_string()),
            author_name: author.as_ref().and_then(|a| a.name.clone()),
            author_avatar_url: author
                .as_ref()
                .and_then(|a| a.avatar_url.clone())
                .filter(|url| url.starts_with("https://")),
            author_url: author
                .and_then(|a| a.url)
                .filter(|url| url.starts_with("https://")),
        }
    }
}

impl From<ApiFile> for CurseforgeFile {
    fn from(value: ApiFile) -> Self {
        let id = value.id.to_string();
        Self {
            display_name: value
                .display_name
                .or(value.file_name)
                .unwrap_or_else(|| id.clone()),
            id,
            project_id: value.mod_id.to_string(),
            file_date: value.file_date,
        }
    }
}

/// Names and pictures for the given CurseForge ids, from the cache where it is
/// fresh and from CurseForge where it is not.
///
/// Never returns an error: the caller is drawing a list, and a list with
/// filenames in it beats no list at all.
pub async fn resolve(
    project_ids: &HashSet<String>,
    file_ids: &HashSet<String>,
    state: &State,
) -> CurseforgeMetadata {
    if project_ids.is_empty() && file_ids.is_empty() {
        return CurseforgeMetadata::default();
    }

    let now = Utc::now();
    let mut metadata = CurseforgeMetadata {
        projects: read_cached_projects(project_ids, now, state)
            .await
            .unwrap_or_else(|error| {
                tracing::warn!(
                    "Could not read CurseForge project cache: {error}"
                );
                HashMap::new()
            }),
        files: read_cached_files(file_ids, now, state)
            .await
            .unwrap_or_else(|error| {
                tracing::warn!("Could not read CurseForge file cache: {error}");
                HashMap::new()
            }),
    };

    let missing_projects = difference(project_ids, &metadata.projects);
    let missing_files = difference(file_ids, &metadata.files);
    if missing_projects.is_empty() && missing_files.is_empty() {
        return metadata;
    }

    let Some(key) = api_key() else {
        tracing::debug!(
            "No CurseForge API key, so {} project(s) and {} file(s) stay \
             unnamed",
            missing_projects.len(),
            missing_files.len()
        );
        return metadata;
    };

    if !missing_projects.is_empty()
        && let Err(error) =
            fetch_projects(&missing_projects, &key, &mut metadata, state).await
    {
        tracing::warn!("Could not look up CurseForge projects: {error}");
    }
    if !missing_files.is_empty()
        && let Err(error) =
            fetch_files(&missing_files, &key, &mut metadata, state).await
    {
        tracing::warn!("Could not look up CurseForge files: {error}");
    }

    metadata
}

fn difference<T>(
    wanted: &HashSet<String>,
    have: &HashMap<String, T>,
) -> Vec<i64> {
    let mut ids = wanted
        .iter()
        .filter(|id| !have.contains_key(*id))
        // Anything that is not a number was never a CurseForge id, so asking
        // about it would only waste the request.
        .filter_map(|id| id.parse::<i64>().ok())
        .collect::<Vec<_>>();
    ids.sort_unstable();
    ids.truncate(MAX_IDS_PER_REQUEST);
    ids
}

async fn fetch_projects(
    ids: &[i64],
    key: &str,
    metadata: &mut CurseforgeMetadata,
    state: &State,
) -> crate::Result<()> {
    let body = fetch_advanced(
        Method::POST,
        MODS_ENDPOINT,
        None,
        Some(serde_json::json!({ "modIds": ids })),
        Some(("x-api-key", key)),
        None,
        None,
        None,
        &state.fetch_semaphore,
        &state.pool,
    )
    .await?;
    let response: ModsResponse = serde_json::from_slice(&body)?;
    let projects = response
        .data
        .into_iter()
        .map(CurseforgeProject::from)
        .collect::<Vec<_>>();

    write_cached_projects(&projects, state).await?;
    for project in projects {
        metadata.projects.insert(project.id.clone(), project);
    }

    Ok(())
}

async fn fetch_files(
    ids: &[i64],
    key: &str,
    metadata: &mut CurseforgeMetadata,
    state: &State,
) -> crate::Result<()> {
    let body = fetch_advanced(
        Method::POST,
        FILES_ENDPOINT,
        None,
        Some(serde_json::json!({ "fileIds": ids })),
        Some(("x-api-key", key)),
        None,
        None,
        None,
        &state.fetch_semaphore,
        &state.pool,
    )
    .await?;
    let response: FilesResponse = serde_json::from_slice(&body)?;
    let files = response
        .data
        .into_iter()
        .map(CurseforgeFile::from)
        .collect::<Vec<_>>();

    write_cached_files(&files, state).await?;
    for file in files {
        metadata.files.insert(file.id.clone(), file);
    }

    Ok(())
}

/// CurseForge's ids for files that came with none, recognised by content.
///
/// `files` pairs each file's SHA1 with where it is on disk. What comes back is
/// keyed by that SHA1, in the same project/file shape a content entry carries
/// its ids in, so the caller can hand the ids straight to [`resolve`]. A file
/// CurseForge does not know is simply absent.
///
/// Like [`resolve`], never fails: without a key, a network or an answer, the
/// files stay as unnamed as they were.
pub async fn identify(
    files: &[(String, PathBuf)],
    state: &State,
) -> HashMap<String, FileMetadata> {
    if files.is_empty() {
        return HashMap::new();
    }

    let cached = read_cached_fingerprints(files, Utc::now(), state)
        .await
        .unwrap_or_else(|error| {
            tracing::warn!(
                "Could not read CurseForge fingerprint cache: {error}"
            );
            HashMap::new()
        });
    let mut found = cached
        .iter()
        .filter_map(|(sha1, ids)| Some((sha1.clone(), ids.clone()?)))
        .collect::<HashMap<_, _>>();

    let unknown = files
        .iter()
        .filter(|(sha1, _)| !cached.contains_key(sha1))
        .take(MAX_IDS_PER_REQUEST)
        .cloned()
        .collect::<Vec<_>>();
    if unknown.is_empty() {
        return found;
    }

    let Some(key) = api_key() else {
        tracing::debug!(
            "No CurseForge API key, so {} file(s) stay unrecognised",
            unknown.len()
        );
        return found;
    };

    if let Err(error) =
        fetch_fingerprints(unknown, &key, &mut found, state).await
    {
        tracing::warn!("Could not match CurseForge fingerprints: {error}");
    }

    found
}

async fn fetch_fingerprints(
    files: Vec<(String, PathBuf)>,
    key: &str,
    found: &mut HashMap<String, FileMetadata>,
    state: &State,
) -> crate::Result<()> {
    // Every byte of every file has to be read, twice, so it is done off the
    // async runtime. A file that cannot be read is left out and asked about
    // again next time.
    let fingerprints = tokio::task::spawn_blocking(move || {
        files
            .into_iter()
            .filter_map(|(sha1, path)| match fingerprint_file(&path) {
                Ok(fingerprint) => Some((sha1, fingerprint)),
                Err(error) => {
                    tracing::debug!(
                        "Could not fingerprint {}: {error}",
                        path.display()
                    );
                    None
                }
            })
            .collect::<Vec<_>>()
    })
    .await?;
    if fingerprints.is_empty() {
        return Ok(());
    }

    let body = fetch_advanced(
        Method::POST,
        FINGERPRINTS_ENDPOINT,
        None,
        Some(serde_json::json!({
            "fingerprints": fingerprints
                .iter()
                .map(|(_, fingerprint)| *fingerprint)
                .collect::<Vec<_>>(),
        })),
        Some(("x-api-key", key)),
        None,
        None,
        None,
        &state.fetch_semaphore,
        &state.pool,
    )
    .await?;
    let response: FingerprintsResponse = serde_json::from_slice(&body)?;
    let matched = response
        .data
        .exact_matches
        .into_iter()
        .filter_map(|m| Some((m.file.file_fingerprint?, m.file)))
        .collect::<HashMap<_, _>>();

    // The match already says what the file is called, so it goes straight
    // into the file cache and `resolve` has nothing left to ask about it.
    let files = matched
        .values()
        .cloned()
        .map(CurseforgeFile::from)
        .collect::<Vec<_>>();
    write_cached_files(&files, state).await?;

    let answers = fingerprints
        .into_iter()
        .map(|(sha1, fingerprint)| {
            let ids = matched.get(&fingerprint).map(|file| FileMetadata {
                project_id: file.mod_id.to_string(),
                version_id: file.id.to_string(),
            });
            (sha1, ids)
        })
        .collect::<Vec<_>>();
    write_cached_fingerprints(&answers, state).await?;
    for (sha1, ids) in answers {
        if let Some(ids) = ids {
            found.insert(sha1, ids);
        }
    }

    Ok(())
}

/// The bytes CurseForge leaves out of a fingerprint: tab, LF, CR and space.
fn is_fingerprint_whitespace(byte: u8) -> bool {
    matches!(byte, 9 | 10 | 13 | 32)
}

/// CurseForge's file fingerprint: MurmurHash2 (seed 1) over the file with
/// every whitespace byte removed. The length that seeds the hash is the
/// length after removal, which is why a file is read twice.
struct Fingerprint {
    hash: u32,
    block: u32,
    shift: u32,
}

impl Fingerprint {
    const M: u32 = 0x5bd1_e995;

    fn new(normalized_len: u64) -> Self {
        Self {
            // Only the low 32 bits of the length take part, as in CurseForge's
            // own implementation.
            hash: 1 ^ normalized_len as u32,
            block: 0,
            shift: 0,
        }
    }

    fn update(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            if is_fingerprint_whitespace(byte) {
                continue;
            }
            self.block |= u32::from(byte) << self.shift;
            self.shift += 8;
            if self.shift == 32 {
                let mut k = self.block.wrapping_mul(Self::M);
                k ^= k >> 24;
                k = k.wrapping_mul(Self::M);
                self.hash = self.hash.wrapping_mul(Self::M) ^ k;
                self.block = 0;
                self.shift = 0;
            }
        }
    }

    fn finish(self) -> u32 {
        let mut hash = self.hash;
        if self.shift > 0 {
            hash ^= self.block;
            hash = hash.wrapping_mul(Self::M);
        }
        hash ^= hash >> 13;
        hash = hash.wrapping_mul(Self::M);
        hash ^ (hash >> 15)
    }
}

#[cfg(test)]
fn fingerprint(bytes: &[u8]) -> u32 {
    let len = bytes
        .iter()
        .filter(|&&b| !is_fingerprint_whitespace(b))
        .count();
    let mut fingerprint = Fingerprint::new(len as u64);
    fingerprint.update(bytes);
    fingerprint.finish()
}

fn fingerprint_file(path: &Path) -> std::io::Result<u32> {
    let mut file = std::fs::File::open(path)?;
    let mut buffer = vec![0; 256 * 1024];

    let mut len = 0u64;
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        len += buffer[..read]
            .iter()
            .filter(|&&b| !is_fingerprint_whitespace(b))
            .count() as u64;
    }

    file.rewind()?;
    let mut fingerprint = Fingerprint::new(len);
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        fingerprint.update(&buffer[..read]);
    }
    Ok(fingerprint.finish())
}

async fn read_cached_fingerprints(
    files: &[(String, PathBuf)],
    now: DateTime<Utc>,
    state: &State,
) -> crate::Result<HashMap<String, Option<FileMetadata>>> {
    let wanted = files
        .iter()
        .map(|(sha1, _)| sha1.as_str())
        .collect::<HashSet<_>>();
    let expires_after = now.timestamp();
    let rows = sqlx::query!(
        "
        SELECT sha1, project_id, file_id
        FROM curseforge_fingerprint_cache
        WHERE expires > $1
        ",
        expires_after
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(rows
        .into_iter()
        .filter(|row| wanted.contains(row.sha1.as_str()))
        .map(|row| {
            let ids = row.project_id.zip(row.file_id).map(
                |(project_id, version_id)| FileMetadata {
                    project_id,
                    version_id,
                },
            );
            (row.sha1, ids)
        })
        .collect())
}

async fn write_cached_fingerprints(
    answers: &[(String, Option<FileMetadata>)],
    state: &State,
) -> crate::Result<()> {
    if answers.is_empty() {
        return Ok(());
    }
    let expires = (Utc::now() + Duration::days(CACHE_DAYS)).timestamp();
    let mut tx = state.pool.begin().await?;
    for (sha1, ids) in answers {
        let project_id = ids.as_ref().map(|ids| ids.project_id.as_str());
        let file_id = ids.as_ref().map(|ids| ids.version_id.as_str());
        sqlx::query!(
            "
            INSERT INTO curseforge_fingerprint_cache (
                sha1, project_id, file_id, expires
            )
            VALUES ($1, $2, $3, $4)
            ON CONFLICT(sha1) DO UPDATE SET
                project_id = excluded.project_id,
                file_id = excluded.file_id,
                expires = excluded.expires
            ",
            sha1,
            project_id,
            file_id,
            expires,
        )
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    Ok(())
}

async fn read_cached_projects(
    ids: &HashSet<String>,
    now: DateTime<Utc>,
    state: &State,
) -> crate::Result<HashMap<String, CurseforgeProject>> {
    if ids.is_empty() {
        return Ok(HashMap::new());
    }
    let expires_after = now.timestamp();
    let rows = sqlx::query!(
        "
        SELECT project_id, name, slug, summary, icon_url, website_url,
               author_id, author_name, author_url, author_avatar_url
        FROM curseforge_project_cache
        WHERE expires > $1
        ",
        expires_after
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(rows
        .into_iter()
        .filter(|row| ids.contains(&row.project_id))
        .map(|row| {
            (
                row.project_id.clone(),
                CurseforgeProject {
                    id: row.project_id,
                    name: row.name,
                    slug: row.slug,
                    summary: row.summary,
                    icon_url: row.icon_url,
                    website_url: row.website_url,
                    author_id: row.author_id,
                    author_name: row.author_name,
                    author_url: row.author_url,
                    author_avatar_url: row.author_avatar_url,
                },
            )
        })
        .collect())
}

async fn read_cached_files(
    ids: &HashSet<String>,
    now: DateTime<Utc>,
    state: &State,
) -> crate::Result<HashMap<String, CurseforgeFile>> {
    if ids.is_empty() {
        return Ok(HashMap::new());
    }
    let expires_after = now.timestamp();
    let rows = sqlx::query!(
        "
        SELECT file_id, project_id, display_name, file_date
        FROM curseforge_file_cache
        WHERE expires > $1
        ",
        expires_after
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(rows
        .into_iter()
        .filter(|row| ids.contains(&row.file_id))
        .map(|row| {
            (
                row.file_id.clone(),
                CurseforgeFile {
                    id: row.file_id,
                    project_id: row.project_id,
                    display_name: row.display_name,
                    file_date: row.file_date,
                },
            )
        })
        .collect())
}

async fn write_cached_projects(
    projects: &[CurseforgeProject],
    state: &State,
) -> crate::Result<()> {
    if projects.is_empty() {
        return Ok(());
    }
    let expires = (Utc::now() + Duration::days(CACHE_DAYS)).timestamp();
    let mut tx = state.pool.begin().await?;
    for project in projects {
        sqlx::query!(
            "
            INSERT INTO curseforge_project_cache (
                project_id, name, slug, summary, icon_url, website_url,
                author_id, author_name, author_url, author_avatar_url, expires
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ON CONFLICT(project_id) DO UPDATE SET
                name = excluded.name,
                slug = excluded.slug,
                summary = excluded.summary,
                icon_url = excluded.icon_url,
                website_url = excluded.website_url,
                author_id = excluded.author_id,
                author_name = excluded.author_name,
                author_url = excluded.author_url,
                author_avatar_url = excluded.author_avatar_url,
                expires = excluded.expires
            ",
            project.id,
            project.name,
            project.slug,
            project.summary,
            project.icon_url,
            project.website_url,
            project.author_id,
            project.author_name,
            project.author_url,
            project.author_avatar_url,
            expires,
        )
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    Ok(())
}

async fn write_cached_files(
    files: &[CurseforgeFile],
    state: &State,
) -> crate::Result<()> {
    if files.is_empty() {
        return Ok(());
    }
    let expires = (Utc::now() + Duration::days(CACHE_DAYS)).timestamp();
    let mut tx = state.pool.begin().await?;
    for file in files {
        sqlx::query!(
            "
            INSERT INTO curseforge_file_cache (
                file_id, project_id, display_name, file_date, expires
            )
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT(file_id) DO UPDATE SET
                project_id = excluded.project_id,
                display_name = excluded.display_name,
                file_date = excluded.file_date,
                expires = excluded.expires
            ",
            file.id,
            file.project_id,
            file.display_name,
            file.file_date,
            expires,
        )
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_project_takes_its_thumbnail_over_its_full_size_logo() {
        let api_mod: ApiMod = serde_json::from_str(
            r#"{
                "id": 238222,
                "name": "Just Enough Items",
                "slug": "jei",
                "logo": {
                    "thumbnailUrl": "https://media.forgecdn.net/small.png",
                    "url": "https://media.forgecdn.net/big.png"
                },
                "links": { "websiteUrl": "https://www.curseforge.com/x/jei" },
                "authors": [
                    {
                        "id": 42,
                        "name": "mezz",
                        "url": "https://www.curseforge.com/m",
                        "avatarUrl": "https://media.forgecdn.net/a.png"
                    }
                ]
            }"#,
        )
        .unwrap();
        let project = CurseforgeProject::from(api_mod);

        assert_eq!(project.name, "Just Enough Items");
        assert_eq!(
            project.icon_url.as_deref(),
            Some("https://media.forgecdn.net/small.png")
        );
        assert_eq!(project.author_name.as_deref(), Some("mezz"));
        assert_eq!(project.author_id.as_deref(), Some("42"));
        assert_eq!(
            project.author_avatar_url.as_deref(),
            Some("https://media.forgecdn.net/a.png")
        );
        assert_eq!(
            project.website_url.as_deref(),
            Some("https://www.curseforge.com/x/jei")
        );
    }

    #[test]
    fn a_project_with_nothing_but_a_name_still_resolves() {
        let api_mod: ApiMod =
            serde_json::from_str(r#"{ "id": 1, "name": "Bare" }"#).unwrap();
        let project = CurseforgeProject::from(api_mod);

        assert_eq!(project.id, "1");
        assert_eq!(project.name, "Bare");
        assert!(project.icon_url.is_none());
        assert!(project.author_name.is_none());
    }

    #[test]
    fn an_http_url_is_not_taken() {
        // CurseForge serves everything over TLS; anything else in a payload
        // is not something to go fetching an image from.
        let api_mod: ApiMod = serde_json::from_str(
            r#"{
                "id": 2,
                "name": "Sketchy",
                "logo": { "url": "http://example.invalid/x.png" },
                "links": { "websiteUrl": "http://example.invalid" }
            }"#,
        )
        .unwrap();
        let project = CurseforgeProject::from(api_mod);

        assert!(project.icon_url.is_none());
        assert!(project.website_url.is_none());
    }

    #[test]
    fn a_file_prefers_the_name_curseforge_displays() {
        let api_file: ApiFile = serde_json::from_str(
            r#"{
                "id": 5090810,
                "modId": 627196,
                "displayName": "JEI 1.20.1-15.3.0.4",
                "fileName": "jei-1.20.1-forge-15.3.0.4.jar",
                "fileDate": "2024-01-01T00:00:00Z"
            }"#,
        )
        .unwrap();
        let file = CurseforgeFile::from(api_file);

        assert_eq!(file.id, "5090810");
        assert_eq!(file.project_id, "627196");
        assert_eq!(file.display_name, "JEI 1.20.1-15.3.0.4");
    }

    #[test]
    fn a_file_falls_back_to_its_filename() {
        let api_file: ApiFile = serde_json::from_str(
            r#"{ "id": 7, "modId": 8, "fileName": "thing.jar" }"#,
        )
        .unwrap();

        assert_eq!(CurseforgeFile::from(api_file).display_name, "thing.jar");
    }

    #[test]
    fn only_ids_that_are_not_already_known_are_asked_about() {
        let wanted = HashSet::from([
            "1".to_string(),
            "2".to_string(),
            // A Modrinth id that somehow reached here is not a number and
            // must never be sent to CurseForge.
            "AANobbMI".to_string(),
        ]);
        let have = HashMap::from([(
            "1".to_string(),
            CurseforgeFile {
                id: "1".to_string(),
                project_id: "1".to_string(),
                display_name: "known".to_string(),
                file_date: None,
            },
        )]);

        assert_eq!(difference(&wanted, &have), vec![2]);
    }

    /// Expected values come from running CurseForge's own `Curse.Hashing`
    /// (`MurmurHash2.ComputeNormalizedHash`) on the same bytes.
    #[test]
    fn fingerprints_match_curseforges_own_implementation() {
        let cases: &[(&[u8], u32)] = &[
            (b"", 1540447798),
            (b"a", 626045324),
            (b"ab", 1692487918),
            (b"abc", 1621425345),
            (b"abcd", 3376380438),
            (b"abcde", 3469237630),
            (b"hello world", 2824650221),
            (b"Noctrinth fingerprint test vector", 2776368908),
        ];
        for (input, expected) in cases {
            assert_eq!(fingerprint(input), *expected, "{input:?}");
        }
    }

    #[test]
    fn whitespace_does_not_change_a_fingerprint() {
        assert_eq!(
            fingerprint(b" \t\r\nhello\n world \r\n"),
            fingerprint(b"helloworld")
        );
        assert_eq!(fingerprint(b"    "), fingerprint(b""));
    }

    #[test]
    fn a_file_fingerprints_the_same_as_its_bytes() {
        // Longer than one read buffer, with whitespace bytes scattered
        // through it, so both passes cross buffer boundaries.
        let bytes = (0..600_000u32)
            .map(|i| ((i * 31 + 7) % 251) as u8)
            .collect::<Vec<_>>();
        assert_eq!(fingerprint(&bytes[..200_000]), 2392768800);

        let mut file = tempfile::NamedTempFile::new().unwrap();
        std::io::Write::write_all(&mut file, &bytes).unwrap();
        assert_eq!(fingerprint_file(file.path()).unwrap(), fingerprint(&bytes));
    }

    #[test]
    fn a_fingerprint_match_reads_the_file_it_names() {
        let response: FingerprintsResponse = serde_json::from_str(
            r#"{
                "data": {
                    "isCacheBuilt": true,
                    "exactMatches": [{
                        "id": 5090810,
                        "file": {
                            "id": 5090810,
                            "modId": 238222,
                            "displayName": "JEI 1.20.1-15.3.0.4",
                            "fileName": "jei.jar",
                            "fileFingerprint": 2824650221
                        },
                        "latestFiles": []
                    }],
                    "exactFingerprints": [2824650221],
                    "unmatchedFingerprints": [1]
                }
            }"#,
        )
        .unwrap();
        let file = &response.data.exact_matches[0].file;

        assert_eq!(file.file_fingerprint, Some(2824650221));
        assert_eq!(file.mod_id, 238222);
    }

    #[test]
    fn a_key_that_is_only_whitespace_counts_as_no_key() {
        set_api_key(Some("   ".to_string()));
        assert!(api_key().is_none());

        set_api_key(Some("real-key".to_string()));
        assert_eq!(api_key().as_deref(), Some("real-key"));

        set_api_key(None);
        assert!(api_key().is_none());
    }
}
