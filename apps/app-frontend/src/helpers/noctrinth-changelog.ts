/**
 * Noctrinth changelog.
 *
 * Mirrors the shape of Modrinth's `packages/blog/changelog.ts` so the
 * Changelog settings tab can render both with the same logic. This file is
 * Noctrinth-specific and intentionally lives outside `packages/blog` so it
 * is never overwritten when the repo is synced with upstream Modrinth.
 *
 * Add a new entry on top for each Noctrinth release. The `body` is markdown:
 * use `### Added`, `### Changed`, `### Deprecated`, `### Removed`,
 * `### Fixed`, `### Security` section headings (matching the Modrinth PR
 * changelog template / Keep a Changelog convention) and `- item` bullets.
 */

export interface NoctrinthVersionEntry {
	version: string
	/** ISO date string. */
	date: string
	body: string
}

export const NOCTRINTH_CHANGELOG: NoctrinthVersionEntry[] = [
	{
		version: '0.14.6',
		date: '2026-06-12T00:00:00+00:00',
		body: `### Added
- Ely.by skin management inside the app. "Change skin" on the Skins page opens the Ely.by skin page in an embedded window (signing in once is remembered); closing it refreshes the preview and the account avatar automatically. The preview now shows the account's cape, detects slim/classic arms from the actual texture, falls back to the default Steve skin (with a notice) when no custom skin exists, and has a "Refresh preview" button. Freshly uploaded skins appear immediately — texture requests bypass stale CDN/HTTP caches.
- Proxy setting in Settings → Resource management. One proxy URL (http://, https://, socks5:// or socks5h://) routes all launcher traffic — Modrinth API, CDN downloads, CurseForge — through your proxy. Useful where Modrinth is geo-blocked. Empty by default; applies after an app restart.
- CurseForge master toggle in Settings → Appearance. One switch turns every CurseForge feature on or off: the catalog toggle on the Discover page, CurseForge search results, and CurseForge project pages. Turning it off while CurseForge is the active catalog switches Discover back to Modrinth.

### Changed
- Modrinth and CurseForge are now fully separate catalogs. The toggle on the Discover page always picks exactly one source — including text searches, which previously merged both into one list. Source badges on project cards are gone (a result's catalog is now always the selected one), pagination in CurseForge mode reflects real CurseForge result counts, and the sidebar only shows filters CurseForge actually supports: game version, loader and categories (client/server environment and license filters are Modrinth-only and are hidden in CF mode).
- CurseForge downloads hardened. Mod and modpack downloads are verified against CurseForge's SHA1 checksums and retried automatically; every download has a CDN mirror fallback (derived from the file ID, the Prism/MultiMC approach) for files whose authors hid the API download URL. Modpack installs no longer silently skip such files — previously a pack could install "successfully" with mods missing. Files dropped from CurseForge's bulk lookup are re-fetched individually (the XMCL approach), and API requests retry transient errors (429/5xx) with backoff.
- Synced with upstream Modrinth (0.14.5 → 0.14.6): drag-and-drop reordering of saved skins, draggable Mojang default skins, fixed translucency on the skin outer layer, better offline handling on the Skins page, live system-theme updates without an app restart, new error cases in the Microsoft sign-in error modal, and fixes for notification pop-ups appearing over open modals, the loading flash when deleting a skin, and hardcoded skins not being editable.

### Fixed
- Ely.by skin preview no longer hangs on an endless "Loading" state, and the account picker avatar updates after a skin change.
- Russian translations for the new settings (CurseForge toggle, proxy) and the Ely.by skin page.
- App startup no longer logs a spurious "User is not logged in" backend error when an Ely.by account is the active one.`,
	},
	{
		version: '0.14.5',
		date: '2026-06-09T00:00:00+00:00',
		body: `### Changed
- Synced with upstream Modrinth (0.14.3 → 0.14.5): new "release channels" instance setting that controls which content versions count as an update (defaults to release-only), reworked updater UI (the "Reload to update" action moved to the top-right action bar and update pop-ups now appear only after the prompt has been ignored for 24 hours), uncommon plugin loaders collapsed behind a "Show more" toggle, and assorted analytics and translation updates.
- Update pop-up "Changelog" button now opens the Noctrinth GitHub page instead of the Modrinth website.
- Changelog website timestamps are now recalculated live on every visit (and refreshed each minute) instead of being frozen at build time, so "X days ago" labels stay accurate. Hovering a timestamp shows the exact patch date and time.

### Fixed
- Fixed drag-and-drop file upload not working in the instance Files tab.
- Fixed a layering issue where the expanded console view in the Logs tab could render behind other elements.
- Fixed the updater restarting the app even when a restart wasn't requested.`,
	},
	{
		version: '0.14.3',
		date: '2026-06-07T00:00:00+00:00',
		body: `### Added
- Screenshots tab for instances. Browse, zoom, copy to clipboard, reveal in explorer, and delete screenshots taken in-game — all from inside the app.

### Changed
- Synced with upstream Modrinth (0.14.1 → 0.14.3): play-time toggle in instance header, Mr. Pack default skin, Pride Fundraiser banner, tooltip and card styling improvements, large \`.mrpack\` import crash fix, content-update modal fix, Windows path separator fix in the Files tab, and translation updates.
- Update popup changelog button now links to the Noctrinth GitHub page instead of the Modrinth website.
- Download notification icon colour updated to match Noctrinth's purple brand palette.

### Fixed
- Russian locale updated with all missing translation keys (skin selector strings, collections, notifications, screenshots).`,
	},
	{
		version: '0.14.0',
		date: '2026-05-29T00:00:00+00:00',
		body: `### Added
- CurseForge install detection via fingerprint API. Borrowed XMCL's approach: every \`.jar\`, \`.zip\` in \`mods/\`, \`shaderpacks/\`, \`resourcepacks/\`, \`datapacks/\` is hashed with CurseForge's Murmur2 (whitespace-stripped, seed = 1) and resolved through \`POST /v1/fingerprints/432\`. Catches mods you dragged into the folder manually, mods installed before this app version existed, and instances restored from backup — without any per-install bookkeeping.
- Cross-platform mapping. A curated table of well-known Modrinth ↔ CurseForge ID pairs (Sodium, JEI, Iris, Create, Botania, JourneyMap, …) is consulted whenever \`unifiedSearch\` merges a Modrinth hit with a CurseForge hit; the table also grows automatically from successful runtime slug-matches (persisted to localStorage). Mods that exist on both platforms now show up as a single card in unified search, and installing one platform's version marks the other as installed too.
- CurseForge dependency resolver with relationType propagation. The XMCL algorithm: \`Embedded\` / \`Tool\` / \`Include\` deps are skipped (not runtime), \`Incompatible\` mods are surfaced as a warning, \`Optional\` deps are returned for the UI to offer (not silently installed), and \`Required\` deps stay required ONLY if every ancestor was also required — preventing silent install of a hard requirement of something you didn't ask for.
- CurseForge categories for shader / resource pack / datapack tabs. Sidebar in CF mode now has curated category lists for all four content types (mods + 3 others), each mapping a Modrinth icon-bearing slug to a real CurseForge category slug that actually returns results. The same Modrinth slug can resolve to different CF categories depending on the active project type.
- Pagination "…" → page input. Clicking the gap in the pagination row turns it into a small number input; Enter jumps to that page (clamped to \`[1, count]\`), Escape or blur cancels. Works everywhere \`Pagination\` is rendered.
- CurseForge "Alpha" badge on the source toggle, visible only when CurseForge is the active catalog — flags the integration as experimental until proven stable.
- Russian translations for the account dropdown, Collections page (incl. sort/search), Notifications page (incl. all type chips), Collection detail page, sidebar tooltips, Ely.by login modal, Changelog tab, dashboard nav tabs.
- Search and sort for the Followed-projects view (and any collection): filter by query (matches title, slug, description) + sort by Name (A-Z) / Downloads / Followers / Recently updated. Sort choice persists across reloads.

### Changed
- Synced with upstream Modrinth 0.14.0. New skin selector out of beta, skin preview improvements, project analytics performance, Daedalus manifest uploads on new game versions, allowing \`user-images.githubusercontent.com\` in CSP, and various smaller fixes.
- Skin preview rotation. Drag up / down now tilts the model around its pitch axis (clamped to ±90°) so the crown and soles are reachable, in addition to the existing yaw drag. The spotlight shadow is now parented to the model and sits at the feet — it tilts and translates with the figure instead of staying as a static rug below.
- CurseForge content sidebar redesign. Curated Modrinth-style category list (Library, Magic, Storage, Technology, Equipment, Mobs, …) instead of CurseForge's raw taxonomy.\
- Compact number suffixes are now Latin (\`k\`, \`m\`, \`b\`) regardless of locale, while keeping the numeric formatting locale-correct (\`1,23m\` in Russian, \`1.23m\` in English). \`355,64 млн\` and similar long ICU labels no longer crowd the UI on Cyrillic locales.

### Fixed
- Modpack file import no longer mysteriously deletes the instance.** If the selected file isn't a recognised modpack (no \`modrinth.index.json\` and no \`manifest.json\`), or if it's a CurseForge modpack without a CurseForge API key, the user sees a clear error message BEFORE any profile is created. The dispatcher's cleanup-on-error contract guarantees that even when something fails mid-install, no orphan profiles are left behind.
- Filename sanitisation at the Windows filesystem boundary.** Modpacks shipping files with names like \`EpicSiegeMod_汉字.jar\` (or its mangled \`EpicSiegeMod_???? ???.jar\` form from a lossy ANSI round-trip) no longer crash the install with \`ERROR_INVALID_NAME (os error 123)\`. Reserved Windows characters (\`<>:"|?*\`), control bytes, and U+FFFD are replaced with \`_\` right before \`write()\` — the manifest itself is untouched.
- CurseForge categories were swapped.**
- Vanilla instances can now install CurseForge resource packs, shaders, and datapacks. The "compatible loader" check was rejecting any file whose \`gameVersions\` contained a non-numeric entry, but those entries also include informational tags like \`Iris\`, \`OptiFine\`, \`Data Pack\` that aren't actual mod loaders. The check now only treats Forge / Fabric / Quilt / NeoForge / Cauldron / LiteLoader as disqualifying.`,
	},
	{
		version: '0.13.24',
		date: '2026-05-26T00:00:00+00:00',
		body: `### Changed
- Synced with upstream Modrinth (0.13.22 → 0.13.24): default Minecraft memory raised from 2 GB to 4 GB, improved Java installation UI, new "Enabled" sort button next to "Disabled" in content lists, compact log handling to prevent crashes on log spam, locale updates and various smaller fixes.

### Fixed
- Guarded against non-string values when decorating download URLs.
- Page no longer fails to load when an auth cookie is invalid.`,
	},
	{
		version: '0.13.21',
		date: '2026-05-22T00:00:00+00:00',
		body: `### Added
- CurseForge integration in Discover. Search now spans both Modrinth and CurseForge. When you type a query, results from both catalogs are merged and de-duplicated; small Modrinth and CurseForge logos on each card show where a mod is hosted.
- Catalog source toggle. When browsing without a query, an animated toggle in the controls bar picks a single catalog (Modrinth or CurseForge). The active option expands to show its name in its brand colour; the inactive one collapses to just its icon. The choice persists across navigation and app restarts.
- Smart default ordering.
- CurseForge mod page. Opening a CurseForge mod from search now shows a full project page that reuses Modrinth's layout — header, sidebar (compatibility, links, tags, details), Description, **Versions** tab (full versions table with filters, channels and pagination — identical to Modrinth's), and **Gallery** tab (with the same lightbox, zoom and keyboard navigation).
- Install CurseForge mods. Pick a CurseForge mod and install it straight into an existing instance, or open the same instance-picker modal as Modrinth mods to choose where it goes (or create a new instance). Downloads are routed through the CurseForge file API.
- Install CurseForge modpacks. CurseForge modpacks are now installable from Discover. The pack zip is downloaded, its \`manifest.json\` parsed, every mod resolved through the CurseForge API, and the whole pack installed as a new instance — reusing the existing modpack progress UI.
- CurseForge modpack import. The "Import modpack" file picker now accepts \`.zip\` files in addition to \`.mrpack\`. The backend auto-detects CurseForge format and routes to the right installer.
- Catalog colour accent. An optional setting in Settings → Appearance recolors the Discover page accent (Install buttons, highlights) to match the active catalog — green for Modrinth, orange for CurseForge, blended for unified search. Off by default; scoped to the Discover page only, so the rest of the app stays on the Noctrinth purple theme.

### Changed
- Synced with upstream Modrinth (0.13.21): Kyros upload sessions for hosting, project analytics events, moderation queue tooling improvements, and various fixes.`,
	},
	{
		version: '0.13.20',
		date: '2026-05-21T00:00:00+00:00',
		body: `### Changed
- Synced with upstream Modrinth (0.13.20): content management improvements, new date picker, improved Intercom bubble positioning, macOS window occlusion checks, and various routing fixes.`,
	},
	{
		version: '0.13.19',
		date: '2026-05-20T00:00:00+00:00',
		body: `### Fixed
- Fixed sidebar show/hide animation: the right sidebar now slides in and out with a smooth \`transform\` transition instead of animating the grid column width. This eliminates content reflow — skin model previews, modpack cards and content grids no longer jump or temporarily show extra items while the sidebar is animating.

### Changed
- Synced with upstream Modrinth (0.13.19)`,
	},
	{
		version: '0.13.18',
		date: '2026-05-18T00:00:00+00:00',
		body: `### Changed
- Updated to the latest Modrinth App release (0.13.18) — synced with upstream to bring in its newest features and fixes.
- Discord Rich Presence now runs on Noctrinth's own Discord application with its own presence artwork, instead of Modrinth's.

### Fixed
- Fixed Ely.by account sign-in and launching so authenticating and starting the game with an Ely.by account work reliably.`,
	},
	{
		version: '0.13.17',
		date: '2026-05-17T00:00:00+00:00',
		body: `### Added
- Added Ely.by as a second account provider, authenticating against the Ely.by Yggdrasil server.
- Added support for launching Minecraft with Ely.by accounts by injecting the authlib-injector Java agent at launch.
- Added a Collections section for browsing, creating, editing and deleting Modrinth collections.
- Added a "Followed projects" view that lists every project the signed-in user follows.
- Added a "Save to collection" button to project pages.
- Added an in-app Notifications page backed by the Modrinth notifications API.
- Added native desktop notifications for downloads and updates.
- Added a Changelog tab in settings showing both the Noctrinth and Modrinth App changelogs.
- Added signed application updates delivered through GitHub Releases.

### Changed
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
