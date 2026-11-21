/**
 * A picture behind the launcher, after Celestial Launcher's
 * (github.com/Wemsur/Celestial-Launcher) — or an animated GIF, or a looping
 * video. It is a file the app keeps next to its database; how it is blurred and
 * darkened are settings.
 */
import { convertFileSrc, invoke } from '@tauri-apps/api/core'
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

type BackgroundInfo = {
	path: string
	is_video: boolean
	revision: number
}

/** Asset URL for the background, or null when there is none. */
export const backgroundUrl = ref<string | null>(null)
/** Whether that is a video, which is played rather than painted. */
export const backgroundIsVideo = ref(false)
export const backgroundBlur = ref(true)
/** From 0, the picture as it is, to 100. */
export const backgroundDim = ref(0)

export async function loadBackground(blur: boolean, dim = backgroundDim.value): Promise<void> {
	backgroundBlur.value = blur
	backgroundDim.value = dim
	const info = await invoke<BackgroundInfo | null>('plugin:noctrinth-background|background_get')
	backgroundIsVideo.value = info?.is_video ?? false
	backgroundUrl.value = info ? `${convertFileSrc(info.path)}?v=${info.revision}` : null
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
	backgroundUrl.value = null
	backgroundIsVideo.value = false
}

let pendingSettingsWrite: Promise<void> = Promise.resolve()

function patchBackgroundSettings(patch: {
	background_blur?: boolean
	background_dim?: number
}): Promise<void> {
	const next = pendingSettingsWrite.then(async () => {
		const current = await getSettings()
		await setSettings({ ...current, ...patch })
	})
	pendingSettingsWrite = next.catch(() => {})
	return next
}

export async function saveBackgroundBlur(blur: boolean): Promise<void> {
	backgroundBlur.value = blur
	await patchBackgroundSettings({ background_blur: blur })
}

export async function saveBackgroundDim(dim: number): Promise<void> {
	backgroundDim.value = dim
	await patchBackgroundSettings({ background_dim: dim })
}
