/**
 * Noctrinth's accent presets.
 *
 * A theme fixes its accent at one colour, and the accent is most of what the
 * interface's own character comes from — so this is a second, smaller choice
 * next to the theme: pick a colour, not a number. Each preset carries two
 * values, one for light backgrounds and one for dark, because a purple that
 * reads on near-black is not the one that reads on white.
 *
 * A preset colours more than the accent: the surfaces the app is built out of —
 * its background, its panels, its buttons — are given the same hue at a fraction
 * of the strength, so picking Ember gives a warm dark app rather than a violet
 * one with orange buttons. They keep their own lightness, which is what the
 * theme's depth and contrast are made of; only the cast over them changes.
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
const theme = useTheme()

/** The colour a preset is drawn in under the theme currently on screen. */
export function accentColorFor(preset: AccentPreset, active: string = theme.active): string {
	return active === 'light' ? preset.light : preset.dark
}

function apply(id: string): void {
	const html = document.documentElement

	// Cleared first, so what is read back is the theme's own colour and not the
	// last thing this wrote. Also the whole of the work for `theme`.
	for (const variable of [...ACCENT_VARIABLES, ...SURFACE_VARIABLES]) {
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

	// The surfaces keep their lightness — that is the theme's depth and its
	// contrast — and are given the preset's hue at a fraction of its strength.
	const hue = round(hueOf(rgb), 2)
	for (const variable of SURFACE_VARIABLES) {
		const surface = parseHex(styles.getPropertyValue(variable))
		if (!surface) continue

		const lightness = lightnessOf(surface)
		html.style.setProperty(variable, `oklch(${round(lightness, 4)} ${SURFACE_TINT} ${hue})`)
	}
}

// Each theme carries its own alphas, and a preset is drawn differently on light
// than on dark, so the override is recomputed rather than kept.
watch([presetId, () => theme.active], ([id]) => apply(id), { immediate: true })

export function useAccentPreset() {
	return {
		id: presetId,
		preset: computed(() => findAccentPreset(presetId.value) ?? ACCENT_PRESETS[0]),
	}
}

/** Sets the accent without going through the settings tab — used at startup. */
export function setAccentPreset(id: string | null | undefined): void {
	presetId.value = id && findAccentPreset(id) ? id : DEFAULT_ACCENT_PRESET
}
