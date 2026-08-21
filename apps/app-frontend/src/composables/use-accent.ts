/**
 * How vivid Noctrinth's accent is drawn.
 *
 * Themes define their accent as a fixed colour, so the only way to make it
 * quieter or louder is to recolour it. This takes the theme's own purple, reads
 * its chroma — how far it sits from grey in OKLCh, which is the axis the eye
 * reads as "how strong is this colour" — and scales that, leaving lightness and
 * hue exactly where the theme put them. At 100 nothing is overridden at all, so
 * the default is the theme untouched rather than a re-derivation of it.
 *
 * The overrides are written onto the document element as inline custom
 * properties, which beat every theme stylesheet without editing one, and are
 * recomputed when the theme changes because each theme carries its own purple.
 */
import { ref, watch } from 'vue'

import { useTheme } from '@/composables/use-theme.ts'

export const ACCENT_INTENSITY_DEFAULT = 100
export const ACCENT_INTENSITY_MIN = 40
export const ACCENT_INTENSITY_MAX = 160
export const ACCENT_INTENSITY_STEP = 5

/**
 * Every variable that carries the accent, recoloured together.
 *
 * `--color-brand` is the accent; the purple variables are the same colour under
 * the name components reach for when they mean "purple" rather than "brand",
 * which in this fork is the same thing. Each keeps whatever alpha its theme
 * gave it, so a highlight stays a highlight.
 */
const ACCENT_VARIABLES = [
	'--color-brand',
	'--color-brand-highlight',
	'--color-brand-shadow',
	'--color-purple',
	'--color-purple-highlight',
	'--color-purple-bg',
] as const

interface Oklch {
	l: number
	c: number
	h: number
	alpha: number
}

const HEX = /^#([0-9a-f]{3}|[0-9a-f]{6})$/i
const RGB = /^rgba?\(([^)]+)\)$/i

/** Parses the colour formats the theme files actually use: hex and rgb/rgba. */
function parseColor(value: string): [number, number, number, number] | null {
	const text = value.trim()

	const hex = HEX.exec(text)
	if (hex) {
		const digits =
			hex[1].length === 3
				? hex[1]
						.split('')
						.map((digit) => digit + digit)
						.join('')
				: hex[1]
		const number = Number.parseInt(digits, 16)
		return [((number >> 16) & 255) / 255, ((number >> 8) & 255) / 255, (number & 255) / 255, 1]
	}

	const rgb = RGB.exec(text)
	if (rgb) {
		const parts = rgb[1]
			.split(/[\s,/]+/)
			.filter(Boolean)
			.map(Number)
		if (parts.length < 3 || parts.some(Number.isNaN)) return null
		return [parts[0] / 255, parts[1] / 255, parts[2] / 255, parts[3] ?? 1]
	}

	return null
}

function toLinear(channel: number): number {
	return channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4
}

/** sRGB → OKLCh, following Björn Ottosson's published matrices. */
function toOklch(value: string): Oklch | null {
	const parsed = parseColor(value)
	if (!parsed) return null

	const [red, green, blue, alpha] = parsed
	const r = toLinear(red)
	const g = toLinear(green)
	const b = toLinear(blue)

	const l = Math.cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b)
	const m = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b)
	const s = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b)

	const lightness = 0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s
	const a = 1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s
	const bAxis = 0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s

	return {
		l: lightness,
		c: Math.hypot(a, bAxis),
		h: (Math.atan2(bAxis, a) * 180) / Math.PI,
		alpha,
	}
}

function round(value: number, places: number): number {
	const factor = 10 ** places
	return Math.round(value * factor) / factor
}

function format({ l, c, h, alpha }: Oklch, scale: number): string {
	// Capped short of the point where scaling stops adding colour and only
	// pushes the value outside what the display can show.
	const chroma = round(Math.min(c * scale, 0.37), 4)
	const base = `oklch(${round(l, 4)} ${chroma} ${round(h, 2)}`
	return alpha >= 1 ? `${base})` : `${base} / ${round(alpha, 3)})`
}

export function clampAccentIntensity(value: number): number {
	if (!Number.isFinite(value)) return ACCENT_INTENSITY_DEFAULT
	return Math.min(ACCENT_INTENSITY_MAX, Math.max(ACCENT_INTENSITY_MIN, Math.round(value)))
}

const intensity = ref(ACCENT_INTENSITY_DEFAULT)

function apply(value: number): void {
	const html = document.documentElement

	// Cleared first, so what is read back is the theme's own colour and not the
	// last thing this wrote. Also the whole of the work at 100.
	for (const variable of ACCENT_VARIABLES) {
		html.style.removeProperty(variable)
	}

	if (value === ACCENT_INTENSITY_DEFAULT) return

	const scale = value / 100
	const styles = getComputedStyle(html)

	for (const variable of ACCENT_VARIABLES) {
		const color = toOklch(styles.getPropertyValue(variable))
		if (color) {
			html.style.setProperty(variable, format(color, scale))
		}
	}
}

const theme = useTheme()

// Each theme carries its own purple, so the override is recomputed from the
// new one rather than kept from the old.
watch([intensity, () => theme.active], ([value]) => apply(clampAccentIntensity(value)), {
	immediate: true,
})

export function useAccentIntensity() {
	return intensity
}

/** Sets the accent without going through the settings tab — used at startup. */
export function setAccentIntensity(value: number): void {
	intensity.value = clampAccentIntensity(value)
}
