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

import { defineMessages } from '@modrinth/ui'

/**
 * The shape of a translated string in this catalogue.
 *
 * Declared here rather than imported. `@modrinth/ui` reaches this project
 * without type declarations, so its own `OptionMessage` arrives as `any`,
 * and a plain string in a `label` or a `unit` type-checks perfectly happily —
 * right up until the screen renders and asks it for an `id`. A local type is
 * the difference between catching that at build time and finding out from a
 * broken settings tab.
 */
interface OptionMessage {
	id: string
	defaultMessage: string
}

const messages = defineMessages({
	fovLabel: {
		id: 'app.settings.game-options.fov.label',
		defaultMessage: 'Field of view',
	},
	renderDistanceLabel: {
		id: 'app.settings.game-options.renderDistance.label',
		defaultMessage: 'Render distance',
	},
	unitChunks: {
		id: 'app.settings.game-options.unit.chunks',
		defaultMessage: ' chunks',
	},
	simulationDistanceLabel: {
		id: 'app.settings.game-options.simulationDistance.label',
		defaultMessage: 'Simulation distance',
	},
	gammaLabel: {
		id: 'app.settings.game-options.gamma.label',
		defaultMessage: 'Brightness',
	},
	guiScaleLabel: {
		id: 'app.settings.game-options.guiScale.label',
		defaultMessage: 'GUI scale',
	},
	guiScaleChoice0: {
		id: 'app.settings.game-options.guiScale.choice.0',
		defaultMessage: 'Auto',
	},
	guiScaleChoice1: {
		id: 'app.settings.game-options.guiScale.choice.1',
		defaultMessage: 'Small',
	},
	guiScaleChoice2: {
		id: 'app.settings.game-options.guiScale.choice.2',
		defaultMessage: 'Normal',
	},
	guiScaleChoice3: {
		id: 'app.settings.game-options.guiScale.choice.3',
		defaultMessage: 'Large',
	},
	guiScaleChoice4: {
		id: 'app.settings.game-options.guiScale.choice.4',
		defaultMessage: 'Very large',
	},
	maxFpsLabel: {
		id: 'app.settings.game-options.maxFps.label',
		defaultMessage: 'Max framerate',
	},
	unitFps: {
		id: 'app.settings.game-options.unit.fps',
		defaultMessage: ' fps',
	},
	unitPercent: {
		id: 'app.settings.game-options.unit.percent',
		defaultMessage: '%',
	},
	unitDegrees: {
		id: 'app.settings.game-options.unit.degrees',
		defaultMessage: '°',
	},
	maxFpsDescription: {
		id: 'app.settings.game-options.maxFps.description',
		defaultMessage: '260 means unlimited, matching the in-game slider.',
	},
	enableVsyncLabel: {
		id: 'app.settings.game-options.enableVsync.label',
		defaultMessage: 'VSync',
	},
	particlesLabel: {
		id: 'app.settings.game-options.particles.label',
		defaultMessage: 'Particles',
	},
	particlesChoice0: {
		id: 'app.settings.game-options.particles.choice.0',
		defaultMessage: 'All',
	},
	particlesChoice1: {
		id: 'app.settings.game-options.particles.choice.1',
		defaultMessage: 'Decreased',
	},
	particlesChoice2: {
		id: 'app.settings.game-options.particles.choice.2',
		defaultMessage: 'Minimal',
	},
	graphicsModeLabel: {
		id: 'app.settings.game-options.graphicsMode.label',
		defaultMessage: 'Graphics',
	},
	graphicsModeChoice0: {
		id: 'app.settings.game-options.graphicsMode.choice.0',
		defaultMessage: 'Fast',
	},
	graphicsModeChoice1: {
		id: 'app.settings.game-options.graphicsMode.choice.1',
		defaultMessage: 'Fancy',
	},
	graphicsModeChoice2: {
		id: 'app.settings.game-options.graphicsMode.choice.2',
		defaultMessage: 'Fabulous',
	},
	aoLabel: {
		id: 'app.settings.game-options.ao.label',
		defaultMessage: 'Smooth lighting',
	},
	entityShadowsLabel: {
		id: 'app.settings.game-options.entityShadows.label',
		defaultMessage: 'Entity shadows',
	},
	soundCategoryMasterLabel: {
		id: 'app.settings.game-options.soundCategory_master.label',
		defaultMessage: 'Master volume',
	},
	soundCategoryMusicLabel: {
		id: 'app.settings.game-options.soundCategory_music.label',
		defaultMessage: 'Music',
	},
	soundCategoryRecordLabel: {
		id: 'app.settings.game-options.soundCategory_record.label',
		defaultMessage: 'Jukebox / note blocks',
	},
	soundCategoryWeatherLabel: {
		id: 'app.settings.game-options.soundCategory_weather.label',
		defaultMessage: 'Weather',
	},
	soundCategoryBlockLabel: {
		id: 'app.settings.game-options.soundCategory_block.label',
		defaultMessage: 'Blocks',
	},
	soundCategoryHostileLabel: {
		id: 'app.settings.game-options.soundCategory_hostile.label',
		defaultMessage: 'Hostile creatures',
	},
	soundCategoryNeutralLabel: {
		id: 'app.settings.game-options.soundCategory_neutral.label',
		defaultMessage: 'Friendly creatures',
	},
	soundCategoryPlayerLabel: {
		id: 'app.settings.game-options.soundCategory_player.label',
		defaultMessage: 'Players',
	},
	soundCategoryAmbientLabel: {
		id: 'app.settings.game-options.soundCategory_ambient.label',
		defaultMessage: 'Ambient / environment',
	},
	soundCategoryVoiceLabel: {
		id: 'app.settings.game-options.soundCategory_voice.label',
		defaultMessage: 'Voice / speech',
	},
	mouseSensitivityLabel: {
		id: 'app.settings.game-options.mouseSensitivity.label',
		defaultMessage: 'Sensitivity',
	},
	mouseSensitivityDescription: {
		id: 'app.settings.game-options.mouseSensitivity.description',
		defaultMessage: '50% is the game’s "Normal".',
	},
	invertYMouseLabel: {
		id: 'app.settings.game-options.invertYMouse.label',
		defaultMessage: 'Invert mouse',
	},
	autoJumpLabel: {
		id: 'app.settings.game-options.autoJump.label',
		defaultMessage: 'Auto-jump',
	},
	toggleCrouchLabel: {
		id: 'app.settings.game-options.toggleCrouch.label',
		defaultMessage: 'Sneak: toggle',
	},
	toggleSprintLabel: {
		id: 'app.settings.game-options.toggleSprint.label',
		defaultMessage: 'Sprint: toggle',
	},
	videoGroup: {
		id: 'app.settings.game-options.group.video',
		defaultMessage: 'Video',
	},
	soundGroup: {
		id: 'app.settings.game-options.group.sound',
		defaultMessage: 'Sound',
	},
	controlsGroup: {
		id: 'app.settings.game-options.group.controls',
		defaultMessage: 'Controls',
	},
})

export type GameOptionGroup = 'video' | 'sound' | 'controls'

interface GameOptionBase {
	/** The `options.txt` key. */
	key: string
	label: OptionMessage
	description?: OptionMessage
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
				unit?: OptionMessage
				encode: (value: number) => string
				decode: (raw: string) => number
		  }
		| {
				control: 'select'
				default: string
				choices: { value: string; label: OptionMessage }[]
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
		label: messages.fovLabel,
		group: 'video',
		control: 'slider',
		default: 70,
		min: 30,
		max: 110,
		step: 1,
		unit: messages.unitDegrees,
		encode: (value) => float((value - FOV_BASE) / FOV_RANGE),
		decode: (raw) => Math.round(Number(raw) * FOV_RANGE + FOV_BASE),
	},
	{
		key: 'renderDistance',
		label: messages.renderDistanceLabel,
		group: 'video',
		control: 'slider',
		default: 12,
		min: 2,
		max: 32,
		step: 1,
		unit: messages.unitChunks,
		encode: (value) => String(Math.round(value)),
		decode: (raw) => Number(raw),
	},
	{
		key: 'simulationDistance',
		label: messages.simulationDistanceLabel,
		group: 'video',
		control: 'slider',
		default: 12,
		min: 5,
		max: 32,
		step: 1,
		unit: messages.unitChunks,
		encode: (value) => String(Math.round(value)),
		decode: (raw) => Number(raw),
	},
	{
		key: 'gamma',
		label: messages.gammaLabel,
		group: 'video',
		control: 'slider',
		default: 50,
		min: 0,
		max: 100,
		step: 1,
		unit: messages.unitPercent,
		...percent,
	},
	{
		key: 'guiScale',
		label: messages.guiScaleLabel,
		group: 'video',
		control: 'select',
		default: '0',
		choices: [
			{ value: '0', label: messages.guiScaleChoice0 },
			{ value: '1', label: messages.guiScaleChoice1 },
			{ value: '2', label: messages.guiScaleChoice2 },
			{ value: '3', label: messages.guiScaleChoice3 },
			{ value: '4', label: messages.guiScaleChoice4 },
		],
		encode: (value) => value,
		decode: (raw) => raw,
	},
	{
		key: 'maxFps',
		label: messages.maxFpsLabel,
		group: 'video',
		control: 'slider',
		default: 120,
		min: 10,
		max: 260,
		step: 10,
		unit: messages.unitFps,
		description: messages.maxFpsDescription,
		encode: (value) => String(Math.round(value)),
		decode: (raw) => Number(raw),
	},
	{
		key: 'enableVsync',
		label: messages.enableVsyncLabel,
		group: 'video',
		control: 'toggle',
		default: true,
		...bool,
	},
	{
		key: 'particles',
		label: messages.particlesLabel,
		group: 'video',
		control: 'select',
		default: '0',
		choices: [
			{ value: '0', label: messages.particlesChoice0 },
			{ value: '1', label: messages.particlesChoice1 },
			{ value: '2', label: messages.particlesChoice2 },
		],
		encode: (value) => value,
		decode: (raw) => raw,
	},
	{
		key: 'graphicsMode',
		label: messages.graphicsModeLabel,
		group: 'video',
		control: 'select',
		default: '1',
		choices: [
			{ value: '0', label: messages.graphicsModeChoice0 },
			{ value: '1', label: messages.graphicsModeChoice1 },
			{ value: '2', label: messages.graphicsModeChoice2 },
		],
		// Before 1.16 this was the boolean `fancyGraphics`, a different key.
		minVersion: '1.16',
		encode: (value) => value,
		decode: (raw) => raw,
	},
	{
		key: 'ao',
		label: messages.aoLabel,
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
		label: messages.entityShadowsLabel,
		group: 'video',
		control: 'toggle',
		default: true,
		...bool,
	},

	// ── Sound ──────────────────────────────────────────────────────────────
	{
		key: 'soundCategory_master',
		label: messages.soundCategoryMasterLabel,
		group: 'sound',
		control: 'slider',
		default: 100,
		min: 0,
		max: 100,
		step: 1,
		unit: messages.unitPercent,
		...percent,
	},
	{
		key: 'soundCategory_music',
		label: messages.soundCategoryMusicLabel,
		group: 'sound',
		control: 'slider',
		default: 100,
		min: 0,
		max: 100,
		step: 1,
		unit: messages.unitPercent,
		...percent,
	},
	{
		key: 'soundCategory_record',
		label: messages.soundCategoryRecordLabel,
		group: 'sound',
		control: 'slider',
		default: 100,
		min: 0,
		max: 100,
		step: 1,
		unit: messages.unitPercent,
		...percent,
	},
	{
		key: 'soundCategory_weather',
		label: messages.soundCategoryWeatherLabel,
		group: 'sound',
		control: 'slider',
		default: 100,
		min: 0,
		max: 100,
		step: 1,
		unit: messages.unitPercent,
		...percent,
	},
	{
		key: 'soundCategory_block',
		label: messages.soundCategoryBlockLabel,
		group: 'sound',
		control: 'slider',
		default: 100,
		min: 0,
		max: 100,
		step: 1,
		unit: messages.unitPercent,
		...percent,
	},
	{
		key: 'soundCategory_hostile',
		label: messages.soundCategoryHostileLabel,
		group: 'sound',
		control: 'slider',
		default: 100,
		min: 0,
		max: 100,
		step: 1,
		unit: messages.unitPercent,
		...percent,
	},
	{
		key: 'soundCategory_neutral',
		label: messages.soundCategoryNeutralLabel,
		group: 'sound',
		control: 'slider',
		default: 100,
		min: 0,
		max: 100,
		step: 1,
		unit: messages.unitPercent,
		...percent,
	},
	{
		key: 'soundCategory_player',
		label: messages.soundCategoryPlayerLabel,
		group: 'sound',
		control: 'slider',
		default: 100,
		min: 0,
		max: 100,
		step: 1,
		unit: messages.unitPercent,
		...percent,
	},
	{
		key: 'soundCategory_ambient',
		label: messages.soundCategoryAmbientLabel,
		group: 'sound',
		control: 'slider',
		default: 100,
		min: 0,
		max: 100,
		step: 1,
		unit: messages.unitPercent,
		...percent,
	},
	{
		key: 'soundCategory_voice',
		label: messages.soundCategoryVoiceLabel,
		group: 'sound',
		control: 'slider',
		default: 100,
		min: 0,
		max: 100,
		step: 1,
		unit: messages.unitPercent,
		...percent,
	},

	// ── Controls ───────────────────────────────────────────────────────────
	{
		key: 'mouseSensitivity',
		label: messages.mouseSensitivityLabel,
		group: 'controls',
		control: 'slider',
		default: 50,
		min: 0,
		max: 100,
		step: 1,
		unit: messages.unitPercent,
		description: messages.mouseSensitivityDescription,
		...percent,
	},
	{
		key: 'invertYMouse',
		label: messages.invertYMouseLabel,
		group: 'controls',
		control: 'toggle',
		default: false,
		...bool,
	},
	{
		key: 'autoJump',
		label: messages.autoJumpLabel,
		group: 'controls',
		control: 'toggle',
		default: false,
		...bool,
	},
	{
		key: 'toggleCrouch',
		label: messages.toggleCrouchLabel,
		group: 'controls',
		control: 'toggle',
		default: false,
		...bool,
	},
	{
		key: 'toggleSprint',
		label: messages.toggleSprintLabel,
		group: 'controls',
		control: 'toggle',
		default: false,
		...bool,
	},
]

export const GAME_OPTION_GROUPS: { id: GameOptionGroup; label: OptionMessage }[] = [
	{ id: 'video', label: messages.videoGroup },
	{ id: 'sound', label: messages.soundGroup },
	{ id: 'controls', label: messages.controlsGroup },
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
