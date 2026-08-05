<div align="center">
  <img src="apps/app/icons/icon.png" alt="Noctrinth" width="256"/>

[![CI](https://img.shields.io/github/actions/workflow/status/Everelsu/Noctrinth/noctrinth-ci.yml?branch=main&label=CI)](https://github.com/Everelsu/Noctrinth/actions/workflows/noctrinth-ci.yml)
[![Release](https://img.shields.io/github/v/release/Everelsu/Noctrinth?include_prereleases&label=release)](https://github.com/Everelsu/Noctrinth/releases)
[![Downloads](https://img.shields.io/github/downloads/Everelsu/Noctrinth/total?label=downloads)](https://github.com/Everelsu/Noctrinth/releases)
[![License](https://img.shields.io/badge/license-GPL--3.0-blue)](apps/app/LICENSE)
[![Stars](https://img.shields.io/github/stars/Everelsu/Noctrinth)](https://github.com/Everelsu/Noctrinth/stargazers)

**Switching launchers? Your instances come with you — one click, nothing left behind.**

🌙 A Modrinth App fork that speaks Ely.by, installs CurseForge packs, and imports your instances from six launchers 🚀

[Changelog](https://everelsu.github.io/Noctrinth/) · [Releases](https://github.com/Everelsu/Noctrinth/releases) · [Issues](https://github.com/Everelsu/Noctrinth/issues) · [Upstream](https://github.com/modrinth/code)

</div>

---

## 💡 Why Noctrinth

**The pain.** The Modrinth App is a genuinely good launcher — but it only knows Microsoft accounts, only installs Modrinth's own `.mrpack` files, and shows you ads while you browse. If you play on Ely.by, keep a shelf of CurseForge zips, or sit behind a blocked connection, you end up running a second launcher just to cover the gaps.

**The solution.** Noctrinth is that launcher, minus the gaps. It tracks upstream Modrinth release for release — you keep every feature they ship — and adds the pieces their roadmap doesn't cover.

**The result.** One launcher. Your Modrinth App instances move over in a single click, and nothing you already rely on goes away.

<div align="center">

| Metric                 | Value                                   |
| ---------------------- | --------------------------------------- |
| 🔑 Account providers   | Microsoft **+ Ely.by**                  |
| 📦 Import sources      | **6** launchers, including Modrinth App |
| 🌍 Interface languages | **33**                                  |
| 🚫 Ads                 | **none**                                |

</div>

## ⚖️ Noctrinth vs. Modrinth App

|                                                                    | Noctrinth | Modrinth App |
| ------------------------------------------------------------------ | :-------: | :----------: |
| Microsoft accounts                                                 |    ✅     |      ✅      |
| **Ely.by accounts** (authlib-injector)                             |    ✅     |      ❌      |
| Import from Prism / MultiMC / ATLauncher / GDLauncher / CurseForge |    ✅     |      ✅      |
| **Import from Modrinth App**                                       |    ✅     |      —       |
| Modrinth `.mrpack` modpacks                                        |    ✅     |      ✅      |
| **CurseForge `.zip` modpacks**                                     |    ✅     |      ❌      |
| **Per-instance screenshots tab**                                   |    ✅     |      ❌      |
| **Collections & notifications in-app**                             |    ✅     |      ❌      |
| **Proxy for all launcher traffic**                                 |    ✅     |      ❌      |
| **In-app changelog**                                               |    ✅     |      ❌      |
| Ads & Modrinth+ upsell                                             |  ❌ none  |   ✅ shown   |

> Everything in the first column that isn't bold is upstream's work, kept as-is. Noctrinth's rule is that upstream always wins — the fork adds, it doesn't replace.

## ✨ What Noctrinth adds

- 🔐 **Ely.by accounts** — sign in alongside Microsoft, launch through authlib-injector, manage skins in an embedded window
- 📥 **Modrinth App migration** — a banner spots an existing Modrinth App install and offers to bring instances over, all at once or hand-picked, optionally clearing them from the source once the copy lands
- 📦 **CurseForge modpack `.zip` import** — install a pack straight from disk through the same job pipeline as everything else: queued, resumable, rolled back cleanly on failure
- 🖼️ **Screenshots tab** — a per-instance gallery with a full-screen viewer, zoom and pan, copy to clipboard, reveal in folder and delete
- 📚 **Collections & followed projects** — browse, create and edit collections, plus a virtual collection of everything you follow
- 🔔 **Notifications** — Modrinth's notification feed, built into the app
- 🌐 **Proxy** — one URL (`http://`, `https://`, `socks5://`, `socks5h://`) routes every launcher request, for regions where Modrinth is blocked
- 📝 **In-app changelog** — Noctrinth and Modrinth release notes side by side under Settings → Changelog
- 💜 **Purple, ad-free** — Noctrinth branding throughout, with Modrinth's ads and upsells switched off

## 🚀 Get started

### Install

Grab the installer for your platform from the [latest release](https://github.com/Everelsu/Noctrinth/releases/latest):

| Platform   | Notes                               |
| ---------- | ----------------------------------- |
| 🪟 Windows | Installer (NSIS)                    |
| 🍎 macOS   | Universal — Intel and Apple Silicon |
| 🐧 Linux   | Built on Ubuntu 22.04               |

Updates are signed and delivered automatically through GitHub Releases — no reinstalling.

> [!NOTE]
> Pre-release builds (`0.17.4-beta.1` and similar) are **not** served to the auto-updater. Install them by hand; the app will pick up the matching stable release as a normal update once it ships.

### Bring your instances over

Already on the Modrinth App? Open Noctrinth — a banner offers to import everything it finds. Prefer to choose? **Create instance → Import** lists Modrinth App next to Prism, MultiMC, ATLauncher, GDLauncher and CurseForge.

## 🛠️ Build from source

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

## 📁 Repository layout

This is the upstream Modrinth monorepo, so it carries far more than the launcher — the website, the backend API and the shared libraries all live here. Noctrinth ships the desktop app.

| Path                                | What it is                                             |
| ----------------------------------- | ------------------------------------------------------ |
| `apps/app`                          | Tauri shell — Rust commands, window and updater config |
| `apps/app-frontend`                 | Desktop UI (Vue 3), Noctrinth's changelog and locales  |
| `packages/app-lib`                  | Launcher core — accounts, instances, installs, imports |
| `packages/ui`                       | Shared Vue component library                           |
| `apps/frontend`, `apps/labrinth`, … | Modrinth's website and backend, carried from upstream  |

For architecture and infrastructure that isn't fork-specific, the [upstream repository](https://github.com/modrinth/code) remains the reference.

## 🔄 Relationship with upstream

Noctrinth syncs with [modrinth/code](https://github.com/modrinth/code) and pins its version to upstream's exactly — when Modrinth is on `0.17.3`, so is Noctrinth. Where both sides implement the same thing, upstream's version wins and the fork's is dropped. Fork-only work survives only where it doesn't collide.

Fast patches between upstream releases ship as semver pre-releases (`0.17.4-beta.1`), which sort above the current stable and below the next one — so testers roll onto the real release the moment it lands.

## 🤝 Contributing

Bug reports and pull requests are welcome — [open an issue](https://github.com/Everelsu/Noctrinth/issues) to start.

Found a bug that isn't Noctrinth-specific? It belongs [upstream](https://github.com/modrinth/code/issues); fixing it there means everyone gets it, and it reaches this fork on the next sync.

⭐ If Noctrinth saved you from running two launchers, [give it a star](https://github.com/Everelsu/Noctrinth/stargazers) — it's the whole marketing budget.

## 📄 License

The desktop app is licensed under [GPL-3.0](apps/app/LICENSE). Other packages carry their own licenses — see the `LICENSE` file in each, and [COPYING.md](COPYING.md) for details.

Modrinth branding is the property of Rinth, Inc. and is not used here; Noctrinth ships its own. Noctrinth is an independent fork and is not affiliated with or endorsed by Rinth, Inc.

<div align="center">

Built by [Relsev](https://everelsu.github.io/RelsevLink/) · on the shoulders of [Modrinth](https://modrinth.com)

</div>
