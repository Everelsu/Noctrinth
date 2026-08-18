# Writing a changelog entry

One file per release: `<version>.md`, where the version is exactly the one in
the repository's `VERSION` file — the app matches entries to versions by that
name, and the fork's version always equals the upstream Modrinth release it is
built from.

```markdown
---
date: 2026-08-16T00:00:00+00:00
---

### Added

- One-click modern Java for 1.7.10. Instances on that version are pinned to
  Java 8 by Mojang's own manifest; installing
  [lwjgl3ify](https://github.com/GTNewHorizons/lwjgl3ify) from the instance's
  Java settings lifts that, and removing it puts the instance back as it was.

### Fixed

- Finishing an update download raised an error for an update that had in fact
  downloaded fine. The cleanup step treated a plain unsubscribe function as a
  promise, so the update was never marked ready to install.
```

The `date` in the front matter is what the changelog sorts by; the body is
markdown under `### Added`, `### Changed`, `### Deprecated`, `### Removed`,
`### Fixed` and `### Security` headings — the Keep a Changelog convention that
Modrinth's own pull request template follows. Skip the headings that have
nothing under them.

## What an entry says

Write for someone who uses the launcher, not for someone who reads the diff.
An entry earns its place by describing the change the user meets:

- Name the symptom before the fix — "the keyboard selection was invisible on
  the OLED theme", not "adjusted `--color-surface-3`".
- Say what a setting now does, and where it lives, so it can be found.
- Note when something is put back the way it was — reversibility is the part
  people worry about.
- No commit hashes, no file paths, no internal identifiers.
- Sync commits get one bullet naming the upstream range and what came with it.

## Screenshots

Drop the image in `screenshots/` next to these entries and reference it from
the root of the changelog site:

```markdown
![Modern Java](/changelog/modern-java.png)
```

It is published with the changelog site and loaded from there, so it costs the
download once for whoever scrolls to it instead of riding inside every
installer. Only paths under `/changelog/` are accepted.

## Where it shows up

- **In the app** — Settings → Display → Changelog, from the files bundled here.
- **On the web** — <https://everelsu.github.io/Noctrinth>, rebuilt by
  `.github/workflows/pages.yml` on every push to `main` that touches this
  folder, `site/changelog/`, or `scripts/build-changelog-site.mjs`.
- **Back in the app, over the network** — the same build publishes
  `changelog.json`, which an installed launcher fetches and overlays on what it
  shipped with. So an entry written after a release still reaches the people
  running it, and a correction does not need a release of its own.

Run the site build locally to check an entry before pushing:

```bash
node scripts/build-changelog-site.mjs /tmp/site
```
