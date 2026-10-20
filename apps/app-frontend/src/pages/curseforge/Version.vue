<script setup lang="ts">
/**
 * Noctrinth's own: one CurseForge file on the version page a Modrinth version
 * has, with its changelog and what it needs, can do without, or cannot run
 * beside.
 */
import type { Labrinth } from '@modrinth/api-client'
import {
	CheckIcon,
	DownloadIcon,
	ExternalIcon,
	MoreVerticalIcon,
	SpinnerIcon,
	VersionIcon,
} from '@modrinth/assets'
import {
	BackToParentLink,
	Button,
	commonMessages,
	defineMessages,
	type DependencyContext,
	TeleportOverflowMenu,
	useVIntl,
	VersionPage,
} from '@modrinth/ui'
import { useQuery } from '@tanstack/vue-query'
import { computed } from 'vue'
import { useRoute } from 'vue-router'

import { SwapIcon } from '@/assets/icons'
import { curseforgeFileUrl, curseforgeVersionQueryOptions } from '@/helpers/curseforge'
import { useBreadcrumb } from '@/shared/breadcrumbs'

const props = defineProps<{
	project: {
		id: string
		project_type?: string
		website_url?: string | null
		authors?: { name: string; url: string | null; avatar_url: string | null }[]
	}
	install: (fileId: string | null) => Promise<void>
	installing: boolean
	installed?: boolean
	installedVersion?: string | null
}>()

const { formatMessage } = useVIntl()
const route = useRoute()

const messages = defineMessages({
	allVersions: {
		id: 'app.project.version.all-versions',
		defaultMessage: 'All versions',
	},
	installing: {
		id: 'app.project.version.installing',
		defaultMessage: 'Installing',
	},
	loadError: {
		id: 'app.curseforge.project.load-error',
		defaultMessage: 'This CurseForge project could not be loaded.',
	},
})

/** Stands for the project's owner as a version's uploader; see `version`. */
const AUTHOR_ID = 'curseforge-owner'

const fileId = computed(() => String(route.params.version ?? ''))
const versionQuery = useQuery(
	computed(() => ({
		...curseforgeVersionQueryOptions(props.project.id, fileId.value),
		enabled: !!fileId.value,
	})),
)

/** The file in the shape the shared version page reads. */
const version = computed(() => {
	const file = versionQuery.data.value
	if (!file) return null
	return {
		id: file.id,
		project_id: file.project_id,
		name: file.name,
		version_number: file.name,
		version_type: file.version_type,
		game_versions: file.game_versions,
		// As in the versions list: an untagged mod file is a Forge one.
		loaders:
			file.loaders.length > 0 || (props.project.project_type ?? 'mod') !== 'mod'
				? file.loaders
				: ['forge'],
		date_published: file.date_published ?? '',
		downloads: file.downloads,
		changelog: file.changelog,
		dependencies: file.dependencies.map((dependency) => ({
			project_id: dependency.project_id,
			dependency_type: dependency.dependency_type,
			version_id: null,
			file_name: dependency.name,
		})),
		files: [{ filename: file.file_name, size: file.size ?? 0, primary: true, url: '', hashes: {} }],
		// The project's owner stands in for the uploader, whom CurseForge
		// does not name per file.
		author_id: AUTHOR_ID,
		featured: false,
		status: 'listed',
	} as unknown as Labrinth.Versions.v3.Version
})

const owner = computed(() => props.project.authors?.[0])
const members = computed(
	() =>
		[
			{
				user: {
					id: AUTHOR_ID,
					username: owner.value?.name ?? '',
					avatar_url: owner.value?.avatar_url,
				},
			},
		] as unknown as Labrinth.Projects.v3.TeamMember[],
)

const enrichment = computed(
	() =>
		({
			projects: (versionQuery.data.value?.dependencies ?? [])
				.filter((dependency) => dependency.name)
				.map((dependency) => ({
					id: dependency.project_id,
					slug: dependency.slug,
					title: dependency.name,
					icon_url: dependency.icon_url,
					project_type: dependency.project_type,
				})),
			versions: [],
		}) as unknown as Labrinth.Projects.v2.DependencyInfo,
)

function withQuery(path: string) {
	const query = new URLSearchParams(
		Object.entries(route.query).flatMap(([key, value]) =>
			(Array.isArray(value) ? value : [value]).flatMap((v) => (v ? [[key, String(v)]] : [])),
		),
	).toString()
	return query ? `${path}?${query}` : path
}

function dependencyLink(context: DependencyContext) {
	return context.project ? withQuery(`/curseforge/${context.project.id}`) : undefined
}

useBreadcrumb({
	slot: 'project-version',
	id: () => `curseforge-version:${props.project.id}:${fileId.value}`,
	label: () => {
		const label = formatMessage(commonMessages.versionLabel)
		return version.value ? `${label} ${version.value.version_number}` : label
	},
	visual: { type: 'icon', component: VersionIcon },
	to: () => route.fullPath,
})

const isInstalled = computed(() => !!props.installed && props.installedVersion === fileId.value)
const moreOptions = computed(() =>
	props.project.website_url
		? [
				{
					id: 'open-in-browser',
					label: formatMessage(commonMessages.openInBrowserButton),
					type: 'link' as const,
					href: curseforgeFileUrl(props.project.website_url, fileId.value),
					target: '_blank',
				},
			]
		: [],
)
</script>

<template>
	<div class="flex flex-col">
		<BackToParentLink :to="withQuery(`/curseforge/${project.id}/versions`)">
			{{ formatMessage(messages.allVersions) }}
		</BackToParentLink>
		<div v-if="versionQuery.isPending.value" class="flex justify-center p-12">
			<SpinnerIcon class="size-8 animate-spin text-secondary" />
		</div>
		<p v-else-if="!version" class="p-6 text-secondary">{{ formatMessage(messages.loadError) }}</p>
		<VersionPage
			v-else
			:version="version"
			:enrichment="enrichment"
			:members="members"
			:dependency-link-creator="dependencyLink"
			:user-link-creator="() => owner?.url ?? undefined"
		>
			<template #headerActions>
				<Button
					type="colored"
					color="brand"
					:disabled="installing || isInstalled"
					@click="() => install(fileId)"
				>
					<DownloadIcon v-if="!installed" />
					<SwapIcon v-else-if="!isInstalled" />
					<CheckIcon v-else />
					{{
						installing
							? formatMessage(messages.installing)
							: isInstalled
								? formatMessage(commonMessages.installedLabel)
								: installed
									? formatMessage(commonMessages.switchToVersionButton)
									: formatMessage(commonMessages.installButton)
					}}
				</Button>
				<TeleportOverflowMenu
					v-if="moreOptions.length > 0"
					type="outlined"
					:label="formatMessage(commonMessages.moreOptionsButton)"
					:tooltip="formatMessage(commonMessages.moreOptionsButton)"
					:options="moreOptions"
				>
					<MoreVerticalIcon aria-hidden="true" />
					<template #open-in-browser>
						<ExternalIcon aria-hidden="true" />
						{{ formatMessage(commonMessages.openInBrowserButton) }}
					</template>
				</TeleportOverflowMenu>
			</template>
		</VersionPage>
	</div>
</template>
