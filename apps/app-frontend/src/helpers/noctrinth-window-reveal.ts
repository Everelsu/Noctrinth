import { invoke } from '@tauri-apps/api/core'

/**
 * Shows the main window once the splash has been painted, on a background
 * already the splash's colour, so it never opens on a black frame that then
 * snaps to the theme. A hidden window may never run animation frames, so a
 * short timeout stands in for them.
 */
export async function revealMainWindow() {
	const base = document.querySelector('.splash-screen .base-bg') ?? document.body
	const channels = getComputedStyle(base).backgroundColor.match(/\d+(\.\d+)?/g)
	const background = channels && channels.length >= 3 ? channels.slice(0, 3).map(Number) : null

	await Promise.race([
		new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))),
		new Promise((resolve) => setTimeout(resolve, 150)),
	])
	await invoke('show_window', { background })
}
