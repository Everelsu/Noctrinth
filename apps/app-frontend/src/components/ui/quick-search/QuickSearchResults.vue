<script setup lang="ts">
/**
 * Noctrinth's own: the results of a quick search, for the library's field and
 * the Ctrl+K palette alike. The field they hang under keeps the focus and hands
 * its keys over through `onKeydown`: ↑↓ pick a row, Enter opens it, and
 * Ctrl+Enter does the row's own action — play, stop or update.
 */
import {
	CurseForgeIcon,
	DownloadIcon,
	ModrinthIcon,
	PlayIcon,
	SearchIcon,
	SpinnerIcon,
	StopCircleIcon,
} from '@modrinth/assets'
import { Avatar, Button, defineMessages, injectNotificationManager, useVIntl } from '@modrinth/ui'
import { computed, ref, toRef, watch } from 'vue'
import { useRouter } from 'vue-router'

import { handleSevereError } from '@/composables/use-error.js'
import { type QuickSearchResult, useQuickSearch } from '@/composables/use-quick-search'
import { curseforgeIdOf, isCurseforgeContent } from '@/helpers/curseforge'
import {
	getInstanceIconUrl,
	kill,
	run,
	switch_project_version_with_dependencies,
} from '@/helpers/instance'
import { injectContentInstall } from '@/providers/content-install'

const props = defineProps<{ query: string; active: boolean }>()
const emit = defineEmits<{ done: [] }>()

const { formatMessage } = useVIntl()
const { handleError } = injectNotificationManager()
const { installCurseforgeFile } = injectContentInstall()
const router = useRouter()

const messages = defineMessages({
	recent: { id: 'noctrinth.quick-search.recent', defaultMessage: 'Played recently' },
	instances: { id: 'noctrinth.quick-search.instances', defaultMessage: 'Instances' },
	installed: { id: 'noctrinth.quick-search.installed', defaultMessage: 'Installed' },
	discover: { id: 'noctrinth.quick-search.discover', defaultMessage: 'Not installed yet' },
	settings: { id: 'noctrinth.quick-search.settings', defaultMessage: 'Settings' },
	play: { id: 'noctrinth.quick-search.play', defaultMessage: 'Play' },
	stop: { id: 'noctrinth.quick-search.stop', defaultMessage: 'Stop' },
	update: { id: 'noctrinth.quick-search.update', defaultMessage: 'Update' },
	inInstance: { id: 'noctrinth.quick-search.in-instance', defaultMessage: 'In {instance}' },
	updateAvailable: {
		id: 'noctrinth.quick-search.update-available',
		defaultMessage: 'Update available',
	},
	searchModrinth: {
		id: 'noctrinth.quick-search.search-modrinth',
		defaultMessage: 'Search “{query}” on Modrinth',
	},
	searchCurseforge: {
		id: 'noctrinth.quick-search.search-curseforge',
		defaultMessage: 'Search “{query}” on CurseForge',
	},
	nothing: { id: 'noctrinth.quick-search.nothing', defaultMessage: 'Nothing matches' },
	reading: {
		id: 'noctrinth.quick-search.reading',
		defaultMessage: 'Still reading what is installed…',
	},
	hint: {
		id: 'noctrinth.quick-search.hint',
		defaultMessage: '↑↓ to pick · Enter to open · Ctrl+Enter to play or update · Esc to close',
	},
})

const { results, loading, contentTitle, openSettingsTab } = useQuickSearch(
	toRef(props, 'query'),
	toRef(props, 'active'),
)

const activeIndex = ref(0)
watch(results, () => (activeIndex.value = 0))

/** Rows an action is running for, so their button spins instead of firing twice. */
const busy = ref(new Set<string>())

const sections = computed(() => {
	const titles: Record<QuickSearchResult['kind'], string> = {
		instance: formatMessage(props.query.trim() ? messages.instances : messages.recent),
		content: formatMessage(messages.installed),
		settings: formatMessage(messages.settings),
		discover: formatMessage(messages.discover),
	}
	const list: { title: string; rows: { result: QuickSearchResult; index: number }[] }[] = []
	results.value.forEach((result, index) => {
		const title = titles[result.kind]
		if (list[list.length - 1]?.title !== title) list.push({ title, rows: [] })
		list[list.length - 1].rows.push({ result, index })
	})
	return list
})

function loaderLabel(loader: string): string {
	return loader === 'vanilla' ? 'Minecraft' : loader[0].toUpperCase() + loader.slice(1)
}

async function open(result: QuickSearchResult) {
	emit('done')
	if (result.kind === 'settings') {
		openSettingsTab(result.tab.id)
	} else if (result.kind === 'discover') {
		await router.push({
			path: '/browse/mod',
			query: { q: result.query, src: result.source === 'curseforge' ? 'cf' : 'mr' },
		})
	} else {
		await router.push(`/instance/${encodeURIComponent(result.instance.id)}`)
	}
}

/** The row's own action; false when it has none, and Enter should open it instead. */
function hasAction(result: QuickSearchResult): boolean {
	if (result.kind === 'instance') return !result.instance.quarantined
	return result.kind === 'content' && result.canUpdate
}

async function act(result: QuickSearchResult) {
	if (!hasAction(result) || busy.value.has(result.key)) return
	busy.value.add(result.key)
	try {
		if (result.kind === 'instance') {
			if (result.running) await kill(result.instance.id).catch(handleError)
			else {
				await run(result.instance.id).catch((error) =>
					handleSevereError(error, { instanceId: result.instance.id }),
				)
			}
		} else if (result.kind === 'content') {
			const { instance, item } = result
			if (isCurseforgeContent(item) && item.project?.id) {
				await installCurseforgeFile(
					instance.id,
					curseforgeIdOf(item.project.id),
					item.update_version_id!,
				)
			} else {
				await switch_project_version_with_dependencies(
					instance.id,
					item.file_path!,
					item.update_version_id!,
				)
			}
		}
	} catch (error) {
		handleError(error as Error)
	} finally {
		busy.value.delete(result.key)
	}
}

function onKeydown(event: KeyboardEvent): boolean {
	const count = results.value.length
	if (!count) return false
	if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
		event.preventDefault()
		const step = event.key === 'ArrowDown' ? 1 : -1
		activeIndex.value = (activeIndex.value + step + count) % count
		return true
	}
	if (event.key === 'Enter') {
		const result = results.value[activeIndex.value]
		if (!result) return false
		event.preventDefault()
		if ((event.ctrlKey || event.metaKey) && hasAction(result)) void act(result)
		else void open(result)
		return true
	}
	return false
}

defineExpose({ onKeydown })
</script>

<template>
	<div class="flex flex-col">
		<div class="max-h-[min(28rem,60vh)] overflow-y-auto py-1" role="listbox">
			<template v-for="section in sections" :key="section.title">
				<p class="m-0 px-4 pb-1 pt-2 text-xs font-semibold uppercase tracking-wide text-secondary">
					{{ section.title }}
				</p>
				<div
					v-for="{ result, index } in section.rows"
					:key="result.key"
					role="option"
					:aria-selected="index === activeIndex"
					class="relative flex cursor-pointer items-center gap-3 py-2 pl-4 pr-3 before:absolute before:bottom-1 before:left-0 before:top-1 before:w-1 before:rounded-r before:content-['']"
					:class="
						index === activeIndex
							? 'bg-brand-highlight before:bg-brand'
							: 'before:bg-transparent hover:bg-surface-4'
					"
					@mousedown.prevent
					@mouseenter="activeIndex = index"
					@click="open(result)"
				>
					<template v-if="result.kind === 'settings'">
						<span
							class="flex size-9 shrink-0 items-center justify-center rounded-lg bg-surface-5 text-contrast"
						>
							<component :is="result.tab.icon" class="size-5" />
						</span>
						<span class="min-w-0 flex-1">
							<span class="block truncate font-semibold text-contrast">
								{{ formatMessage(result.tab.name) }}
							</span>
							<span class="block truncate text-sm text-secondary">
								{{ formatMessage(messages.settings) }} ·
								{{ formatMessage(result.tab.category) }}
							</span>
						</span>
					</template>

					<template v-else-if="result.kind === 'discover'">
						<span
							class="flex size-9 shrink-0 items-center justify-center rounded-lg bg-surface-5 text-contrast"
						>
							<CurseForgeIcon v-if="result.source === 'curseforge'" class="size-5" />
							<ModrinthIcon v-else class="size-5" />
						</span>
						<span class="min-w-0 flex-1 truncate font-semibold text-contrast">
							{{
								formatMessage(
									result.source === 'curseforge'
										? messages.searchCurseforge
										: messages.searchModrinth,
									{ query: result.query },
								)
							}}
						</span>
						<SearchIcon class="size-4 shrink-0 text-secondary" />
					</template>

					<template v-else>
						<Avatar
							:src="
								result.kind === 'instance'
									? getInstanceIconUrl(result.instance.icon_path)
									: (result.item.project?.icon_url ?? null)
							"
							:tint-by="result.kind === 'instance' ? result.instance.id : contentTitle(result.item)"
							:alt="result.kind === 'instance' ? result.instance.name : contentTitle(result.item)"
							size="2.25rem"
							rounded="md"
							pad-transparent-corners
							class="shrink-0"
						/>
						<span class="min-w-0 flex-1">
							<span class="block truncate font-semibold text-contrast">
								{{ result.kind === 'instance' ? result.instance.name : contentTitle(result.item) }}
							</span>
							<span class="block truncate text-sm text-secondary">
								<template v-if="result.kind === 'instance'">
									{{ loaderLabel(result.instance.loader) }} {{ result.instance.game_version }}
								</template>
								<template v-else>
									{{ formatMessage(messages.inInstance, { instance: result.instance.name }) }}
									<template v-if="result.item.has_update">
										· <span class="text-brand">{{ formatMessage(messages.updateAvailable) }}</span>
									</template>
								</template>
							</span>
						</span>
						<Button
							v-if="hasAction(result)"
							size="sm"
							:type="result.kind === 'instance' && !result.running ? 'colored' : 'outlined'"
							:color="result.kind === 'instance' && !result.running ? 'brand' : undefined"
							:disabled="busy.has(result.key)"
							@click.stop="act(result)"
						>
							<SpinnerIcon v-if="busy.has(result.key)" class="animate-spin" />
							<StopCircleIcon v-else-if="result.kind === 'instance' && result.running" />
							<PlayIcon v-else-if="result.kind === 'instance'" />
							<DownloadIcon v-else />
							{{
								formatMessage(
									result.kind === 'content'
										? messages.update
										: result.running
											? messages.stop
											: messages.play,
								)
							}}
						</Button>
					</template>
				</div>
			</template>
			<p v-if="!results.length" class="m-0 px-4 py-3 text-sm text-secondary">
				{{ formatMessage(loading ? messages.reading : messages.nothing) }}
			</p>
		</div>
		<p class="m-0 border-0 border-t border-solid border-surface-5 px-4 py-2 text-xs text-secondary">
			<template v-if="loading && results.length">{{ formatMessage(messages.reading) }} · </template
			>{{ formatMessage(messages.hint) }}
		</p>
	</div>
</template>
