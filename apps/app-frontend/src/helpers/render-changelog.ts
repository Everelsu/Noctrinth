/**
 * Renders changelog markdown, including the screenshots entries can carry.
 *
 * The shared `renderString` cannot be used here. It is built for project
 * descriptions written by strangers, so it whitelists a handful of image hosts
 * and pushes everything else through an external proxy — which would route the
 * changelog's own screenshots through a third party, or drop them.
 *
 * Entries are either bundled with the app or fetched from the changelog site,
 * so the guard is that a stray tag cannot do anything: images must be under
 * `/changelog/` — which resolves to the site, since screenshots are published
 * there rather than shipped — and links must be http(s).
 */
import { md } from '@modrinth/utils'
import { FilterXSS } from 'xss'

/** Where changelog screenshots are published. */
const SCREENSHOT_BASE = 'https://everelsu.github.io/Noctrinth'

const changelogXss = new FilterXSS({
	whiteList: {
		h1: [],
		h2: [],
		h3: [],
		h4: [],
		p: [],
		ul: [],
		ol: [],
		li: [],
		a: ['href', 'target', 'rel'],
		img: ['src', 'alt', 'title'],
		code: [],
		pre: [],
		strong: [],
		em: [],
		br: [],
		hr: [],
		blockquote: [],
	},
	stripIgnoreTag: true,
	stripIgnoreTagBody: ['script', 'style'],
	safeAttrValue(tag, name, value) {
		// Only the site's own screenshots, whether an entry names them by the
		// path it is written with or by the URL they are published at.
		if (tag === 'img' && name === 'src') {
			if (value.startsWith('/changelog/')) {
				return `${SCREENSHOT_BASE}${value}`
			}
			return value.startsWith(`${SCREENSHOT_BASE}/changelog/`) ? value : ''
		}
		if (tag === 'a' && name === 'href') {
			return /^https?:\/\//.test(value) ? value : ''
		}
		return value
	},
})

export function renderChangelog(body: string): string {
	// Added after sanitising rather than whitelisted, so it cannot be set to
	// anything else by an entry: screenshots load when they are scrolled to,
	// not when the changelog opens.
	return changelogXss.process(md().render(body)).replaceAll('<img ', '<img loading="lazy" ')
}
