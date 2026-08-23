/**
 * Noctrinth changelog.
 *
 * Mirrors the shape of Modrinth's `packages/blog/changelog.ts` so the
 * Changelog settings tab can render both with the same logic. This file is
 * Noctrinth-specific and intentionally lives outside `packages/blog` so it is
 * never overwritten when the repo is synced with upstream Modrinth.
 *
 * Add a new entry on top for each Noctrinth release. The `body` is markdown:
 * use `### Added`, `### Changed`, `### Deprecated`, `### Removed`,
 * `### Fixed`, `### Security` section headings (matching the Modrinth PR
 * changelog template / Keep a Changelog convention) and `- item` bullets.
 *
 * Keep every bullet to one line, in the plain style Modrinth's own changelog
 * uses: what changed, not why or how it was done. Links work, and so do
 * screenshots:
 *
 * ```markdown
 * ![Modern Java](/changelog/screenshots/modern-java.png)
 * ```
 *
 * Screenshots are the one part of an entry that is not written here. They live
 * in `src/changelog/screenshots/` and are published with the changelog site,
 * which is what the path above resolves against — so one is downloaded by
 * whoever scrolls to it instead of sitting inside every installer from then
 * on. See that folder's README.
 */

export interface NoctrinthVersionEntry {
	version: string
	/** ISO date string. */
	date: string
	body: string
}

export const NOCTRINTH_CHANGELOG: NoctrinthVersionEntry[] = [
	{
		version: '0.18.2',
		date: '2026-08-21T00:00:00+00:00',
		body: `### Added
- Accent presets in Settings → Appearance: Theme, Amethyst, Nightshade, Midnight, Glacier, Verdant, Lantern, Ember and Rose, each drawn deeper on light themes and brighter on dark ones.
- A toggle beside the presets that tints the app's backgrounds and panels with the chosen colour.
- The splash screen and the window icon are drawn in the chosen accent.
- The wordmark in the title bar turns and sweeps while a download, an install or a page load is running.
- Signing in to Minecraft from the getting started checklist asks which account to add: Microsoft or Ely.by.

### Changed
- The empty home screen says Noctrinth and shows the fork's own mark in the accent colour, instead of Modrinth's name and icon.
- Synced with upstream Modrinth (0.18.0 → 0.18.2): the pagination ellipsis jumps to a page, compact mode for instances in the library, a drag handle to resize "Jump in", icon editing from an instance's context menu, sorting by loader and game version, appearance and behaviour settings synced across devices, depends-on and included-content search filters, and fixes for installed state, "Jump in" cards, tab bars, a Discover memory leak and \`.mrpack\` exports.
- Noctrinth's translations moved to \`src/locales-noctrinth/\` and are applied over upstream's catalogues instead of being edited into them.
- The changelog is one file again; screenshots are still published with the changelog site rather than shipped in the installer.
- The in-app changelog is no longer fetched or cached at runtime — it is the one the build shipped with.

### Fixed
- The launcher could die on a cold start with "should be initialized when used", and open fine the next time.
- Signing in with an Ely.by account left the checklist's "Sign in to Minecraft" step outstanding.
- Closing the app panicked with "there is no reactor running" instead of exiting.
- DNS failures on Windows were logged as errors on every background poll.
- The right-hand sidebar, promo cards, dialog dimmer and the far end of the loading bar stayed purple whatever accent was chosen.
- The empty-server page's tiles and selected backups on the hosting page were drawn in Modrinth green.
- The launcher refused to start after the sync: upstream and Noctrinth had both shipped a migration numbered 20260818120000. Upstream's is renumbered, and a duplicate version now fails at startup naming both files, and in CI.
- Players signed in with a different account system than yours — an Ely.by player seen from a licensed client, or the other way about — stayed Steve on offline-mode servers.
- The by-name skin lookup did nothing at all on Minecraft 1.20.2 and newer, where the game asks for textures somewhere else.
- Names an offline server can hand out but Mojang would not issue, such as one with a dash in it, were never looked up.
- The app's icon files still carried the old mark, so the installer, the taskbar and the dock showed it.`,
	},
	{
		version: '0.18.0',
		date: '2026-08-19T00:00:00+00:00',
		body: `### Added
- Ely.by skins are managed inside the launcher: the account's catalogue is browsable in the same grid the Microsoft account uses, with preview, Reset and Apply.
![Ely Skins](/changelog/screenshots/0.18.0_elybyskin.png)
- Skins can be uploaded to Ely.by and removed from it without leaving the app.
- Skins for every player on offline-mode servers, looked up by name from Ely.by with a Mojang fallback. No mods needed; turn it off under Settings → Default instance options. [BETA]
- One-click modern Java for 1.7.10, installing [lwjgl3ify](https://github.com/GTNewHorizons/lwjgl3ify) or [Cleanroom](https://github.com/CleanroomMC/Cleanroom) along with the launcher-side patches. Disabling puts the instance back. [BETA]
- A graphics adapter picker for each Java runtime, on Windows, carried over when that runtime updates.
![Java Graphs](/changelog/screenshots/0.18.0_javagraphicsadapter.png)
- A Java runtime manager listing what is installed, with the option to remove it.
- Changelog entries render as markdown, so links and screenshots work.
- A notification for update downloads, carrying the version, how much has arrived and how large it is.

### Changed
- Synced with upstream Modrinth (0.17.10 → 0.18.0): a new Play page replacing Home and Library, an instance icon creator, a "Getting started" checklist, a clearer create-instance menu that can search projects, skin previews before signing in, "Managed content" renamed to "Provided content", and fixes for hooks and environment variables, instance icons, unlinked instance updates, the project gallery viewer, content-tab flickering and macOS LAN play.
![Lib](/changelog/screenshots/0.18.0_mainpage.png)
- Instance content search moved onto upstream's new library toolbar; the query language is unchanged and the content index is only read when a query needs it.
- The Java settings page matches the rest of the app, and the runtime selector is laid out as a column.
- The graphics adapter row is hidden on machines with one adapter instead of shown disabled.
- The fork's own strings speak Russian: modern Java, the Ely.by panel, the library search language, the graphics adapter picker and the Minecraft options screen.
- Reworked the Minecraft options screen: one value per slider, aligned controls, and version constraints written out in words.
- The instance Play button is brand-coloured, like the rows beside it in "Jump in".
- The version is written once, in the \`VERSION\` file at the root; a hand-edited copy fails CI.

### Fixed
- Applying an Ely.by skin appeared to hang and then opened the website; both applying and uploading now wait for the worn skin to change before assuming a missing session.
- The Ely.by panel waited for every texture before drawing anything; tiles appear immediately now.
- A dropped connection while polling the skin list raised a notification every time.
- An update whose files were not published yet was never retried until a restart.
- A network blip could write over a hundred \`ERROR\` lines carrying nothing but an OS error code.
- The keyboard selection in instance search suggestions was invisible on the OLED theme.
- A long server status line stretched its row in "Jump in" on longer translations.`,
	},
	{
		version: '0.17.10',
		date: '2026-08-15T00:00:00+00:00',
		body: `### Changed
- Synced with upstream Modrinth (0.17.8 → 0.17.10): pages failing to load while signed out.

### Fixed
- Sharing an instance reported "Unable to connect to shared instances API" whatever had gone wrong; the real reason is shown now.`,
	},
	{
		version: '0.17.8',
		date: '2026-08-15T00:00:00+00:00',
		body: `### Changed
- Synced with upstream Modrinth (0.17.7 → 0.17.8): Ears skins render correctly, \`.mrpack\` exports write the right environment values, and an instance linked to a deleted modpack version shows its managed content card again.

### Fixed
- Updating on Linux failed with "Permission denied (os error 13)"; Debian and RPM installs now update through the system's own privilege prompt.
- An update that cannot be installed is refused before it downloads, naming the directory, instead of failing after fetching it.

### Note for Linux
- If you installed Noctrinth from the \`.deb\`, this update cannot install itself. Install this release's \`.deb\` once by hand; updates after it work normally.`,
	},
	{
		version: '0.17.7',
		date: '2026-08-14T00:00:00+00:00',
		body: `### Added
- Search across every instance at once from the library: \`@sodium\` matches instances with a mod, \`#shader\` filters by content type, \`!outdated\` by state, terms combine, and a leading \`-\` flips one around.
- Suggestions as you type in that field: the sigils when empty, installed mods after \`@\`, types and states after \`#\` and \`!\`.
- A shared Minecraft options profile under Settings → Minecraft options, written into every instance's \`options.txt\` at launch, with per-option opt-in.
- "Copy link" in the overflow menu on project pages.

### Changed
- Synced with upstream Modrinth (0.17.6 → 0.17.7).
- Follow and Save left the project overflow menu; both have buttons in the header.

### Fixed
- The Save-to-collection panel opened in the top-left corner of the window, with a wrongly sized button.`,
	},
	{
		version: '0.17.6',
		date: '2026-08-14T00:00:00+00:00',
		body: `### Added
- Synced with upstream Modrinth (0.17.5 → 0.17.6): projects carry content disclosures — AI-generated content, ads, paid features, telemetry, derivative content and photosensitivity hazards — and can be filtered out when browsing.

### Changed
- The "Advanced" group of search filters is now "Advanced exclusions" and holds the new disclosure filters.
- Archived projects are marked by a disclosure rather than by their visibility.

### Fixed
- The "more options" menu on project pages opened empty.
- Adding a local file to an instance works for archives upstream's content inspection refuses: plain \`.jar\` and \`.litemod\` files without loader metadata, and packs zipped inside a wrapping folder.`,
	},
	{
		version: '0.17.5',
		date: '2026-08-14T00:00:00+00:00',
		body: `### Changed
- Synced with upstream Modrinth (0.17.4 → 0.17.5).
- An instance's content tab checks for updates when opened instead of waiting for a cached answer to expire.
- Config and RPO files are no longer flagged when importing a modpack from outside Modrinth, and the remaining warning explains itself better.
- Translations refreshed from upstream's Crowdin pull, without putting the Modrinth name back where Noctrinth had renamed it.

### Fixed
- Animated GIFs in project descriptions play again.
- Clicking an entry in an instance's content tab reported "Instance not found".
- The window close button is the right colour on Windows and Linux.
- Download buttons on version pages show the file name and size again.
- The buttons on the "Minecraft account required" dialog work on Windows.
- The right sidebar stays open when collapsing a friends-list section with "Hide right sidebar" on.`,
	},
	{
		version: '0.17.4',
		date: '2026-08-10T00:00:00+00:00',
		body: `### Added
- Modrinth App migration: a banner on the home page imports all of its instances or a hand-picked few, read straight out of its database, with an option to remove each one from Modrinth App once its copy has finished.
- CurseForge modpack \`.zip\` import from disk is back, on the same job pipeline as everything else. Search, browse and project pages stay removed.
- A link to the fork's author on the Changelog settings tab.
- Environment variables for an instance's pre- and post-launch hooks (from upstream).

### Changed
- Synced with upstream Modrinth (0.17.3 → 0.17.4): buttons rebuilt on a new shared component set, an "Inspecting modpack" progress bar on file import, friends lists collapsing offline members by default, better popout animations and noticeably lower memory use.
- The top-left wordmark dropped "app", matching Modrinth's own rebrand.
- The "Beta" badge on the Language settings tab is purple instead of green.
- Byte-size units (KiB/MiB/GiB/TiB) are shown in English.
- The instance Screenshots tab was rebuilt to match the project Gallery, with zoom, copy, reveal and delete in one floating control bar.
- The embedded Ely.by skin window hides instead of closing, so a sign-in lasts the session.
- Russian translations refreshed from upstream's Crowdin pull.

### Fixed
- Install links did nothing: they said \`modrinth://\` while the app answers to \`noctrinth://\`, and its own parser rejected \`noctrinth://\`.
- Install links on modrinth.com open Noctrinth: the app claims \`modrinth://\` alongside its own scheme on every launch.
- Ctrl+C and Ctrl+A failed in instance logs and consoles on non-Latin keyboard layouts.
- Uploading a profile picture rejected anything over 256 KiB; oversized images are scaled and compressed to fit.
- The instance Screenshots tab errored with "Cannot read properties of undefined".
- The account menu had a blank row above "Sign out", and Collections and Notifications had no accessible label.
- Upstream fixes from the sync: editing your profile in-app, stale Microsoft tokens signing the account out, \`.mrpack\` exports past 4 GB, export modal paths, \`.nbt\` config files in shared instances, and the version list freezing on game-version toggles.`,
	},
	{
		version: '0.17.3',
		date: '2026-07-31T00:00:00+00:00',
		body: `### Added
- Synced with upstream Modrinth (0.15.11 → 0.17.x): user profile pages in the app, shared instances with invites and updates, an instance Share tab, settings grouped into Display / Account / Instances, a new breadcrumbs system, a rebuilt project page header, instance quarantine handling, a toggle to hide installed modpacks, and a fix for search jumping back to page 1.

### Changed
- Upstream is the source of truth: the fork's custom Modrinth OAuth flow, browse page-reset patch and feature-flag redesign gave way to upstream's.
- The proxy setting and Noctrinth branding were carried into upstream's new settings layout; the changelog now lives under Display.

### Removed
- CurseForge integration: search, installs, project pages, the catalog toggle, the fingerprint and mapping helpers and the backend installer. Importing a CurseForge instance from another launcher still works.
- Modrinth's ads stay disabled, including the new consent popup and Modrinth+ upsell.

### Fixed
- The Screenshots tab listed nothing, passing an instance path where the backend expects an id.
- A byte-order mark in a shared UI component and a locale file, and a dead OAuth loopback listener.`,
	},
	{
		version: '0.15.11',
		date: '2026-07-15T00:00:00+00:00',
		body: `### Changed
- Synced with upstream Modrinth (0.15.10 → 0.15.11): malware warning modal changes, shift-click to toggle file selection, shader configs renamed on version change, dependents search in Discover, download modal fixes and new moderation keybinds.
- The Ely.by sign-in dialog matches the app's standard modals.
- The changelog tab uses the standard chip selector, and each source's "open full changelog" link points at the right site.
- The screenshot counter is plain text instead of a pill badge.

### Fixed
- Local files failed to add with "Unable to infer project type": plain \`.jar\` files without loader metadata are accepted as mods, packs zipped inside a wrapping folder are recognised, and filenames Windows forbids are sanitised.`,
	},
	{
		version: '0.15.10',
		date: '2026-07-14T00:00:00+00:00',
		body: `### Added
- Two-factor authentication (TOTP) for Ely.by sign-in.
- Failed required CurseForge dependencies are reported after an install instead of being skipped silently.
- The Versions tab of CurseForge project pages pages through the full file list instead of the 50 newest.

### Changed
- CurseForge modpack installs run through the launcher's install-job pipeline: queued, cancellable, retryable, with live per-file progress, parallel downloads and one error listing every failure.
- Synced with upstream Modrinth (0.15.1 → 0.15.10): an advanced filter category on Discover, redesigned version pages, download modal and modpack export modal, the "Chaos Cubed" skin pack, better install error handling with a "Copy details" button and a three-job queue, and connect and read timeouts on all launcher HTTP requests.
- Upstream fixes from the sync: freezes when opening instance pages, instance edits not appearing saved, content desync on enabling or removing mods, a 10-minute search cache, environment filters in Discover, links keeping track of the instance you came from, and a Files tab memory leak.
- "Update all" for CurseForge checks several mods at a time and skips disabled files.
- CurseForge install notifications on project pages are translated.
- Noctrinth's CurseForge search filters, install pipeline and proxy support were re-integrated on upstream's reworked search and version page.

### Fixed
- Importing a CurseForge modpack from a local \`.zip\` reported "no CurseForge API key is available" with a key configured, and the preview now shows the pack's real name, version and loader.
- A flaky connection or an Ely.by outage no longer signs you out; credentials are dropped only when both the token and its refresh are rejected.
- Updating a CurseForge mod deleted the old file before the new one had downloaded.
- Install buttons were disabled for files whose author hid the download URL.
- Links inside CurseForge descriptions pointing to other CurseForge pages opened a dead app-origin URL.
- The Oculus entry in the cross-platform mapping used a slug instead of a Modrinth project id.`,
	},
	{
		version: '0.15.1',
		date: '2026-06-27T00:00:00+00:00',
		body: `### Added
- CurseForge modpack installation on the new 0.15 content pipeline: the instance is created from the manifest, files are resolved and downloaded with a CDN fallback, overrides applied and Minecraft installed.
- A manual download fallback for files whose author disabled third-party distribution: an embedded CurseForge window where one download is captured and imported automatically.

### Changed
- Synced with upstream Modrinth (0.14.8 → 0.15.1): the backend was rewritten, instances and their content are managed through a new content pipeline, the old "profile" model is gone, and installs run as background jobs.
- Ely.by account support was re-ported onto the new backend.
- CurseForge files are tracked natively in the launcher database, so installed state and updates survive restarts and backups.
- A required CurseForge dependency that also exists on Modrinth is installed from Modrinth.
- "Update all" is provider-aware: CurseForge content updates through CurseForge, Modrinth content through Modrinth.

### Fixed
- Discover jumped back to the first page when installing a mod with "hide installed" on.
- CurseForge installs crashed with "Unknown instance" from every entry point.
- The CurseForge version and loader filter was ignored when browsing inside an instance.`,
	},
	{
		version: '0.14.8',
		date: '2026-06-20T00:00:00+00:00',
		body: `### Changed
- CurseForge search is off by default; turn it on in Settings → Appearance.
- Synced with upstream Modrinth (0.14.6 → 0.14.8): reworked content dependency handling, more reliable update matching, game-version search fixes, Maven version-range matching, modpack export filtering, Babric mods no longer misdetected as Fabric, the mobile floating action bar, comboboxes closing on scrollbar clicks, and billing and analytics fixes.

### Fixed
- CurseForge browse failed on later pages; the page size is clamped to the 10,000-result window.
- Switching catalogs resets to the first page instead of landing on a blank one.
- CurseForge project pages opened from Discover keep a working breadcrumb.
- Restored the Pride Fundraiser banner.
- Russian translations for CurseForge pages and the remaining settings, instance, world and friend-request strings.`,
	},
	{
		version: '0.14.6',
		date: '2026-06-12T00:00:00+00:00',
		body: `### Added
- Ely.by skin management in the app: "Change skin" opens an embedded Ely.by window, and the preview shows the account's cape, detects slim or classic arms and refreshes on demand.
- A proxy setting in Settings → Resource management, routing all launcher traffic through one \`http\`, \`https\`, \`socks5\` or \`socks5h\` URL. Applies after a restart.
- A CurseForge master toggle in Settings → Appearance covering the catalog toggle, search results and project pages.

### Changed
- Modrinth and CurseForge are fully separate catalogs: the toggle picks exactly one, including for text searches, and the sidebar shows only filters CurseForge supports.
- CurseForge downloads are verified against SHA1 checksums, retried, and given a CDN mirror fallback; modpack installs no longer skip such files silently.
- Synced with upstream Modrinth (0.14.5 → 0.14.6): drag-and-drop skin reordering, draggable Mojang defaults, skin outer-layer translucency, offline handling on the Skins page, live system-theme updates, new Microsoft sign-in error cases, and fixes for notifications over modals and skin editing.

### Fixed
- The Ely.by skin preview hung on "Loading", and the account picker avatar did not update after a skin change.
- Russian translations for the new settings and the Ely.by skin page.
- Startup logged a spurious "User is not logged in" error with an Ely.by account active.`,
	},
	{
		version: '0.14.5',
		date: '2026-06-09T00:00:00+00:00',
		body: `### Changed
- Synced with upstream Modrinth (0.14.3 → 0.14.5): a "release channels" instance setting, a reworked updater UI with the action in the top-right bar and pop-ups only after 24 hours, uncommon plugin loaders behind "Show more", and analytics and translation updates.
- The update pop-up's "Changelog" button opens the Noctrinth GitHub page.
- Changelog website timestamps recalculate on every visit instead of being frozen at build time, with the exact date on hover.

### Fixed
- Drag-and-drop file upload in the instance Files tab.
- The expanded console in the Logs tab rendered behind other elements.
- The updater restarted the app when no restart was requested.`,
	},
	{
		version: '0.14.3',
		date: '2026-06-07T00:00:00+00:00',
		body: `### Added
- A Screenshots tab for instances: browse, zoom, copy, reveal in explorer and delete.

### Changed
- Synced with upstream Modrinth (0.14.1 → 0.14.3): a play-time toggle in the instance header, the Mr. Pack default skin, the Pride Fundraiser banner, tooltip and card styling, a large \`.mrpack\` import crash fix, a content-update modal fix, a Windows path separator fix in the Files tab, and translation updates.
- The update popup's changelog button links to the Noctrinth GitHub page.
- Download notification icons match Noctrinth's purple.

### Fixed
- Russian locale updated with the missing skin selector, collections, notifications and screenshots strings.`,
	},
	{
		version: '0.14.0',
		date: '2026-05-29T00:00:00+00:00',
		body: `### Added
- CurseForge install detection through the fingerprint API, catching mods dropped into the folder by hand, installed before this version, or restored from a backup.
- A cross-platform mapping of Modrinth ↔ CurseForge ids, so a mod on both platforms is one card in unified search and installing one marks the other installed.
- A CurseForge dependency resolver: embedded and tool dependencies are skipped, incompatible mods warned about, optional ones offered rather than installed, and required ones kept only when every ancestor was required too.
- CurseForge categories for the shader, resource pack and datapack tabs.
- Clicking the "…" in pagination turns it into a page input.
- A CurseForge "Alpha" badge on the source toggle.
- Russian translations for the account dropdown, Collections, Notifications, sidebar tooltips, the Ely.by login modal, the Changelog tab and the dashboard nav.
- Search and sort for the Followed-projects view and collections, remembered across reloads.

### Changed
- Synced with upstream Modrinth 0.14.0: the new skin selector out of beta, skin preview improvements, project analytics performance, Daedalus manifest uploads and CSP changes.
- The skin preview tilts around its pitch axis as well as yaw, and the shadow follows the model.
- The CurseForge content sidebar uses a curated Modrinth-style category list.
- Compact number suffixes are Latin (\`k\`, \`m\`, \`b\`) while the numbers stay locale-correct.

### Fixed
- Importing an unrecognised modpack file deleted the instance; the error now comes before anything is created.
- Modpacks shipping filenames Windows forbids crashed the install with \`ERROR_INVALID_NAME\`.
- CurseForge categories were swapped.
- Vanilla instances can install CurseForge resource packs, shaders and datapacks; informational tags were being read as loaders.`,
	},
	{
		version: '0.13.24',
		date: '2026-05-26T00:00:00+00:00',
		body: `### Changed
- Synced with upstream Modrinth (0.13.22 → 0.13.24): default Minecraft memory raised from 2 GB to 4 GB, an improved Java installation UI, an "Enabled" sort button in content lists, compact log handling to survive log spam, and locale updates.

### Fixed
- Non-string values when decorating download URLs.
- Pages failed to load when an auth cookie was invalid.`,
	},
	{
		version: '0.13.21',
		date: '2026-05-22T00:00:00+00:00',
		body: `### Added
- CurseForge in Discover: search spans both catalogs, merged and de-duplicated, with a logo on each card showing where a mod is hosted.
- A catalog source toggle in the controls bar, remembered across navigation and restarts.
- CurseForge project pages reusing Modrinth's layout, including the Versions and Gallery tabs.
- Installing CurseForge mods into an existing instance or through the instance picker.
- Installing CurseForge modpacks from Discover, and importing them from a \`.zip\`.
- An optional setting recolouring the Discover accent to match the active catalog. Off by default.

### Changed
- Synced with upstream Modrinth (0.13.21): Kyros upload sessions for hosting, project analytics events and moderation queue tooling.`,
	},
	{
		version: '0.13.20',
		date: '2026-05-21T00:00:00+00:00',
		body: `### Changed
- Synced with upstream Modrinth (0.13.20): content management improvements, a new date picker, Intercom bubble positioning, macOS window occlusion checks and routing fixes.`,
	},
	{
		version: '0.13.19',
		date: '2026-05-20T00:00:00+00:00',
		body: `### Changed
- Synced with upstream Modrinth (0.13.19).

### Fixed
- The right sidebar slides in and out with a transform instead of animating the grid column, so content no longer reflows while it moves.`,
	},
	{
		version: '0.13.18',
		date: '2026-05-18T00:00:00+00:00',
		body: `### Changed
- Synced with upstream Modrinth (0.13.18).
- Discord Rich Presence runs on Noctrinth's own Discord application and artwork.

### Fixed
- Signing in with an Ely.by account and launching the game with it work reliably.`,
	},
	{
		version: '0.13.17',
		date: '2026-05-17T00:00:00+00:00',
		body: `### Added
- Ely.by as a second account provider, authenticating against its Yggdrasil server.
- Launching Minecraft with Ely.by accounts, through the authlib-injector Java agent.
- A Collections section for browsing, creating, editing and deleting Modrinth collections.
- A "Followed projects" view.
- A "Save to collection" button on project pages.
- An in-app Notifications page.
- Native desktop notifications for downloads and updates.
- A Changelog tab in settings showing both the Noctrinth and Modrinth App changelogs.
- Signed application updates delivered through GitHub Releases.

### Changed
- Rebranded to Noctrinth: new logo, generated app icons and the \`com.noctrinth.app\` bundle identifier.
- The embedded sign-in WebView was replaced with a loopback HTTP redirect for the Modrinth OAuth flow.
- Exactly one account is active across the Microsoft and Ely.by providers.
- Recoloured the interface to a purple brand scheme.
- The Feature Flags settings show readable names and descriptions instead of raw keys.
- The sidebar slides in and out when toggled.
- Extended the requested Modrinth OAuth scopes.
`,
	},
]
