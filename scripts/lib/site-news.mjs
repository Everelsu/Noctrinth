// Noctrinth's news: articles, as opposed to the changelog's release notes.
//
// One folder per article under news/, the way Modrinth's own blog keeps them
// (packages/blog): `article.md` with a title, summary and date at the top, an
// optional `article.ru.md` in Russian, a `thumbnail.svg` (or .webp/.png/.jpg) and
// any pictures the text shows. The site gets a page per article and a feed in
// the same shape as modrinth.com/news/feed/articles.json, which is what the
// launcher reads to put these in its News sidebar next to Modrinth's.

import { cpSync, existsSync, readdirSync, readFileSync, statSync } from 'node:fs'
import { join } from 'node:path'

import { escapeHtml, parseFrontmatter, renderMarkdown } from './site-markdown.mjs'
import { dateTag, icon, SITE_URL, t } from './site-style.mjs'

export const NEWS_DIR = 'news'
const THUMBNAIL_NAMES = ['thumbnail.svg', 'thumbnail.webp', 'thumbnail.png', 'thumbnail.jpg']

function readArticleFile(path) {
	if (!existsSync(path)) return null
	const { data, content } = parseFrontmatter(readFileSync(path, 'utf-8'))
	return { ...data, content }
}

/** Every article, newest first. Fails the build on one missing what a card needs. */
export function readArticles(dir = NEWS_DIR) {
	if (!existsSync(dir)) return []

	const articles = []
	for (const slug of readdirSync(dir)) {
		const folder = join(dir, slug)
		if (!statSync(folder).isDirectory()) continue

		const en = readArticleFile(join(folder, 'article.md'))
		if (!en) continue
		const ru = readArticleFile(join(folder, 'article.ru.md'))
		if (!en.title || !en.summary || !en.date) {
			console.error(`News article ${slug} needs a title, summary and date at the top of article.md`)
			process.exit(1)
		}

		const thumbnail = THUMBNAIL_NAMES.find((name) => existsSync(join(folder, name)))
		articles.push({
			slug,
			folder,
			en,
			ru,
			date: en.date,
			thumbnail,
			unlisted: en.unlisted === 'true',
		})
	}

	return articles
		.filter((article) => !article.unlisted)
		.sort((a, b) => new Date(b.date) - new Date(a.date))
}

/** The same text in both languages, the Russian one falling back to English. */
function both(article, field, render = escapeHtml) {
	const en = render(article.en[field] ?? '')
	const ru = article.ru?.[field] ? render(article.ru[field]) : en
	return en === ru ? en : t(en, ru)
}

function thumbnailUrl(article, prefix) {
	return article.thumbnail
		? `${prefix}news/article/${article.slug}/${article.thumbnail}`
		: `${prefix}assets/social-preview.png`
}

/** A card the way Modrinth's NewsArticleCard draws one. */
export function articleCard(article, prefix) {
	return `
		<a class="news-card" href="${prefix}news/article/${article.slug}/">
			<article>
				<img src="${thumbnailUrl(article, prefix)}" alt="" loading="lazy">
				<h3>${both(article, 'title')}</h3>
				<p>${both(article, 'summary')}</p>
				<span class="news-card__date">${dateTag(article.date)}</span>
			</article>
		</a>`
}

export const NEWS_CSS = `
	.news-header { display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between; gap: 1rem; padding-top: 1.5rem; }
	.news-header h1 { margin: 0; font-size: 1.875rem; font-weight: 600; color: var(--text-contrast); }
	.news-header h1 a { color: inherit; }
	.icon-buttons { display: flex; gap: 0.5rem; }
	.icon-buttons .btn { width: 2.25rem; padding: 0; }

	.featured { margin-top: 1.5rem; padding-block: 1rem; }
	.featured a { display: block; color: inherit; transition: filter 0.15s ease, transform 0.1s ease; }
	.featured a:hover { filter: brightness(1.25); text-decoration: none; }
	.featured a:hover h3 { text-decoration: underline; }
	.featured a:active { transform: scale(0.99); }
	.featured article { display: flex; flex-wrap: wrap; gap: 1rem; }
	.featured img { flex: 1; min-width: 0; width: 100%; aspect-ratio: 16 / 9; object-fit: cover; border-radius: 1rem; border: 1px solid var(--button-border); }
	.featured__content { flex: 1; min-width: 16rem; display: flex; flex-direction: column; gap: 0.5rem; }
	.featured__content .eyebrow { margin: 0; font-weight: 700; color: var(--text); }
	.featured__content h3 { margin: 0; font-size: 1.875rem; line-height: 1.2; color: var(--text-contrast); }
	.featured__content p { margin: 0; font-size: 1.125rem; line-height: 1.3; }
	.featured__content .news-card__date { margin-top: auto; color: var(--text-secondary); }

	.news-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(250px, 1fr)); gap: 1rem; margin-top: 1rem; }
	.news-card { display: flex; color: inherit; transition: filter 0.15s ease, transform 0.1s ease; }
	.news-card:hover { filter: brightness(1.25); text-decoration: none; }
	.news-card:hover h3 { text-decoration: underline; }
	.news-card:active { transform: scale(0.99); }
	.news-card article { display: flex; flex-direction: column; gap: 0.6rem; width: 100%; }
	.news-card img { width: 100%; aspect-ratio: 16 / 9; object-fit: cover; border-radius: 0.75rem; border: 1px solid var(--button-border); }
	.news-card h3 { margin: 0.4rem 0 0; font-size: 1rem; line-height: 1.25; color: var(--text-contrast); }
	.news-card p { margin: 0; font-size: 0.875rem; line-height: 1.3; color: var(--text); }
	.news-card__date { margin-top: auto; font-size: 0.875rem; color: var(--text-secondary); }

	.article { display: flex; flex-direction: column; gap: 1rem; margin-top: 1.5rem; }
	.article h2.article__title { margin: 0; font-size: clamp(1.5rem, 4vw, 2.25rem); font-weight: 600; line-height: 1.15; color: var(--text-contrast); }
	.article__summary { margin: 0; font-size: 1.125rem; line-height: 1.35; }
	.article__date { color: var(--text-secondary); }
	.article__cover { width: 100%; aspect-ratio: 16 / 9; object-fit: cover; border-radius: 1rem; border: 1px solid var(--button-border); }
	.markdown { font-size: 1.05rem; line-height: 1.7; }
	.markdown h2, .markdown h3 { color: var(--text-contrast); margin: 2rem 0 0.5rem; line-height: 1.25; }
	.markdown h2 { font-size: 1.5rem; padding-bottom: 0.4rem; border-bottom: 1px solid var(--divider); }
	.markdown h3 { font-size: 1.2rem; }
	.markdown p { margin: 0.75rem 0; }
	.markdown ul { margin: 0.75rem 0; padding-left: 1.4rem; }
	.markdown li { margin: 0.35rem 0; }
	.markdown strong { color: var(--text-contrast); }
	.markdown figure { margin: 1.25rem 0; }
	.markdown figure img, .markdown p img { border-radius: 0.75rem; border: 1px solid var(--button-border); }
	.markdown blockquote { margin: 1rem 0; padding: 0.25rem 1rem; border-left: 3px solid var(--brand); background: var(--surface-2); border-radius: 0 0.75rem 0.75rem 0; }
	.article__back { margin-top: 2.5rem; padding-top: 1.5rem; border-top: 1px solid var(--divider); }
	.empty { margin-top: 3rem; text-align: center; color: var(--text-secondary); }

	@media (max-width: 640px) {
		.featured article { flex-direction: column; }
		.featured__content { min-width: 0; }
	}
`

function newsHeader(prefix, titleLink) {
	const title = t('News', 'Новости')
	return `
	<div class="container news-header">
		<h1>${titleLink ? `<a href="${prefix}news/">${title}</a>` : title}</h1>
		<div class="icon-buttons">
			<a class="btn" href="${prefix}news/feed/rss.xml" aria-label="RSS" title="RSS">${icon('rss')}</a>
			<a class="btn" href="${prefix}changelog/" aria-label="Changelog" title="Changelog">${icon('changelog')}</a>
		</div>
	</div>`
}

/** The news index: the newest article featured on the brand band, the rest in a grid. */
export function renderNewsIndex(articles) {
	const prefix = '../'
	if (articles.length === 0) {
		return `${newsHeader(prefix, false)}<p class="container empty">${t('No news yet.', 'Новостей пока нет.')}</p>`
	}

	const [featured, ...rest] = articles
	return `
	${newsHeader(prefix, false)}
	<div class="full-width-bg featured">
		<div class="container">
			<a href="${prefix}news/article/${featured.slug}/">
				<article>
					<img src="${thumbnailUrl(featured, prefix)}" alt="">
					<div class="featured__content">
						<p class="eyebrow">${t('Featured article', 'Главная новость')}</p>
						<h3>${both(featured, 'title')}</h3>
						<p>${both(featured, 'summary')}</p>
						<span class="news-card__date">${dateTag(featured.date)}</span>
					</div>
				</article>
			</a>
		</div>
	</div>
	${
		rest.length
			? `<div class="container" style="margin-top:1.5rem">
		<h2 class="section-heading">${t('More articles', 'Ещё статьи')}</h2>
		<div class="news-grid">${rest.map((article) => articleCard(article, prefix)).join('')}</div>
	</div>`
			: ''
	}`
}

/** One article's page. Its own pictures sit beside it, so their paths stay as written. */
export function renderArticle(article) {
	const prefix = '../../../'
	const body = (lang) => renderMarkdown(lang.content)
	const content =
		article.ru && article.ru.content.trim()
			? `<div class="i18n" lang="en">${body(article.en)}</div><div class="i18n" lang="ru">${body(article.ru)}</div>`
			: body(article.en)

	return `
	${newsHeader(prefix, true)}
	<div class="container">
		<article class="article">
			<h2 class="article__title">${both(article, 'title')}</h2>
			<p class="article__summary">${both(article, 'summary')}</p>
			<span class="article__date">${dateTag(article.date)}</span>
			${article.thumbnail ? `<img class="article__cover" src="${article.thumbnail}" alt="">` : ''}
			<div class="markdown">${content}</div>
			<div class="article__back"><a class="btn" href="${prefix}news/">← ${t('All news', 'Все новости')}</a></div>
		</article>
	</div>`
}

/**
 * The feed the launcher reads, in modrinth.com/news/feed/articles.json's shape.
 *
 * `title_ru` and `summary_ru` are Noctrinth's own additions, for a launcher in
 * Russian; anything that only knows Modrinth's shape reads past them.
 */
export function newsFeed(articles) {
	return {
		articles: articles.map((article) => ({
			title: article.en.title,
			summary: article.en.summary,
			thumbnail: article.thumbnail
				? `${SITE_URL}/news/article/${article.slug}/${article.thumbnail}`
				: `${SITE_URL}/assets/social-preview.png`,
			date: article.date,
			link: `${SITE_URL}/news/article/${article.slug}/`,
			...(article.ru?.title ? { title_ru: article.ru.title } : {}),
			...(article.ru?.summary ? { summary_ru: article.ru.summary } : {}),
		})),
	}
}

export function rssFeed(articles) {
	const items = articles
		.map(
			(article) => `
		<item>
			<title>${escapeHtml(article.en.title)}</title>
			<link>${SITE_URL}/news/article/${article.slug}/</link>
			<guid>${SITE_URL}/news/article/${article.slug}/</guid>
			<pubDate>${new Date(article.date).toUTCString()}</pubDate>
			<description>${escapeHtml(article.en.summary)}</description>
		</item>`,
		)
		.join('')
	return `<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
	<channel>
		<title>Noctrinth News</title>
		<link>${SITE_URL}/news/</link>
		<description>News from Noctrinth, a Modrinth App fork.</description>${items}
	</channel>
</rss>
`
}

/** Each article's folder, pictures and all, copied to where its page is. */
export function copyArticleAssets(articles, outDir) {
	for (const article of articles) {
		cpSync(article.folder, join(outDir, 'news', 'article', article.slug), {
			recursive: true,
			filter: (src) => !src.endsWith('.md'),
		})
	}
}
