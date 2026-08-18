<script setup lang="ts">
import { PlusIcon, SearchIcon, SquarePlusIcon } from '@modrinth/assets'
import { Avatar, Button, defineMessages, StyledInput, useVIntl } from '@modrinth/ui'
import { computed, inject } from 'vue'

import FilterMenu from '@/components/ui/library/library-toolbar/filter-menu.vue'
import NewGroupModal from '@/components/ui/library/library-toolbar/new-group-modal.vue'
import SortMenu from '@/components/ui/library/library-toolbar/sort-menu.vue'
import { useLibrary } from '@/components/ui/library/use-library'

const {
	search,
	searchFocused,
	searchPending,
	suggestions,
	activeSuggestion,
	chooseSuggestion,
	selectedLibraryInstances,
	openNewGroupModal,
} = useLibrary()
const showCreationModal = inject<() => void>('showCreationModal')
const { formatMessage } = useVIntl()
const messages = defineMessages({
	search: {
		id: 'app.library.search.placeholder',
		defaultMessage: 'Search instances, or type @ # ! to filter',
	},
	searchHint: {
		id: 'app.library.search.hint',
		defaultMessage: '↑↓ to pick · Tab to insert · terms combine, and - flips one around',
	},
	searchPending: {
		id: 'app.library.search.pending',
		defaultMessage: 'Still reading what is installed…',
	},
	newGroup: { id: 'app.library.group.new', defaultMessage: 'New group' },
	newInstance: { id: 'app.library.instance.new', defaultMessage: 'New instance' },
})
const selectedInstanceIds = computed(
	() =>
		new Set([...selectedLibraryInstances.value.values()].map((selection) => selection.instanceId)),
)

// Closing on focusout would fire before a suggestion's click lands, so the
// panel's own buttons use mousedown.prevent and only a focus leaving the whole
// wrapper closes it.
function onSearchBlur(event: FocusEvent) {
	const wrapper = event.currentTarget as HTMLElement | null
	if (wrapper?.contains(event.relatedTarget as Node | null)) return
	searchFocused.value = false
}

function onSearchKeydown(event: KeyboardEvent) {
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

function openNewGroup() {
	openNewGroupModal(selectedInstanceIds.value)
}
</script>

<template>
	<div class="flex flex-col gap-2">
		<div class="flex flex-wrap gap-2">
			<div
				class="relative min-w-[16rem] flex-1"
				@focusin="searchFocused = true"
				@focusout="onSearchBlur"
			>
				<StyledInput
					v-model="search"
					:icon="SearchIcon"
					type="text"
					:placeholder="formatMessage(messages.search)"
					clearable
					wrapper-class="w-full"
					@keydown="onSearchKeydown"
				/>
				<div
					v-if="suggestions.length"
					role="listbox"
					:aria-label="formatMessage(messages.search)"
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
						{{ formatMessage(messages.searchHint) }}
					</p>
				</div>
			</div>
			<Button @click="openNewGroup">
				<SquarePlusIcon />
				{{ formatMessage(messages.newGroup) }}
			</Button>
			<Button type="colored" color="brand" @click="showCreationModal?.()">
				<PlusIcon />
				{{ formatMessage(messages.newInstance) }}
			</Button>
		</div>
		<p v-if="searchPending" class="m-0 text-xs text-secondary">
			{{ formatMessage(messages.searchPending) }}
		</p>
		<div class="flex flex-wrap items-center gap-2">
			<SortMenu />
			<div class="mx-2 h-6 w-px bg-surface-5" />
			<FilterMenu />
		</div>
	</div>
	<NewGroupModal />
</template>
