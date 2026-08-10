/**
 * URL scheme our own links use to open the desktop app.
 *
 * The app claims both this and `modrinth://` (see `deep-link.desktop.schemes`
 * in `apps/app/tauri.conf.json`) so that install links on modrinth.com reach it
 * too. Our own links use the unambiguous one: `modrinth://` is contested, and
 * with Modrinth App installed the winner is whichever registered last.
 *
 * Kept in sync with `DEEP_LINK_SCHEME` in
 * `packages/app-lib/src/api/handler.rs`.
 */
export const APP_DEEP_LINK_SCHEME = 'noctrinth'

/** Builds a deep link into the desktop app, e.g. `noctrinth://mod/sodium`. */
export function appDeepLink(path: string): string {
	return `${APP_DEEP_LINK_SCHEME}://${path.replace(/^\/+/, '')}`
}
