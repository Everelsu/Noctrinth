<script setup lang="ts">
import {
	BoxIcon,
	EditIcon,
	ExternalIcon,
	GlobeIcon,
	HeartIcon,
	LinkIcon,
	LockIcon,
	SearchIcon,
	TrashIcon,
	XIcon,
} from '@modrinth/assets'
import {
	Avatar,
	ButtonStyled,
	ContentPageHeader,
	defineMessages,
	DropdownSelect,
	type FilterPillOption,
	FilterPills,
	injectNotificationManager,
	LoadingIndicator,
	NavTabs,
	ProjectCard,
	StyledInput,
	useCompactNumber,
	useVIntl,
} from '@modrinth/ui'
import { openUrl } from '@tauri-apps/plugin-opener'
import dayjs from 'dayjs'
import relativeTime from 'dayjs/plugin/relativeTime'
import { computed, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'

import CollectionDeleteModal from '@/components/ui/modal/CollectionDeleteModal.vue'
import CollectionEditModal from '@/components/ui/modal/CollectionEditModal.vue'
import { get_project_many } from '@/helpers/cache.js'
import {
	type Collection,
	getCollection,
	getUserFollowedProjects,
	removeProjectFromCollection,
	unfollowProject,
} from '@/helpers/modrinth-api'
import { get as getCreds } from '@/helpers/mr_auth.ts'
import { useBreadcrumbs } from '@/store/breadcrumbs'

dayjs.extend(relativeTime)

const { handleError } = injectNotificationManager()
const { formatCompactNumber } = useCompactNumber()
const { formatMessage } = useVIntl()
const route = useRoute()
const router = useRouter()
const breadcrumbs = useBreadcrumbs()

const messages = defineMessages({
	tabCollections: { id: 'collection.tab.collections', defaultMessage: 'Collections' },
	tabNotifications: { id: 'collection.tab.notifications', defaultMessage: 'Notifications' },
	loading: { id: 'collection.loading', defaultMessage: 'Loading...' },
	statusPrivate: { id: 'collection.status.private', defaultMessage: 'Private' },
	statusPublic: { id: 'collection.status.public', defaultMessage: 'Public' },
	statusUnlisted: { id: 'collection.status.unlisted', defaultMessage: 'Unlisted' },
	statusRejected: { id: 'collection.status.rejected', defaultMessage: 'Rejected' },
	projectsCountOne: { id: 'collection.projects-count.one', defaultMessage: 'project' },
	projectsCountOther: { id: 'collection.projects-count.other', defaultMessage: 'projects' },
	updatedAgo: { id: 'collection.updated-ago', defaultMessage: 'Updated {time}' },
	edit: { id: 'collection.edit', defaultMessage: 'Edit' },
	delete: { id: 'collection.delete', defaultMessage: 'Delete' },
	openOnWeb: { id: 'collection.open-on-web', defaultMessage: 'Open on web' },
	noMatching: { id: 'collection.no-matching', defaultMessage: 'No matching projects' },
	noFollowed: {
		id: 'collection.no-followed',
		defaultMessage: "You haven't followed any projects yet",
	},
	noProjects: { id: 'collection.no-projects', defaultMessage: 'No projects in this collection' },
	unfollowProject: { id: 'collection.unfollow-project', defaultMessage: 'Unfollow project' },
	unfollowing: { id: 'collection.unfollowing', defaultMessage: 'Unfollowing...' },
	removeProject: { id: 'collection.remove-project', defaultMessage: 'Remove project' },
	removing: { id: 'collection.removing', defaultMessage: 'Removing...' },
	removeTooltip: { id: 'collection.remove-tooltip', defaultMessage: 'Remove from collection' },
	sortBy: { id: 'collection.sort-by', defaultMessage: 'Sort by: ' },
	sortByName: { id: 'collection.sort-by-name', defaultMessage: 'Sort by' },
	sortNameAsc: { id: 'collection.sort.name', defaultMessage: 'Name (A-Z)' },
	sortDownloads: { id: 'collection.sort.downloads', defaultMessage: 'Downloads' },
	sortFollowers: { id: 'collection.sort.followers', defaultMessage: 'Followers' },
	sortUpdated: { id: 'collection.sort.updated', defaultMessage: 'Recently updated' },
	searchPlaceholder: { id: 'collection.search-placeholder', defaultMessage: 'Search projects...' },
	followingName: { id: 'collection.following.name', defaultMessage: 'Followed projects' },
	followingDescription: {
		id: 'collection.following.description',
		defaultMessage: "Auto-generated collection of all the projects you're following.",
	},
	filterAll: { id: 'collection.filter.all', defaultMessage: 'All' },
	typeMod: { id: 'collection.type.mod', defaultMessage: 'Mods' },
	typeModpack: { id: 'collection.type.modpack', defaultMessage: 'Modpacks' },
	typeResourcepack: { id: 'collection.type.resourcepack', defaultMessage: 'Resource packs' },
	typeShader: { id: 'collection.type.shader', defaultMessage: 'Shaders' },
	typeDatapack: { id: 'collection.type.datapack', defaultMessage: 'Data packs' },
	typePlugin: { id: 'collection.type.plugin', defaultMessage: 'Plugins' },
})

const TYPE_LABELS: Record<string, (typeof messages)[keyof typeof messages]> = {
	mod: messages.typeMod,
	modpack: messages.typeModpack,
	resourcepack: messages.typeResourcepack,
	shader: messages.typeShader,
	datapack: messages.typeDatapack,
	plugin: messages.typePlugin,
}

function formatTypeLabel(type: string): string {
	const message = TYPE_LABELS[type]
	return message ? formatMessage(message) : type.charAt(0).toUpperCase() + type.slice(1) + 's'
}

const searchQuery = ref('')

type SortMode = 'name' | 'downloads' | 'followers' | 'updated'
const SORT_OPTIONS: SortMode[] = ['name', 'downloads', 'followers', 'updated']
const SORT_STORAGE_KEY = 'noctrinth:collection:followed-sort'

function loadInitialSort(): SortMode {
	try {
		const raw = localStorage.getItem(SORT_STORAGE_KEY)
		if (raw && (SORT_OPTIONS as string[]).includes(raw)) return raw as SortMode
	} catch {
		// ignore
	}
	return 'name'
}

const sortMode = ref<SortMode>(loadInitialSort())

function setSortMode(value: SortMode) {
	sortMode.value = value
	try {
		localStorage.setItem(SORT_STORAGE_KEY, value)
	} catch {
		// ignore
	}
}

function formatSortOption(option: SortMode): string {
	switch (option) {
		case 'name':
			return formatMessage(messages.sortNameAsc)
		case 'downloads':
			return formatMessage(messages.sortDownloads)
		case 'followers':
			return formatMessage(messages.sortFollowers)
		case 'updated':
			return formatMessage(messages.sortUpdated)
	}
}

interface CollectionProject {
	id: string
	slug?: string
	title?: string
	name?: string
	description?: string
	summary?: string
	project_type?: string
	project_types?: string[]
	display_categories?: string[]
	categories?: string[]
	loaders?: string[]
	[key: string]: unknown
}

const loading = ref(true)
const collection = ref<Collection | null>(null)
const projects = ref<CollectionProject[]>([])
const currentUserId = ref<string | null>(null)
const removingId = ref<string | null>(null)
const typeFilters = ref<string[]>([])

/** Strip common markdown / HTML so ProjectCard summary is plain text. */
function stripMarkdown(text: string | null | undefined): string {
	if (!text) return ''
	return text
		.replace(/<[^>]*>/g, '')
		.replace(/!\[.*?\]\(.*?\)/g, '')
		.replace(/\[([^\]]*)\]\([^)]*\)/g, '$1')
		.replace(/^#{1,6}\s+/gm, '')
		.replace(/\*{1,3}([^*\n]+)\*{1,3}/g, '$1')
		.replace(/_{1,2}([^_\n]+)_{1,2}/g, '$1')
		.replace(/`{1,3}[^`]*`{1,3}/g, '')
		.replace(/^>\s+/gm, '')
		.replace(/^[-*_]{3,}$/gm, '')
		.replace(/\n+/g, ' ')
		.trim()
}

/**
 * Build the tag list for a project card. Combines display_categories, categories, and loaders
 * (de-duped), so users see both "adventure"/"library" tags AND the mod loaders (forge, fabric...).
 * The website does the same thing on its collection page.
 */
function buildTags(p: CollectionProject): string[] {
	const seen = new Set<string>()
	const out: string[] = []
	const push = (arr?: string[] | null) => {
		for (const t of arr ?? []) {
			if (!t || seen.has(t)) continue
			seen.add(t)
			out.push(t)
		}
	}
	push(p.display_categories)
	push(p.categories)
	push(p.loaders)
	return out.slice(0, 5)
}

/**
 * Resolves a project's type from either API shape: regular collections use the
 * v2 cache (`project_type` string) while followed projects come from a v3
 * endpoint (`project_types` array). Without this, the Followed view had no
 * type filter because `project_type` was always undefined there.
 */
function getProjectType(p: CollectionProject): string | undefined {
	if (p.project_type) return p.project_type
	if (Array.isArray(p.project_types) && p.project_types.length > 0) {
		return p.project_types[0]
	}
	return undefined
}

const typeFilterOptions = computed<FilterPillOption[]>(() => {
	const seen = new Set<string>()
	for (const p of projects.value) {
		const t = getProjectType(p)
		if (t) seen.add(t)
	}
	return Array.from(seen).map((t) => ({
		id: t,
		label: formatTypeLabel(t),
	}))
})

const showTypeFilter = computed(() => typeFilterOptions.value.length > 1)

const filteredProjects = computed(() => {
	const q = searchQuery.value.trim().toLowerCase()
	const list = projects.value.filter((p) => {
		if (typeFilters.value.length > 0) {
			const t = getProjectType(p)
			if (t == null || !typeFilters.value.includes(t)) return false
		}
		if (q) {
			const haystack =
				`${p.title ?? p.name ?? ''} ${p.slug ?? ''} ${p.description ?? p.summary ?? ''}`.toLowerCase()
			if (!haystack.includes(q)) return false
		}
		return true
	})

	const sorted = [...list]
	switch (sortMode.value) {
		case 'name':
			sorted.sort((a, b) =>
				String(a.title ?? a.name ?? '').localeCompare(String(b.title ?? b.name ?? '')),
			)
			break
		case 'downloads':
			sorted.sort((a, b) => (b.downloads ?? 0) - (a.downloads ?? 0))
			break
		case 'followers':
			sorted.sort((a, b) => (b.follows ?? b.followers ?? 0) - (a.follows ?? a.followers ?? 0))
			break
		case 'updated':
			sorted.sort(
				(a, b) =>
					new Date(b.date_modified ?? b.updated ?? 0).getTime() -
					new Date(a.date_modified ?? a.updated ?? 0).getTime(),
			)
			break
	}
	return sorted
})

const editModal = ref<InstanceType<typeof CollectionEditModal>>()
const deleteModal = ref<InstanceType<typeof CollectionDeleteModal>>()

const isFollowing = computed(() => route.params.id === 'following')
const isOwner = computed(
	() =>
		!isFollowing.value &&
		!!currentUserId.value &&
		!!collection.value &&
		collection.value.user === currentUserId.value,
)

async function loadFollowing() {
	const creds = await getCreds()
	if (!creds) {
		throw new Error('Please sign in to view your followed projects.')
	}
	const followed = await getUserFollowedProjects(creds.user_id)
	collection.value = {
		id: 'following',
		user: creds.user_id,
		name: formatMessage(messages.followingName),
		description: formatMessage(messages.followingDescription),
		icon_url: 'https://cdn.modrinth.com/follow-collection.png',
		color: null,
		status: 'private',
		created: '',
		updated: '',
		projects: followed.map((p) => p.id),
	}
	projects.value = followed
}

async function loadCollection(id: string) {
	const col = await getCollection(id)
	collection.value = col
	if (col.projects && col.projects.length > 0) {
		const fetched = await get_project_many(col.projects)
		projects.value = (fetched || []).filter(Boolean)
	} else {
		projects.value = []
	}
}

async function load() {
	loading.value = true
	projects.value = []
	collection.value = null
	typeFilters.value = []
	try {
		const creds = await getCreds()
		currentUserId.value = creds?.user_id ?? null
		const id = String(route.params.id)
		if (id === 'following') {
			await loadFollowing()
		} else {
			await loadCollection(id)
		}
		if (collection.value) {
			breadcrumbs.setName('Collection', collection.value.name)
		}
	} catch (e) {
		handleError(e)
	} finally {
		loading.value = false
	}
}

// Top-level await blocks <Suspense> until the initial load finishes — the
// app's top loading bar shows during the wait and the page only renders
// when fully ready. The route-param watch below handles in-place navigation
// between different collections (still uses `loading` for that case).
await load()

function openEdit() {
	if (collection.value) editModal.value?.show(collection.value)
}

function openDelete() {
	if (collection.value) deleteModal.value?.show(collection.value)
}

function onEditSaved(updated: Collection) {
	collection.value = updated
}

function onDeleted() {
	router.push('/dashboard/collections')
}

async function removeProject(projectId: string) {
	if (!collection.value) return
	const c = collection.value
	removingId.value = projectId
	try {
		if (isFollowing.value) {
			await unfollowProject(projectId)
		} else {
			await removeProjectFromCollection(c.id, projectId, c.projects)
			collection.value = {
				...c,
				projects: c.projects.filter((p) => p !== projectId),
				updated: new Date().toISOString(),
			}
		}
		projects.value = projects.value.filter((p) => p.id !== projectId)
	} catch (e) {
		handleError(e)
	} finally {
		removingId.value = null
	}
}

function openOnWeb() {
	if (!collection.value) return
	openUrl(`https://modrinth.com/collection/${collection.value.id}`)
}

watch(
	() => route.params.id,
	(id) => {
		// Only reload when the user navigates between collections (still on Collection route).
		// Without this guard, navigating to /project/:id briefly fires this watcher with the
		// project id, causing a phantom "load collection" 404.
		if (route.name === 'Collection' && id) load()
	},
)
</script>

<template>
	<div v-if="loading" class="p-6 flex justify-center py-16">
		<LoadingIndicator />
	</div>
	<div v-else class="p-6 flex flex-col gap-4">
		<CollectionEditModal ref="editModal" @saved="onEditSaved" />
		<CollectionDeleteModal ref="deleteModal" @deleted="onDeleted" />

		<NavTabs
			:links="[
				{
					label: formatMessage(messages.tabCollections),
					href: `/dashboard/collections`,
					subpages: ['/collection/'],
				},
				{ label: formatMessage(messages.tabNotifications), href: `/dashboard/notifications` },
			]"
		/>

		<template v-if="collection">
			<ContentPageHeader>
				<template #icon>
					<Avatar
						:src="collection.icon_url ?? undefined"
						:alt="collection.name"
						size="64px"
						:tint-by="collection.id"
					/>
				</template>
				<template #title>
					{{ collection.name }}
				</template>
				<template v-if="collection.description" #summary>
					{{ collection.description }}
				</template>
				<template #stats>
					<div class="flex items-center gap-2 font-medium">
						<BoxIcon class="size-4" aria-hidden="true" />
						{{ formatCompactNumber(projects.length) }}
						{{
							formatMessage(
								projects.length === 1 ? messages.projectsCountOne : messages.projectsCountOther,
							)
						}}
					</div>

					<div class="w-1.5 h-1.5 rounded-full bg-surface-5"></div>

					<div class="flex items-center gap-2 capitalize font-medium">
						<template v-if="isFollowing || collection.status === 'private'">
							<LockIcon class="size-4" aria-hidden="true" />
							<span>{{ formatMessage(messages.statusPrivate) }}</span>
						</template>
						<template v-else-if="collection.status === 'listed'">
							<GlobeIcon class="size-4" aria-hidden="true" />
							<span>{{ formatMessage(messages.statusPublic) }}</span>
						</template>
						<template v-else-if="collection.status === 'unlisted'">
							<LinkIcon class="size-4" aria-hidden="true" />
							<span>{{ formatMessage(messages.statusUnlisted) }}</span>
						</template>
						<template v-else-if="collection.status === 'rejected'">
							<XIcon class="size-4" aria-hidden="true" />
							<span>{{ formatMessage(messages.statusRejected) }}</span>
						</template>
					</div>

					<template v-if="collection.updated">
						<div class="w-1.5 h-1.5 rounded-full bg-surface-5"></div>
						<div class="flex items-center gap-2 font-medium">
							{{
								formatMessage(messages.updatedAgo, { time: dayjs(collection.updated).fromNow() })
							}}
						</div>
					</template>
				</template>
				<template #actions>
					<ButtonStyled v-if="isOwner">
						<button @click="openEdit">
							<EditIcon />
							{{ formatMessage(messages.edit) }}
						</button>
					</ButtonStyled>
					<ButtonStyled v-if="isOwner" color="red">
						<button @click="openDelete">
							<TrashIcon />
							{{ formatMessage(messages.delete) }}
						</button>
					</ButtonStyled>
					<ButtonStyled v-if="!isFollowing">
						<button @click="openOnWeb">
							<ExternalIcon />
							{{ formatMessage(messages.openOnWeb) }}
						</button>
					</ButtonStyled>
				</template>
			</ContentPageHeader>

			<StyledInput
				v-model="searchQuery"
				:icon="SearchIcon"
				type="text"
				clearable
				:placeholder="formatMessage(messages.searchPlaceholder)"
				wrapper-class="w-full mt-1"
				input-class="!h-11"
			/>
			<div class="flex flex-wrap items-center gap-2">
				<FilterPills v-if="showTypeFilter" v-model="typeFilters" :options="typeFilterOptions">
					<template #all>{{ formatMessage(messages.filterAll) }}</template>
				</FilterPills>
				<DropdownSelect
					v-slot="{ selected }"
					:model-value="sortMode"
					class="!w-auto ml-auto"
					:name="formatMessage(messages.sortByName)"
					:options="SORT_OPTIONS"
					:display-name="formatSortOption"
					@update:model-value="setSortMode"
				>
					<span class="font-semibold text-primary">{{ formatMessage(messages.sortBy) }}</span>
					<span class="font-semibold text-secondary">{{ selected }}</span>
				</DropdownSelect>
			</div>

			<div v-if="filteredProjects.length === 0" class="empty-state">
				<BoxIcon class="mx-auto h-12 w-12 text-secondary opacity-50" aria-hidden="true" />
				<p class="mt-4 text-lg font-medium text-contrast">
					{{
						formatMessage(
							typeFilters.length
								? messages.noMatching
								: isFollowing
									? messages.noFollowed
									: messages.noProjects,
						)
					}}
				</p>
			</div>

			<div v-else class="project-list">
				<ProjectCard
					v-for="p in filteredProjects"
					:key="p.id"
					layout="list"
					:link="`/project/${p.id || p.slug}`"
					:icon-url="p.icon_url"
					:title="p.title || p.name"
					:summary="stripMarkdown(p.summary || p.description)"
					:tags="buildTags(p)"
					:downloads="p.downloads"
					:followers="p.follows ?? p.followers"
					:date-updated="p.date_modified || p.updated"
					:color="p.color"
				>
					<template v-if="isFollowing" #actions>
						<ButtonStyled>
							<button :disabled="removingId === p.id" @click.stop.prevent="removeProject(p.id)">
								<HeartIcon />
								{{
									formatMessage(
										removingId === p.id ? messages.unfollowing : messages.unfollowProject,
									)
								}}
							</button>
						</ButtonStyled>
					</template>
					<template v-else-if="isOwner" #actions>
						<ButtonStyled>
							<button
								v-tooltip="formatMessage(messages.removeTooltip)"
								:disabled="removingId === p.id"
								@click.stop.prevent="removeProject(p.id)"
							>
								<XIcon />
								{{
									formatMessage(removingId === p.id ? messages.removing : messages.removeProject)
								}}
							</button>
						</ButtonStyled>
					</template>
				</ProjectCard>
			</div>
		</template>
	</div>
</template>

<style lang="scss" scoped>
.project-list {
	display: flex;
	flex-direction: column;
	gap: var(--gap-sm);
}

.empty-state {
	display: flex;
	flex-direction: column;
	align-items: center;
	justify-content: center;
	padding: 3rem 1rem;
	text-align: center;
}
</style>
