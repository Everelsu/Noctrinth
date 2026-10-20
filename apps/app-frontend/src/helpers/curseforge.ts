/**
 * Noctrinth's own: CurseForge as a second source on the browse page.
 *
 * Everything here goes through the backend, which talks to CurseForge's
 * public API with the user's own key, or else the one the build was
 * configured with. A CurseForge
 * project's id always carries the `cf-` prefix on the frontend, so it can never
 * be taken for a Modrinth id by anything that reads it later.
 */

import type { Labrinth } from '@modrinth/api-client'
import { invoke } from '@tauri-apps/api/core'
import { ref } from 'vue'

import { curseforgeKeyAvailable, curseforgeSettings } from './curseforge-settings'

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

/** Whether CurseForge is offered: there is a key and it is not turned off. */
export function curseforgeEnabled(): boolean {
	return curseforgeKeyAvailable.value && curseforgeSettings.enabled
}

export function isCurseforgeId(id: string | null | undefined): boolean {
	return !!id && id.startsWith(CURSEFORGE_ID_PREFIX)
}

/**
 * Whether an instance's content row is a CurseForge project: installed from
 * CurseForge, part of a CurseForge pack, or recognised as one by its
 * fingerprint (whose entry is only marked so from the next listing on).
 */
export function isCurseforgeContent(item: {
	source_kind?: string | null
	project?: { external_url?: string | null } | null
}): boolean {
	return (
		item.source_kind === 'curseforge' ||
		item.source_kind === 'curseforge_modpack' ||
		!!item.project?.external_url?.startsWith('https://www.curseforge.com/')
	)
}

type ContentRow = Parameters<typeof isCurseforgeContent>[0] & {
	project?: { id: string } | null
	version?: { id: string } | null
}

/**
 * The CurseForge projects among content rows, as (`cf-` id, file id) pairs.
 * A pack installed from a CurseForge zip records its files only by
 * fingerprint, so its listing is the one place that knows what they are.
 */
export function curseforgeFilesIn(rows: ContentRow[]): [string, string | null][] {
	return rows.flatMap((row) =>
		row.project?.id && isCurseforgeContent(row)
			? [[curseforgeIdOf(row.project.id), row.version?.id ?? null] as [string, string | null]]
			: [],
	)
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
	wiki_url: string | null
	issues_url: string | null
	source_url: string | null
	authors: { id: string | null; name: string; url: string | null; avatar_url: string | null }[]
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

/** One file with what its own page shows besides the versions list. */
export interface CurseforgeVersion extends CurseforgeFile {
	/** HTML, as CurseForge stores it. */
	changelog: string
	dependencies: {
		project_id: string
		dependency_type: 'required' | 'optional' | 'incompatible' | 'embedded'
		name: string | null
		slug: string | null
		icon_url: string | null
		project_type: string
	}[]
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
	missing_dependencies: { project_id: string; name: string | null; website_url: string | null }[]
}

/** The bare numeric id a CurseForge content entry stores. */
export function curseforgeNumericId(id: string): string {
	return id.startsWith(CURSEFORGE_ID_PREFIX) ? id.slice(CURSEFORGE_ID_PREFIX.length) : id
}

/**
 * A CurseForge project and its files in the shapes the shared install flow
 * reads a Modrinth project and its versions in, so the same instance picker,
 * compatibility checks and incompatible-version warning serve both. Only the
 * fields that flow reads are filled.
 */
export function curseforgeForInstallFlow(project: CurseforgeProject, files: CurseforgeFile[]) {
	// What the shared compatibility check matches an instance's loader
	// against. A mod file that names no loader is a Forge one, as CurseForge's
	// own app takes it; anything that is not a mod runs on any loader, which
	// the shared flow spells "minecraft".
	const loadersOf = (file: CurseforgeFile) =>
		file.loaders.length > 0
			? file.loaders
			: project.project_type === 'mod'
				? ['forge']
				: ['minecraft']
	const versions = files
		.slice()
		.sort((a, b) => (b.date_published ?? '').localeCompare(a.date_published ?? ''))
		.map((file) => ({
			id: file.id,
			project_id: project.id,
			name: file.name,
			version_number: file.name,
			version_type: file.version_type,
			game_versions: file.game_versions,
			loaders: loadersOf(file),
			date_published: file.date_published ?? '',
			downloads: file.downloads,
			changelog: '',
			dependencies: [],
			featured: false,
			status: 'listed',
			files: [
				{
					filename: file.file_name,
					size: file.size ?? 0,
					primary: true,
					url: '',
					hashes: {},
				},
			],
		})) as unknown as Labrinth.Versions.v2.Version[]

	return {
		project: {
			id: project.id,
			slug: project.slug,
			title: project.name,
			description: project.summary,
			icon_url: project.icon_url,
			raw_icon_url: project.icon_url,
			project_type: project.project_type,
			organization: null,
			team: undefined,
			versions: versions.map((version) => version.id),
			game_versions: project.game_versions,
			loaders: project.loaders,
		} as unknown as Labrinth.Projects.v2.Project,
		versions,
	}
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
	version: (id: string, fileId: string) => ['curseforge', 'version', id, fileId] as const,
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

export const curseforgeVersionQueryOptions = (id: string, fileId: string) => ({
	queryKey: curseforgeKeys.version(id, fileId),
	queryFn: () => curseforgeGetVersion(id, fileId),
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

export async function curseforgeGetVersion(id: string, fileId: string) {
	return await invoke<CurseforgeVersion>(plugin('curseforge_get_version'), { id, fileId })
}

/** A file's page on CurseForge's site. */
export function curseforgeFileUrl(websiteUrl: string, fileId: string): string {
	return `${websiteUrl.replace(/\/$/, '')}/files/${fileId}`
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

/**
 * CurseForge projects starred from their page. Kept on this computer: there
 * is no CurseForge account to keep them in, as CurseForge's own app has.
 */
const FAVORITES_KEY = 'noctrinth.curseforge.favorites'
const favorites = ref<Set<string>>(readFavorites())

function readFavorites(): Set<string> {
	try {
		const stored = JSON.parse(localStorage.getItem(FAVORITES_KEY) ?? '[]')
		return new Set(Array.isArray(stored) ? stored.filter((id) => typeof id === 'string') : [])
	} catch {
		return new Set()
	}
}

export function useCurseforgeFavorites() {
	return {
		isFavorite: (id: string) => favorites.value.has(id),
		toggleFavorite(id: string) {
			const next = new Set(favorites.value)
			if (!next.delete(id)) next.add(id)
			favorites.value = next
			try {
				localStorage.setItem(FAVORITES_KEY, JSON.stringify([...next]))
			} catch {
				// Not remembered past this session; nothing else depends on it.
			}
		},
	}
}
