/**
 * Catalog search router — runs a Browse search against exactly one catalog
 * (Modrinth or CurseForge), selected by the source toggle. The two catalogs
 * are deliberately separate modules: Modrinth results come from the regular
 * Labrinth search, CurseForge results from `curseforge-api.ts`, and they are
 * never merged into one list. Every hit still carries a `sources` marker so
 * install routing and cross-platform installed-detection keep working.
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
	/** Which platform carries this mod. Present on all hits after a search. */
	sources?: ModSources
	/** CurseForge numeric mod ID, set when CF data is present. */
	cf_id?: number
	/** Latest file names from CurseForge — used for filename-based installed detection. */
	_cfFileNames?: string[]
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

const EMPTY_RESPONSE: UnifiedBrowseSearchResponse = {
	projectHits: [],
	serverHits: [],
	total_hits: 0,
	per_page: 20,
}

// ─── Main export ──────────────────────────────────────────────────────────────

/**
 * Run a search against the selected catalog.
 *
 * @param requestParams  URL query string produced by `useSearch`
 * @param projectType    Current project type (mod, modpack, …)
 * @param modrinthSearch The original `search()` function from Browse.vue
 * @param sourceMode     Which catalog to query.
 * @param cfFilters      Structured sidebar filters (game version, loader,
 *                       categories) applied to the CurseForge query.
 */
export async function unifiedSearch(
	requestParams: string,
	projectType: string,
	modrinthSearch: (params: string) => Promise<BrowseSearchResponse>,
	sourceMode: 'modrinth' | 'curseforge',
	cfFilters?: CfFilters,
): Promise<UnifiedBrowseSearchResponse> {
	const parsedParams = parseRequestParams(requestParams)

	// A search query with the default (relevance) sort is instead ordered by
	// downloads — popular results read better. The sort dropdown still shows
	// "Relevance"; only the underlying ordering changes.
	const orderByDownloads = !!parsedParams.query && parsedParams.sortBy === 'relevance'
	const effectiveSortBy = orderByDownloads ? 'downloads' : parsedParams.sortBy

	if (sourceMode === 'curseforge') {
		const categoryId = cfFilters?.categories?.length
			? await resolveCfCategoryId(cfFilters.categories, projectType)
			: undefined
		const cf = await searchCurseForge({
			...parsedParams,
			sortBy: effectiveSortBy,
			projectType,
			gameVersion: cfFilters?.gameVersion,
			modLoader: cfFilters?.modLoader,
			categoryId,
		})
		if (!cf) return EMPTY_RESPONSE
		return {
			projectHits: cf.data.map((mod) => mapCfMod(mod) as unknown as UnifiedHit),
			serverHits: [],
			total_hits: cf.pagination.totalCount,
			per_page: cf.pagination.pageSize,
		}
	}

	let modrinthParams = requestParams
	if (orderByDownloads) {
		const params = new URLSearchParams(requestParams)
		params.set('index', 'downloads')
		modrinthParams = `?${params.toString()}`
	}

	const mr = await modrinthSearch(modrinthParams)
	const hits: UnifiedHit[] = (mr?.projectHits ?? []).map((h) => ({
		...h,
		sources: { modrinth: { project_id: h.project_id, slug: h.slug } },
	}))
	return { ...(mr ?? EMPTY_RESPONSE), projectHits: hits }
}
