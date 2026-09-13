import { invoke } from '@tauri-apps/api/core'

export type ElyCredentials = {
	profile: {
		id: string
		name: string
		skins: unknown[]
		capes: unknown[]
	}
	access_token: string
	active: boolean
	auth_provider: 'ely_by'
}

export async function ely_login(username: string, password: string): Promise<ElyCredentials> {
	return await invoke('plugin:ely-auth|ely_login', { username, password })
}

/**
 * Signs in on Ely.by's own page, in a window of its own.
 *
 * Resolves with the account once it is signed in and saved, or with `null` if
 * the player closed the window — that is a decision, not a failure, and there
 * is nothing to apologise for. It rejects only when Ely.by refused or something
 * genuinely went wrong. Nothing is passed in because nothing about the sign-in
 * belongs to this side any more.
 */
export async function ely_oauth_login(): Promise<ElyCredentials | null> {
	return await invoke('plugin:ely-auth|ely_oauth_login')
}

export async function ely_logout(user: string): Promise<void> {
	await invoke('plugin:ely-auth|ely_logout', { user })
}

export async function ely_get_users(): Promise<ElyCredentials[]> {
	return await invoke('plugin:ely-auth|ely_get_users')
}

export async function ely_get_default_user(): Promise<string | null> {
	return await invoke('plugin:ely-auth|ely_get_default_user')
}

export async function ely_set_default_user(user: string): Promise<void> {
	await invoke('plugin:ely-auth|ely_set_default_user', { user })
}

/**
 * Opens (or focuses) the embedded Ely.by skin-management window. Listen for
 * the `ely-skin-window-closed` Tauri event to refresh skin previews after the
 * user closes it.
 */
export async function ely_open_skin_window(): Promise<void> {
	await invoke('plugin:ely-auth|ely_open_skin_window')
}
