<script setup lang="ts">
import {
	ClipboardCopyIcon,
	DownloadIcon,
	ExternalIcon,
	MoreVerticalIcon,
	SpinnerIcon,
} from '@modrinth/assets'
import {
	Button,
	commonMessages,
	defineMessages,
	IconButton,
	injectNotificationManager,
	injectTags,
	NavTabs,
	ProjectPageHeader,
	ProjectPageVersions,
	ProjectSidebarCompatibility,
	TeleportOverflowMenu,
	useVIntl,
} from '@modrinth/ui'
import { useQuery } from '@tanstack/vue-query'
import { computed, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'

import Description from '@/pages/project/Description.vue'
import Gallery from '@/pages/project/Gallery.vue'
import { openExternalUrl } from '@/platform/external-links'
import { injectContentInstall } from '@/providers/content-install'
import { useBreadcrumb } from '@/shared/breadcrumbs'

import { ftbHit, ftbMessages, ftbPackQuery, ftbProjectPath, ftbVersions } from '../queries'

const props = defineProps<{ id: string; tab?: 'versions' | 'gallery' }>()
const { formatMessage } = useVIntl()
const { addNotification, handleError } = injectNotificationManager()
const { install: installVersion } = injectContentInstall()
const tags = injectTags(null)
const route = useRoute()
const router = useRouter()
const installing = ref(false)

const messages = defineMessages({
	descriptionTab: { id: 'app.project.tab.description', defaultMessage: 'Description' },
	versionsTab: { id: 'app.project.tab.versions', defaultMessage: 'Versions' },
	galleryTab: { id: 'app.project.tab.gallery', defaultMessage: 'Gallery' },
	linkCopied: { id: 'app.project.copy-link.copied', defaultMessage: 'Link copied to clipboard' },
})

const packQuery = useQuery(computed(() => ftbPackQuery(Number(props.id))))
const websiteUrl = computed(() => `https://www.feed-the-beast.com/modpacks/${Number(props.id)}`)
const moreActions = computed(() => [
	{
		id: 'copy-link',
		label: formatMessage(commonMessages.copyLinkButton),
		icon: ClipboardCopyIcon,
		action: async () => {
			const url = websiteUrl.value
			try {
				await navigator.clipboard.writeText(url)
				addNotification({ title: formatMessage(messages.linkCopied), text: url, type: 'success' })
			} catch (error) {
				handleError(error)
			}
		},
	},
	{
		id: 'open-in-browser',
		label: formatMessage(commonMessages.openInBrowserButton),
		icon: ExternalIcon,
		action: () => openExternalUrl(websiteUrl.value).catch(handleError),
	},
])
const pack = computed(() => {
	const data = packQuery.data.value
	return data && !data.private ? data : undefined
})
const hit = computed(() => (pack.value ? ftbHit(pack.value) : undefined))
const project = computed(() => {
	if (!pack.value || !hit.value) return
	return {
		id: hit.value.project_id,
		title: hit.value.name,
		description: hit.value.summary,
		icon_url: hit.value.icon_url,
		downloads: hit.value.downloads,
		followers: undefined as unknown as number,
		categories: [],
		status: 'approved' as const,
		project_type: 'modpack',
		actualProjectType: 'modpack',
		client_side: 'required',
		server_side: 'unknown',
		versions: hit.value.versions,
		game_versions: hit.value.versions,
		loaders: hit.value.loaders,
		body: pack.value.description?.trim() || hit.value.summary,
		gallery: pack.value.art
			.filter(({ type }) => type === 'screenshot' || type === 'splash')
			.map(({ url, title }) => ({ url, raw_url: url, title: title ?? pack.value?.name })),
	}
})

const versions = computed(() => {
	const currentPack = pack.value
	if (!currentPack) return []
	return ftbVersions(currentPack).map((version) => ({
		id: String(version.id),
		project_id: `ftb:${props.id}`,
		name: version.name,
		version_number: version.name,
		displayUrlEnding: String(version.id),
		version_type:
			version.type === 'alpha' || version.type === 'beta' ? version.type : ('release' as const),
		date_published: new Date(
			(version.released || version.updated || currentPack.released) * 1000,
		).toISOString(),
		game_versions: version.targets
			.filter(({ type }) => type === 'game')
			.map(({ version }) => version),
		loaders: version.targets.filter(({ type }) => type === 'modloader').map(({ name }) => name),
		author_id: '',
		status: 'listed' as const,
		featured: false,
		dependencies: [],
		files: [],
		downloads: 0,
	}))
})
const loaders = computed(() => tags?.loaders.value ?? [])
const gameVersions = computed(() => tags?.gameVersions.value ?? [])
const tabs = computed(() => {
	const base = ftbProjectPath(props.id)
	const href = (path: string) =>
		router.resolve({ path: base + path, query: { ...route.query, page: undefined } }).fullPath
	return [
		{ label: formatMessage(messages.descriptionTab), href: href('') },
		{ label: formatMessage(messages.versionsTab), href: href('/versions') },
		{
			label: formatMessage(messages.galleryTab),
			href: href('/gallery'),
			shown: !!project.value?.gallery.length,
		},
	]
})

useBreadcrumb({
	slot: 'project',
	id: () => `ftb:${props.id}`,
	label: () => project.value?.title ?? props.id,
	visual: () => ({
		type: 'image',
		src: project.value?.icon_url ?? undefined,
		alt: project.value?.title ?? props.id,
		tintBy: props.id,
	}),
	to: () => route.fullPath,
})

async function install(versionId: string | null = null) {
	if (installing.value || !project.value) return
	installing.value = true
	try {
		await installVersion(project.value.id, versionId, null, 'ProjectPage', undefined, (id) => {
			void router.push(`/instance/${encodeURIComponent(id)}`)
		})
	} catch (error) {
		handleError(error)
	} finally {
		installing.value = false
	}
}
</script>

<template>
	<div v-if="project" class="flex flex-col gap-4 p-6">
		<Teleport to="#sidebar-teleport-target">
			<ProjectSidebarCompatibility
				:project="project"
				:tags="{ loaders, gameVersions }"
				class="border-0 border-b border-solid border-[--brand-gradient-border] p-4"
			/>
		</Teleport>
		<ProjectPageHeader :project="project">
			<template #actions>
				<Button
					type="colored"
					color="brand"
					size="xl"
					:disabled="installing || !versions.length"
					@click="install()"
				>
					<SpinnerIcon v-if="installing" class="animate-spin" />
					<DownloadIcon v-else />
					{{
						formatMessage(
							installing ? commonMessages.installingLabel : commonMessages.installButton,
						)
					}}
				</Button>
				<TeleportOverflowMenu
					type="quiet"
					size="xl"
					:label="formatMessage(commonMessages.moreOptionsButton)"
					:options="moreActions"
				>
					<MoreVerticalIcon />
				</TeleportOverflowMenu>
			</template>
		</ProjectPageHeader>
		<div class="flex flex-wrap items-center justify-between gap-3">
			<NavTabs :links="tabs" />
			<div id="project-tabs-actions" class="flex items-center empty:hidden" />
		</div>
		<div :key="tab" class="nm-page-enter">
			<ProjectPageVersions
				v-if="tab === 'versions'"
				:project="project"
				:versions="versions"
				:loaders="loaders"
				:game-versions="gameVersions"
				:show-downloads="false"
			>
				<template #actions="{ version }">
					<IconButton
						v-tooltip="formatMessage(commonMessages.installButton)"
						type="quiet"
						color="green"
						:label="formatMessage(commonMessages.installButton)"
						:disabled="installing"
						@click.stop="install(version.id)"
					>
						<DownloadIcon />
					</IconButton>
				</template>
			</ProjectPageVersions>
			<Gallery v-else-if="tab === 'gallery' && project.gallery.length" :project="project" />
			<Description v-else :project="project" :fallback-description="project.description" />
		</div>
	</div>
	<div v-else-if="packQuery.isPending.value" class="flex justify-center p-12">
		<SpinnerIcon class="size-8 animate-spin text-secondary" />
	</div>
	<div v-else class="flex items-center gap-4 p-6 text-secondary">
		{{ formatMessage(ftbMessages.loadError) }}
		<Button v-if="packQuery.isError.value" @click="packQuery.refetch()">
			{{ formatMessage(commonMessages.retryButton) }}
		</Button>
	</div>
</template>
