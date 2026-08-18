/**
 * Noctrinth changelog.
 *
 * Entries live as markdown in `src/changelog/<version>.md` rather than in this
 * file, so writing one is editing a document instead of editing code. Each
 * needs a `date` in its front matter; the version is the file name.
 *
 * Screenshots go in `public/changelog/` and are referenced from the root, for
 * example `![Modern Java](/changelog/modern-java.png)`. Keeping them in
 * `public` means the path in the markdown is the path that ships — nothing has
 * to resolve or bundle them.
 *
 * The body is markdown with `### Added`, `### Changed`, `### Deprecated`,
 * `### Removed`, `### Fixed` and `### Security` headings (the Keep a Changelog
 * convention that Modrinth's own PR template follows) and `- item` bullets.
 *
 * This file is Noctrinth-specific and deliberately sits outside
 * `packages/blog`, so a sync with upstream Modrinth never overwrites it.
 */

export interface NoctrinthVersionEntry {
	version: string
	/** ISO date string. */
	date: string
	body: string
}

const entryFiles = import.meta.glob('../changelog/*.md', {
	query: '?raw',
	import: 'default',
	eager: true,
}) as Record<string, string>

const FRONT_MATTER = /^---\r?\n([\s\S]*?)\r?\n---\r?\n?/

function parseEntry(path: string, raw: string): NoctrinthVersionEntry | null {
	const version = path.split('/').pop()?.replace(/\.md$/, '')
	if (!version) return null

	const match = FRONT_MATTER.exec(raw)
	const date = match ? (/^date:\s*(.+)$/m.exec(match[1])?.[1]?.trim() ?? '') : ''

	if (!date) {
		console.warn(`Changelog entry ${path} has no date in its front matter`)
		return null
	}

	return { version, date, body: raw.slice(match?.[0].length ?? 0).trim() }
}

/** Compares two dotted versions numerically, so 0.9 sorts below 0.10. */
function compareVersions(left: string, right: string): number {
	const parts = (value: string) => value.split(/[.-]/).map((part) => Number(part))
	const a = parts(left)
	const b = parts(right)

	for (let i = 0; i < Math.max(a.length, b.length); i++) {
		const x = a[i]
		const y = b[i]
		// A pre-release suffix leaves a NaN, which sorts below the plain release.
		if (Number.isNaN(x) || x === undefined) return Number.isNaN(y) || y === undefined ? 0 : -1
		if (Number.isNaN(y) || y === undefined) return 1
		if (x !== y) return x - y
	}

	return 0
}

const entries: NoctrinthVersionEntry[] = Object.entries(entryFiles)
	.map(([path, raw]) => parseEntry(path, raw))
	.filter((entry): entry is NoctrinthVersionEntry => entry !== null)
	.sort((a, b) => {
		const byDate = new Date(b.date).getTime() - new Date(a.date).getTime()
		return byDate !== 0 ? byDate : compareVersions(b.version, a.version)
	})

export function getNoctrinthChangelog(): NoctrinthVersionEntry[] {
	return entries
}
