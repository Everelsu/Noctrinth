---
title: Plugins, themes, and Quick CSS
summary: Noctrinth now has its own plugin system — pure CSS themes with live settings, custom Discover sources, and isolated extensions.
date: 2026-11-22T12:00:00+09:00
---

Noctrinth now has a built-in plugin system. You can change how the launcher looks, plug new content sources directly into **Discover**, or automate your workflow with small extensions — all from **Settings → Plugins**.

## Themes without a single line of JavaScript

A theme plugin can be just `plugin.json` and `theme.css`. Better yet, themes can declare live settings (`boolean`, `select`, `color`, `number`, `string`) that automatically show up in the plugin's settings card and update CSS variables and `data-plugin-*` attributes on the fly.

The first official theme in the catalog, **[Floating Bars](https://github.com/Everelsu/Noctrinth-Floating-Bars)**, uses this to detach the title bar, navigation rail, and sidebar into rounded floating islands with a live toggle, corner radius selector, and border tint color picker — zero JavaScript required.

If you just want a quick personal tweak, **Quick CSS** sits right at the bottom of the Installed tab and applies live on top of any theme.

## Extensions and custom Discover sources

For anything beyond CSS, plugins can include an `entry` script running inside an isolated **Web Worker**:

- **UI extension points:** register buttons in the sidebar, instance header, or mod overflow menu, show toast notifications (`noctrinth.notify`), or open declarative modals (`noctrinth.ui.showModal`) built from native launcher blocks.
- **Content sources:** add Hangar, SpigotMC, PlanetMinecraft, or any custom mirror straight into the **Discover** source switcher alongside Modrinth and CurseForge.
- **Instance API & events:** read/manage instances and react to `instance:launch`, `instance:exit`, `instance:crash`, and `download:complete`.

## Built to fail safely

Plugins are isolated by design:

- **No DOM or raw filesystem access:** scripts run in a sandboxed Web Worker and only talk to the launcher through explicit permissions (`domains`, `events`, `instances`, `ui`). Network requests are strictly checked against the plugin's declared domain list.
- **Integrity checks:** archives from the official catalog are checked before installation so corrupted or tampered files are rejected automatically.
- **Crash protection:** runaway RPC floods are rate-limited, crashing plugins are automatically disabled with their error shown in Settings, and a built-in **Plugin logs** console + **Reload** button make debugging painless.
- **Safe Mode:** if a broken CSS theme or Quick CSS ever locks up the UI, press **`Ctrl+Shift+Alt+P`** (or launch with `--safe-mode`) to pause all plugins immediately.

> **Disclaimer:** Plugins are created by third-party authors. Noctrinth is not responsible for the safety, reliability, data loss, or malicious code in third-party plugins. Only install plugins from sources you trust.

## Try it or build your own

- Browse the live catalog on the new **[Plugins page](../../../plugins/)** — every entry has a one-click `noctrinth://plugin/install/<id>` button.
- Read the **[Plugin Documentation](../../../docs/)** for the full manifest reference, CSS theming guide, and SDK examples.
- Start your own plugin in seconds from **[Noctrinth-Plugin-Template](https://github.com/Everelsu/Noctrinth-Plugin-Template)** and submit it via Pull Request to **[Noctrinth-Plugins](https://github.com/Everelsu/Noctrinth-Plugins)**.
