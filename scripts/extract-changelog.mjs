#!/usr/bin/env node
// Extract a single Noctrinth changelog entry so the release workflow can use it
// as the GitHub release notes, under the banner and download table for that
// release's own files. `TAG` in the environment names the release (the
// workflows set it); without it the tag is taken to be `v<version>`.
//
// Entries live in apps/app-frontend/src/helpers/noctrinth-changelog.ts, newest
// first, and are read with the same parser the changelog site uses.
//
// A micropatch (`0.21.9+1`) goes out on its version's existing release, with
// files named for the version alone, so its notes are that build's entry
// followed by every earlier build of the same version.
//
// Usage:
//   node scripts/extract-changelog.mjs <version> [output-file]
//
// Defaults the output to RELEASE_BODY.md in the current directory.

import { writeFileSync } from 'node:fs'

import { NOCTRINTH_CHANGELOG_SRC, readEntries } from './lib/changelog-entries.mjs'

const [, , versionArg, outputArg] = process.argv

if (!versionArg) {
	console.error('Usage: extract-changelog.mjs <version> [output-file]')
	process.exit(1)
}

const output = outputArg ?? 'RELEASE_BODY.md'
const version = versionArg.split('+')[0]
const entries = readEntries(NOCTRINTH_CHANGELOG_SRC)
const start = entries.findIndex((entry) => entry.version === versionArg)

if (start === -1) {
	console.error(
		`No changelog entry for version ${versionArg} (looked in ${NOCTRINTH_CHANGELOG_SRC})`,
	)
	process.exit(1)
}

const builds = entries.slice(start).filter((entry) => entry.version.split('+')[0] === version)

if (builds.some((entry) => !entry.body.trim())) {
	console.error(`A changelog entry for ${version} is empty`)
	process.exit(1)
}

const body =
	builds.length === 1
		? builds[0].body.trim()
		: builds.map((entry) => `## ${entry.version}\n\n${entry.body.trim()}`).join('\n\n')

const REPO = 'Everelsu/Noctrinth'
const SITE = 'https://everelsu.github.io/Noctrinth'
const tag = process.env.TAG || `v${version}`
// Pictures from main rather than the tag: the buttons are newer than some of
// the releases they go on, and a tag from before them has none to show.
const raw = (path) => `https://raw.githubusercontent.com/${REPO}/main/${path}`
const download = (name) => `https://github.com/${REPO}/releases/download/${tag}/${name}`

const installers = [
	['Windows', 'x64 (Intel / AMD)', '.exe', `Noctrinth_${version}_x64-setup.exe`],
	['Windows', 'ARM64', '.exe', `Noctrinth_${version}_arm64-setup.exe`],
	['macOS', 'Universal (Intel / Apple Silicon)', '.dmg', `Noctrinth_${version}_universal.dmg`],
	['Linux', 'x64 (Intel / AMD)', 'AppImage', `Noctrinth_${version}_amd64.AppImage`],
	['Linux', 'x64 (Intel / AMD)', '.deb', `Noctrinth_${version}_amd64.deb`],
	['Linux', 'ARM64', 'AppImage', `Noctrinth_${version}_aarch64.AppImage`],
	['Linux', 'ARM64', '.deb', `Noctrinth_${version}_arm64.deb`],
]
const assets = process.env.RELEASE_ASSETS ? new Set(JSON.parse(process.env.RELEASE_ASSETS)) : null
const rows = installers
	.filter(([, , , file]) => !assets || assets.has(file))
	.map(([os, arch, format, file]) => `| ${os} | ${arch} | [${format}](${download(file)}) |`)
	.join('\n')
const header = `<p align="center"><img src="${raw('.github/assets/noctrinth-banner.svg')}" alt="Noctrinth" width="100%"></p>

| System | Architecture | Download |
| --- | --- | --- |
${rows}

[News](${SITE}/news/) · [Changelog](${SITE}/changelog/#v${versionArg})

---

`

// The entries point their screenshots at the changelog site as `/changelog/…`,
// which only resolves there; a release page needs the whole address.
const notes = body.replace(/\]\((\/[^)]+)\)/g, `](${SITE}$1)`)

writeFileSync(output, `${header}${notes}\n`)
console.log(`Wrote ${body.length} chars of changelog for ${versionArg} → ${output}`)
