/**
 * A picture behind the launcher, after Celestial Launcher's
 * (github.com/Wemsur/Celestial-Launcher) — or an animated GIF, or a looping
 * video. It is a file the app keeps next to its database; how it is blurred and
 * darkened are settings.
 */
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { ref } from 'vue'

import { get as getSettings, set as setSettings } from '@/helpers/settings.ts'

/** What can be a background; also what a dropped file is checked against. */
export const VIDEO_EXTENSIONS = ['mp4', 'm4v', 'webm']
export const BACKGROUND_EXTENSIONS = [
	'png',
	'jpg',
	'jpeg',
	'webp',
	'gif',
	'avif',
	...VIDEO_EXTENSIONS,
]

/** An object URL for the picture, or null when there is none. */
export const backgroundUrl = ref<string | null>(null)
/** Whether that is a video, which is played rather than painted. */
export const backgroundIsVideo = ref(false)

/**
 * The video type a file's first bytes say it is, or null for a picture.
 *
 * The file is kept without its extension, so this is how a video is told from
 * a picture again; a `<video>` also wants the type its blob was made with.
 */
export function videoTypeOf(bytes: Uint8Array): string | null {
	const ascii = (from: number, to: number) => String.fromCharCode(...bytes.subarray(from, to))
	// AVIF is an ISO media file too; its brand, `avif` or `avis`, says picture.
	if (ascii(4, 8) === 'ftyp' && !ascii(8, 12).startsWith('avi')) return 'video/mp4'
	if (bytes[0] === 0x1a && bytes[1] === 0x45 && bytes[2] === 0xdf && bytes[3] === 0xa3) {
		return 'video/webm'
	}
	return null
}
export const backgroundBlur = ref(true)
/** From 0, the picture as it is, to 100. */
export const backgroundDim = ref(0)

export async function loadBackground(blur: boolean, dim = backgroundDim.value): Promise<void> {
	backgroundBlur.value = blur
	backgroundDim.value = dim
	const bytes = await invoke<ArrayBuffer>('plugin:noctrinth-background|background_get')
	if (backgroundUrl.value) URL.revokeObjectURL(backgroundUrl.value)
	const type = videoTypeOf(new Uint8Array(bytes, 0, Math.min(16, bytes.byteLength)))
	backgroundIsVideo.value = type !== null
	backgroundUrl.value = bytes.byteLength
		? URL.createObjectURL(new Blob([bytes], type ? { type } : undefined))
		: null
}

export function isBackgroundFile(path: string): boolean {
	const extension = path.split('.').pop()?.toLowerCase() ?? ''
	return BACKGROUND_EXTENSIONS.includes(extension)
}

/** Makes the picture at `path` the background. */
export async function setBackgroundFromPath(path: string): Promise<void> {
	await invoke('plugin:noctrinth-background|background_set', { path })
	await loadBackground(backgroundBlur.value)
}

/** Asks for a picture and makes it the background. False when none was picked. */
export async function chooseBackground(): Promise<boolean> {
	const path = await open({
		multiple: false,
		filters: [{ name: 'Image or video', extensions: BACKGROUND_EXTENSIONS }],
	})
	if (typeof path !== 'string') return false
	await setBackgroundFromPath(path)
	return true
}

export async function clearBackground(): Promise<void> {
	await invoke('plugin:noctrinth-background|background_clear')
	await loadBackground(backgroundBlur.value)
}

export async function saveBackgroundBlur(blur: boolean): Promise<void> {
	backgroundBlur.value = blur
	await setSettings({ ...(await getSettings()), background_blur: blur })
}

export async function saveBackgroundDim(dim: number): Promise<void> {
	await setSettings({ ...(await getSettings()), background_dim: dim })
}
