/**
 * Ely.by skin helpers.
 *
 * Ely.by's skin server (skinsystem.ely.by) sends no CORS headers, so a skin
 * image loaded directly into a <canvas> taints it and toDataURL() throws.
 * To avoid that, the skin texture is fetched by the Rust backend (native
 * HTTP, no CORS) and handed to the frontend as raw bytes. We turn those bytes
 * into a data: URL, which is same-origin and never taints the canvas.
 */
import { invoke } from '@tauri-apps/api/core'

const skinTextureCache = new Map<string, Promise<string>>()
const headCache = new Map<string, string>()

export function getElySkinUrl(username: string): string {
	return `https://skinsystem.ely.by/skins/${encodeURIComponent(username)}.png`
}

export function getElyCapeUrl(username: string): string {
	return `https://skinsystem.ely.by/cloaks/${encodeURIComponent(username)}.png`
}

function bytesToDataUrl(bytes: number[] | Uint8Array): string {
	const arr = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes)
	let binary = ''
	for (let i = 0; i < arr.length; i++) {
		binary += String.fromCharCode(arr[i])
	}
	return `data:image/png;base64,${btoa(binary)}`
}

/**
 * Fetches the Ely.by skin texture for a user and returns it as a data: URL.
 * Cached per username. Rejects if the user has no custom skin.
 */
export function getElySkinTexture(username: string): Promise<string> {
	const cached = skinTextureCache.get(username)
	if (cached) return cached

	const promise = invoke<number[]>('plugin:ely-auth|ely_get_skin_texture', { username })
		.then((bytes) => bytesToDataUrl(bytes))
		.catch((error) => {
			skinTextureCache.delete(username)
			throw error
		})

	skinTextureCache.set(username, promise)
	return promise
}

/**
 * Renders a Minecraft head (face + hat overlay) from an Ely.by skin texture
 * and returns a data: URL. Results are cached per username+size.
 */
export async function getElyHeadUrl(username: string, size = 128): Promise<string> {
	const cacheKey = `${username}:${size}`
	const cached = headCache.get(cacheKey)
	if (cached) return cached

	const textureDataUrl = await getElySkinTexture(username)

	const url = await new Promise<string>((resolve, reject) => {
		const img = new Image()
		// Source is a data: URL (same-origin) — the canvas is never tainted.
		img.src = textureDataUrl
		img.onload = () => {
			try {
				const canvas = document.createElement('canvas')
				canvas.width = size
				canvas.height = size
				const ctx = canvas.getContext('2d')
				if (!ctx) {
					reject(new Error('Failed to create canvas context'))
					return
				}
				ctx.imageSmoothingEnabled = false
				// A standard skin texture is 64px wide. Ely.by also allows
				// high-resolution skins (128, 256, ...), so the head regions
				// must be scaled proportionally to the actual texture width
				// rather than using hard-coded 64px coordinates.
				const unit = (img.naturalWidth || 64) / 8
				// Base head face: region at (1,1)-(2,2) in 8-unit grid.
				ctx.drawImage(img, unit, unit, unit, unit, 0, 0, size, size)
				// Hat / overlay layer: region at (5,1)-(6,2) in 8-unit grid.
				ctx.drawImage(img, unit * 5, unit, unit, unit, 0, 0, size, size)
				resolve(canvas.toDataURL('image/png'))
			} catch (e) {
				reject(e instanceof Error ? e : new Error(String(e)))
			}
		}
		img.onerror = () => reject(new Error(`Failed to load Ely.by skin for ${username}`))
	})

	headCache.set(cacheKey, url)
	return url
}

/** Clears all cached Ely.by skin/head data (e.g. after a skin change). */
export function clearElySkinCache(): void {
	skinTextureCache.clear()
	headCache.clear()
}
