---
title: CurseForge is back
summary: Removed in 0.17.3, it is back in the launcher, rewritten from scratch and probably calmer and steadier than it was.
date: 2026-10-21T12:00:00+09:00
---

In [0.17.3](https://github.com/Everelsu/Noctrinth/releases/tag/noctrinth-v0.17.3) CurseForge left the launcher: search, installs, project pages and mod mapping all went, and only importing instances from other launchers stayed. In 0.21.10 it comes back. This is not the old code pulled out of the archive but a new integration, written again on top of what the launcher already does.

## What it looks like

Discover has a **Modrinth / CurseForge** switch above the results. CurseForge brings its own categories and filters, and every project has a full page: the description, the versions, a page for each version with its changelog, the gallery, and a star to keep a project among your favourites.

Mods, resource packs, shaders and modpacks install through the same window as Modrinth's, with the same checks: which instances they fit, a warning for an incompatible version, and their required dependencies. A CurseForge modpack installs as an ordinary install job, with progress and a cancel button, and with the right game version and loader from the start.

## Then and now

- **Then:** CurseForge had an install path of its own, and it broke in its own ways: installs crashed with "Unknown instance", the version and loader filters were lost inside an instance, search broke on later pages, and an update deleted the old file before the new one had arrived. **Now:** CurseForge goes through the same windows and checks as Modrinth, with no separate, fragile path of its own.
- **Then:** since 0.17.3, a CurseForge mod you dropped in by hand or brought from another launcher stayed a nameless file. **Now:** a mod is recognised by its file's fingerprint, even one you dropped in by hand or brought from another launcher, with its name, icon, author and a link to its page.
- **Then:** updates were checked separately for each mod, several at a time. **Now:** with one request for the whole instance, the way the CurseForge app does it, and the update badge, Update all and switching versions with their changelogs work as they do for Modrinth mods.
- **Then:** CurseForge search was switched on somewhere in the appearance settings. **Now:** CurseForge has a settings tab of its own, to turn it off altogether, choose where Discover opens, and decide whether files are recognised, how far updates go (releases only, releases and betas, or anything) and whether other launchers are searched for files.

CurseForge's answers are remembered, and the launcher asks for nothing it does not need: the file list loads only when the Versions tab opens, and a project is loaded ahead only when the pointer rests on its card, not on every card it crosses.

## What else is new

- Synced with Modrinth 0.21.10: settings are sorted into Display, Account, Instance defaults and System, and mods can be installed and switched off while the game is running.
- In the Files tab, mods, resource packs and shaders show their icons, names, versions and authors, and screenshots now they have a preview—not exactly a beautiful one, but a preview nonetheless.
- The empty Logs tab is kept by a night owl instead of Modrinth's frog. I’m not really feeling it. I think I’d redo it.

Everything else is in the [0.21.10 changelog](https://everelsu.github.io/Noctrinth/changelog/#v0.21.10). If something with CurseForge does not work as it should, [open an issue](https://github.com/Everelsu/Noctrinth/issues).
