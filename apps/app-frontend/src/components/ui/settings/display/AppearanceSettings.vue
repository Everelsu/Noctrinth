<script setup lang="ts">
import { CheckIcon } from '@modrinth/assets'
import {
	AppearanceSettingsLayout,
	defineMessages,
	injectAuth,
	injectUserPreferences,
	provideAppearanceSettings,
	useSavable,
	useVIntl,
} from '@modrinth/ui'
import { computed, inject, onBeforeUnmount, onMounted, ref, watch } from 'vue'

import {
	ACCENT_PRESETS,
	accentColorFor,
	type AccentPreset,
	DEFAULT_ACCENT_PRESET,
	findAccentPreset,
	setAccentPreset,
} from '@/composables/use-accent.ts'
import { type ColorTheme, useTheme } from '@/composables/use-theme.ts'
import { type AppSettings, get, set } from '@/helpers/settings.ts'
import { getOS } from '@/helpers/utils'
import { appSettingsModalContextKey } from '@/providers/app-settings-modal'

const theme = useTheme()
const auth = injectAuth()
const { updatePreferences } = injectUserPreferences()
const settingsModal = inject(appSettingsModalContextKey, null)
const os = await getOS()
const settings = ref(await get())
const { formatMessage } = useVIntl()

const messages = defineMessages({
	accentTitle: {
		id: 'app.appearance-settings.accent.title',
		defaultMessage: 'Accent colour',
	},
	accentDescription: {
		id: 'app.appearance-settings.accent.description',
		defaultMessage:
			'The colour the interface is picked out in. Each one is drawn deeper on light themes and brighter on dark ones.',
	},
})

// Names live in their own map so each preset carries a message id of its own,
// rather than being labelled by whatever its list entry happens to say.
const presetNames = defineMessages({
	theme: { id: 'app.appearance-settings.accent.theme', defaultMessage: 'Theme' },
	amethyst: { id: 'app.appearance-settings.accent.amethyst', defaultMessage: 'Amethyst' },
	nightshade: { id: 'app.appearance-settings.accent.nightshade', defaultMessage: 'Nightshade' },
	midnight: { id: 'app.appearance-settings.accent.midnight', defaultMessage: 'Midnight' },
	glacier: { id: 'app.appearance-settings.accent.glacier', defaultMessage: 'Glacier' },
	verdant: { id: 'app.appearance-settings.accent.verdant', defaultMessage: 'Verdant' },
	lantern: { id: 'app.appearance-settings.accent.lantern', defaultMessage: 'Lantern' },
	ember: { id: 'app.appearance-settings.accent.ember', defaultMessage: 'Ember' },
	rose: { id: 'app.appearance-settings.accent.rose', defaultMessage: 'Rose' },
})

function presetName(preset: AccentPreset): string {
	const message = presetNames[preset.id as keyof typeof presetNames]
	return message ? formatMessage(message) : preset.name
}

type AppearanceSettingsState = {
	theme: ColorTheme
	syncAcrossDevices: boolean
	advancedRendering: boolean
	nativeDecorations: boolean
	accentPreset: string
}

function getAppearanceSettingsState(settings: AppSettings): AppearanceSettingsState {
	return {
		theme: settings.theme,
		syncAcrossDevices: settings.sync_theme_across_devices,
		advancedRendering: settings.advanced_rendering,
		nativeDecorations: settings.native_decorations,
		accentPreset: findAccentPreset(settings.accent_preset)?.id ?? DEFAULT_ACCENT_PRESET,
	}
}

const { saved, current, changes, saving, hasChanges, reset, save } = useSavable(
	() => getAppearanceSettingsState(settings.value),
	async (appearanceChanges) => {
		const value = current.value
		if (
			value.syncAcrossDevices &&
			auth.user.value &&
			(appearanceChanges.theme !== undefined || appearanceChanges.syncAcrossDevices !== undefined)
		) {
			await updatePreferences({
				appearance: value.theme === 'system' ? { auto: true } : { auto: false, theme: value.theme },
			})
		}

		const nextSettings: AppSettings = {
			...settings.value,
			theme: value.theme,
			sync_theme_across_devices: value.syncAcrossDevices,
			advanced_rendering: value.advancedRendering,
			native_decorations: value.nativeDecorations,
			accent_preset: value.accentPreset,
		}

		await set(nextSettings)
		settings.value = nextSettings
		theme.preferred = value.theme
		theme.syncAcrossDevices = value.syncAcrossDevices
		theme.advancedRendering = value.advancedRendering
	},
)

const themeOptions = computed(() =>
	theme.options.filter(
		(option) =>
			option !== 'retro' || settings.value.developer_mode || current.value.theme === 'retro',
	),
)

function setTheme(value: ColorTheme): void {
	current.value.theme = value
}

function setSyncAcrossDevices(enabled: boolean): void {
	current.value.syncAcrossDevices = enabled
}

function setAdvancedRendering(enabled: boolean): void {
	current.value.advancedRendering = enabled
}

function setNativeDecorations(enabled: boolean): void {
	current.value.nativeDecorations = enabled
}

// The accent is previewed the same way the theme is: the whole app recolours
// while the slider moves, and closing the tab without saving puts back what was
// saved — which `useSavable` has already restored into `saved` by then.
watch(
	() => current.value.accentPreset,
	(id) => setAccentPreset(id),
)

watch(
	[() => current.value.theme, () => saved.value.theme],
	([selectedTheme, savedTheme]) => {
		theme.preview = selectedTheme === savedTheme ? null : selectedTheme
	},
	{ immediate: true },
)

async function saveAppearanceSettings(): Promise<void> {
	try {
		await save()
	} catch {
		return
	}
}

onMounted(() => {
	settingsModal?.registerUnsavedChangesController({
		hasChanges: () => hasChanges.value,
		getOriginal: () => saved.value,
		getModified: () => changes.value,
		isSaving: () => saving.value,
		reset,
		save: saveAppearanceSettings,
	})
})

onBeforeUnmount(() => {
	theme.preview = null
	setAccentPreset(saved.value.accentPreset)
	settingsModal?.registerUnsavedChangesController(null)
})

provideAppearanceSettings({
	deferPersistence: true,
	theme: {
		current: computed(() => current.value.theme),
		options: themeOptions,
		system: computed(() => theme.native),
		set: setTheme,
		syncAcrossDevices: {
			value: computed(() => current.value.syncAcrossDevices),
			set: setSyncAcrossDevices,
		},
		syncDisabled: computed(() => !auth.user.value),
	},
	advancedRendering: {
		value: computed(() => current.value.advancedRendering),
		set: setAdvancedRendering,
	},
	nativeDecorations:
		os !== 'MacOS'
			? {
					value: computed(() => current.value.nativeDecorations),
					set: setNativeDecorations,
				}
			: undefined,
	updatePreferences,
})
</script>

<template>
	<AppearanceSettingsLayout />

	<!-- Noctrinth's own: the shared layout has no notion of an accent that can
	     be chosen, so the picker is added here rather than to it. -->
	<div class="mt-6 flex flex-col gap-3">
		<div>
			<h2 class="m-0 text-lg font-semibold text-contrast">
				{{ formatMessage(messages.accentTitle) }}
			</h2>
			<p class="m-0 mt-1 text-secondary">
				{{ formatMessage(messages.accentDescription) }}
			</p>
		</div>
		<div class="grid grid-cols-[repeat(auto-fill,minmax(9rem,1fr))] gap-2">
			<button
				v-for="preset in ACCENT_PRESETS"
				:key="preset.id"
				type="button"
				class="accent-option flex items-center gap-3 rounded-xl border border-solid border-button-border bg-button-bg p-3 text-left"
				:class="{ 'accent-option--selected': current.accentPreset === preset.id }"
				:style="{ '--accent-swatch': accentColorFor(preset, theme.active) }"
				:aria-pressed="current.accentPreset === preset.id"
				@click="current.accentPreset = preset.id"
			>
				<span class="accent-option__swatch" aria-hidden="true">
					<CheckIcon v-if="current.accentPreset === preset.id" class="size-4 text-contrast" />
				</span>
				<span class="truncate font-medium text-contrast">{{ presetName(preset) }}</span>
			</button>
		</div>
	</div>
</template>

<style scoped>
.accent-option {
	transition:
		border-color 150ms ease,
		box-shadow 150ms ease;
}

.accent-option:hover {
	border-color: var(--accent-swatch);
}

.accent-option--selected {
	border-color: var(--accent-swatch);
	box-shadow: 0 0 0 1px var(--accent-swatch);
}

.accent-option__swatch {
	display: grid;
	place-items: center;
	width: 1.75rem;
	height: 1.75rem;
	flex-shrink: 0;
	border-radius: 999px;
	background: var(--accent-swatch);
}
</style>
