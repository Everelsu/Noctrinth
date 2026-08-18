/**
 * Noctrinth changelog.
 *
 * Entries live as markdown in `src/changelog/<version>.md` rather than in this
 * file, so writing one is editing a document instead of editing code. Each
 * needs a `date` in its front matter; the version is the file name.
 *
 * Screenshots go in `src/changelog/screenshots/` and are referenced from the
 * root of the changelog site, for
 * example `![Modern Java](/changelog/modern-java.png)`. They are published with
 * the changelog site and resolved against it at render time, so a screenshot
 * costs the download once for whoever scrolls to it rather than sitting in
 * every installer from then on.
 *
 * The body is markdown with `### Added`, `### Changed`, `### Deprecated`,
 * `### Removed`, `### Fixed` and `### Security` headings (the Keep a Changelog
 * convention that Modrinth's own PR template follows) and `- item` bullets.
 *
 * The entries bundled here are the offline copy. The same files are published
 * as `changelog.json` on the changelog site, and `refreshNoctrinthChangelog`
 * pulls that in so an installed build can show releases written after it — and
 * so a corrected entry does not need a release to reach anyone.
 *
 * This file is Noctrinth-specific and deliberately sits outside
 * `packages/blog`, so a sync with upstream Modrinth never overwrites it.
 */
import { proxiedFetch } from './proxy-fetch'

export interface NoctrinthVersionEntry {
	version: string
	/** ISO date string. */
	date: string
	body: string
}

const entryFiles = import.meta.glob('../changelog/*.md', {
	query: '?raw',
	import: 'default',
	eager: true,
}) as Record<string, string>

const FRONT_MATTER = /^---\r?\n([\s\S]*?)\r?\n---\r?\n?/

function parseEntry(path: string, raw: string): NoctrinthVersionEntry | null {
	const version = path.split('/').pop()?.replace(/\.md$/, '')
	// Entries are named after their version; the folder's README is not one.
	if (!version || !/^[0-9]/.test(version)) return null

	const match = FRONT_MATTER.exec(raw)
	const date = match ? (/^date:\s*(.+)$/m.exec(match[1])?.[1]?.trim() ?? '') : ''

	if (!date) {
		console.warn(`Changelog entry ${path} has no date in its front matter`)
		return null
	}

	return { version, date, body: raw.slice(match?.[0].length ?? 0).trim() }
}

/** Compares two dotted versions numerically, so 0.9 sorts below 0.10. */
function compareVersions(left: string, right: string): number {
	const parts = (value: string) => value.split(/[.-]/).map((part) => Number(part))
	const a = parts(left)
	const b = parts(right)

	for (let i = 0; i < Math.max(a.length, b.length); i++) {
		const x = a[i]
		const y = b[i]
		// A pre-release suffix leaves a NaN, which sorts below the plain release.
		if (Number.isNaN(x) || x === undefined) return Number.isNaN(y) || y === undefined ? 0 : -1
		if (Number.isNaN(y) || y === undefined) return 1
		if (x !== y) return x - y
	}

	return 0
}

/** Newest first, with same-day releases ordered by version. */
function sortEntries(a: NoctrinthVersionEntry, b: NoctrinthVersionEntry): number {
	const byDate = new Date(b.date).getTime() - new Date(a.date).getTime()
	return byDate !== 0 ? byDate : compareVersions(b.version, a.version)
}

const bundledEntries: NoctrinthVersionEntry[] = Object.entries(entryFiles)
	.map(([path, raw]) => parseEntry(path, raw))
	.filter((entry): entry is NoctrinthVersionEntry => entry !== null)
	.sort(sortEntries)

/** The feed the changelog site publishes next to its index page. */
const FEED_URL = 'https://everelsu.github.io/Noctrinth/changelog.json'
const CACHE_KEY = 'noctrinth-changelog-feed'

/**
 * What the cache is allowed to cost.
 *
 * The feed is plain text and the whole history currently weighs under 40 KB,
 * so these are ceilings against a broken or hostile feed rather than limits
 * anyone is expected to reach. Screenshots are never cached here: entries
 * reference them by URL and the webview fetches them only when they scroll
 * into view.
 */
const MAX_FEED_BYTES = 512 * 1024
const MAX_ENTRIES = 200
const MAX_BODY_CHARS = 32 * 1024

/** How long a fetched feed is trusted before the site is asked again. */
const REFRESH_AFTER_MS = 30 * 60 * 1000

interface CachedFeed {
	etag?: string
	fetchedAt: number
	entries: NoctrinthVersionEntry[]
}

function isEntry(value: unknown): value is NoctrinthVersionEntry {
	const entry = value as Partial<NoctrinthVersionEntry> | null
	return (
		!!entry &&
		typeof entry.version === 'string' &&
		entry.version.length > 0 &&
		entry.version.length < 64 &&
		typeof entry.date === 'string' &&
		!Number.isNaN(new Date(entry.date).getTime()) &&
		typeof entry.body === 'string' &&
		entry.body.length <= MAX_BODY_CHARS
	)
}

function parseFeed(payload: unknown): NoctrinthVersionEntry[] | null {
	const list = (payload as { entries?: unknown })?.entries
	if (!Array.isArray(list)) {
		return null
	}

	return list
		.filter(isEntry)
		.slice(0, MAX_ENTRIES)
		.map(({ version, date, body }) => ({ version, date, body }))
}

function readCache(): CachedFeed | null {
	try {
		const raw = localStorage.getItem(CACHE_KEY)
		if (!raw) {
			return null
		}

		const cached = JSON.parse(raw) as Partial<CachedFeed>
		const entries = Array.isArray(cached.entries) ? cached.entries.filter(isEntry) : []
		if (!entries.length) {
			return null
		}

		return { etag: cached.etag, fetchedAt: cached.fetchedAt ?? 0, entries }
	} catch {
		return null
	}
}

function writeCache(feed: CachedFeed): void {
	try {
		const raw = JSON.stringify(feed)
		// Better to refetch next time than to leave something oversized parked
		// in storage for the life of the install.
		if (raw.length > MAX_FEED_BYTES) {
			localStorage.removeItem(CACHE_KEY)
			return
		}
		localStorage.setItem(CACHE_KEY, raw)
	} catch (error) {
		console.warn('Failed to cache the changelog feed:', error)
	}
}

let cache = readCache()
let inFlight: Promise<boolean> | null = null

/**
 * Bundled entries, overlaid with anything the site has said since.
 *
 * A version present in both takes the fetched copy: it is the same entry, only
 * newer. Versions the site knows and this build does not are kept, which is
 * what lets the changelog describe an update before it is installed.
 */
function merge(): NoctrinthVersionEntry[] {
	const byVersion = new Map(bundledEntries.map((entry) => [entry.version, entry]))
	for (const entry of cache?.entries ?? []) {
		byVersion.set(entry.version, entry)
	}

	return [...byVersion.values()].sort(sortEntries)
}

let merged: NoctrinthVersionEntry[] = merge()

export function getNoctrinthChangelog(): NoctrinthVersionEntry[] {
	return merged
}

/**
 * Asks the site for the changelog, and reports whether anything changed.
 *
 * Failures are silent by design: the bundled entries are already on screen,
 * and a launcher that cannot reach GitHub has nothing to say about it here.
 */
export async function refreshNoctrinthChangelog(force = false): Promise<boolean> {
	if (inFlight) {
		return inFlight
	}

	if (!force && cache && Date.now() - cache.fetchedAt < REFRESH_AFTER_MS) {
		return false
	}

	inFlight = (async () => {
		try {
			const response = await proxiedFetch(FEED_URL, {
				headers: cache?.etag ? { 'If-None-Match': cache.etag } : undefined,
			})

			if (response.status === 304) {
				if (cache) {
					cache = { ...cache, fetchedAt: Date.now() }
					writeCache(cache)
				}
				return false
			}

			if (!response.ok) {
				return false
			}

			const entries = parseFeed(await response.json())
			if (!entries?.length) {
				return false
			}

			cache = {
				etag: response.headers.get('etag') ?? undefined,
				fetchedAt: Date.now(),
				entries,
			}
			writeCache(cache)
			merged = merge()
			return true
		} catch (error) {
			console.warn('Failed to fetch the changelog feed:', error)
			return false
		} finally {
			inFlight = null
		}
	})()

	return inFlight
}
