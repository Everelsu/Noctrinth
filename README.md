<img src=".github/assets/noctrinth-banner.svg" alt="Noctrinth" width="100%"/>

<div align="center">

[![Release](https://img.shields.io/github/v/release/Everelsu/Noctrinth?include_prereleases&style=for-the-badge&logo=github&logoColor=white&label=Release&labelColor=16181c&color=ac51fb)](https://github.com/Everelsu/Noctrinth/releases)
[![Downloads](https://img.shields.io/github/downloads/Everelsu/Noctrinth/total?style=for-the-badge&logo=github&logoColor=white&label=Downloads&labelColor=16181c&color=ac51fb)](https://github.com/Everelsu/Noctrinth/releases)
[![License](https://img.shields.io/badge/License-GPL--3.0-ac51fb?style=for-the-badge&logo=gnu&logoColor=white&labelColor=16181c)](apps/app/LICENSE)
[![Stars](https://img.shields.io/github/stars/Everelsu/Noctrinth?style=for-the-badge&logo=github&logoColor=white&label=Stars&labelColor=16181c&color=ac51fb)](https://github.com/Everelsu/Noctrinth/stargazers)

**English** · [Русский](README.ru.md)

**Switching launchers? Your instances come with you — one click, nothing left behind.**

🌙 A Modrinth App fork that speaks Ely.by, installs CurseForge packs, and imports your instances from six launchers 🚀

[Changelog](https://everelsu.github.io/Noctrinth/) · [Releases](https://github.com/Everelsu/Noctrinth/releases) · [Issues](https://github.com/Everelsu/Noctrinth/issues) · [Discussions](https://github.com/Everelsu/Noctrinth/discussions) · [Upstream](https://github.com/modrinth/code)

</div>

---

## Screenshots

<div align="center">
<table>
<tr>
<td width="50%"><img src=".github/assets/screenshots/library.png" alt="Your instance library, with a live news feed and friends list" width="100%"/><br/><sub>Library — every instance, with an activity feed alongside</sub></td>
<td width="50%"><img src=".github/assets/screenshots/ely-by-skins.png" alt="Managing an Ely.by skin from inside Noctrinth" width="100%"/><br/><sub>Ely.by skins, managed without leaving the launcher</sub></td>
</tr>
</table>
</div>

## Why Noctrinth

The Modrinth App is a genuinely good launcher — but it only knows Microsoft accounts, only installs Modrinth's own `.mrpack` files, and shows you ads while you browse. If you play on Ely.by, keep a shelf of CurseForge zips, or sit behind a blocked connection, you end up running a second launcher just to cover the gaps.

<div align="center">

| Metric            | Value                                   |
| ----------------- | --------------------------------------- |
| Account providers | Microsoft **+ Ely.by**                  |
| Import sources    | **6** launchers, including Modrinth App |
| Ads               | **none**                                |

</div>

## What Noctrinth adds

- **Ely.by accounts** — sign in alongside Microsoft, launch through authlib-injector, manage skins in an embedded window
- **Modrinth App migration** — a banner spots an existing Modrinth App install and offers to bring instances over, all at once or hand-picked, optionally clearing them from the source once the copy lands
- **CurseForge modpack `.zip` import** — install a pack straight from disk through the same job pipeline as everything else: queued, resumable, rolled back cleanly on failure
- **Screenshots tab** — a per-instance gallery with a full-screen viewer, zoom and pan, copy to clipboard, reveal in folder and delete
- **Collections & followed projects** — browse, create and edit collections, plus a virtual collection of everything you follow
- **Notifications** — Modrinth's notification feed, built into the app
- **Proxy** — one URL (`http://`, `https://`, `socks5://`, `socks5h://`) routes every launcher request, for regions where Modrinth is blocked
- **In-app changelog** — Noctrinth and Modrinth release notes side by side under Settings → Changelog
- **Purple, ad-free** — Noctrinth branding throughout, with Modrinth's ads and upsells switched off

## Get started

### Install

Grab the installer for your platform from the [latest release](https://github.com/Everelsu/Noctrinth/releases/latest):

| Platform | Notes                               |
| -------- | ----------------------------------- |
| Windows  | Installer (NSIS)                    |
| macOS    | Universal — Intel and Apple Silicon |
| Linux    | Built on Ubuntu 22.04               |

Updates are signed and delivered automatically through GitHub Releases — no reinstalling.

> [!NOTE]
> Pre-release builds (`0.17.4-beta.1` and similar) are **not** served to the auto-updater. Install them by hand; the app will pick up the matching stable release as a normal update once it ships.

### Bring your instances over

Already on the Modrinth App? Open Noctrinth — a banner offers to import everything it finds. Prefer to choose? **Create instance → Import** lists Modrinth App next to Prism, MultiMC, ATLauncher, GDLauncher and CurseForge.

## Build from source

Requires [Node.js](https://nodejs.org/) ≥ 24.15, [pnpm](https://pnpm.io/), [Rust](https://www.rust-lang.org/tools/install), and the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).

```bash
pnpm install
```

Copy the environment template in `packages/app-lib/`, then start the desktop app with hot reload:

```bash
pnpm app:dev
```

Before opening a pull request, run the frontend checks:

```bash
pnpm prepr:frontend:app
```

## Repository layout

This is the upstream Modrinth monorepo, so it carries far more than the launcher — the website, the backend API and the shared libraries all live here. Noctrinth ships the desktop app.

| Path                                | What it is                                             |
| ----------------------------------- | ------------------------------------------------------ |
| `apps/app`                          | Tauri shell — Rust commands, window and updater config |
| `apps/app-frontend`                 | Desktop UI (Vue 3), Noctrinth's changelog and locales  |
| `packages/app-lib`                  | Launcher core — accounts, instances, installs, imports |
| `packages/ui`                       | Shared Vue component library                           |
| `apps/frontend`, `apps/labrinth`, … | Modrinth's website and backend, carried from upstream  |

For architecture and infrastructure that isn't fork-specific, the [upstream repository](https://github.com/modrinth/code) remains the reference.

## Relationship with upstream

Noctrinth syncs with [modrinth/code](https://github.com/modrinth/code) and pins its version to upstream's exactly — when Modrinth is on `0.17.3`, so is Noctrinth. Where both sides implement the same thing, upstream's version wins and the fork's is dropped. Fork-only work survives only where it doesn't collide.

Fast patches between upstream releases ship as semver pre-releases (`0.17.4-beta.1`), which sort above the current stable and below the next one — so testers roll onto the real release the moment it lands.

## Contributing

Bug reports and pull requests are welcome — [open an issue](https://github.com/Everelsu/Noctrinth/issues) to start.

Found a bug that isn't Noctrinth-specific? It belongs [upstream](https://github.com/modrinth/code/issues); fixing it there means everyone gets it, and it reaches this fork on the next sync.

If Noctrinth helped you or something like that, [give it a star](https://github.com/Everelsu/Noctrinth/stargazers).

## License

The desktop app is licensed under [GPL-3.0](apps/app/LICENSE). Other packages carry their own licenses — see the `LICENSE` file in each, and [COPYING.md](COPYING.md) for details.

Modrinth branding is the property of Rinth, Inc. and is not used here; Noctrinth ships its own. Noctrinth is an independent fork and is not affiliated with or endorsed by Rinth, Inc.
