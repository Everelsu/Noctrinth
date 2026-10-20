/**
 * Noctrinth's own: the CurseForge settings. Kept in local storage, like the
 * translation settings; what the backend acts on is handed to it at startup
 * and on every change, as the key is.
 */
import { invoke } from '@tauri-apps/api/core'
import { reactive, watch } from 'vue'

import { CURSEFORGE_API_KEY } from './curseforge-key'

export type CurseforgeReleaseType = 'release' | 'beta' | 'alpha'

export interface CurseforgeSettings {
	/** Off, and the launcher asks CurseForge nothing at all. */
	enabled: boolean
	/** Where the browse page opens when it is not told. */
	defaultSource: 'modrinth' | 'curseforge'
	/** Recognise files that came with no ids by their fingerprint. */
	recognizeFiles: boolean
	checkUpdates: boolean
	/** The least stable file an update may be, as CurseForge's app has it. */
	updateReleaseType: CurseforgeReleaseType
	/** Look for files kept off launchers in other launchers' instances. */
	searchOtherLaunchers: boolean
}

const STORAGE_KEY = 'noctrinth-curseforge'

const DEFAULTS: CurseforgeSettings = {
	enabled: true,
	defaultSource: 'modrinth',
	recognizeFiles: true,
	checkUpdates: true,
	updateReleaseType: 'release',
	searchOtherLaunchers: true,
}

function load(): CurseforgeSettings {
	try {
		const stored = JSON.parse(
			localStorage.getItem(STORAGE_KEY) ?? 'null',
		) as Partial<CurseforgeSettings> | null
		return { ...DEFAULTS, ...stored }
	} catch {
		return { ...DEFAULTS }
	}
}

export const curseforgeSettings = reactive<CurseforgeSettings>(load())

const RELEASE_TYPE_IDS: Record<CurseforgeReleaseType, number> = { release: 1, beta: 2, alpha: 3 }

/** Hands the backend the key, or none when CurseForge is off, and the options. */
export async function applyCurseforgeSettings(): Promise<void> {
	await invoke('plugin:noctrinth-curseforge|curseforge_set_api_key', {
		key: (curseforgeSettings.enabled && CURSEFORGE_API_KEY) || null,
	})
	await invoke('plugin:noctrinth-curseforge|curseforge_set_options', {
		options: {
			recognize_files: curseforgeSettings.recognizeFiles,
			check_updates: curseforgeSettings.checkUpdates,
			update_release_type: RELEASE_TYPE_IDS[curseforgeSettings.updateReleaseType],
			search_other_launchers: curseforgeSettings.searchOtherLaunchers,
		},
	})
}

export async function clearCurseforgeCache(): Promise<void> {
	await invoke('plugin:noctrinth-curseforge|curseforge_clear_cache')
}

watch(
	curseforgeSettings,
	(settings) => {
		try {
			localStorage.setItem(STORAGE_KEY, JSON.stringify(settings))
		} catch (error) {
			console.warn('Failed to remember the CurseForge settings:', error)
		}
		void applyCurseforgeSettings().catch((error) =>
			console.warn('Failed to hand the CurseForge settings over:', error),
		)
	},
	{ deep: true },
)
