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
