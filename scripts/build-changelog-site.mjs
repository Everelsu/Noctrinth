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

import { copyFileSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'

const OUT_DIR = process.argv[2] ?? 'site'
const NOCTRINTH_SRC = 'apps/app-frontend/src/helpers/noctrinth-changelog.ts'
const MODRINTH_SRC = 'packages/blog/changelog.ts'
const LOGO_SRC = 'apps/app-frontend/src/assets/noctrinth-icon.svg'

// ─── Changelog parsing ───────────────────────────────────────────────────────

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

const noctrinth = parseEntries(readFileSync(NOCTRINTH_SRC, 'utf-8')).map((e) => ({
	...e,
	source: 'noctrinth',
}))
const modrinth = parseEntries(readFileSync(MODRINTH_SRC, 'utf-8'))
	.filter((e) => e.product === 'app')
	.map((e) => ({ ...e, source: 'modrinth' }))

const all = [...noctrinth, ...modrinth]
	.filter((e) => e.date)
	.sort((a, b) => new Date(b.date) - new Date(a.date))

// ─── Minimal markdown → HTML ─────────────────────────────────────────────────
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
	return escapeHtml(s)
		.replace(/`([^`]+)`/g, '<code>$1</code>')
		.replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>')
		.replace(/\[([^\]]+)\]\(([^)]+)\)/g, '<a href="$2" rel="noopener">$1</a>')
}

function renderMarkdown(src) {
	const blocks = src.trim().split(/\n{2,}/)
	const out = []
	for (const raw of blocks) {
		const block = raw.trim()
		if (!block) continue
		if (block.startsWith('### ')) {
			out.push(`<h3>${escapeHtml(block.slice(4).trim())}</h3>`)
		} else if (block.startsWith('## ')) {
			out.push(`<h2>${escapeHtml(block.slice(3).trim())}</h2>`)
		} else if (/^- /m.test(block)) {
			const items = block
				.split('\n')
				.map((l) => l.replace(/^- /, '').trim())
				.filter(Boolean)
			out.push(`<ul>${items.map((i) => `<li>${renderInline(i)}</li>`).join('')}</ul>`)
		} else {
			out.push(`<p>${renderInline(block)}</p>`)
		}
	}
	return out.join('\n')
}

// ─── Date formatting + recency ───────────────────────────────────────────────

const longDateFmt = new Intl.DateTimeFormat('en-US', {
	year: 'numeric',
	month: 'long',
	day: 'numeric',
})

const relTimeFmt = new Intl.RelativeTimeFormat('en-US', { numeric: 'auto' })

function formatLongDate(iso) {
	const d = new Date(iso)
	return Number.isNaN(d.getTime()) ? '' : longDateFmt.format(d)
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

// ─── HTML rendering ──────────────────────────────────────────────────────────

const SOURCE_LABEL = { noctrinth: 'Noctrinth', modrinth: 'Modrinth App' }

function renderEntry(entry, index) {
	const recent = isRecent(entry.date)
	const isFirst = index === 0
	const dotClass = recent || isFirst ? 'entry__dot--brand' : ''
	const versionText = entry.version ?? formatLongDate(entry.date)
	const dateText = recent ? formatRelative(entry.date) : formatLongDate(entry.date)

	return `
		<div class="entry">
			<div class="entry__head">
				<div class="entry__dot ${dotClass}"></div>
				<div class="entry__title">
					<h2>
						<span class="source-tag source-tag--${entry.source}">${SOURCE_LABEL[entry.source]}</span>
						<span class="dot-sep"></span>
						<span class="version">${escapeHtml(versionText)}</span>
					</h2>
					<time class="entry__date" datetime="${escapeHtml(entry.date)}">${dateText}</time>
				</div>
			</div>
			<div class="entry__body changelog-body">${renderMarkdown(entry.body)}</div>
		</div>`
}

const entriesHtml = all.map(renderEntry).join('\n')

const CSS = `
	:root {
		--bg: #16101e;
		--bg-raised: #1f1730;
		--surface: #2a1f3d;
		--border: #2d2240;
		--button-bg: #2a1f3d;
		--button-border: #3a2c54;
		--text-contrast: #f3eefd;
		--text: #d6cee8;
		--text-secondary: #9b91b6;
		--brand: #8e32f3;
		--brand-soft: rgba(142, 50, 243, 0.18);
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
		line-height: 1.5;
		-webkit-font-smoothing: antialiased;
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
		background: var(--button-bg);
	}
	.entry__dot--brand {
		background: var(--brand);
	}
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
		font-size: 1.15rem;
		font-weight: 800;
		color: var(--text-contrast);
	}
	.dot-sep {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: var(--text-secondary);
	}
	.version {
		font-family: 'JetBrains Mono', ui-monospace, 'SF Mono', Menlo, monospace;
		font-size: 0.95rem;
		font-weight: 600;
		color: var(--text);
	}
	.entry__date {
		color: var(--text-secondary);
		font-size: 0.875rem;
	}

	/* Source tags (Modrinth/Noctrinth chips) */
	.source-tag {
		display: inline-flex;
		align-items: center;
		padding: 0.15rem 0.55rem;
		border-radius: 999px;
		font-size: 0.7rem;
		font-weight: 800;
		letter-spacing: 0.04em;
		text-transform: uppercase;
		line-height: 1;
		height: 1.4rem;
	}
	.source-tag--noctrinth {
		background: var(--brand-soft);
		color: var(--brand);
	}
	.source-tag--modrinth {
		background: var(--mr-green-soft);
		color: var(--mr-green);
	}

	/* Body card (Modrinth's "ml-8 mt-3 rounded-2xl bg-bg-raised px-4 py-3") */
	.entry__body {
		margin-left: 2rem;
		margin-top: 0.75rem;
		padding: 0.85rem 1.1rem 1rem;
		border-radius: var(--radius-xl);
		background: var(--bg-raised);
	}

	/* ── Markdown body (mirrors packages/ui's .changelog-body styles) ────────── */
	.changelog-body {
		line-height: 1.45;
		word-break: break-word;
	}
	.changelog-body h2,
	.changelog-body h3 {
		margin: 0 0 0.25em;
		font-weight: 700;
		color: var(--text-contrast);
	}
	.changelog-body h2:not(:first-child),
	.changelog-body h3:not(:first-child) {
		margin-top: 0.75em;
	}
	.changelog-body h2 { font-size: 1.05rem; }
	.changelog-body h3 { font-size: 0.95rem; }
	.changelog-body ul {
		padding-left: 1.25rem;
		margin: 0.4rem 0 0;
	}
	.changelog-body li { margin: 0; }
	.changelog-body * + li { margin-top: 0.5rem; }
	.changelog-body p { margin: 0; }
	.changelog-body * + p { margin-top: 0.5rem; }
	.changelog-body h3 + * { margin-top: 0.5rem; }
	.changelog-body * + h3 { margin-top: 0.75rem; }
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
			<img class="site-header__logo" src="logo.svg" alt="Noctrinth">
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
</body>
</html>
`

// ─── Write output ────────────────────────────────────────────────────────────

mkdirSync(OUT_DIR, { recursive: true })
writeFileSync(join(OUT_DIR, 'index.html'), HTML)
copyFileSync(LOGO_SRC, join(OUT_DIR, 'logo.svg'))

console.log(`Wrote ${all.length} entries → ${OUT_DIR}/index.html`)
