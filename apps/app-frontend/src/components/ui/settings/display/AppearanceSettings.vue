<script setup lang="ts">
import {
	AppearanceSettingsLayout,
	defineMessages,
	injectAuth,
	injectUserPreferences,
	provideAppearanceSettings,
	Slider,
	useSavable,
	useVIntl,
} from '@modrinth/ui'
import { computed, inject, onBeforeUnmount, onMounted, ref, watch } from 'vue'

import {
	ACCENT_BRIGHTNESS_MAX,
	ACCENT_BRIGHTNESS_MIN,
	ACCENT_DEFAULT,
	ACCENT_INTENSITY_MAX,
	ACCENT_INTENSITY_MIN,
	ACCENT_STEP,
	clampAccentBrightness,
	clampAccentIntensity,
	setAccent,
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
	accentIntensityTitle: {
		id: 'app.appearance-settings.accent-intensity.title',
		defaultMessage: 'Accent intensity',
	},
	accentIntensityDescription: {
		id: 'app.appearance-settings.accent-intensity.description',
		defaultMessage:
			"How strongly the theme's accent colour is drawn, from nearly grey to further out than any theme goes. 100% is the theme as it ships.",
	},
	accentBrightnessTitle: {
		id: 'app.appearance-settings.accent-brightness.title',
		defaultMessage: 'Accent brightness',
	},
	accentBrightnessDescription: {
		id: 'app.appearance-settings.accent-brightness.description',
		defaultMessage:
			'How light that same colour is drawn. Below 100% it deepens without changing hue.',
	},
})

type AppearanceSettingsState = {
	theme: ColorTheme
	syncAcrossDevices: boolean
	advancedRendering: boolean
	nativeDecorations: boolean
	accentIntensity: number
	accentBrightness: number
}

function getAppearanceSettingsState(settings: AppSettings): AppearanceSettingsState {
	return {
		theme: settings.theme,
		syncAcrossDevices: settings.sync_theme_across_devices,
		advancedRendering: settings.advanced_rendering,
		nativeDecorations: settings.native_decorations,
		accentIntensity: clampAccentIntensity(settings.accent_intensity ?? ACCENT_DEFAULT),
		accentBrightness: clampAccentBrightness(settings.accent_brightness ?? ACCENT_DEFAULT),
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
			accent_intensity: value.accentIntensity,
			accent_brightness: value.accentBrightness,
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
	[() => current.value.accentIntensity, () => current.value.accentBrightness],
	([intensity, brightness]) => setAccent(intensity, brightness),
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
	setAccent(saved.value.accentIntensity, saved.value.accentBrightness)
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
	     be turned up or down, so the row is added here rather than to it. -->
	<div class="mt-6 flex flex-col gap-2">
		<div>
			<h2 id="accent-intensity-label" class="m-0 text-lg font-semibold text-contrast">
				{{ formatMessage(messages.accentIntensityTitle) }}
			</h2>
			<p class="m-0 mt-1 text-secondary">
				{{ formatMessage(messages.accentIntensityDescription) }}
			</p>
		</div>
		<Slider
			id="accent-intensity"
			v-model="current.accentIntensity"
			aria-labelledby="accent-intensity-label"
			:min="ACCENT_INTENSITY_MIN"
			:max="ACCENT_INTENSITY_MAX"
			:step="ACCENT_STEP"
			unit="%"
		/>
	</div>

	<div class="mt-6 flex flex-col gap-2">
		<div>
			<h2 id="accent-brightness-label" class="m-0 text-lg font-semibold text-contrast">
				{{ formatMessage(messages.accentBrightnessTitle) }}
			</h2>
			<p class="m-0 mt-1 text-secondary">
				{{ formatMessage(messages.accentBrightnessDescription) }}
			</p>
		</div>
		<Slider
			id="accent-brightness"
			v-model="current.accentBrightness"
			aria-labelledby="accent-brightness-label"
			:min="ACCENT_BRIGHTNESS_MIN"
			:max="ACCENT_BRIGHTNESS_MAX"
			:step="ACCENT_STEP"
			unit="%"
		/>
	</div>
</template>
