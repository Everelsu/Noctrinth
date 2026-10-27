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
 * That part is a setting of its own, for anyone who would rather keep the
 * theme's own greys.
 *
 * Anything the theme already painted in the brand colour follows the preset
 * regardless of that setting — the right-hand sidebar, the promo cards, the
 * dimmer behind a dialog, the loading bar. All four were written out in purple
 * by hand, which is why they used to stay purple behind a blue accent; they
 * were the brand's before and they are the brand's now.
 *
 * `theme` is the default and overrides nothing at all, so a build that has
 * never been touched draws exactly what its theme defines.
 *
 * The overrides are written onto the document element as inline custom
 * properties, which beat every theme stylesheet without editing one, and are
 * recomputed when the theme changes. The choice is also mirrored into local
 * storage, because the settings it really lives in are read from the database a
 * moment after the window opens — long enough for the splash screen to come up
 * in the wrong colour, which is exactly the flash this is meant to avoid.
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
	{ id: 'nightshade', name: 'Nightshade', light: '#5b45d6', dark: '#8b7bff' },
	{ id: 'midnight', name: 'Midnight', light: '#1f68c0', dark: '#4f9cff' },
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
	// The accent on its own, under a name no theme block declares. Anything
	// carrying a theme class of its own — the splash screen does — re-declares
	// every colour the theme defines on itself, which would drop the accent
	// chosen for the window; such a place can reach for this instead.
	'--noctrinth-accent',
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
/**
 * The accent as something to lay *under* content, rather than on it.
 *
 * The theme writes these washes as dark purples — the dimmer behind a dialog is
 * `rgba(80, 35, 130, 0.45)`, not the accent itself — because their job is to
 * take light out of what is behind them. Built from the accent as it comes,
 * a bright preset such as Lantern brightens the window instead of dimming it,
 * and everything under the wash loses its contrast. Each one is therefore mixed
 * toward black by the same amount the theme's own value was, keeping its alpha.
 */
function wash(accent: string, strength: number, alpha: number): string {
	return `color-mix(in oklab, rgb(${accent} / ${alpha}) ${strength}%, rgb(0 0 0 / ${alpha}))`
}

function brandGradients(accent: string, borderAlpha: number): Record<string, string> {
	return {
		'--brand-gradient-bg': `linear-gradient(0deg, ${wash(accent, 25, 0.2)} 0%, ${wash(accent, 55, 0.1)} 100%)`,
		'--brand-gradient-strong-bg': `linear-gradient(270deg, color-mix(in oklab, var(--color-bg) 88%, rgb(${accent})) 10%, color-mix(in oklab, var(--color-bg) 80%, rgb(${accent})) 100%)`,
		'--brand-gradient-border': `rgb(${accent} / ${borderAlpha})`,
		'--brand-gradient-fade-out-color': `linear-gradient(to bottom, rgb(${accent} / 0) 0%, color-mix(in oklab, var(--color-bg) 86%, rgb(${accent})) 80%)`,
		// What a dialog dims the window with.
		'--modal-overlay-standard': `linear-gradient(to bottom, ${wash(accent, 50, 0.45)} 0%, ${wash(accent, 12, 0.95)} 100%)`,
		// The wash over the splash screen, which the theme also writes in purple.
		'--splash-wash': `linear-gradient(180deg, ${wash(accent, 55, 0.15)} 0%, ${wash(accent, 20, 0.3)} 97.29%)`,
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
	'--modal-overlay-standard',
	'--loading-bar-gradient',
	'--splash-wash',
] as const

const HEX = /^#([0-9a-f]{3,8})$/i
const RGB = /^rgba?\(([^)]+)\)$/i

interface ParsedColor {
	rgb: [number, number, number]
	alpha: number
}

/**
 * Reads a colour the way a theme might have written it — or been minified into.
 *
 * The four- and eight-digit hex forms matter more than they look: the release
 * build's minifier rewrites `rgba(199, 138, 255, 0.25)` as `#c78aff40`, so a
 * parser that only knew `rgb()` read every translucent variable as opaque and
 * repainted it solid. That is invisible in development, where the stylesheet is
 * not minified, and turned the selected tab, the badges and the sidebar's
 * divider into solid blocks of accent in a packaged build.
 */
function parseColor(value: string): ParsedColor | null {
	const text = value.trim()

	const hex = HEX.exec(text)
	if (hex) {
		const digits = hex[1]
		const short = digits.length <= 4
		const expanded = short
			? digits
					.split('')
					.map((digit) => digit + digit)
					.join('')
			: digits
		if (expanded.length !== 6 && expanded.length !== 8) return null

		const number = Number.parseInt(expanded.slice(0, 6), 16)
		const alpha = expanded.length === 8 ? Number.parseInt(expanded.slice(6, 8), 16) / 255 : 1
		return {
			rgb: [(number >> 16) & 255, (number >> 8) & 255, number & 255],
			alpha,
		}
	}

	const rgb = RGB.exec(text)
	if (rgb) {
		const parts = rgb[1]
			.split(/[\s,/]+/)
			.filter(Boolean)
			.map(Number)
		if (parts.length < 3 || parts.slice(0, 3).some(Number.isNaN)) return null
		return {
			rgb: [parts[0], parts[1], parts[2]],
			alpha: Number.isFinite(parts[3]) ? parts[3] : 1,
		}
	}

	return null
}

function parseHex(value: string): [number, number, number] | null {
	return parseColor(value)?.rgb ?? null
}

/** The alpha a theme gave a variable, so repainting it does not flatten it. */
function alphaOf(value: string): number {
	return parseColor(value)?.alpha ?? 1
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

/**
 * A colour picked on the wheel rather than from the list, stored in place of a
 * preset id as `#rrggbb`. It is drawn as picked on every theme: the picker
 * repaints the window as it moves, so what reads well is the user's to see.
 */
const CUSTOM_ACCENT = /^#[0-9a-f]{6}$/i

export function isCustomAccent(id: string): boolean {
	return CUSTOM_ACCENT.test(id)
}

/**
 * Presets that were dropped from the list, kept as the colour they drew on dark
 * themes so that whoever had picked one keeps it, now as a custom colour.
 */
const RETIRED_PRESETS: Record<string, string> = {
	amethyst: '#a855f7',
	glacier: '#3ad2ec',
}

function knownAccent(id: unknown): string | null {
	if (typeof id !== 'string') return null
	if (RETIRED_PRESETS[id]) return RETIRED_PRESETS[id]
	return isCustomAccent(id) || findAccentPreset(id) ? id : null
}

/** The colour an accent id is drawn in under the theme on screen. */
export function accentColorOf(id: string, active: string = theme.active): string | null {
	if (isCustomAccent(id)) return id.toLowerCase()
	const preset = findAccentPreset(id)
	return preset ? accentColorFor(preset, active) : null
}

/**
 * What was chosen last time, read before anything is drawn.
 *
 * The database is the real home of both of these; this is only so the window
 * opens in the right colour rather than repainting itself once the settings
 * arrive.
 */
const CACHE_KEY = 'noctrinth-accent'

function readCache(): { preset: string; tint: boolean } {
	try {
		const raw = localStorage.getItem(CACHE_KEY)
		const cached = raw ? (JSON.parse(raw) as { preset?: unknown; tint?: unknown }) : null
		return {
			preset: knownAccent(cached?.preset) ?? DEFAULT_ACCENT_PRESET,
			tint: cached?.tint !== false,
		}
	} catch {
		return { preset: DEFAULT_ACCENT_PRESET, tint: true }
	}
}

function writeCache(preset: string, tint: boolean): void {
	try {
		localStorage.setItem(CACHE_KEY, JSON.stringify({ preset, tint }))
	} catch (error) {
		console.warn('Failed to remember the accent for the next launch:', error)
	}
}

const cached = readCache()
const presetId = ref(cached.preset)
const tintBackground = ref(cached.tint)
const theme = useTheme()

/** The colour a preset is drawn in under the theme currently on screen. */
/**
 * The accent of a theme that is not purple, which is what "the theme's own"
 * means there: the swatch, the window icon and the shortcuts show it.
 */
const THEME_ACCENTS: Record<string, string> = { retro: '#4d9227' }

export function accentColorFor(preset: AccentPreset, active: string = theme.active): string {
	if (preset.id === DEFAULT_ACCENT_PRESET && THEME_ACCENTS[active]) return THEME_ACCENTS[active]
	return active === 'light' ? preset.light : preset.dark
}

function clear(html: HTMLElement, variables: readonly string[]): void {
	for (const variable of variables) {
		html.style.removeProperty(variable)
	}
}

const ALL_VARIABLES = [...ACCENT_VARIABLES, ...SURFACE_VARIABLES, ...GRADIENT_VARIABLES]

/**
 * What the theme on screen gives the variables this repaints, read once per
 * theme rather than on every repaint.
 *
 * Reading them means taking the overrides off and asking for the computed
 * style, which recalculates the style of the whole window there and then — and
 * again once the overrides go back on. Done for every step of a drag across the
 * colour wheel, that was most of why the wheel lagged. Not kept while the theme
 * has not reached the page yet, which is what an empty brand colour means.
 */
let themeValues: { active: string; values: Map<string, string> } | null = null

function readThemeValues(html: HTMLElement): Map<string, string> {
	if (themeValues?.active === theme.active) return themeValues.values

	// Cleared first, so what is read back is the theme's own colour and not the
	// last thing this wrote.
	clear(html, ALL_VARIABLES)
	const styles = getComputedStyle(html)
	const values = new Map(
		[...ACCENT_VARIABLES, ...SURFACE_VARIABLES, '--brand-gradient-border'].map((variable) => [
			variable,
			styles.getPropertyValue(variable),
		]),
	)
	themeValues = values.get('--color-brand')?.trim() ? { active: theme.active, values } : null
	return values
}

function apply(id: string, tint: boolean): void {
	const html = document.documentElement

	const color = id === DEFAULT_ACCENT_PRESET ? null : accentColorOf(id)
	const rgb = color ? parseHex(color) : null
	// The whole of the work for `theme`.
	if (!rgb) return clear(html, ALL_VARIABLES)

	const [red, green, blue] = rgb
	const values = readThemeValues(html)

	for (const variable of ACCENT_VARIABLES) {
		const alpha = alphaOf(values.get(variable) ?? '')
		html.style.setProperty(
			variable,
			alpha >= 1 ? `rgb(${red} ${green} ${blue})` : `rgb(${red} ${green} ${blue} / ${alpha})`,
		)
	}

	// Everything the theme already painted in the brand colour — the right-hand
	// sidebar, the promo cards, the dialog dimmer, the loading bar — follows the
	// preset whether or not the greys do. It was the brand's before; it is the
	// brand's now.
	const accent = `${red} ${green} ${blue}`
	const borderAlpha = alphaOf(values.get('--brand-gradient-border') ?? '')
	for (const [variable, value] of Object.entries(brandGradients(accent, borderAlpha))) {
		html.style.setProperty(variable, value)
	}
	html.style.setProperty('--loading-bar-gradient', loadingBarGradient(accent, theme.active))

	if (!tint) return clear(html, SURFACE_VARIABLES)

	// The surfaces keep their lightness — that is the theme's depth and its
	// contrast — and are given the preset's hue at a fraction of its strength.
	const hue = round(hueOf(rgb), 2)
	for (const variable of SURFACE_VARIABLES) {
		const surface = parseHex(values.get(variable) ?? '')
		if (!surface) continue

		const lightness = lightnessOf(surface)
		html.style.setProperty(variable, `oklch(${round(lightness, 4)} ${SURFACE_TINT} ${hue})`)
	}
}

let iconTimer: ReturnType<typeof setTimeout> | undefined
let settleTimer: ReturnType<typeof setTimeout> | undefined
let frame = 0

/**
 * Holds every transition and animation off while the accent changes, the way a
 * theme change does, with the same class.
 *
 * Elements that fade their colours would otherwise start a fade on every step
 * of a drag across the wheel, each from wherever the last one had got to, so
 * the window trailed behind the pointer in a smear of half-finished fades.
 */
function holdTransitions(): void {
	const html = document.documentElement
	html.classList.add('theme-color-transitioning')
	clearTimeout(settleTimer)
	settleTimer = setTimeout(() => html.classList.remove('theme-color-transitioning'), 200)
}

// Each theme carries its own alphas, and a preset is drawn differently on light
// than on dark, so the override is recomputed rather than kept.
watch(
	[presetId, tintBackground, () => theme.active],
	([id, tint, active], previous) => {
		cancelAnimationFrame(frame)
		if (previous?.[2] === active) {
			// A new colour, at most once a frame: the wheel can report more moves
			// than the window can draw.
			holdTransitions()
			frame = requestAnimationFrame(() => apply(presetId.value, tintBackground.value))
		} else {
			// The first paint and a theme change are drawn at once, so the window
			// never shows a frame in the theme's own colour.
			apply(id, tint)
		}

		// Remembering it, and the window's own icon, which is the last purple thing
		// left when the app is not purple any more, wait until the colour settles:
		// the icon is redrawn by the backend, along with every shortcut's.
		clearTimeout(iconTimer)
		iconTimer = setTimeout(() => {
			writeCache(id, tint)
			const color = accentColorOf(id)
			if (!color) return
			void import('@/helpers/noctrinth-window-icon').then(({ paintWindowIcon }) =>
				paintWindowIcon(
					color,
					// The shortcuts go back to the executable's icon only when that
					// icon is the right colour, which it is not for a green theme.
					id === DEFAULT_ACCENT_PRESET && !THEME_ACCENTS[theme.active],
				),
			)
		}, 400)
	},
	{ immediate: true },
)

export function useAccentPreset() {
	return {
		id: presetId,
		tintBackground,
		preset: computed(() => findAccentPreset(presetId.value) ?? ACCENT_PRESETS[0]),
	}
}

/** Sets the accent without going through the settings tab — used at startup. */
export function setAccentPreset(id: string | null | undefined, tint = true): void {
	presetId.value = knownAccent(id) ?? DEFAULT_ACCENT_PRESET
	tintBackground.value = tint
}
