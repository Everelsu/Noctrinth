#!/usr/bin/env node
// Extract a single Noctrinth changelog entry so the release workflow can use it
// as the GitHub release notes, under the banner and download buttons for that
// release's own files. `TAG` in the environment names the release (the
// workflows set it); without it the tag is taken to be `v<version>`.
//
// Entries live in apps/app-frontend/src/helpers/noctrinth-changelog.ts, newest
// first, and are read with the same parser the changelog site uses.
//
// Usage:
//   node scripts/extract-changelog.mjs <version> [output-file]
//
// Defaults the output to RELEASE_BODY.md in the current directory.

import { writeFileSync } from 'node:fs'

import { findNoctrinthEntry, NOCTRINTH_CHANGELOG_SRC } from './lib/changelog-entries.mjs'

const [, , versionArg, outputArg] = process.argv

if (!versionArg) {
	console.error('Usage: extract-changelog.mjs <version> [output-file]')
	process.exit(1)
}

const output = outputArg ?? 'RELEASE_BODY.md'
const entry = findNoctrinthEntry(versionArg)

if (!entry) {
	console.error(
		`No changelog entry for version ${versionArg} (looked in ${NOCTRINTH_CHANGELOG_SRC})`,
	)
	process.exit(1)
}

const body = entry.body.trim()

if (!body) {
	console.error(`The changelog entry for ${versionArg} is empty`)
	process.exit(1)
}

const REPO = 'Everelsu/Noctrinth'
const SITE = 'https://everelsu.github.io/Noctrinth'
const tag = process.env.TAG || `v${versionArg}`
// Pictures from main rather than the tag: the buttons are newer than some of
// the releases they go on, and a tag from before them has none to show.
const raw = (path) => `https://raw.githubusercontent.com/${REPO}/main/${path}`
const download = (name) => `https://github.com/${REPO}/releases/download/${tag}/${name}`

const button = (os, label, file) =>
	`<a href="${download(file)}"><img src="${raw(`.github/assets/readme/download-${os}.svg`)}" alt="Download for ${label}" height="56"></a>`

const header = `<p align="center"><img src="${raw('.github/assets/noctrinth-banner.svg')}" alt="Noctrinth" width="100%"></p>

<p align="center">
${button('windows', 'Windows', `Noctrinth_${versionArg}_x64-setup.exe`)}
${button('macos', 'macOS', `Noctrinth_${versionArg}_universal.dmg`)}
${button('linux', 'Linux', `Noctrinth_${versionArg}_amd64.AppImage`)}
</p>

<p align="center"><a href="${SITE}/news/">News</a> · <a href="${SITE}/changelog/#v${versionArg}">Changelog</a> · <a href="${download(`Noctrinth_${versionArg}_amd64.deb`)}">Linux .deb</a></p>

---

`

// The entries point their screenshots at the changelog site as `/changelog/…`,
// which only resolves there; a release page needs the whole address.
const notes = body.replace(/\]\((\/[^)]+)\)/g, `](${SITE}$1)`)

writeFileSync(output, `${header}${notes}\n`)
console.log(`Wrote ${body.length} chars of changelog for ${versionArg} → ${output}`)
