#!/usr/bin/env node
// Build a static GitHub Pages site listing every Noctrinth and (upstream)
// Modrinth App changelog entry, sorted newest-first, source-tagged.
//
// Visually mirrors Modrinth's own /news/changelog Timeline + ChangelogEntry
// layout (vertical dashed line + dot markers + raised body cards) ported to
// plain CSS, with Noctrinth's purple brand and logo.
//
// Usage:
//   node scripts/build-changelog-site.mjs [output-dir]
//   Defaults to ./site

import {
	copyFileSync,
	cpSync,
	existsSync,
	mkdirSync,
	readdirSync,
	readFileSync,
	writeFileSync,
} from 'node:fs'
import { join, resolve } from 'node:path'

const OUT_DIR = process.argv[2] ?? 'site'
/** One markdown file per release, named after the version. */
const NOCTRINTH_DIR = 'apps/app-frontend/src/changelog'
const MODRINTH_SRC = 'packages/blog/changelog.ts'
const LOGO_SRC = 'apps/app/icons/noctrinth.svg'
/** Screenshots referenced from entries as `/changelog/<file>`. */
const SCREENSHOT_DIR = 'apps/app-frontend/src/changelog/screenshots'

/**
 * Pull every `{ ... body: `...` ... }` object out of a changelog TS source
 * and read the fields we care about with separate per-field regexes.
 */
function parseEntries(src) {
	const entries = []
	const entryRe = /\{((?:[^{}]|\{[^{}]*\})*?body:\s*`((?:\\`|[^`])*)`[^}]*)\}/g
	let m
	while ((m = entryRe.exec(src)) !== null) {
		const block = m[1]
		const body = m[2].replace(/\\`/g, '`').replace(/\\\$\{/g, '${')
		const version = block.match(/version:\s*['"`]([^'"`]+)['"`]/)?.[1]
		const date = block.match(/date:\s*['"`]([^'"`]+)['"`]/)?.[1]
		const product = block.match(/product:\s*['"`]([^'"`]+)['"`]/)?.[1]
		entries.push({ version, date, body, product })
	}
	return entries
}

const FRONT_MATTER = /^---\r?\n([\s\S]*?)\r?\n---\r?\n?/

/**
 * Read one release from `src/changelog/<version>.md`: the version is the file
 * name, the date comes from the front matter, everything after it is the body.
 */
function readMarkdownEntry(dir, file) {
	const raw = readFileSync(join(dir, file), 'utf-8')
	const match = FRONT_MATTER.exec(raw)
	const date = match ? /^date:\s*(.+)$/m.exec(match[1])?.[1]?.trim() : undefined

	if (!date) {
		console.warn(`Skipping ${file}: no date in its front matter`)
		return null
	}

	return {
		version: file.replace(/\.md$/, ''),
		date,
		body: raw.slice(match[0].length).trim(),
	}
}

const noctrinth = readdirSync(NOCTRINTH_DIR)
	// Entries are named after their version; the folder's README is not one.
	.filter((file) => file.endsWith('.md') && /^[0-9]/.test(file))
	.map((file) => readMarkdownEntry(NOCTRINTH_DIR, file))
	.filter((entry) => entry !== null)
	.map((entry) => ({ ...entry, source: 'noctrinth' }))
const modrinth = parseEntries(readFileSync(MODRINTH_SRC, 'utf-8'))
	.filter((e) => e.product === 'app')
	.map((e) => ({ ...e, source: 'modrinth' }))

const all = [...noctrinth, ...modrinth]
	.filter((e) => e.date)
	.sort((a, b) => new Date(b.date) - new Date(a.date))

// Just what changelog entries use: ##/### headings, - bullets, **bold**,
// `code`, [text](url) links.

function escapeHtml(s) {
	return s
		.replace(/&/g, '&amp;')
		.replace(/</g, '&lt;')
		.replace(/>/g, '&gt;')
		.replace(/"/g, '&quot;')
}

function renderInline(s) {
	return (
		escapeHtml(s)
			.replace(/`([^`]+)`/g, '<code>$1</code>')
			.replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>')
			// Images before links: the syntax differs by one leading `!`.
			.replace(/!\[([^\]]*)\]\(([^)]+)\)/g, '<img src="$2" alt="$1" loading="lazy">')
			.replace(/\[([^\]]+)\]\(([^)]+)\)/g, '<a href="$2" rel="noopener">$1</a>')
	)
}

function renderMarkdown(src) {
	const lines = src.replace(/\r\n/g, '\n').split('\n')
	const out = []
	let i = 0

	while (i < lines.length) {
		const trimmed = lines[i].trim()

		// Blank line
		if (!trimmed) {
			i++
			continue
		}

		// Headings — each on its own line
		if (trimmed.startsWith('### ')) {
			out.push(`<h3>${escapeHtml(trimmed.slice(4).trim())}</h3>`)
			i++
			continue
		}
		if (trimmed.startsWith('## ')) {
			out.push(`<h2>${escapeHtml(trimmed.slice(3).trim())}</h2>`)
			i++
			continue
		}
		if (trimmed.startsWith('# ')) {
			out.push(`<h1>${escapeHtml(trimmed.slice(2).trim())}</h1>`)
			i++
			continue
		}

		// Bullet list — consume consecutive `- ` lines
		if (trimmed.startsWith('- ')) {
			const items = []
			while (i < lines.length && lines[i].trim().startsWith('- ')) {
				items.push(lines[i].trim().slice(2).trim())
				i++
			}
			out.push(`<ul>${items.map((it) => `<li>${renderInline(it)}</li>`).join('')}</ul>`)
			continue
		}

		// Paragraph — consume until blank / heading / list
		const paraLines = []
		while (i < lines.length) {
			const t = lines[i].trim()
			if (!t || t.startsWith('#') || t.startsWith('- ')) break
			paraLines.push(t)
			i++
		}
		out.push(`<p>${renderInline(paraLines.join(' '))}</p>`)
	}

	return out.join('\n')
}

const longDateFmt = new Intl.DateTimeFormat('en-US', {
	year: 'numeric',
	month: 'long',
	day: 'numeric',
})

const absDateTimeFmt = new Intl.DateTimeFormat('en-US', {
	year: 'numeric',
	month: 'long',
	day: 'numeric',
	hour: 'numeric',
	minute: '2-digit',
})

const relTimeFmt = new Intl.RelativeTimeFormat('en-US', { numeric: 'auto' })

function formatLongDate(iso) {
	const d = new Date(iso)
	return Number.isNaN(d.getTime()) ? '' : longDateFmt.format(d)
}

function formatAbsoluteDateTime(iso) {
	const d = new Date(iso)
	return Number.isNaN(d.getTime()) ? '' : absDateTimeFmt.format(d)
}

function formatRelative(iso) {
	const d = new Date(iso).getTime()
	const now = Date.now()
	const diffMs = d - now
	const sec = diffMs / 1000
	if (Math.abs(sec) < 60) return relTimeFmt.format(Math.round(sec), 'second')
	const min = sec / 60
	if (Math.abs(min) < 60) return relTimeFmt.format(Math.round(min), 'minute')
	const hr = min / 60
	if (Math.abs(hr) < 24) return relTimeFmt.format(Math.round(hr), 'hour')
	const day = hr / 24
	if (Math.abs(day) < 30) return relTimeFmt.format(Math.round(day), 'day')
	return formatLongDate(iso)
}

const oneWeekMs = 7 * 24 * 60 * 60 * 1000
function isRecent(iso) {
	const t = new Date(iso).getTime()
	return Date.now() - t < oneWeekMs
}

const SOURCE_LABEL = { noctrinth: 'Noctrinth', modrinth: 'App' }

function renderEntry(entry, index) {
	const recent = isRecent(entry.date)
	const isFirst = index === 0
	// Dot is filled with the source brand colour when recent or first;
	// older entries get a muted hollow dot.
	const dotClass = recent || isFirst ? `entry__dot--${entry.source}` : 'entry__dot--muted'

	const versionName = entry.version ?? formatLongDate(entry.date)

	// Match Modrinth: show relative time for recent entries; long date when
	// the entry has a version (so the date isn't already in the title); and
	// nothing extra when the date itself IS the title.
	let trailingDate = ''
	if (recent) trailingDate = formatRelative(entry.date)
	else if (entry.version) trailingDate = formatLongDate(entry.date)

	return `
		<div class="entry">
			<div class="entry__head">
				<div class="entry__dot ${dotClass}"></div>
				<div class="entry__title">
					<h2>
						<span class="entry__type">${SOURCE_LABEL[entry.source]}</span>
						<span class="dot-sep"></span>
						<span class="entry__version">${escapeHtml(versionName)}</span>
					</h2>
					${trailingDate ? `<time class="entry__date" datetime="${escapeHtml(entry.date)}" title="${escapeHtml(formatAbsoluteDateTime(entry.date))}">${trailingDate}</time>` : ''}
				</div>
			</div>
			<div class="entry__body changelog-body">${renderMarkdown(entry.body)}</div>
		</div>`
}

const entriesHtml = all.map(renderEntry).join('\n')

// Client-side live time. The build bakes in a relative label ("3 days ago"),
// but a static page never rebuilds — so recompute every `<time datetime>` on
// load and once a minute, mirroring the build-time formatRelative logic. Each
// element also gets the exact patch date + time as a hover title.
const TIME_SCRIPT = `
	(function () {
		var rtf = new Intl.RelativeTimeFormat('en-US', { numeric: 'auto' });
		var longFmt = new Intl.DateTimeFormat('en-US', { year: 'numeric', month: 'long', day: 'numeric' });
		var absFmt = new Intl.DateTimeFormat('en-US', { year: 'numeric', month: 'long', day: 'numeric', hour: 'numeric', minute: '2-digit' });

		function relative(date) {
			var sec = (date.getTime() - Date.now()) / 1000;
			if (Math.abs(sec) < 60) return rtf.format(Math.round(sec), 'second');
			var min = sec / 60;
			if (Math.abs(min) < 60) return rtf.format(Math.round(min), 'minute');
			var hr = min / 60;
			if (Math.abs(hr) < 24) return rtf.format(Math.round(hr), 'hour');
			var day = hr / 24;
			if (Math.abs(day) < 30) return rtf.format(Math.round(day), 'day');
			return longFmt.format(date);
		}

		function update() {
			var nodes = document.querySelectorAll('time[datetime]');
			for (var i = 0; i < nodes.length; i++) {
				var el = nodes[i];
				var date = new Date(el.getAttribute('datetime'));
				if (isNaN(date.getTime())) continue;
				el.textContent = relative(date);
				el.title = absFmt.format(date);
			}
		}

		update();
		setInterval(update, 60000);
	})();
`

const CSS = `
	:root {
		--bg: #16101e;
		--bg-raised: #1f1730;
		--surface: #2a1f3d;
		--border: #2d2240;
		--button-bg: #2a1f3d;
		--button-border: #3a2c54;
		--text-contrast: #f7f3ff;
		--text: #e4dff0;
		--text-secondary: #ada3c9;
		--brand: #c78aff;
		--brand-soft: rgba(199, 138, 255, 0.18);
		--mr-green: #1bd96a;
		--mr-green-soft: rgba(27, 217, 106, 0.18);
		--link: #b283f9;
		--radius-md: 8px;
		--radius-lg: 12px;
		--radius-xl: 16px;
		--font-size-sm: 0.875rem;
	}

	* { box-sizing: border-box; }
	html, body {
		margin: 0;
		padding: 0;
		background: var(--bg);
		color: var(--text);
		font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
		font-size: 16px;
		line-height: 1.6;
		-webkit-font-smoothing: antialiased;
		-moz-osx-font-smoothing: grayscale;
		text-rendering: optimizeLegibility;
	}
	a { color: var(--link); text-decoration: none; }
	a:hover { filter: brightness(1.2); text-decoration: underline; }

	.page {
		max-width: 880px;
		margin: 0 auto;
		padding: 3rem 1.5rem 5rem;
	}

	/* ── Header ─────────────────────────────────────────────────────────────── */
	.site-header {
		display: flex;
		align-items: center;
		gap: 1rem;
		padding-bottom: 2rem;
		margin-bottom: 2rem;
		border-bottom: 1px solid var(--border);
	}
	.site-header__logo {
		width: 56px;
		height: 56px;
		flex-shrink: 0;
		background-color: var(--brand);
		-webkit-mask-image: url('logo.svg');
		mask-image: url('logo.svg');
		-webkit-mask-repeat: no-repeat;
		mask-repeat: no-repeat;
		-webkit-mask-position: center;
		mask-position: center;
		-webkit-mask-size: contain;
		mask-size: contain;
	}
	.site-header__text h1 {
		margin: 0;
		font-size: 1.75rem;
		font-weight: 800;
		color: var(--text-contrast);
	}
	.site-header__text p {
		margin: 0.3rem 0 0;
		color: var(--text-secondary);
		font-size: 0.95rem;
	}

	/* ── Timeline (dashed vertical line, like Modrinth's Timeline.vue) ──────── */
	.timeline {
		position: relative;
		display: flex;
		flex-direction: column;
		gap: 1rem;
		padding-bottom: 1.5rem;
		isolation: isolate;
	}
	.timeline::before {
		content: '';
		position: absolute;
		left: 6px;
		top: 1rem;
		height: calc(100% - 1rem);
		width: 4px;
		background-image: linear-gradient(
			to bottom,
			var(--bg-raised) 66%,
			rgba(255, 255, 255, 0) 0%
		);
		background-size: 100% 30px;
		background-repeat: repeat-y;
		mask-image: linear-gradient(to bottom, black calc(100% - 8rem), transparent 100%);
		z-index: -1;
	}

	/* ── Entry ──────────────────────────────────────────────────────────────── */
	.entry { display: flex; flex-direction: column; }
	.entry__head {
		display: flex;
		align-items: center;
		gap: 1rem;
	}
	.entry__dot {
		width: 16px;
		height: 16px;
		flex-shrink: 0;
		border-radius: 50%;
		border: 2px solid var(--button-border);
	}
	.entry__dot--noctrinth { background: var(--brand); }
	.entry__dot--modrinth { background: var(--mr-green); }
	.entry__dot--muted { background: var(--button-bg); }

	.entry__title {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.5rem 0.75rem;
	}
	.entry__title h2 {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		margin: 0;
		font-size: 1.25rem;
		font-weight: 800;
		color: var(--text-contrast);
		line-height: 1.2;
	}
	.entry__type {
		color: var(--text-contrast);
	}
	.dot-sep {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: var(--text-secondary);
	}
	.entry__version {
		font-weight: 700;
		color: var(--text);
	}
	.entry__date {
		color: var(--text-secondary);
		font-size: 0.9rem;
		font-weight: 500;
	}

	/* Body card (Modrinth's "ml-8 mt-3 rounded-2xl bg-bg-raised px-4 py-3") */
	.entry__body {
		margin-left: 2rem;
		margin-top: 1rem;
		padding: 1.25rem 1.5rem;
		border-radius: 1rem;
		background: var(--bg-raised);
		border: 1px solid var(--border);
	}

	/* ── Markdown body (mirrors packages/ui's .changelog-body styles) ────────── */
	.changelog-body {
		line-height: 1.65;
		word-break: break-word;
		color: var(--text);
		font-size: 0.975rem;
	}
	.changelog-body h1,
	.changelog-body h2,
	.changelog-body h3,
	.changelog-body h4,
	.changelog-body h5,
	.changelog-body h6 {
		margin: 0 0 0.5em;
		font-weight: 700;
		color: var(--text-contrast);
		letter-spacing: -0.01em;
	}
	.changelog-body h1:not(:first-child),
	.changelog-body h2:not(:first-child),
	.changelog-body h3:not(:first-child),
	.changelog-body h4:not(:first-child),
	.changelog-body h5:not(:first-child),
	.changelog-body h6:not(:first-child) {
		margin-top: 1.5em;
	}
	.changelog-body h2 {
		font-size: 1.15rem;
		padding-bottom: 0.4rem;
		border-bottom: 1px solid var(--border);
	}
	.changelog-body h3 {
		color: var(--brand);
		text-transform: uppercase;
		letter-spacing: 0.06em;
		font-size: 0.78rem;
		font-weight: 700;
	}
	.changelog-body ul {
		padding-left: 1.25rem;
		margin: 0.5rem 0 0;
		list-style: none;
	}
	.changelog-body li {
		margin: 0;
		position: relative;
		padding-left: 0.25rem;
	}
	.changelog-body li::before {
		content: '';
		position: absolute;
		left: -1rem;
		top: 0.7em;
		width: 5px;
		height: 5px;
		border-radius: 50%;
		background: var(--brand);
		opacity: 0.7;
	}
	.changelog-body * + li { margin-top: 0.45rem; }
	.changelog-body p { margin: 0; }
	.changelog-body * + p { margin-top: 0.75rem; }
	.changelog-body h3 + * { margin-top: 0.5rem; }
	.changelog-body * + h3 { margin-top: 1.25rem; }
	.changelog-body code {
		background: var(--bg);
		font-size: var(--font-size-sm);
		padding: 0.125rem 0.3rem;
		border-radius: 4px;
		font-family: 'JetBrains Mono', ui-monospace, 'SF Mono', Menlo, monospace;
	}
	.changelog-body strong { color: var(--text-contrast); }
	.changelog-body img {
		max-width: 100%;
		border-radius: var(--radius-md);
	}

	/* ── Footer ─────────────────────────────────────────────────────────────── */
	.footer {
		margin-top: 3rem;
		padding-top: 1.5rem;
		border-top: 1px solid var(--border);
		text-align: center;
		color: var(--text-secondary);
		font-size: 0.85rem;
	}
`

const HTML = `<!DOCTYPE html>
<html lang="en">
<head>
	<meta charset="UTF-8">
	<meta name="viewport" content="width=device-width, initial-scale=1">
	<title>Noctrinth — Changelog</title>
	<link rel="icon" type="image/svg+xml" href="logo.svg">
	<link rel="preconnect" href="https://fonts.googleapis.com">
	<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
	<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700;800&family=JetBrains+Mono:wght@400;500;600&display=swap">
	<style>${CSS}</style>
</head>
<body>
	<div class="page">
		<header class="site-header">
			<div class="site-header__logo" role="img" aria-label="Noctrinth"></div>
			<div class="site-header__text">
				<h1>Noctrinth Changelog</h1>
				<p>Combined release notes for Noctrinth and upstream Modrinth App.</p>
			</div>
		</header>

		<main class="timeline">${entriesHtml}</main>

		<footer class="footer">
			Generated from the in-app changelog ·
			<a href="https://github.com/Everelsu/noctrinth" rel="noopener">Source on GitHub</a>
		</footer>
	</div>
	<script>${TIME_SCRIPT}</script>
</body>
</html>
`

mkdirSync(OUT_DIR, { recursive: true })
writeFileSync(join(OUT_DIR, 'index.html'), HTML)
copyFileSync(LOGO_SRC, join(OUT_DIR, 'logo.svg'))

// The same entries as data, for the app to fetch instead of shipping them.
// Only the fork's own: upstream's are already bundled through @modrinth/blog.
const FEED = {
	generated: new Date().toISOString(),
	entries: noctrinth
		.slice()
		.sort((a, b) => new Date(b.date) - new Date(a.date))
		.map(({ version, date, body }) => ({ version, date, body })),
}
writeFileSync(join(OUT_DIR, 'changelog.json'), `${JSON.stringify(FEED)}\n`)

// Screenshots live on the site rather than inside the installer, so an entry
// can carry one without every future release paying for it in download size.
if (existsSync(SCREENSHOT_DIR)) {
	cpSync(SCREENSHOT_DIR, join(OUT_DIR, 'changelog'), { recursive: true })
}

console.log(
	`Wrote ${all.length} entries (${FEED.entries.length} Noctrinth) → ${OUT_DIR}/index.html + changelog.json`,
)
