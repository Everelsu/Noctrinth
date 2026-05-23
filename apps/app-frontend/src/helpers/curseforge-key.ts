/**
 * CurseForge API key — isolated in its own module so both `curseforge-api.ts`
 * and `pack.ts` can import it without creating a circular dependency.
 *
 * vite.config.ts injects `__NOCTRINTH_CURSEFORGE_KEY__` (read raw from
 * .env.local) — see curseforge-api.ts for the full rationale.
 */

declare const __NOCTRINTH_CURSEFORGE_KEY__: string

export const CURSEFORGE_API_KEY: string =
	typeof __NOCTRINTH_CURSEFORGE_KEY__ === 'string' ? __NOCTRINTH_CURSEFORGE_KEY__ : ''
