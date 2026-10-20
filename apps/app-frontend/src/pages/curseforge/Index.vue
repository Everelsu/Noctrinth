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
import { CheckIcon, ExternalIcon, PlusIcon, SpinnerIcon } from '@modrinth/assets'
import {
	Button,
	commonMessages,
	defineMessages,
	IconButton,
	injectNotificationManager,
	NavTabs,
	ProjectPageHeader,
	ProjectSidebarCompatibility,
	useVIntl,
} from '@modrinth/ui'
import { useQuery, useQueryClient } from '@tanstack/vue-query'
import { openUrl } from '@tauri-apps/plugin-opener'
import { computed, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'

import InstanceIndicator from '@/components/ui/InstanceIndicator.vue'
import {
	curseforgeDescriptionQueryOptions,
	curseforgeFilesQueryOptions,
	curseforgeIdOf,
	curseforgeKeys,
	curseforgeProjectQueryOptions,
	type CurseforgeSearchHit,
} from '@/helpers/curseforge'
import {
	get as getInstance,
	get_installed_project_ids as getInstalledProjectIds,
} from '@/helpers/instance'
import { get_game_versions, get_loaders } from '@/helpers/tags'
import type { GameInstance } from '@/helpers/types'
import { injectContentInstall } from '@/providers/content-install'
import { provideBreadcrumbParent, useBreadcrumb } from '@/shared/breadcrumbs'

const props = defineProps<{ id: string }>()

const { handleError } = injectNotificationManager()
const { formatMessage } = useVIntl()
const route = useRoute()
const router = useRouter()
const queryClient = useQueryClient()
const { install: installVersion } = injectContentInstall()

const messages = defineMessages({
	descriptionTab: { id: 'app.curseforge.project.description-tab', defaultMessage: 'Description' },
	versionsTab: { id: 'app.curseforge.project.versions-tab', defaultMessage: 'Versions' },
	galleryTab: { id: 'app.curseforge.project.gallery-tab', defaultMessage: 'Gallery' },
	openOnCurseforge: {
		id: 'app.curseforge.project.open-on-curseforge',
		defaultMessage: 'Open on CurseForge',
	},
	loadError: {
		id: 'app.curseforge.project.load-error',
		defaultMessage: 'This CurseForge project could not be loaded.',
	},
})

const projectQuery = useQuery(computed(() => curseforgeProjectQueryOptions(props.id)))
const descriptionQuery = useQuery(computed(() => curseforgeDescriptionQueryOptions(props.id)))
const filesQuery = useQuery(computed(() => curseforgeFilesQueryOptions(props.id)))
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
const installedIds = ref<Set<string>>(new Set())
const installing = ref(false)

async function refreshInstance() {
	if (!instanceId.value) {
		instance.value = null
		installedIds.value = new Set()
		return
	}
	try {
		const [loaded, ids] = await Promise.all([
			getInstance(instanceId.value),
			getInstalledProjectIds(instanceId.value),
		])
		instance.value = loaded
		installedIds.value = new Set(ids.filter((id) => /^\d+$/.test(id)).map(curseforgeIdOf))
	} catch (err) {
		handleError(err)
	}
}
watch(instanceId, refreshInstance, { immediate: true })

const projectType = computed(
	() => project.value?.project_type ?? summary.value?.project_types[0] ?? 'mod',
)
const isModpack = computed(() => projectType.value === 'modpack')
const installed = computed(() => !isModpack.value && installedIds.value.has(props.id))
const websiteUrl = computed(() => project.value?.website_url ?? summary.value?.website_url ?? null)

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
			followers: project.value.follows,
			categories: [] as string[],
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
			followers: summary.value.follows,
			categories: [] as string[],
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
/** What the shared Description and Gallery tabs read. */
const tabProject = computed(() => ({
	id: props.id,
	project_type: projectType.value,
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
const tabs = computed(() => [
	{
		label: formatMessage(messages.descriptionTab),
		href: `/curseforge/${props.id}${tabSuffix.value}`,
	},
	{
		label: formatMessage(messages.versionsTab),
		href: `/curseforge/${props.id}/versions${tabSuffix.value}`,
	},
	{
		label: formatMessage(messages.galleryTab),
		href: `/curseforge/${props.id}/gallery${tabSuffix.value}`,
		shown: (project.value?.gallery.length ?? 0) > 0,
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
						<PlusIcon v-else />
						{{
							installed
								? formatMessage(commonMessages.installedLabel)
								: installing
									? formatMessage(commonMessages.installingLabel)
									: formatMessage(commonMessages.installButton)
						}}
					</Button>
					<IconButton
						v-if="websiteUrl"
						v-tooltip="formatMessage(messages.openOnCurseforge)"
						type="quiet"
						size="xl"
						:label="formatMessage(messages.openOnCurseforge)"
						@click="openUrl(websiteUrl)"
					>
						<ExternalIcon />
					</IconButton>
				</template>
			</ProjectPageHeader>
			<NavTabs :links="tabs" />
			<RouterView
				:project="tabProject"
				:files="filesQuery.data.value ?? []"
				:files-loading="filesQuery.isPending.value"
				:instance="instance"
				:installed="installed"
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
