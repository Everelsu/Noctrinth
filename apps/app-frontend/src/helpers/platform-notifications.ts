/**
 * Port of `apps/frontend/src/helpers/platform-notifications.ts` for the launcher.
 * Uses tauriFetch + cache helpers instead of the Modrinth API client.
 */
import { fetch as tauriFetch } from '@tauri-apps/plugin-http'

import {
	get_organization_many,
	get_project_many,
	get_user_many,
	get_version_many,
} from '@/helpers/cache.js'
import { get as getCreds } from '@/helpers/mr_auth.ts'

const API_BASE = 'https://api.modrinth.com'

export interface RawNotification {
	id: string
	user_id: string
	read: boolean
	created: string
	link?: string
	title?: string
	text?: string
	type?: string | null
	actions?: { title: string; action_route: [string, string] }[]
	body?: {
		type?: string
		project_id?: string
		version_id?: string
		report_id?: string
		thread_id?: string
		invited_by?: string
		organization_id?: string
		team_id?: string
		message_id?: string
		new_status?: string
		old_status?: string
	}
}

export interface PlatformNotification extends RawNotification {
	extra_data?: {
		project?: any
		organization?: any
		user?: any
		version?: any
		thread?: any
		report?: any
		invited_by?: any
	}
	grouped_notifs?: PlatformNotification[]
}

async function safe<T>(fn: () => Promise<T[] | null | undefined>): Promise<T[]> {
	try {
		const res = await fn()
		return Array.isArray(res) ? res : []
	} catch {
		return []
	}
}

export async function fetchExtraNotificationData(
	notifications: PlatformNotification[],
): Promise<PlatformNotification[]> {
	const bulk = {
		projects: new Set<string>(),
		versions: new Set<string>(),
		users: new Set<string>(),
		organizations: new Set<string>(),
	}

	for (const n of notifications) {
		if (n.body) {
			if (n.body.project_id) bulk.projects.add(n.body.project_id)
			if (n.body.version_id) bulk.versions.add(n.body.version_id)
			if (n.body.invited_by) bulk.users.add(n.body.invited_by)
			if (n.body.organization_id) bulk.organizations.add(n.body.organization_id)
		}
	}

	const versions = await safe<any>(() =>
		bulk.versions.size > 0 ? get_version_many([...bulk.versions]) : Promise.resolve([]),
	)
	for (const v of versions) {
		if (v?.project_id) bulk.projects.add(v.project_id)
	}

	const [projects, users, organizations] = await Promise.all([
		safe<any>(() =>
			bulk.projects.size > 0 ? get_project_many([...bulk.projects]) : Promise.resolve([]),
		),
		safe<any>(() =>
			bulk.users.size > 0 ? get_user_many([...bulk.users]) : Promise.resolve([]),
		),
		safe<any>(() =>
			bulk.organizations.size > 0
				? get_organization_many([...bulk.organizations])
				: Promise.resolve([]),
		),
	])

	for (const n of notifications) {
		n.extra_data = {}
		if (n.body) {
			if (n.body.project_id) {
				n.extra_data.project = projects.find((x) => x?.id === n.body!.project_id)
			}
			if (n.body.organization_id) {
				n.extra_data.organization = organizations.find(
					(x) => x?.id === n.body!.organization_id,
				)
			}
			if (n.body.invited_by) {
				n.extra_data.invited_by = users.find((x) => x?.id === n.body!.invited_by)
			}
			if (n.body.version_id) {
				n.extra_data.version = versions.find((x) => x?.id === n.body!.version_id)
				if (!n.extra_data.project && n.extra_data.version?.project_id) {
					n.extra_data.project = projects.find(
						(x) => x?.id === n.extra_data!.version.project_id,
					)
				}
			}
		}
	}

	return notifications
}

function isSimilar(a: PlatformNotification, b: PlatformNotification | undefined): boolean {
	return !!a?.body?.project_id && a.body!.project_id === b?.body?.project_id
}

export function groupNotifications(
	notifications: PlatformNotification[],
): PlatformNotification[] {
	const grouped: PlatformNotification[] = []
	for (let i = 0; i < notifications.length; i++) {
		const current = notifications[i]
		const next = notifications[i + 1]
		if (current.body && i < notifications.length - 1 && isSimilar(current, next)) {
			const groupedNotif: PlatformNotification = { ...current, grouped_notifs: [next] }
			let j = i + 2
			while (j < notifications.length && isSimilar(current, notifications[j])) {
				groupedNotif.grouped_notifs!.push(notifications[j])
				j++
			}
			grouped.push(groupedNotif)
			i = j - 1
		} else {
			grouped.push(current)
		}
	}
	return grouped
}

async function authHeader(): Promise<string> {
	const creds = await getCreds()
	if (!creds) throw new Error('Please sign in to Modrinth first.')
	return creds.session
}

export async function markIdsAsRead(ids: string[]): Promise<void> {
	if (ids.length === 0) return
	const auth = await authHeader()
	const params = new URLSearchParams()
	params.set('ids', JSON.stringify(ids))
	const res = await tauriFetch(`${API_BASE}/v3/notifications?${params.toString()}`, {
		method: 'PATCH',
		headers: { Authorization: auth },
	})
	if (!res.ok && res.status !== 204) {
		throw new Error(`Failed to mark notifications as read (HTTP ${res.status})`)
	}
}

export async function deleteIds(ids: string[]): Promise<void> {
	if (ids.length === 0) return
	const auth = await authHeader()
	const params = new URLSearchParams()
	params.set('ids', JSON.stringify(ids))
	const res = await tauriFetch(`${API_BASE}/v3/notifications?${params.toString()}`, {
		method: 'DELETE',
		headers: { Authorization: auth },
	})
	if (!res.ok && res.status !== 204) {
		throw new Error(`Failed to delete notifications (HTTP ${res.status})`)
	}
}

export async function acceptTeamInvite(teamId: string): Promise<void> {
	const auth = await authHeader()
	const res = await tauriFetch(`${API_BASE}/v3/team/${teamId}/join`, {
		method: 'POST',
		headers: { Authorization: auth },
	})
	if (!res.ok && res.status !== 204) {
		throw new Error(`Failed to accept invite (HTTP ${res.status})`)
	}
}

export async function declineTeamInvite(teamId: string, userId: string): Promise<void> {
	const auth = await authHeader()
	const res = await tauriFetch(`${API_BASE}/v3/team/${teamId}/members/${userId}`, {
		method: 'DELETE',
		headers: { Authorization: auth },
	})
	if (!res.ok && res.status !== 204) {
		throw new Error(`Failed to decline invite (HTTP ${res.status})`)
	}
}
