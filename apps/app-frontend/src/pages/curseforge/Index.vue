<script setup lang="ts">
/**
 * Noctrinth's own: a CurseForge project's page.
 *
 * Built from the pieces a Modrinth project's page is built from — the same
 * header, compatibility sidebar, tabs, description and gallery — so a project
 * found on CurseForge looks like any other. What CurseForge does not have
 * (followers as Modrinth counts them, a team, licences) is simply not drawn.
 *
 * Nothing here waits before the page is drawn. The header comes straight from
 * the search hit that was clicked, if there was one, and everything else
 * fills in as it arrives — usually already there, because the browse page
 * starts loading a project the moment its card is hovered.
 */
import {
	CalendarIcon,
	CheckIcon,
	ClipboardCopyIcon,
	DownloadIcon,
	ExternalIcon,
	MoreVerticalIcon,
	SpinnerIcon,
	StarIcon,
	VersionIcon,
} from '@modrinth/assets'
import {
	Button,
	commonMessages,
	defineMessages,
	IconButton,
	injectNotificationManager,
	NavTabs,
	ProjectPageHeader,
	ProjectSidebarCompatibility,
	ProjectSidebarCreators,
	ProjectSidebarLinks,
	ProjectSidebarTags,
	TeleportOverflowMenu,
	useFormatDateTime,
	useRelativeTime,
	useVIntl,
} from '@modrinth/ui'
import { capitalizeString } from '@modrinth/utils'
import { useQuery, useQueryClient } from '@tanstack/vue-query'
import { openUrl } from '@tauri-apps/plugin-opener'
import { computed, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'

import InstanceIndicator from '@/components/ui/InstanceIndicator.vue'
import {
	curseforgeDescriptionQueryOptions,
	curseforgeFilesIn,
	curseforgeIdOf,
	curseforgeKeys,
	curseforgeProjectQueryOptions,
	type CurseforgeSearchHit,
	useCurseforgeFavorites,
} from '@/helpers/curseforge'
import {
	get as getInstance,
	get_linked_modpack_content,
	get_projects as getInstanceProjects,
} from '@/helpers/instance'
import { get_game_versions, get_loaders } from '@/helpers/tags'
import type { GameInstance } from '@/helpers/types'
import { injectContentInstall } from '@/providers/content-install'
import { provideBreadcrumbParent, useBreadcrumb } from '@/shared/breadcrumbs'

const props = defineProps<{ id: string }>()

const { handleError, addNotification } = injectNotificationManager()
const { formatMessage } = useVIntl()
const route = useRoute()
const router = useRouter()
const queryClient = useQueryClient()
const { install: installVersion } = injectContentInstall()

const messages = defineMessages({
	// The Modrinth project page's own ids, so their translations apply here too.
	descriptionTab: { id: 'app.project.tab.description', defaultMessage: 'Description' },
	versionsTab: { id: 'app.project.tab.versions', defaultMessage: 'Versions' },
	galleryTab: { id: 'app.project.tab.gallery', defaultMessage: 'Gallery' },
	openOnCurseforge: {
		id: 'app.curseforge.project.open-on-curseforge',
		defaultMessage: 'Open on CurseForge',
	},
	moreOptions: { id: 'app.project.more-options', defaultMessage: 'More options' },
	linkCopied: { id: 'app.project.copy-link.copied', defaultMessage: 'Link copied to clipboard' },
	addFavorite: {
		id: 'app.curseforge.project.add-favorite',
		defaultMessage: 'Add to favorites',
	},
	removeFavorite: {
		id: 'app.curseforge.project.remove-favorite',
		defaultMessage: 'Remove from favorites',
	},
	loadError: {
		id: 'app.curseforge.project.load-error',
		defaultMessage: 'This CurseForge project could not be loaded.',
	},
})

const projectQuery = useQuery(computed(() => curseforgeProjectQueryOptions(props.id)))
const descriptionQuery = useQuery(computed(() => curseforgeDescriptionQueryOptions(props.id)))
const project = computed(() => projectQuery.data.value ?? null)
/** The search hit that led here, enough to draw the header while loading. */
const summary = computed(
	() => queryClient.getQueryData<CurseforgeSearchHit>(curseforgeKeys.summary(props.id)) ?? null,
)

const allLoaders = ref<unknown[]>([])
const allGameVersions = ref<unknown[]>([])
get_loaders()
	.then((loaders) => (allLoaders.value = loaders ?? []))
	.catch(handleError)
get_game_versions()
	.then((versions) => (allGameVersions.value = versions ?? []))
	.catch(handleError)

const instanceId = computed(() => (typeof route.query.i === 'string' ? route.query.i : null))
const instance = ref<GameInstance | null>(null)
/** Each CurseForge project in the instance, by `cf-` id, to its file id. */
const installedFiles = ref<Map<string, string | null>>(new Map())
const installing = ref(false)

async function refreshInstance() {
	if (!instanceId.value) {
		instance.value = null
		installedFiles.value = new Map()
		return
	}
	try {
		const [loaded, files] = await Promise.all([
			getInstance(instanceId.value),
			getInstanceProjects(instanceId.value),
		])
		instance.value = loaded
		// A pack's own files, from a CurseForge zip, are known only to its listing.
		const packFiles =
			loaded?.link && loaded.link.type !== 'modrinth_modpack'
				? curseforgeFilesIn(
						(await get_linked_modpack_content(instanceId.value).catch(() => null)) ?? [],
					)
				: []
		installedFiles.value = new Map([
			...packFiles,
			...Object.values(files).flatMap((file) =>
				file.metadata && /^\d+$/.test(file.metadata.project_id)
					? [[curseforgeIdOf(file.metadata.project_id), file.metadata.version_id ?? null] as const]
					: [],
			),
		])
	} catch (err) {
		handleError(err)
	}
}
watch(instanceId, refreshInstance, { immediate: true })

const formatRelativeTime = useRelativeTime()
const formatDateTime = useFormatDateTime({ dateStyle: 'long', timeStyle: 'short' })

const projectType = computed(
	() => project.value?.project_type ?? summary.value?.project_types[0] ?? 'mod',
)
const isModpack = computed(() => projectType.value === 'modpack')
const installed = computed(() => !isModpack.value && installedFiles.value.has(props.id))
const installedVersion = computed(() => installedFiles.value.get(props.id) ?? null)
const websiteUrl = computed(() => project.value?.website_url ?? summary.value?.website_url ?? null)

/** Its CurseForge categories, by CurseForge's own names. */
const categoryTags = computed(() => [
	...new Set((project.value?.categories ?? []).map((c) => c.name)),
])

/** The project in the shape the shared header reads. */
const headerProject = computed(() => {
	if (project.value) {
		return {
			id: project.value.id,
			title: project.value.name,
			description: project.value.summary,
			icon_url: project.value.icon_url,
			raw_icon_url: project.value.icon_url,
			downloads: project.value.downloads,
			// CurseForge counts likes, not followers; the header leaves it out.
			followers: undefined as unknown as number,
			categories: categoryTags.value,
		}
	}
	if (summary.value) {
		return {
			id: summary.value.project_id,
			title: summary.value.name,
			description: summary.value.summary,
			icon_url: summary.value.icon_url,
			raw_icon_url: summary.value.icon_url,
			downloads: summary.value.downloads,
			followers: undefined as unknown as number,
			categories: summary.value.categories,
		}
	}
	return null
})
// The title bar names the project, as it does on a Modrinth project's page.
const breadcrumb = useBreadcrumb({
	slot: 'project',
	id: () => `curseforge:${props.id}`,
	label: () => headerProject.value?.title ?? props.id,
	visual: () => ({
		type: 'image' as const,
		src: headerProject.value?.icon_url ?? undefined,
		alt: headerProject.value?.title ?? props.id,
		tintBy: props.id,
	}),
	to: () => route.fullPath,
})
provideBreadcrumbParent(breadcrumb)

const sidebarProject = computed(() =>
	project.value
		? {
				project_type: project.value.project_type,
				versions: project.value.game_versions,
				game_versions: project.value.game_versions,
				loaders: project.value.loaders,
				client_side: 'unknown',
				server_side: 'unknown',
			}
		: null,
)
/** What the shared links section reads; CurseForge's page is its site. */
const linksProject = computed(() =>
	project.value
		? {
				issues_url: project.value.issues_url ?? '',
				source_url: project.value.source_url ?? '',
				wiki_url: project.value.wiki_url ?? '',
				discord_url: '',
				site_url: project.value.website_url ?? undefined,
				donation_urls: [],
			}
		: null,
)

/**
 * Its authors in the shape the shared creators section reads. CurseForge
 * lists the owner first and has no roles; each opens their CurseForge page.
 */
const creators = computed(() =>
	(project.value?.authors ?? []).map((author, index) => ({
		id: author.id ?? author.name,
		team_id: '',
		user: {
			id: author.id ?? author.name,
			username: author.name,
			avatar_url: author.avatar_url ?? undefined,
		},
		role: '',
		is_owner: index === 0,
		accepted: true,
		ordering: index,
	})),
)
function authorLink(username: string) {
	const author = project.value?.authors.find((a) => a.name === username)
	return author?.url ?? project.value?.website_url ?? ''
}

const createdAt = computed(() => project.value?.date_created ?? null)
const updatedAt = computed(() => project.value?.date_modified ?? null)

/** What the shared Description and Gallery tabs, and the versions tabs, read. */
const tabProject = computed(() => ({
	id: props.id,
	title: headerProject.value?.title ?? '',
	project_type: projectType.value,
	website_url: websiteUrl.value,
	authors: project.value?.authors ?? [],
	body: descriptionQuery.data.value ?? '',
	gallery: (project.value?.gallery ?? []).map((image) => ({
		url: image.url,
		raw_url: image.url,
		title: image.title,
		description: image.description,
		created: null,
	})),
}))

// NavTabs reads its hrefs as strings, so the instance travels as a query
// string rather than a route object.
const tabSuffix = computed(() =>
	instanceId.value ? `?i=${encodeURIComponent(instanceId.value)}` : '',
)
// Opened from an instance, the versions tab starts filtered to what fits it,
// through the same g/l query its filters read.
const versionsSuffix = computed(() => {
	if (!instance.value) return tabSuffix.value
	const params = new URLSearchParams({ i: instance.value.id, g: instance.value.game_version })
	if (projectType.value === 'mod' && instance.value.loader !== 'vanilla') {
		params.set('l', instance.value.loader)
	}
	return `?${params.toString()}`
})
const tabs = computed(() => [
	{
		label: formatMessage(messages.descriptionTab),
		href: `/curseforge/${props.id}${tabSuffix.value}`,
	},
	{
		label: formatMessage(messages.versionsTab),
		href: `/curseforge/${props.id}/versions${versionsSuffix.value}`,
	},
	{
		label: formatMessage(messages.galleryTab),
		href: `/curseforge/${props.id}/gallery${tabSuffix.value}`,
		shown: (project.value?.gallery.length ?? 0) > 0,
	},
])

const { isFavorite, toggleFavorite } = useCurseforgeFavorites()
const favorite = computed(() => isFavorite(props.id))

async function copyProjectLink() {
	if (!websiteUrl.value) return
	try {
		await navigator.clipboard.writeText(websiteUrl.value)
		addNotification({
			title: formatMessage(messages.linkCopied),
			text: websiteUrl.value,
			type: 'success',
		})
	} catch (err) {
		handleError(err)
	}
}

/** The header's ⋮ menu, as a Modrinth project's page has it. */
const moreActions = computed(() => [
	{
		id: 'copy-link',
		label: formatMessage(commonMessages.copyLinkButton),
		icon: ClipboardCopyIcon,
		action: copyProjectLink,
	},
	{
		id: 'open-in-browser',
		label: formatMessage(messages.openOnCurseforge),
		icon: ExternalIcon,
		action: () => websiteUrl.value && openUrl(websiteUrl.value),
	},
])

/**
 * Install the project, or one file of it, through the same flow a Modrinth
 * project's page uses: with its instance picker and compatibility checks when
 * the page was not opened from an instance, its incompatible-version warning
 * when it was, and a new instance for a modpack.
 */
async function install(fileId: string | null = null) {
	if (installing.value) return
	installing.value = true
	try {
		await installVersion(
			props.id,
			fileId,
			instanceId.value,
			'ProjectPage',
			(versionId) => {
				installing.value = false
				if (versionId) void refreshInstance()
			},
			(createdInstanceId) => {
				router.push(`/instance/${encodeURIComponent(createdInstanceId)}`)
			},
			{ showProjectInfo: !instanceId.value },
		)
	} catch (err) {
		handleError(err)
	} finally {
		installing.value = false
	}
}
</script>

<template>
	<div>
		<Teleport to="#sidebar-teleport-target">
			<ProjectSidebarCompatibility
				v-if="sidebarProject"
				:project="sidebarProject"
				:tags="{ loaders: allLoaders, gameVersions: allGameVersions }"
				class="project-sidebar-section"
			/>
			<ProjectSidebarLinks
				v-if="linksProject"
				link-target="_blank"
				:project="linksProject"
				class="project-sidebar-section"
			/>
			<ProjectSidebarTags
				v-if="categoryTags.length > 0"
				:project="{ categories: categoryTags, additional_categories: [] }"
				class="project-sidebar-section"
			/>
			<ProjectSidebarCreators
				v-if="creators.length > 0"
				:members="creators"
				:org-link="() => ''"
				:user-link="authorLink"
				link-target="_blank"
				user-link-target="_blank"
				class="project-sidebar-section"
			/>
			<div v-if="createdAt || updatedAt" class="project-sidebar-section flex flex-col gap-3">
				<h2 class="m-0 text-lg">{{ formatMessage(commonMessages.detailsLabel) }}</h2>
				<div
					class="flex flex-col gap-3 [&>div>svg]:mt-[1px] [&>div>svg]:shrink-0 [&>div]:flex [&>div]:items-start [&>div]:gap-2"
				>
					<div v-if="createdAt" v-tooltip="formatDateTime(createdAt)">
						<CalendarIcon aria-hidden="true" />
						<div>
							{{
								capitalizeString(
									formatMessage(commonMessages.projectCreated, {
										date: formatRelativeTime(createdAt),
									}),
								)
							}}
						</div>
					</div>
					<div v-if="updatedAt" v-tooltip="formatDateTime(updatedAt)">
						<VersionIcon aria-hidden="true" />
						<div>
							{{
								capitalizeString(
									formatMessage(commonMessages.projectUpdated, {
										date: formatRelativeTime(updatedAt),
									}),
								)
							}}
						</div>
					</div>
				</div>
			</div>
		</Teleport>
		<div v-if="headerProject" class="flex flex-col gap-4 p-6">
			<InstanceIndicator v-if="instance" :instance="instance" />
			<ProjectPageHeader :project="headerProject">
				<template #actions>
					<Button
						type="colored"
						color="brand"
						size="xl"
						native-type="button"
						:disabled="installed || installing"
						@click="install(null)"
					>
						<SpinnerIcon v-if="installing" class="animate-spin" />
						<CheckIcon v-else-if="installed" />
						<DownloadIcon v-else />
						{{
							installed
								? formatMessage(commonMessages.installedLabel)
								: installing
									? formatMessage(commonMessages.installingLabel)
									: formatMessage(commonMessages.installButton)
						}}
					</Button>
					<IconButton
						v-tooltip="formatMessage(favorite ? messages.removeFavorite : messages.addFavorite)"
						type="quiet"
						size="xl"
						:label="formatMessage(favorite ? messages.removeFavorite : messages.addFavorite)"
						@click="toggleFavorite(props.id)"
					>
						<StarIcon :class="favorite ? 'fill-current text-brand-orange' : ''" />
					</IconButton>
					<TeleportOverflowMenu
						v-if="websiteUrl"
						type="quiet"
						size="xl"
						:label="formatMessage(messages.moreOptions)"
						:options="moreActions"
					>
						<MoreVerticalIcon />
					</TeleportOverflowMenu>
				</template>
			</ProjectPageHeader>
			<NavTabs :links="tabs" />
			<RouterView
				:project="tabProject"
				:instance="instance"
				:installed="installed"
				:installed-version="installedVersion"
				:installing="installing"
				:install="install"
			/>
		</div>
		<div v-else-if="projectQuery.isPending.value" class="flex justify-center p-12">
			<SpinnerIcon class="size-8 animate-spin text-secondary" />
		</div>
		<p v-else class="p-6 text-secondary">{{ formatMessage(messages.loadError) }}</p>
	</div>
</template>

<style scoped lang="scss">
// The same sections, spacing and dividers as a Modrinth project's sidebar.
.project-sidebar-section {
	@apply flex flex-col gap-2 border-0 border-b-[1px] border-solid border-[--brand-gradient-border] p-4;
}
</style>
