# Noctrinth

**Noctrinth** is a fork of [Modrinth](https://github.com/modrinth/code). This monorepo includes the desktop app, web frontend, and shared packages.

For documentation, architecture, and general infrastructure details refer to the **[original repository](https://github.com/modrinth/code)**

---

### Added in the desktop app

- **Ely.by accounts** — sign in with Ely.by alongside Microsoft, launch via authlib-injector, manage skins in an embedded window
- **Collections** — browse, create, edit, delete, add/remove projects, plus a "Save to collection" button on project pages
- **Followed projects** — a virtual collection of everything you follow
- **Notifications** — the Modrinth notifications page, built into the app
- **Screenshots tab** — per-instance gallery with lightbox, zoom, copy and delete
- **In-app changelog** — Noctrinth release notes under Settings → Changelog
- **Proxy** — one proxy URL (`http://`, `https://`, `socks5://`, `socks5h://`) for all launcher traffic, under Settings → Resource management

## License

All packages are licensed under their respective LICENSE files inside each package — same as the original repository.
