/**
 * Lazily-built index of installed content across every instance, backing the
 * `@mod` / `#type` / `!outdated` terms in the instance grid search.
 *
 * Content is fetched per instance, so this only runs when a query actually asks
 * about content, and results are cached module-wide: the library tabs each mount
 * their own grid, and a user flipping between them shouldn't re-fetch.
 */
import type { ContentItem } from '@modrinth/ui'
import { computed, readonly, ref } from 'vue'

import { useAppEvent } from '@/composables/use-app-event'
import { get_content_items } from '@/helpers/instance'

/** How many instances to query at once. Each call can touch the network cache. */
const CONCURRENCY = 4

const cache = new Map<string, ContentItem[]>()
const inFlight = new Map<string, Promise<void>>()
const version = ref(0)
const loadingCount = ref(0)

async function loadOne(instanceId: string): Promise<void> {
	try {
		cache.set(instanceId, await get_content_items(instanceId))
	} catch {
		// A single unreadable instance shouldn't sink the whole search — treat it
		// as having no content and let the other instances still match.
		cache.set(instanceId, [])
	}
	version.value += 1
}

function ensureOne(instanceId: string): Promise<void> {
	const existing = inFlight.get(instanceId)
	if (existing) return existing

	const promise = loadOne(instanceId).finally(() => {
		inFlight.delete(instanceId)
	})
	inFlight.set(instanceId, promise)
	return promise
}

export function useInstanceContentIndex() {
	// Content changes whenever an instance is edited or removed; drop just that
	// instance's entry so the next content query re-reads it. The cache is shared
	// module-wide but the subscription is per-caller, which `useAppEvent` tears
	// down with the owning component.
	useAppEvent('instance', (payload) => {
		if (!payload?.instance_id) return
		if (cache.delete(payload.instance_id)) version.value += 1
	})

	/** Fetches content for any instance not already cached, in small batches. */
	async function ensureLoaded(instanceIds: string[]): Promise<void> {
		const missing = instanceIds.filter((id) => !cache.has(id))
		if (!missing.length) return

		loadingCount.value += 1
		try {
			for (let index = 0; index < missing.length; index += CONCURRENCY) {
				await Promise.all(missing.slice(index, index + CONCURRENCY).map(ensureOne))
			}
		} finally {
			loadingCount.value -= 1
		}
	}

	function contentFor(instanceId: string): ContentItem[] | undefined {
		return cache.get(instanceId)
	}

	function hasContentFor(instanceIds: string[]): boolean {
		return instanceIds.every((id) => cache.has(id))
	}

	/**
	 * Every distinct content name across the cached instances, for completing
	 * `@` terms. Sorted so the list under the search field is stable while the
	 * index is still streaming in.
	 */
	function contentNames(instanceIds: string[]): { name: string; iconUrl?: string | null }[] {
		const byName = new Map<string, { name: string; iconUrl?: string | null }>()

		for (const id of instanceIds) {
			for (const item of cache.get(id) ?? []) {
				const name = item.project?.title ?? item.embedded_metadata?.name ?? item.file_name
				if (!name) continue

				const key = name.toLowerCase()
				const existing = byName.get(key)
				// Prefer whichever copy actually has an icon.
				if (!existing) byName.set(key, { name, iconUrl: item.project?.icon_url })
				else if (!existing.iconUrl && item.project?.icon_url) {
					existing.iconUrl = item.project.icon_url
				}
			}
		}

		return [...byName.values()].sort((left, right) => left.name.localeCompare(right.name))
	}

	return {
		ensureLoaded,
		contentFor,
		hasContentFor,
		contentNames,
		/** Read this in a computed to re-run matching as instances finish loading. */
		version: readonly(version),
		loading: computed(() => loadingCount.value > 0),
	}
}
