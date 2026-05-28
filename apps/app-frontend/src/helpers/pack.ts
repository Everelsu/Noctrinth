import { invoke } from '@tauri-apps/api/core'

import { CURSEFORGE_API_KEY } from './curseforge-key'
import { create } from './profile'
import type { InstanceLoader } from './types'

interface PackProfileCreator {
	name: string
	gameVersion: string
	modloader: InstanceLoader
	loaderVersion: string | null
	unknownFile: boolean
}

interface PackLocationVersionId {
	type: 'fromVersionId'
	project_id: string
	version_id: string
	title: string
	icon_url?: string
}

interface PackLocationFile {
	type: 'fromFile'
	path: string
	/** Lets the backend resolve CurseForge modpack zips imported from a file. */
	curseforge_api_key?: string
}

export async function create_profile_and_install(
	projectId: string,
	versionId: string,
	packTitle: string,
	iconUrl?: string,
	createInstanceCallback: (profile: string) => void = () => {},
): Promise<void> {
	const location: PackLocationVersionId = {
		type: 'fromVersionId',
		project_id: projectId,
		version_id: versionId,
		title: packTitle,
		icon_url: iconUrl,
	}
	const profile_creator = await invoke<PackProfileCreator>(
		'plugin:pack|pack_get_profile_from_pack',
		{ location },
	)
	const profile = await create(
		profile_creator.name,
		profile_creator.gameVersion,
		profile_creator.modloader,
		profile_creator.loaderVersion,
		null,
		true,
	)
	createInstanceCallback(profile)

	return await invoke('plugin:pack|pack_install', { location, profile })
}

export async function install_to_existing_profile(
	projectId: string,
	versionId: string,
	title: string,
	profilePath: string,
): Promise<void> {
	const location: PackLocationVersionId = {
		type: 'fromVersionId',
		project_id: projectId,
		version_id: versionId,
		title,
	}
	return await invoke('plugin:pack|pack_install', { location, profile: profilePath })
}

/**
 * Install a CurseForge modpack: create a fresh profile, then download &
 * install the pack into it. Game version and loader are corrected from the
 * pack manifest during install, so the placeholders here are temporary.
 *
 * @returns the created profile path
 */
export async function create_profile_and_install_from_curseforge(
	modpackUrl: string,
	title: string,
	curseforgeApiKey: string,
): Promise<string> {
	const profile = await create(title, '1.21.1', 'vanilla' as InstanceLoader, null, null, true)
	await invoke('plugin:pack|pack_install_curseforge', {
		modpackUrl,
		curseforgeApiKey,
		profile,
	})
	return profile
}

export async function create_profile_and_install_from_file(
	path: string,
	showUnknownPackWarningModal?: (createProfile: () => Promise<void>, fileName: string) => void,
): Promise<void> {
	const location: PackLocationFile = {
		type: 'fromFile',
		path,
		// Always supplied — the backend only uses it when the file turns
		// out to be a CurseForge modpack (manifest.json), and now also
		// rejects CF modpacks early when this is empty so the user sees
		// "need an API key" rather than a confusing 401 mid-install.
		curseforge_api_key: CURSEFORGE_API_KEY,
	}
	const profile_creator = await invoke<PackProfileCreator>(
		'plugin:pack|pack_get_profile_from_pack',
		{ location },
	)

	const createProfile = async () => {
		const profile = await create(
			profile_creator.name,
			profile_creator.gameVersion,
			profile_creator.modloader,
			profile_creator.loaderVersion,
			null,
			true,
		)
		await invoke('plugin:pack|pack_install', { location, profile })
	}

	if (profile_creator.unknownFile && showUnknownPackWarningModal) {
		const splitPath = path.split(/[\\/]/)
		const fileName = splitPath ? splitPath[splitPath.length - 1] : path
		showUnknownPackWarningModal(createProfile, fileName)
	} else {
		await createProfile()
	}
}
