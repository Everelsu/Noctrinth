/**
 * Noctrinth's own: CurseForge as a second source on the browse page.
 *
 * Everything here goes through the backend, which talks to CurseForge's
 * public API with the key the build was configured with. A CurseForge
 * project's id always carries the `cf-` prefix on the frontend, so it can never
 * be taken for a Modrinth id by anything that reads it later.
 */

import type { Labrinth } from '@modrinth/api-client'
import { invoke } from '@tauri-apps/api/core'

import { CURSEFORGE_API_KEY } from './curseforge-key'

export const CURSEFORGE_ID_PREFIX = 'cf-'

/** Project types CurseForge has something to browse for. */
export const CURSEFORGE_PROJECT_TYPES = [
	'mod',
	'modpack',
	'resourcepack',
	'shader',
	'datapack',
] as const

export type BrowseSource = 'modrinth' | 'curseforge'

export const curseforgeAvailable = !!CURSEFORGE_API_KEY

export function isCurseforgeId(id: string | null | undefined): boolean {
	return !!id && id.startsWith(CURSEFORGE_ID_PREFIX)
}

/** An installed content entry's CurseForge id, as the browse page writes it. */
export function curseforgeIdOf(numericId: string): string {
	return `${CURSEFORGE_ID_PREFIX}${numericId}`
}

export interface CurseforgeSearchQuery {
	query: string | null
	project_type: string
	game_versions: string[]
	loaders: string[]
	category_ids: number[]
	sort: string
	offset: number
	limit: number
}

export interface CurseforgeCategory {
	id: number
	name: string
	slug: string
	iconUrl: string | null
	parentCategoryId: number | null
}

export type CurseforgeSearchHit = Labrinth.Search.v3.ResultSearchProject & {
	website_url: string | null
}

export interface CurseforgeSearchResults {
	hits: CurseforgeSearchHit[]
	offset: number
	limit: number
	total_hits: number
	hits_per_page: number
}

export interface CurseforgeProject {
	id: string
	name: string
	slug: string | null
	summary: string
	project_type: string
	icon_url: string | null
	website_url: string | null
	downloads: number
	follows: number
	date_created: string | null
	date_modified: string | null
	authors: { name: string; url: string | null }[]
	categories: { name: string; slug: string; icon_url: string | null }[]
	gallery: {
		url: string
		thumbnail_url: string | null
		title: string | null
		description: string | null
	}[]
	loaders: string[]
	game_versions: string[]
}

export interface CurseforgeFile {
	id: string
	project_id: string
	name: string
	file_name: string
	version_type: 'release' | 'beta' | 'alpha'
	game_versions: string[]
	loaders: string[]
	date_published: string | null
	downloads: number
	size: number | null
	launcher_download: boolean
}

export interface CurseforgePendingFile {
	project_id: string
	file_id: string
	file_name: string
	project_name: string | null
	icon_url: string | null
	download_page_url: string | null
}

export interface CurseforgeInstallOutcome {
	installed: string[]
	pending: CurseforgePendingFile[]
}

const plugin = (command: string) => `plugin:noctrinth-curseforge|${command}`

export async function curseforgeSearch(query: CurseforgeSearchQuery) {
	return await invoke<CurseforgeSearchResults>(plugin('curseforge_search'), { query })
}

export async function curseforgeGetCategories(projectType: string) {
	return await invoke<CurseforgeCategory[]>(plugin('curseforge_get_categories'), { projectType })
}

export async function curseforgeGetProject(id: string) {
	return await invoke<CurseforgeProject>(plugin('curseforge_get_project'), { id })
}

/**
 * Query options shared by the browse page and a project's page, so a project
 * hovered or clicked on the browse page is already loaded when its page
 * opens. The backend caches CurseForge's answers too; these keep the frontend
 * from even asking.
 */
const CURSEFORGE_STALE_TIME = 5 * 60 * 1000

export const curseforgeKeys = {
	/** A search hit, kept so a project's page can draw its header at once. */
	summary: (id: string) => ['curseforge', 'summary', id] as const,
	project: (id: string) => ['curseforge', 'project', id] as const,
	description: (id: string) => ['curseforge', 'description', id] as const,
	files: (id: string) => ['curseforge', 'files', id] as const,
	categories: (projectType: string) => ['curseforge', 'categories', projectType] as const,
}

export const curseforgeProjectQueryOptions = (id: string) => ({
	queryKey: curseforgeKeys.project(id),
	queryFn: () => curseforgeGetProject(id),
	staleTime: CURSEFORGE_STALE_TIME,
})

export const curseforgeDescriptionQueryOptions = (id: string) => ({
	queryKey: curseforgeKeys.description(id),
	queryFn: () => curseforgeGetDescription(id),
	staleTime: CURSEFORGE_STALE_TIME,
})

export const curseforgeFilesQueryOptions = (id: string) => ({
	queryKey: curseforgeKeys.files(id),
	queryFn: () => curseforgeGetFiles(id),
	staleTime: CURSEFORGE_STALE_TIME,
})

export const curseforgeCategoriesQueryOptions = (projectType: string) => ({
	queryKey: curseforgeKeys.categories(projectType),
	queryFn: () => curseforgeGetCategories(projectType),
	staleTime: 60 * 60 * 1000,
})

export async function curseforgeGetDescription(id: string) {
	return await invoke<string>(plugin('curseforge_get_description'), { id })
}

export async function curseforgeGetFiles(
	id: string,
	gameVersion?: string | null,
	loader?: string | null,
) {
	return await invoke<CurseforgeFile[]>(plugin('curseforge_get_files'), {
		id,
		gameVersion: gameVersion ?? null,
		loader: loader ?? null,
	})
}

export async function curseforgeInstallProject(
	instanceId: string,
	projectId: string,
	fileId?: string | null,
) {
	return await invoke<CurseforgeInstallOutcome>(plugin('curseforge_install_project'), {
		instanceId,
		projectId,
		fileId: fileId ?? null,
	})
}

/** Downloads a modpack's zip; returns where it was saved. */
export async function curseforgeDownloadModpack(projectId: string, fileId?: string | null) {
	return await invoke<string>(plugin('curseforge_download_modpack'), {
		projectId,
		fileId: fileId ?? null,
	})
}

export async function curseforgePendingFiles(instanceId: string) {
	return await invoke<CurseforgePendingFile[]>(plugin('curseforge_pending_files'), {
		instanceId,
	})
}

/** Installs whatever of the pending files has arrived in Downloads. */
export async function curseforgeCheckPendingDownloads(instanceId: string) {
	return await invoke<CurseforgePendingFile[]>(plugin('curseforge_check_pending_downloads'), {
		instanceId,
	})
}

export async function curseforgeDismissPendingFile(instanceId: string, fileId: string) {
	await invoke(plugin('curseforge_dismiss_pending_file'), { instanceId, fileId })
}
