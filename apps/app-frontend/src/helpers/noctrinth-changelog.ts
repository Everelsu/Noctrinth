/**
 * Noctrinth changelog.
 *
 * Mirrors the shape of Modrinth's `packages/blog/changelog.ts` so the
 * Changelog settings tab can render both with the same logic. This file is
 * Noctrinth-specific and intentionally lives outside `packages/blog` so it
 * is never overwritten when the repo is synced with upstream Modrinth.
 *
 * Add a new entry on top for each Noctrinth release. The `body` is markdown:
 * use `## Section` headings and `- item` bullet points.
 */

export interface NoctrinthVersionEntry {
	version: string
	/** ISO date string. */
	date: string
	body: string
}

export const NOCTRINTH_CHANGELOG: NoctrinthVersionEntry[] = [
	{
		version: '0.13.17',
		date: '2026-05-17T00:00:00+00:00',
		body: `## Added
- Added Ely.by as a second account provider, authenticating against the Ely.by Yggdrasil server.
- Added support for launching Minecraft with Ely.by accounts by injecting the authlib-injector Java agent at launch.
- Added a Collections section for browsing, creating, editing and deleting Modrinth collections.
- Added a "Followed projects" view that lists every project the signed-in user follows.
- Added a "Save to collection" button to project pages.
- Added an in-app Notifications page backed by the Modrinth notifications API.
- Added native desktop notifications for downloads and updates.
- Added a Changelog tab in settings showing both the Noctrinth and Modrinth App changelogs.
- Added signed application updates delivered through GitHub Releases.

## Changed
- Rebranded the application to Noctrinth, including a new logo, generated app icons and the \`com.noctrinth.app\` bundle identifier.
- Replaced the embedded sign-in WebView with a loopback HTTP redirect for the Modrinth OAuth flow.
- Unified account selection so exactly one account is active across the Microsoft and Ely.by providers.
- Recolored the interface to a purple brand scheme — modal overlays, the server status indicator, the skin selector and download notifications no longer use green.
- Reworked the Feature Flags settings to show readable names and descriptions instead of raw flag keys.
- Animated the sidebar so it slides in and out when toggled.
- Extended the requested Modrinth OAuth scopes.`,
	},
]

/** Returns the Noctrinth changelog, newest version first. */
export function getNoctrinthChangelog(): NoctrinthVersionEntry[] {
	return NOCTRINTH_CHANGELOG
}
