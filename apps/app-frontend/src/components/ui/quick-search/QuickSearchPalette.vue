<script setup lang="ts">
/**
 * Noctrinth's own: quick search over whatever page is open, on Ctrl+K (⌘K on
 * macOS). Mounted once for the whole app; on the library's page the shortcut
 * focuses the library's own field instead, which shows the same results.
 */
import { SearchIcon } from '@modrinth/assets'
import { defineMessages, Input, useVIntl } from '@modrinth/ui'
import { useEventListener } from '@vueuse/core'
import { nextTick, ref, watch } from 'vue'
import { useRoute } from 'vue-router'

import QuickSearchResults from '@/components/ui/quick-search/QuickSearchResults.vue'
import { openQuickSearch, quickSearchOpen } from '@/composables/use-quick-search'

const { formatMessage } = useVIntl()
const route = useRoute()

const messages = defineMessages({
	placeholder: {
		id: 'noctrinth.quick-search.placeholder',
		defaultMessage: 'Search instances, what is installed in them, and settings',
	},
})

const query = ref('')
const input = ref<InstanceType<typeof Input> | null>(null)
const results = ref<InstanceType<typeof QuickSearchResults> | null>(null)

/** Whether a key press is Ctrl+K, by the key's place so any layout works. */
function isShortcut(event: KeyboardEvent): boolean {
	return (
		(event.ctrlKey || event.metaKey) && !event.altKey && !event.shiftKey && event.code === 'KeyK'
	)
}

useEventListener(window, 'keydown', (event: KeyboardEvent) => {
	if (!isShortcut(event) || event.defaultPrevented) return
	event.preventDefault()
	if (quickSearchOpen.value) close()
	else openQuickSearch()
})

function close() {
	quickSearchOpen.value = false
}

watch(quickSearchOpen, async (open) => {
	if (!open) return
	query.value = ''
	await nextTick()
	input.value?.focus()
})

// Leaving the page closes it, whichever result led there.
watch(() => route.fullPath, close)

function onKeydown(event: KeyboardEvent) {
	if (event.key === 'Escape') {
		event.preventDefault()
		close()
		return
	}
	results.value?.onKeydown(event)
}
</script>

<template>
	<Teleport to="body">
		<Transition name="quick-search">
			<div
				v-if="quickSearchOpen"
				class="fixed inset-0 z-[200] flex items-start justify-center bg-black/50 px-4 pt-[12vh] backdrop-blur-sm"
				@mousedown.self="close"
			>
				<div
					class="quick-search-panel flex w-full max-w-[40rem] flex-col overflow-hidden rounded-2xl border border-solid border-surface-5 bg-surface-3 shadow-2xl"
					role="dialog"
					:aria-label="formatMessage(messages.placeholder)"
				>
					<div class="border-0 border-b border-solid border-surface-5 p-2">
						<Input
							ref="input"
							v-model="query"
							:icon="SearchIcon"
							:placeholder="formatMessage(messages.placeholder)"
							wrapper-class="w-full"
							@keydown="onKeydown"
						/>
					</div>
					<QuickSearchResults
						ref="results"
						:query="query"
						:active="quickSearchOpen"
						@done="close"
					/>
				</div>
			</div>
		</Transition>
	</Teleport>
</template>

<style scoped>
.quick-search-enter-active,
.quick-search-leave-active {
	transition: opacity 120ms ease;
}

.quick-search-enter-active .quick-search-panel,
.quick-search-leave-active .quick-search-panel {
	transition: transform 120ms ease;
}

.quick-search-enter-from,
.quick-search-leave-to {
	opacity: 0;
}

.quick-search-enter-from .quick-search-panel,
.quick-search-leave-to .quick-search-panel {
	transform: translateY(-0.5rem) scale(0.98);
}
</style>
