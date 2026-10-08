/**
 * Which cube the splash shows. The splash is drawn before the settings are
 * read, so the choice lives in local storage, like the accent's mirror does.
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

export function writeSplashCubeChoice(choice: SplashCubeChoice): void {
	try {
		localStorage.setItem(STORAGE_KEY, choice)
	} catch (error) {
		console.warn('Failed to remember the splash cube:', error)
	}
}

/** Decided once per launch. */
export function rollSplashCubeInverted(choice = readSplashCubeChoice()): boolean {
	if (choice === 'inverted') return true
	if (choice === 'random') return Math.random() < 0.5
	return Math.random() < SURPRISE_CHANCE
}
