<script setup lang="ts">
import { BookmarkIcon, CheckIcon, PlusIcon, SearchIcon } from '@modrinth/assets'
import {
	FloatingPanel,
	injectNotificationManager,
	StyledInput,
} from '@modrinth/ui'
import { computed, onMounted, ref, watch } from 'vue'

import CollectionCreateModal from '@/components/ui/modal/CollectionCreateModal.vue'
import {
	addProjectToCollection,
	type Collection,
	getUserCollections,
	removeProjectFromCollection,
} from '@/helpers/modrinth-api'
import { get as getCreds } from '@/helpers/mr_auth.ts'

const props = defineProps<{
	projectId: string
}>()

const { handleError } = injectNotificationManager()

const collections = ref<Collection[]>([])
const loading = ref(false)
const signedIn = ref(false)
const filter = ref('')
const togglingId = ref<string | null>(null)
const panel = ref<InstanceType<typeof FloatingPanel>>()
const createModal = ref<InstanceType<typeof CollectionCreateModal>>()

const containsProject = (c: Collection): boolean =>
	(c.projects || []).includes(props.projectId)

const isInAnyCollection = computed(() => collections.value.some(containsProject))

const filtered = computed(() => {
	const q = filter.value.trim().toLowerCase()
	if (!q) return collections.value
	return collections.value.filter((c) => c.name.toLowerCase().includes(q))
})

async function load() {
	loading.value = true
	try {
		const creds = await getCreds()
		if (!creds) {
			signedIn.value = false
			collections.value = []
			return
		}
		signedIn.value = true
		collections.value = await getUserCollections(creds.user_id)
	} catch (e) {
		handleError(e)
	} finally {
		loading.value = false
	}
}

async function toggle(c: Collection) {
	if (togglingId.value) return
	togglingId.value = c.id
	const isMember = containsProject(c)
	try {
		if (isMember) {
			await removeProjectFromCollection(c.id, props.projectId, c.projects)
			c.projects = c.projects.filter((p) => p !== props.projectId)
		} else {
			await addProjectToCollection(c.id, props.projectId, c.projects)
			c.projects = [...c.projects, props.projectId]
		}
	} catch (e) {
		handleError(e)
	} finally {
		togglingId.value = null
	}
}

function openCreate() {
	createModal.value?.show()
}

function onCreated(collection: Collection) {
	// Auto-add the current project to the just-created collection.
	collection.projects = [...(collection.projects || []), props.projectId]
	collections.value = [collection, ...collections.value]
	addProjectToCollection(collection.id, props.projectId, []).catch(handleError)
}

watch(() => props.projectId, () => {
	if (signedIn.value) load()
})

onMounted(load)
</script>

<template>
	<CollectionCreateModal ref="createModal" @created="onCreated" />
	<FloatingPanel
		ref="panel"
		size="large"
		circular
		type="transparent"
		placement="bottom-end"
		panel-class="!p-0 min-w-[18rem]"
		@open="load"
	>
		<BookmarkIcon
			v-tooltip="isInAnyCollection ? 'Saved' : 'Save'"
			:class="{ 'text-brand fill-current': isInAnyCollection }"
		/>
		<template #panel>
			<div class="picker-panel">
				<div class="px-3 pt-3">
					<StyledInput
						v-model="filter"
						:icon="SearchIcon"
						type="text"
						clearable
						placeholder="Search..."
					/>
				</div>
				<div class="picker-list">
					<p v-if="!signedIn" class="hint">Sign in to Modrinth to save projects.</p>
					<p v-else-if="loading" class="hint">Loading collections...</p>
					<p v-else-if="filtered.length === 0" class="hint">
						{{ filter ? 'No collections match your search.' : "You don't have any collections yet." }}
					</p>
					<button
						v-for="c in filtered"
						:key="c.id"
						class="picker-row"
						:disabled="togglingId === c.id"
						@click="toggle(c)"
					>
						<span
							class="check-box"
							:class="{ checked: containsProject(c) }"
							aria-hidden="true"
						>
							<CheckIcon v-if="containsProject(c)" />
						</span>
						<span class="row-name">{{ c.name }}</span>
					</button>
				</div>
				<div class="picker-footer">
					<button class="picker-row" @click="openCreate">
						<span class="check-box" aria-hidden="true"><PlusIcon /></span>
						<span class="row-name">Create new collection</span>
					</button>
				</div>
			</div>
		</template>
	</FloatingPanel>
</template>

<style lang="scss" scoped>
.picker-panel {
	display: flex;
	flex-direction: column;
	gap: var(--gap-xs);
	min-width: 18rem;
	max-width: 22rem;
	padding-bottom: var(--gap-xs);
}

.picker-list {
	display: flex;
	flex-direction: column;
	max-height: 18rem;
	overflow-y: auto;
	padding: var(--gap-xs) var(--gap-xs);
}

.picker-row {
	display: flex;
	align-items: center;
	gap: var(--gap-sm);
	background: transparent;
	border: none;
	color: var(--color-contrast);
	font: inherit;
	padding: 0.5rem 0.75rem;
	border-radius: var(--radius-sm);
	cursor: pointer;
	text-align: left;
	width: 100%;

	&:hover:not(:disabled) {
		background-color: var(--color-button-bg-hover, var(--color-button-bg));
	}

	&:disabled {
		opacity: 0.6;
		cursor: progress;
	}
}

.row-name {
	flex: 1;
	min-width: 0;
	overflow: hidden;
	text-overflow: ellipsis;
	white-space: nowrap;
}

.check-box {
	display: inline-flex;
	align-items: center;
	justify-content: center;
	width: 1.25rem;
	height: 1.25rem;
	border-radius: var(--radius-xs);
	border: 1px solid var(--color-button-border);
	color: var(--color-contrast);
	flex-shrink: 0;

	&.checked {
		background-color: var(--color-brand);
		border-color: var(--color-brand);
		color: var(--color-accent-contrast);
	}

	svg {
		width: 0.85rem;
		height: 0.85rem;
	}
}

.picker-footer {
	border-top: 1px solid var(--color-divider);
	padding: var(--gap-xs);
}

.hint {
	color: var(--color-secondary);
	font-size: var(--font-size-sm);
	padding: 0.5rem 0.75rem;
	margin: 0;
}
</style>
