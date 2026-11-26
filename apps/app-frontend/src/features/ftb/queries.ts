import type { Labrinth } from '@modrinth/api-client'
import { defineMessages } from '@modrinth/ui'
import { type QueryClient, queryOptions } from '@tanstack/vue-query'

import { proxiedFetch } from '@/helpers/proxy-fetch'

const ROOT = 'https://api.feed-the-beast.com/v1/modpacks/public/modpack'
const USER_AGENT = 'Noctrinth (+https://github.com/Everelsu/Noctrinth)'
const PACK_REQUEST_CONCURRENCY = 6
const REQUEST_TIMEOUT = 12000
const CACHE_RETENTION = 30 * 60 * 1000

export const ftbMessages = defineMessages({
	noVersion: {
		id: 'noctrinth.ftb.no-versions',
		defaultMessage: 'This modpack has no public versions.',
	},
	loadError: {
		id: 'noctrinth.ftb.load-error',
		defaultMessage: 'This modpack could not be loaded.',
	},
})

interface Target {
	name: string
	version: string
	type: string
}
interface Version {
	id: number
	name: string
	type: string
	private: boolean
	updated: number
	released: number
	targets: Target[]
}
export interface FtbPack {
	id: number
	name: string
	slug: string
	synopsis: string
	description?: string
	private: boolean
	installs: number
	released: number
	updated: number
	art: { type: string; url: string; title?: string }[]
	authors: { name: string }[]
	versions: Version[]
}

async function fetchJson<T>(path: string): Promise<T> {
	const response = await proxiedFetch(ROOT + path, {
		headers: { 'User-Agent': USER_AGENT },
		connectTimeout: 10000,
	})
	if (!response.ok) {
		await response.body?.cancel().catch(() => undefined)
		throw new Error(`FTB HTTP ${response.status}`)
	}
	const result = (await response.json()) as T & { status?: string }
	if (result.status === 'error') throw new Error('FTB could not load the modpack')
	return result
}

async function get<T>(path: string, signal: AbortSignal): Promise<T> {
	const deadline = AbortSignal.any([signal, AbortSignal.timeout(REQUEST_TIMEOUT)])
	deadline.throwIfAborted()
	// Tauri's HTTP JS client retains its abort handler even after consuming the body.
	let onAbort: () => void
	const stopped = new Promise<never>((_, reject) => {
		onAbort = () => reject(deadline.reason)
		deadline.addEventListener('abort', onAbort, { once: true })
	})
	try {
		return await Promise.race([fetchJson<T>(path), stopped])
	} finally {
		deadline.removeEventListener('abort', onAbort!)
	}
}

export function ftbPackQuery(id: number) {
	return queryOptions({
		queryKey: ['ftb', 'pack', id],
		queryFn: ({ signal }) => get<FtbPack>(`/${id}`, signal),
		staleTime: 5 * 60 * 1000,
		gcTime: CACHE_RETENTION,
		retry: false,
	})
}

function ftbListingQuery(path: string) {
	return queryOptions({
		queryKey: ['ftb', 'listing', path],
		queryFn: ({ signal }) => get<{ packs: number[] }>(path, signal),
		staleTime: 5 * 60 * 1000,
		gcTime: CACHE_RETENTION,
		retry: false,
	})
}

export async function ftbSearchPage(
	client: QueryClient,
	{
		query,
		updated,
		offset,
		limit,
		signal,
	}: {
		query: string
		updated: boolean
		offset: number
		limit: number
		signal: AbortSignal
	},
) {
	const path = query
		? `/search/100?term=${encodeURIComponent(query)}`
		: updated
			? '/updated/100'
			: '/all'
	const [listing, recent] = await Promise.all([
		client.fetchQuery(ftbListingQuery(path)),
		query && updated
			? client.fetchQuery(ftbListingQuery('/updated/100')).catch(() => undefined)
			: undefined,
	])
	signal.throwIfAborted()
	const ids = [...new Set(listing.packs)].filter((id) => Number.isSafeInteger(id) && id > 0)
	if (recent) {
		const rank = new Map(recent.packs.map((id, index) => [id, index]))
		ids.sort((a, b) => (rank.get(a) ?? Infinity) - (rank.get(b) ?? Infinity))
	}
	const selected = ids.slice(offset, offset + limit)
	const details = new Array<FtbPack | undefined>(selected.length)
	const failures: unknown[] = []
	const pageSignal = AbortSignal.any([signal, AbortSignal.timeout(REQUEST_TIMEOUT)])
	let cursor = 0
	const pending = Promise.all(
		Array.from({ length: Math.min(PACK_REQUEST_CONCURRENCY, selected.length) }, async () => {
			while (!pageSignal.aborted && cursor < selected.length) {
				const index = cursor++
				try {
					details[index] = await client.ensureQueryData(ftbPackQuery(selected[index]))
				} catch (error) {
					failures.push(error)
				}
			}
		}),
	)
	const stopped = new Promise<void>((resolve) =>
		pageSignal.addEventListener('abort', () => resolve(), { once: true }),
	)
	await Promise.race([pending, stopped])
	signal.throwIfAborted()
	const available = details.filter((pack): pack is FtbPack => !!pack)
	if (available.length === 0) {
		pageSignal.throwIfAborted()
		if (failures.length > 0) throw failures[0]
	}
	return { packs: available, total: ids.length }
}

export function ftbId(projectId: string): number | null {
	const id = /^ftb:(\d+)$/.exec(projectId)?.[1]
	return id && Number.isSafeInteger(Number(id)) && Number(id) > 0 ? Number(id) : null
}

export function ftbVersions(pack: FtbPack) {
	return pack.versions.filter((version) => !version.private).sort((a, b) => b.updated - a.updated)
}

export function ftbProjectPath(id: string | number) {
	return `/project/ftb/${id}`
}

export function ftbHit(
	pack: FtbPack,
): Labrinth.Search.v3.ResultSearchProject & { versions: string[] } {
	const versions = ftbVersions(pack)
	const targets = versions.flatMap((version) => version.targets)
	const gallery = pack.art.filter(({ type }) => type === 'splash').map(({ url }) => url)
	return {
		project_id: `ftb:${pack.id}`,
		slug: pack.slug,
		project_types: ['modpack'],
		all_project_types: ['modpack'],
		name: pack.name,
		summary: pack.synopsis,
		icon_url: pack.art.find(({ type }) => type === 'square')?.url ?? null,
		author: pack.authors[0]?.name ?? 'Feed The Beast',
		author_id: null,
		organization: null,
		organization_id: null,
		downloads: pack.installs,
		follows: 0,
		categories: [],
		display_categories: [],
		gallery,
		featured_gallery: gallery[0] ?? null,
		color: null,
		license: '',
		disclosure_types: [],
		loaders: [
			...new Set(targets.filter(({ type }) => type === 'modloader').map(({ name }) => name)),
		],
		versions: [
			...new Set(targets.filter(({ type }) => type === 'game').map(({ version }) => version)),
		],
		date_created: new Date(pack.released * 1000).toISOString(),
		date_modified: new Date(pack.updated * 1000).toISOString(),
	}
}
