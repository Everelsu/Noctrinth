#!/usr/bin/env node
/**
 * Propagates the version in the root `VERSION` file everywhere it is needed.
 *
 * `VERSION` is the single place a release number is written by hand. Tauri
 * reads the app version out of `apps/app-frontend/package.json`, and the Rust
 * crates carry their own, so this copies it into both rather than asking
 * anyone to remember three files.
 *
 * It may carry a build number after a `+`, as semver build metadata: `0.18.2+1`
 * is the second build published under the number 0.18.2, which is how a release
 * is patched without moving a version that is upstream's to set. Only the
 * release number is copied onward — a build number is not something a crate or
 * a package.json has any use for. See PATCH handling in apps/app/build.rs.
 *
 * Run with `--check` to verify they already agree without writing anything,
 * which is what CI uses to catch a hand-edited package.json.
 */
import { readFileSync, writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = join(dirname(fileURLToPath(import.meta.url)), '..')
const checkOnly = process.argv.includes('--check')

const declared = readFileSync(join(root, 'VERSION'), 'utf8').trim()
if (!/^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?(\+\d+)?$/.test(declared)) {
	console.error(`VERSION does not hold a semver version: ${JSON.stringify(declared)}`)
	process.exit(1)
}

/** The release number, and which build of it this is. */
const [version, build = '0'] = declared.split('+')

const mismatches = []

function updatePackageJson(relative) {
	const path = join(root, relative)
	const raw = readFileSync(path, 'utf8')
	const parsed = JSON.parse(raw)
	if (parsed.version === version) return

	if (checkOnly) {
		mismatches.push(`${relative} is at ${parsed.version}`)
		return
	}

	// Rewritten textually so the file keeps its own formatting and key order.
	writeFileSync(path, raw.replace(/("version":\s*)"[^"]*"/, `$1"${version}"`))
	console.log(`${relative} -> ${version}`)
}

function updateCargoToml(relative) {
	const path = join(root, relative)
	const raw = readFileSync(path, 'utf8')
	// Only the first `version =` line, which is the one under [package].
	const replaced = raw.replace(/^version = .*$/m, `version = "${version}"`)
	if (replaced === raw) return

	if (checkOnly) {
		mismatches.push(`${relative} disagrees`)
		return
	}

	writeFileSync(path, replaced)
	console.log(`${relative} -> ${version}`)
}

updatePackageJson('apps/app-frontend/package.json')
updateCargoToml('apps/app/Cargo.toml')
updateCargoToml('packages/app-lib/Cargo.toml')

if (mismatches.length) {
	console.error(`VERSION says ${version}, but:\n  ${mismatches.join('\n  ')}`)
	console.error('Run `pnpm version:sync` and commit the result.')
	process.exit(1)
}

if (checkOnly) {
	console.log(
		build === '0' ? `Everything is at ${version}` : `Everything is at ${version}, build ${build}`,
	)
}
