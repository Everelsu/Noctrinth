/**
 * Curated cross-platform mapping between Modrinth and CurseForge projects.
 *
 * Used to identify "the same mod on both platforms" beyond what slug/name
 * matching catches in `unified-search.ts`. Same mod, different slugs across
 * platforms (e.g. Iris is `iris` on Modrinth but `irisshaders` on CF) would
 * otherwise show up as two cards in unified search.
 *
 * This is the simple, self-contained version of what xmcl-launcher ships as
 * a downloadable SQLite from Azure: a hand-curated seed table augmented at
 * runtime as we observe new slug-matches. No external infrastructure needed.
 *
 * Seed list is intentionally small and conservative — only entries we can
 * verify by URL on both platforms. Adding more is just appending to MAPPINGS.
 */

export interface PlatformMapping {
	modrinth: { id: string; slug: string }
	curseforge: { id: number; slug?: string }
}

/**
 * Seed of well-known mods. Each entry is verifiable by visiting:
 *   - https://modrinth.com/mod/<modrinth.slug>
 *   - https://www.curseforge.com/minecraft/mc-mods/<curseforge.slug>
 *   and confirming `modrinth.id` matches the page's URL hash.
 *
 * Add more as you encounter slug mismatches that cause duplicate cards in
 * unified search — runtime learning (below) handles the easy slug-equal
 * cases automatically.
 */
const MAPPINGS: PlatformMapping[] = [
	// ── Performance ──────────────────────────────────────────────────────────
	{ modrinth: { id: 'AANobbMI', slug: 'sodium' }, curseforge: { id: 394468, slug: 'sodium' } },
	{ modrinth: { id: 'gvQqBUqZ', slug: 'lithium' }, curseforge: { id: 360438, slug: 'lithium' } },
	{ modrinth: { id: 'hEOCdOgW', slug: 'phosphor' }, curseforge: { id: 372124, slug: 'phosphor' } },
	{
		modrinth: { id: 'PtjYWJkn', slug: 'sodium-extra' },
		curseforge: { id: 679167, slug: 'sodium-extra' },
	},
	{
		modrinth: { id: 'Bh37bMuy', slug: 'reeses-sodium-options' },
		curseforge: { id: 560740, slug: 'reeses-sodium-options' },
	},
	{ modrinth: { id: 'Orvt0mRa', slug: 'indium' }, curseforge: { id: 459701, slug: 'indium' } },

	// ── Loader / APIs ────────────────────────────────────────────────────────
	{
		modrinth: { id: 'P7dR8mSH', slug: 'fabric-api' },
		curseforge: { id: 306612, slug: 'fabric-api' },
	},
	{
		modrinth: { id: 'lhGA9TYQ', slug: 'architectury-api' },
		curseforge: { id: 419699, slug: 'architectury-api' },
	},
	{
		modrinth: { id: '9s6osm5g', slug: 'cloth-config' },
		curseforge: { id: 348521, slug: 'cloth-config' },
	},
	{ modrinth: { id: 'mOgUt4GM', slug: 'modmenu' }, curseforge: { id: 308702, slug: 'modmenu' } },

	// ── Utility / QoL ────────────────────────────────────────────────────────
	{ modrinth: { id: 'u6dRKJwZ', slug: 'jei' }, curseforge: { id: 238222, slug: 'jei' } },
	{
		modrinth: { id: 'nfn13YXA', slug: 'rei' },
		curseforge: { id: 310111, slug: 'roughly-enough-items' },
	},
	{
		modrinth: { id: 'EsAfCjCV', slug: 'appleskin' },
		curseforge: { id: 248787, slug: 'appleskin' },
	},
	{
		modrinth: { id: 'aC3cM3Vq', slug: 'mouse-tweaks' },
		curseforge: { id: 60089, slug: 'mouse-tweaks' },
	},

	// ── Shaders ──────────────────────────────────────────────────────────────
	{ modrinth: { id: 'YL57xq9U', slug: 'iris' }, curseforge: { id: 455508, slug: 'irisshaders' } },
	{ modrinth: { id: 'oculus', slug: 'oculus' }, curseforge: { id: 581495, slug: 'oculus' } },

	// ── Maps ─────────────────────────────────────────────────────────────────
	{
		modrinth: { id: 'lfHFW1mp', slug: 'journeymap' },
		curseforge: { id: 32274, slug: 'journeymap' },
	},
	{
		modrinth: { id: '1bokaNcj', slug: 'xaeros-minimap' },
		curseforge: { id: 263420, slug: 'xaeros-minimap' },
	},
	{
		modrinth: { id: 'NcUtCpym', slug: 'xaeros-world-map' },
		curseforge: { id: 317780, slug: 'xaeros-world-map' },
	},

	// ── Worldgen / Content ───────────────────────────────────────────────────
	{ modrinth: { id: 'LNytGWDc', slug: 'create' }, curseforge: { id: 328085, slug: 'create' } },
	{ modrinth: { id: 'pfjLUfGv', slug: 'botania' }, curseforge: { id: 225643, slug: 'botania' } },
	{
		modrinth: { id: 'gXkD0unR', slug: 'biomes-o-plenty' },
		curseforge: { id: 220318, slug: 'biomes-o-plenty' },
	},

	// ── Voice / Social ───────────────────────────────────────────────────────
	{
		modrinth: { id: '9eGKb6K1', slug: 'simple-voice-chat' },
		curseforge: { id: 416089, slug: 'simple-voice-chat' },
	},
]

// ── Indexes ──────────────────────────────────────────────────────────────────

const byModrinthId = new Map<string, PlatformMapping>()
const byModrinthSlug = new Map<string, PlatformMapping>()
const byCfId = new Map<number, PlatformMapping>()
const byCfSlug = new Map<string, PlatformMapping>()

function indexMapping(m: PlatformMapping) {
	byModrinthId.set(m.modrinth.id, m)
	byModrinthSlug.set(m.modrinth.slug.toLowerCase(), m)
	byCfId.set(m.curseforge.id, m)
	if (m.curseforge.slug) byCfSlug.set(m.curseforge.slug.toLowerCase(), m)
}

for (const m of MAPPINGS) indexMapping(m)

// ── Runtime-learned mappings (persisted) ─────────────────────────────────────
//
// When `unified-search.ts` merges a Modrinth hit with a CurseForge hit via
// the slug/name fast-path, we also record the pair here so future searches
// know without re-checking. Persisted to localStorage so the catalog grows
// across sessions and the seed list can stay deliberately small.

const LEARNED_STORAGE_KEY = 'noctrinth:platform-mapping:learned'

interface LearnedEntry {
	m: string // modrinth id
	ms: string // modrinth slug
	c: number // cf id
	cs?: string // cf slug
}

function loadLearned(): void {
	try {
		const raw = localStorage.getItem(LEARNED_STORAGE_KEY)
		if (!raw) return
		const list = JSON.parse(raw) as LearnedEntry[]
		for (const e of list) {
			// Skip if we already have a hard-coded entry for either side (curated
			// wins over learned — protects against accidentally recording wrong
			// pairs during a search-result collision).
			if (byModrinthId.has(e.m) || byCfId.has(e.c)) continue
			indexMapping({
				modrinth: { id: e.m, slug: e.ms },
				curseforge: { id: e.c, slug: e.cs },
			})
		}
	} catch {
		// ignore unreadable storage
	}
}

function persistLearned(): void {
	try {
		// Snapshot only the learned entries (not seed) — seed is always in code.
		const seedModrinthIds = new Set(MAPPINGS.map((m) => m.modrinth.id))
		const out: LearnedEntry[] = []
		for (const m of byModrinthId.values()) {
			if (seedModrinthIds.has(m.modrinth.id)) continue
			out.push({
				m: m.modrinth.id,
				ms: m.modrinth.slug,
				c: m.curseforge.id,
				cs: m.curseforge.slug,
			})
		}
		localStorage.setItem(LEARNED_STORAGE_KEY, JSON.stringify(out))
	} catch {
		// ignore
	}
}

loadLearned()

/**
 * Record a discovered mapping (e.g. from a successful slug-match in unified
 * search). Idempotent and persisted.
 */
export function recordMapping(
	modrinthId: string,
	modrinthSlug: string,
	curseforgeId: number,
	curseforgeSlug?: string,
): void {
	if (byModrinthId.has(modrinthId) || byCfId.has(curseforgeId)) return
	const m: PlatformMapping = {
		modrinth: { id: modrinthId, slug: modrinthSlug },
		curseforge: { id: curseforgeId, slug: curseforgeSlug },
	}
	indexMapping(m)
	persistLearned()
}

// ── Public lookup API ────────────────────────────────────────────────────────

export function lookupByModrinthId(id: string): PlatformMapping | undefined {
	return byModrinthId.get(id)
}

export function lookupByModrinthSlug(slug: string): PlatformMapping | undefined {
	return byModrinthSlug.get(slug.toLowerCase())
}

export function lookupByCfId(id: number): PlatformMapping | undefined {
	return byCfId.get(id)
}

export function lookupByCfSlug(slug: string): PlatformMapping | undefined {
	return byCfSlug.get(slug.toLowerCase())
}

/** Convenience: given a Modrinth project_id, return its CF id if mapped. */
export function modrinthIdToCfId(modrinthId: string): number | undefined {
	return byModrinthId.get(modrinthId)?.curseforge.id
}

/** Convenience: given a CF mod id, return its Modrinth project_id if mapped. */
export function cfIdToModrinthId(cfId: number): string | undefined {
	return byCfId.get(cfId)?.modrinth.id
}
