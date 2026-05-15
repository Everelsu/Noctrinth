<script setup lang="ts">
import {
	BoxIcon,
	EditIcon,
	ExternalIcon,
	GlobeIcon,
	HeartIcon,
	LeftArrowIcon,
	LinkIcon,
	LockIcon,
	TrashIcon,
	XIcon,
} from '@modrinth/assets'
import {
	Avatar,
	ButtonStyled,
	ContentPageHeader,
	FilterPills,
	type FilterPillOption,
	injectNotificationManager,
	NavTabs,
	ProjectCard,
	useCompactNumber,
} from '@modrinth/ui'
import { openUrl } from '@tauri-apps/plugin-opener'
import dayjs from 'dayjs'
import relativeTime from 'dayjs/plugin/relativeTime'
import { computed, onMounted, ref, watch } from 'vue'
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

dayjs.extend(relativeTime)

const { handleError } = injectNotificationManager()
const { formatCompactNumber } = useCompactNumber()
const route = useRoute()
const router = useRouter()

const loading = ref(true)
const collection = ref<Collection | null>(null)
const projects = ref<any[]>([])
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
function buildTags(p: any): string[] {
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

const typeFilterOptions = computed<FilterPillOption[]>(() => {
	const seen = new Set<string>()
	for (const p of projects.value) {
		if (p.project_type) seen.add(p.project_type)
	}
	return Array.from(seen).map((t) => ({
		id: t,
		label: t.charAt(0).toUpperCase() + t.slice(1) + 's',
	}))
})

const showTypeFilter = computed(() => typeFilterOptions.value.length > 1)

const filteredProjects = computed(() => {
	if (typeFilters.value.length === 0) return projects.value
	return projects.value.filter((p) => typeFilters.value.includes(p.project_type))
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
		name: 'Followed projects',
		description: "Auto-generated collection of all the projects you're following.",
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
	} catch (e) {
		handleError(e)
	} finally {
		loading.value = false
	}
}

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
onMounted(load)
</script>

<template>
	<div class="p-6 flex flex-col gap-4">
		<CollectionEditModal ref="editModal" @saved="onEditSaved" />
		<CollectionDeleteModal ref="deleteModal" @deleted="onDeleted" />

		<NavTabs
			:links="[
				{
					label: 'Collections',
					href: `/dashboard/collections`,
					subpages: ['/collection/'],
				},
				{ label: 'Notifications', href: `/dashboard/notifications` },
			]"
		/>

		<div class="flex items-center gap-2">
			<ButtonStyled type="transparent" circular>
				<button v-tooltip="'Back to collections'" @click="router.push('/dashboard/collections')">
					<LeftArrowIcon />
				</button>
			</ButtonStyled>
			<span class="text-sm text-secondary">Collections</span>
		</div>

		<p v-if="loading">Loading...</p>

		<template v-else-if="collection">
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
						{{ projects.length === 1 ? 'project' : 'projects' }}
					</div>

					<div class="w-1.5 h-1.5 rounded-full bg-surface-5"></div>

					<div class="flex items-center gap-2 capitalize font-medium">
						<template v-if="isFollowing || collection.status === 'private'">
							<LockIcon class="size-4" aria-hidden="true" />
							<span>Private</span>
						</template>
						<template v-else-if="collection.status === 'listed'">
							<GlobeIcon class="size-4" aria-hidden="true" />
							<span>Public</span>
						</template>
						<template v-else-if="collection.status === 'unlisted'">
							<LinkIcon class="size-4" aria-hidden="true" />
							<span>Unlisted</span>
						</template>
						<template v-else-if="collection.status === 'rejected'">
							<XIcon class="size-4" aria-hidden="true" />
							<span>Rejected</span>
						</template>
					</div>

					<template v-if="collection.updated">
						<div class="w-1.5 h-1.5 rounded-full bg-surface-5"></div>
						<div class="flex items-center gap-2 font-medium">
							Updated {{ dayjs(collection.updated).fromNow() }}
						</div>
					</template>
				</template>
				<template #actions>
					<ButtonStyled v-if="isOwner">
						<button @click="openEdit">
							<EditIcon />
							Edit
						</button>
					</ButtonStyled>
					<ButtonStyled v-if="isOwner" color="red">
						<button @click="openDelete">
							<TrashIcon />
							Delete
						</button>
					</ButtonStyled>
					<ButtonStyled v-if="!isFollowing">
						<button @click="openOnWeb">
							<ExternalIcon />
							Open on web
						</button>
					</ButtonStyled>
				</template>
			</ContentPageHeader>

			<FilterPills
				v-if="showTypeFilter"
				v-model="typeFilters"
				:options="typeFilterOptions"
				class="mt-1"
			/>

			<div v-if="filteredProjects.length === 0" class="empty-state">
				<BoxIcon class="mx-auto h-12 w-12 text-secondary opacity-50" aria-hidden="true" />
				<p class="mt-4 text-lg font-medium text-contrast">
					{{ typeFilters.length ? `No matching projects` : isFollowing ? "You haven't followed any projects yet" : 'No projects in this collection' }}
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
							<button
								:disabled="removingId === p.id"
								@click.stop.prevent="removeProject(p.id)"
							>
								<HeartIcon />
								{{ removingId === p.id ? 'Unfollowing...' : 'Unfollow project' }}
							</button>
						</ButtonStyled>
					</template>
					<template v-else-if="isOwner" #actions>
						<ButtonStyled>
							<button
								v-tooltip="'Remove from collection'"
								:disabled="removingId === p.id"
								@click.stop.prevent="removeProject(p.id)"
							>
								<XIcon />
								{{ removingId === p.id ? 'Removing...' : 'Remove project' }}
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
