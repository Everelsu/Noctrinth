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
		version: '0.13.21',
		date: '2026-05-22T00:00:00+00:00',
		body: `## Added
- CurseForge integration in Discover. Search now spans both Modrinth and CurseForge. When you type a query, results from both catalogs are merged and de-duplicated; small Modrinth and CurseForge logos on each card show where a mod is hosted.
- Catalog source toggle. When browsing without a query, an animated toggle in the controls bar picks a single catalog (Modrinth or CurseForge). The active option expands to show its name in its brand colour; the inactive one collapses to just its icon. The choice persists across navigation and app restarts.
- Smart default ordering. 
-*CurseForge mod page. Opening a CurseForge mod from search now shows a full project page that reuses Modrinth’s layout — header, sidebar (compatibility, links, tags, details), Description, **Versions** tab (full versions table with filters, channels and pagination — identical to Modrinth’s), and **Gallery** tab (with the same lightbox, zoom and keyboard navigation).
- Install CurseForge mods. Pick a CurseForge mod and install it straight into an existing instance, or open the same instance-picker modal as Modrinth mods to choose where it goes (or create a new instance). Downloads are routed through the CurseForge file API.
- Install CurseForge modpacks. CurseForge modpacks are now installable from Discover. The pack zip is downloaded, its \`manifest.json\` parsed, every mod resolved through the CurseForge API, and the whole pack installed as a new instance — reusing the existing modpack progress UI.
- CurseForge modpack import. The “Import modpack” file picker now accepts \`.zip\` files in addition to \`.mrpack\`. The backend auto-detects CurseForge format and routes to the right installer.
- Catalog colour accent. An optional setting in Settings → Appearance recolors the Discover page accent (Install buttons, highlights) to match the active catalog — green for Modrinth, orange for CurseForge, blended for unified search. Off by default; scoped to the Discover page only, so the rest of the app stays on the Noctrinth purple theme.

## Changed
- Synced with upstream Modrinth (0.13.21): Kyros upload sessions for hosting, project analytics events, moderation queue tooling improvements, and various fixes.`,
	},
	{
		version: '0.13.20',
		date: '2026-05-21T00:00:00+00:00',
		body: `## Changed
- Synced with upstream Modrinth (0.13.20): content management improvements, new date picker, improved Intercom bubble positioning, macOS window occlusion checks, and various routing fixes.`,
	},
	{
		version: '0.13.19',
		date: '2026-05-20T00:00:00+00:00',
		body: `## Fixed
- Fixed sidebar show/hide animation: the right sidebar now slides in and out with a smooth \`transform\` transition instead of animating the grid column width. This eliminates content reflow — skin model previews, modpack cards and content grids no longer jump or temporarily show extra items while the sidebar is animating.

## Changed
- Synced with upstream Modrinth (0.13.19)`,
	},
	{
		version: '0.13.18',
		date: '2026-05-18T00:00:00+00:00',
		body: `## Changed
- Updated to the latest Modrinth App release(0.13.18) — synced with upstream to bring in its newest features and fixes.
- Discord Rich Presence now runs on Noctrinth's own Discord application with its own presence artwork, instead of Modrinth's.

## Fixed
- Fixed Ely.by account sign-in and launching so authenticating and starting the game with an Ely.by account work reliably.`,
	},
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
