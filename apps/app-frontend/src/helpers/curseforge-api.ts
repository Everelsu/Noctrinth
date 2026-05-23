/**
 * CurseForge REST API client.
 *
 * Requires a CurseForge API key in the VITE_CURSEFORGE_API_KEY environment
 * variable. If the key is absent every function returns null so the rest of
 * the app can treat CurseForge as simply unavailable.
 *
 * Docs: https://docs.curseforge.com/rest-api/
 */

import { fetch as tauriFetch } from '@tauri-apps/plugin-http'

import { CURSEFORGE_API_KEY } from './curseforge-key'
import { create_profile_and_install_from_curseforge } from './pack'
import { add_project_from_curseforge } from './profile'

const CF_BASE = 'https://api.curseforge.com/v1'

/**
 * CurseForge API key.
 *
 * vite.config.ts reads VITE_CURSEFORGE_API_KEY from .env.local directly and
 * injects it (bypassing dotenv-expand so the bcrypt-style `$` characters
 * survive). Re-exported from curseforge-key.ts to avoid an import cycle.
 */
const CF_API_KEY: string = CURSEFORGE_API_KEY

// ─── Minecraft class IDs ──────────────────────────────────────────────────────

export const CF_CLASS_IDS: Record<string, number> = {
  mod: 6,
  modpack: 4471,
  resourcepack: 12,
  shader: 6945,
  datapack: 6552,
}

// ─── Mod-loader type IDs ──────────────────────────────────────────────────────

export const CF_LOADER_TYPES: Record<string, number> = {
  forge: 1,
  fabric: 4,
  quilt: 5,
  neoforge: 6,
}

// ─── Sort field IDs ──────────────────────────────────────────────────────────
// CurseForge ModsSearchSortField: 1 Featured, 2 Popularity, 3 LastUpdated,
// 4 Name, 5 Author, 6 TotalDownloads, 7 Category, 8 GameVersion,
// 9 EarlyAccess, 10 FeaturedReleased, 11 ReleasedDate, 12 Rating.

const CF_SORT_FIELDS: Record<string, number> = {
  relevance: 2, // Popularity
  downloads: 6, // TotalDownloads
  follows: 12, // Rating
  newest: 11, // ReleasedDate
  updated: 3, // LastUpdated
}

// ─── API types ────────────────────────────────────────────────────────────────

export interface CfCategory {
  id: number
  name: string
  slug: string
}

export interface CfAuthor {
  id: number
  name: string
}

export interface CfFileIndex {
  gameVersion: string
  /** Loader type ID — 0 = any, 1 = Forge, 4 = Fabric, 5 = Quilt, 6 = NeoForge */
  modLoader: number
}

export interface CfLogo {
  url: string
  thumbnailUrl?: string
}

export interface CfMod {
  id: number
  name: string
  slug: string
  summary: string
  downloadCount: number
  thumbsUpCount: number
  logo?: CfLogo
  categories: CfCategory[]
  authors: CfAuthor[]
  dateCreated: string
  dateModified: string
  latestFilesIndexes: CfFileIndex[]
  classId?: number
}

export interface CfSearchResult {
  data: CfMod[]
  pagination: {
    index: number
    pageSize: number
    resultCount: number
    totalCount: number
  }
}

// ─── Mapped hit shape (compatible with Modrinth v2 result) ───────────────────

export interface CfMappedHit {
  /** Namespaced ID so it never collides with a Modrinth project_id */
  project_id: string
  slug: string
  /** Modrinth-compatible aliases */
  title: string
  name: string
  description: string
  summary: string
  icon_url: string | null
  downloads: number
  follows: number
  author: string
  categories: string[]
  display_categories: string[]
  date_created: string
  date_modified: string
  color: null
  featured_gallery: null
  gallery: string[]
  /** Original CurseForge numeric ID */
  cf_id: number
  sources: {
    curseforge: { mod_id: number; slug: string }
  }
}

// ─── Public API ───────────────────────────────────────────────────────────────

export function isCurseForgeAvailable(): boolean {
  return CF_API_KEY.length > 0
}

export interface CfSearchParams {
  query?: string
  projectType?: string
  gameVersion?: string
  modLoader?: string
  categoryId?: number
  limit?: number
  offset?: number
  sortBy?: string
}

/**
 * Search CurseForge for Minecraft mods/modpacks/etc.
 * Returns null when the API key is missing or the request fails.
 */
/** CurseForge rejects pagination beyond this many results. */
const CF_MAX_INDEX = 10_000

export async function searchCurseForge(params: CfSearchParams): Promise<CfSearchResult | null> {
  if (!isCurseForgeAvailable()) return null

  // CurseForge caps pagination — skip the request entirely past the limit.
  if ((params.offset ?? 0) >= CF_MAX_INDEX) return null

  const classId = CF_CLASS_IDS[params.projectType ?? 'mod'] ?? CF_CLASS_IDS.mod
  const loaderType = params.modLoader ? (CF_LOADER_TYPES[params.modLoader] ?? 0) : 0
  const sortField = CF_SORT_FIELDS[params.sortBy ?? 'relevance'] ?? CF_SORT_FIELDS.relevance
  // CurseForge max page size is 50
  const pageSize = Math.min(params.limit ?? 20, 50)

  const qs = new URLSearchParams({
    gameId: '432', // Minecraft
    classId: String(classId),
    pageSize: String(pageSize),
    index: String(params.offset ?? 0),
    sortField: String(sortField),
    sortOrder: 'desc',
  })

  if (params.query) qs.set('searchFilter', params.query)
  if (params.gameVersion) qs.set('gameVersion', params.gameVersion)
  if (loaderType) qs.set('modLoaderType', String(loaderType))
  if (params.categoryId) qs.set('categoryId', String(params.categoryId))

  return cfFetch<CfSearchResult>(`/mods/search?${qs}`)
}

// ─── Categories ───────────────────────────────────────────────────────────────

export interface CfCategoryInfo {
  id: number
  name: string
  slug: string
  classId?: number
}

let categoriesCache: Promise<CfCategoryInfo[]> | null = null

/** Fetch (and cache) the full CurseForge Minecraft category list. */
export function getCurseForgeCategories(): Promise<CfCategoryInfo[]> {
  if (!categoriesCache) {
    categoriesCache = cfFetch<{ data: CfCategoryInfo[] }>('/categories?gameId=432').then(
      (json) => json?.data ?? [],
    )
  }
  return categoriesCache
}

/**
 * Resolve a list of category slugs to a single CurseForge category ID.
 * CurseForge's `/mods/search` only accepts one categoryId, so the first
 * slug that maps to a known CurseForge category wins.
 */
export async function resolveCfCategoryId(slugs: string[]): Promise<number | undefined> {
  if (!slugs.length) return undefined
  const categories = await getCurseForgeCategories()
  for (const slug of slugs) {
    const match = categories.find((c) => c.slug === slug)
    if (match) return match.id
  }
  return undefined
}

/**
 * Shared CurseForge GET helper — attaches the API key, handles errors,
 * and returns parsed JSON (or null on any failure / missing key).
 */
async function cfFetch<T>(path: string): Promise<T | null> {
  if (!isCurseForgeAvailable()) return null

  try {
    const res = await tauriFetch(`${CF_BASE}${path}`, {
      headers: { 'x-api-key': CF_API_KEY },
    })

    if (!res.ok) {
      const body = await res.text().catch(() => '(unreadable)')
      console.warn('[CurseForge] Non-OK response:', res.status, path, body)
      return null
    }

    return (await res.json()) as T
  } catch (err) {
    console.warn('[CurseForge] Request failed:', path, err)
    return null
  }
}

/**
 * Map a raw CurseForge mod to a shape that is compatible with
 * Modrinth's v2 ResultSearchProject so both can flow through the same
 * card component and list logic.
 */
export function mapCfMod(mod: CfMod): CfMappedHit {
  const slugList = mod.categories.map((c) => c.slug)
  return {
    project_id: `cf:${mod.id}`,
    slug: mod.slug,
    title: mod.name,
    name: mod.name,
    description: mod.summary,
    summary: mod.summary,
    icon_url: mod.logo?.thumbnailUrl ?? mod.logo?.url ?? null,
    downloads: mod.downloadCount,
    follows: mod.thumbsUpCount,
    author: mod.authors?.[0]?.name ?? '',
    categories: slugList,
    display_categories: slugList,
    date_created: mod.dateCreated,
    date_modified: mod.dateModified,
    color: null,
    featured_gallery: null,
    gallery: [],
    cf_id: mod.id,
    sources: { curseforge: { mod_id: mod.id, slug: mod.slug } },
  }
}

// ─── Mod detail API ───────────────────────────────────────────────────────────

export interface CfModLinks {
  websiteUrl?: string | null
  wikiUrl?: string | null
  issuesUrl?: string | null
  sourceUrl?: string | null
}

export interface CfScreenshot {
  id: number
  url: string
  thumbnailUrl?: string
  title?: string
  description?: string
}

/** Full mod object returned by GET /v1/mods/{modId} — a superset of CfMod. */
export interface CfModDetail extends CfMod {
  links?: CfModLinks
  screenshots?: CfScreenshot[]
  dateReleased?: string
}

/** Fetch the full detail object for a single CurseForge mod. */
export async function getCurseForgeMod(modId: number | string): Promise<CfModDetail | null> {
  const json = await cfFetch<{ data: CfModDetail }>(`/mods/${modId}`)
  return json?.data ?? null
}

/** Fetch the rendered HTML description for a CurseForge mod. */
export async function getCurseForgeModDescription(
  modId: number | string,
): Promise<string | null> {
  const json = await cfFetch<{ data: string }>(`/mods/${modId}/description`)
  return json?.data ?? null
}

/** Human-readable loader name for a CurseForge modLoader type ID. */
export function cfLoaderName(modLoader: number): string {
  const found = Object.entries(CF_LOADER_TYPES).find(([, id]) => id === modLoader)
  return found ? found[0] : 'unknown'
}

/** Modrinth-style project type for a CurseForge class ID. */
export function cfProjectType(classId?: number): string {
  const found = Object.entries(CF_CLASS_IDS).find(([, id]) => id === classId)
  return found ? found[0] : 'mod'
}

// ─── Mod files & installation ─────────────────────────────────────────────────

/** A single downloadable file of a CurseForge mod. */
export interface CfModFile {
  id: number
  modId: number
  displayName: string
  fileName: string
  /** 1 = release, 2 = beta, 3 = alpha */
  releaseType: number
  fileDate: string
  /** Null when the author disabled third-party API downloads. */
  downloadUrl: string | null
  /** Mixed list — contains both Minecraft versions and loader names. */
  gameVersions: string[]
  fileLength: number
  downloadCount: number
}

/**
 * List a mod's files, optionally filtered to a Minecraft version and loader.
 * CurseForge returns these newest-first.
 */
export async function getCurseForgeModFiles(
  modId: number | string,
  gameVersion?: string,
  modLoader?: string,
): Promise<CfModFile[] | null> {
  const qs = new URLSearchParams({ pageSize: '50' })
  if (gameVersion) qs.set('gameVersion', gameVersion)

  const loaderType = modLoader ? CF_LOADER_TYPES[modLoader] : undefined
  if (loaderType) qs.set('modLoaderType', String(loaderType))

  const json = await cfFetch<{ data: CfModFile[] }>(`/mods/${modId}/files?${qs}`)
  return json?.data ?? null
}

/**
 * Choose the best file to install from a list — prefers a full release,
 * then falls back to the newest downloadable file of any release type.
 */
export function pickBestCfFile(files: CfModFile[]): CfModFile | null {
  const downloadable = files.filter((f) => f.downloadUrl)
  if (downloadable.length === 0) return null
  return downloadable.find((f) => f.releaseType === 1) ?? downloadable[0]
}

/**
 * From an already-fetched file list, pick the newest downloadable file that
 * is compatible with a given Minecraft version and loader.
 */
export function bestCfFileFor(
  files: CfModFile[],
  gameVersion?: string,
  loader?: string,
): CfModFile | null {
  const matches = files.filter((f) => {
    if (!f.downloadUrl) return false
    if (gameVersion && !f.gameVersions.includes(gameVersion)) return false
    if (loader && loader !== 'vanilla') {
      const hasLoader = f.gameVersions.some((v) => v.toLowerCase() === loader.toLowerCase())
      if (!hasLoader) return false
    }
    return true
  })
  return pickBestCfFile(matches.length > 0 ? matches : files)
}

/** Install one specific CurseForge file into a profile. */
export async function installCurseForgeFile(
  file: CfModFile,
  profilePath: string,
): Promise<void> {
  if (!file.downloadUrl) {
    throw new Error(
      'This file cannot be downloaded — its author disabled third-party downloads on CurseForge.',
    )
  }
  await add_project_from_curseforge(profilePath, file.downloadUrl, file.fileName)
}

/**
 * Install a CurseForge modpack — downloads the pack and creates a new
 * instance from it. Unlike a mod, a modpack is not added to an existing
 * instance.
 *
 * @param file  Optional specific modpack file; otherwise the best is picked.
 * @returns the created profile path
 */
export async function installCurseForgeModpack(
  modId: number,
  modName: string,
  file?: CfModFile,
): Promise<string> {
  let target = file
  if (!target) {
    const files = await getCurseForgeModFiles(modId)
    target = pickBestCfFile(files ?? []) ?? undefined
  }
  if (!target || !target.downloadUrl) {
    throw new Error('No downloadable modpack file is available on CurseForge.')
  }
  return create_profile_and_install_from_curseforge(target.downloadUrl, modName, CF_API_KEY)
}

/**
 * Download and install the best-matching CurseForge file for a mod into a
 * profile. Throws a descriptive error when nothing compatible can be installed.
 *
 * @param modId        CurseForge numeric mod ID
 * @param profilePath  Target instance path
 * @param gameVersion  Instance Minecraft version (narrows file selection)
 * @param loader       Instance loader name (fabric/forge/quilt/neoforge)
 */
export async function installCurseForgeMod(
  modId: number,
  profilePath: string,
  gameVersion?: string,
  loader?: string,
): Promise<void> {
  const files = await getCurseForgeModFiles(modId, gameVersion, loader)

  if (!files || files.length === 0) {
    throw new Error(
      'No CurseForge file is available for this instance’s Minecraft version and loader.',
    )
  }

  const file = pickBestCfFile(files)
  if (!file || !file.downloadUrl) {
    throw new Error(
      'This mod cannot be downloaded — its author disabled third-party downloads on CurseForge.',
    )
  }

  await add_project_from_curseforge(profilePath, file.downloadUrl, file.fileName)
}
