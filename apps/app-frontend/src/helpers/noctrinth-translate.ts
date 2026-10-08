/**
 * Translates project descriptions into the reader's language.
 *
 * The idea is Celestial-Launcher's, which offers four keyless services and
 * starts on Tencent's, the one reachable from mainland China. Here there are
 * two, picked by what answers from where Noctrinth's players are: Google's
 * keyless endpoint, which takes several texts a request, keeps HTML intact
 * and says what language it detected; and DeepL on the player's own free key,
 * which translates better and does the same. Microsoft's keyless endpoint, the
 * other one Celestial relies on, no longer hands out tokens; MyMemory caps a
 * day at less than one long description.
 *
 * What comes back is another service's HTML, so it goes through the same
 * sanitiser the description itself does before anything renders it.
 */
import { configuredXss } from '@modrinth/utils'
import { reactive, watch } from 'vue'

import { proxiedFetch } from '@/helpers/proxy-fetch'

export type TranslationProviderId = 'google' | 'deepl'

export interface TranslationSettings {
	/** The button over descriptions. Off, and nothing is translated at all. */
	enabled: boolean
	provider: TranslationProviderId
	deeplKey: string
	/** `app` follows the launcher's language. */
	target: string
	/** Translate as soon as a description opens. */
	auto: boolean
	/** Ask what language a description is in first, and offer nothing if it is one of these. */
	precheck: boolean
	/** Languages the reader reads anyway. */
	skip: string[]
}

export interface TranslatedHtml {
	html: string
	/** The language the service decided the original was in. */
	sourceLanguage: string
	provider: TranslationProviderId
}

const STORAGE_KEY = 'noctrinth-translation'
const LEGACY_AUTO_KEY = 'noctrinth-auto-translate'

const DEFAULTS: TranslationSettings = {
	enabled: true,
	provider: 'google',
	deeplKey: '',
	target: 'app',
	auto: false,
	precheck: true,
	skip: [],
}

function load(): TranslationSettings {
	try {
		const stored = JSON.parse(
			localStorage.getItem(STORAGE_KEY) ?? 'null',
		) as Partial<TranslationSettings> | null
		const legacyAuto = localStorage.getItem(LEGACY_AUTO_KEY) === 'true'
		return { ...DEFAULTS, auto: legacyAuto, ...stored }
	} catch {
		return { ...DEFAULTS }
	}
}

/** Kept in local storage: it is the interface's alone and read before anything asks the backend. */
export const translationSettings = reactive<TranslationSettings>(load())

watch(
	translationSettings,
	(settings) => {
		try {
			localStorage.setItem(STORAGE_KEY, JSON.stringify(settings))
		} catch (error) {
			console.warn('Failed to remember the translation settings:', error)
		}
	},
	{ deep: true },
)

/** Languages offered as a target and in the skip list, beyond the launcher's own. */
export const COMMON_LANGUAGES = [
	'en',
	'ru',
	'uk',
	'be',
	'kk',
	'de',
	'fr',
	'es',
	'pt',
	'it',
	'pl',
	'cs',
	'tr',
	'zh-CN',
	'ja',
	'ko',
]

/** A language code from the launcher's locale: `ru-RU` is `ru`. */
export function languageOfLocale(locale: string): string {
	const [language, region] = locale.split('-')
	if (language === 'zh') return region === 'TW' || region === 'HK' ? 'zh-TW' : 'zh-CN'
	return language
}

export function resolveTarget(locale: string): string {
	return translationSettings.target === 'app'
		? languageOfLocale(locale)
		: translationSettings.target
}

/** The language's own name in the reader's language: "русский", "английский". */
export function languageName(code: string, locale: string): string {
	try {
		return new Intl.DisplayNames([locale], { type: 'language' }).of(code) ?? code
	} catch {
		return code
	}
}

const sameLanguage = (a: string, b: string) =>
	a.toLowerCase().split('-')[0] === b.toLowerCase().split('-')[0]

/** Whether a description in `source` needs no translation for this reader. */
export function needsNoTranslation(source: string, target: string): boolean {
	return (
		sameLanguage(source, target) ||
		translationSettings.skip.some((language) => sameLanguage(language, source))
	)
}

type Translated = { text: string; language: string }

async function googleBatch(texts: string[], target: string): Promise<Translated[]> {
	const body = new URLSearchParams()
	for (const text of texts) body.append('q', text)

	const response = await proxiedFetch(
		`https://translate.googleapis.com/translate_a/t?client=gtx&sl=auto&tl=${encodeURIComponent(target)}&format=html`,
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

/** DeepL's own codes: it wants a region for English and Portuguese, a script for Chinese. */
function deeplTarget(target: string): string {
	const map: Record<string, string> = {
		en: 'EN-US',
		pt: 'PT-BR',
		'zh-CN': 'ZH-HANS',
		'zh-TW': 'ZH-HANT',
	}
	return map[target] ?? target.split('-')[0].toUpperCase()
}

async function deeplBatch(texts: string[], target: string): Promise<Translated[]> {
	const key = translationSettings.deeplKey.trim()
	if (!key) throw new Error('DeepL needs an API key in Settings → Language')

	// A free key ends in ":fx" and has a host of its own.
	const host = key.endsWith(':fx') ? 'api-free.deepl.com' : 'api.deepl.com'
	const response = await proxiedFetch(`https://${host}/v2/translate`, {
		method: 'POST',
		headers: { Authorization: `DeepL-Auth-Key ${key}`, 'Content-Type': 'application/json' },
		body: JSON.stringify({ text: texts, target_lang: deeplTarget(target), tag_handling: 'html' }),
	})
	if (response.status === 403) throw new Error('DeepL did not accept the API key')
	if (response.status === 456) throw new Error("This month's DeepL quota is used up")
	if (!response.ok) throw new Error(`DeepL answered ${response.status}`)

	const payload = (await response.json()) as {
		translations?: { text: string; detected_source_language?: string }[]
	}
	if (payload.translations?.length !== texts.length) {
		throw new Error('DeepL answered in an unexpected shape')
	}
	return payload.translations.map((entry) => ({
		text: entry.text,
		language: (entry.detected_source_language ?? '').toLowerCase(),
	}))
}

const BATCH = { google: googleBatch, deepl: deeplBatch }
/** Characters per request: under Google's limit, and well under DeepL's. */
const REQUEST_CHARS = { google: 4500, deepl: 30000 }

/** Packs pieces into requests under the size limit, in order. */
function packRequests(pieces: string[], limit: number): string[][] {
	const requests: string[][] = []
	let current: string[] = []
	let size = 0
	for (const piece of pieces) {
		if (current.length > 0 && size + piece.length > limit) {
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

const translations = new Map<string, Promise<TranslatedHtml | null>>()

/**
 * Translates rendered, sanitised description HTML. Code blocks are taken out
 * first and put back untouched, so code stays code and does not count against
 * the size of a request. Resolves to `null` when the description needs no
 * translation after all.
 */
export function translateHtml(html: string, target: string): Promise<TranslatedHtml | null> {
	const provider = translationSettings.provider
	const key = `${provider}\n${target}\n${html}`
	let pending = translations.get(key)
	if (!pending) {
		pending = translateUncached(html, target, provider)
		pending.catch(() => translations.delete(key))
		translations.set(key, pending)
	}
	return pending
}

async function translateUncached(
	html: string,
	target: string,
	provider: TranslationProviderId,
): Promise<TranslatedHtml | null> {
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
	for (const request of packRequests(pieces, REQUEST_CHARS[provider])) {
		for (const { text, language } of await BATCH[provider](request, target)) {
			translated.push(text)
			sourceLanguage ||= language
		}
	}

	if (sourceLanguage && sameLanguage(sourceLanguage, target)) return null

	const restored = translated
		.join('\n')
		// Google spaces punctuation off a closing tag: "<a>Fabric</a> ." for "<a>Fabric</a>."
		.replace(/(<\/(?:a|b|i|em|strong|code|span|u|s)>) ([.,;:!?)])/g, '$1$2')
		.replace(
			/<pre data-noctrinth-kept="(\d+)"\s*>\s*<\/pre>/g,
			(_, index: string) => kept[Number(index)] ?? '',
		)

	return { html: configuredXss.process(restored), sourceLanguage, provider }
}

const detections = new Map<string, Promise<string | null>>()

/**
 * What language a description is in, from its opening few hundred characters,
 * so the button can stay away from one the reader reads anyway. Always asks
 * Google, which has a cheap way to ask; `null` when it cannot say, which shows
 * the button rather than hiding it.
 */
export function detectLanguage(html: string): Promise<string | null> {
	const text = new DOMParser()
		.parseFromString(`<body>${html}</body>`, 'text/html')
		.body.textContent?.replace(/\s+/g, ' ')
		.trim()
		.slice(0, 400)
	if (!text) return Promise.resolve(null)

	let pending = detections.get(text)
	if (!pending) {
		pending = (async () => {
			const response = await proxiedFetch(
				'https://translate.googleapis.com/translate_a/single?client=gtx&sl=auto&tl=en&dt=ld',
				{
					method: 'POST',
					headers: { 'Content-Type': 'application/x-www-form-urlencoded;charset=UTF-8' },
					body: new URLSearchParams({ q: text }).toString(),
				},
			)
			if (!response.ok) return null
			const payload = (await response.json()) as unknown[]
			return typeof payload?.[2] === 'string' ? payload[2] : null
		})().catch(() => null)
		detections.set(text, pending)
	}
	return pending
}

/** A short round trip through the chosen service, for the settings' check button. */
export async function testTranslation(
	target: string,
): Promise<{ text: string; milliseconds: number }> {
	const started = performance.now()
	const [result] = await BATCH[translationSettings.provider](
		['This mod adds new ores and tools to the game.'],
		target,
	)
	return { text: result.text, milliseconds: Math.round(performance.now() - started) }
}
