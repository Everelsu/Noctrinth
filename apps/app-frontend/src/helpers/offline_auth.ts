/**
 * Accounts that are only a name.
 *
 * What the launcher plays as when nothing can be reached — see
 * `packages/app-lib/src/state/offline_auth.rs` for what one of these can and
 * cannot do.
 */
import { invoke } from '@tauri-apps/api/core'

export interface OfflineCredentials {
	profile: { id: string; name: string; skins: never[]; capes: never[] }
	active: boolean
	auth_provider: 'offline'
	created: string
}

/** Adds an account for this name, and makes it the one to play as. */
export async function offline_add(username: string): Promise<OfflineCredentials> {
	return await invoke('plugin:noctrinth-offline-auth|offline_add', { username })
}

export async function offline_remove(uuid: string): Promise<void> {
	return await invoke('plugin:noctrinth-offline-auth|offline_remove', { uuid })
}

export async function offline_users(): Promise<OfflineCredentials[]> {
	return await invoke('plugin:noctrinth-offline-auth|offline_users')
}

export async function offline_get_default_user(): Promise<string | null> {
	return await invoke('plugin:noctrinth-offline-auth|offline_get_default_user')
}

export async function offline_set_default_user(uuid: string): Promise<void> {
	return await invoke('plugin:noctrinth-offline-auth|offline_set_default_user', { uuid })
}

/** The UUID a name would play as, before anything is saved. */
export async function offline_preview_uuid(username: string): Promise<string | null> {
	return await invoke('plugin:noctrinth-offline-auth|offline_preview_uuid', { username })
}

export type PlayerCapeSource =
	| 'local'
	| 'ely_by'
	| 'mojang'
	| 'opti_fine'
	| 'laby_mod'
	| 'minecraft_capes'
	| 'skin_mc'

/**
 * The cape a name has anywhere that hands them out, as a data URL, or null.
 * See `packages/app-lib/src/api/player_capes.rs` for where it looks.
 */
export async function offline_player_cape(
	username: string,
): Promise<{ source: PlayerCapeSource; texture: string } | null> {
	const cape = await invoke<{ source: PlayerCapeSource; png: number[] } | null>(
		'plugin:noctrinth-offline-auth|offline_player_cape',
		{ username },
	)
	if (!cape) return null
	let binary = ''
	for (const byte of cape.png) binary += String.fromCharCode(byte)
	return { source: cape.source, texture: `data:image/png;base64,${btoa(binary)}` }
}
