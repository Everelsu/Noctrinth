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
							{{
								installBtnBusy
									? formatMessage(messages.installing)
									: installBtnDone
										? formatMessage(messages.installed)
										: formatMessage(messages.install)
							}}
						</button>
					</ButtonStyled>

					<ButtonStyled size="large" color="standard">
						<button @click="openExternal(mod.links?.websiteUrl)">
							<ExternalIcon />
							{{ formatMessage(messages.openOnCurseForge) }}
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
							{{
								rowInstalled(version)
									? formatMessage(messages.installed)
									: formatMessage(messages.install)
							}}
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
			<h2 class="m-0 text-contrast">{{ formatMessage(messages.notFoundTitle) }}</h2>
			<p class="m-0 text-secondary">
				{{ formatMessage(messages.notFoundDescription) }}
			</p>
		</Card>
	</div>
</template>

<script setup>
import { CheckIcon, DownloadIcon, ExternalIcon, SpinnerIcon } from '@modrinth/assets'
import {
	ButtonStyled,
	Card,
	defineMessages,
	injectNotificationManager,
	NavTabs,
	ProjectHeader,
	ProjectPageVersions,
	ProjectSidebarCompatibility,
	ProjectSidebarDetails,
	ProjectSidebarLinks,
	ProjectSidebarTags,
	useVIntl,
} from '@modrinth/ui'
import { openUrl } from '@tauri-apps/plugin-opener'
import { computed, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'

import CurseForgeInstallModal from '@/components/ui/CurseForgeInstallModal.vue'
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
import { get as getInstance } from '@/helpers/instance'
import { get_game_versions, get_loaders } from '@/helpers/tags'
import Gallery from '@/pages/project/Gallery.vue'

const props = defineProps({
	modId: {
		type: String,
		required: true,
	},
})

const route = useRoute()
const router = useRouter()
const { formatMessage } = useVIntl()
const { handleError, addNotification } = injectNotificationManager()

const messages = defineMessages({
	install: { id: 'app.curseforge-project.install', defaultMessage: 'Install' },
	installing: { id: 'app.curseforge-project.installing', defaultMessage: 'Installing…' },
	installed: { id: 'app.curseforge-project.installed', defaultMessage: 'Installed' },
	openOnCurseForge: {
		id: 'app.curseforge-project.open-on-curseforge',
		defaultMessage: 'Open on CurseForge',
	},
	tabDescription: {
		id: 'app.curseforge-project.tab.description',
		defaultMessage: 'Description',
	},
	tabVersions: { id: 'app.curseforge-project.tab.versions', defaultMessage: 'Versions' },
	tabGallery: { id: 'app.curseforge-project.tab.gallery', defaultMessage: 'Gallery' },
	noDescription: {
		id: 'app.curseforge-project.no-description',
		defaultMessage: 'No description available.',
	},
	notFoundTitle: {
		id: 'app.curseforge-project.not-found.title',
		defaultMessage: 'Mod not found',
	},
	notFoundDescription: {
		id: 'app.curseforge-project.not-found.description',
		defaultMessage:
			'This CurseForge mod could not be loaded. It may have been removed, or CurseForge is unavailable right now.',
	},
})

// Async setup — RouterView wraps pages in <Suspense>, so top-level await is fine.
const [mod, allLoaders, allGameVersions] = await Promise.all([
	getCurseForgeMod(props.modId),
	get_loaders().catch(() => []),
	get_game_versions().catch(() => []),
])

const cfInstallModal = ref(null)

const description = mod ? await getCurseForgeModDescription(props.modId) : null
const descriptionHtml = description || `<p>${formatMessage(messages.noDescription)}</p>`

// Instance context — present when this page was opened while adding content
// to a specific instance (?i=<instance path> in the URL).
const instanceId = typeof route.query.i === 'string' ? route.query.i : null
const instanceContext = instanceId ? await getInstance(instanceId).catch(() => null) : null

// When opened from an instance, narrow file list to versions compatible with
// that instance's Minecraft version and loader so only installable versions
// appear in the Versions tab. Without this filter all historical files (1.0 …
// latest, every loader) are shown regardless of the instance.
const files = mod
	? ((await getCurseForgeModFiles(
			props.modId,
			instanceContext?.game_version,
			instanceContext?.loader,
		)) ?? [])
	: []

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
		{ label: formatMessage(messages.tabDescription), href: 'cf-description' },
		{ label: formatMessage(messages.tabVersions), href: 'cf-versions' },
	]
	if (mod?.screenshots?.length) {
		list.push({ label: formatMessage(messages.tabGallery), href: 'cf-gallery' })
	}
	return list
})

// ── Versions ────────────────────────────────────────────────────────────────
// Map CurseForge files into the Modrinth version shape so the rich
// <ProjectPageVersions> table (filters, pagination, columns) renders them.
const CF_RELEASE_TYPE = { 1: 'release', 2: 'beta', 3: 'alpha' }

function mapCfFileToVersion(file) {
	const versions = file.gameVersions.filter((v) => /^\d/.test(v))
	const fileLoaders = file.gameVersions.filter((v) => !/^\d/.test(v)).map((v) => v.toLowerCase())
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
		const profile = await installCurseForgeModpack(mod.id, file)
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
	const file = mode === 'auto' ? bestCfFileFor(files, target.game_version, target.loader) : mode

	if (!file) {
		// Build a useful "no match" message — list what's actually available.
		const availableVersions = Array.from(
			new Set(files.flatMap((f) => f.gameVersions.filter((v) => /^\d/.test(v)))),
		).sort()
		const versionHint =
			availableVersions.length > 0
				? ` Available: ${availableVersions.slice(0, 8).join(', ')}${availableVersions.length > 8 ? '…' : ''}.`
				: ''
		handleError(
			new Error(
				`No CurseForge file fits ${target.game_version}${target.loader ? ` / ${target.loader}` : ''}.${versionHint}`,
			),
		)
		return
	}

	installBusy.value = true
	try {
		const result = await installCurseForgeFile(file, target.id, target.game_version, target.loader)
		installedIds.value.add(file.id)
		if (mode === 'auto') headerInstalled.value = true
		addNotification({
			title: 'Mod installed',
			text: `${mod.name} was added to ${target.name}.`,
			type: 'success',
		})
		if (result?.incompatible?.length) {
			addNotification({
				title: 'Incompatible mod warning',
				text: `${mod.name} declares ${result.incompatible.length} incompatible mod(s) — check ${target.name} for conflicts.`,
				type: 'warn',
			})
		}
		if (result?.optional?.length) {
			addNotification({
				title: 'Optional dependencies available',
				text: `${mod.name} has ${result.optional.length} optional add-on(s) — open its page to install them.`,
				type: 'info',
			})
		}
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
