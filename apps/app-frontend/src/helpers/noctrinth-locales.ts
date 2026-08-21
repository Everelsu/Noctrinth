/**
 * Noctrinth's own translations, kept out of the upstream catalogues.
 *
 * `src/locales/<code>/index.json` is upstream's file, pulled from Modrinth's
 * Crowdin. Editing it — to name the fork, or to translate a string Crowdin has
 * not covered yet — put a fork change in the middle of a file that upstream
 * rewrites wholesale, which is a merge conflict on every sync. The fork's
 * strings live in `src/locales-noctrinth/<code>/` instead, so the upstream
 * files can be taken as they come.
 *
 * Two files per locale, differing only in who wins:
 *
 * - `messages.json` — the fork's own. Strings for features Modrinth does not
 *   have, and the handful of upstream strings Noctrinth says differently (its
 *   own name, mostly). These are applied over the upstream catalogue.
 * - `fallback.json` — a stand-in for upstream. Translations of *upstream's*
 *   strings for a locale Crowdin has not finished, used only where the
 *   upstream catalogue has nothing. When a translation does land upstream, it
 *   is the one that shows, without anything here needing to be deleted.
 */
import { transformCrowdinMessages } from '@modrinth/ui'
import type { CrowdinMessages } from '@modrinth/ui'

export type LocaleModules = Record<string, { default: CrowdinMessages }>

/** `./locales-noctrinth/ru-RU/messages.json` → `ru-RU`. */
function localeOf(path: string): string | null {
	return /\/([^/]+)\/[^/]+\.json$/.exec(path)?.[1] ?? null
}

/**
 * Overlays the fork's messages onto the upstream catalogue.
 *
 * Locales are only extended, never introduced: a fork file for a locale the
 * app does not carry is ignored, the same way `buildLocaleMessages` ignores
 * one that is not in `LOCALES`.
 */
export function applyNoctrinthMessages(
	messages: Record<string, Record<string, string>>,
	overrides: LocaleModules,
	fallbacks: LocaleModules,
): Record<string, Record<string, string>> {
	for (const [path, module] of Object.entries(fallbacks)) {
		const locale = localeOf(path)
		const catalogue = locale ? messages[locale] : undefined
		if (!catalogue) continue

		for (const [key, message] of Object.entries(transformCrowdinMessages(module.default))) {
			catalogue[key] ??= message
		}
	}

	for (const [path, module] of Object.entries(overrides)) {
		const locale = localeOf(path)
		const catalogue = locale ? messages[locale] : undefined
		if (!catalogue) continue

		Object.assign(catalogue, transformCrowdinMessages(module.default))
	}

	return messages
}
