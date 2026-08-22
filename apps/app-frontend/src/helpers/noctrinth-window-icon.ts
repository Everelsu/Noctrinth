/**
 * Paints the window's icon in the accent colour.
 *
 * The icon that ships with the app is the knot in the fork's purple, so a
 * launcher set to any other preset had one purple thing left on screen — in the
 * title bar and the taskbar, next to a window that was no longer purple at all.
 * The mark is a single path, so it can simply be drawn again in the colour that
 * is wanted: the SVG is recoloured as text, rasterised through a canvas, and
 * handed to the window as raw pixels.
 *
 * Windows keeps two icons per window — a small one for the window itself and a
 * big one for the taskbar and Alt-Tab — and only the small one is Tauri's to
 * set, so a command of the fork's own copies it across afterwards.
 *
 * Failures are silent by design. An icon is not worth an error dialog, and the
 * one the app was installed with is already on screen if this does not land.
 */
import { invoke } from '@tauri-apps/api/core'
import { Image } from '@tauri-apps/api/image'
import { getCurrentWindow } from '@tauri-apps/api/window'

import iconSource from '@/assets/noctrinth-icon.svg?raw'

/**
 * Drawn at the largest size Windows asks for, so the taskbar has something to
 * scale down rather than up. One bitmap covers every size the shell wants.
 */
const ICON_SIZE = 256

let lastPainted: string | null = null

function rasterise(color: string): Promise<ImageData> {
	return new Promise((resolve, reject) => {
		// `currentColor` has no meaning in a standalone SVG — it resolves to
		// black — so it is replaced with the colour being painted.
		const svg = iconSource.replaceAll('currentColor', color)
		const url = URL.createObjectURL(new Blob([svg], { type: 'image/svg+xml' }))
		const image = new window.Image()

		image.onload = () => {
			try {
				const canvas = document.createElement('canvas')
				canvas.width = ICON_SIZE
				canvas.height = ICON_SIZE

				const context = canvas.getContext('2d')
				if (!context) throw new Error('No 2D context to draw the icon on')

				context.drawImage(image, 0, 0, ICON_SIZE, ICON_SIZE)
				resolve(context.getImageData(0, 0, ICON_SIZE, ICON_SIZE))
			} catch (error) {
				reject(error instanceof Error ? error : new Error(String(error)))
			} finally {
				URL.revokeObjectURL(url)
			}
		}

		image.onerror = () => {
			URL.revokeObjectURL(url)
			reject(new Error('Failed to load the icon for recolouring'))
		}

		image.src = url
	})
}

/** Repaints the window icon, unless it is already the colour asked for. */
export async function paintWindowIcon(color: string): Promise<void> {
	if (!color || color === lastPainted) return
	lastPainted = color

	try {
		const pixels = await rasterise(color)
		// Same bytes, as the type the window expects rather than the clamped one
		// a canvas hands back.
		const rgba = new Uint8Array(pixels.data.buffer)
		const icon = await Image.new(rgba, pixels.width, pixels.height)
		await getCurrentWindow().setIcon(icon)
		await icon.close()

		// Windows keeps a second icon for the taskbar and Alt-Tab, and setIcon
		// does not touch it — so without this the taskbar keeps the icon built
		// into the executable. A no-op on every other platform.
		await invoke('plugin:window-icon|sync_taskbar_icon')
	} catch (error) {
		lastPainted = null
		console.warn('Failed to paint the window icon in the accent colour:', error)
	}
}
