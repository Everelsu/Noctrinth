<script setup lang="ts">
import {
	BoxIcon,
	GlobeIcon,
	LinkIcon,
	LockIcon,
	PlusIcon,
	SearchIcon,
	XIcon,
} from '@modrinth/assets'
import {
	Avatar,
	Button,
	defineMessages,
	DropdownSelect,
	injectNotificationManager,
	StyledInput,
	useCompactNumber,
	useVIntl,
} from '@modrinth/ui'
import { computed, ref } from 'vue'
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
const { formatMessage } = useVIntl()
const router = useRouter()

const messages = defineMessages({
	searchPlaceholder: {
		id: 'collections.search-placeholder',
		defaultMessage: 'Search collections...',
	},
	sortBy: { id: 'collections.sort-by', defaultMessage: 'Sort by: ' },
	sortUpdated: { id: 'collections.sort.updated', defaultMessage: 'Recently Updated' },
	sortCreated: { id: 'collections.sort.created', defaultMessage: 'Recently Created' },
	sortName: { id: 'collections.sort.name', defaultMessage: 'Name (A-Z)' },
	sortByName: { id: 'collections.sort-by-name', defaultMessage: 'Sort by' },
	createNew: { id: 'collections.create-new', defaultMessage: 'Create new' },
	loading: { id: 'collections.loading', defaultMessage: 'Loading collections...' },
	signInPrompt: {
		id: 'collections.sign-in-prompt',
		defaultMessage: 'Sign in to view your collections',
	},
	signInPromptBody: {
		id: 'collections.sign-in-prompt-body',
		defaultMessage: 'Sign in to your Modrinth account to see your collections here.',
	},
	noMatch: { id: 'collections.no-match', defaultMessage: 'No collections match your search' },
	noCollections: {
		id: 'collections.no-collections',
		defaultMessage: "You don't have any collections yet",
	},
	noMatchBody: {
		id: 'collections.no-match-body',
		defaultMessage: 'Try adjusting your filters or search terms.',
	},
	noCollectionsBody: {
		id: 'collections.no-collections-body',
		defaultMessage: 'Create your first collection on modrinth.com to get started!',
	},
	followedProjects: { id: 'collections.followed-projects', defaultMessage: 'Followed projects' },
	followedProjectsDesc: {
		id: 'collections.followed-projects-desc',
		defaultMessage: "Auto-generated collection of all the projects you're following.",
	},
	projectsCountOne: { id: 'collections.projects-count.one', defaultMessage: 'project' },
	projectsCountOther: { id: 'collections.projects-count.other', defaultMessage: 'projects' },
	statusPrivate: { id: 'collections.status.private', defaultMessage: 'Private' },
	statusPublic: { id: 'collections.status.public', defaultMessage: 'Public' },
	statusUnlisted: { id: 'collections.status.unlisted', defaultMessage: 'Unlisted' },
	statusRejected: { id: 'collections.status.rejected', defaultMessage: 'Rejected' },
})

const signedIn = ref(true)
const collections = ref<Collection[]>([])
const followsCount = ref(0)
const filterQuery = ref('')
const sortBy = ref<'updated' | 'created' | 'name'>('updated')

// Top-level await so RouterView's <Suspense> holds the navigation (and shows
// the top progress bar) until all data is ready — matches the Browse page UX.
try {
	const creds = await getCreds()
	if (!creds) {
		signedIn.value = false
	} else {
		const [cols, follows] = await Promise.all([
			getUserCollections(creds.user_id),
			getUserFollowedProjects(creds.user_id).catch(() => []),
		])
		collections.value = cols
		followsCount.value = follows.length
	}
} catch (e) {
	handleError(e)
}

function formatSortOption(option: string) {
	if (option === 'updated') return formatMessage(messages.sortUpdated)
	if (option === 'created') return formatMessage(messages.sortCreated)
	return formatMessage(messages.sortName)
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
	() => signedIn.value && 'followed projects'.includes(filterQuery.value.toLowerCase()),
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
</script>

<template>
	<div class="flex flex-col gap-4">
		<CollectionCreateModal ref="createModal" @created="onCreated" />
		<div class="flex flex-col gap-3 sm:flex-row sm:items-center">
			<StyledInput
				v-model="filterQuery"
				:icon="SearchIcon"
				type="text"
				clearable
				:placeholder="formatMessage(messages.searchPlaceholder)"
				wrapper-class="w-full sm:flex-1"
				input-class="!h-10"
			/>
			<div class="flex flex-wrap items-center gap-2">
				<DropdownSelect
					v-slot="{ selected }"
					v-model="sortBy"
					class="!w-auto flex-grow md:flex-grow-0"
					:name="formatMessage(messages.sortByName)"
					:options="['updated', 'created', 'name']"
					:display-name="formatSortOption"
				>
					<span class="font-semibold text-primary">{{ formatMessage(messages.sortBy) }}</span>
					<span class="font-semibold text-secondary">{{ selected }}</span>
				</DropdownSelect>
				<Button type="colored" color="brand" @click="openCreate">
					<PlusIcon aria-hidden="true" />
					{{ formatMessage(messages.createNew) }}
				</Button>
			</div>
		</div>

		<div v-if="!signedIn" class="empty-state">
			<BoxIcon class="h-10 w-10 text-secondary opacity-50" aria-hidden="true" />
			<p class="m-0 text-lg font-medium text-contrast">
				{{ formatMessage(messages.signInPrompt) }}
			</p>
			<p class="m-0 text-sm text-secondary">
				{{ formatMessage(messages.signInPromptBody) }}
			</p>
		</div>

		<div v-else-if="orderedCollections.length === 0 && !showFollowingCard" class="empty-state">
			<BoxIcon class="h-10 w-10 text-secondary opacity-50" aria-hidden="true" />
			<p class="m-0 text-lg font-medium text-contrast">
				{{ formatMessage(filterQuery ? messages.noMatch : messages.noCollections) }}
			</p>
			<p class="m-0 text-sm text-secondary">
				{{ formatMessage(filterQuery ? messages.noMatchBody : messages.noCollectionsBody) }}
			</p>
		</div>

		<div v-else class="collections-grid">
			<button v-if="showFollowingCard" class="collection" @click="openFollowing">
				<Avatar src="https://cdn.modrinth.com/follow-collection.png" size="64px" />
				<div class="details">
					<span class="title">{{ formatMessage(messages.followedProjects) }}</span>
					<span class="description">
						{{ formatMessage(messages.followedProjectsDesc) }}
					</span>
					<div class="stat-bar">
						<div class="stats">
							<BoxIcon aria-hidden="true" />
							{{ formatCompactNumber(followsCount) }}
							{{
								formatMessage(
									followsCount === 1 ? messages.projectsCountOne : messages.projectsCountOther,
								)
							}}
						</div>
						<div class="stats">
							<LockIcon aria-hidden="true" />
							<span>{{ formatMessage(messages.statusPrivate) }}</span>
						</div>
					</div>
				</div>
			</button>
			<button
				v-for="collection in orderedCollections"
				:key="collection.id"
				class="collection"
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
							{{
								formatMessage(
									(collection.projects?.length || 0) === 1
										? messages.projectsCountOne
										: messages.projectsCountOther,
								)
							}}
						</div>
						<div class="stats">
							<template v-if="collection.status === 'listed'">
								<GlobeIcon aria-hidden="true" />
								<span>{{ formatMessage(messages.statusPublic) }}</span>
							</template>
							<template v-else-if="collection.status === 'unlisted'">
								<LinkIcon aria-hidden="true" />
								<span>{{ formatMessage(messages.statusUnlisted) }}</span>
							</template>
							<template v-else-if="collection.status === 'private'">
								<LockIcon aria-hidden="true" />
								<span>{{ formatMessage(messages.statusPrivate) }}</span>
							</template>
							<template v-else-if="collection.status === 'rejected'">
								<XIcon aria-hidden="true" />
								<span>{{ formatMessage(messages.statusRejected) }}</span>
							</template>
						</div>
					</div>
				</div>
			</button>
		</div>
	</div>
</template>

<style lang="scss" scoped>
/* The page is a column of cards now, in the surfaces and radii the rest of the
   app is built from — it used to be one big raised box with a grid inside it,
   which is why it read as a page from an older version of the launcher. */

.collections-grid {
	display: grid;
	grid-template-columns: repeat(auto-fill, minmax(20rem, 1fr));
	gap: var(--gap-md);
}

.collection {
	display: grid;
	grid-template-columns: auto 1fr;
	gap: var(--gap-md);
	padding: var(--gap-lg);
	border: none;
	border-radius: var(--radius-xl);
	background-color: var(--color-bg-raised);
	text-align: left;
	font: inherit;
	color: inherit;
	cursor: pointer;
	transition:
		background-color 0.15s ease,
		outline-color 0.15s ease;
	outline: 2px solid transparent;
	outline-offset: -2px;

	&:hover,
	&:focus-visible {
		background-color: var(--color-button-bg);
		outline-color: var(--color-brand);
	}

	.details {
		display: flex;
		flex-direction: column;
		gap: var(--gap-xs);
		min-width: 0;
	}

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
		margin-top: var(--gap-xs);
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

.empty-state {
	display: flex;
	flex-direction: column;
	align-items: center;
	justify-content: center;
	gap: var(--gap-sm);
	padding: 4rem var(--gap-lg);
	border-radius: var(--radius-xl);
	background-color: var(--color-bg-raised);
	text-align: center;
}
</style>
