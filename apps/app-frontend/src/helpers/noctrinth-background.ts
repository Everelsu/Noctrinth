/**
 * A picture behind the launcher, after Celestial Launcher's
 * (github.com/Wemsur/Celestial-Launcher). The picture is a file the app keeps
 * next to its database; whether it is blurred is a setting.
 */
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { ref } from 'vue'

import { get as getSettings, set as setSettings } from '@/helpers/settings.ts'

/** An object URL for the picture, or null when there is none. */
export const backgroundUrl = ref<string | null>(null)
export const backgroundBlur = ref(true)

export async function loadBackground(blur: boolean): Promise<void> {
	backgroundBlur.value = blur
	const bytes = await invoke<ArrayBuffer>('plugin:noctrinth-background|background_get')
	if (backgroundUrl.value) URL.revokeObjectURL(backgroundUrl.value)
	backgroundUrl.value = bytes.byteLength ? URL.createObjectURL(new Blob([bytes])) : null
}

/** Asks for a picture and makes it the background. False when none was picked. */
export async function chooseBackground(): Promise<boolean> {
	const path = await open({
		multiple: false,
		filters: [{ name: 'Image', extensions: ['png', 'jpg', 'jpeg', 'webp', 'gif', 'avif'] }],
	})
	if (typeof path !== 'string') return false
	await invoke('plugin:noctrinth-background|background_set', { path })
	await loadBackground(backgroundBlur.value)
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
