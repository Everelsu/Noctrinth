<script setup lang="ts">
/**
 * Noctrinth's own: a CurseForge project's files, in the versions table a
 * Modrinth project uses. A file whose author keeps it off launchers is still
 * installable: it is then downloaded by hand, through the pending-files modal.
 */
import { DownloadIcon } from '@modrinth/assets'
import {
	commonMessages,
	IconButton,
	injectNotificationManager,
	ProjectPageVersions,
	useVIntl,
} from '@modrinth/ui'
import { computed, ref } from 'vue'

import type { CurseforgeFile } from '@/helpers/curseforge'
import { get_game_versions, get_loaders } from '@/helpers/tags.js'

const props = defineProps<{
	project: { id: string; project_type?: string }
	files: CurseforgeFile[]
	install: (fileId: string | null) => Promise<void>
	installing: boolean
}>()

const { formatMessage } = useVIntl()
const { handleError } = injectNotificationManager()

const [loaders, gameVersions] = await Promise.all([
	get_loaders().catch(handleError).then(ref),
	get_game_versions().catch(handleError).then(ref),
])

/** Each file in the shape the shared versions table reads. */
const versions = computed(() =>
	props.files.map((file) => ({
		id: file.id,
		project_id: file.project_id,
		name: file.name,
		version_number: file.name,
		version_type: file.version_type,
		game_versions: file.game_versions,
		loaders: file.loaders,
		date_published: file.date_published ?? '',
		downloads: file.downloads,
		files: [{ filename: file.file_name, size: file.size ?? 0, primary: true }],
	})),
)
</script>

<template>
	<ProjectPageVersions
		:loaders="loaders"
		:game-versions="gameVersions"
		:versions="versions"
		:project="{ id: project.id, project_type: project.project_type ?? 'mod' }"
		show-files
	>
		<template #actions="{ version }">
			<IconButton
				v-tooltip="formatMessage(commonMessages.installButton)"
				type="quiet"
				color="green"
				:label="formatMessage(commonMessages.installButton)"
				:disabled="installing"
				@click.stop="() => install(version.id)"
			>
				<DownloadIcon />
			</IconButton>
		</template>
	</ProjectPageVersions>
</template>
