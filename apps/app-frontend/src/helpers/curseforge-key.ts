/**
 * CurseForge API key — isolated in its own module so both `curseforge-api.ts`
 * and `pack.ts` can import it without creating a circular dependency.
 *
 * vite.config.ts injects `__NOCTRINTH_CURSEFORGE_KEY__` (read raw from
 * .env.local) — see curseforge-api.ts for the full rationale.
 */

import { invoke } from '@tauri-apps/api/core'

declare const __NOCTRINTH_CURSEFORGE_KEY__: string

export const CURSEFORGE_API_KEY: string =
	typeof __NOCTRINTH_CURSEFORGE_KEY__ === 'string' ? __NOCTRINTH_CURSEFORGE_KEY__ : ''

/**
 * Tells the backend what the key is, once, at startup.
 *
 * Naming an installed CurseForge file happens in the content listing, far from
 * anywhere a key could be passed as an argument, so the backend keeps its own
 * copy. A build with no key configured says so rather than staying silent —
 * the backend then skips CurseForge lookups instead of waiting on them.
 */
export async function registerCurseforgeApiKey(): Promise<void> {
	await invoke('plugin:noctrinth-curseforge|curseforge_set_api_key', {
		key: CURSEFORGE_API_KEY || null,
	})
}
