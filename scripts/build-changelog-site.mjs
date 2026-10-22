#!/usr/bin/env node
// Build the static GitHub Pages site:
//
//   /                      the front page (scripts/lib/site-home.mjs)
//   /news/                 Noctrinth's news, from news/ (scripts/lib/site-news.mjs)
//   /news/feed/            the same as JSON for the launcher's sidebar, and RSS
//   /changelog/            every Noctrinth and upstream Modrinth App release note
//   /changelog.json        Noctrinth's release notes for the launcher's Changelog tab
//
// Styled after Modrinth's own site, with Noctrinth's purple. Runs on a bare Node
// with no install, which is how the Pages workflow runs it.
//
// Usage:
//   node scripts/build-changelog-site.mjs [output-dir]
//   Defaults to ./site

import { copyFileSync, cpSync, existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'

import { NOCTRINTH_CHANGELOG_SRC, parseEntries } from './lib/changelog-entries.mjs'
import { renderHome } from './lib/site-home.mjs'
import { escapeHtml, renderMarkdown } from './lib/site-markdown.mjs'
import {
	copyArticleAssets,
	NEWS_CSS,
	newsFeed,
	readArticles,
	renderArticle,
	renderNewsIndex,
	rssFeed,
} from './lib/site-news.mjs'
import { page, t } from './lib/site-style.mjs'

const OUT_DIR = process.argv[2] ?? 'site'
const MODRINTH_SRC = 'packages/blog/changelog.ts'
const LOGO_SRC = 'apps/app/icons/noctrinth.svg'
/** Screenshots, referenced from entries as `/changelog/screenshots/<file>`. */
const SCREENSHOT_DIR = 'apps/app-frontend/src/changelog/screenshots'
/** Where those land on the site, and the prefix an entry has to use. */
const SCREENSHOT_PATH = 'changelog/screenshots'
/** Pictures the front page shows, copied next to it under assets/. */
const SITE_ASSETS = [
	'.github/assets/noctrinth-banner.svg',
	'.github/assets/social-preview.png',
	'.github/assets/screenshots/accents.gif',
	'.github/assets/screenshots/accounts.png',
	'.github/assets/screenshots/curseforge-import.png',
	'.github/assets/screenshots/ely-by-skins.png',
	'.github/assets/screenshots/library-search.gif',
	'.github/assets/screenshots/skin-preview.gif',
	'.github/assets/screenshots/translate.gif',
]
const VERSION = readFileSync('VERSION', 'utf-8').trim().split('+')[0]

const noctrinth = parseEntries(readFileSync(NOCTRINTH_CHANGELOG_SRC, 'utf-8')).map((entry) => ({
	...entry,
	source: 'noctrinth',
}))
const modrinth = parseEntries(readFileSync(MODRINTH_SRC, 'utf-8'))
	.filter((e) => e.product === 'app')
	.map((e) => ({ ...e, source: 'modrinth' }))

const all = [...noctrinth, ...modrinth]
	.filter((e) => e.date)
	.sort((a, b) => new Date(b.date) - new Date(a.date))

/**
 * Turns an entry's `/changelog/...` reference into one the changelog page can
 * load. The site lives under `/<repo>/` on GitHub Pages, so a leading slash
 * would point at the domain root; the page is in `changelog/`, so its own
 * screenshots are next to it and anything else is one level up.
 */
function changelogRelative(url) {
	if (url.startsWith('/changelog/')) return url.slice('/changelog/'.length)
	return url.startsWith('/') ? `../${url.slice(1)}` : url
}

const longDateFmt = new Intl.DateTimeFormat('en-US', {
	year: 'numeric',
	month: 'long',
	day: 'numeric',
})

function formatLongDate(iso) {
	const d = new Date(iso)
	return Number.isNaN(d.getTime()) ? '' : longDateFmt.format(d)
}

const oneWeekMs = 7 * 24 * 60 * 60 * 1000
const isRecent = (iso) => Date.now() - new Date(iso).getTime() < oneWeekMs

const SOURCE_LABEL = { noctrinth: 'Noctrinth', modrinth: 'Modrinth App' }

/** The entry's text; with a Russian one too, the site's EN/RU switch picks between them. */
function renderBody(entry) {
	const body = (text, lang) =>
		`<div class="entry__body markdown${lang ? ' i18n' : ''}"${lang ? ` lang="${lang}"` : ''}>${renderMarkdown(text, changelogRelative)}</div>`
	return entry.body_ru ? body(entry.body, 'en') + body(entry.body_ru, 'ru') : body(entry.body)
}

function renderEntry(entry, index) {
	// Filled with the source's colour when recent or first, hollow otherwise —
	// the way Modrinth's changelog timeline marks them.
	const dotClass =
		isRecent(entry.date) || index === 0 ? `entry__dot--${entry.source}` : 'entry__dot--muted'
	const id = `${entry.source === 'noctrinth' ? 'v' : 'modrinth-'}${entry.version ?? entry.date}`

	return `
		<div class="entry" id="${escapeHtml(id)}" data-source="${entry.source}">
			<div class="entry__head">
				<div class="entry__dot ${dotClass}"></div>
				<h2>
					<span class="entry__type entry__type--${entry.source}">${SOURCE_LABEL[entry.source]}</span>
					<span class="entry__version">${escapeHtml(entry.version ?? formatLongDate(entry.date))}</span>
				</h2>
				<time class="entry__date" datetime="${escapeHtml(entry.date)}" data-relative>${formatLongDate(entry.date)}</time>
			</div>
			${renderBody(entry)}
		</div>`
}

/**
 * Every image an entry points at, checked against what is actually here.
 *
 * A screenshot is only visible once the site carries it, so a typo in a path
 * is invisible until after a deploy. Catching it at build time costs nothing.
 */
function checkScreenshots(entries) {
	const problems = []

	for (const entry of entries) {
		for (const [, url] of [entry.body, entry.body_ru ?? '']
			.join(' ')
			.matchAll(/!\[[^\]]*\]\(([^)]+)\)/g)) {
			if (/^https?:\/\//.test(url)) continue

			const path = url.startsWith('/') ? url.slice(1) : url
			if (!path.startsWith(`${SCREENSHOT_PATH}/`)) {
				problems.push(`${entry.version ?? entry.date}: ${url} is not under /${SCREENSHOT_PATH}/`)
				continue
			}

			const file = join(SCREENSHOT_DIR, path.slice(SCREENSHOT_PATH.length + 1))
			if (!existsSync(file)) {
				problems.push(`${entry.version ?? entry.date}: ${url} has no file at ${file}`)
			}
		}
	}

	if (problems.length > 0) {
		console.error(`Changelog screenshots that would not load:\n  ${problems.join('\n  ')}`)
		process.exit(1)
	}
}

checkScreenshots(noctrinth)

/**
 * A recent entry says how long ago it was ("3 days ago"); the build cannot,
 * since a static page never rebuilds, so it is worked out on load in the page's
 * own language, with the exact date and time on hover.
 */
const RELATIVE_TIME_SCRIPT = `
	(function () {
		function lang() { return document.documentElement.getAttribute('data-lang') === 'ru' ? 'ru-RU' : 'en-US'; }
		function update() {
			var rtf = new Intl.RelativeTimeFormat(lang(), { numeric: 'auto' });
			var longFmt = new Intl.DateTimeFormat(lang(), { year: 'numeric', month: 'long', day: 'numeric' });
			var absFmt = new Intl.DateTimeFormat(lang(), { year: 'numeric', month: 'long', day: 'numeric', hour: 'numeric', minute: '2-digit' });
			var nodes = document.querySelectorAll('time[data-relative]');
			for (var i = 0; i < nodes.length; i++) {
				var date = new Date(nodes[i].getAttribute('datetime'));
				if (isNaN(date.getTime())) continue;
				var day = (date.getTime() - Date.now()) / 86400000;
				var text = longFmt.format(date);
				if (Math.abs(day) < 1) text = rtf.format(Math.round(day * 24), 'hour');
				else if (Math.abs(day) < 7) text = rtf.format(Math.round(day), 'day');
				nodes[i].textContent = text;
				nodes[i].title = absFmt.format(date);
			}
		}
		update();
		setInterval(update, 60000);
		new MutationObserver(update).observe(document.documentElement, { attributes: true, attributeFilter: ['data-lang'] });
	})();
`

/** Modrinth's changelog timeline: a dashed line, a dot per release, the notes in a raised card. */
const CHANGELOG_CSS = `
	.changelog-header { padding-top: 1.5rem; }
	.changelog-header h1 { margin: 0; font-size: 1.875rem; font-weight: 600; color: var(--text-contrast); }
	.changelog-header p { margin: 0.4rem 0 0; color: var(--text-secondary); }
	.filters { display: flex; gap: 0.5rem; margin: 1.25rem 0 1.5rem; flex-wrap: wrap; }
	.filters button { min-height: 2rem; padding: 0.3rem 0.9rem; font-size: 0.875rem; }
	.filters button[aria-pressed="true"] { background: var(--brand-highlight); color: var(--brand); border-color: transparent; }
	html[data-filter="noctrinth"] .entry[data-source="modrinth"],
	html[data-filter="modrinth"] .entry[data-source="noctrinth"] { display: none; }

	.timeline { position: relative; display: flex; flex-direction: column; gap: 1.25rem; padding-bottom: 1.5rem; isolation: isolate; }
	.timeline::before {
		content: '';
		position: absolute;
		left: 6px;
		top: 1rem;
		height: calc(100% - 1rem);
		width: 4px;
		background-image: linear-gradient(to bottom, var(--surface-3) 66%, transparent 0%);
		background-size: 100% 30px;
		background-repeat: repeat-y;
		mask-image: linear-gradient(to bottom, black calc(100% - 8rem), transparent 100%);
		z-index: -1;
	}
	.entry { display: flex; flex-direction: column; scroll-margin-top: 5rem; }
	.entry__head { display: flex; align-items: center; flex-wrap: wrap; gap: 0.5rem 0.9rem; }
	.entry__dot { width: 16px; height: 16px; flex-shrink: 0; border-radius: 50%; border: 2px solid var(--button-border); }
	.entry__dot--noctrinth { background: var(--brand); }
	.entry__dot--modrinth { background: #1bd96a; }
	.entry__dot--muted { background: var(--button-bg); }
	.entry__head h2 { display: flex; align-items: center; gap: 0.6rem; margin: 0; font-size: 1.25rem; font-weight: 800; color: var(--text-contrast); }
	.entry__type { font-size: 0.75rem; font-weight: 700; padding: 0.15rem 0.55rem; border-radius: 999px; }
	.entry__type--noctrinth { background: var(--brand-highlight); color: var(--brand); }
	.entry__type--modrinth { background: rgba(27, 217, 106, 0.15); color: #1bd96a; }
	.entry__date { color: var(--text-secondary); font-size: 0.9rem; }
	.entry__body { margin: 0.75rem 0 0 2rem; padding: 1rem 1.25rem; border-radius: var(--radius-xl); background: var(--surface-2); border: 1px solid var(--button-border); font-size: 0.98rem; }
	.entry__body h3 { margin: 1rem 0 0.25rem; font-size: 0.78rem; text-transform: uppercase; letter-spacing: 0.06em; color: var(--brand); border: 0; }
	.entry__body h3:first-child { margin-top: 0; }
	.entry__body ul { margin: 0.25rem 0; }
	.entry__body img { border-radius: var(--radius-md); }
`

const FILTER_SCRIPT = `
	(function () {
		var buttons = document.querySelectorAll('[data-filter-button]');
		function set(value) {
			document.documentElement.setAttribute('data-filter', value);
			for (var i = 0; i < buttons.length; i++) {
				buttons[i].setAttribute('aria-pressed', String(buttons[i].getAttribute('data-filter-button') === value));
			}
		}
		for (var i = 0; i < buttons.length; i++) {
			buttons[i].addEventListener('click', function () { set(this.getAttribute('data-filter-button')); });
		}
		set('all');
	})();
`

const changelogHtml = page({
	title: 'Noctrinth — Changelog',
	prefix: '../',
	current: 'changelog',
	css: NEWS_CSS + CHANGELOG_CSS,
	body: `
	<main class="container">
		<header class="changelog-header">
			<h1>${t('Changelog', 'Список изменений')}</h1>
			<p>${t(
				'Release notes for Noctrinth and the upstream Modrinth App it follows.',
				'Заметки о релизах Noctrinth и Modrinth App, за которым он идёт.',
			)}</p>
		</header>
		<div class="filters">
			<button class="btn" type="button" data-filter-button="all">${t('All', 'Все')}</button>
			<button class="btn" type="button" data-filter-button="noctrinth">Noctrinth</button>
			<button class="btn" type="button" data-filter-button="modrinth">Modrinth App</button>
		</div>
		<div class="timeline">${all.map(renderEntry).join('\n')}</div>
	</main>`,
	scripts: `<script>${RELATIVE_TIME_SCRIPT}</script><script>${FILTER_SCRIPT}</script>`,
})

const articles = readArticles()

const newsIndexHtml = page({
	title: 'Noctrinth — News',
	prefix: '../',
	current: 'news',
	css: NEWS_CSS,
	body: renderNewsIndex(articles),
})

mkdirSync(join(OUT_DIR, 'changelog'), { recursive: true })
mkdirSync(join(OUT_DIR, 'news', 'feed'), { recursive: true })
mkdirSync(join(OUT_DIR, 'assets'), { recursive: true })

writeFileSync(join(OUT_DIR, 'index.html'), renderHome({ version: VERSION, articles }))
writeFileSync(join(OUT_DIR, 'changelog', 'index.html'), changelogHtml)
writeFileSync(join(OUT_DIR, 'news', 'index.html'), newsIndexHtml)
for (const article of articles) {
	const dir = join(OUT_DIR, 'news', 'article', article.slug)
	mkdirSync(dir, { recursive: true })
	writeFileSync(
		join(dir, 'index.html'),
		page({
			title: `${article.en.title} — Noctrinth`,
			description: article.en.summary,
			prefix: '../../../',
			current: 'news',
			css: NEWS_CSS,
			body: renderArticle(article),
		}),
	)
}
copyArticleAssets(articles, OUT_DIR)
writeFileSync(
	join(OUT_DIR, 'news', 'feed', 'articles.json'),
	JSON.stringify(newsFeed(articles), null, '\t'),
)
writeFileSync(join(OUT_DIR, 'news', 'feed', 'rss.xml'), rssFeed(articles))

// The same entries for the launcher to read, so a release's notes can be
// corrected, or newer ones shown to an older build, without a rebuild. The app
// still ships them too, and falls back to its own copy offline.
writeFileSync(
	join(OUT_DIR, 'changelog.json'),
	JSON.stringify(
		noctrinth.map(({ version, date, body, body_ru }) => ({ version, date, body, body_ru })),
		null,
		'	',
	),
)
copyFileSync(LOGO_SRC, join(OUT_DIR, 'logo.svg'))
for (const asset of SITE_ASSETS) {
	copyFileSync(asset, join(OUT_DIR, 'assets', asset.split('/').pop()))
}

// Screenshots live on the site rather than inside the installer, so an entry
// can carry one without every future release paying for it in download size.
if (existsSync(SCREENSHOT_DIR)) {
	cpSync(SCREENSHOT_DIR, join(OUT_DIR, SCREENSHOT_PATH), { recursive: true })
}

console.log(
	`Wrote the front page, ${articles.length} news articles and ${all.length} changelog entries (${noctrinth.length} Noctrinth) → ${OUT_DIR}/`,
)
