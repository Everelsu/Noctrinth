#!/usr/bin/env node
// Build a static GitHub Pages site listing every Noctrinth and (upstream)
// Modrinth App changelog entry, sorted newest-first, source-tagged.
//
// Usage:
//   node scripts/build-changelog-site.mjs [output-dir]
//   Defaults to ./site

import { copyFileSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'

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
// Just what changelog entries use: ## headings, - bullets, **bold**, `code`.

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
		if (block.startsWith('## ')) {
			out.push(`<h3>${escapeHtml(block.slice(3).trim())}</h3>`)
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

// ─── HTML template ───────────────────────────────────────────────────────────

const dateFmt = new Intl.DateTimeFormat('en-US', {
	year: 'numeric',
	month: 'long',
	day: 'numeric',
})

function formatDate(iso) {
	const d = new Date(iso)
	return Number.isNaN(d.getTime()) ? '' : dateFmt.format(d)
}

const SOURCE_LABEL = { noctrinth: 'Noctrinth', modrinth: 'Modrinth App' }

const entriesHtml = all
	.map(
		(e) => `
			<article class="entry" data-source="${e.source}">
				<header class="entry__header">
					<span class="entry__source entry__source--${e.source}">${SOURCE_LABEL[e.source]}</span>
					${e.version ? `<span class="entry__version">${escapeHtml(e.version)}</span>` : ''}
					<time class="entry__date" datetime="${escapeHtml(e.date)}">${formatDate(e.date)}</time>
				</header>
				<div class="entry__body">${renderMarkdown(e.body)}</div>
			</article>`,
	)
	.join('\n')

const CSS = `
	:root {
		--bg: #0e0a14;
		--bg-raised: #181122;
		--surface: #221830;
		--surface-2: #2a1f3a;
		--border: #2d2240;
		--text: #e8e3f3;
		--text-dim: #a59cbb;
		--brand: #8e32f3;
		--brand-soft: rgba(142, 50, 243, 0.18);
		--mr: #1bd96a;
		--cf: #f16436;
	}
	* { box-sizing: border-box; }
	html, body {
		margin: 0;
		padding: 0;
		background: var(--bg);
		color: var(--text);
		font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
		line-height: 1.55;
		-webkit-font-smoothing: antialiased;
	}
	a { color: var(--brand); text-decoration: none; }
	a:hover { text-decoration: underline; }
	code {
		font-family: 'JetBrains Mono', ui-monospace, 'SF Mono', Menlo, monospace;
		background: var(--surface-2);
		padding: 0.1em 0.4em;
		border-radius: 4px;
		font-size: 0.9em;
	}

	.page {
		max-width: 880px;
		margin: 0 auto;
		padding: 3rem 1.5rem 5rem;
	}

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
		font-weight: 700;
	}
	.site-header__text p {
		margin: 0.3rem 0 0;
		color: var(--text-dim);
		font-size: 0.95rem;
	}

	.entry {
		background: var(--bg-raised);
		border: 1px solid var(--border);
		border-radius: 14px;
		padding: 1.5rem 1.75rem;
		margin-bottom: 1.25rem;
	}
	.entry__header {
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		gap: 0.75rem;
		margin-bottom: 1rem;
	}
	.entry__source {
		display: inline-flex;
		align-items: center;
		padding: 0.2rem 0.6rem;
		border-radius: 999px;
		font-size: 0.75rem;
		font-weight: 700;
		letter-spacing: 0.02em;
		text-transform: uppercase;
	}
	.entry__source--noctrinth {
		background: var(--brand-soft);
		color: var(--brand);
	}
	.entry__source--modrinth {
		background: rgba(27, 217, 106, 0.15);
		color: var(--mr);
	}
	.entry__version {
		font-family: 'JetBrains Mono', ui-monospace, monospace;
		font-size: 0.95rem;
		color: var(--text);
		font-weight: 600;
	}
	.entry__date {
		margin-left: auto;
		color: var(--text-dim);
		font-size: 0.85rem;
	}
	.entry__body h3 {
		font-size: 1rem;
		font-weight: 700;
		color: var(--text);
		margin: 1.25rem 0 0.5rem;
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}
	.entry__body h3:first-child { margin-top: 0; }
	.entry__body p {
		margin: 0.5rem 0;
		color: var(--text);
	}
	.entry__body ul {
		margin: 0.4rem 0 0.8rem;
		padding-left: 1.25rem;
	}
	.entry__body li {
		margin: 0.3rem 0;
	}
	.entry__body strong { color: var(--text); }

	.footer {
		margin-top: 3rem;
		padding-top: 1.5rem;
		border-top: 1px solid var(--border);
		text-align: center;
		color: var(--text-dim);
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
	<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&family=JetBrains+Mono:wght@400;500&display=swap">
	<style>${CSS}</style>
</head>
<body>
	<div class="page">
		<header class="site-header">
			<img class="site-header__logo" src="logo.svg" alt="Noctrinth">
			<div class="site-header__text">
				<h1>Noctrinth Changelog</h1>
				<p>Combined release notes for Noctrinth and upstream Modrinth App, newest first.</p>
			</div>
		</header>

		<main>${entriesHtml}</main>

		<footer class="footer">
			Generated from the in-app changelog · <a href="https://github.com/Everelsu/noctrinth" rel="noopener">Source on GitHub</a>
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
