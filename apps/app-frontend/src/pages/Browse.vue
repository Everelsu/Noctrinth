<script setup lang="ts">
import type { Labrinth } from '@modrinth/api-client'
import {
	CheckIcon,
	ClipboardCopyIcon,
	CompassIcon,
	DownloadIcon,
	ExternalIcon,
	GlobeIcon,
	PlusIcon,
	ServerStackIcon,
	SpinnerIcon,
} from '@modrinth/assets'
import type { BrowseInstallContentType, CardAction, ProjectType, Tags } from '@modrinth/ui'
import {
	BrowsePageLayout,
	BrowseSidebar,
	commonMessages,
	CreationFlowModal,
	defineMessages,
	formatProjectTypeSentence,
	getLatestMatchingInstallVersion,
	getSelectedInstallPreferences,
	getTargetInstallPreferences,
	injectNotificationManager,
	preferencesDiffer,
	provideBrowseManager,
	requestInstall,
	resolveInstallPlan,
	stripServerRuntimeInstallFilters,
	stripServerRuntimeInstallOverrides,
	useBrowseSearch,
	useDebugLogger,
	useVIntl,
} from '@modrinth/ui'
import { useQueryClient } from '@tanstack/vue-query'
import { convertFileSrc } from '@tauri-apps/api/core'
import type { Ref } from 'vue'
import { computed, onMounted, onUnmounted, ref, shallowRef, watch } from 'vue'
import type { LocationQuery } from 'vue-router'
import { useRoute, useRouter } from 'vue-router'

import ContextMenu from '@/components/ui/ContextMenu.vue'
import CurseForgeInstallModal from '@/components/ui/CurseForgeInstallModal.vue'
import { useAppServerBrowse } from '@/composables/browse/use-app-server-browse'
import { useSourceAccent, useSourceMode } from '@/composables/source-mode'
import {
	get_project,
	get_project_v3,
	get_search_results_v3,
	get_version_many,
} from '@/helpers/cache.js'
import {
	loadCfInstalledProjectIds,
	refreshCfInstalledFromFingerprints,
	storeCfInstalled,
} from '@/helpers/cf-installed-store'
import {
	cfIdToModrinthId as lookupCfIdToMr,
	modrinthIdToCfId as lookupMrIdToCf,
} from '@/helpers/cross-platform-mapping'
import {
	getCurseForgeCategories,
	installCurseForgeMod,
	installCurseForgeModpack,
	isCurseForgeAvailable,
	mapCfCategoriesToTags,
} from '@/helpers/curseforge-api'
import { instance_listener } from '@/helpers/events.js'
import {
	get as getInstance,
	get_installed_project_ids as getInstalledProjectIds,
	get_projects as getProfileProjects,
	list as listInstances,
} from '@/helpers/instance'
import { get_loader_versions as getLoaderManifest } from '@/helpers/metadata'
import { get as getSettings, set as setSettings } from '@/helpers/settings.ts'
import { get_categories, get_game_versions, get_loaders } from '@/helpers/tags'
import { unifiedSearch } from '@/helpers/unified-search'
import { get_instance_worlds } from '@/helpers/worlds'
import {
	type BreadcrumbDefinition,
	useBreadcrumb,
	useRootBreadcrumb,
} from '@/providers/breadcrumbs'
import { injectContentInstall } from '@/providers/content-install'
import { injectServerInstall } from '@/providers/server-install'
import {
	createServerInstallContent,
	provideServerInstallContent,
} from '@/providers/setup/server-install-content'
import { useTheming } from '@/store/state'

const { handleError, addNotification } = injectNotificationManager()
const { formatMessage } = useVIntl()
const { installingServerProjects, playServerProject, showAddServerToInstanceModal } =
	injectServerInstall()
const { install: installVersion } = injectContentInstall()
const queryClient = useQueryClient()
const debugLog = useDebugLogger('Browse')

// Catalog source — every search (browse and text query) targets exactly the
// catalog selected by this toggle. Declared early so `tags` computed can
// reference it when building the sidebar category list.
const sourceMode = useSourceMode()

// CurseForge search supports a game version, a mod loader, and (mapped)
// categories — nothing else. Modrinth-only filter groups (client/server
// environment, license/open-source) are hidden entirely in CF mode instead
// of being shown as clickable-but-ignored buttons.
const CF_UNSUPPORTED_FILTER_IDS = new Set(['environment', 'license'])

const router = useRouter()
const route = useRoute()
const displayedBrowseRoute = shallowRef(router.currentRoute.value)
watch(
	() => router.currentRoute.value,
	(nextRoute) => {
		if (nextRoute.path.startsWith('/browse/')) {
			displayedBrowseRoute.value = nextRoute
		}
	},
	{ immediate: true },
)
const breadcrumbMessages = defineMessages({
	discoverProjectType: {
		id: 'app.browse.discover-project-type',
		defaultMessage: 'Discover {projectType}',
	},
	discoverServers: {
		id: 'app.browse.discover-servers',
		defaultMessage: 'Discover servers',
	},
})
const breadcrumbLabel = computed(() => {
	const browseRoute = displayedBrowseRoute.value
	if (browseRoute.query.from === 'worlds' || browseRoute.params.projectType === 'server') {
		return formatMessage(breadcrumbMessages.discoverServers)
	}

	return formatMessage(breadcrumbMessages.discoverProjectType, {
		projectType: formatProjectTypeSentence(
			formatMessage,
			String(browseRoute.params.projectType ?? ''),
			2,
		),
	})
})
const themeStore = useTheming()
const browseRouteActive = computed(() => route.path.startsWith('/browse/'))
const serverSetupModalRef = ref<InstanceType<typeof CreationFlowModal> | null>(null)
const serverInstallContent = createServerInstallContent({ serverSetupModalRef })
provideServerInstallContent(serverInstallContent)
const {
	serverIdQuery,
	serverFlowFrom,
	isFromWorlds,
	isServerContext,
	isSetupServerContext,
	effectiveServerWorldId,
	serverContextServerData,
	serverContentProjectIds,
	queuedServerInstallProjectIds,
	queuedServerInstallCount,
	selectedServerInstallProjects,
	isInstallingQueuedServerInstalls,
	queuedInstallProgress,
	serverBackUrl,
	serverBackLabel,
	serverBrowseHeading,
	clearQueuedServerInstalls,
	removeQueuedServerInstall,
	flushQueuedServerInstalls,
	discardQueuedServerInstallsAndBack,
	installQueuedServerInstallsAndBack,
	initServerContext,
	watchServerContextChanges,
	searchServerModpacks,
	getServerProjectVersions,
	enforceSetupModpackRoute,
	getQueuedServerInstallPlans,
	setQueuedServerInstallPlans,
	openServerModpackInstallFlow,
	onServerFlowBack,
	handleServerModpackFlowCreate,
	markServerProjectInstalled,
} = serverInstallContent

type Instance = {
	id: string
	game_version: string
	loader: string
	path: string
	install_stage: string
	icon_path?: string
	name: string
	link?: {
		type: string
		project_id: string
		version_id: string
	}
}

const initialInstanceId = String(route.query.i ?? '')
const instance: Ref<Instance | null> = ref(
	queryClient.getQueryData<Instance>(['instances', 'summary', initialInstanceId]) ?? null,
)
const installedProjectIds: Ref<string[] | null> = ref(null)
const instanceHideInstalled = ref(route.query.ai === 'true')
const newlyInstalled = ref<string[]>([])
const hiddenInstanceProjectIds = ref<Set<string>>(new Set())
const hiddenInstanceProjectIdsInitialized = ref(false)
const isServerInstance = ref(false)

const instanceBreadcrumb = route.query.i
	? useBreadcrumb({
			slot: 'instance',
			id: () => `instance:${String(displayedBrowseRoute.value.query.i ?? '')}`,
			label: () => instance.value?.name ?? formatMessage(commonMessages.loadingLabel),
			visual: () => ({
				type: 'image',
				src: instance.value?.icon_path ? convertFileSrc(instance.value.icon_path) : undefined,
				alt: instance.value?.name,
				tintBy: String(displayedBrowseRoute.value.query.i ?? ''),
			}),
			to: () => {
				const instancePath = `/instance/${encodeURIComponent(
					String(displayedBrowseRoute.value.query.i ?? ''),
				)}`
				return displayedBrowseRoute.value.query.from === 'worlds'
					? `${instancePath}/worlds`
					: instancePath
			},
		})
	: undefined
const serverBreadcrumbTo = ref(serverBackUrl.value)
watch(serverBackUrl, (value) => {
	if (route.path.startsWith('/browse/')) {
		serverBreadcrumbTo.value = value
	}
})
const serverBreadcrumb =
	!instanceBreadcrumb && serverIdQuery.value
		? useBreadcrumb({
				slot: 'server',
				id: () => `server:${String(displayedBrowseRoute.value.query.sid ?? '')}`,
				label: () =>
					serverContextServerData.value?.name ?? formatMessage(commonMessages.loadingLabel),
				visual: { type: 'icon', component: ServerStackIcon },
				to: serverBreadcrumbTo,
			})
		: undefined
const breadcrumbParent = instanceBreadcrumb ?? serverBreadcrumb
const breadcrumbDefinition = {
	slot: 'browse',
	id: () =>
		`browse:${String(displayedBrowseRoute.value.params.projectType ?? '')}:${String(
			displayedBrowseRoute.value.query.i ?? '',
		)}:${String(displayedBrowseRoute.value.query.sid ?? '')}:${String(
			displayedBrowseRoute.value.query.from ?? '',
		)}`,
	label: breadcrumbLabel,
	to: () => displayedBrowseRoute.value.fullPath,
	visual: { type: 'icon', component: CompassIcon },
} satisfies BreadcrumbDefinition
const browseBreadcrumb = breadcrumbParent
	? useBreadcrumb(breadcrumbDefinition, { parent: breadcrumbParent })
	: useRootBreadcrumb(breadcrumbDefinition)

debugLog('fetching tags (categories, loaders, gameVersions)')
const [categories, loaders, availableGameVersions] = await Promise.all([
	get_categories()
		.catch(handleError)
		.then(ref<Labrinth.Tags.v2.Category[]>),
	get_loaders()
		.catch(handleError)
		.then(ref<Labrinth.Tags.v2.Loader[]>),
	get_game_versions()
		.catch(handleError)
		.then(ref<Labrinth.Tags.v2.GameVersion[]>),
])

// CurseForge categories — loaded in the background so they're ready when
// the user switches to CF-only mode. Typed as the Modrinth category shape
// so `useSearch` / BrowseSidebar can consume them without changes.
const cfCategoriesRaw = ref<Labrinth.Tags.v2.Category[]>([])
if (isCurseForgeAvailable()) {
	getCurseForgeCategories()
		.then((cfCats) => {
			cfCategoriesRaw.value = mapCfCategoriesToTags(cfCats) as Labrinth.Tags.v2.Category[]
		})
		.catch(() => {})
}

const tags: Ref<Tags> = computed(() => ({
	gameVersions: availableGameVersions.value ?? [],
	loaders: loaders.value ?? [],
	// In CurseForge-only browse mode replace Modrinth categories with CF ones
	// so the sidebar shows meaningful CurseForge sub-categories. In Modrinth
	// or mixed (text-search) mode keep the Modrinth categories.
	categories: sourceMode.value === 'curseforge' ? cfCategoriesRaw.value : (categories.value ?? []),
}))

// Basenames of files already installed in the instance (e.g. "jei-1.21.1-fabric-19.27.0.336.jar").
// Used to detect CF mods installed before the localStorage store existed.
const installedFileNames = ref<Set<string>>(new Set())

if (isFromWorlds.value && route.params.projectType !== 'server') {
	router.replace({
		path: '/browse/server',
		query: route.query,
	})
}

enforceSetupModpackRoute(route.params.projectType as string | undefined)

const allInstalledIds = computed(
	() => new Set([...newlyInstalled.value, ...(installedProjectIds.value ?? [])]),
)

function syncHiddenInstanceProjectIds() {
	hiddenInstanceProjectIds.value = new Set([
		...(installedProjectIds.value ?? []),
		...newlyInstalled.value,
	])
	hiddenInstanceProjectIdsInitialized.value = true
}

watch(
	installedProjectIds,
	(ids) => {
		if (!ids) return
		if (!hiddenInstanceProjectIdsInitialized.value) {
			syncHiddenInstanceProjectIds()
		}
	},
	{ immediate: true },
)

watchServerContextChanges()

await initInstanceContext()

async function refreshInstalledProjectIds() {
	if (!route.query.i) {
		const instances = await listInstances().catch(handleError)
		if (!instances) return

		const ids = instances
			.map((gameInstance) => gameInstance.link?.project_id)
			.filter((id): id is string => !!id)
		debugLog('installedInstanceProjectIds loaded', { count: ids.length })
		installedProjectIds.value = ids
		return
	}

	if (route.query.from === 'worlds') {
		const worlds = await get_instance_worlds(route.query.i as string).catch(handleError)
		if (!worlds) return

		const serverProjectIds = worlds
			.filter((w) => w.type === 'server' && 'project_id' in w && w.project_id)
			.map((w) => (w as { project_id: string }).project_id)
		debugLog('installedServerProjectIds loaded', { count: serverProjectIds.length })
		installedProjectIds.value = serverProjectIds
		return
	}

	const instancePath = route.query.i as string
	// Modrinth IDs (hash-based) + cached CF IDs from localStorage —
	// the cached set renders the sidebar instantly without waiting on
	// the fingerprint round-trip below.
	const ids = await getInstalledProjectIds(instancePath).catch(handleError)
	if (ids) {
		const cfIds = loadCfInstalledProjectIds(instancePath)
		const merged = [...ids, ...cfIds]
		debugLog('installedProjectIds loaded', { count: merged.length, cfCount: cfIds.length })
		installedProjectIds.value = merged
	}

	// Authoritative CF discovery in the background — XMCL-style:
	// hash every .jar locally, POST to CurseForge /v1/fingerprints,
	// union with the cached CF set. Catches mods added manually
	// (drag-and-drop). The store keeps freshly-installed mods even
	// if CF temporarily fails to recognise their fingerprint.
	refreshCfInstalledFromFingerprints(instancePath)
		.then((cfIds) => {
			debugLog('CF fingerprint scan complete', { cfCount: cfIds.length })
			// Replace the CF portion of installedProjectIds with the
			// merged result the store just produced — it's already the
			// union of (previous cache) ∪ (fingerprint-discovered).
			const current = installedProjectIds.value ?? []
			const modrinthOnly = current.filter((id) => !id.startsWith('cf:'))
			installedProjectIds.value = [...modrinthOnly, ...cfIds]
		})
		.catch((err) => {
			console.warn('[Browse] CF fingerprint refresh failed:', err)
		})

	// Load file basenames for filename-based CF installed detection.
	// Kept as a secondary fallback for the brief window before the
	// fingerprint scan finishes, and for any CF mod whose fingerprint
	// CF doesn't recognise (rare — usually means the file was edited).
	getProfileProjects(instancePath)
		.then((projects) => {
			const names = new Set<string>()
			for (const filePath of Object.keys(projects)) {
				const base = filePath.split('/').pop()
				if (base) names.add(base)
			}
			installedFileNames.value = names
			debugLog('installedFileNames loaded', { count: names.size })
		})
		.catch(() => {})
}

async function initInstanceContext() {
	debugLog('initInstanceContext', {
		queryI: route.query.i,
		queryAi: route.query.ai,
		querySid: route.query.sid,
		queryWid: route.query.wid,
		queryFrom: route.query.from,
	})
	await initServerContext()
	await refreshInstalledProjectIds()

	if (route.query.i) {
		instance.value =
			((await getInstance(route.query.i as string).catch(handleError)) as Instance | null) ?? null
		debugLog('instance loaded', {
			name: instance.value?.name,
			loader: instance.value?.loader,
			gameVersion: instance.value?.game_version,
		})

		if (instance.value?.link?.project_id) {
			debugLog('checking linked project for server status', instance.value.link.project_id)
			const projectV3 = await get_project_v3(
				instance.value.link.project_id,
				'must_revalidate',
			).catch(handleError)
			if (projectV3?.minecraft_server != null) {
				debugLog('instance is a server instance')
				isServerInstance.value = true
			}
		}
	}
}

function setBrowseHideInstalledFlag(flag: 'hide_installed_modpacks', value: boolean) {
	themeStore.featureFlags[flag] = value
	getSettings()
		.then((settings) => {
			settings.feature_flags[flag] = value
			return setSettings(settings)
		})
		.catch(handleError)
}

const hideInstalledModpacks = computed({
	get: () => themeStore.getFeatureFlag('hide_installed_modpacks'),
	set: (value: boolean) => setBrowseHideInstalledFlag('hide_installed_modpacks', value),
})

const instanceFilters = computed(() => {
	const filters = []

	if (instance.value) {
		const gameVersion = instance.value.game_version
		if (gameVersion) {
			filters.push({ type: 'game_version', option: gameVersion })
		}

		const platform = instance.value.loader
		const supportedModLoaders = ['fabric', 'forge', 'quilt', 'neoforge']

		if (platform && projectType.value === 'mod' && supportedModLoaders.includes(platform)) {
			filters.push({ type: 'mod_loader', option: platform })
		}

		if (isServerInstance.value) {
			filters.push({ type: 'environment', option: 'client' })
		}
	}

	if (
		(instance.value || projectType.value === 'modpack') &&
		(projectType.value === 'modpack' ? hideInstalledModpacks.value : instanceHideInstalled.value) &&
		hiddenInstanceProjectIds.value.size > 0
	) {
		for (const id of hiddenInstanceProjectIds.value) {
			filters.push({ type: 'project_id', option: `project_id:${id}`, negative: true })
		}
	}

	return filters
})

const serverHideInstalled = ref(false)
const hideSelectedServerInstalls = ref(false)
if (route.query.shi) {
	serverHideInstalled.value = route.query.shi === 'true'
}
const hiddenServerContentProjectIds = ref<Set<string>>(new Set())
const hiddenServerContentProjectIdsInitialized = ref(false)

function syncHiddenServerContentProjectIds() {
	hiddenServerContentProjectIds.value = new Set(serverContentProjectIds.value)
	hiddenServerContentProjectIdsInitialized.value = true
}

watch(
	serverContentProjectIds,
	() => {
		if (!hiddenServerContentProjectIdsInitialized.value) {
			syncHiddenServerContentProjectIds()
		}
	},
	{ immediate: true },
)

const serverContextFilters = computed(() => {
	const filters: { type: string; option: string; negative?: boolean }[] = []
	if (!serverContextServerData.value) return filters
	const pt = projectType.value

	if (pt !== 'modpack') {
		const gameVersion = serverContextServerData.value.mc_version
		if (gameVersion) filters.push({ type: 'game_version', option: gameVersion })

		const platform = serverContextServerData.value.loader?.toLowerCase()
		if (platform && ['fabric', 'forge', 'quilt', 'neoforge'].includes(platform))
			filters.push({ type: 'mod_loader', option: platform })
		if (platform && ['paper', 'purpur'].includes(platform))
			filters.push({ type: 'plugin_loader', option: platform })

		if (pt === 'mod') filters.push({ type: 'environment', option: 'server' })

		if (hideSelectedServerInstalls.value && queuedServerInstallProjectIds.value.size > 0) {
			for (const id of queuedServerInstallProjectIds.value) {
				filters.push({ type: 'project_id', option: `project_id:${id}`, negative: true })
			}
		}
	}

	if (pt === 'modpack') {
		filters.push(
			{ type: 'environment', option: 'client' },
			{ type: 'environment', option: 'server' },
		)

		if (hideInstalledModpacks.value && hiddenInstanceProjectIds.value.size > 0) {
			for (const id of hiddenInstanceProjectIds.value) {
				filters.push({ type: 'project_id', option: `project_id:${id}`, negative: true })
			}
		}
	}

	if (serverHideInstalled.value && hiddenServerContentProjectIds.value.size > 0) {
		for (const id of hiddenServerContentProjectIds.value) {
			filters.push({ type: 'project_id', option: `project_id:${id}`, negative: true })
		}
	}

	return filters
})

const combinedProvidedFilters = computed(() =>
	isServerContext.value ? serverContextFilters.value : instanceFilters.value,
)

const {
	serverPings,
	contextMenuRef,
	updateServerHits,
	getServerModpackContent,
	getServerCardActions,
	handleRightClick,
	handleOptionsClick,
} = useAppServerBrowse({
	instance,
	isFromWorlds,
	allInstalledIds,
	newlyInstalled,
	installingServerProjects,
	playServerProject,
	showAddServerToInstanceModal,
	handleError,
	router,
})

const offline = ref(!navigator.onLine)
window.addEventListener('offline', () => {
	debugLog('went offline')
	offline.value = true
})
window.addEventListener('online', () => {
	debugLog('went online')
	offline.value = false
})

const messages = defineMessages({
	addServersToInstance: {
		id: 'app.browse.add-servers-to-instance',
		defaultMessage: 'Adding server to instance',
	},
	addToAnInstance: {
		id: 'app.browse.add-to-an-instance',
		defaultMessage: 'Add to an instance',
	},
	environmentProvidedByServer: {
		id: 'search.filter.locked.server-environment.title',
		defaultMessage: 'Only client-side mods can be added to the server instance',
	},
	gameVersionProvidedByInstance: {
		id: 'search.filter.locked.instance-game-version.title',
		defaultMessage: 'Game version is provided by the instance',
	},
	gameVersionProvidedByServer: {
		id: 'search.filter.locked.server-game-version.title',
		defaultMessage: 'Game version is provided by the server',
	},
	hideAddedServers: {
		id: 'app.browse.hide-added-servers',
		defaultMessage: 'Hide servers already added',
	},
	hideInstalledModpacks: {
		id: 'app.browse.hide-installed-modpacks',
		defaultMessage: 'Hide already installed',
	},
	installingToServer: {
		id: 'app.browse.server.installing',
		defaultMessage: 'Installing',
	},
	backToInstance: {
		id: 'app.browse.back-to-instance',
		defaultMessage: 'Back to instance',
	},
	serverInstanceContentWarning: {
		id: 'app.browse.server-instance-content-warning',
		defaultMessage:
			'Adding content can break compatibility when joining the server. Any added content will also be lost when you update the server instance content.',
	},
	modLoaderProvidedByInstance: {
		id: 'search.filter.locked.instance-loader.title',
		defaultMessage: 'Loader is provided by the instance',
	},
	modpacksProjectType: {
		id: 'app.browse.project-type.modpacks',
		defaultMessage: 'Modpacks',
	},
	modLoaderProvidedByServer: {
		id: 'search.filter.locked.server-loader.title',
		defaultMessage: 'Loader is provided by the server',
	},
	providedByInstance: {
		id: 'search.filter.locked.instance',
		defaultMessage: 'Provided by the instance',
	},
	providedByServer: {
		id: 'search.filter.locked.server',
		defaultMessage: 'Provided by the server',
	},
	syncFilterButton: {
		id: 'search.filter.locked.instance.sync',
		defaultMessage: 'Sync with instance',
	},
	cfModInstalledTitle: {
		id: 'app.browse.cf-mod-installed.title',
		defaultMessage: 'Mod installed',
	},
	cfModInstalledText: {
		id: 'app.browse.cf-mod-installed.text',
		defaultMessage: 'The CurseForge mod was added to {instanceName}.',
	},
	cfModpackInstalledTitle: {
		id: 'app.browse.cf-modpack-installed.title',
		defaultMessage: 'Modpack installed',
	},
	cfModpackInstalledText: {
		id: 'app.browse.cf-modpack-installed.text',
		defaultMessage: '{name} is ready to play.',
	},
})

const projectType = ref<ProjectType>(route.params.projectType as ProjectType)

function resetInstanceContext() {
	if (!instance.value) return

	debugLog('instance context removed, resetting')
	instance.value = null
	installedProjectIds.value = null
	installedFileNames.value = new Set()
	instanceHideInstalled.value = false
	newlyInstalled.value = []
	hiddenInstanceProjectIds.value = new Set()
	hiddenInstanceProjectIdsInitialized.value = false
	isServerInstance.value = false
	browseBreadcrumb.reset()
	void refreshInstalledProjectIds()
}

watch(
	() => route.params.projectType as ProjectType,
	async (newType) => {
		if (!browseRouteActive.value) {
			return
		}
		if (isSetupServerContext.value) {
			enforceSetupModpackRoute(newType)
			if (newType !== 'modpack') return
		}

		if (!newType || newType === projectType.value) return

		debugLog('projectType route param changed', { from: projectType.value, to: newType })
		projectType.value = newType
	},
)

watch(
	() => route.query.i,
	(instanceId) => {
		if (!instanceId && route.path.startsWith('/browse')) {
			resetInstanceContext()
		}
	},
)

const selectableProjectTypes = computed(() => {
	let dataPacks = false,
		mods = false,
		modpacks = false

	if (instance.value) {
		if (
			availableGameVersions.value &&
			availableGameVersions.value.findIndex((x) => x.version === instance.value?.game_version) <=
				availableGameVersions.value.findIndex((x) => x.version === '1.13') &&
			!isServerInstance.value
		) {
			dataPacks = true
		}

		if (instance.value.loader !== 'vanilla') {
			mods = true
		}
	} else {
		dataPacks = true
		mods = true
		modpacks = true
	}

	const params: LocationQuery = {}

	if (route.query.i) params.i = route.query.i
	if (route.query.ai) params.ai = route.query.ai
	if (route.query.from) params.from = route.query.from
	if (route.query.sid) params.sid = route.query.sid
	if (effectiveServerWorldId.value) params.wid = effectiveServerWorldId.value

	const queryString = new URLSearchParams(params as Record<string, string>).toString()
	const suffix = queryString ? `?${queryString}` : ''

	if (isSetupServerContext.value) {
		return [
			{ label: formatMessage(messages.modpacksProjectType), href: `/browse/modpack${suffix}` },
		]
	}

	if (isFromWorlds.value) {
		return [{ label: 'Servers', href: `/browse/server${suffix}` }]
	}

	return [
		{ label: 'Modpacks', href: `/browse/modpack${suffix}`, shown: modpacks },
		{ label: 'Mods', href: `/browse/mod${suffix}`, shown: mods },
		{ label: 'Resource Packs', href: `/browse/resourcepack${suffix}` },
		{ label: 'Data Packs', href: `/browse/datapack${suffix}`, shown: dataPacks },
		{ label: 'Shaders', href: `/browse/shader${suffix}` },
		{ label: 'Servers', href: `/browse/server${suffix}`, shown: !instance.value },
	]
})

const installContext = computed(() => {
	if (isServerContext.value && serverContextServerData.value) {
		return {
			name: serverContextServerData.value.name,
			loader: serverContextServerData.value.loader ?? '',
			gameVersion: serverContextServerData.value.mc_version ?? '',
			serverId: serverIdQuery.value,
			upstream: serverContextServerData.value.upstream,
			iconSrc: null as string | null,
			isMedal: serverContextServerData.value.is_medal,
			backUrl: serverBackUrl.value,
			backLabel: serverBackLabel.value,
			heading: serverBrowseHeading.value,
			queuedCount: queuedServerInstallCount.value,
			selectedProjects: selectedServerInstallProjects.value,
			isInstallingSelected: isInstallingQueuedServerInstalls.value,
			skipNonEssentialWarnings: themeStore.getFeatureFlag('skip_non_essential_warnings'),
			installProgress: queuedInstallProgress.value,
			clearQueued: clearQueuedServerInstalls,
			clearSelected: clearQueuedServerInstalls,
			onBack: flushQueuedServerInstalls,
			discardSelectedAndBack: discardQueuedServerInstallsAndBack,
			installSelected: installQueuedServerInstallsAndBack,
		}
	}
	if (instance.value) {
		return {
			name: instance.value.name,
			loader: instance.value.loader,
			gameVersion: instance.value.game_version,
			iconSrc: instance.value.icon_path ? convertFileSrc(instance.value.icon_path) : null,
			backUrl: `/instance/${encodeURIComponent(instance.value.id)}${isFromWorlds.value ? '/worlds' : ''}`,
			backLabel: formatMessage(messages.backToInstance),
			heading: formatMessage(
				isFromWorlds.value ? messages.addServersToInstance : commonMessages.installingContentLabel,
			),
			warning:
				isServerInstance.value && !isFromWorlds.value
					? formatMessage(messages.serverInstanceContentWarning)
					: undefined,
		}
	}
	return null
})

const installingProjectIds = ref<Set<string>>(new Set())

function setProjectInstalling(projectId: string, installing: boolean) {
	const next = new Set(installingProjectIds.value)
	if (installing) {
		next.add(projectId)
	} else {
		next.delete(projectId)
	}
	installingProjectIds.value = next
}

const serverInstallQueue = {
	get: getQueuedServerInstallPlans,
	set: setQueuedServerInstallPlans,
}

function getCurrentSelectedInstallPreferences(projectTypeValue: string) {
	return getSelectedInstallPreferences({
		contentType: projectTypeValue,
		selectedFilters: searchState.currentFilters.value,
		providedFilters: combinedProvidedFilters.value,
		overriddenProvidedFilterTypes: searchState.overriddenProvidedFilterTypes.value,
	})
}

function getServerInstallTargetPreferences(contentType: BrowseInstallContentType) {
	return getTargetInstallPreferences(
		{
			gameVersion: serverContextServerData.value?.mc_version,
			loader: serverContextServerData.value?.loader,
		},
		contentType,
	)
}

function getInstanceInstallTargetPreferences(projectTypeValue: string) {
	return getTargetInstallPreferences(
		{
			gameVersion: instance.value?.game_version,
			loader: instance.value?.loader,
		},
		projectTypeValue,
	)
}

async function getInstallProjectVersions(projectId: string) {
	const project = await get_project(projectId, 'must_revalidate')
	return (await get_version_many(
		project.versions,
		'must_revalidate',
	)) as Labrinth.Versions.v2.Version[]
}

async function chooseInstanceInstallVersion(
	project: Labrinth.Search.v3.ResultSearchProject,
	projectTypeValue: string,
) {
	const targetInstance = instance.value
	if (!targetInstance) {
		return { versionId: null as string | null }
	}

	const selectedPreferences = getCurrentSelectedInstallPreferences(projectTypeValue)
	const targetPreferences = getInstanceInstallTargetPreferences(projectTypeValue)
	if (!preferencesDiffer(selectedPreferences, targetPreferences)) {
		return { versionId: null as string | null }
	}

	const selectedVersion = getLatestMatchingInstallVersion(
		await getInstallProjectVersions(project.project_id),
		selectedPreferences,
	)

	if (!selectedVersion) {
		return { versionId: null as string | null }
	}

	return { versionId: selectedVersion.id }
}

async function chooseFilterMatchingInstallVersion(
	project: Labrinth.Search.v3.ResultSearchProject,
	projectTypeValue: string,
) {
	const plan = await resolveInstallPlan({
		project: {
			project_id: project.project_id,
			title: project.title,
			icon_url: project.icon_url,
		},
		contentType: projectTypeValue as BrowseInstallContentType,
		selectedFilters: searchState.currentFilters.value,
		providedFilters: combinedProvidedFilters.value,
		overriddenProvidedFilterTypes: searchState.overriddenProvidedFilterTypes.value,
		targetPreferences: {},
		getProjectVersions: getInstallProjectVersions,
	})

	return { versionId: plan.versionId }
}

// ── CurseForge installs ──────────────────────────────────────────────────────
// Keyed by CurseForge numeric mod id — CF mods have no Modrinth project_id, so
// they can't reuse the Modrinth install-tracking state.
const cfInstalling = ref(new Set<number>())
const cfInstalled = ref(new Set<number>())
const cfInstallModal = ref<{ show: (mod: unknown) => void } | null>(null)

async function installCfMod(cfModId: number) {
	const inst = instance.value
	if (!inst || cfInstalling.value.has(cfModId) || cfInstalled.value.has(cfModId)) {
		return
	}
	cfInstalling.value.add(cfModId)
	try {
		await installCurseForgeMod(cfModId, inst.id, inst.game_version, inst.loader)
		cfInstalled.value.add(cfModId)
		// Register as newly-installed so the card shows "Installed" for the
		// rest of this session (cf: prefix matches the project_id format used
		// in unified-search hit objects for CurseForge-exclusive mods).
		onSearchResultInstalled(`cf:${cfModId}`)
		addNotification({
			title: formatMessage(messages.cfModInstalledTitle),
			text: formatMessage(messages.cfModInstalledText, { instanceName: inst.name }),
			type: 'success',
		})
	} catch (e) {
		handleError(e)
	} finally {
		cfInstalling.value.delete(cfModId)
	}
}

/** Install a CurseForge modpack — creates a new instance, no instance picker. */
async function installCfModpack(cfModId: number, name: string) {
	if (cfInstalling.value.has(cfModId) || cfInstalled.value.has(cfModId)) {
		return
	}
	cfInstalling.value.add(cfModId)
	try {
		const profile = await installCurseForgeModpack(cfModId)
		cfInstalled.value.add(cfModId)
		addNotification({
			title: formatMessage(messages.cfModpackInstalledTitle),
			text: formatMessage(messages.cfModpackInstalledText, { name }),
			type: 'success',
		})
		router.push(`/instance/${encodeURIComponent(profile)}`)
	} catch (e) {
		handleError(e)
	} finally {
		cfInstalling.value.delete(cfModId)
	}
}

function getCardActions(
	result: Labrinth.Search.v3.ResultSearchProject,
	currentProjectType: string,
): CardAction[] {
	if (currentProjectType === 'server') {
		return getServerCardActions(result)
	}

	const projectResult = result as Labrinth.Search.v3.ResultSearchProject & {
		installed?: boolean
		installing?: boolean
	}

	// CurseForge-exclusive mods install via the CurseForge pipeline. Installing
	// needs a target instance — without one the card just links to the CF page.
	const sources = (
		projectResult as {
			sources?: { modrinth?: unknown; curseforge?: { mod_id: number } }
		}
	).sources
	if (sources?.curseforge && !sources.modrinth) {
		const cfModId = sources.curseforge.mod_id
		const isCfInstalling = cfInstalling.value.has(cfModId)
		const isCfInstalled = cfInstalled.value.has(cfModId)

		// Modpacks create their own instance — install directly, no picker.
		if (currentProjectType === 'modpack') {
			return [
				{
					key: 'install',
					label: formatMessage(
						isCfInstalled
							? commonMessages.installedLabel
							: isCfInstalling
								? commonMessages.installingLabel
								: commonMessages.installButton,
					),
					icon: isCfInstalling ? SpinnerIcon : isCfInstalled ? CheckIcon : PlusIcon,
					iconClass: isCfInstalling ? 'animate-spin' : undefined,
					disabled: isCfInstalling || isCfInstalled,
					color: 'brand',
					type: 'outlined',
					onClick: () =>
						installCfModpack(cfModId, projectResult.title ?? projectResult.name ?? 'Modpack'),
				},
			]
		}

		const hasInstance = !!instance.value
		return [
			{
				key: 'install',
				label: formatMessage(
					isCfInstalled
						? commonMessages.installedLabel
						: isCfInstalling
							? commonMessages.installingLabel
							: hasInstance
								? commonMessages.installButton
								: messages.addToAnInstance,
				),
				icon: isCfInstalling
					? SpinnerIcon
					: isCfInstalled
						? CheckIcon
						: hasInstance
							? DownloadIcon
							: PlusIcon,
				iconClass: isCfInstalling ? 'animate-spin' : undefined,
				disabled: isCfInstalling || isCfInstalled,
				color: 'brand',
				type: 'outlined',
				onClick: () => {
					// With an instance context, install straight to it; otherwise
					// open the instance picker modal (same modal as Modrinth mods).
					if (hasInstance) {
						installCfMod(cfModId)
					} else {
						cfInstallModal.value?.show({
							id: cfModId,
							name: projectResult.title ?? projectResult.name ?? 'Mod',
							iconUrl: projectResult.icon_url ?? null,
						})
					}
				},
			},
		]
	}

	const isInstalled =
		projectResult.installed ||
		allInstalledIds.value.has(projectResult.project_id || '') ||
		serverContentProjectIds.value.has(projectResult.project_id || '') ||
		serverContextServerData.value?.upstream?.project_id === projectResult.project_id
	const isInstalling = installingProjectIds.value.has(projectResult.project_id)
	const showAsInstalled = isInstalled && currentProjectType !== 'modpack'

	if (
		isServerContext.value &&
		['modpack', 'mod', 'plugin', 'datapack'].includes(currentProjectType)
	) {
		const isQueued = queuedServerInstallProjectIds.value.has(projectResult.project_id)
		const isInstallingSelection = isInstallingQueuedServerInstalls.value
		const validatingInstall =
			isInstalling && currentProjectType !== 'modpack' && !isInstallingSelection
		const installLabel = showAsInstalled
			? commonMessages.installedLabel
			: isQueued
				? isInstalling || isInstallingSelection
					? validatingInstall
						? commonMessages.validatingLabel
						: messages.installingToServer
					: commonMessages.selectedLabel
				: isInstalling || isInstallingSelection
					? validatingInstall
						? commonMessages.validatingLabel
						: messages.installingToServer
					: commonMessages.installButton
		return [
			{
				key: 'install',
				label: formatMessage(installLabel),
				icon:
					isInstalling || isInstallingSelection
						? SpinnerIcon
						: isQueued || showAsInstalled
							? CheckIcon
							: PlusIcon,
				iconClass: isInstalling || isInstallingSelection ? 'animate-spin' : undefined,
				disabled: showAsInstalled || isInstalling || isInstallingSelection,
				color: isQueued && !isInstalling && !isInstallingSelection ? 'green' : 'brand',
				type: 'outlined',
				onClick: async () => {
					if (isQueued) {
						removeQueuedServerInstall(projectResult.project_id)
						return
					}

					const contentType = currentProjectType as BrowseInstallContentType
					const isModpack = contentType === 'modpack'
					const shouldShowInstalling = isModpack || !isQueued
					if (shouldShowInstalling) {
						setProjectInstalling(projectResult.project_id, true)
					}
					try {
						await requestInstall({
							project: projectResult,
							contentType,
							mode: isModpack ? 'immediate' : 'queue',
							selectedFilters: isModpack
								? []
								: stripServerRuntimeInstallFilters(searchState.currentFilters.value),
							providedFilters: isModpack ? [] : combinedProvidedFilters.value,
							overriddenProvidedFilterTypes: isModpack
								? []
								: stripServerRuntimeInstallOverrides(
										searchState.overriddenProvidedFilterTypes.value,
									),
							targetPreferences: getServerInstallTargetPreferences(contentType),
							getProjectVersions: getInstallProjectVersions,
							queue: serverInstallQueue,
							install: (plan) =>
								openServerModpackInstallFlow({
									projectId: plan.projectId,
									versionId: plan.versionId,
									name: plan.project.name,
									iconUrl: plan.project.icon_url ?? undefined,
								}),
						})
					} catch (err) {
						handleError(err as Error)
					} finally {
						if (shouldShowInstalling) {
							setProjectInstalling(projectResult.project_id, false)
						}
					}
				},
			},
		]
	}

	const isModpack = projectResult.project_types?.includes('modpack')
	const shouldUseInstallIcon = !!instance.value || isModpack

	return [
		{
			key: 'install',
			label: formatMessage(
				isInstalling
					? messages.installingToServer
					: showAsInstalled
						? commonMessages.installedLabel
						: shouldUseInstallIcon
							? commonMessages.installButton
							: messages.addToAnInstance,
			),
			icon: isInstalling ? SpinnerIcon : showAsInstalled ? CheckIcon : PlusIcon,
			iconClass: isInstalling ? 'animate-spin' : undefined,
			disabled: showAsInstalled || isInstalling,
			color: 'brand',
			type: 'outlined',
			onClick: async () => {
				setProjectInstalling(projectResult.project_id, true)
				try {
					const selectedInstall = instance.value
						? await chooseInstanceInstallVersion(projectResult, currentProjectType)
						: isModpack
							? await chooseFilterMatchingInstallVersion(projectResult, currentProjectType)
							: { versionId: null as string | null }
					if (selectedInstall === null) {
						setProjectInstalling(projectResult.project_id, false)
						return
					}
					const selectedPreferences = getCurrentSelectedInstallPreferences(currentProjectType)
					await installVersion(
						projectResult.project_id,
						selectedInstall.versionId,
						instance.value ? instance.value.id : null,
						'SearchCard',
						(versionId, installedProjectIds) => {
							setProjectInstalling(projectResult.project_id, false)
							if (versionId) {
								onSearchResultsInstalled(installedProjectIds ?? [projectResult.project_id])
							}
						},
						(profile) => {
							router.push(`/instance/${profile}`)
						},
						{
							preferredLoader: instance.value?.loader ?? selectedPreferences.loaders?.[0],
							preferredGameVersion:
								instance.value?.game_version ?? selectedPreferences.gameVersions?.[0],
						},
					)
				} catch (err) {
					setProjectInstalling(projectResult.project_id, false)
					handleError(err)
				}
			},
		},
	]
}

function onSearchResultInstalled(id: string) {
	if (isServerContext.value) {
		markServerProjectInstalled(id)
		return
	}
	const toAdd = [id]

	// Cross-platform mirroring: if the installed project is known to also
	// exist on the other platform, mark that ID as installed too — so a
	// Modrinth-source install also lights up the CurseForge card for the
	// same mod, and vice-versa. No-op when no mapping is known.
	if (id.startsWith('cf:')) {
		const cfId = Number(id.slice(3))
		if (!Number.isNaN(cfId)) {
			const mrId = lookupCfIdToMr(cfId)
			if (mrId) toAdd.push(mrId)
		}
	} else {
		const cfId = lookupMrIdToCf(id)
		if (cfId != null) toAdd.push(`cf:${cfId}`)
	}

	newlyInstalled.value = Array.from(new Set([...newlyInstalled.value, ...toAdd]))
}

function onSearchResultsInstalled(ids: string[]) {
	if (isServerContext.value) {
		for (const id of ids) {
			markServerProjectInstalled(id)
		}
		return
	}
	newlyInstalled.value = Array.from(new Set([...newlyInstalled.value, ...ids]))
}

/** Modrinth-only search — used internally and passed to unifiedSearch */
async function modrinthSearch(requestParams: string) {
	debugLog('searching modrinth v3', requestParams)
	const isServer = projectType.value === 'server'

	const rawResults = await queryClient.fetchQuery({
		queryKey: ['search', 'v3', requestParams],
		queryFn: () =>
			get_search_results_v3(requestParams, 'must_revalidate') as Promise<{
				result: Labrinth.Search.v3.SearchResults & {
					hits: (Labrinth.Search.v3.ResultSearchProject & { installed?: boolean })[]
				}
			} | null>,
		staleTime: 30_000,
	})

	if (!rawResults) {
		return { projectHits: [], serverHits: [], total_hits: 0, per_page: 20 }
	}

	for (const hit of rawResults.result.hits) {
		for (const identifier of [hit.project_id, hit.slug]) {
			if (identifier) {
				queryClient.setQueryData(['projects', 'summary', identifier], hit)
			}
		}
	}

	if (isServer) {
		const hits = rawResults.result.hits ?? []
		updateServerHits(hits)
		return {
			projectHits: [],
			serverHits: hits,
			total_hits: rawResults.result.total_hits ?? 0,
			per_page: rawResults.result.hits_per_page,
		}
	}

	const hits = rawResults.result.hits.map((hit) => {
		const mapped: Labrinth.Search.v3.ResultSearchProject & { installed?: boolean } = {
			...hit,
		}

		if (instance.value || isServerContext.value || projectType.value === 'modpack') {
			const installedIds =
				isServerContext.value && projectType.value !== 'modpack'
					? serverContentProjectIds.value
					: new Set([...newlyInstalled.value, ...(installedProjectIds.value ?? [])])
			mapped.installed = installedIds.has(hit.project_id)
		}

		return mapped
	})

	return {
		projectHits: hits,
		serverHits: [],
		total_hits: rawResults.result.total_hits,
		per_page: rawResults.result.hits_per_page,
	}
}

/**
 * Search entry point passed to useBrowseSearch.
 * - Servers: Modrinth only (CurseForge has no servers).
 * - Everything else: a single catalog chosen by the source toggle — the two
 *   catalogs are separate modules and results are never merged.
 */
/** Sidebar filter type ids that represent a loader choice. */
const LOADER_FILTER_TYPE_IDS = [
	'mod_loader',
	'modpack_loader',
	'plugin_loader',
	'shader_loader',
	'plugin_platform',
]

/** Pull game version / loader / categories out of the active sidebar filters. */
function extractCfFilters() {
	// Provided filters (the instance's locked game version + loader, shown as
	// "Provided by the instance") live in combinedProvidedFilters, NOT in
	// currentFilters — so they must be merged in, otherwise a CurseForge search
	// inside an instance silently ignores the instance's version/loader and
	// surfaces mods that have no file for it. Provided filters take precedence.
	const filters = [...combinedProvidedFilters.value, ...searchState.currentFilters.value]
	return {
		gameVersion: filters.find((f) => f.type === 'game_version')?.option,
		modLoader: filters.find((f) => LOADER_FILTER_TYPE_IDS.includes(f.type))?.option,
		categories: filters.filter((f) => f.type.startsWith('category_')).map((f) => f.option),
	}
}

async function search(requestParams: string) {
	if (projectType.value === 'server') {
		return modrinthSearch(requestParams)
	}
	const mode = isCurseForgeAvailable() ? sourceMode.value : 'modrinth'
	const result = await unifiedSearch(
		requestParams,
		projectType.value,
		modrinthSearch,
		mode,
		extractCfFilters(),
	)

	// Stamp installed flag for CurseForge hits — modrinthSearch only covers
	// Modrinth project IDs; CF hits need additional checks:
	//   1. localStorage-persisted CF mod IDs (new installs via this app)
	//   2. Curated cross-platform mapping (same mod on both platforms)
	//   3. Filename matching against the profile's installed files (catches
	//      pre-existing installs and edge cases CF fingerprint missed)
	if (instance.value) {
		const installedSet = new Set([...newlyInstalled.value, ...(installedProjectIds.value ?? [])])
		const fileNames = installedFileNames.value
		result.projectHits = result.projectHits.map((hit) => {
			if (hit.installed) return hit

			const directCfId = (hit as { cf_id?: number }).cf_id
			const cfProjectId = hit.project_id?.startsWith('cf:')
				? hit.project_id
				: directCfId != null
					? `cf:${directCfId}`
					: null

			// (1) Direct CF ID match
			if (cfProjectId && installedSet.has(cfProjectId)) {
				return { ...hit, installed: true }
			}

			// (2) Cross-platform mapping — if this is a CF hit whose mapped
			// Modrinth ID is in the installed set (user installed via Modrinth
			// but viewing a CF result), mark installed. And vice-versa.
			if (directCfId != null) {
				const mapped = lookupCfIdToMr(directCfId)
				if (mapped && installedSet.has(mapped)) {
					return { ...hit, installed: true }
				}
			}
			if (hit.project_id && !hit.project_id.startsWith('cf:')) {
				const mappedCfId = lookupMrIdToCf(hit.project_id)
				if (mappedCfId != null && installedSet.has(`cf:${mappedCfId}`)) {
					return { ...hit, installed: true }
				}
			}

			// (3) Filename fallback — latestFiles from the CF search response
			// carry the actual .jar names; if any matches an installed file,
			// the mod is already in the instance even if no ID matched.
			const cfFileNames = (hit as { _cfFileNames?: string[] })._cfFileNames
			if (fileNames.size > 0 && cfFileNames?.some((fn) => fileNames.has(fn))) {
				// Persist so the next visit uses the fast ID path.
				if (cfProjectId) {
					const cfModId = Number(cfProjectId.replace('cf:', ''))
					if (!Number.isNaN(cfModId)) {
						storeCfInstalled(route.query.i as string, cfModId)
					}
				}
				return { ...hit, installed: true }
			}
			return hit
		})
	}

	return result
}

const isServerFilterContext = computed(() => isServerContext.value || isServerInstance.value)

const lockedFilterMessages = computed(() => ({
	gameVersion: formatMessage(
		isServerFilterContext.value
			? messages.gameVersionProvidedByServer
			: messages.gameVersionProvidedByInstance,
	),
	modLoader: formatMessage(
		isServerFilterContext.value
			? messages.modLoaderProvidedByServer
			: messages.modLoaderProvidedByInstance,
	),
	environment: formatMessage(messages.environmentProvidedByServer),
	syncButton: formatMessage(messages.syncFilterButton),
	providedBy: formatMessage(
		isServerFilterContext.value ? messages.providedByServer : messages.providedByInstance,
	),
}))

const searchState = useBrowseSearch({
	projectType,
	tags,
	active: browseRouteActive,
	providedFilters: combinedProvidedFilters,
	search,
	persistentQueryParams: ['i', 'ai', 'shi', 'sid', 'wid', 'from'],
	getExtraQueryParams: () => ({
		sid: serverIdQuery.value || undefined,
		wid: effectiveServerWorldId.value || undefined,
		ai: instanceHideInstalled.value ? 'true' : undefined,
		shi: serverHideInstalled.value ? 'true' : undefined,
	}),
})

// installedProjectIds and installedFileNames load asynchronously after the
// initial search has already run, so the first render shows CF mods as
// "Install" instead of "Installed". Re-run the search when they arrive —
// but coalesced through a single rAF, because during init these three refs
// update in quick succession (Modrinth IDs, CF fingerprint scan, file-name
// scan) and we don't want to fire three concurrent refreshSearches that
// race + cancel each other (which also makes Vue's Suspense unmount path
// throw null-DOM errors).
let refreshScheduled = false
watch([installedProjectIds, installedFileNames], () => {
	if (!instance.value || refreshScheduled) return
	refreshScheduled = true
	requestAnimationFrame(() => {
		refreshScheduled = false
		if (!instance.value) return
		searchState.refreshSearch().catch(() => {})
	})
})

// ── Source colour accent ─────────────────────────────────────────────────────
// When enabled, recolours the Discover page's brand accent to match the active
// catalog. Scoped to this page via CSS vars on the root element.
const accentBySource = useSourceAccent()

const SOURCE_ACCENTS = {
	modrinth: {
		'--color-brand': '#00af5c',
		'--color-brand-highlight': 'rgba(0, 175, 92, 0.25)',
		'--color-brand-shadow': 'rgba(0, 175, 92, 0.6)',
	},
	curseforge: {
		'--color-brand': '#e0561f',
		'--color-brand-highlight': 'rgba(224, 86, 31, 0.25)',
		'--color-brand-shadow': 'rgba(224, 86, 31, 0.6)',
	},
} as const

const accentStyle = computed<Record<string, string>>(() => {
	if (!accentBySource.value) return {}
	return SOURCE_ACCENTS[sourceMode.value]
})

watch(
	[
		() => searchState.query.value,
		() =>
			searchState.isServerType.value
				? searchState.serverCurrentFilters.value
				: searchState.currentFilters.value,
		() => projectType.value,
	],
	() => {
		if (isServerContext.value && projectType.value !== 'modpack') {
			syncHiddenServerContentProjectIds()
		} else if (instance.value || projectType.value === 'modpack') {
			syncHiddenInstanceProjectIds()
		}
	},
	{ deep: true },
)

watch(queuedServerInstallCount, (count) => {
	if (count === 0) {
		hideSelectedServerInstalls.value = false
	}
})

// Re-run the search when the catalog toggle changes (it is not part of the
// request params that useBrowseSearch watches). On switch, drop filters that
// don't carry across catalogs so they don't silently shape the next query:
//   - category filters: Modrinth and CurseForge have different taxonomies, so
//     a selection from one catalog is meaningless (and returns nothing) in the
//     other. Game version and loader are universal and are kept.
//   - CurseForge-unsupported groups (environment, license) when entering CF.
watch(sourceMode, (mode) => {
	const cleaned = searchState.currentFilters.value.filter((f) => {
		if (f.type.startsWith('category_')) return false
		if (mode === 'curseforge' && CF_UNSUPPORTED_FILTER_IDS.has(f.type)) return false
		return true
	})
	if (cleaned.length !== searchState.currentFilters.value.length) {
		searchState.currentFilters.value = cleaned
	}
	// Catalogs have different result counts, so the current page may not exist
	// in the catalog being switched to — reset to the first page.
	searchState.currentPage.value = 1
	searchState.refreshSearch()
})

if (instance.value?.game_version) {
	const gv = instance.value.game_version
	const alreadyHasGv = searchState.serverCurrentFilters.value.some(
		(f) => f.type === 'server_game_version' && f.option === gv,
	)
	if (!alreadyHasGv) {
		searchState.serverCurrentFilters.value.push({ type: 'server_game_version', option: gv })
	}
}

void searchState.refreshSearch()

type UnlistenFn = () => void

let isUnmounted = false
let unlistenInstances: UnlistenFn | null = null

onMounted(() => {
	instance_listener(async (event: { event: string; instance_id: string }) => {
		if (event.event === 'added' || event.event === 'created' || event.event === 'removed') {
			if (!route.query.i) {
				await refreshInstalledProjectIds()
				if (projectType.value === 'modpack') {
					if (event.event === 'removed') {
						syncHiddenInstanceProjectIds()
					}
					await searchState.refreshSearch()
				}
			}
		}

		if (instance.value && event.instance_id === instance.value.id && event.event === 'synced') {
			await refreshInstalledProjectIds()
			await searchState.refreshSearch()
		}
	})
		.then((unlisten) => {
			if (isUnmounted) {
				unlisten()
				return
			}

			unlistenInstances = unlisten
		})
		.catch(handleError)
})

onUnmounted(() => {
	isUnmounted = true
	unlistenInstances?.()
})

function getProjectBrowseQuery() {
	if (!browseRouteActive.value) {
		return undefined
	}
	if (!installContext.value) return undefined
	return {
		...route.query,
		b: route.fullPath,
	}
}

const visibleFilters = computed(() =>
	sourceMode.value === 'curseforge'
		? searchState.filters.value.filter((f) => !CF_UNSUPPORTED_FILTER_IDS.has(f.id))
		: searchState.filters.value,
)

const advancedFiltersCollapsed = computed({
	get: () => themeStore.getFeatureFlag('advanced_filters_collapsed'),
	set: (value) => {
		themeStore.featureFlags['advanced_filters_collapsed'] = value
		getSettings()
			.then((settings) => {
				settings.feature_flags['advanced_filters_collapsed'] = value
				return setSettings(settings)
			})
			.catch(handleError)
	},
})

provideBrowseManager({
	tags,
	projectType,
	...searchState,
	advancedFiltersCollapsed,
	filters: visibleFilters,
	// Catalog toggle — only offered when a CurseForge API key is configured.
	sourceMode: isCurseForgeAvailable() ? sourceMode : undefined,
	getProjectLink: (result: Labrinth.Search.v3.ResultSearchProject) => {
		// CurseForge-exclusive mods have no Modrinth page — route them to the
		// in-app CurseForge mod page instead of a /project/* page that 404s.
		const sources = (
			result as {
				sources?: { modrinth?: unknown; curseforge?: { mod_id: number } }
			}
		).sources
		if (sources?.curseforge && !sources.modrinth) {
			return {
				path: `/curseforge/${sources.curseforge.mod_id}`,
				query: getProjectBrowseQuery(),
			}
		}
		return {
			path: `/project/${result.project_id ?? result.slug}`,
			query: getProjectBrowseQuery(),
		}
	},
	getServerProjectLink: (result: Labrinth.Search.v3.ResultSearchProject) => ({
		path: `/project/${result.slug ?? result.project_id}`,
		query: getProjectBrowseQuery(),
	}),
	selectableProjectTypes,
	showProjectTypeTabs: computed(() => !isServerContext.value),
	variant: 'app',
	getCardActions,
	installContext,
	providedFilters: combinedProvidedFilters,
	hideInstalled: computed({
		get: () => {
			if (projectType.value === 'modpack') return hideInstalledModpacks.value
			if (isServerContext.value) return serverHideInstalled.value
			return instanceHideInstalled.value
		},
		set: (val: boolean) => {
			if (projectType.value === 'modpack') {
				hideInstalledModpacks.value = val
				if (val) syncHiddenInstanceProjectIds()
				return
			}
			if (isServerContext.value) {
				serverHideInstalled.value = val
				if (val) syncHiddenServerContentProjectIds()
			} else {
				instanceHideInstalled.value = val
				if (val) syncHiddenInstanceProjectIds()
			}
		},
	}),
	showHideInstalled: computed(
		() =>
			projectType.value === 'modpack' ||
			(isServerContext.value && projectType.value !== 'modpack') ||
			!!instance.value,
	),
	hideInstalledLabel: computed(() =>
		formatMessage(
			isFromWorlds.value
				? messages.hideAddedServers
				: projectType.value === 'modpack'
					? messages.hideInstalledModpacks
					: commonMessages.hideInstalledContentLabel,
		),
	),
	hideSelected: hideSelectedServerInstalls,
	showHideSelected: computed(
		() =>
			isServerContext.value &&
			projectType.value !== 'modpack' &&
			queuedServerInstallCount.value > 0,
	),
	hideSelectedLabel: computed(() => formatMessage(commonMessages.hideSelectedContentLabel)),
	onInstalled: onSearchResultInstalled,
	serverPings,
	getServerModpackContent,
	onContextMenu: handleRightClick,
	offline,
	lockedFilterMessages,
})
</script>

<template>
	<div class="flex flex-col gap-3 p-6" :style="accentStyle">
		<BrowsePageLayout>
			<template #after>
				<ContextMenu ref="contextMenuRef" @option-clicked="handleOptionsClick">
					<template #open_link>
						<GlobeIcon /> {{ formatMessage(commonMessages.openInModrinthButton) }} <ExternalIcon />
					</template>
					<template #copy_link>
						<ClipboardCopyIcon /> {{ formatMessage(commonMessages.copyLinkButton) }}
					</template>
				</ContextMenu>
			</template>
		</BrowsePageLayout>
		<CreationFlowModal
			v-if="isServerContext && projectType === 'modpack'"
			ref="serverSetupModalRef"
			:type="serverFlowFrom === 'reset-server' ? 'reset-server' : 'server-onboarding'"
			:available-loaders="['vanilla', 'fabric', 'neoforge', 'forge', 'quilt', 'paper', 'purpur']"
			:show-snapshot-toggle="true"
			:on-back="onServerFlowBack"
			:search-modpacks="searchServerModpacks"
			:get-project-versions="getServerProjectVersions"
			:get-loader-manifest="getLoaderManifest"
			@hide="() => {}"
			@browse-modpacks="() => {}"
			@create="handleServerModpackFlowCreate"
		/>
		<CurseForgeInstallModal ref="cfInstallModal" />
		<Teleport v-if="browseRouteActive" to="#sidebar-teleport-target">
			<BrowseSidebar />
		</Teleport>
	</div>
</template>
