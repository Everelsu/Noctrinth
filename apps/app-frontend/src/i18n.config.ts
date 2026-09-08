import {
	buildLocaleMessages,
	createMessageCompiler,
	type CrowdinMessages,
	LOCALES,
} from '@modrinth/ui'
import englishUi from '@modrinth/ui/src/locales/en-US/index.json'
import { createI18n } from 'vue-i18n'

import { applyNoctrinthMessages } from '@/helpers/noctrinth-locales'

import englishApp from './locales/en-US/index.json'

const localeUrls = import.meta.glob<string>('./locales/*/index.json', {
	eager: true,
	query: '?url',
	import: 'default',
})
const uiLocaleUrls = import.meta.glob<string>('../../../packages/ui/src/locales/*/index.json', {
	eager: true,
	query: '?url',
	import: 'default',
})

// Noctrinth's own strings, kept in their own folder so upstream's catalogues
// can be taken exactly as they come. See helpers/noctrinth-locales.ts.
const noctrinthOverrides = import.meta.glob<{ default: CrowdinMessages }>(
	'./locales-noctrinth/*/messages.json',
	{ eager: true },
)
const noctrinthFallbacks = import.meta.glob<{ default: CrowdinMessages }>(
	'./locales-noctrinth/*/fallback.json',
	{ eager: true },
)

const i18n = createI18n({
	legacy: false,
	locale: 'en-US',
	fallbackLocale: 'en-US',
	messageCompiler: createMessageCompiler(),
	missingWarn: false,
	fallbackWarn: false,
	messages: applyNoctrinthMessages(
		buildLocaleMessages({
			'./app/en-US/index.json': { default: englishApp },
			'./ui/en-US/index.json': { default: englishUi },
		}),
		noctrinthOverrides,
		noctrinthFallbacks,
	),
})

const pendingLocales = new Map<string, Promise<Record<string, string>>>()
let localeRequest = 0

async function fetchMessages(url: string): Promise<CrowdinMessages> {
	const response = await fetch(url)
	if (!response.ok) throw new Error(`Could not load translations: ${response.status}`)
	return response.json()
}

export async function setLocale(requestedLocale: string): Promise<void> {
	const request = ++localeRequest
	const locale = LOCALES.some((candidate) => candidate.code === requestedLocale)
		? requestedLocale
		: 'en-US'
	if (locale !== 'en-US' && locale !== i18n.global.locale.value) {
		let pending = pendingLocales.get(locale)
		if (!pending) {
			pending = Promise.all([
				fetchMessages(localeUrls[`./locales/${locale}/index.json`]),
				fetchMessages(uiLocaleUrls[`../../../packages/ui/src/locales/${locale}/index.json`]),
			])
				.then(
					([app, ui]) =>
						// The fork's strings go on as the catalogue is built, which is
						// once per locale now rather than once at startup.
						applyNoctrinthMessages(
							buildLocaleMessages({
								[`./app/${locale}/index.json`]: { default: app },
								[`./ui/${locale}/index.json`]: { default: ui },
							}),
							noctrinthOverrides,
							noctrinthFallbacks,
						)[locale],
				)
				.finally(() => pendingLocales.delete(locale))
			pendingLocales.set(locale, pending)
		}
		const messages = await pending
		if (request !== localeRequest) return
		i18n.global.setLocaleMessage(locale, messages)
	}
	if (request !== localeRequest) return
	const previous = i18n.global.locale.value
	i18n.global.locale.value = locale
	if (previous !== locale && previous !== 'en-US') {
		i18n.global.setLocaleMessage(previous, {})
	}
}

export default i18n
