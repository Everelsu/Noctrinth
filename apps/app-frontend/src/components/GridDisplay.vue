<script setup>
import {
	ClipboardCopyIcon,
	EyeIcon,
	FolderOpenIcon,
	PlayIcon,
	PlusIcon,
	SearchIcon,
	StopCircleIcon,
	TrashIcon,
} from '@modrinth/assets'
import {
	Accordion,
	Avatar,
	DropdownSelect,
	formatLoader,
	injectNotificationManager,
	StyledInput,
	useVIntl,
} from '@modrinth/ui'
import { useStorage } from '@vueuse/core'
import dayjs from 'dayjs'
import { computed, ref, watch } from 'vue'

import ContextMenu from '@/components/ui/ContextMenu.vue'
import Instance from '@/components/ui/Instance.vue'
import ConfirmDeleteInstanceModal from '@/components/ui/modal/ConfirmDeleteInstanceModal.vue'
import { useInstanceContentIndex } from '@/composables/instances/use-instance-content-index'
import { install_duplicate_instance } from '@/helpers/install'
import { remove } from '@/helpers/instance'
import {
	activeToken,
	applySuggestion,
	KNOWN_STATES,
	KNOWN_TYPES,
	matchInstance,
	parseQuery,
	suggestionsFor,
} from '@/helpers/instance-search'

const { handleError } = injectNotificationManager()

const { formatMessage } = useVIntl()

const props = defineProps({
	instances: {
		type: Array,
		default() {
			return []
		},
	},
	label: {
		type: String,
		default: '',
	},
})
const instanceOptions = ref(null)
const instanceComponents = ref(null)

const currentDeleteInstance = ref(null)
const confirmModal = ref(null)

async function deleteInstance() {
	if (currentDeleteInstance.value) {
		instanceComponents.value = instanceComponents.value.filter(
			(x) => x.instance.id !== currentDeleteInstance.value,
		)
		await remove(currentDeleteInstance.value).catch(handleError)
	}
}

async function duplicateInstance(p) {
	await install_duplicate_instance(p).catch(handleError)
}

const handleRightClick = (event, instanceId) => {
	const item = instanceComponents.value.find((x) => x.instance.id === instanceId)
	const baseOptions = [
		...(item.instance.quarantined ? [] : [{ name: 'add_content' }, { type: 'divider' }]),
		{ name: 'edit' },
		{ name: 'duplicate' },
		{ name: 'open' },
		{ name: 'copy' },
		{ type: 'divider' },
		{
			name: 'delete',
			color: 'danger',
		},
	]

	instanceOptions.value.showMenu(
		event,
		item,
		item.playing
			? [
					{
						name: 'stop',
						color: 'danger',
					},
					...baseOptions,
				]
			: [
					...(item.instance.quarantined
						? []
						: [
								{
									name: 'play',
									color: 'primary',
								},
							]),
					...baseOptions,
				],
	)
}

const handleOptionsClick = async (args) => {
	switch (args.option) {
		case 'play':
			args.item.play(null, 'InstanceGridContextMenu')
			break
		case 'stop':
			args.item.stop(null, 'InstanceGridContextMenu')
			break
		case 'add_content':
			await args.item.addContent()
			break
		case 'edit':
			await args.item.seeInstance()
			break
		case 'duplicate':
			if (args.item.instance.install_stage == 'installed')
				await duplicateInstance(args.item.instance.id)
			break
		case 'open':
			await args.item.openFolder()
			break
		case 'copy':
			await navigator.clipboard.writeText(args.item.instance.id)
			break
		case 'delete':
			currentDeleteInstance.value = args.item.instance.id
			confirmModal.value.show()
			break
	}
}

const state = useStorage(
	`${props.label}-grid-display-state`,
	{
		group: 'Group',
		sortBy: 'Name',
		collapsedGroups: [],
	},
	localStorage,
	{ mergeDefaults: true },
)

const search = ref('')

const contentIndex = useInstanceContentIndex()
const parsedQuery = computed(() => parseQuery(search.value))

const instanceIds = computed(() => props.instances.map((instance) => instance.id))

// A lone "@" isn't a term yet, so `needsContent` is false — but it's exactly
// when the suggestion list needs the index. Load on either signal.
const wantsContentIndex = computed(
	() => parsedQuery.value.needsContent || activeToken(search.value).replace(/^-/, '')[0] === '@',
)

// Only pay for the cross-instance content index once a query actually asks about
// content — plain name searches stay as cheap as they were before.
watch(
	() => [wantsContentIndex.value, instanceIds.value.join(',')],
	([wanted]) => {
		if (!wanted) return
		contentIndex.ensureLoaded(instanceIds.value)
	},
	{ immediate: true },
)

const searchFocused = ref(false)

const suggestions = computed(() => {
	if (!searchFocused.value) return []
	// Touch the version so completions appear as the index streams in.
	void contentIndex.version.value
	return suggestionsFor(search.value, contentIndex.contentNames(instanceIds.value))
})

const activeSuggestion = ref(0)

watch(suggestions, () => {
	activeSuggestion.value = 0
})

// Closing on focusout would fire before a suggestion's click lands, so the
// panel's own buttons use mousedown.prevent and only a focus leaving the whole
// wrapper closes it.
function onSearchBlur(event) {
	if (event.currentTarget.contains(event.relatedTarget)) return
	searchFocused.value = false
}

function chooseSuggestion(suggestion) {
	search.value = applySuggestion(search.value, suggestion)
	searchFocused.value = true
}

function onSearchKeydown(event) {
	if (!suggestions.value.length) return

	if (event.key === 'ArrowDown') {
		event.preventDefault()
		activeSuggestion.value = (activeSuggestion.value + 1) % suggestions.value.length
	} else if (event.key === 'ArrowUp') {
		event.preventDefault()
		activeSuggestion.value =
			(activeSuggestion.value - 1 + suggestions.value.length) % suggestions.value.length
	} else if (event.key === 'Tab' || event.key === 'Enter') {
		const suggestion = suggestions.value[activeSuggestion.value]
		if (!suggestion) return
		event.preventDefault()
		chooseSuggestion(suggestion)
	} else if (event.key === 'Escape') {
		searchFocused.value = false
	}
}

const contentPending = computed(
	() => parsedQuery.value.needsContent && !contentIndex.hasContentFor(instanceIds.value),
)

const instanceMatches = computed(() => {
	// Touch the index version so matching re-runs as instances stream in.
	void contentIndex.version.value

	const query = parsedQuery.value
	if (query.isEmpty) {
		return props.instances.map((instance) => ({ instance, contentMatches: [] }))
	}

	return props.instances
		.map((instance) => matchInstance(instance, query, contentIndex.contentFor(instance.id)))
		.filter((match) => match !== null)
})

const contentMatchesById = computed(() => {
	const map = new Map()
	for (const match of instanceMatches.value) {
		if (match.contentMatches.length) map.set(match.instance.id, match.contentMatches)
	}
	return map
})

const searchHint = computed(() => {
	const unknown = parsedQuery.value.unknown
	if (unknown.length) {
		const terms = unknown.map((token) => token.raw).join(', ')
		return `Ignoring ${terms} — types are ${KNOWN_TYPES.join(', ')}; states are ${KNOWN_STATES.join(', ')}.`
	}
	if (contentPending.value) return 'Searching instance content…'
	if (!parsedQuery.value.isEmpty && instanceMatches.value.length === 0) {
		return 'No instances match that search.'
	}
	return ''
})

const collapsedSectionKeys = computed(() => new Set(state.value.collapsedGroups ?? []))

const getSectionKey = (sectionName) => `${state.value.group}:${sectionName}`

const isSectionCollapsed = (sectionName) => {
	return collapsedSectionKeys.value.has(getSectionKey(sectionName))
}

const setSectionCollapsed = (sectionName, collapsed) => {
	const sectionKey = getSectionKey(sectionName)
	const collapsedSections = new Set(state.value.collapsedGroups ?? [])

	if (collapsed) {
		collapsedSections.add(sectionKey)
	} else {
		collapsedSections.delete(sectionKey)
	}

	state.value.collapsedGroups = [...collapsedSections]
}

const filteredResults = computed(() => {
	const { group = 'Group', sortBy = 'Name' } = state.value

	const instances = instanceMatches.value.map((match) => match.instance)

	if (sortBy === 'Name') {
		instances.sort((a, b) => {
			return a.name.localeCompare(b.name)
		})
	}

	if (sortBy === 'Game version') {
		instances.sort((a, b) => {
			return a.game_version.localeCompare(b.game_version, undefined, { numeric: true })
		})
	}

	if (sortBy === 'Last played') {
		instances.sort((a, b) => {
			return dayjs(b.last_played ?? 0).diff(dayjs(a.last_played ?? 0))
		})
	}

	if (sortBy === 'Date created') {
		instances.sort((a, b) => {
			return dayjs(b.date_created).diff(dayjs(a.date_created))
		})
	}

	if (sortBy === 'Date modified') {
		instances.sort((a, b) => {
			return dayjs(b.date_modified).diff(dayjs(a.date_modified))
		})
	}

	const instanceMap = new Map()

	if (group === 'Loader') {
		instances.forEach((instance) => {
			const loader = formatLoader(formatMessage, instance.loader)
			if (!instanceMap.has(loader)) {
				instanceMap.set(loader, [])
			}

			instanceMap.get(loader).push(instance)
		})
	} else if (group === 'Game version') {
		instances.forEach((instance) => {
			if (!instanceMap.has(instance.game_version)) {
				instanceMap.set(instance.game_version, [])
			}

			instanceMap.get(instance.game_version).push(instance)
		})
	} else if (group === 'Group') {
		instances.forEach((instance) => {
			if (instance.groups.length === 0) {
				instance.groups.push('None')
			}

			for (const category of instance.groups) {
				if (!instanceMap.has(category)) {
					instanceMap.set(category, [])
				}

				instanceMap.get(category).push(instance)
			}
		})
	} else {
		return instanceMap.set('None', instances)
	}

	// For 'name', we intuitively expect the sorting to apply to the name of the group first, not just the name of the instance
	// ie: Category A should come before B, even if the first instance in B comes before the first instance in A
	if (sortBy === 'Name') {
		const sortedEntries = [...instanceMap.entries()].sort((a, b) => {
			// None should always be first
			if (a[0] === 'None' && b[0] !== 'None') {
				return -1
			}
			if (a[0] !== 'None' && b[0] === 'None') {
				return 1
			}
			return a[0].localeCompare(b[0])
		})
		instanceMap.clear()
		sortedEntries.forEach((entry) => {
			instanceMap.set(entry[0], entry[1])
		})
	}
	// default sorting would do 1.20.4 < 1.8.9 because 2 < 8
	// localeCompare with numeric=true puts 1.8.9 < 1.20.4 because 8 < 20
	if (group === 'Game version') {
		const sortedEntries = [...instanceMap.entries()].sort((a, b) => {
			return a[0].localeCompare(b[0], undefined, { numeric: true })
		})
		instanceMap.clear()
		sortedEntries.forEach((entry) => {
			instanceMap.set(entry[0], entry[1])
		})
	}

	return instanceMap
})
</script>
<template>
	<div class="flex gap-2">
		<div class="relative flex-1 min-w-0" @focusin="searchFocused = true" @focusout="onSearchBlur">
			<StyledInput
				v-model="search"
				:icon="SearchIcon"
				type="text"
				placeholder="Search instances, or type @ # ! to filter"
				clearable
				wrapper-class="w-full"
				@keydown="onSearchKeydown"
			/>
			<div
				v-if="suggestions.length"
				role="listbox"
				aria-label="Search filters"
				class="absolute left-0 right-0 top-full z-[60] mt-1 overflow-hidden rounded-xl border border-solid border-surface-5 bg-surface-3 py-1 shadow-xl"
			>
				<button
					v-for="(suggestion, index) in suggestions"
					:key="suggestion.insert"
					type="button"
					role="option"
					:aria-selected="index === activeSuggestion"
					class="relative flex w-full items-center gap-2 border-none bg-transparent py-2 pl-4 pr-3 text-left font-inherit cursor-pointer before:absolute before:left-0 before:top-1 before:bottom-1 before:w-1 before:rounded-r before:content-['']"
					:class="
						index === activeSuggestion
							? 'bg-brand-highlight before:bg-brand'
							: 'before:bg-transparent hover:bg-surface-4'
					"
					@mousedown.prevent="chooseSuggestion(suggestion)"
					@mouseenter="activeSuggestion = index"
				>
					<Avatar
						v-if="suggestion.kind === 'content'"
						size="20px"
						:src="suggestion.iconUrl ?? null"
						:tint-by="suggestion.label"
						:alt="suggestion.label"
						class="shrink-0"
					/>
					<span
						v-else
						class="flex h-5 w-5 shrink-0 items-center justify-center rounded-md bg-surface-5 text-xs font-bold text-contrast"
					>
						{{ suggestion.insert.replace('-', '')[0] }}
					</span>
					<span class="min-w-0 flex-1">
						<span class="block truncate text-sm font-semibold text-contrast">
							{{ suggestion.label }}
						</span>
						<span class="block truncate text-xs text-secondary">{{ suggestion.hint }}</span>
					</span>
				</button>
				<p class="m-0 px-3 pt-1.5 pb-1 text-xs text-secondary">
					↑↓ to pick · Tab to insert · terms combine, and <code>-</code> flips one around
				</p>
			</div>
		</div>
		<DropdownSelect
			v-slot="{ selected }"
			v-model="state.sortBy"
			name="Sort Dropdown"
			class="max-w-[16rem]"
			:options="['Name', 'Last played', 'Date created', 'Date modified', 'Game version']"
			placeholder="Select..."
		>
			<span class="font-semibold text-primary">Sort by: </span>
			<span class="font-semibold text-secondary">{{ selected }}</span>
		</DropdownSelect>
		<DropdownSelect
			v-slot="{ selected }"
			v-model="state.group"
			class="max-w-[16rem]"
			name="Group Dropdown"
			:options="['Group', 'Loader', 'Game version', 'None']"
			placeholder="Select..."
		>
			<span class="font-semibold text-primary">Group by: </span>
			<span class="font-semibold text-secondary">{{ selected }}</span>
		</DropdownSelect>
	</div>
	<p v-if="searchHint" class="m-0 text-sm text-secondary">{{ searchHint }}</p>
	<Accordion
		v-for="instanceSection in Array.from(filteredResults, ([key, value]) => ({
			key,
			value,
		}))"
		:key="instanceSection.key"
		:divider="instanceSection.key !== 'None'"
		:open-by-default="!isSectionCollapsed(instanceSection.key)"
		class="row"
		@on-open="setSectionCollapsed(instanceSection.key, false)"
		@on-close="setSectionCollapsed(instanceSection.key, true)"
	>
		<template v-if="instanceSection.key !== 'None'" #title>
			<span class="text-base">{{ instanceSection.key }}</span>
		</template>
		<section class="instances">
			<Instance
				v-for="instance in instanceSection.value"
				ref="instanceComponents"
				:key="instance.id + instance.install_stage"
				:instance="instance"
				:content-matches="contentMatchesById.get(instance.id)"
				@contextmenu.prevent.stop="(event) => handleRightClick(event, instance.id)"
			/>
		</section>
	</Accordion>
	<ConfirmDeleteInstanceModal ref="confirmModal" @delete="deleteInstance" />
	<ContextMenu ref="instanceOptions" @option-clicked="handleOptionsClick">
		<template #play> <PlayIcon /> Play </template>
		<template #stop> <StopCircleIcon /> Stop </template>
		<template #add_content> <PlusIcon /> Add content </template>
		<template #edit> <EyeIcon /> View instance </template>
		<template #duplicate> <ClipboardCopyIcon /> Duplicate instance</template>
		<template #delete> <TrashIcon /> Delete </template>
		<template #open> <FolderOpenIcon /> Open folder </template>
		<template #copy> <ClipboardCopyIcon /> Copy path </template>
	</ContextMenu>
</template>
<style lang="scss" scoped>
.row {
	width: 100%;
}

.instances {
	display: grid;
	grid-template-columns: repeat(auto-fill, minmax(16rem, 1fr));
	width: 100%;
	gap: 0.75rem;
	margin-right: auto;
	scroll-behavior: smooth;
	overflow-y: auto;
}
</style>
