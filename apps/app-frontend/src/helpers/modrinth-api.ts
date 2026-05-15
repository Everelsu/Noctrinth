/**
 * Authenticated requests to the Modrinth REST API.
 * Uses the OAuth session token from the logged-in user.
 */
import { fetch as tauriFetch } from '@tauri-apps/plugin-http'

import { get as getCreds } from '@/helpers/mr_auth.ts'

const API_BASE = 'https://api.modrinth.com'

async function authHeader(): Promise<string> {
	const creds = await getCreds()
	if (!creds) throw new Error('Please sign in to Modrinth first.')
	return creds.session
}

function friendlyError(status: number, action: string, body?: string): string {
	const detail = body ? extractApiMessage(body) : ''
	const suffix = detail ? ` — ${detail}` : ''
	switch (status) {
		case 400:
			return `Bad request${suffix || ` while trying to ${action}.`}`
		case 401:
			return `You need to sign in again to ${action}.${suffix}`
		case 403:
			return `You don't have permission to ${action}.${suffix}`
		case 404:
			return `Couldn't find what you're looking for to ${action}.${suffix}`
		case 429:
			return `Too many requests, please wait a moment before trying again.`
		case 502:
		case 503:
		case 504:
			return `Modrinth's servers are temporarily unavailable. Try again later.`
		default:
			return `Failed to ${action} (HTTP ${status}).${suffix}`
	}
}

function extractApiMessage(body: string): string {
	try {
		const parsed = JSON.parse(body)
		return parsed?.description || parsed?.message || parsed?.error || ''
	} catch {
		return body.length > 120 ? `${body.slice(0, 117)}...` : body
	}
}

async function readBody(res: { text: () => Promise<string> }): Promise<string> {
	try {
		return await res.text()
	} catch {
		return ''
	}
}

export async function followProject(projectId: string): Promise<void> {
	const auth = await authHeader()
	const res = await tauriFetch(`${API_BASE}/v3/project/${projectId}/follow`, {
		method: 'POST',
		headers: { Authorization: auth },
	})
	if (!res.ok && res.status !== 204) {
		throw new Error(friendlyError(res.status, 'follow this project'))
	}
}

export async function unfollowProject(projectId: string): Promise<void> {
	const auth = await authHeader()
	const res = await tauriFetch(`${API_BASE}/v3/project/${projectId}/follow`, {
		method: 'DELETE',
		headers: { Authorization: auth },
	})
	if (!res.ok && res.status !== 204) {
		throw new Error(friendlyError(res.status, 'unfollow this project'))
	}
}

export async function getUserFollowedProjects(userId: string): Promise<{ id: string }[]> {
	const auth = await authHeader()
	const res = await tauriFetch(`${API_BASE}/v3/user/${userId}/follows`, {
		method: 'GET',
		headers: { Authorization: auth },
	})
	if (!res.ok) return []
	return await res.json()
}

export async function isFollowingProject(userId: string, projectId: string): Promise<boolean> {
	try {
		const follows = await getUserFollowedProjects(userId)
		return follows.some((p) => p.id === projectId)
	} catch {
		return false
	}
}

export interface Collection {
	id: string
	user: string
	name: string
	description: string | null
	icon_url: string | null
	color: number | null
	status: string
	created: string
	updated: string
	projects: string[]
}

export async function getUserCollections(userId: string): Promise<Collection[]> {
	const auth = await authHeader()
	const res = await tauriFetch(`${API_BASE}/v3/user/${userId}/collections`, {
		method: 'GET',
		headers: { Authorization: auth },
	})
	if (!res.ok) {
		const body = await readBody(res)
		throw new Error(friendlyError(res.status, 'load collections', body))
	}
	return await res.json()
}

export async function getCollection(id: string): Promise<Collection> {
	const auth = await authHeader()
	const res = await tauriFetch(`${API_BASE}/v3/collection/${encodeURIComponent(id)}`, {
		method: 'GET',
		headers: { Authorization: auth },
	})
	if (!res.ok) {
		const body = await readBody(res)
		throw new Error(friendlyError(res.status, 'load collection', body))
	}
	return await res.json()
}

export interface CollectionCreate {
	name: string
	description?: string
	projects?: string[]
}

export async function createCollection(data: CollectionCreate): Promise<Collection> {
	const auth = await authHeader()
	const body: Record<string, unknown> = {
		name: data.name,
		projects: data.projects ?? [],
	}
	if (data.description) body.description = data.description

	const res = await tauriFetch(`${API_BASE}/v3/collection`, {
		method: 'POST',
		headers: { Authorization: auth, 'Content-Type': 'application/json' },
		body: JSON.stringify(body),
	})
	if (!res.ok) {
		const body = await readBody(res)
		throw new Error(friendlyError(res.status, 'create collection', body))
	}
	return await res.json()
}

export async function addProjectToCollection(
	collectionId: string,
	projectId: string,
	currentProjects: string[],
): Promise<void> {
	if (currentProjects.includes(projectId)) return
	await editCollection(collectionId, { new_projects: [...currentProjects, projectId] })
}

export interface CollectionEdit {
	name?: string
	description?: string | null
	status?: 'listed' | 'unlisted' | 'private'
	new_projects?: string[]
}

export async function editCollection(id: string, data: CollectionEdit): Promise<void> {
	const auth = await authHeader()
	const res = await tauriFetch(`${API_BASE}/v3/collection/${encodeURIComponent(id)}`, {
		method: 'PATCH',
		headers: { Authorization: auth, 'Content-Type': 'application/json' },
		body: JSON.stringify(data),
	})
	if (!res.ok && res.status !== 204) {
		const body = await readBody(res)
		throw new Error(friendlyError(res.status, 'save collection changes', body))
	}
}

export async function deleteCollection(id: string): Promise<void> {
	const auth = await authHeader()
	const res = await tauriFetch(`${API_BASE}/v3/collection/${encodeURIComponent(id)}`, {
		method: 'DELETE',
		headers: { Authorization: auth },
	})
	if (!res.ok && res.status !== 204) {
		const body = await readBody(res)
		throw new Error(friendlyError(res.status, 'delete collection', body))
	}
}

export async function removeProjectFromCollection(
	collectionId: string,
	projectId: string,
	currentProjects: string[],
): Promise<void> {
	const next = currentProjects.filter((p) => p !== projectId)
	await editCollection(collectionId, { new_projects: next })
}

export interface Notification {
	id: string
	user_id: string
	read: boolean
	created: string
	type?: string | null
	title?: string
	text?: string
	link?: string
	body?: unknown
}

export async function getUserNotifications(userId: string): Promise<Notification[]> {
	const auth = await authHeader()
	const res = await tauriFetch(`${API_BASE}/v3/user/${userId}/notifications`, {
		method: 'GET',
		headers: { Authorization: auth },
	})
	if (!res.ok) {
		throw new Error(friendlyError(res.status, 'load notifications'))
	}
	return await res.json()
}

export async function markNotificationRead(notificationId: string): Promise<void> {
	const auth = await authHeader()
	const res = await tauriFetch(`${API_BASE}/v3/notification/${notificationId}`, {
		method: 'PATCH',
		headers: { Authorization: auth },
	})
	if (!res.ok && res.status !== 204) {
		throw new Error(friendlyError(res.status, 'mark notification as read'))
	}
}

export async function deleteNotification(notificationId: string): Promise<void> {
	const auth = await authHeader()
	const res = await tauriFetch(`${API_BASE}/v3/notification/${notificationId}`, {
		method: 'DELETE',
		headers: { Authorization: auth },
	})
	if (!res.ok && res.status !== 204) {
		throw new Error(friendlyError(res.status, 'delete notification'))
	}
}
