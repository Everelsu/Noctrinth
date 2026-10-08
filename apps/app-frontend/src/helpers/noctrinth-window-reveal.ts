import { invoke } from '@tauri-apps/api/core'

const wait = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms))

/** The splash's cube, decoded before anybody can see it arrive late. */
async function decodeSplashImage(splash: Element) {
	const value = getComputedStyle(splash).getPropertyValue('--splash-cube-image')
	const url = value.match(/url\(["']?([^"')]+)["']?\)/)?.[1]
	if (!url) return
	const image = new Image()
	image.src = url
	await image.decode().catch(() => {})
}

/**
 * Any CSS colour as the three bytes the window takes. A tinted theme writes its
 * surfaces in oklch, which only the browser can be trusted to convert.
 */
function toRgb(color: string): number[] | null {
	const context = document.createElement('canvas').getContext('2d', { willReadFrequently: true })
	if (!context) return null
	context.fillStyle = color
	context.fillRect(0, 0, 1, 1)
	const [red, green, blue, alpha] = context.getImageData(0, 0, 1, 1).data
	return alpha === 0 ? null : [red, green, blue]
}

/**
 * Shows the main window once the splash is ready to be seen, on a background
 * already the splash's colour, so it never opens on a black frame that then
 * snaps to the theme, nor with the cube popping in after it. Both waits are
 * capped: a hidden window may never run animation frames, and a slow disk must
 * not keep the window from opening.
 */
export async function revealMainWindow() {
	const splash = document.querySelector('.splash-screen') ?? document.body
	const background = toRgb(getComputedStyle(splash).backgroundColor)

	await Promise.race([
		Promise.all([
			decodeSplashImage(splash),
			new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))),
		]),
		wait(400),
	])
	await invoke('show_window', { background })
}
