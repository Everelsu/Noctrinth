/**
 * Catalogue of Minecraft `options.txt` settings the launcher can apply to every
 * instance, modelled after the game's own options screens.
 *
 * Version handling has two layers, and they solve different problems:
 *
 *  1. `onlyIfPresent` (the default). Minecraft writes out every option it knows
 *     about, so a key missing from an instance's `options.txt` means that
 *     version has no such option. Those entries overwrite but never create,
 *     which adapts to any version without a table to maintain.
 *
 *  2. `minVersion` / `maxVersion`. Only needed where a key survived across
 *     versions but its *value format* changed — presence alone can't catch
 *     that. `ao` is the motivating case: 0/1/2 before 1.13, a boolean after.
 *
 * Values are stored exactly as the game writes them; the encode/decode pair on
 * each option converts to something sane to put in front of a person (FOV in
 * degrees rather than -1..1, volumes in percent rather than 0..1).
 */

export type GameOptionGroup = 'video' | 'sound' | 'controls'

interface GameOptionBase {
	/** The `options.txt` key. */
	key: string
	label: string
	description?: string
	group: GameOptionGroup
	/** Overwrite only when the instance already has the key. Defaults to true. */
	onlyIfPresent?: boolean
	/** Inclusive bounds, for options whose value format changed. */
	minVersion?: string
	maxVersion?: string
}

export type GameOption = GameOptionBase &
	(
		| {
				control: 'toggle'
				default: boolean
				encode: (value: boolean) => string
				decode: (raw: string) => boolean
		  }
		| {
				control: 'slider'
				default: number
				min: number
				max: number
				step: number
				unit?: string
				encode: (value: number) => string
				decode: (raw: string) => number
		  }
		| {
				control: 'select'
				default: string
				choices: { value: string; label: string }[]
				encode: (value: string) => string
				decode: (raw: string) => string
		  }
	)

const bool = {
	encode: (value: boolean) => (value ? 'true' : 'false'),
	decode: (raw: string) => raw === 'true',
}

/** Rounds to at most `places` decimals without trailing zeroes, the way the game writes floats. */
function float(value: number, places = 4): string {
	return String(Number(value.toFixed(places)))
}

/** Percent in the UI, 0.0–1.0 in the file. */
const percent = {
	encode: (value: number) => float(value / 100),
	decode: (raw: string) => Math.round(Number(raw) * 100),
}

// FOV is stored as an offset from the default 70°, scaled so -1 is 30° and 1 is 110°.
const FOV_BASE = 70
const FOV_RANGE = 40

export const GAME_OPTIONS: GameOption[] = [
	// ── Video ──────────────────────────────────────────────────────────────
	{
		key: 'fov',
		label: 'Field of view',
		group: 'video',
		control: 'slider',
		default: 70,
		min: 30,
		max: 110,
		step: 1,
		unit: '°',
		encode: (value) => float((value - FOV_BASE) / FOV_RANGE),
		decode: (raw) => Math.round(Number(raw) * FOV_RANGE + FOV_BASE),
	},
	{
		key: 'renderDistance',
		label: 'Render distance',
		group: 'video',
		control: 'slider',
		default: 12,
		min: 2,
		max: 32,
		step: 1,
		unit: ' chunks',
		encode: (value) => String(Math.round(value)),
		decode: (raw) => Number(raw),
	},
	{
		key: 'simulationDistance',
		label: 'Simulation distance',
		group: 'video',
		control: 'slider',
		default: 12,
		min: 5,
		max: 32,
		step: 1,
		unit: ' chunks',
		encode: (value) => String(Math.round(value)),
		decode: (raw) => Number(raw),
	},
	{
		key: 'gamma',
		label: 'Brightness',
		group: 'video',
		control: 'slider',
		default: 50,
		min: 0,
		max: 100,
		step: 1,
		unit: '%',
		...percent,
	},
	{
		key: 'guiScale',
		label: 'GUI scale',
		group: 'video',
		control: 'select',
		default: '0',
		choices: [
			{ value: '0', label: 'Auto' },
			{ value: '1', label: 'Small' },
			{ value: '2', label: 'Normal' },
			{ value: '3', label: 'Large' },
			{ value: '4', label: 'Very large' },
		],
		encode: (value) => value,
		decode: (raw) => raw,
	},
	{
		key: 'maxFps',
		label: 'Max framerate',
		group: 'video',
		control: 'slider',
		default: 120,
		min: 10,
		max: 260,
		step: 10,
		unit: ' fps',
		description: '260 means unlimited, matching the in-game slider.',
		encode: (value) => String(Math.round(value)),
		decode: (raw) => Number(raw),
	},
	{
		key: 'enableVsync',
		label: 'VSync',
		group: 'video',
		control: 'toggle',
		default: true,
		...bool,
	},
	{
		key: 'particles',
		label: 'Particles',
		group: 'video',
		control: 'select',
		default: '0',
		choices: [
			{ value: '0', label: 'All' },
			{ value: '1', label: 'Decreased' },
			{ value: '2', label: 'Minimal' },
		],
		encode: (value) => value,
		decode: (raw) => raw,
	},
	{
		key: 'graphicsMode',
		label: 'Graphics',
		group: 'video',
		control: 'select',
		default: '1',
		choices: [
			{ value: '0', label: 'Fast' },
			{ value: '1', label: 'Fancy' },
			{ value: '2', label: 'Fabulous' },
		],
		// Before 1.16 this was the boolean `fancyGraphics`, a different key.
		minVersion: '1.16',
		encode: (value) => value,
		decode: (raw) => raw,
	},
	{
		key: 'ao',
		label: 'Smooth lighting',
		group: 'video',
		control: 'toggle',
		default: true,
		// The key predates 1.13 but held 0/1/2 back then, so writing a boolean
		// into an older instance would corrupt it. Presence can't catch this.
		minVersion: '1.13',
		...bool,
	},
	{
		key: 'entityShadows',
		label: 'Entity shadows',
		group: 'video',
		control: 'toggle',
		default: true,
		...bool,
	},

	// ── Sound ──────────────────────────────────────────────────────────────
	{
		key: 'soundCategory_master',
		label: 'Master volume',
		group: 'sound',
		control: 'slider',
		default: 100,
		min: 0,
		max: 100,
		step: 1,
		unit: '%',
		...percent,
	},
	{
		key: 'soundCategory_music',
		label: 'Music',
		group: 'sound',
		control: 'slider',
		default: 100,
		min: 0,
		max: 100,
		step: 1,
		unit: '%',
		...percent,
	},
	{
		key: 'soundCategory_record',
		label: 'Jukebox / note blocks',
		group: 'sound',
		control: 'slider',
		default: 100,
		min: 0,
		max: 100,
		step: 1,
		unit: '%',
		...percent,
	},
	{
		key: 'soundCategory_weather',
		label: 'Weather',
		group: 'sound',
		control: 'slider',
		default: 100,
		min: 0,
		max: 100,
		step: 1,
		unit: '%',
		...percent,
	},
	{
		key: 'soundCategory_block',
		label: 'Blocks',
		group: 'sound',
		control: 'slider',
		default: 100,
		min: 0,
		max: 100,
		step: 1,
		unit: '%',
		...percent,
	},
	{
		key: 'soundCategory_hostile',
		label: 'Hostile creatures',
		group: 'sound',
		control: 'slider',
		default: 100,
		min: 0,
		max: 100,
		step: 1,
		unit: '%',
		...percent,
	},
	{
		key: 'soundCategory_neutral',
		label: 'Friendly creatures',
		group: 'sound',
		control: 'slider',
		default: 100,
		min: 0,
		max: 100,
		step: 1,
		unit: '%',
		...percent,
	},
	{
		key: 'soundCategory_player',
		label: 'Players',
		group: 'sound',
		control: 'slider',
		default: 100,
		min: 0,
		max: 100,
		step: 1,
		unit: '%',
		...percent,
	},
	{
		key: 'soundCategory_ambient',
		label: 'Ambient / environment',
		group: 'sound',
		control: 'slider',
		default: 100,
		min: 0,
		max: 100,
		step: 1,
		unit: '%',
		...percent,
	},
	{
		key: 'soundCategory_voice',
		label: 'Voice / speech',
		group: 'sound',
		control: 'slider',
		default: 100,
		min: 0,
		max: 100,
		step: 1,
		unit: '%',
		...percent,
	},

	// ── Controls ───────────────────────────────────────────────────────────
	{
		key: 'mouseSensitivity',
		label: 'Sensitivity',
		group: 'controls',
		control: 'slider',
		default: 50,
		min: 0,
		max: 100,
		step: 1,
		unit: '%',
		description: '50% is the game’s "Normal".',
		...percent,
	},
	{
		key: 'invertYMouse',
		label: 'Invert mouse',
		group: 'controls',
		control: 'toggle',
		default: false,
		...bool,
	},
	{
		key: 'autoJump',
		label: 'Auto-jump',
		group: 'controls',
		control: 'toggle',
		default: false,
		...bool,
	},
	{
		key: 'toggleCrouch',
		label: 'Sneak: toggle',
		group: 'controls',
		control: 'toggle',
		default: false,
		...bool,
	},
	{
		key: 'toggleSprint',
		label: 'Sprint: toggle',
		group: 'controls',
		control: 'toggle',
		default: false,
		...bool,
	},
]

export const GAME_OPTION_GROUPS: { id: GameOptionGroup; label: string }[] = [
	{ id: 'video', label: 'Video' },
	{ id: 'sound', label: 'Sound' },
	{ id: 'controls', label: 'Controls' },
]

export function optionsForGroup(group: GameOptionGroup): GameOption[] {
	return GAME_OPTIONS.filter((option) => option.group === group)
}

/** Matches the Rust `SharedGameOption`. */
export interface SharedGameOptionEntry {
	key: string
	value: string
	only_if_present: boolean
	min_version: string | null
	max_version: string | null
}

export interface SharedGameOptionsProfile {
	enabled: boolean
	entries: SharedGameOptionEntry[]
}

export function emptyProfile(): SharedGameOptionsProfile {
	return { enabled: false, entries: [] }
}

/** Builds the stored entry for one catalogue option at the given UI value. */
export function toEntry(option: GameOption, value: unknown): SharedGameOptionEntry {
	return {
		key: option.key,
		// The encode signatures are per-variant; the caller always hands back the
		// value this option's own control produced.
		value: (option.encode as (input: never) => string)(value as never),
		only_if_present: option.onlyIfPresent ?? true,
		min_version: option.minVersion ?? null,
		max_version: option.maxVersion ?? null,
	}
}

/** Reads a stored entry back into the value its control expects. */
export function fromEntry(option: GameOption, entry: SharedGameOptionEntry): unknown {
	try {
		const decoded = (option.decode as (raw: string) => unknown)(entry.value)
		if (typeof decoded === 'number' && Number.isNaN(decoded)) return option.default
		return decoded
	} catch {
		return option.default
	}
}
