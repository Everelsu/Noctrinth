import { invoke } from '@tauri-apps/api/core'
import { nextTick, ref } from 'vue'

import { debugStartup } from '@/helpers/startup-debug'

const wait = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms))
const nextFrame = () => new Promise<void>((resolve) => requestAnimationFrame(() => resolve()))

/**
 * The theme and accent cached from the last session are only a guess; the
 * settings are the truth, and they arrive a moment after the window could
 * open. Opening on the guess is how a light theme started black and then went
 * white. App.vue calls `markThemeSettled` the moment the settings' theme is
 * applied; the splash draws nothing until then, and the window does not open.
 */
export const themeReady = ref(false)

let settleTheme: () => void = () => {}
const themeSettled = new Promise<void>((resolve) => {
	settleTheme = resolve
})

export function markThemeSettled() {
	themeReady.value = true
	settleTheme()
}

/** Long enough for any real start; a start that failed must still show its window. */
const THEME_WAIT_MS = 3000
/** The most the window waits for the splash, once the theme is known. */
const PAINT_WAIT_MS = 800
/**
 * After the frames are drawn, a little longer for them to reach the screen.
 * Recorded: the frame painted for a theme change reached the screen some
 * fifty milliseconds after the main thread had moved on, and a window shown
 * in between showed the frame before it.
 */
const COMPOSITE_MS = 90

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
 * Shows the main window once the splash is ready to be seen: in the settings'
 * theme, with its cube decoded and its frame on screen, over a window
 * background already that colour. Every wait is capped, because a hidden
 * window may never run animation frames and a failed start must still open.
 */
export async function revealMainWindow() {
	await Promise.race([themeSettled, wait(THEME_WAIT_MS)])
	themeReady.value = true
	// The theme's class and the splash's ready state land in Vue's next flush.
	await nextTick()

	const splash = document.querySelector('.splash-screen') ?? document.body
	await Promise.race([
		(async () => {
			await decodeSplashImage(splash)
			await nextFrame()
			await nextFrame()
			await wait(COMPOSITE_MS)
		})(),
		wait(PAINT_WAIT_MS),
	])

	const background = toRgb(getComputedStyle(splash).backgroundColor)
	debugStartup('Window revealed', { theme: document.documentElement.className, background })
	await invoke('show_window', { background })
}
