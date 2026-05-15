<script setup lang="ts">
import { BoxIcon, GlobeIcon, LinkIcon, LockIcon, PlusIcon, SearchIcon, XIcon } from '@modrinth/assets'
import {
	Avatar,
	ButtonStyled,
	DropdownSelect,
	injectNotificationManager,
	StyledInput,
	useCompactNumber,
} from '@modrinth/ui'
import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'

import CollectionCreateModal from '@/components/ui/modal/CollectionCreateModal.vue'
import {
	type Collection,
	getUserCollections,
	getUserFollowedProjects,
} from '@/helpers/modrinth-api'
import { get as getCreds } from '@/helpers/mr_auth.ts'

const { handleError } = injectNotificationManager()
const { formatCompactNumber } = useCompactNumber()
const router = useRouter()

const loading = ref(true)
const signedIn = ref(true)
const collections = ref<Collection[]>([])
const followsCount = ref(0)
const filterQuery = ref('')
const sortBy = ref<'updated' | 'created' | 'name'>('updated')

async function load() {
	loading.value = true
	try {
		const creds = await getCreds()
		if (!creds) {
			signedIn.value = false
			collections.value = []
			followsCount.value = 0
			return
		}
		signedIn.value = true
		const [cols, follows] = await Promise.all([
			getUserCollections(creds.user_id),
			getUserFollowedProjects(creds.user_id).catch(() => []),
		])
		collections.value = cols
		followsCount.value = follows.length
	} catch (e) {
		handleError(e)
		collections.value = []
	} finally {
		loading.value = false
	}
}

function formatSortOption(option: string) {
	if (option === 'updated') return 'Recently Updated'
	if (option === 'created') return 'Recently Created'
	return 'Name (A-Z)'
}

const orderedCollections = computed(() => {
	return [...collections.value]
		.filter(
			(c) => !filterQuery.value || c.name.toLowerCase().includes(filterQuery.value.toLowerCase()),
		)
		.sort((a, b) => {
			if (sortBy.value === 'name') return a.name.localeCompare(b.name)
			if (sortBy.value === 'created')
				return new Date(b.created).getTime() - new Date(a.created).getTime()
			return new Date(b.updated).getTime() - new Date(a.updated).getTime()
		})
})

const showFollowingCard = computed(
	() =>
		signedIn.value && 'followed projects'.includes(filterQuery.value.toLowerCase()),
)

function openCollection(id: string) {
	router.push(`/collection/${id}`)
}

function openFollowing() {
	router.push('/collection/following')
}

const createModal = ref<InstanceType<typeof CollectionCreateModal>>()

function openCreate() {
	createModal.value?.show()
}

function onCreated(collection: Collection) {
	collections.value = [collection, ...collections.value]
	router.push(`/collection/${collection.id}`)
}

onMounted(load)
</script>

<template>
	<div class="universal-card">
		<CollectionCreateModal ref="createModal" @created="onCreated" />
		<div class="mb-3 flex flex-col gap-3">
			<StyledInput
				v-model="filterQuery"
				:icon="SearchIcon"
				type="text"
				clearable
				placeholder="Search collections..."
				wrapper-class="w-full"
				input-class="!h-12"
			/>
			<div class="flex flex-wrap items-center gap-2">
				<DropdownSelect
					v-slot="{ selected }"
					v-model="sortBy"
					class="!w-auto flex-grow md:flex-grow-0"
					name="Sort by"
					:options="['updated', 'created', 'name']"
					:display-name="formatSortOption"
				>
					<span class="font-semibold text-primary">Sort by: </span>
					<span class="font-semibold text-secondary">{{ selected }}</span>
				</DropdownSelect>
				<ButtonStyled color="brand">
					<button class="ml-auto" @click="openCreate">
						<PlusIcon aria-hidden="true" />
						Create new
					</button>
				</ButtonStyled>
			</div>
		</div>

		<p v-if="loading">Loading collections...</p>

		<div v-else-if="!signedIn" class="empty-state-container">
			<div class="py-12 text-center">
				<BoxIcon class="mx-auto h-12 w-12 text-secondary opacity-50" aria-hidden="true" />
				<p class="mt-4 text-lg font-medium text-contrast">Sign in to view your collections</p>
				<p class="text-sm text-secondary">
					Sign in to your Modrinth account to see your collections here.
				</p>
			</div>
		</div>

		<div
			v-else-if="orderedCollections.length === 0 && !showFollowingCard"
			class="empty-state-container"
		>
			<div class="py-12 text-center">
				<BoxIcon class="mx-auto h-12 w-12 text-secondary opacity-50" aria-hidden="true" />
				<p class="mt-4 text-lg font-medium text-contrast">
					{{
						filterQuery
							? 'No collections match your search'
							: "You don't have any collections yet"
					}}
				</p>
				<p class="text-sm text-secondary">
					{{
						filterQuery
							? 'Try adjusting your filters or search terms.'
							: 'Create your first collection on modrinth.com to get started!'
					}}
				</p>
			</div>
		</div>

		<div v-else class="collections-grid">
			<button
				v-if="showFollowingCard"
				class="universal-card recessed collection"
				@click="openFollowing"
			>
				<Avatar src="https://cdn.modrinth.com/follow-collection.png" size="64px" />
				<div class="details">
					<span class="title">Followed projects</span>
					<span class="description">
						Auto-generated collection of all the projects you're following.
					</span>
					<div class="stat-bar">
						<div class="stats">
							<BoxIcon aria-hidden="true" />
							{{ formatCompactNumber(followsCount) }}
							{{ followsCount === 1 ? 'project' : 'projects' }}
						</div>
						<div class="stats">
							<LockIcon aria-hidden="true" />
							<span>Private</span>
						</div>
					</div>
				</div>
			</button>
			<button
				v-for="collection in orderedCollections"
				:key="collection.id"
				class="universal-card recessed collection"
				@click="openCollection(collection.id)"
			>
				<Avatar :src="collection.icon_url" size="64px" />
				<div class="details">
					<span class="title">{{ collection.name }}</span>
					<span v-if="collection.description" class="description">
						{{ collection.description }}
					</span>
					<div class="stat-bar">
						<div class="stats">
							<BoxIcon aria-hidden="true" />
							{{ formatCompactNumber(collection.projects?.length || 0) }}
							{{ (collection.projects?.length || 0) === 1 ? 'project' : 'projects' }}
						</div>
						<div class="stats">
							<template v-if="collection.status === 'listed'">
								<GlobeIcon aria-hidden="true" />
								<span>Public</span>
							</template>
							<template v-else-if="collection.status === 'unlisted'">
								<LinkIcon aria-hidden="true" />
								<span>Unlisted</span>
							</template>
							<template v-else-if="collection.status === 'private'">
								<LockIcon aria-hidden="true" />
								<span>Private</span>
							</template>
							<template v-else-if="collection.status === 'rejected'">
								<XIcon aria-hidden="true" />
								<span>Rejected</span>
							</template>
						</div>
					</div>
				</div>
			</button>
		</div>
	</div>
</template>

<style lang="scss" scoped>
.universal-card {
	padding: var(--gap-lg);
	background-color: var(--color-bg-raised);
	border-radius: var(--radius-lg);
	margin-bottom: var(--gap-md);

	h2 {
		margin: 0 0 var(--gap-md) 0;
		color: var(--color-contrast);
	}

	&.recessed {
		background-color: var(--color-bg);
		box-shadow: none;
	}
}

.collections-grid {
	display: grid;
	grid-template-columns: repeat(2, 1fr);
	gap: var(--gap-md);

	@media screen and (max-width: 800px) {
		grid-template-columns: repeat(1, 1fr);
	}

	.collection {
		display: grid;
		grid-template-columns: auto 1fr;
		gap: var(--gap-md);
		margin-bottom: 0;
		border: none;
		text-align: left;
		font: inherit;
		color: inherit;
		cursor: pointer;
		transition: outline-color 0.15s ease;
		outline: 2px solid transparent;
		outline-offset: -2px;

		&:hover {
			outline-color: var(--color-brand);
		}

		.details {
			display: flex;
			flex-direction: column;
			gap: var(--gap-sm);
			min-width: 0;

			.title {
				color: var(--color-contrast);
				font-weight: 600;
				font-size: var(--font-size-md);
			}

			.description {
				color: var(--color-secondary);
				font-size: var(--font-size-sm);
				word-break: break-word;
				display: -webkit-box;
				-webkit-line-clamp: 2;
				-webkit-box-orient: vertical;
				overflow: hidden;
			}

			.stat-bar {
				display: flex;
				align-items: center;
				gap: var(--gap-md);
				margin-top: auto;
				flex-wrap: wrap;
			}

			.stats {
				display: flex;
				align-items: center;
				gap: var(--gap-xs);
				color: var(--color-secondary);
				font-size: var(--font-size-sm);

				svg {
					color: var(--color-secondary);
					width: 1rem;
					height: 1rem;
				}
			}
		}
	}
}

.empty-state-container {
	display: flex;
	justify-content: center;
}
</style>
