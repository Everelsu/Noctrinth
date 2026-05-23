/**
 * Unified search — merges Modrinth and CurseForge results into a single list.
 *
 * Deduplication strategy:
 *   1. Exact slug match   (e.g. "sodium" on both platforms)
 *   2. Case-insensitive name match  (fallback)
 *
 * Mods that appear on both platforms receive a `sources` object with entries
 * for both. Exclusive mods get a single-entry `sources` object.
 */

import type { Labrinth } from '@modrinth/api-client'
import type { BrowseSearchResponse } from '@modrinth/ui'

import { mapCfMod, resolveCfCategoryId, searchCurseForge } from './curseforge-api'

/** Structured filters extracted from the browse sidebar, for the CF query. */
export interface CfFilters {
  gameVersion?: string
  modLoader?: string
  categories?: string[]
}

// ─── Extended hit type ────────────────────────────────────────────────────────

export interface ModSources {
  modrinth?: { project_id: string; slug: string }
  curseforge?: { mod_id: number; slug: string }
}

export type UnifiedHit = Labrinth.Search.v2.ResultSearchProject & {
  installed?: boolean
  installing?: boolean
  /** Which platforms carry this mod. Present on all hits after unified search. */
  sources?: ModSources
  /** CurseForge numeric mod ID, set when CF data is present. */
  cf_id?: number
}

export type UnifiedBrowseSearchResponse = BrowseSearchResponse & {
  projectHits: UnifiedHit[]
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

/**
 * Parse the plain query params (query / limit / offset / sort) from the
 * request string. Filters come separately as structured `CfFilters` — far
 * more reliable than regex-scraping Modrinth's filter DSL.
 */
function parseRequestParams(params: string): {
  query: string
  limit: number
  offset: number
  sortBy: string
} {
  const p = new URLSearchParams(params)
  return {
    query: p.get('query') ?? '',
    limit: parseInt(p.get('limit') ?? '20', 10),
    offset: parseInt(p.get('offset') ?? '0', 10),
    sortBy: p.get('index') ?? 'relevance',
  }
}

/** Numeric sort key for a hit, by sort type (downloads/follows/date). */
function sortValue(hit: UnifiedHit, sortBy: string): number {
  switch (sortBy) {
    case 'downloads':
      return hit.downloads ?? 0
    case 'follows':
      return hit.follows ?? 0
    case 'newest':
      return new Date(hit.date_created ?? 0).getTime()
    case 'updated':
      return new Date(hit.date_modified ?? 0).getTime()
    default:
      return 0
  }
}

// ─── Main export ──────────────────────────────────────────────────────────────

/**
 * Run Modrinth and CurseForge searches in parallel, then merge the results.
 *
 * @param requestParams  URL query string produced by `useSearch`
 * @param projectType    Current project type (mod, modpack, …)
 * @param modrinthSearch The original `search()` function from Browse.vue
 * @param sourceMode     Restrict to a single catalog. `undefined` = both
 *                       (used for text search); `'modrinth'` / `'curseforge'`
 *                       restrict to one catalog (used for browse-mode toggle).
 * @param cfFilters      Structured sidebar filters (game version, loader,
 *                       categories) applied to the CurseForge query.
 */
export async function unifiedSearch(
  requestParams: string,
  projectType: string,
  modrinthSearch: (params: string) => Promise<BrowseSearchResponse>,
  sourceMode?: 'modrinth' | 'curseforge',
  cfFilters?: CfFilters,
): Promise<UnifiedBrowseSearchResponse> {
  const parsedParams = parseRequestParams(requestParams)

  const wantModrinth = sourceMode !== 'curseforge'
  const wantCurseForge = sourceMode !== 'modrinth'

  // A search query with the default (relevance) sort is instead ordered by
  // downloads — popular results read better. The sort dropdown still shows
  // "Relevance"; only the underlying ordering changes.
  const orderByDownloads = !!parsedParams.query && parsedParams.sortBy === 'relevance'
  const effectiveSortBy = orderByDownloads ? 'downloads' : parsedParams.sortBy

  let modrinthParams = requestParams
  if (orderByDownloads) {
    const params = new URLSearchParams(requestParams)
    params.set('index', 'downloads')
    modrinthParams = `?${params.toString()}`
  }

  // Resolve category slug → CurseForge category ID, then run the CF query.
  const curseForgeQuery = async () => {
    const categoryId = cfFilters?.categories?.length
      ? await resolveCfCategoryId(cfFilters.categories)
      : undefined
    return searchCurseForge({
      ...parsedParams,
      sortBy: effectiveSortBy,
      projectType,
      gameVersion: cfFilters?.gameVersion,
      modLoader: cfFilters?.modLoader,
      categoryId,
    })
  }

  // Run the requested APIs in parallel — neither failure should block the other
  const [mrSettled, cfSettled] = await Promise.allSettled([
    wantModrinth ? modrinthSearch(modrinthParams) : Promise.resolve(null),
    wantCurseForge ? curseForgeQuery() : Promise.resolve(null),
  ])

  const mr = mrSettled.status === 'fulfilled' ? mrSettled.value : null
  const cf = cfSettled.status === 'fulfilled' ? cfSettled.value : null

  if (cfSettled.status === 'rejected') {
    console.warn('[UnifiedSearch] CurseForge search failed:', cfSettled.reason)
  }

  // ── Fallback: only one source available ──────────────────────────────────

  if (!mr && !cf) {
    return { projectHits: [], serverHits: [], total_hits: 0, per_page: 20 }
  }

  if (!cf || !cf.data?.length) {
    // Tag every Modrinth hit with a modrinth-only source marker
    const hits: UnifiedHit[] = (mr?.projectHits ?? []).map((h) => ({
      ...h,
      sources: { modrinth: { project_id: h.project_id, slug: h.slug } },
    }))
    return { ...(mr ?? { serverHits: [], total_hits: 0, per_page: 20 }), projectHits: hits }
  }

  if (!mr) {
    const hits: UnifiedHit[] = cf.data.map((mod) => mapCfMod(mod) as unknown as UnifiedHit)
    return {
      projectHits: hits,
      serverHits: [],
      total_hits: cf.pagination.totalCount,
      per_page: cf.pagination.pageSize,
    }
  }

  // ── Merge & deduplicate ───────────────────────────────────────────────────

  const mrHits = mr.projectHits as UnifiedHit[]
  const cfMapped = cf.data.map(mapCfMod)

  // Build lookup maps so matching is O(n)
  const cfBySlug = new Map(cfMapped.map((h) => [h.slug, h]))
  const cfByName = new Map(cfMapped.map((h) => [h.name.toLowerCase(), h]))
  const matchedCfSlugs = new Set<string>()

  const unified: UnifiedHit[] = []

  for (const mrHit of mrHits) {
    const mrName = (mrHit.title ?? (mrHit as { name?: string }).name ?? '').toLowerCase()
    const cfMatch = cfBySlug.get(mrHit.slug) ?? cfByName.get(mrName)

    const hit: UnifiedHit = {
      ...mrHit,
      sources: { modrinth: { project_id: mrHit.project_id, slug: mrHit.slug } },
    }

    if (cfMatch) {
      hit.sources = {
        modrinth: { project_id: mrHit.project_id, slug: mrHit.slug },
        curseforge: { mod_id: cfMatch.cf_id, slug: cfMatch.slug },
      }
      hit.cf_id = cfMatch.cf_id
      // Combined download count across both platforms for a truthful total.
      hit.downloads = (mrHit.downloads ?? 0) + (cfMatch.downloads ?? 0)
      matchedCfSlugs.add(cfMatch.slug)
    }

    unified.push(hit)
  }

  // Append CurseForge-exclusive mods (not matched above)
  for (const cfHit of cfMapped) {
    if (matchedCfSlugs.has(cfHit.slug)) continue
    unified.push({
      ...(cfHit as unknown as UnifiedHit),
      sources: { curseforge: { mod_id: cfHit.cf_id, slug: cfHit.slug } },
    })
  }

  // For objective sorts, re-order the merged page so the combined download
  // counts and CurseForge-exclusive results land in the right place (the raw
  // merge is Modrinth-first, which would otherwise mis-rank them).
  if (['downloads', 'follows', 'newest', 'updated'].includes(effectiveSortBy)) {
    unified.sort((a, b) => sortValue(b, effectiveSortBy) - sortValue(a, effectiveSortBy))
  }

  return {
    projectHits: unified,
    serverHits: mr.serverHits,
    // Pagination follows Modrinth — it is the primary, paginated source.
    // CurseForge results enrich each page (badges + exclusive mods) but do
    // not drive the page count, which keeps paging predictable.
    total_hits: mr.total_hits,
    per_page: mr.per_page,
  }
}
