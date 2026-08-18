#!/usr/bin/env node
// Extract a single Noctrinth changelog entry so the release workflow can use it
// verbatim as the GitHub release notes.
//
// Entries live one per file in apps/app-frontend/src/changelog/<version>.md,
// with the date in front matter; only the body below it is release notes.
//
// Usage:
//   node scripts/extract-changelog.mjs <version> [output-file]
//
// Defaults the output to RELEASE_BODY.md in the current directory.

import { existsSync, readFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'

const [, , versionArg, outputArg] = process.argv

if (!versionArg) {
	console.error('Usage: extract-changelog.mjs <version> [output-file]')
	process.exit(1)
}

const DIRECTORY = 'apps/app-frontend/src/changelog'
const output = outputArg ?? 'RELEASE_BODY.md'
const source = join(DIRECTORY, `${versionArg}.md`)

if (!existsSync(source)) {
	console.error(`No changelog entry for version ${versionArg} (looked for ${source})`)
	process.exit(1)
}

const raw = readFileSync(source, 'utf-8')
const body = raw.replace(/^---\r?\n[\s\S]*?\r?\n---\r?\n?/, '').trim()

if (!body) {
	console.error(`${source} has front matter but no body`)
	process.exit(1)
}

writeFileSync(output, `${body}\n`)
console.log(`Wrote ${body.length} chars of changelog for ${versionArg} → ${output}`)
