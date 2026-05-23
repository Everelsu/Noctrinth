<template>
	<div v-if="mod">
		<!-- Right app sidebar — same components & layout as the Modrinth project page -->
		<Teleport to="#sidebar-teleport-target">
			<ProjectSidebarCompatibility
				:project="project"
				:tags="{ loaders: allLoaders, gameVersions: allGameVersions }"
				class="project-sidebar-section"
			/>
			<ProjectSidebarLinks
				link-target="_blank"
				:project="project"
				class="project-sidebar-section"
			/>
			<ProjectSidebarTags :project="project" class="project-sidebar-section" />
			<ProjectSidebarDetails
				:project="project"
				:has-versions="files.length > 0"
				link-target="_blank"
				hide-license
				class="project-sidebar-section"
			/>
		</Teleport>

		<div class="flex flex-col gap-4 p-6">
			<ProjectHeader :project="project" :project-v3="null">
				<template #actions>
					<ButtonStyled size="large" color="brand">
						<button :disabled="installBtnBusy || installBtnDone" @click="onInstallClick('auto')">
							<SpinnerIcon v-if="installBtnBusy" class="animate-spin" />
							<CheckIcon v-else-if="installBtnDone" />
							<DownloadIcon v-else />
							{{ installBtnBusy ? 'Installing…' : installBtnDone ? 'Installed' : 'Install' }}
						</button>
					</ButtonStyled>

					<ButtonStyled size="large" color="standard">
						<button @click="openExternal(mod.links?.websiteUrl)">
							<ExternalIcon />
							Open on CurseForge
						</button>
					</ButtonStyled>
				</template>
			</ProjectHeader>

			<NavTabs
				v-if="tabs.length > 1"
				:links="tabs"
				mode="local"
				:active-index="activeTab"
				@tab-click="(index) => (activeTab = index)"
			/>

			<!-- Description -->
			<Card v-if="activeTab === 0">
				<!-- eslint-disable-next-line vue/no-v-html -->
				<div class="markdown-body" @click="onDescriptionClick" v-html="descriptionHtml" />
			</Card>

			<!-- Versions -->
			<ProjectPageVersions
				v-else-if="activeTab === 1"
				:project="project"
				:versions="mappedVersions"
				:loaders="allLoaders"
				:game-versions="allGameVersions"
			>
				<template #actions="{ version }">
					<ButtonStyled>
						<button
							:disabled="installBtnBusy || rowInstalled(version) || !rowFile(version).downloadUrl"
							@click="onInstallClick(rowFile(version))"
						>
							<CheckIcon v-if="rowInstalled(version)" />
							<DownloadIcon v-else />
							{{ rowInstalled(version) ? 'Installed' : 'Install' }}
						</button>
					</ButtonStyled>
				</template>
			</ProjectPageVersions>

			<!-- Gallery -->
			<Gallery v-else-if="activeTab === 2" :project="project" />
		</div>

		<CurseForgeInstallModal ref="cfInstallModal" />
	</div>

	<!-- Error state -->
	<div v-else class="flex flex-col gap-4 p-6">
		<Card class="flex flex-col items-center gap-2 py-12 text-center">
			<h2 class="m-0 text-contrast">Mod not found</h2>
			<p class="m-0 text-secondary">
				This CurseForge mod could not be loaded. It may have been removed, or CurseForge is
				unavailable right now.
			</p>
		</Card>
	</div>
</template>

<script setup>
import { CheckIcon, DownloadIcon, ExternalIcon, SpinnerIcon } from '@modrinth/assets'
import {
	ButtonStyled,
	Card,
	injectNotificationManager,
	NavTabs,
	ProjectHeader,
	ProjectPageVersions,
	ProjectSidebarCompatibility,
	ProjectSidebarDetails,
	ProjectSidebarLinks,
	ProjectSidebarTags,
} from '@modrinth/ui'
import { openUrl } from '@tauri-apps/plugin-opener'
import { computed, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'

import CurseForgeInstallModal from '@/components/ui/CurseForgeInstallModal.vue'
import Gallery from '@/pages/project/Gallery.vue'
import {
	bestCfFileFor,
	cfLoaderName,
	cfProjectType,
	getCurseForgeMod,
	getCurseForgeModDescription,
	getCurseForgeModFiles,
	installCurseForgeFile,
	installCurseForgeModpack,
} from '@/helpers/curseforge-api'
import { get as getInstance } from '@/helpers/profile'
import { get_game_versions, get_loaders } from '@/helpers/tags'

const props = defineProps({
	modId: {
		type: String,
		required: true,
	},
})

const route = useRoute()
const router = useRouter()
const { handleError, addNotification } = injectNotificationManager()

// Async setup — RouterView wraps pages in <Suspense>, so top-level await is fine.
const [mod, allLoaders, allGameVersions] = await Promise.all([
	getCurseForgeMod(props.modId),
	get_loaders().catch(() => []),
	get_game_versions().catch(() => []),
])

const cfInstallModal = ref(null)

const description = mod ? await getCurseForgeModDescription(props.modId) : null
const descriptionHtml = description || '<p>No description available.</p>'

const files = (mod ? await getCurseForgeModFiles(props.modId) : null) ?? []

// Instance context — present when this page was opened while adding content
// to a specific instance (?i=<instance path> in the URL).
const instanceId = typeof route.query.i === 'string' ? route.query.i : null
const instanceContext = instanceId ? await getInstance(instanceId).catch(() => null) : null

// ── Map CurseForge mod → Modrinth v2 project shape ──────────────────────────
// Lets us reuse ProjectHeader / ProjectSidebar* unchanged.

const fileIndexes = mod?.latestFilesIndexes ?? []
const gameVersions = [...new Set(fileIndexes.map((i) => i.gameVersion).filter(Boolean))]
const loaders = [
	...new Set(
		fileIndexes
			.map((i) => i.modLoader)
			.filter((id) => id != null && id !== 0)
			.map(cfLoaderName),
	),
]
const categoryNames = (mod?.categories ?? []).map((c) => c.name)
const projectType = cfProjectType(mod?.classId)

// CurseForge screenshots → Modrinth gallery shape (reuses the Gallery page).
const galleryItems = (mod?.screenshots ?? []).map((shot) => ({
	url: shot.thumbnailUrl ?? shot.url,
	raw_url: shot.url,
	title: shot.title ?? '',
	description: shot.description ?? '',
	created: mod?.dateModified,
}))

const project = mod
	? {
			id: `cf:${mod.id}`,
			slug: mod.slug,
			title: mod.name,
			description: mod.summary,
			body: '',
			project_type: projectType,
			actualProjectType: projectType,
			status: 'approved',
			downloads: mod.downloadCount,
			followers: mod.thumbsUpCount,
			icon_url: mod.logo?.url ?? mod.logo?.thumbnailUrl ?? null,
			categories: categoryNames,
			additional_categories: [],
			display_categories: categoryNames,
			loaders,
			game_versions: gameVersions,
			versions: fileIndexes,
			client_side: 'unknown',
			server_side: 'unknown',
			license: { id: '', url: '' },
			published: mod.dateCreated,
			updated: mod.dateModified,
			approved: mod.dateReleased ?? mod.dateCreated,
			queued: null,
			issues_url: mod.links?.issuesUrl ?? '',
			source_url: mod.links?.sourceUrl ?? '',
			wiki_url: mod.links?.wikiUrl ?? '',
			discord_url: '',
			site_url: mod.links?.websiteUrl ?? '',
			donation_urls: [],
			gallery: galleryItems,
		}
	: null

// ── Tabs ────────────────────────────────────────────────────────────────────
const activeTab = ref(0)
const tabs = computed(() => {
	const list = [
		{ label: 'Description', href: 'cf-description' },
		{ label: 'Versions', href: 'cf-versions' },
	]
	if (mod?.screenshots?.length) {
		list.push({ label: 'Gallery', href: 'cf-gallery' })
	}
	return list
})

// ── Versions ────────────────────────────────────────────────────────────────
// Map CurseForge files into the Modrinth version shape so the rich
// <ProjectPageVersions> table (filters, pagination, columns) renders them.
const CF_RELEASE_TYPE = { 1: 'release', 2: 'beta', 3: 'alpha' }

function mapCfFileToVersion(file) {
	const versions = file.gameVersions.filter((v) => /^\d/.test(v))
	const fileLoaders = file.gameVersions
		.filter((v) => !/^\d/.test(v))
		.map((v) => v.toLowerCase())
	return {
		id: String(file.id),
		version_number: file.displayName || file.fileName,
		name: file.fileName,
		game_versions: versions,
		loaders: fileLoaders,
		version_type: CF_RELEASE_TYPE[file.releaseType] ?? 'release',
		date_published: file.fileDate,
		downloads: file.downloadCount ?? 0,
		files: [{ primary: true, filename: file.fileName, size: file.fileLength }],
		// Original CF file kept for the install action.
		_cfFile: file,
	}
}

const mappedVersions = files.map(mapCfFileToVersion)

/** Slot helpers — version is loosely typed, so `_cfFile` access stays clean. */
function rowFile(version) {
	return version._cfFile
}
function rowInstalled(version) {
	return isModpack ? modpackInstalled.value : installedIds.value.has(version._cfFile.id)
}

// ── Installation ────────────────────────────────────────────────────────────
const isModpack = projectType === 'modpack'

// Mod install state (a mod is added into an instance).
const installBusy = ref(false)
const headerInstalled = ref(false)
const installedIds = ref(new Set())

// Modpack install state (a modpack creates its own instance).
const modpackBusy = ref(false)
const modpackInstalled = ref(false)

// Unified state for the shared install button / version rows.
const installBtnBusy = computed(() => (isModpack ? modpackBusy.value : installBusy.value))
const installBtnDone = computed(() => (isModpack ? modpackInstalled.value : headerInstalled.value))

// `mode` is 'auto' (best file) or a concrete CfModFile (a version row).
function onInstallClick(mode) {
	if (isModpack) {
		handleInstallModpack(mode === 'auto' ? undefined : mode)
		return
	}
	if (installBusy.value) return
	if (instanceContext) {
		// Came from an instance — install straight to it.
		runInstall(instanceContext, mode)
	} else {
		// No instance context — open the shared instance-picker modal.
		cfInstallModal.value?.show(
			{ id: mod.id, name: mod.name, iconUrl: mod.logo?.url ?? mod.logo?.thumbnailUrl ?? null },
			mode === 'auto' ? null : mode,
		)
	}
}

// Install a CurseForge modpack — creates a new instance from the pack.
async function handleInstallModpack(file) {
	if (!mod || modpackBusy.value || modpackInstalled.value) return
	modpackBusy.value = true
	try {
		const profile = await installCurseForgeModpack(mod.id, mod.name, file)
		modpackInstalled.value = true
		addNotification({
			title: 'Modpack installed',
			text: `${mod.name} is ready to play.`,
			type: 'success',
		})
		router.push(`/instance/${encodeURIComponent(profile)}`)
	} catch (e) {
		handleError(e)
	} finally {
		modpackBusy.value = false
	}
}

async function runInstall(target, mode) {
	const file =
		mode === 'auto' ? bestCfFileFor(files, target.game_version, target.loader) : mode

	if (!file) {
		handleError(new Error('No CurseForge file is compatible with this instance.'))
		return
	}

	installBusy.value = true
	try {
		await installCurseForgeFile(file, target.path)
		installedIds.value.add(file.id)
		if (mode === 'auto') headerInstalled.value = true
		addNotification({
			title: 'Mod installed',
			text: `${mod.name} was added to ${target.name}.`,
			type: 'success',
		})
	} catch (e) {
		handleError(e)
	} finally {
		installBusy.value = false
	}
}

function openExternal(url) {
	if (url) openUrl(url)
}

// Open links inside the rendered description in the system browser.
function onDescriptionClick(event) {
	const anchor = event.target?.closest?.('a')
	if (anchor?.href) {
		event.preventDefault()
		openUrl(anchor.href)
	}
}
</script>

<style scoped lang="scss">
.project-sidebar-section {
	@apply p-4 flex flex-col gap-2 border-0 border-b-[1px] border-[--brand-gradient-border] border-solid;
}
</style>
