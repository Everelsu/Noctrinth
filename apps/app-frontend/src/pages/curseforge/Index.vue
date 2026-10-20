<script setup lang="ts">
/**
 * Noctrinth's own: a CurseForge project's page.
 *
 * Built from the pieces a Modrinth project's page is built from — the same
 * header, compatibility sidebar, tabs, description and gallery — so a project
 * found on CurseForge looks like any other. What CurseForge does not have
 * (followers as Modrinth counts them, a team, licences) is simply not drawn.
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
import { openUrl } from '@tauri-apps/plugin-opener'
import { computed, ref, watch } from 'vue'
import { useRoute } from 'vue-router'

import CurseforgeInstancePickerModal from '@/components/ui/curseforge/CurseforgeInstancePickerModal.vue'
import CurseforgePendingFilesModal from '@/components/ui/curseforge/CurseforgePendingFilesModal.vue'
import InstanceIndicator from '@/components/ui/InstanceIndicator.vue'
import {
	curseforgeDownloadModpack,
	type CurseforgeFile,
	curseforgeGetDescription,
	curseforgeGetFiles,
	curseforgeGetProject,
	curseforgeIdOf,
	curseforgeInstallProject,
	type CurseforgeProject,
} from '@/helpers/curseforge'
import { install_create_modpack_instance, packLocationFromFile } from '@/helpers/install'
import {
	get as getInstance,
	get_installed_project_ids as getInstalledProjectIds,
} from '@/helpers/instance'
import { get_game_versions, get_loaders } from '@/helpers/tags'
import type { GameInstance } from '@/helpers/types'

const props = defineProps<{ id: string }>()

const { handleError } = injectNotificationManager()
const { formatMessage } = useVIntl()
const route = useRoute()

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

const project = ref<CurseforgeProject | null>(null)
const description = ref('')
const files = ref<CurseforgeFile[]>([])
const instance = ref<GameInstance | null>(null)
const installedIds = ref<Set<string>>(new Set())
const installing = ref(false)
const loadFailed = ref(false)
const [allLoaders, allGameVersions] = await Promise.all([
	get_loaders().catch(handleError).then(ref),
	get_game_versions().catch(handleError).then(ref),
])

const pickerModal = ref<InstanceType<typeof CurseforgeInstancePickerModal> | null>(null)
const pendingModal = ref<InstanceType<typeof CurseforgePendingFilesModal> | null>(null)
/** The file chosen on the versions tab, waiting for an instance to go into. */
const pickedFileId = ref<string | null>(null)

const instanceId = computed(() => (typeof route.query.i === 'string' ? route.query.i : null))
const isModpack = computed(() => project.value?.project_type === 'modpack')
const installed = computed(
	() => !isModpack.value && !!project.value && installedIds.value.has(project.value.id),
)

async function load() {
	loadFailed.value = false
	try {
		const [loaded, body, loadedFiles, loadedInstance] = await Promise.all([
			curseforgeGetProject(props.id),
			curseforgeGetDescription(props.id).catch(() => ''),
			curseforgeGetFiles(props.id).catch(() => []),
			instanceId.value ? getInstance(instanceId.value) : Promise.resolve(null),
		])
		project.value = loaded
		description.value = body
		files.value = loadedFiles
		instance.value = loadedInstance
		await refreshInstalled()
	} catch (err) {
		loadFailed.value = true
		handleError(err)
	}
}

async function refreshInstalled() {
	if (!instanceId.value) {
		installedIds.value = new Set()
		return
	}
	const ids = await getInstalledProjectIds(instanceId.value).catch(() => [] as string[])
	installedIds.value = new Set(ids.filter((id) => /^\d+$/.test(id)).map(curseforgeIdOf))
}

await load()
watch(() => props.id, load)

/** The project in the shapes the shared header and sidebar read. */
const headerProject = computed(() =>
	project.value
		? {
				id: project.value.id,
				title: project.value.name,
				description: project.value.summary,
				icon_url: project.value.icon_url,
				raw_icon_url: project.value.icon_url,
				downloads: project.value.downloads,
				followers: project.value.follows,
				categories: [] as string[],
			}
		: null,
)
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
const tabProject = computed(() =>
	project.value
		? {
				id: project.value.id,
				project_type: project.value.project_type,
				body: description.value,
				gallery: project.value.gallery.map((image) => ({
					url: image.url,
					raw_url: image.url,
					title: image.title,
					description: image.description,
					created: null,
				})),
			}
		: null,
)

const tabQuery = computed(() => (instanceId.value ? { i: instanceId.value } : {}))
const tabs = computed(() => [
	{
		label: formatMessage(messages.descriptionTab),
		href: { path: `/curseforge/${props.id}`, query: tabQuery.value },
	},
	{
		label: formatMessage(messages.versionsTab),
		href: { path: `/curseforge/${props.id}/versions`, query: tabQuery.value },
	},
	{
		label: formatMessage(messages.galleryTab),
		href: { path: `/curseforge/${props.id}/gallery`, query: tabQuery.value },
		shown: (project.value?.gallery.length ?? 0) > 0,
	},
])

/**
 * Install the project, or one file of it. A modpack becomes a new instance
 * through the regular CurseForge pack installer; anything else goes into the
 * instance the page was opened from, or one the user picks.
 */
async function install(fileId: string | null = null) {
	if (!project.value || installing.value) return
	if (isModpack.value) {
		installing.value = true
		try {
			const path = await curseforgeDownloadModpack(project.value.id, fileId)
			await install_create_modpack_instance(packLocationFromFile(path))
		} catch (err) {
			handleError(err)
		} finally {
			installing.value = false
		}
		return
	}
	if (!instanceId.value) {
		pickedFileId.value = fileId
		pickerModal.value?.show()
		return
	}
	await installInto(instanceId.value, fileId)
}

async function installInto(targetInstanceId: string, fileId: string | null) {
	if (!project.value) return
	installing.value = true
	try {
		const outcome = await curseforgeInstallProject(targetInstanceId, project.value.id, fileId)
		if (outcome.pending.length > 0) {
			pendingModal.value?.show(targetInstanceId, outcome.pending)
		}
		if (targetInstanceId === instanceId.value) await refreshInstalled()
	} catch (err) {
		handleError(err)
	} finally {
		installing.value = false
	}
}

function onInstancePicked(targetInstanceId: string) {
	void installInto(targetInstanceId, pickedFileId.value)
	pickedFileId.value = null
}
</script>

<template>
	<div v-if="project && headerProject && tabProject">
		<Teleport to="#sidebar-teleport-target">
			<ProjectSidebarCompatibility
				:project="sidebarProject"
				:tags="{ loaders: allLoaders, gameVersions: allGameVersions }"
				class="project-sidebar-section"
			/>
		</Teleport>
		<div class="flex flex-col gap-4 p-6">
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
						v-if="project.website_url"
						v-tooltip="formatMessage(messages.openOnCurseforge)"
						type="quiet"
						size="xl"
						:label="formatMessage(messages.openOnCurseforge)"
						@click="openUrl(project.website_url)"
					>
						<ExternalIcon />
					</IconButton>
				</template>
			</ProjectPageHeader>
			<NavTabs :links="tabs" />
			<RouterView
				:project="tabProject"
				:files="files"
				:instance="instance"
				:installed="installed"
				:installing="installing"
				:install="install"
			/>
		</div>
		<CurseforgeInstancePickerModal ref="pickerModal" @pick="onInstancePicked" />
		<CurseforgePendingFilesModal ref="pendingModal" />
	</div>
	<p v-else-if="loadFailed" class="p-6 text-secondary">{{ formatMessage(messages.loadError) }}</p>
</template>
