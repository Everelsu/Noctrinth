/**
 * URL scheme the desktop app registers with the OS.
 *
 * Noctrinth registers `noctrinth://` rather than `modrinth://` so it never
 * fights the official Modrinth App for the protocol when both are installed.
 * Every link meant to open the app has to be built from here — a hardcoded
 * `modrinth://` would silently hand the user off to the other launcher, or do
 * nothing at all when it isn't installed.
 *
 * Kept in sync with `deep-link.desktop.schemes` in `apps/app/tauri.conf.json`
 * and `DEEP_LINK_SCHEME` in `packages/app-lib/src/api/handler.rs`.
 */
export const APP_DEEP_LINK_SCHEME = 'noctrinth'

/** Builds a deep link into the desktop app, e.g. `noctrinth://mod/sodium`. */
export function appDeepLink(path: string): string {
	return `${APP_DEEP_LINK_SCHEME}://${path.replace(/^\/+/, '')}`
}
