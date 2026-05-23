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
import { ContentInstallModal, injectNotificationManager } from '@modrinth/ui'
import { convertFileSrc } from '@tauri-apps/api/core'
import { ref } from 'vue'
import { useRouter } from 'vue-router'

import {
	bestCfFileFor,
	getCurseForgeModFiles,
	installCurseForgeFile,
} from '@/helpers/curseforge-api'
import { create, get as getInstance, list as listInstances } from '@/helpers/profile.js'
import { get_game_versions } from '@/helpers/tags'

const { handleError, addNotification } = injectNotificationManager()
const router = useRouter()

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

function isCompatible(gameVersion, loader) {
	const loaderLower = (loader ?? '').toLowerCase()
	return cfFiles.some(
		(f) =>
			f.gameVersions.includes(gameVersion) &&
			f.gameVersions.some((v) => v.toLowerCase() === loaderLower),
	)
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

		instances.value = profiles.map((profile) => ({
			id: profile.path,
			name: profile.name,
			iconUrl: profile.icon_path ? convertFileSrc(profile.icon_path) : null,
			installed: false,
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
		handleError(new Error('Instance could not be loaded.'))
		return
	}

	const file = bestCfFileFor(cfFiles, target.game_version, target.loader)
	if (!file) {
		handleError(new Error('No CurseForge file is compatible with this instance.'))
		return
	}

	if (row) row.installing = true
	try {
		await installCurseForgeFile(file, target.path)
		if (row) {
			row.installed = true
			row.installing = false
		}
		addNotification({
			title: 'Mod installed',
			text: `${currentMod.name} was added to ${target.name}.`,
			type: 'success',
		})
	} catch (e) {
		if (row) row.installing = false
		handleError(e)
	}
}

async function onCreateAndInstall(data) {
	try {
		const id = await create(data.name, data.gameVersion, data.loader, 'latest', data.iconPath, false)
		if (!id) return

		const file = bestCfFileFor(cfFiles, data.gameVersion, data.loader)
		if (!file) {
			handleError(new Error('No CurseForge file is compatible with this version.'))
			return
		}

		// ContentInstallModal already closes itself on create-and-install.
		await installCurseForgeFile(file, id)
		addNotification({
			title: 'Mod installed',
			text: `${currentMod.name} was added to ${data.name}.`,
			type: 'success',
		})
		await router.push(`/instance/${encodeURIComponent(id)}`)
	} catch (e) {
		handleError(e)
	}
}

defineExpose({ show })
</script>
