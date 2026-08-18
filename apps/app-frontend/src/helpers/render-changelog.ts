/**
 * Renders changelog markdown, including local screenshots.
 *
 * The shared `renderString` cannot be used here. It is built for project
 * descriptions written by strangers, so it whitelists a handful of image hosts
 * and pushes everything else through an external proxy — which would send the
 * app's own bundled screenshots out to a third party, or drop them.
 *
 * Changelog entries ship inside the app, so the only thing worth guarding is
 * that a stray tag cannot do anything: images are limited to paths under
 * `/changelog/`, and links to http(s).
 */
import { md } from '@modrinth/utils'
import { FilterXSS } from 'xss'

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
		// Only assets that ship with the app; nothing may point outward.
		if (tag === 'img' && name === 'src') {
			return value.startsWith('/changelog/') ? value : ''
		}
		if (tag === 'a' && name === 'href') {
			return /^https?:\/\//.test(value) ? value : ''
		}
		return value
	},
})

export function renderChangelog(body: string): string {
	return changelogXss.process(md().render(body))
}
