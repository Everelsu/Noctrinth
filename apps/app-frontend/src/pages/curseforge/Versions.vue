<script setup lang="ts">
/**
 * Noctrinth's own: a CurseForge project's files, in the versions table a
 * Modrinth project uses. A file whose author keeps it off launchers is still
 * installable: it is then downloaded by hand, through the pending-files modal.
 * The files load only once this tab opens, so a project's page alone asks
 * CurseForge for nothing it does not draw.
 */
import { CheckIcon, DownloadIcon, ExternalIcon, SpinnerIcon } from '@modrinth/assets'
import {
	ButtonLink,
	commonMessages,
	defineMessages,
	IconButton,
	injectNotificationManager,
	ProjectPageVersions,
	useVIntl,
} from '@modrinth/ui'
import { useQuery } from '@tanstack/vue-query'
import { computed, ref } from 'vue'
import { useRoute } from 'vue-router'

import { SwapIcon } from '@/assets/icons'
import { curseforgeFilesQueryOptions, curseforgeFileUrl } from '@/helpers/curseforge'
import { get_game_versions, get_loaders } from '@/helpers/tags.js'

const props = defineProps<{
	project: { id: string; project_type?: string; website_url?: string | null }
	install: (fileId: string | null) => Promise<void>
	installing: boolean
	installed?: boolean
	installedVersion?: string | null
}>()

const { formatMessage } = useVIntl()
const { handleError } = injectNotificationManager()
const route = useRoute()

const messages = defineMessages({
	alreadyInstalled: {
		id: 'app.project.versions.already-installed',
		defaultMessage: 'Already installed',
	},
})

const filesQuery = useQuery(computed(() => curseforgeFilesQueryOptions(props.project.id)))

const loaders = ref([])
const gameVersions = ref([])
get_loaders()
	.then((value) => (loaders.value = value ?? []))
	.catch(handleError)
get_game_versions()
	.then((value) => (gameVersions.value = value ?? []))
	.catch(handleError)

/** Each file in the shape the shared versions table reads. */
const versions = computed(() =>
	(filesQuery.data.value ?? []).map((file) => ({
		id: file.id,
		project_id: file.project_id,
		name: file.name,
		version_number: file.name,
		version_type: file.version_type,
		game_versions: file.game_versions,
		// A mod file from before CurseForge tagged loaders is a Forge one, as
		// CurseForge's own app takes it.
		loaders:
			file.loaders.length > 0 || (props.project.project_type ?? 'mod') !== 'mod'
				? file.loaders
				: ['forge'],
		date_published: file.date_published ?? '',
		downloads: file.downloads,
		files: [{ filename: file.file_name, size: file.size ?? 0, primary: true }],
	})),
)

/** A file's own page, keeping the instance and the filters picked here. */
function versionLink(version: { id: string }) {
	const query = new URLSearchParams(
		Object.entries(route.query).flatMap(([key, value]) =>
			(Array.isArray(value) ? value : [value]).flatMap((v) => (v ? [[key, String(v)]] : [])),
		),
	).toString()
	const path = `/curseforge/${props.project.id}/version/${version.id}`
	return query ? `${path}?${query}` : path
}

function installLabel(id: string) {
	if (!props.installed) return formatMessage(commonMessages.installButton)
	return id === props.installedVersion
		? formatMessage(messages.alreadyInstalled)
		: formatMessage(commonMessages.switchToVersionButton)
}
</script>

<template>
	<div v-if="filesQuery.isPending.value" class="flex justify-center p-12">
		<SpinnerIcon class="size-8 animate-spin text-secondary" />
	</div>
	<ProjectPageVersions
		v-else
		:loaders="loaders"
		:game-versions="gameVersions"
		:versions="versions"
		:project="{ id: project.id, project_type: project.project_type ?? 'mod' }"
		:version-link="versionLink"
	>
		<template #actions="{ version }">
			<IconButton
				v-tooltip="installLabel(version.id)"
				type="quiet"
				:color="installed && version.id === installedVersion ? undefined : 'green'"
				:label="installLabel(version.id)"
				:disabled="installing || (installed && version.id === installedVersion)"
				@click.stop="() => install(version.id)"
			>
				<DownloadIcon v-if="!installed" />
				<SwapIcon v-else-if="version.id !== installedVersion" />
				<CheckIcon v-else />
			</IconButton>
			<ButtonLink
				v-if="project.website_url"
				v-tooltip="formatMessage(commonMessages.openInBrowserButton)"
				type="quiet"
				:href="curseforgeFileUrl(project.website_url, version.id)"
				target="_blank"
				class="!w-9 !px-0 !rounded-full"
			>
				<ExternalIcon />
			</ButtonLink>
		</template>
	</ProjectPageVersions>
</template>
