#!/usr/bin/env node
// Extract a single Noctrinth changelog entry so the release workflow can use it
// verbatim as the GitHub release notes.
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

writeFileSync(output, `${body}\n`)
console.log(`Wrote ${body.length} chars of changelog for ${versionArg} → ${output}`)
