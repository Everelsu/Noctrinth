import { LRUCache } from 'lru-cache'

import { injectI18n } from '../providers/i18n'
import { LOCALES } from './i18n.ts'

const formatterCache = new LRUCache<string, Intl.NumberFormat>({ max: 15 })

// `formatNumber(1234567)` → `1,234,567`
export function useFormatNumber() {
	const { locale } = injectI18n()

	function format(value: number | bigint): string {
		const formatter = getStandardFormatter(locale.value)
		return formatter!.format(value)
	}

	return format
}

// `formatCompactNumber(1234567)` → `1.23m`
//
// We bypass Intl's compact notation so the suffix is always the short Latin
// `k` / `m` / `b` rather than locale-specific words (Russian "млн"/"тыс" etc.
// look ugly and inflate the layout). The numeric part still goes through Intl
// so the decimal/thousands separators stay locale-correct (e.g. "1,23m" in ru,
// "1.23m" in en).
//
// Use `formatCompactNumberPlural` over `{(here!), plural, one {...} other {...}}`
export function useCompactNumber() {
	const { locale } = injectI18n()

	function formatCompactNumber(value: number | bigint): string {
		const n = typeof value === 'bigint' ? Number(value) : value
		if (n < 10_000) {
			return getStandardFormatter(locale.value).format(value)
		}
		if (n < 1_000_000) {
			return formatFixed(locale.value, n / 1_000, 1) + 'k'
		}
		if (n < 1_000_000_000) {
			return formatFixed(locale.value, n / 1_000_000, 2) + 'M'
		}
		return formatFixed(locale.value, n / 1_000_000_000, 2) + 'b'
	}

	function formatCompactNumberPlural(value: number | bigint): number | bigint {
		if (value < 10_000) {
			return value
		}
		const currentLocale = locale.value
		const localeDefinition = LOCALES.find((l) => l.code === currentLocale)
		return localeDefinition?.compactNumberPlural ?? NaN
	}

	return { formatCompactNumber, formatCompactNumberPlural }
}

function getStandardFormatter(locale: string): Intl.NumberFormat {
	const cacheKey = `${locale}:standard`
	let formatter = formatterCache.get(cacheKey)
	if (!formatter) {
		formatter = new Intl.NumberFormat(locale)
		formatterCache.set(cacheKey, formatter)
	}
	return formatter
}

function getFixedFormatter(locale: string, maximumFractionDigits: number): Intl.NumberFormat {
	const cacheKey = `${locale}:fixed:${maximumFractionDigits}`
	let formatter = formatterCache.get(cacheKey)
	if (!formatter) {
		formatter = new Intl.NumberFormat(locale, { maximumFractionDigits })
		formatterCache.set(cacheKey, formatter)
	}
	return formatter
}

function formatFixed(locale: string, value: number, maxFractionDigits: number): string {
	return getFixedFormatter(locale, maxFractionDigits).format(value)
}
