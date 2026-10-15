import { get as getSettings, set as setSettings } from '@/helpers/settings.ts'

/**
 * Which cube the splash shows. The choice is kept in the app's settings, and
 * mirrored to local storage at startup, because the splash is drawn before the
 * settings are read. The mirror alone was not enough: the dev server and the
 * installed app are different origins, each with its own local storage.
 *
 * - `theme`: the cube matches the theme, dark on dark and light on light, and
 *   once in a while the other way round, as a small surprise.
 * - `inverted`: always the other way round: black on white, white on black.
 * - `random`: either, every launch.
 */
export const SPLASH_CUBE_CHOICES = ['theme', 'inverted', 'random'] as const
export type SplashCubeChoice = (typeof SPLASH_CUBE_CHOICES)[number]

const STORAGE_KEY = 'noctrinth-splash-cube'

/** How often the default choice shows the inverted cube anyway. */
const SURPRISE_CHANCE = 1 / 25

export function readSplashCubeChoice(): SplashCubeChoice {
	try {
		const stored = localStorage.getItem(STORAGE_KEY)
		return (SPLASH_CUBE_CHOICES as readonly string[]).includes(stored ?? '')
			? (stored as SplashCubeChoice)
			: 'theme'
	} catch {
		return 'theme'
	}
}

/** Mirrors the saved choice for the next launch's splash. */
export function cacheSplashCubeChoice(choice: string): void {
	try {
		localStorage.setItem(STORAGE_KEY, choice)
	} catch (error) {
		console.warn('Failed to remember the splash cube:', error)
	}
}

export async function saveSplashCubeChoice(choice: SplashCubeChoice): Promise<void> {
	cacheSplashCubeChoice(choice)
	await setSettings({ ...(await getSettings()), splash_cube: choice })
}

/** Decided once per launch. */
export function rollSplashCubeInverted(choice = readSplashCubeChoice()): boolean {
	if (choice === 'inverted') return true
	if (choice === 'random') return Math.random() < 0.5
	return Math.random() < SURPRISE_CHANCE
}
