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
	extra_data?: Record<string, unknown>
	grouped_notifs?: PlatformNotification[]
}

interface CacheEntity {
	id?: string
	project_id?: string
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

	// All four bulk fetches launch in parallel — the previous implementation
	// awaited versions first to discover extra project IDs, which serialised
	// the slowest call with the second-slowest one (~2× the latency). The few
	// version-derived projects not already in `bulk.projects` are fetched in
	// a fast follow-up round-trip below.
	const [versions, projects, users, organizations] = await Promise.all([
		safe<CacheEntity>(() =>
			bulk.versions.size > 0 ? get_version_many([...bulk.versions]) : Promise.resolve([]),
		),
		safe<CacheEntity>(() =>
			bulk.projects.size > 0 ? get_project_many([...bulk.projects]) : Promise.resolve([]),
		),
		safe<CacheEntity>(() =>
			bulk.users.size > 0 ? get_user_many([...bulk.users]) : Promise.resolve([]),
		),
		safe<CacheEntity>(() =>
			bulk.organizations.size > 0
				? get_organization_many([...bulk.organizations])
				: Promise.resolve([]),
		),
	])

	// Pick up any project ids referenced only via a version (and not already
	// fetched). In practice this set is usually empty.
	const fetchedProjectIds = new Set(projects.map((p) => p?.id).filter(Boolean))
	const extraIds: string[] = []
	for (const v of versions) {
		if (v?.project_id && !fetchedProjectIds.has(v.project_id)) extraIds.push(v.project_id)
	}
	const allProjects =
		extraIds.length > 0
			? [...projects, ...(await safe<CacheEntity>(() => get_project_many([...new Set(extraIds)])))]
			: projects

	// Index for O(1) lookup instead of repeated .find() per notification.
	const projectMap = new Map<string, CacheEntity>(
		allProjects.filter((p) => p?.id).map((p) => [p.id as string, p]),
	)
	const versionMap = new Map<string, CacheEntity>(
		versions.filter((v) => v?.id).map((v) => [v.id as string, v]),
	)
	const userMap = new Map<string, CacheEntity>(
		users.filter((u) => u?.id).map((u) => [u.id as string, u]),
	)
	const orgMap = new Map<string, CacheEntity>(
		organizations.filter((o) => o?.id).map((o) => [o.id as string, o]),
	)

	for (const n of notifications) {
		n.extra_data = {}
		if (!n.body) continue
		if (n.body.project_id) n.extra_data.project = projectMap.get(n.body.project_id)
		if (n.body.organization_id) n.extra_data.organization = orgMap.get(n.body.organization_id)
		if (n.body.invited_by) n.extra_data.invited_by = userMap.get(n.body.invited_by)
		if (n.body.version_id) {
			const v = versionMap.get(n.body.version_id)
			n.extra_data.version = v
			if (!n.extra_data.project && v?.project_id) {
				n.extra_data.project = projectMap.get(v.project_id)
			}
		}
	}

	return notifications
}

function isSimilar(a: PlatformNotification, b: PlatformNotification | undefined): boolean {
	return !!a?.body?.project_id && a.body!.project_id === b?.body?.project_id
}

export function groupNotifications(notifications: PlatformNotification[]): PlatformNotification[] {
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

// ── Notification cache ───────────────────────────────────────────────────────
//
// `fetchExtraNotificationData` makes 3–4 bulk requests (projects, versions,
// users, organizations) to enrich the raw notification list — fast on a warm
// connection but visibly slow on first load. We persist the enriched result
// per-user in localStorage so the next visit (even after an app restart) is
// instant; a background refresh fixes anything stale. Callers can mutate the
// cache (`patchCachedNotifications`) so optimistic UI changes survive reloads.
//
// localStorage (not sessionStorage) because Tauri spins up a fresh webview on
// each launch — sessionStorage would be empty every time the user reopens
// the app, defeating the cache.

const CACHE_PREFIX = 'noctrinth:notifications:'
const CACHE_TTL_MS = 60 * 60 * 1000 // 1 hour — stale-while-revalidate covers drift

interface CachedNotifications {
	at: number
	data: PlatformNotification[]
}

function cacheKey(userId: string): string {
	return CACHE_PREFIX + userId
}

/** Read the cached notification list for a user, or null if stale/missing. */
export function readNotificationsCache(userId: string): PlatformNotification[] | null {
	try {
		const raw = localStorage.getItem(cacheKey(userId))
		if (!raw) return null
		const parsed = JSON.parse(raw) as CachedNotifications
		if (!parsed || typeof parsed.at !== 'number') return null
		if (Date.now() - parsed.at > CACHE_TTL_MS) return null
		return parsed.data
	} catch {
		return null
	}
}

/** Persist the enriched notification list. */
export function writeNotificationsCache(userId: string, data: PlatformNotification[]): void {
	try {
		const payload: CachedNotifications = { at: Date.now(), data }
		localStorage.setItem(cacheKey(userId), JSON.stringify(payload))
	} catch {
		// Quota exceeded or localStorage unavailable — silently skip.
	}
}

/** Drop the cache (e.g. after a destructive action you don't want to mirror). */
export function clearNotificationsCache(userId: string): void {
	try {
		localStorage.removeItem(cacheKey(userId))
	} catch {
		// ignore
	}
}

/** In-place update the cached entries marked by the given ids as read. */
export function patchCachedNotifications(
	userId: string,
	patch: (n: PlatformNotification) => PlatformNotification | null,
): void {
	const data = readNotificationsCache(userId)
	if (!data) return
	const next = data.map((n) => patch(n)).filter((n): n is PlatformNotification => n !== null)
	writeNotificationsCache(userId, next)
}
