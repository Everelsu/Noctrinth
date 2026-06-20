<template>
	<ContentInstallModal
		ref="modal"
		:instances="instances"
		:compatible-loaders="compatibleLoaders"
		:game-versions="gameVersions"
		:release-game-versions="releaseGameVersions"
		:loading="loading"
		:project-info="projectInfo"
		@install="onInstall"
		@create-and-install="onCreateAndInstall"
	/>
</template>

<script setup>
/**
 * CurseForge install modal.
 *
 * Reuses Modrinth's presentational <ContentInstallModal> (existing/new
 * instance picker) but feeds it CurseForge data and installs through the
 * CurseForge pipeline. The Modrinth content-install provider is untouched.
 *
 * Usage:
 *   <CurseForgeInstallModal ref="cfModal" />
 *   cfModal.value.show({ id, name, iconUrl })
 */
import {
	ContentInstallModal,
	defineMessages,
	injectNotificationManager,
	useVIntl,
} from '@modrinth/ui'
import { convertFileSrc } from '@tauri-apps/api/core'
import { ref } from 'vue'
import { useRouter } from 'vue-router'

import { loadCfInstalledProjectIds } from '@/helpers/cf-installed-store'
import {
	bestCfFileFor,
	getCurseForgeModFiles,
	installCurseForgeFile,
} from '@/helpers/curseforge-api'
import { create, get as getInstance, list as listInstances } from '@/helpers/profile.js'
import { get_game_versions } from '@/helpers/tags'

const { formatMessage } = useVIntl()
const { handleError, addNotification } = injectNotificationManager()
const router = useRouter()

const messages = defineMessages({
	instanceLoadError: {
		id: 'app.curseforge-install.instance-load-error',
		defaultMessage: 'Instance could not be loaded.',
	},
	installedTitle: {
		id: 'app.curseforge-install.installed.title',
		defaultMessage: 'Mod installed',
	},
	installedText: {
		id: 'app.curseforge-install.installed.text',
		defaultMessage: '{modName} was added to {instanceName}.',
	},
	incompatibleTitle: {
		id: 'app.curseforge-install.incompatible.title',
		defaultMessage: 'Incompatible mod warning',
	},
	incompatibleText: {
		id: 'app.curseforge-install.incompatible.text',
		defaultMessage:
			'{modName} declares {count, plural, one {# incompatible mod} other {# incompatible mods}} — check {instanceName} for conflicts.',
	},
	optionalTitle: {
		id: 'app.curseforge-install.optional.title',
		defaultMessage: 'Optional dependencies available',
	},
	optionalText: {
		id: 'app.curseforge-install.optional.text',
		defaultMessage:
			'{modName} has {count, plural, one {# optional add-on} other {# optional add-ons}} — open its page to install them.',
	},
})

const modal = ref(null)
const loading = ref(false)
const instances = ref([])
const compatibleLoaders = ref([])
const gameVersions = ref([])
const releaseGameVersions = ref(new Set())
const projectInfo = ref(null)

// Non-reactive working state for the current modal session.
let cfFiles = []
let currentMod = null

// Mirror of CF_REAL_LOADERS in helpers/curseforge-api.ts. Only real mod
// loaders disqualify a file from a vanilla instance — informational tags
// like "Iris", "OptiFine", "Data Pack" do not.
const CF_REAL_LOADERS = new Set(['forge', 'fabric', 'quilt', 'neoforge', 'cauldron', 'liteloader'])

function isCompatible(gameVersion, loader) {
	const loaderLower = (loader ?? '').toLowerCase()
	const isVanilla = loaderLower === '' || loaderLower === 'vanilla'
	return cfFiles.some((f) => {
		if (!f.gameVersions.includes(gameVersion)) return false
		// Only treat actual mod loaders as constraining — see CF_REAL_LOADERS.
		const fileLoaders = f.gameVersions
			.filter((v) => !/^\d/.test(v))
			.map((v) => v.toLowerCase())
			.filter((v) => CF_REAL_LOADERS.has(v))
		if (isVanilla) {
			// Vanilla: only files that don't declare a specific loader qualify.
			return fileLoaders.length === 0
		}
		// Modded: accept this loader's files, or universal files with no loader.
		return fileLoaders.length === 0 || fileLoaders.includes(loaderLower)
	})
}

/**
 * Build a descriptive "no match" error listing the MC versions the mod
 * actually has files for — much more helpful than just "no compatible file".
 */
function buildNoFitMessage(gameVersion, loader) {
	const availableVersions = Array.from(
		new Set(cfFiles.flatMap((f) => f.gameVersions.filter((v) => /^\d/.test(v)))),
	).sort()
	const hint =
		availableVersions.length > 0
			? ` Available: ${availableVersions.slice(0, 8).join(', ')}${
					availableVersions.length > 8 ? '…' : ''
				}.`
			: ''
	const target = `${gameVersion}${loader ? ` / ${loader}` : ''}`
	return `No CurseForge file fits ${target}.${hint}`
}

/**
 * Open the modal for a CurseForge mod.
 *
 * @param mod         { id, name, iconUrl }
 * @param forcedFile  Optional specific CfModFile — when given, only that file
 *                    is offered (used by the per-version install buttons).
 *                    When omitted, the best file is auto-picked per instance.
 */
async function show(mod, forcedFile = null) {
	currentMod = mod
	cfFiles = []
	instances.value = []
	compatibleLoaders.value = []
	gameVersions.value = []
	releaseGameVersions.value = new Set()
	projectInfo.value = {
		title: mod.name,
		iconUrl: mod.iconUrl ?? null,
		link: `/curseforge/${mod.id}`,
	}

	loading.value = true
	modal.value?.show()

	try {
		const [profiles, allGameVersions] = await Promise.all([
			listInstances().catch(() => []),
			get_game_versions().catch(() => []),
		])
		cfFiles = forcedFile ? [forcedFile] : ((await getCurseForgeModFiles(mod.id)) ?? [])

		// CurseForge file.gameVersions mixes Minecraft versions and loader names.
		const loaderSet = new Set()
		const gvSet = new Set()
		for (const file of cfFiles) {
			for (const v of file.gameVersions) {
				if (/^\d/.test(v)) gvSet.add(v)
				else loaderSet.add(v.toLowerCase())
			}
		}
		compatibleLoaders.value = [...loaderSet]

		// Order game versions by the canonical list; collect release versions.
		const ordered = []
		const releases = new Set()
		for (const gv of allGameVersions) {
			if (gvSet.has(gv.version)) {
				ordered.push(gv.version)
				if (gv.version_type === 'release') releases.add(gv.version)
			}
		}
		gameVersions.value = ordered.length > 0 ? ordered : [...gvSet]
		releaseGameVersions.value = releases

		const cfTag = `cf:${mod.id}`
		instances.value = profiles.map((profile) => ({
			id: profile.path,
			name: profile.name,
			iconUrl: profile.icon_path ? convertFileSrc(profile.icon_path) : null,
			// Check the per-instance localStorage store for prior CF installs of
			// this mod, so already-installed instances show "Installed" up-front.
			installed: loadCfInstalledProjectIds(profile.path).includes(cfTag),
			compatible: isCompatible(profile.game_version, profile.loader),
			installing: false,
		}))
	} catch (e) {
		handleError(e)
	} finally {
		loading.value = false
	}
}

async function onInstall(instance) {
	const row = instances.value.find((i) => i.id === instance.id)
	const target = await getInstance(instance.id).catch(() => null)
	if (!target) {
		handleError(new Error(formatMessage(messages.instanceLoadError)))
		return
	}

	const file = bestCfFileFor(cfFiles, target.game_version, target.loader)
	if (!file) {
		handleError(new Error(buildNoFitMessage(target.game_version, target.loader)))
		return
	}

	if (row) row.installing = true
	try {
		const result = await installCurseForgeFile(
			file,
			target.path,
			target.game_version,
			target.loader,
		)
		if (row) {
			row.installed = true
			row.installing = false
		}
		addNotification({
			title: formatMessage(messages.installedTitle),
			text: formatMessage(messages.installedText, {
				modName: currentMod.name,
				instanceName: target.name,
			}),
			type: 'success',
		})
		notifyAboutDeps(result, target.name)
	} catch (e) {
		if (row) row.installing = false
		handleError(e)
	}
}

/** Surface optional/incompatible CF deps the auto-installer didn't pull in. */
function notifyAboutDeps(result, instanceName) {
	if (result?.incompatible?.length) {
		addNotification({
			title: formatMessage(messages.incompatibleTitle),
			text: formatMessage(messages.incompatibleText, {
				modName: currentMod.name,
				count: result.incompatible.length,
				instanceName,
			}),
			type: 'warn',
		})
	}
	if (result?.optional?.length) {
		addNotification({
			title: formatMessage(messages.optionalTitle),
			text: formatMessage(messages.optionalText, {
				modName: currentMod.name,
				count: result.optional.length,
			}),
			type: 'info',
		})
	}
}

async function onCreateAndInstall(data) {
	try {
		// Pick the file BEFORE creating the profile — otherwise an incompatible
		// pick leaves the user with an empty orphan instance.
		const file = bestCfFileFor(cfFiles, data.gameVersion, data.loader)
		if (!file) {
			handleError(new Error(buildNoFitMessage(data.gameVersion, data.loader)))
			return
		}

		const id = await create(
			data.name,
			data.gameVersion,
			data.loader,
			'latest',
			data.iconPath,
			false,
		)
		if (!id) return

		// ContentInstallModal already closes itself on create-and-install.
		const result = await installCurseForgeFile(file, id, data.gameVersion, data.loader)
		addNotification({
			title: formatMessage(messages.installedTitle),
			text: formatMessage(messages.installedText, {
				modName: currentMod.name,
				instanceName: data.name,
			}),
			type: 'success',
		})
		notifyAboutDeps(result, data.name)
		await router.push(`/instance/${encodeURIComponent(id)}`)
	} catch (e) {
		handleError(e)
	}
}

defineExpose({ show })
</script>
