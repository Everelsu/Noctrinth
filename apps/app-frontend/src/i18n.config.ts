import { buildLocaleMessages, createMessageCompiler, type CrowdinMessages } from '@modrinth/ui'
import { uiLocaleModulesEager } from '@modrinth/ui/src/locales.eager.ts'
import { createI18n } from 'vue-i18n'

import { applyNoctrinthMessages } from '@/helpers/noctrinth-locales'

const localeModules = import.meta.glob<{ default: CrowdinMessages }>('./locales/*/index.json', {
	eager: true,
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
		buildLocaleMessages(localeModules, uiLocaleModulesEager),
		noctrinthOverrides,
		noctrinthFallbacks,
	),
})

export default i18n
