/**
 * Ely.by skin helpers.
 *
 * Ely.by's skin server (skinsystem.ely.by) sends no CORS headers, so a skin
 * image loaded directly into a <canvas> taints it and toDataURL() throws.
 * To avoid that, textures are fetched natively (no CORS): primarily by the
 * Rust backend, with a Tauri HTTP plugin fetch as fallback. The raw bytes are
 * turned into a data: URL, which is same-origin and never taints the canvas.
 */
import { invoke } from '@tauri-apps/api/core'

import { proxiedFetch } from './proxy-fetch'

const skinTextureCache = new Map<string, Promise<string>>()
const capeTextureCache = new Map<string, Promise<string | null>>()
const headCache = new Map<string, string>()

/**
 * Bundled classic Steve texture — shown when a user has no custom Ely.by
 * skin (the skin server returns 404) or every fetch path failed, so the
 * preview never hangs on an eternal loading state.
 */
export const ELY_FALLBACK_SKIN =
	'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAEAAAABACAYAAACqaXHeAAAE5klEQVR4Xu1aMWsUQRgVVJCAoIIgglaJBG2UGIKCOU0hJHZKijRBsAnaWSjaiNjESgtTpT0bmxQWNvkJ+U9n3ube8ubNt7e5xOzdxX3wmNmZb/fmvflmdtm9M2dqcOfmxR44N325KFnn8ecXDwbSrzdxoODO7auleNTdgHfP5hKeKgNUsJff1zqZeBJ9fr2Jg4vW9Ef5Xxiga9+XAkRWLYFTYYDOuGYDxIMU6psf2r6tL54OA3wPUEIkBFeVfr2Jg4r2DRGk2OWZSwnZ7tcbO2hqU9j0tQtJyjMLlGqCnqPX0WtEcSh9PI0jGiBIIVpX8V7yvEg04/z6aPPxNA4V4QNm2/ar5d7vD+u9P1/eFOWvt2tF2/2ZK1ksyGO2aZ0msM3H0zg4IB2oziSEQjCEUzyJvkhUdE3WGc9jH0/j0IGxxC0M4iACQoHnj76WolEHcIwYtOGc6Fo0NDJoLAzwGaIgJWf89eO7BTUDlBTlM6/7gxqCYx9P4+BAUHLmeQ//8XKxfKDBre390nzBpRtTSQxvecwEvaYb4Mc+nsbBpzYV7yaghGiYwJKiaZCey+vpk+Gt61MJ2e7jGTke3vvaUy4sLBScnZ0t6PEZ9vbK5aHGog19fn2nXy7Dzk6vZLebLNHiN44LHxCF0wiPz7AvUsWrCSdhgO5H+B0PHxo+oKNkwEkbMLW7mxkwVhlwoktgX3S0BI6cAU/mt3sgfhzl04c/S7KNJan957a2EurMYFCl+P7Aq+LBpF2FiuCSbPv0KaWfUwcKPKwB2g9isOc3NwuWgvpCmZbeHsUnIr2N7S4ehOiNjQPSAI2rwyCBnh3eD0LI2Y8fC6Je/nC/9DaPdxPcrEwQ2T3ImNAAPacOgwRWGaB1iiHrBHl8IoyiVISnuJPi1QRlHVwgj6M27SNdUDLjZoDOfmRAtva7B5scyQ1Vy4h6juvN4Ab4DEeG6LGndGlAX4gb4PGeIaX4/rHe4nTH1zYVrfHF5luHyABfBoP6KYr0FNZZ9dgo3s+NRKkRkQFad70ZKKxKYJ0BGKSKKTJAZ5TZ0Bfk8aEBEh8ZoEaoASg9xvVm8PRW4UqPIylIxVG4i2OMxyUGiHjE6Ky6OBqgRniM682wsrLSA31zczLOqaIiM1Sgx5YGiGgVD6pYFxeZ4P2uNwMebzudTiJqdXU1E4oYj0Vdl4gapplDIyjMY3T5aAwyscoAzQw3QGNdbwY+41OcC6TwKA51NYDUZcSSM+rxvtcwhu0qxkWCfmv0GNfbokWLFi1atJgc4AVoFf3xmU+dSUyLFi1atGjRokVT8FdpQ39c1XeE+/S3QB4+dnADhv68bgbwTc+hX3GNGm7AUTOAj7qaARNpwLEyYL+uGTCWS0Df/vpLTu/TfsZkHzODbwS6JDKOGocRyWzwPjD5wqsGDDJB20YNNwClG6D9dQaUH0uqBLsho4YLdJHer22FAXXf9+uoWdI3xt8bVPGfGOjieRy1aR/p3/b9Q4d/8FCiz8fTOKoE+2xrv8arGL3f687vt8Kxui1GAuuWgNINGCRc+1j38TQOF4gyMiAyB4xS3YW7AUofT+Pw9PeZ1vZomURr3EUOygofT+Pgl2QKrKJ/bifrxFcJHxsD+LlcRQ3z/4LIABfsx9rm42kc0f8GVCCFR3GoQ0jVPuBZEfX5eIbFX3srPNN8aUvJAAAAAElFTkSuQmCC'

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
 * Native fetch of a texture URL via the Tauri HTTP plugin (no CORS). The
 * timestamp query busts HTTP caches (CDN and WebView) so a freshly uploaded
 * skin shows up immediately instead of a stale cached texture.
 */
async function fetchTextureDataUrl(url: string): Promise<string> {
	const res = await proxiedFetch(`${url}?_=${Date.now()}`)
	if (!res.ok) {
		throw new Error(`HTTP ${res.status} for ${url}`)
	}
	const buffer = await res.arrayBuffer()
	return bytesToDataUrl(new Uint8Array(buffer))
}

/**
 * Fetches the Ely.by skin texture for a user and returns it as a data: URL.
 * Tries the Rust backend first, then falls back to a direct native fetch so
 * a backend hiccup can't break the preview. Cached per username. Rejects
 * only when both paths fail (e.g. the user has no custom skin — HTTP 404).
 */
export function getElySkinTexture(username: string): Promise<string> {
	const cached = skinTextureCache.get(username)
	if (cached) return cached

	const promise = invoke<number[]>('plugin:ely-auth|ely_get_skin_texture', { username })
		.then((bytes) => bytesToDataUrl(bytes))
		.catch((backendError) => {
			console.warn(
				`[Ely.by] Backend skin fetch failed for ${username}, falling back to direct fetch:`,
				backendError,
			)
			return fetchTextureDataUrl(getElySkinUrl(username))
		})
		.catch((error) => {
			skinTextureCache.delete(username)
			throw error
		})

	skinTextureCache.set(username, promise)
	return promise
}

/**
 * Fetches the Ely.by cape texture for a user as a data: URL, or null when
 * the user has no cape (the skin server answers 404). Cached per username.
 */
export function getElyCapeTexture(username: string): Promise<string | null> {
	const cached = capeTextureCache.get(username)
	if (cached) return cached

	const promise = fetchTextureDataUrl(getElyCapeUrl(username)).catch(() => null)

	capeTextureCache.set(username, promise)
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

/** Clears all cached Ely.by skin/cape/head data (e.g. after a skin change). */
export function clearElySkinCache(): void {
	skinTextureCache.clear()
	capeTextureCache.clear()
	headCache.clear()
}
