/**
 * Noctrinth's own: quick search, after CurseForge's Ctrl+K spotlight.
 *
 * One query finds instances and what is installed in them, with the action
 * each one most likely wants — play an instance, update a mod — right in the
 * result, and offers the query to Discover for whatever is not installed yet.
 * It speaks the library's query language: a bare word finds instances by name
 * and installed content by name alike, and `@`, `#` and `!` narrow it the way
 * they do in the grid.
 *
 * The library's search field is where it lives, and Ctrl+K reaches it from
 * anywhere: on the library's page it focuses the field, elsewhere it opens the
 * same results in a palette over the page.
 */
import type { ContentItem } from '@modrinth/ui'
import { useQuery } from '@tanstack/vue-query'
import { computed, type Ref, ref, shallowRef, watch } from 'vue'

import { useInstanceContentIndex } from '@/composables/instances/use-instance-content-index'
import { canUpdateContentIn } from '@/composables/instances/use-managed-content-policy'
import { useAppEvent } from '@/composables/use-app-event'
import { curseforgeEnabled } from '@/helpers/curseforge'
import { matchContentItem, matchInstance, parseQuery } from '@/helpers/instance-search'
import { get_all as getRunningProcesses } from '@/helpers/process'
import type { GameInstance } from '@/helpers/types'
import { instanceListQueryOptions } from '@/pages/instance/query-options'

export type QuickSearchResult =
	| { kind: 'instance'; key: string; instance: GameInstance; running: boolean }
	| {
			kind: 'content'
			key: string
			instance: GameInstance
			item: ContentItem
			canUpdate: boolean
	  }
	| { kind: 'discover'; key: string; source: 'modrinth' | 'curseforge'; query: string }

const MAX_INSTANCES = 6
const MAX_CONTENT = 8

/** Whether the palette is open over the page. */
export const quickSearchOpen = ref(false)

/**
 * The library's own field, while it is on screen: Ctrl+K focuses it rather
 * than opening the palette over the very page it belongs to.
 */
const inlineField = shallowRef<(() => void) | null>(null)

export function registerInlineQuickSearch(focus: () => void): () => void {
	inlineField.value = focus
	return () => {
		if (inlineField.value === focus) inlineField.value = null
	}
}

/** What Ctrl+K does. */
export function openQuickSearch(): void {
	if (inlineField.value) inlineField.value()
	else quickSearchOpen.value = true
}

function contentTitle(item: ContentItem): string {
	return item.project?.title ?? item.embedded_metadata?.name ?? item.file_name
}

function lastPlayed(instance: GameInstance): number {
	return instance.last_played ? new Date(instance.last_played).getTime() : 0
}

/**
 * The results for a query. `active` says whether anyone is looking, so the
 * installed content of every instance is only read once someone searches.
 */
export function useQuickSearch(query: Ref<string>, active: Ref<boolean>) {
	const instancesQuery = useQuery(instanceListQueryOptions())
	const instances = computed(() => instancesQuery.data.value ?? [])
	const contentIndex = useInstanceContentIndex()

	const running = ref(new Set<string>())
	async function refreshRunning() {
		const processes = ((await getRunningProcesses().catch(() => [])) ?? []) as {
			instance_id: string
		}[]
		running.value = new Set(processes.map((process) => process.instance_id))
	}
	useAppEvent('process', () => void refreshRunning())

	watch(
		[active, instances],
		([isActive, list]) => {
			if (!isActive) return
			void refreshRunning()
			void contentIndex.ensureLoaded(list.map((instance) => instance.id))
		},
		{ immediate: true },
	)

	const parsed = computed(() => parseQuery(query.value))

	const results = computed<QuickSearchResult[]>(() => {
		void contentIndex.version.value
		const list = instances.value
		const text = query.value.trim()

		// Nothing typed: what was played last, the likeliest thing to want again.
		if (parsed.value.isEmpty) {
			return [...list]
				.sort((a, b) => lastPlayed(b) - lastPlayed(a))
				.slice(0, MAX_INSTANCES)
				.map((instance) => ({
					kind: 'instance' as const,
					key: `instance:${instance.id}`,
					instance,
					running: running.value.has(instance.id),
				}))
		}

		const found: QuickSearchResult[] = list
			.filter((instance) =>
				matchInstance(instance, parsed.value, contentIndex.contentFor(instance.id)),
			)
			.sort((a, b) => lastPlayed(b) - lastPlayed(a))
			.slice(0, MAX_INSTANCES)
			.map((instance) => ({
				kind: 'instance',
				key: `instance:${instance.id}`,
				instance,
				running: running.value.has(instance.id),
			}))

		// What is installed, newest-played instance first, and what can be
		// updated before what cannot: that is the row worth an action.
		const content: QuickSearchResult[] = []
		for (const instance of [...list].sort((a, b) => lastPlayed(b) - lastPlayed(a))) {
			for (const item of contentIndex.contentFor(instance.id) ?? []) {
				if (!matchContentItem(parsed.value, item)) continue
				content.push({
					kind: 'content',
					key: `content:${instance.id}:${item.file_name}`,
					instance,
					item,
					canUpdate: canUpdateContentIn(instance, item),
				})
			}
		}
		content.sort(
			(a, b) =>
				Number(b.kind === 'content' && b.canUpdate) - Number(a.kind === 'content' && a.canUpdate),
		)
		found.push(...content.slice(0, MAX_CONTENT))

		// A query in the query language is about what is installed; plain words
		// can also be looked for in Discover.
		if (!/[@#!]/.test(text)) {
			found.push({ kind: 'discover', key: 'discover:modrinth', source: 'modrinth', query: text })
			if (curseforgeEnabled()) {
				found.push({
					kind: 'discover',
					key: 'discover:curseforge',
					source: 'curseforge',
					query: text,
				})
			}
		}
		return found
	})

	return {
		results,
		/** True while installed content is still being read, so results may grow. */
		loading: computed(() => contentIndex.loading.value || instancesQuery.isPending.value),
		contentTitle,
	}
}
