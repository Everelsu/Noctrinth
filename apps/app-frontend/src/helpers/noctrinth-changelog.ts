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
 * Links work, and so do screenshots:
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
- Accent presets, under Settings → Appearance, chosen the way a theme is. A theme fixes its accent at one colour, and the accent is most of where the interface's character comes from — so there are nine to pick from: the theme's own, Amethyst, Nightshade, Midnight, Glacier, Verdant, Lantern, Ember and Rose. Each carries two values, a deeper one for light themes and a brighter one for dark, and the whole app repaints the moment one is picked. The background goes with it, unless you would rather it did not: every surface the app is built out of takes the same hue at a fraction of the strength, keeping the lightness the theme gave it, so Ember is a warm dark app rather than a violet one with orange buttons. The right-hand sidebar and the promo cards were the one part that never followed the accent at all — the fork had written their gradients out in purple — and they follow it now too. A toggle beside the presets drops the backgrounds back to the theme's own surfaces — plain and uncoloured, rather than the purple they were written as — leaving only the accent recoloured. The loading bar follows the preset either way; its far end had been a fixed lilac, so it used to leave the accent behind halfway across and finish in purple whatever colour it started.
- The wordmark says when the app is busy. The launcher has always counted what it was waiting on, but the only thing reading that count was a progress bar it keeps switched off, so a mod arriving in the background — or a page that took a moment — looked like a click that had not registered. The knot turns and a light in the accent colour sweeps the lettering while a download or install is running, and for a page load too, held for one full turn since those finish faster than the eye does. It stays still for anyone who has asked for less motion.

### Fixed
- The hosting page was drawn in Modrinth green: the tiles on the empty-server page and the border on a selected backup both named the green ramp outright. They are mixed from the accent now, so they follow the theme — and the intensity above. The greens that mean something, like a finished operation, are left alone.
- The launcher refused to start after this sync with "migration 20260818120000 was previously applied but has been modified". Upstream's 0.18.1 added a migration carrying the same version as one Noctrinth shipped in 0.18.0, and sqlx, which keys applied migrations by version alone, compared the stored checksum against the wrong file. Upstream's is renumbered, and the launcher now checks for a collision before it migrates anything and names both files instead of a number — with a test over the shipped set, so the next one fails in CI rather than on someone's machine a release later.

### Changed
- Synced with upstream Modrinth (0.18.0 → 0.18.2). The ellipsis in pagination controls is a button now, and asks which page to jump to; instances have a compact mode in the library; the "Jump in" row can be resized by dragging its handle; an instance's icon can be edited from its context menu; sorting by loader and game version is back; and appearance and behaviour settings can follow a Modrinth account across devices. Two search filters were added — projects that depend on a given project, and modpacks that include one. Their fixes: projects that are not installed showing as installed, the missing signal icon and player count on "Jump in" cards, several tab bar problems, and a memory leak on the Discover page. 0.18.2 itself carries a single fix, for an \`.mrpack\` export replacing the original file with one from a version matched earlier by hash.
- The changelog is a single file again. Splitting each release into its own \`src/changelog/<version>.md\` bought nothing the file did not already do, and a release turned into an errand across a folder, a build script and a published feed. Entries are written in \`helpers/noctrinth-changelog.ts\` as they were before, and the one thing that stayed is screenshots — still in their own folder, still published with the changelog site rather than carried inside every installer.
- The launcher no longer fetches the changelog while you are reading it, and no longer keeps a copy of one in local storage. What it shows is what it shipped with, which is the changelog for the version you are actually running.
- Noctrinth's translations moved out of upstream's catalogues into \`src/locales-noctrinth/\`. Every string the fork had written or reworded sat inside the files Crowdin rewrites wholesale, so each upstream sync conflicted on all of them, in every language at once. They are applied over upstream's now, and a second file per locale holds translations that stand in for upstream's where Crowdin has none yet — which upstream wins back as soon as it does.`,
	},
	{
		version: '0.18.0',
		date: '2026-08-19T00:00:00+00:00',
		body: `### Added

- Ely.by skins are managed from inside the launcher now. The panel used to offer one thing — "open the website" — because Ely.by publishes no skin API: its OAuth scopes cover the account and its email, and the skin system is read-only. The account's catalogue is drawn in the same grid the Microsoft account uses, so tiles are rendered previews rather than raw texture sheets, and picking one previews it on the model with Reset and Apply, exactly like the rest of the page. Browsing costs nothing and nothing reaches the account until Apply. 
![Ely Skins](/changelog/screenshots/0.18.0_elybyskin.png)
- Adding a skin to Ely.by from the launcher: pick a PNG or drop it on the grid, and it is uploaded to the account's catalogue and worn in one step. Skins can be removed from the catalogue too. The catalogue remains the only place Ely.by keeps skins, so an upload is still required before wearing — the same thing the website does.
- Skins for every player on offline-mode servers, without a single mod. Those servers hand out profiles with no textures at all and the game asks nobody about it, which is why everyone there is S-T-E-V-E or some others basics skins — the launcher now fills those profiles in by name, from Ely.by, which answers for the players registered there and falls back to Mojang for everyone else. So a licensed player and a player without an account both show up as themselves, in vanilla as much as anywhere. It never touches a skin the server did send, and it can be turned off under Settings → Default instance options.[BETA]
- One-click modern Java for legacy versions, marked beta. Instances on 1.7.10 are pinned to Java 8 by Mojang's own manifest and by LWJGL 2; enabling it from the instance's Java settings lifts both, bringing the launcher-side version patches and the mod half in one step, and picking the Java release to run on. [lwjgl3ify](https://github.com/GTNewHorizons/lwjgl3ify) keeps Forge and swaps LWJGL, [Cleanroom](https://github.com/CleanroomMC/Cleanroom) replaces it outright. Disabling puts the instance back as it was, leaving any mods it installed for the Mods tab to remove.[BETA too, im bot so sure about stability especially for 1.12.2]
- A graphics adapter picker for each Java runtime, on Windows. A laptop with two adapters leaves the choice to Windows, which usually means the integrated one — the slow half of the machine for Minecraft. The preference is set per executable, says in words what Windows will do with it, and is carried over when that runtime is updated.
![Java Graphs](/changelog/screenshots/0.18.0_javagraphicsadapter.png)
- A Java runtime manager, listing what the launcher has installed and letting go of what is no longer wanted.
- The in-app changelog keeps itself current. Entries are published alongside the changelog site, and an installed launcher overlays them on what it shipped with — so a release written after your build still shows up, and a correction does not need a release of its own. Entries render as markdown now, meaning links work and screenshots are possible; screenshots live on the site rather than inside the installer.
- Update downloads report themselves while they run. The update pill only appeared once the download had finished, so a download starting quietly in the background was invisible — including the bandwidth it was spending. There is now a notification carrying the version, how much of it has arrived, and how large it is.

### Fixed

- Applying an Ely.by skin appeared to hang and then threw the website in your face. The check ran two seconds after the request while the site takes four to eight, so it always concluded nothing had happened. Both applying and uploading now wait for the worn skin to actually change, up to twenty-five seconds, and only open the sign-in window when it never does — which is what a missing session really looks like.
- Opening the Ely.by panel waited for every texture in the catalogue to download before drawing anything. Tiles appear immediately and fill in as textures land.
- A dropped connection while the skin list was polling raised a notification every time. It retries once and otherwise fails quietly, leaving the grid with what it already had.
- An update whose files were not published yet never arrived. The release answers the manifest but refuses the download, and that stuck: the version was already known, so every later check took the "already known" branch and the download only happened after a restart, long after the release went public. A failed attempt is remembered and retried. The check loop is also wrapped, since one thrown error used to take the five-minute schedule down with it until restart.
- The launcher log was drowning in errors that weren't. A twenty-second network blip could write over a hundred identical \`ERROR\` lines carrying nothing but an OS error code, because every error handed to the interface was logged as a fault on its way there.
- The keyboard selection in the instance search suggestions is visible again. It was drawn a few percent brighter than the panel behind it, which on the OLED theme was no difference at all.
- A long server status line stretched its row in Jump in, on translations where the string runs longer than English. It is clamped like the MOTD beside it.

### Changed

- Synced with upstream Modrinth (0.17.10 → 0.18.0). A new Play page replaces Home and Library, with an icon creator for instances without their own art and a "Getting started" checklist; the create-instance menu explains itself better and can search projects from inside the menu; the skin selector can be previewed before signing in; "Managed content" is now "Provided content". Their fixes: hooks and environment variables not saving, the instance icon being deleted after installing a server project's shared content, unlinked instances not checking for updates, the gallery viewer on project pages, flickering on the content tab at certain window widths, and an attempt at the macOS bug that blocked LAN play.
![Lib](/changelog/screenshots/0.18.0_mainpage.png)
- The instance content search moved onto upstream's new library toolbar, which replaced the page it used to live on. The query language is unchanged, and the content index is now read only when a query actually needs it — a plain name search costs nothing.
- The Java settings page no longer looks like a different app. Each runtime sits in a card with the same icon and heading weight the instance Java settings use, and the runtime selector is laid out as the column it always rendered as, instead of a row that pushed its buttons onto a second line and left them stranded.
- The graphics adapter row is hidden on machines with only one adapter, rather than shown and disabled. A disabled control explaining that there is nothing to choose is worse than no control.
- The fork's own strings speak Russian: modern Java, the Ely.by panel, the library search query language, the graphics adapter picker and the whole Minecraft options screen — which had been English down to the last option name, group heading and unit — all of which had been falling back to English. The version line in settings also stopped saying "Modrinth App" in translations that had never been updated.
- Reworked the Minecraft options screen. Each slider was showing its value three times over, controls ended wherever their own width put them, and a bare "1.16+" chip sat next to a name without saying what it referred to — that constraint is now written out in words, and only on the two options that genuinely need it.
- The instance Play button is brand-coloured, like the world and server rows it sits beside in Jump in; it was the only green one there.
- The version is written in one place. It lived by hand in the frontend's package.json and was copied into the Rust crates by the release workflow, which left local builds reporting a placeholder; everything now mirrors the \`VERSION\` file at the root, and a hand-edited copy fails CI.`,
	},
	{
		version: '0.17.10',
		date: '2026-08-15T00:00:00+00:00',
		body: `### Fixed
- Sharing an instance reported "Unable to connect to shared instances API" whatever had actually gone wrong — a dropped connection, a server fault and a malformed reply all looked the same, and the real message was thrown away. The reason is now shown, and a failed request spells out what happened to the connection instead of stopping at "error sending request".

### Changed
- Synced with upstream Modrinth (0.17.8 → 0.17.10). Upstream published no app changelog for 0.17.10 itself: 0.17.9 pulled a server-panel change that wasn't meant to ship yet, and 0.17.10 put it back. The only other change is a fix for pages failing to load while signed out.`,
	},
	{
		version: '0.17.8',
		date: '2026-08-15T00:00:00+00:00',
		body: `### Fixed
- Updating on Linux failed with "Permission denied (os error 13)". The bundler was five versions behind the Tauri runtime and never stamped each bundle with its own type, so the launcher could not tell a \`.deb\` install from an AppImage and always fell back to overwriting its own executable — which a package-managed install under \`/usr\` will never allow. Debian and RPM installs now update through the system's own privilege prompt.
- An update that cannot be installed is now refused before it downloads, naming the directory and what to do about it, instead of failing with a bare error code after fetching the whole thing.

### Changed
- Synced with upstream Modrinth (0.17.7 → 0.17.8): Ears skins render correctly, \`.mrpack\` exports no longer write the wrong environment values, and an instance linked to a modpack version that was deleted from Modrinth shows its managed content card again.

### Note for Linux
- If you installed Noctrinth from the \`.deb\`, this update cannot install itself — the version you are running is the one with the bug. Install this release's \`.deb\` once by hand; updates after it work normally.`,
	},
	{
		version: '0.17.7',
		date: '2026-08-14T00:00:00+00:00',
		body: `### Added
- Search across every instance at once from the library. A bare word still matches instance names; \`@sodium\` finds instances that have a mod, \`#shader\` filters by content type, and \`!outdated\` by state. Terms combine, and a leading \`-\` flips one around. Matches are shown on the instance card, so "which version, and is it stale" is answered without opening anything.
- The search field suggests as you type — the sigils when it's empty, your own installed mods after \`@\`, and the available types and states after \`#\` and \`!\`.
- A shared Minecraft options profile, under Settings → Minecraft options. Field of view, render distance, volumes, sensitivity and the rest can be set once and written into every instance's \`options.txt\` at launch, with per-option opt-in.
- "Copy link" in the overflow menu on project pages.

### Changed
- Synced with upstream Modrinth (0.17.6 → 0.17.7).
- The Follow and Save entries left the project overflow menu; both already have buttons in the header beside it.

### Fixed
- The Save-to-collection panel opened pinned to the top-left corner of the window, and its button was the wrong size, after upstream's button refactor moved a template ref onto a component.`,
	},
	{
		version: '0.17.6',
		date: '2026-08-14T00:00:00+00:00',
		body: `### Added
- Synced with upstream Modrinth (0.17.5 → 0.17.6). Projects now carry content disclosures — authors state whether a project contains AI-generated content, ads or sponsorships, paid features, telemetry, derivative content or photosensitivity hazards, and you can filter them out when browsing.

### Changed
- The "Advanced" group of search filters is now "Advanced exclusions" and holds the new disclosure filters.
- Archived projects are marked by a disclosure rather than by their visibility, so an archived project can also be unlisted or private.

### Fixed
- The "more options" menu on a project page opened empty. Adding the Follow and Save buttons to that header had dropped an icon import the menu itself still used, so building its list of entries threw and every entry vanished — leaving the bare panel.
- Adding a local file to an instance keeps working for archives upstream's new content inspection refuses: plain \`.jar\` and \`.litemod\` files carrying no loader metadata, and packs zipped inside a wrapping folder.`,
	},
	{
		version: '0.17.5',
		date: '2026-08-14T00:00:00+00:00',
		body: `### Changed
- Synced with upstream Modrinth (0.17.4 → 0.17.5).
- An instance's content tab now checks for content updates as soon as you open it, instead of waiting for a cached answer to expire. A freshly published update can still take up to ten minutes to show up.
- Config files and RPO files are no longer flagged when importing a modpack from outside Modrinth, and the warning about the files that are still flagged explains itself better.
- Translations were refreshed from upstream's latest Crowdin pull, without putting the Modrinth name back into the places Noctrinth had renamed.

### Fixed
- Animated GIFs in project descriptions play again.
- Clicking an entry in an instance's content tab no longer reports "Instance not found".
- The window close button is the right colour again on Windows and Linux.
- Download buttons on version pages show the file name and size again.
- The buttons on the "Minecraft account required" dialog work on Windows.
- The right sidebar stays open when you collapse a section of the friends list with "Hide right sidebar" on.`,
	},
	{
		version: '0.17.4',
		date: '2026-08-10T00:00:00+00:00',
		body: `### Added
- Modrinth App migration. A banner on the home page detects an existing Modrinth App install and offers to import all of its instances, or hand-pick which ones — reading instance metadata straight out of Modrinth App's own database. Modrinth App also appears as its own entry in the regular "Import instance" flow. Either way, an optional checkbox removes each instance from Modrinth App once its copy has safely finished importing.
- CurseForge modpack \`.zip\` import from disk is back — install from a local file, same job pipeline (queueing, progress, retry, rollback) as everything else. Search, browse and project pages stay removed.
- A link to the fork's author on the Changelog settings tab.
- Environment variables can now be passed to an instance's pre- and post-launch hooks (from upstream).

### Changed
- The top-left wordmark dropped "app" from the logo, matching Modrinth's own rebrand.
- The "Beta" badge on the Language settings tab is purple instead of green, matching the rest of Noctrinth's palette.
- Byte-size units (KiB/MiB/GiB/TiB) are shown in English instead of transliterated Russian, matching the units players already know.
- The instance Screenshots tab was rebuilt to match the project Gallery's grid and image viewer, with screenshot-only actions (zoom, copy, reveal in folder, delete) folded into the same floating control bar. Opening the screenshots folder no longer waits for a fresh disk lookup on every click.
- The embedded Ely.by skin window now hides instead of closing, so a sign-in there is remembered for the rest of the session instead of only the first open.
- Synced with upstream Modrinth (0.17.3 → 0.17.4). Buttons across the whole app were rebuilt on a new shared component set — Noctrinth's own screens (Collections, Notifications, Screenshots, the Ely.by sign-in dialog and account card) were moved onto it too, so they match everything else again. Importing a modpack from a file now shows an "Inspecting modpack" progress bar instead of appearing to hang, friends lists collapse offline members by default (with the choice remembered), popout menus animate better, and the launcher uses noticeably less memory.
- Russian translations were refreshed from upstream's latest Crowdin pull, without putting the Modrinth name back into the places Noctrinth had renamed.

### Fixed
- Install links did nothing. Noctrinth answers to \`noctrinth://\`, but every link meant to open it said \`modrinth://\` — so a click went to Modrinth App or nowhere at all — while the launcher's own URL parser had the mirror-image bug and rejected \`noctrinth://\`. Our own links (install, open server, shared-instance invite) and the desktop shortcuts the app writes for instances now use Noctrinth's scheme.
- Install links on modrinth.com now open Noctrinth. The app registers \`modrinth://\` alongside its own scheme and reclaims both on every launch, so "Install with Modrinth App" over there installs here. If Modrinth App is installed too, the two share that scheme and whichever started last wins it.
- Ctrl+C failed to copy text out of instance logs and the terminal-style consoles on a non-Latin keyboard layout (e.g. Russian), because the shortcut was matched against the typed character instead of the physical key. Ctrl+A (select all) had the same problem, and on Windows/Linux didn't work at all.
- Uploading a profile picture in Settings → Profile rejected anything over 256 KiB with a plain browser alert — the exact limit Modrinth's server enforces, so almost any real photo failed. Oversized images are now automatically scaled and compressed to fit, and a failed save now shows the actual error instead of a generic message.
- The instance Screenshots tab errored out with "Cannot read properties of undefined". Upstream moved instance subpages off props and onto a shared page context, and the tab was still reading a prop nobody passes any more.
- The account menu had a blank row above "Sign out" — its divider was still written in the shape the old menu component understood, so the new one rendered it as an empty entry. Collections and Notifications also had no accessible label there.
- Upstream fixes brought in by the sync: editing your profile from inside the app works again, a stale Microsoft token no longer signs the account out, exporting a large instance as an \`.mrpack\` no longer breaks past 4 GB, the export modal handles paths correctly, shared instances accept \`.nbt\` config files, and the version list no longer freezes the page when toggling game versions.`,
	},
	{
		version: '0.17.3',
		date: '2026-07-31T00:00:00+00:00',
		body: `### Added
- Synced with upstream Modrinth (0.15.11 → 0.17.x). Highlights from upstream: user profile pages inside the app, shared instances with invites and updates, an instance Share tab, a reworked settings layout grouped into Display / Account / Instances, a new breadcrumbs system, a rebuilt project page header, instance quarantine handling, a toggle to hide already-installed modpacks, and a fix for search jumping back to page 1.

### Changed
- Upstream is now the source of truth. Where Modrinth had shipped its own version of something Noctrinth carried, the fork's variant was dropped: the custom Modrinth OAuth flow, the browse page-reset patch, and the feature-flag settings redesign all gave way to upstream's.
- The proxy setting and Noctrinth branding were carried over into upstream's new settings layout; the in-app changelog now lives under Display.

### Removed
- CurseForge integration. Search, installs, project pages, the catalog toggle, the fingerprint/mapping helpers and the backend installer are all gone. Importing a CurseForge instance from another launcher still works — that has always been an upstream feature.
- Modrinth's ads stay disabled in Noctrinth, including the new consent popup and Modrinth+ upsell that upstream added.

### Fixed
- The Screenshots tab failed to list anything. It was passing an instance path where the backend expects an instance id after upstream's profile-to-instance rewrite, so every load errored with "Unknown instance".
- A byte-order mark had crept into a shared UI component and a locale file, and a dead OAuth loopback listener was left behind by the reverted sign-in rework.`,
	},
	{
		version: '0.15.11',
		date: '2026-07-15T00:00:00+00:00',
		body: `### Changed
- Synced with upstream Modrinth (0.15.10 → 0.15.11): malware warning modal changes, shift-click to toggle file selection in the Files tab, shader config files renamed on version change, dependents search in Discover, download modal fixes, and new moderation keybinds.
- The Ely.by sign-in dialog was redesigned to match the app's standard modals — regular titled dialog instead of the custom gradient header with a placeholder avatar.
- The changelog settings tab now uses the app's standard chip selector for switching between Noctrinth and Modrinth changelogs, and the "open full changelog" link now points at the right site for each source (it previously opened the Noctrinth site while describing the Modrinth one).
- The screenshot counter on the Screenshots tab is now plain text instead of a pill badge, matching the rest of the app.

### Fixed
- Fixed local files failing to add to an instance with "Unable to infer project type": plain .jar files without loader metadata (common for legacy tweaker/coremod jars with names like "!mixinbootstrap.jar") are now accepted as mods, packs zipped inside a wrapping folder are recognised, and filenames with characters Windows forbids are sanitised on write.`,
	},
	{
		version: '0.15.10',
		date: '2026-07-14T00:00:00+00:00',
		body: `### Added
- Two-factor authentication (TOTP) support for Ely.by sign-in. When an account is protected with 2FA, the login dialog now asks for the 6-digit code instead of failing with an error.
- Failed required CurseForge dependencies are now reported after an install ("N required dependencies could not be installed") instead of being silently skipped.
- The Versions tab of CurseForge project pages now pages through the full file list (previously only the 50 newest files were shown).

### Changed
- CurseForge modpack installs were reworked to run through the launcher's install-job pipeline — the same one Modrinth packs use. Installs now queue, show live per-file download progress in the action bar, can be cancelled and retried, and roll the instance back cleanly on failure. Modpack files download several at a time instead of one-by-one, and when files fail you get one error listing all of them rather than dying on the first.
- Synced with upstream Modrinth (0.15.1 → 0.15.10). Highlights: a new advanced filter category on the Discover page with options to exclude other content types from mod and data pack search; redesigned version pages and project download modal; a redesigned modpack export modal; the "Chaos Cubed" official skin pack in the Skin selector; better install error handling (a "Copy details" button on failure notifications, an install queue capped at three concurrent jobs, and live download progress reporting); and connect/read timeouts on all launcher HTTP requests so stalled connections fail fast instead of hanging.
- Upstream fixes brought in by the sync: app freezes when opening instance pages, instance edits not appearing to be saved immediately, content desync when enabling/disabling/removing mods, search cache shortened to 10 minutes, environment filter fixes in Discover, version/project links keeping track of the instance you came from, and a Files tab memory leak.
- "Update all" for CurseForge content now checks for updates several mods at a time and skips disabled files (updating one would silently re-enable it).
- CurseForge install notifications on the project page are now translated (previously always English).
- Noctrinth's CurseForge search filters, install pipeline and proxy support were re-integrated on top of the reworked upstream search (new environment filter), install error contexts, and the shared version page.

### Fixed
- Importing a CurseForge modpack from a local .zip works again — the creation modal previously rejected it with "no CurseForge API key is available" even when the key was configured, and the new-instance preview now shows the pack's real name, Minecraft version and loader from its manifest.
- A flaky connection or an Ely.by outage no longer signs you out of your Ely.by account: stored credentials are only removed when Ely.by explicitly rejects both the token and its refresh.
- Updating a CurseForge mod no longer deletes the old file before the new one has downloaded — a failed download now leaves the old, working file in place.
- Install buttons on CurseForge version rows are no longer disabled for files whose author hid the download URL — the CDN fallback and manual-download window handle those, same as the main Install button.
- Links inside CurseForge project descriptions that point to other CurseForge pages (relative links and /linkout redirects) now open correctly in the browser instead of a dead app-origin URL.
- Fixed the Oculus entry in the cross-platform mod mapping using a slug instead of the Modrinth project id, which broke installed-state mirroring between the two catalogs for that mod.`,
	},
	{
		version: '0.15.1',
		date: '2026-06-27T00:00:00+00:00',
		body: `### Added
- CurseForge modpack installation, rebuilt on the new 0.15 content pipeline: the instance is created from the pack manifest (Minecraft version + loader), every file is resolved and downloaded (with a CDN fallback for author-restricted files), overrides are applied, and Minecraft is installed.
- Manual download fallback for files whose author disabled third-party distribution — an embedded CurseForge window where you download once and the launcher captures and imports it automatically. It's a true last resort; normal API/CDN download is always tried first.

### Changed
- Synced with upstream Modrinth (0.14.8 → 0.15.1). This is a large release: the launcher backend was rewritten — instances and their content are now managed through a new content-management pipeline (the old "profile" model is gone), instance installs run as background jobs, and there are upstream fixes and UI refinements across the board.
- Re-ported Noctrinth's Ely.by account support on top of the new backend: launching with an Ely.by account (via the authlib-injector agent) works as before.
- CurseForge was reworked end-to-end. CurseForge files are now tracked natively as their own content source (with project and file ids) in the launcher database instead of browser storage, so installed-state and updates survive restarts and backups.
- Modrinth is the primary source. A required CurseForge dependency that also exists on Modrinth is now installed from Modrinth (dependency redirect), avoiding duplicate libraries like Cloth Config or Architectury.
- "Update all" is provider-aware: CurseForge content is updated through the CurseForge API and Modrinth content through Modrinth — the two never touch each other's files.

### Fixed
- The Discover page no longer jumps back to the first page when you install a mod inside an instance with "hide installed" turned on — the list now re-filters in place.
- CurseForge installs crashed with "Unknown instance" because the instance was addressed by its folder path instead of its id under the new model — fixed across every CurseForge install entry point (catalog, project page, and the instance picker modal).
- The CurseForge version/loader filter is now actually applied when browsing inside an instance. Previously the instance's locked version was ignored, so mods with no file for that version still showed up even though the filter appeared set.`,
	},
	{
		version: '0.14.8',
		date: '2026-06-20T00:00:00+00:00',
		body: `### Changed
- CurseForge search is now off by default. New installs see only Modrinth; turn CurseForge on in Settings → Appearance to reveal the catalog toggle on the Discover page and all CurseForge results and project pages.
- Synced with upstream Modrinth (0.14.6 → 0.14.8): reworked content dependency handling (a new dependency-deletion modal in the content tab and the Discover page now updating when dependencies are installed), more reliable update-version matching, fixes to game-version search (token separators and quotes vs backticks), Maven version-range matching, modpack export filtering, Babric mods no longer misdetected as Fabric, the floating action bar on mobile, comboboxes no longer closing when you click the scrollbar, and assorted billing/analytics fixes.

### Fixed
- CurseForge browse no longer fails on later pages. CurseForge rejects any request whose page index plus size exceeds 10,000 results; the page size is now clamped to the remaining window so deep pagination works instead of erroring.
- Switching catalogs (Modrinth ↔ CurseForge) now resets to the first page, so you no longer land on a blank page when the other catalog has fewer results.
- CurseForge project pages opened from the Discover page now keep a working "back to Discover" breadcrumb.
- Restored the Pride Fundraiser banner that briefly went missing after an upstream merge.
- Russian translations for CurseForge project pages, the CurseForge install dialog and its notifications, and the remaining untranslated settings, instance, world and friend-request strings.`,
	},
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
- CurseForge content sidebar redesign. Curated Modrinth-style category list (Library, Magic, Storage, Technology, Equipment, Mobs, …) instead of CurseForge's raw taxonomy.\\
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
