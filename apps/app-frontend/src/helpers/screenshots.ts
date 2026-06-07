/**
 * Screenshot helpers — lists, copies and deletes Minecraft screenshots from a
 * single instance's `screenshots/` folder.
 *
 * All filesystem access goes through direct `invoke` calls to the already-
 * registered `plugin:fs` Tauri plugin, following the same pattern as
 * `profile.ts`, `worlds.ts`, etc. This deliberately avoids importing
 * `@tauri-apps/plugin-fs` as a JS module so Vite never has to discover or
 * re-optimise it as a new dependency — which would force a page reload that
 * races the Tauri webview boot and crashes the app.
 */
import { invoke } from '@tauri-apps/api/core'

import { get_full_path } from '@/helpers/profile'

export interface Screenshot {
	/** Stable unique key — the absolute file path. */
	id: string
	fileName: string
	/** Absolute filesystem path to the screenshot. */
	path: string
	/** Last-modified epoch ms — used for sorting. */
	modified: number
	/** File size in bytes. */
	size: number
}

interface FsDirEntry {
	name: string
	isFile: boolean
	isDir: boolean
	isSymlink: boolean
}

interface FsStat {
	isFile: boolean
	isDir: boolean
	isSymlink: boolean
	size: number
	readonly?: boolean
	mtime?: number
	atime?: number
	birthtime?: number
}

async function fsExists(path: string): Promise<boolean> {
	try {
		await invoke('plugin:fs|stat', { path })
		return true
	} catch {
		return false
	}
}

async function fsReadDir(path: string): Promise<FsDirEntry[]> {
	return invoke<FsDirEntry[]>('plugin:fs|read_dir', { path })
}

async function fsStat(path: string): Promise<FsStat> {
	return invoke<FsStat>('plugin:fs|stat', { path })
}

async function fsReadFile(path: string): Promise<number[]> {
	return invoke<number[]>('plugin:fs|read_file', { path })
}

async function fsRemove(path: string): Promise<void> {
	return invoke('plugin:fs|remove', { path, options: {} })
}

function isScreenshotFile(name: string): boolean {
	return name.toLowerCase().endsWith('.png')
}

/**
 * Lists every screenshot in the given instance, newest first. Returns an empty
 * array if the instance has no `screenshots/` folder (or it can't be read).
 */
export async function listInstanceScreenshots(instancePath: string): Promise<Screenshot[]> {
	const fullPath = await get_full_path(instancePath)
	const screenshotsDir = `${fullPath}/screenshots`

	if (!(await fsExists(screenshotsDir))) {
		return []
	}

	let entries: FsDirEntry[] = []
	try {
		entries = await fsReadDir(screenshotsDir)
	} catch {
		return []
	}

	const files = entries.filter((e) => e.isFile && isScreenshotFile(e.name))

	const screenshots = await Promise.all(
		files.map(async (entry) => {
			const path = `${screenshotsDir}/${entry.name}`
			let modified = 0
			let size = 0
			try {
				const info = await fsStat(path)
				modified = info.mtime ?? info.birthtime ?? 0
				size = info.size
			} catch {
				modified = 0
			}

			return {
				id: path,
				fileName: entry.name,
				path,
				modified,
				size,
			} satisfies Screenshot
		}),
	)

	return screenshots.sort((a, b) => b.modified - a.modified)
}

export async function deleteScreenshot(path: string): Promise<void> {
	await fsRemove(path)
}

/**
 * Reads the PNG bytes and writes the image to the clipboard via the Web
 * Clipboard API (`image/png` ClipboardItem).
 */
export async function copyScreenshotToClipboard(path: string): Promise<void> {
	const bytes = await fsReadFile(path)
	const blob = new Blob([new Uint8Array(bytes)], { type: 'image/png' })
	await navigator.clipboard.write([new ClipboardItem({ 'image/png': blob })])
}
