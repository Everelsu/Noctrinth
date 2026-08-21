/**
 * Noctrinth's accent presets.
 *
 * A theme fixes its accent at one colour, and the accent is most of what the
 * interface's own character comes from — so this is a second, smaller choice
 * next to the theme: pick a colour, not a number. Each preset carries two
 * values, one for light backgrounds and one for dark, because a purple that
 * reads on near-black is not the one that reads on white.
 *
 * A preset can colour more than the accent: the surfaces the app is built out
 * of — its background, its panels, its buttons — take the same hue at a fraction
 * of the strength, so picking Ember gives a warm dark app rather than a violet
 * one with orange buttons. They keep their own lightness, which is what the
 * theme's depth and contrast are made of; only the cast over them changes. The
 * same goes for the gradients the right-hand sidebar and the promo cards are
 * painted with, which the fork had written out in purple and which therefore
 * stayed purple whatever the accent was. That half is a setting of its own —
 * turned off, those gradients are drawn from the theme's own surfaces rather
 * than left as the purple they were written as, which is the whole point of
 * asking for the theme's backgrounds back.
 *
 * The loading bar follows the preset either way: it is the accent drawn as a
 * bar, not a background.
 *
 * `theme` is the default and overrides nothing at all, so a build that has
 * never been touched draws exactly what its theme defines.
 *
 * The overrides are written onto the document element as inline custom
 * properties, which beat every theme stylesheet without editing one, and are
 * recomputed when the theme changes.
 */
import { computed, ref, watch } from 'vue'

import { useTheme } from '@/composables/use-theme.ts'

export interface AccentPreset {
	id: string
	/** Shown in the picker; translated through `app.appearance-settings.accent.<id>`. */
	name: string
	/** Drawn on light backgrounds, then on dark ones. */
	light: string
	dark: string
}

export const DEFAULT_ACCENT_PRESET = 'theme'

/**
 * The list, in the order it is drawn.
 *
 * Each pair is the same colour at two lightnesses rather than two colours: the
 * light one is deep enough to read as a label on white, the dark one bright
 * enough to read on the near-black the app mostly is.
 */
export const ACCENT_PRESETS: AccentPreset[] = [
	{ id: 'theme', name: 'Theme', light: '#8e32f3', dark: '#c78aff' },
	{ id: 'amethyst', name: 'Amethyst', light: '#7b2cbf', dark: '#a855f7' },
	{ id: 'nightshade', name: 'Nightshade', light: '#5b45d6', dark: '#8b7bff' },
	{ id: 'midnight', name: 'Midnight', light: '#1f68c0', dark: '#4f9cff' },
	{ id: 'glacier', name: 'Glacier', light: '#0e8fa8', dark: '#3ad2ec' },
	{ id: 'verdant', name: 'Verdant', light: '#0faa4f', dark: '#42e686' },
	{ id: 'lantern', name: 'Lantern', light: '#a86a00', dark: '#ffc857' },
	{ id: 'ember', name: 'Ember', light: '#c9510c', dark: '#ff8f4a' },
	{ id: 'rose', name: 'Rose', light: '#cb2245', dark: '#ff6f95' },
]

/**
 * Every variable that carries the accent, repainted together.
 *
 * `--color-brand` is the accent; the purple variables are the same colour under
 * the name components reach for when they mean "purple" rather than "brand",
 * which in this fork is the same thing. Each keeps whatever alpha its theme
 * gave it, so a highlight stays a highlight and a shadow stays a shadow.
 */
const ACCENT_VARIABLES = [
	'--color-brand',
	'--color-brand-highlight',
	'--color-brand-shadow',
	'--color-purple',
	'--color-purple-highlight',
	'--color-purple-bg',
] as const

/**
 * Every surface the app is built out of, in the order the theme defines them —
 * `--color-bg`, `--color-raised-bg` and the button and divider colours are all
 * written in terms of these, so tinting them tints everything downstream.
 */
const SURFACE_VARIABLES = [
	'--surface-1',
	'--surface-1-5',
	'--surface-2',
	'--surface-2-5',
	'--surface-3',
	'--surface-4',
	'--surface-5',
] as const

/**
 * How much colour a surface is given, as OKLCh chroma.
 *
 * Enough to read as a cast rather than as grey — upstream's own dark surfaces
 * carry about a third of this, in blue — and far short of anything that would
 * compete with the accent drawn on top of it.
 */
const SURFACE_TINT = 0.016

/**
 * The gradients the fork paints its sidebar and promo cards with.
 *
 * These were written out as purple in the theme, so they stayed purple no
 * matter what the accent was — the one part of the window that never followed
 * it. Rebuilt from the preset at the same weights: a wash at low alpha for the
 * backgrounds, and a fade that has to stay opaque, so it is mixed into the page
 * background rather than laid over it.
 */
function brandGradients(accent: string, borderAlpha: number): Record<string, string> {
	return {
		'--brand-gradient-bg': `linear-gradient(0deg, rgb(${accent} / 0.2) 0%, rgb(${accent} / 0.1) 100%)`,
		'--brand-gradient-strong-bg': `linear-gradient(270deg, color-mix(in oklab, var(--color-bg) 88%, rgb(${accent})) 10%, color-mix(in oklab, var(--color-bg) 80%, rgb(${accent})) 100%)`,
		'--brand-gradient-border': `rgb(${accent} / ${borderAlpha})`,
		'--brand-gradient-fade-out-color': `linear-gradient(to bottom, rgb(${accent} / 0) 0%, color-mix(in oklab, var(--color-bg) 86%, rgb(${accent})) 80%)`,
	}
}

/**
 * The same gradients with the colour taken out of them.
 *
 * Turning the tint off has to mean the backgrounds match the theme, and these
 * do not: they are the fork's own purple, written into the theme by hand, so
 * leaving them alone would leave a violet sidebar beside a blue accent. Drawn
 * from the theme's own surfaces instead — the same panel colour as everything
 * else, which is what "the theme's background" means.
 */
function neutralGradients(): Record<string, string> {
	return {
		'--brand-gradient-bg': 'linear-gradient(0deg, var(--surface-2) 0%, var(--surface-1-5) 100%)',
		'--brand-gradient-strong-bg':
			'linear-gradient(270deg, var(--surface-1-5) 10%, var(--surface-2) 100%)',
		'--brand-gradient-border': 'var(--color-button-border)',
		'--brand-gradient-fade-out-color':
			'linear-gradient(to bottom, rgb(0 0 0 / 0) 0%, var(--surface-2) 80%)',
	}
}

/**
 * The bar that runs along the top of the window while something loads.
 *
 * Its far end was a fixed lilac, so the bar left the accent behind halfway
 * across and finished in purple. It runs from the accent to a lighter cast of
 * itself on dark themes and a deeper one on light, which is the shape the
 * theme's own gradient had.
 */
function loadingBarGradient(accent: string, active: string): string {
	const far =
		active === 'light'
			? `color-mix(in oklab, rgb(${accent}) 65%, black)`
			: `color-mix(in oklab, rgb(${accent}) 60%, white)`
	return `linear-gradient(to right, rgb(${accent}) 0%, ${far} 100%)`
}

const GRADIENT_VARIABLES = [
	'--brand-gradient-bg',
	'--brand-gradient-strong-bg',
	'--brand-gradient-border',
	'--brand-gradient-fade-out-color',
	'--loading-bar-gradient',
] as const

const HEX = /^#([0-9a-f]{3}|[0-9a-f]{6})$/i
const RGB = /^rgba?\(([^)]+)\)$/i

function parseHex(value: string): [number, number, number] | null {
	const hex = HEX.exec(value.trim())
	if (!hex) return null

	const digits =
		hex[1].length === 3
			? hex[1]
					.split('')
					.map((digit) => digit + digit)
					.join('')
			: hex[1]
	const number = Number.parseInt(digits, 16)
	return [(number >> 16) & 255, (number >> 8) & 255, number & 255]
}

/** The alpha a theme gave a variable, so repainting it does not flatten it. */
function alphaOf(value: string): number {
	const rgb = RGB.exec(value.trim())
	if (!rgb) return 1

	const parts = rgb[1]
		.split(/[\s,/]+/)
		.filter(Boolean)
		.map(Number)
	const alpha = parts[3]
	return Number.isFinite(alpha) ? alpha : 1
}

function toLinear(channel: number): number {
	return channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4
}

/** sRGB → OKLab, following Björn Ottosson's published matrices. */
function toOklab([red, green, blue]: [number, number, number]): [number, number, number] {
	const r = toLinear(red / 255)
	const g = toLinear(green / 255)
	const b = toLinear(blue / 255)

	const l = Math.cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b)
	const m = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b)
	const s = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b)

	return [
		0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s,
		1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s,
		0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s,
	]
}

function hueOf(color: [number, number, number]): number {
	const [, a, b] = toOklab(color)
	return (Math.atan2(b, a) * 180) / Math.PI
}

function lightnessOf(color: [number, number, number]): number {
	return toOklab(color)[0]
}

function round(value: number, places: number): number {
	const factor = 10 ** places
	return Math.round(value * factor) / factor
}

export function findAccentPreset(id: string): AccentPreset | undefined {
	return ACCENT_PRESETS.find((preset) => preset.id === id)
}

const presetId = ref(DEFAULT_ACCENT_PRESET)
const tintBackground = ref(true)
const theme = useTheme()

/** The colour a preset is drawn in under the theme currently on screen. */
export function accentColorFor(preset: AccentPreset, active: string = theme.active): string {
	return active === 'light' ? preset.light : preset.dark
}

function apply(id: string, tint: boolean): void {
	const html = document.documentElement

	// Cleared first, so what is read back is the theme's own colour and not the
	// last thing this wrote. Also the whole of the work for `theme`.
	for (const variable of [...ACCENT_VARIABLES, ...SURFACE_VARIABLES, ...GRADIENT_VARIABLES]) {
		html.style.removeProperty(variable)
	}

	const preset = findAccentPreset(id)
	if (!preset || preset.id === DEFAULT_ACCENT_PRESET) return

	const rgb = parseHex(accentColorFor(preset))
	if (!rgb) return

	const [red, green, blue] = rgb
	const styles = getComputedStyle(html)

	for (const variable of ACCENT_VARIABLES) {
		const alpha = alphaOf(styles.getPropertyValue(variable))
		html.style.setProperty(
			variable,
			alpha >= 1 ? `rgb(${red} ${green} ${blue})` : `rgb(${red} ${green} ${blue} / ${alpha})`,
		)
	}

	// The loading bar is the accent's own, so it follows the preset whether or
	// not the backgrounds do.
	const accent = `${red} ${green} ${blue}`
	html.style.setProperty('--loading-bar-gradient', loadingBarGradient(accent, theme.active))

	if (!tint) {
		for (const [variable, value] of Object.entries(neutralGradients())) {
			html.style.setProperty(variable, value)
		}
		return
	}

	// The surfaces keep their lightness — that is the theme's depth and its
	// contrast — and are given the preset's hue at a fraction of its strength.
	const hue = round(hueOf(rgb), 2)
	for (const variable of SURFACE_VARIABLES) {
		const surface = parseHex(styles.getPropertyValue(variable))
		if (!surface) continue

		const lightness = lightnessOf(surface)
		html.style.setProperty(variable, `oklch(${round(lightness, 4)} ${SURFACE_TINT} ${hue})`)
	}

	const borderAlpha = alphaOf(styles.getPropertyValue('--brand-gradient-border'))
	for (const [variable, value] of Object.entries(brandGradients(accent, borderAlpha))) {
		html.style.setProperty(variable, value)
	}
}

// Each theme carries its own alphas, and a preset is drawn differently on light
// than on dark, so the override is recomputed rather than kept.
watch([presetId, tintBackground, () => theme.active], ([id, tint]) => apply(id, tint), {
	immediate: true,
})

export function useAccentPreset() {
	return {
		id: presetId,
		tintBackground,
		preset: computed(() => findAccentPreset(presetId.value) ?? ACCENT_PRESETS[0]),
	}
}

/** Sets the accent without going through the settings tab — used at startup. */
export function setAccentPreset(id: string | null | undefined, tint = true): void {
	presetId.value = id && findAccentPreset(id) ? id : DEFAULT_ACCENT_PRESET
	tintBackground.value = tint
}
