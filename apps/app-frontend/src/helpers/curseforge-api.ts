/**
 * CurseForge REST API client.
 *
 * Requires a CurseForge API key in the VITE_CURSEFORGE_API_KEY environment
 * variable. If the key is absent every function returns null so the rest of
 * the app can treat CurseForge as simply unavailable.
 *
 * Docs: https://docs.curseforge.com/rest-api/
 */

import { useCurseForgeEnabled } from '@/composables/source-mode'
import { proxiedFetch as tauriFetch } from '@/helpers/proxy-fetch'

import { storeCfInstalled } from './cf-installed-store'
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
//
// CurseForge class IDs for Minecraft (gameId 432). Verified against
// `/v1/categories?gameId=432&classesOnly=true` — these are the canonical
// top-level container categories where `isClass=true`.
//
// IMPORTANT: shader=6552 and datapack=6945, NOT the reverse. They were
// swapped previously, which caused the Shaders tab in CF mode to surface
// datapack-compat projects (and vice-versa).

export const CF_CLASS_IDS: Record<string, number> = {
	mod: 6,
	modpack: 4471,
	resourcepack: 12,
	shader: 6552,
	datapack: 6945,
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
	/** Full file objects included in search results — used for filename-based installed detection. */
	latestFiles?: Array<{ id: number; modId: number; fileName: string }>
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
	/** File names from latestFiles — used for filename-based installed detection. */
	_cfFileNames?: string[]
}

// ─── Public API ───────────────────────────────────────────────────────────────

export function isCurseForgeAvailable(): boolean {
	return CF_API_KEY.length > 0 && useCurseForgeEnabled().value
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
	// CurseForge max page size is 50, and it rejects requests where
	// index + pageSize exceeds the cap — clamp to the remaining window.
	const offset = params.offset ?? 0
	const pageSize = Math.min(params.limit ?? 20, 50, CF_MAX_INDEX - offset)

	const qs = new URLSearchParams({
		gameId: '432', // Minecraft
		classId: String(classId),
		pageSize: String(pageSize),
		index: String(offset),
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
	isClass?: boolean
	parentCategoryId?: number
}

let categoriesCache: Promise<CfCategoryInfo[]> | null = null

/**
 * Fetch (and cache) the full CurseForge Minecraft category list.
 *
 * Don't cache failures: `cfFetch` resolves to `null` on error, which would
 * otherwise stick as a permanently-empty list — categories would never appear
 * if the very first call happened to fail. On null/throw we clear the cache
 * so the next caller retries.
 */
export function getCurseForgeCategories(): Promise<CfCategoryInfo[]> {
	if (!categoriesCache) {
		categoriesCache = cfFetch<{ data: CfCategoryInfo[] }>('/categories?gameId=432')
			.then((json) => {
				if (!json) {
					categoriesCache = null
					return []
				}
				return json.data ?? []
			})
			.catch((err) => {
				categoriesCache = null
				throw err
			})
	}
	return categoriesCache
}

/**
 * Resolve a list of category slugs to a single CurseForge category ID.
 * CurseForge's `/mods/search` only accepts one categoryId, so the first
 * slug that maps to a known CurseForge category wins.
 *
 * Sidebar slugs are Modrinth display slugs (see MODRINTH_TO_CF_SLUG below);
 * they are reverse-mapped to a CF slug first. Raw CF slugs are accepted as a
 * fallback so callers passing a CF slug directly still work.
 *
 * `projectType` is critical — many CF slugs (e.g. "magic", "storage") exist
 * in multiple classes (mods, modpacks, …). Without filtering by classId, we
 * may resolve to the wrong class's category, yielding an empty result set.
 */
export async function resolveCfCategoryId(
	slugs: string[],
	projectType?: string,
): Promise<number | undefined> {
	if (!slugs.length) return undefined
	const allCategories = await getCurseForgeCategories()
	const classId = projectType ? CF_CLASS_IDS[projectType] : undefined
	const categories = classId
		? allCategories.filter((c) => c.classId === classId && !c.isClass)
		: allCategories.filter((c) => !c.isClass)

	for (const slug of slugs) {
		// Modrinth display slug path — preferred mapping wins so e.g. "adventure"
		// resolves to CF's "adventure-and-rpg" (bigger pool) instead of CF's
		// smaller "adventure" category. Per-type lookup so the same slug can mean
		// different CF categories across mod / resourcepack / shader / datapack.
		const cfSlug = pickCfSlugForType(slug, projectType)
		let match = cfSlug ? categories.find((c) => c.slug === cfSlug) : undefined
		// Fallback: treat the input as a raw CF slug.
		if (!match) match = categories.find((c) => c.slug === slug)
		if (match) return match.id
	}
	return undefined
}

const CF_REQUEST_ATTEMPTS = 3
const CF_RETRY_BASE_DELAY_MS = 500

function sleep(ms: number): Promise<void> {
	return new Promise((resolve) => setTimeout(resolve, ms))
}

/** Transient statuses worth retrying: rate limit and server-side errors. */
function isRetryableStatus(status: number): boolean {
	return status === 429 || status >= 500
}

/**
 * Shared CurseForge request helper — attaches the API key, retries transient
 * failures (network errors, 429, 5xx) with exponential backoff, and returns
 * parsed JSON (or null on definitive failure / missing key).
 */
async function cfRequest<T>(
	path: string,
	init?: Parameters<typeof tauriFetch>[1],
): Promise<T | null> {
	if (!isCurseForgeAvailable()) return null

	for (let attempt = 1; attempt <= CF_REQUEST_ATTEMPTS; attempt++) {
		const lastAttempt = attempt === CF_REQUEST_ATTEMPTS
		try {
			const res = await tauriFetch(`${CF_BASE}${path}`, {
				...init,
				headers: { 'x-api-key': CF_API_KEY, ...init?.headers },
			})

			if (res.ok) {
				return (await res.json()) as T
			}

			const body = await res.text().catch(() => '(unreadable)')
			if (!isRetryableStatus(res.status) || lastAttempt) {
				console.warn('[CurseForge] Non-OK response:', res.status, path, body)
				return null
			}
			console.warn(
				`[CurseForge] HTTP ${res.status} on ${path}, retrying (${attempt}/${CF_REQUEST_ATTEMPTS})`,
			)
		} catch (err) {
			if (lastAttempt) {
				console.warn('[CurseForge] Request failed:', path, err)
				return null
			}
			console.warn(
				`[CurseForge] Network error on ${path}, retrying (${attempt}/${CF_REQUEST_ATTEMPTS}):`,
				err,
			)
		}
		await sleep(CF_RETRY_BASE_DELAY_MS * 2 ** (attempt - 1))
	}
	return null
}

async function cfFetch<T>(path: string): Promise<T | null> {
	return cfRequest<T>(path)
}

/** POST variant — used by /v1/fingerprints which needs a JSON body. */
async function cfPost<T>(path: string, body: unknown): Promise<T | null> {
	return cfRequest<T>(path, {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify(body),
	})
}

// ─── Fingerprint lookup ──────────────────────────────────────────────────────
//
// CurseForge identifies any file by its Murmur2 fingerprint (computed over
// the file with whitespace stripped — see `compute_cf_fingerprints` on the
// Rust side). POSTing a batch of fingerprints to /v1/fingerprints returns
// which mod/file each one corresponds to. We use this to discover installed
// CurseForge mods without needing to track every install ourselves.

export interface CfFingerprintMatch {
	id: number
	file: {
		id: number
		modId: number
		fileName: string
		fileFingerprint: number
	}
	latestFiles?: unknown[]
}

export interface CfFingerprintMatchesResult {
	isCacheBuilt: boolean
	exactMatches: CfFingerprintMatch[]
	exactFingerprints: number[]
	partialMatches?: CfFingerprintMatch[]
	partialMatchFingerprints?: Record<string, number[]>
	installedFingerprints?: number[]
	unmatchedFingerprints?: number[]
}

/**
 * Bulk-resolve CurseForge file fingerprints to mod/file metadata. CF's
 * `/v1/fingerprints` accepts a list of Murmur2 hashes and returns matches.
 *
 * `exactMatches` is what we want: each entry's `file.modId` tells us
 * the CurseForge project ID for that local jar.
 */
export async function getCurseForgeFingerprintMatches(
	fingerprints: number[],
): Promise<CfFingerprintMatchesResult | null> {
	if (fingerprints.length === 0) {
		return {
			isCacheBuilt: true,
			exactMatches: [],
			exactFingerprints: [],
		}
	}
	// gameId=432 (Minecraft) — the scoped endpoint is what xmcl uses and
	// is the recommended way per CF docs; it lets CF skip cross-game lookups.
	const wrapped = await cfPost<{ data: CfFingerprintMatchesResult }>('/fingerprints/432', {
		fingerprints,
	})
	if (!wrapped) {
		console.warn(`[CurseForge] Fingerprint lookup returned null for ${fingerprints.length} fp(s)`)
		return null
	}
	console.debug(
		`[CurseForge] Fingerprint lookup: ${wrapped.data.exactMatches?.length ?? 0}/${
			fingerprints.length
		} matched`,
	)
	return wrapped.data ?? null
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
		_cfFileNames: mod.latestFiles?.map((f) => f.fileName),
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
export async function getCurseForgeModDescription(modId: number | string): Promise<string | null> {
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

/** CurseForge dependency relation types. */
export const CF_RELATION_TYPE = {
	EmbeddedLibrary: 1,
	OptionalDependency: 2,
	RequiredDependency: 3,
	Tool: 4,
	Incompatible: 5,
	Include: 6,
} as const

/** Legacy alias — keep until call sites are migrated. */
export const CF_RELATION_TYPE_REQUIRED = CF_RELATION_TYPE.RequiredDependency

export interface CfModDependency {
	modId: number
	/** 1=EmbeddedLibrary 2=Optional 3=Required 4=Tool 5=Incompatible 6=Include */
	relationType: number
}

/** Effective dependency category after parent→child type propagation. */
export type CfEffectiveDepType = 'required' | 'optional' | 'incompatible'

export interface CfResolvedDependency {
	modId: number
	type: CfEffectiveDepType
	/** Best matching file for the dep (null for incompatibles / no match). */
	file: CfModFile | null
}

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
	dependencies?: CfModDependency[]
	/** File checksums. algo: 1 = SHA1, 2 = MD5. */
	hashes?: { value: string; algo: number }[]
}

/** SHA1 checksum of a CurseForge file, when the API provides one. */
export function cfFileSha1(file: CfModFile): string | undefined {
	return file.hashes?.find((h) => h.algo === 1)?.value
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
 * Reconstruct a downloadable URL for a CurseForge file when the API hides
 * `downloadUrl` (the author opted out of third-party distribution).
 *
 * The trick is that CurseForge's CDN (`edge.forgecdn.net`) serves files at
 * a predictable path derived from the numeric file ID:
 *   https://edge.forgecdn.net/files/<fileId / 1000>/<fileId % 1000>/<fileName>
 *
 * This URL is publicly served by Cloudflare's CDN even for projects whose
 * authors disabled API distribution — the toggle only hides the URL from
 * the JSON response, not the underlying CDN. PolyMC / MultiMC / Prism etc.
 * all use this same workaround.
 *
 * Returns null when the input doesn't have a usable id/filename.
 */
export function reconstructCfDownloadUrl(file: CfModFile): string | null {
	if (!Number.isInteger(file.id) || file.id < 1000 || !file.fileName) {
		return null
	}
	const head = Math.floor(file.id / 1000)
	const tail = file.id % 1000
	// Encode just the spaces — CF CDN serves filenames with `+`, `'`, `(`, `)`
	// as literals; over-encoding (encodeURIComponent) breaks lookups.
	const safeName = file.fileName.replace(/ /g, '%20')
	return `https://edge.forgecdn.net/files/${head}/${tail}/${safeName}`
}

/** Returns a downloadable URL — either the API-provided one or a reconstructed CDN URL. */
export function effectiveCfDownloadUrl(file: CfModFile): string | null {
	return file.downloadUrl ?? reconstructCfDownloadUrl(file)
}

/**
 * All download URL candidates for a file, in preference order: the
 * API-provided URL first, then the reconstructed CDN URL. The Rust side
 * tries them as mirrors, so a flaky primary doesn't fail the install.
 */
export function cfDownloadMirrors(file: CfModFile): string[] {
	const urls: string[] = []
	if (file.downloadUrl) urls.push(file.downloadUrl)
	const cdn = reconstructCfDownloadUrl(file)
	if (cdn && !urls.includes(cdn)) urls.push(cdn)
	return urls
}

/**
 * Choose the best file to install from a list — prefers a full release,
 * then falls back to the newest downloadable file of any release type.
 *
 * Files with a null `downloadUrl` are still kept as candidates because we
 * can reconstruct the CDN URL (see `reconstructCfDownloadUrl`).
 */
export function pickBestCfFile(files: CfModFile[]): CfModFile | null {
	if (files.length === 0) return null
	return files.find((f) => f.releaseType === 1) ?? files[0]
}

/**
 * From an already-fetched file list, pick the newest downloadable file that
 * is compatible with a given Minecraft version and loader.
 *
 * Returns null if no file matches the constraints — never falls back to an
 * incompatible file (the caller decides what to do on null).
 *
 * Vanilla rule: a file is compatible with a vanilla instance only when it
 * declares no loader at all. Otherwise a Forge-only mod would be considered
 * "compatible" with a vanilla profile, which silently breaks the install.
 */
/**
 * The set of strings CurseForge puts in `gameVersions` that actually denote
 * a *mod loader* (so picking the wrong one breaks the runtime).
 *
 * Other non-numeric tags CF mixes into the same array — "Iris", "OptiFine",
 * "Data Pack", "Resources", "Client", "Server", etc. — are NOT loaders.
 * Treating them as disqualifying is what made vanilla installs of shaders /
 * datapacks / resource packs impossible: their files have "Iris" or
 * "Data Pack" in `gameVersions` but absolutely belong on a vanilla
 * instance.
 */
const CF_REAL_LOADERS = new Set(['forge', 'fabric', 'quilt', 'neoforge', 'cauldron', 'liteloader'])

export function bestCfFileFor(
	files: CfModFile[],
	gameVersion?: string,
	loader?: string,
): CfModFile | null {
	const loaderLower = (loader ?? '').toLowerCase()
	const isVanilla = loaderLower === '' || loaderLower === 'vanilla'

	const matches = files.filter((f) => {
		// Don't filter on downloadUrl — we have a CDN-URL reconstruction
		// fallback (reconstructCfDownloadUrl) so files where the author hid the
		// distribution URL are still installable.
		if (gameVersion && !f.gameVersions.includes(gameVersion)) return false

		// Only consider strings that ARE actually mod loaders. Other CF tags
		// (Iris, OptiFine, Data Pack, Client, Server, …) are informational and
		// must not disqualify a file from a vanilla instance.
		const fileLoaders = f.gameVersions
			.filter((v) => !/^\d/.test(v))
			.map((v) => v.toLowerCase())
			.filter((v) => CF_REAL_LOADERS.has(v))

		if (isVanilla) {
			// Vanilla: only files that don't declare a specific loader qualify.
			return fileLoaders.length === 0
		}
		// Modded: accept this loader's files, or universal files with no loader.
		return fileLoaders.length === 0 || fileLoaders.includes(loaderLower)
	})

	return pickBestCfFile(matches)
}

/**
 * Walk the CurseForge dependency tree of a given file and classify every
 * transitive dep as 'required' / 'optional' / 'incompatible' with proper
 * parent → child type propagation (the XMCL algorithm).
 *
 * Propagation rules:
 *  - `Embedded` (1), `Tool` (4), `Include` (6) are skipped entirely — they
 *    aren't runtime dependencies (they're bundled inside the parent jar or
 *    irrelevant at runtime).
 *  - `Incompatible` (5) is collected separately so the caller can warn the
 *    user about conflicts; its sub-tree is not walked.
 *  - `Required` (3) **stays required** only if every ancestor on the path
 *    was also required. The moment an `Optional` appears in the chain, all
 *    downstream "required" sub-deps become `optional` — because if you
 *    didn't ask for the optional parent, you don't need its requirements.
 *  - `Optional` (2) — collected, walked recursively to find its own deps,
 *    but everything beneath stays soft (auto-installer ignores it).
 *
 * Returns a flat list (root excluded). Order is BFS-ish: closer deps first.
 */
export async function resolveCfDependencies(
	rootFile: CfModFile,
	gameVersion?: string,
	loader?: string,
): Promise<CfResolvedDependency[]> {
	const out: CfResolvedDependency[] = []
	const seen = new Set<number>([rootFile.modId])

	async function visit(parentFile: CfModFile, parentType: CfEffectiveDepType) {
		for (const dep of parentFile.dependencies ?? []) {
			const rt = dep.relationType
			// Non-runtime relationships — skip whole sub-tree.
			if (
				rt === CF_RELATION_TYPE.EmbeddedLibrary ||
				rt === CF_RELATION_TYPE.Tool ||
				rt === CF_RELATION_TYPE.Include
			) {
				continue
			}
			if (seen.has(dep.modId)) continue
			seen.add(dep.modId)

			// Effective type after propagation.
			let effective: CfEffectiveDepType
			if (rt === CF_RELATION_TYPE.Incompatible) {
				effective = 'incompatible'
			} else if (rt === CF_RELATION_TYPE.OptionalDependency) {
				effective = 'optional'
			} else if (rt === CF_RELATION_TYPE.RequiredDependency) {
				// Required-under-Optional softens to Optional — we don't install a
				// hard requirement of something the user hasn't even agreed to.
				effective = parentType === 'required' ? 'required' : 'optional'
			} else {
				continue
			}

			if (effective === 'incompatible') {
				out.push({ modId: dep.modId, type: 'incompatible', file: null })
				continue
			}

			// Fetch the best matching file for the dep — version & loader aware.
			let depFile: CfModFile | null = null
			try {
				const files = await getCurseForgeModFiles(dep.modId, gameVersion, loader)
				if (files) depFile = bestCfFileFor(files, gameVersion, loader)
			} catch (err) {
				console.warn(`[CurseForge] Failed to resolve dep ${dep.modId}:`, err)
			}

			out.push({ modId: dep.modId, type: effective, file: depFile })

			// Recurse — propagate the effective type downward.
			if (depFile) {
				await visit(depFile, effective)
			}
		}
	}

	await visit(rootFile, 'required')
	return out
}

/**
 * Install one specific CurseForge file into a profile and auto-install all
 * transitively-required dependencies. Optional deps are returned so the
 * caller can offer them to the user; incompatible deps are returned so the
 * caller can warn.
 *
 * Behaviour summary:
 *  - `Required` (3) deps under a required chain → auto-installed.
 *  - `Optional` (2), or `Required` under an `Optional` ancestor → returned
 *    in `optional`, never installed silently.
 *  - `Incompatible` (5) → returned in `incompatible` (just mod IDs).
 *  - `Embedded`/`Tool`/`Include` (1/4/6) → ignored entirely.
 *
 * @param installedModIds  Tracks IDs already queued in this call tree to
 *                         prevent infinite loops for circular dependencies.
 */
export async function installCurseForgeFile(
	file: CfModFile,
	profilePath: string,
	gameVersion?: string,
	loader?: string,
	installedModIds: Set<number> = new Set(),
): Promise<{
	optional: CfResolvedDependency[]
	incompatible: CfResolvedDependency[]
}> {
	// If the author disabled API distribution, fall back to the CDN URL we
	// can derive from the numeric file ID (works for the vast majority of
	// such files — same trick MultiMC/PolyMC/Prism use). Both URLs are
	// passed as mirrors and the download is sha1-verified when CF provides
	// a checksum.
	const mirrors = cfDownloadMirrors(file)
	if (mirrors.length === 0) {
		throw new Error(
			`Couldn't build a download URL for "${file.fileName}" — the author disabled distribution and the CDN-fallback couldn't construct a valid URL either. Open the project on CurseForge and download it manually.`,
		)
	}

	installedModIds.add(file.modId)
	await add_project_from_curseforge(profilePath, mirrors, file.fileName, cfFileSha1(file))
	storeCfInstalled(profilePath, file.modId)

	// Top-level pass: only the direct deps of this file. The recursive install
	// below handles the deeper levels (via re-entry into installCurseForgeFile).
	const directOptional: CfResolvedDependency[] = []
	const directIncompatible: CfResolvedDependency[] = []

	for (const dep of file.dependencies ?? []) {
		const rt = dep.relationType
		if (
			rt === CF_RELATION_TYPE.EmbeddedLibrary ||
			rt === CF_RELATION_TYPE.Tool ||
			rt === CF_RELATION_TYPE.Include
		) {
			continue
		}
		if (installedModIds.has(dep.modId)) continue

		if (rt === CF_RELATION_TYPE.Incompatible) {
			directIncompatible.push({ modId: dep.modId, type: 'incompatible', file: null })
			continue
		}

		if (rt === CF_RELATION_TYPE.OptionalDependency) {
			// Surface but don't install silently.
			let optFile: CfModFile | null = null
			try {
				const files = await getCurseForgeModFiles(dep.modId, gameVersion, loader)
				if (files) optFile = bestCfFileFor(files, gameVersion, loader)
			} catch (err) {
				console.warn(`[CurseForge] Failed to resolve optional dep ${dep.modId}:`, err)
			}
			directOptional.push({ modId: dep.modId, type: 'optional', file: optFile })
			continue
		}

		if (rt === CF_RELATION_TYPE.RequiredDependency) {
			// Install required. The recursive call propagates the same logic so a
			// transitive `Optional` doesn't drag in its own `Required` automatically.
			await installCurseForgeMod(
				dep.modId,
				profilePath,
				gameVersion,
				loader,
				installedModIds,
			).catch((err) => console.warn(`[CurseForge] Skipping required dependency ${dep.modId}:`, err))
		}
	}

	return { optional: directOptional, incompatible: directIncompatible }
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
	if (!target) {
		throw new Error('No modpack file is available on CurseForge.')
	}
	const url = effectiveCfDownloadUrl(target)
	if (!url) {
		throw new Error(
			`Couldn't build a download URL for "${target.fileName}" — open the project on CurseForge to download it manually.`,
		)
	}
	return create_profile_and_install_from_curseforge(url, modName, CF_API_KEY)
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
	installedModIds: Set<number> = new Set(),
): Promise<void> {
	const files = await getCurseForgeModFiles(modId, gameVersion, loader)

	if (!files || files.length === 0) {
		throw new Error(`This mod has no files on CurseForge for any Minecraft version.`)
	}

	// Apply local game-version + loader constraint. bestCfFileFor keeps files
	// even when downloadUrl is null (we have a CDN reconstruction fallback).
	const file = bestCfFileFor(files, gameVersion, loader)
	if (!file) {
		// Build a useful error: list the unique MC versions the mod actually
		// supports so the user knows what's available.
		const allVersions = Array.from(
			new Set(files.flatMap((f) => f.gameVersions.filter((v) => /^\d/.test(v)))),
		).sort()
		const versionList =
			allVersions.length > 0
				? `Available Minecraft versions: ${allVersions.slice(0, 8).join(', ')}${allVersions.length > 8 ? '…' : ''}.`
				: 'No matching versions were returned by CurseForge.'
		const target =
			gameVersion && loader
				? `Minecraft ${gameVersion} (${loader})`
				: gameVersion
					? `Minecraft ${gameVersion}`
					: `loader "${loader}"`
		throw new Error(`No CurseForge file fits ${target}. ${versionList}`)
	}

	await installCurseForgeFile(file, profilePath, gameVersion, loader, installedModIds)
}

// ─── Category tag helpers ─────────────────────────────────────────────────────

/**
 * Modrinth-style mod category list, each mapped to a CurseForge category slug
 * that the CF `/mods/search` API actually returns mods for.
 *
 * The CF-mode sidebar is built from THIS list (not from CF's own taxonomy) so
 * the labels and icons are the familiar Modrinth ones. The reverse mapping
 * below feeds `resolveCfCategoryId` so filter clicks resolve to the right CF
 * category ID — using the project type's classId to disambiguate slugs that
 * exist in multiple CF classes (e.g. "magic" in both mods and modpacks).
 *
 * Modrinth categories that have no on-topic CF equivalent are intentionally
 * omitted rather than mapped to something broad — better to hide a filter
 * than mislead the user with off-topic results.
 */
const MODRINTH_TO_CF_MOD_CATEGORY: Record<string, string> = {
	// ── Strong 1:1 matches — CF target is well-populated and on-topic ────────
	cursed: 'horror',
	decoration: 'cosmetic',
	equipment: 'armor-weapons-and-tools',
	food: 'food',
	library: 'library-api',
	magic: 'magic',
	management: 'server-utility',
	mobs: 'mobs',
	optimization: 'performance',
	storage: 'storage',
	technology: 'technology',
	transportation: 'player-transport',
	utility: 'utility-qol',
	worldgen: 'world-gen',
	// ── Intentionally omitted (no clean CF equivalent): ──────────────────────
	//   adventure, quests           → CF's "adventure-and-rpg" and "quests"
	//                                  contain off-topic mods that mislead users
	//   game-mechanics, minigame    → no CF analogue
	//   social, economy             → no CF analogue
}

/**
 * Resource pack categories. CurseForge groups them by theme + resolution.
 * Modrinth slugs on the left are the canonical names we render in the
 * sidebar (have icon + i18n).
 */
const MODRINTH_TO_CF_RESOURCEPACK_CATEGORY: Record<string, string> = {
	// Themed packs
	decoration: 'miscellaneous',
	fantasy: 'medieval',
	realistic: 'photo-realistic',
	'semi-realistic': 'photo-realistic',
	cartoon: 'animated',
	'vanilla-like': 'traditional',
	themed: 'modern',
	// Content-focused
	blocks: 'mod-support',
	combat: 'mod-support',
	entities: 'miscellaneous',
	font: 'font-packs',
	fonts: 'font-packs',
	gui: 'miscellaneous',
}

/**
 * Shader pack categories. CF shaders class (classId=6552) is much smaller
 * than Modrinth's — we map a handful of well-populated visual styles.
 */
const MODRINTH_TO_CF_SHADER_CATEGORY: Record<string, string> = {
	realistic: 'realistic',
	fantasy: 'fantasy',
	cartoon: 'cartoon',
	'vanilla-like': 'vanilla-like',
	cursed: 'horror',
	potato: 'potato',
}

/**
 * Datapack categories. CF datapacks class (classId=6945).
 */
const MODRINTH_TO_CF_DATAPACK_CATEGORY: Record<string, string> = {
	adventure: 'adventure',
	magic: 'magic',
	mobs: 'mobs',
	food: 'food',
	utility: 'utility',
	decoration: 'cosmetic',
	storage: 'storage',
	technology: 'technology',
	worldgen: 'world-gen',
	cursed: 'horror',
	optimization: 'performance',
	library: 'library',
}

/**
 * Per-project-type Modrinth→CF category mappings. Each project type maps the
 * Modrinth slugs we surface in the CF-mode sidebar to the CF category slugs
 * the search API actually filters on. Validated per-class by
 * `mapCfCategoriesToTags` so a mapping that's missing on CF doesn't appear.
 */
const MODRINTH_TO_CF_BY_TYPE: Record<string, Record<string, string>> = {
	mod: MODRINTH_TO_CF_MOD_CATEGORY,
	resourcepack: MODRINTH_TO_CF_RESOURCEPACK_CATEGORY,
	shader: MODRINTH_TO_CF_SHADER_CATEGORY,
	datapack: MODRINTH_TO_CF_DATAPACK_CATEGORY,
}

/**
 * Reverse lookup used by resolveCfCategoryId. The same Modrinth slug can map
 * to different CF slugs across project types (e.g. `decoration` → `cosmetic`
 * for mods but → `miscellaneous` for resource packs), so we resolve per-type
 * first and fall back to a flat union for callers that don't pass a type.
 */
const MODRINTH_TO_CF_SLUG: Record<string, string> = (() => {
	const out: Record<string, string> = {}
	for (const map of Object.values(MODRINTH_TO_CF_BY_TYPE)) {
		for (const [mrSlug, cfSlug] of Object.entries(map)) {
			if (!(mrSlug in out)) out[mrSlug] = cfSlug
		}
	}
	return out
})()

function pickCfSlugForType(slug: string, projectType?: string): string | undefined {
	if (projectType) {
		const typed = MODRINTH_TO_CF_BY_TYPE[projectType]?.[slug]
		if (typed) return typed
	}
	return MODRINTH_TO_CF_SLUG[slug]
}

/**
 * Build the BrowseSidebar category list for CurseForge mode.
 *
 * Returns Modrinth-style category entries (one per Modrinth slug in
 * MODRINTH_TO_CF_BY_TYPE) so the sidebar shows the familiar Modrinth labels,
 * icons, and order. The CF response is only used to validate that each
 * target CF category still exists — defensive against CF removing categories.
 *
 * The `name` field is what BrowseSidebar feeds to `formatCategory` (translated
 * label), `getCategoryIcon` (icon lookup), and the `categories:…` filter
 * value; setting it to the Modrinth slug makes all three work transparently.
 */
export function mapCfCategoriesToTags(
	cfCats: CfCategoryInfo[],
): Array<{ icon: string; name: string; project_type: string; header: string }> {
	// Class-scoped slug index: a slug may exist in multiple classes (e.g. "magic"
	// for both mods and modpacks), so validation must be per-class.
	const slugsByClass = new Map<number, Set<string>>()
	for (const c of cfCats) {
		if (c.isClass || c.classId == null) continue
		let set = slugsByClass.get(c.classId)
		if (!set) {
			set = new Set()
			slugsByClass.set(c.classId, set)
		}
		set.add(c.slug)
	}

	const out: Array<{ icon: string; name: string; project_type: string; header: string }> = []

	for (const [projectType, map] of Object.entries(MODRINTH_TO_CF_BY_TYPE)) {
		const classId = CF_CLASS_IDS[projectType]
		const validSlugs = classId != null ? slugsByClass.get(classId) : undefined
		if (!validSlugs) continue
		for (const [mrSlug, cfSlug] of Object.entries(map)) {
			// Defensive: skip entries whose CF target isn't in this class.
			if (!validSlugs.has(cfSlug)) continue
			out.push({
				icon: '',
				name: mrSlug,
				project_type: projectType,
				header: 'category',
			})
		}
	}

	return out
}
