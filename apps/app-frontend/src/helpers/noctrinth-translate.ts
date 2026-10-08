/**
 * Translates project descriptions into the launcher's language.
 *
 * The idea is Celestial-Launcher's, which offers four keyless services and
 * starts on Tencent's, the one reachable from mainland China. Here only
 * Google's keyless endpoint is used: Microsoft's, the other one they rely on,
 * no longer hands out tokens, and Google answers from where Noctrinth's
 * players are. It takes several texts in one request, keeps HTML intact with
 * `format=html`, and says which language it detected, which is how a
 * description already in the reader's language is left alone.
 *
 * What comes back is another service's HTML, so it goes through the same
 * sanitiser the description itself does before anything renders it.
 */
import { configuredXss } from '@modrinth/utils'

import { proxiedFetch } from '@/helpers/proxy-fetch'

const ENDPOINT = 'https://translate.googleapis.com/translate_a/t'
/** Characters per request; Google's limit sits a little above this. */
const REQUEST_CHARS = 4500
const AUTO_KEY = 'noctrinth-auto-translate'

export interface TranslatedHtml {
	html: string
	/** The language Google decided the original was in. */
	sourceLanguage: string
}

/** The language Google is asked for, from the launcher's locale. */
export function translationTarget(locale: string): string {
	const [language, region] = locale.split('-')
	if (language === 'zh') return region === 'TW' || region === 'HK' ? 'zh-TW' : 'zh-CN'
	return language
}

/** The language's own name in the reader's language: "русский", "English". */
export function languageName(code: string, locale: string): string {
	try {
		return new Intl.DisplayNames([locale], { type: 'language' }).of(code) ?? code
	} catch {
		return code
	}
}

export function readAutoTranslate(): boolean {
	try {
		return localStorage.getItem(AUTO_KEY) === 'true'
	} catch {
		return false
	}
}

export function writeAutoTranslate(enabled: boolean): void {
	try {
		localStorage.setItem(AUTO_KEY, String(enabled))
	} catch (error) {
		console.warn('Failed to remember the auto-translate choice:', error)
	}
}

async function requestBatch(
	texts: string[],
	target: string,
): Promise<{ text: string; language: string }[]> {
	const body = new URLSearchParams()
	for (const text of texts) body.append('q', text)

	const response = await proxiedFetch(
		`${ENDPOINT}?client=gtx&sl=auto&tl=${encodeURIComponent(target)}&format=html`,
		{
			method: 'POST',
			headers: { 'Content-Type': 'application/x-www-form-urlencoded;charset=UTF-8' },
			body: body.toString(),
		},
	)
	if (!response.ok) throw new Error(`Google Translate answered ${response.status}`)

	// One text comes back as ["text", "en"]; several as [["text", "en"], ...].
	const payload = (await response.json()) as unknown
	const entries =
		texts.length === 1 && typeof (payload as unknown[])?.[0] === 'string' ? [payload] : payload
	if (!Array.isArray(entries) || entries.length !== texts.length) {
		throw new Error('Google Translate answered in an unexpected shape')
	}
	return entries.map((entry) => {
		const [text, language] = Array.isArray(entry) ? entry : [entry, '']
		if (typeof text !== 'string') throw new Error('Google Translate answered without text')
		return { text, language: typeof language === 'string' ? language : '' }
	})
}

/** Packs pieces into requests under the size limit, in order. */
function packRequests(pieces: string[]): string[][] {
	const requests: string[][] = []
	let current: string[] = []
	let size = 0
	for (const piece of pieces) {
		if (current.length > 0 && size + piece.length > REQUEST_CHARS) {
			requests.push(current)
			current = []
			size = 0
		}
		current.push(piece)
		size += piece.length
	}
	if (current.length > 0) requests.push(current)
	return requests
}

const cache = new Map<string, Promise<TranslatedHtml | null>>()

/**
 * Translates rendered, sanitised description HTML. Code blocks are taken out
 * first and put back untouched, so code stays code and does not count against
 * the size of a request. Resolves to `null` when the description is already
 * in the target language.
 */
export function translateHtml(html: string, target: string): Promise<TranslatedHtml | null> {
	const key = `${target}\n${html}`
	let pending = cache.get(key)
	if (!pending) {
		pending = translateUncached(html, target)
		pending.catch(() => cache.delete(key))
		cache.set(key, pending)
	}
	return pending
}

async function translateUncached(html: string, target: string): Promise<TranslatedHtml | null> {
	const doc = new DOMParser().parseFromString(`<body>${html}</body>`, 'text/html')

	const kept: string[] = []
	for (const block of Array.from(doc.body.querySelectorAll('pre'))) {
		const placeholder = doc.createElement('pre')
		placeholder.setAttribute('data-noctrinth-kept', String(kept.length))
		kept.push(block.outerHTML)
		block.replaceWith(placeholder)
	}

	for (const code of Array.from(doc.body.querySelectorAll('code'))) {
		code.setAttribute('translate', 'no')
	}

	const pieces = Array.from(doc.body.childNodes)
		.map((node) => (node instanceof Element ? node.outerHTML : (node.textContent ?? '')))
		.filter((piece) => piece.trim() !== '')
	if (pieces.length === 0) return null

	const translated: string[] = []
	let sourceLanguage = ''
	for (const request of packRequests(pieces)) {
		for (const { text, language } of await requestBatch(request, target)) {
			translated.push(text)
			sourceLanguage ||= language
		}
	}

	if (sourceLanguage && sourceLanguage.split('-')[0] === target.split('-')[0]) return null

	const restored = translated
		.join('\n')
		// Google spaces punctuation off a closing tag: "<a>Fabric</a> ." for "<a>Fabric</a>."
		.replace(/(<\/(?:a|b|i|em|strong|code|span|u|s)>) ([.,;:!?)])/g, '$1$2')
		.replace(
			/<pre data-noctrinth-kept="(\d+)"\s*>\s*<\/pre>/g,
			(_, index: string) => kept[Number(index)] ?? '',
		)

	return { html: configuredXss.process(restored), sourceLanguage }
}
