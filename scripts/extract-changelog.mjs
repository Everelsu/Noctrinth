#!/usr/bin/env node
// Extract a single Noctrinth changelog entry (markdown body) from
// apps/app-frontend/src/helpers/noctrinth-changelog.ts so the release
// workflow can use it verbatim as the GitHub release notes.
//
// Usage:
//   node scripts/extract-changelog.mjs <version> [output-file]
//
// Defaults the output to RELEASE_BODY.md in the current directory.

import { readFileSync, writeFileSync } from 'node:fs'

const [, , versionArg, outputArg] = process.argv

if (!versionArg) {
	console.error('Usage: extract-changelog.mjs <version> [output-file]')
	process.exit(1)
}

const SOURCE = 'apps/app-frontend/src/helpers/noctrinth-changelog.ts'
const output = outputArg ?? 'RELEASE_BODY.md'

const src = readFileSync(SOURCE, 'utf-8')

// Find the entry: { version: '<version>', date: '...', body: `<body>` }
// The body is a template literal — match content up to the first un-escaped
// backtick. Inside the file backticks are escaped as `\``.
const versionEscaped = versionArg.replace(/[.\\]/g, (c) => `\\${c}`)
const re = new RegExp(
	String.raw`version:\s*['"]` +
		versionEscaped +
		String.raw`['"][\s\S]*?body:\s*\x60((?:\\\x60|[^\x60])*)\x60`,
)

const match = src.match(re)
if (!match) {
	console.error(`No changelog entry for version ${versionArg} in ${SOURCE}`)
	process.exit(1)
}

// Un-escape backticks and dollar-brace placeholders that the TS template
// literal would have interpolated at runtime.
const body = match[1].replace(/\\`/g, '`').replace(/\\\$\{/g, '${')

writeFileSync(output, body)
console.log(`Wrote ${body.length} chars of changelog for ${versionArg} → ${output}`)
