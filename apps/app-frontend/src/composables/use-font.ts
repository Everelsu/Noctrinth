/**
 * The typeface the interface is set in.
 *
 * Every component reaches for `--font-standard`, which the shared stylesheet
 * declares on `body`, so the choice is written there as an inline property and
 * beats it without editing a stylesheet. Inter, the theme's own, always follows
 * the chosen font, so a font missing a glyph — Cyrillic, say, in the Minecraft
 * one — falls back to Inter rather than to a default serif.
 *
 * Like the accent, the choice is mirrored into local storage so the window
 * opens in it, rather than changing font once settings arrive from the database.
 */
import { ref, watch } from 'vue'

/** Offered by name; anything else is the family name of an installed font. */
export const FONT_PRESETS = ['', 'system', 'minecraft'] as const

const PRESET_STACKS: Record<string, string> = {
	system: 'system-ui',
	minecraft: "'bundled-minecraft-font-mrapp'",
}

const CACHE_KEY = 'noctrinth-font'

function readCache(): string {
	try {
		return localStorage.getItem(CACHE_KEY) ?? ''
	} catch {
		return ''
	}
}

const font = ref(readCache())

/** A family name as CSS, quoted, so a name with spaces or digits stays one name. */
function familyOf(name: string): string {
	return PRESET_STACKS[name] ?? `"${name.replace(/["\\]/g, '')}"`
}

watch(
	font,
	(name) => {
		const body = document.body
		const trimmed = name.trim()
		if (trimmed) {
			body.style.setProperty('--font-standard', `${familyOf(trimmed)}, Inter, sans-serif`)
		} else {
			body.style.removeProperty('--font-standard')
		}

		try {
			localStorage.setItem(CACHE_KEY, trimmed)
		} catch (error) {
			console.warn('Failed to remember the font for the next launch:', error)
		}
	},
	{ immediate: true },
)

export function useInterfaceFont() {
	return font
}

/** Sets the font without going through the settings tab — used at startup. */
export function setInterfaceFont(name: string | null | undefined): void {
	font.value = name ?? ''
}

/**
 * The families installed on this computer, for suggestions; empty where the
 * webview does not let a page list them, and the name can still be typed.
 */
export async function installedFontFamilies(): Promise<string[]> {
	const query = (window as { queryLocalFonts?: () => Promise<{ family: string }[]> })
		.queryLocalFonts
	if (!query) return []
	try {
		const fonts = await query()
		return [...new Set(fonts.map((entry) => entry.family))].sort((a, b) => a.localeCompare(b))
	} catch {
		return []
	}
}
