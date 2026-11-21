<template>
	<Teleport to="body">
		<section
			v-if="showControls"
			ref="controlsEl"
			class="app-window-controls pointer-events-auto fixed top-0 right-0 z-[10001] flex h-12 items-center gap-2 bg-bg-raised rounded-bl-2xl px-1.5"
			data-tauri-drag-region-exclude
		>
			<IconButton
				type="quiet"
				:label="formatMessage(messages.minimize)"
				class="relative before:absolute before:-inset-y-[9px] before:-inset-x-[6px] before:content-['']"
				@click="getCurrentWindow().minimize()"
			>
				<MinimizeIcon />
			</IconButton>
			<IconButton
				type="quiet"
				:label="formatMessage(messages.toggleMaximize)"
				class="relative before:absolute before:-inset-y-[9px] before:-inset-x-[6px] before:content-['']"
				@click="getCurrentWindow().toggleMaximize()"
			>
				<RestoreIcon v-if="isMaximized" />
				<MaximizeIcon v-else />
			</IconButton>
			<IconButton
				type="quiet"
				:label="formatMessage(messages.close)"
				class="close-button relative before:absolute before:-inset-y-[9px] before:-left-[6px] before:-right-[9px] before:content-[''] hover:!bg-red focus-visible:!bg-red"
				@click="handleClose"
			>
				<XIcon />
			</IconButton>
		</section>
	</Teleport>
</template>

<script setup>
import { MaximizeIcon, MinimizeIcon, RestoreIcon, XIcon } from '@modrinth/assets'
import { defineMessages, IconButton, useVIntl } from '@modrinth/ui'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { saveWindowState, StateFlags } from '@tauri-apps/plugin-window-state'
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'

import { useAppSettings } from '@/composables/use-app-settings.ts'
import { get as getSettings } from '@/helpers/settings.ts'
import { getOS } from '@/helpers/utils.js'

const { formatMessage } = useVIntl()

const messages = defineMessages({
	minimize: { id: 'app.window-controls.minimize', defaultMessage: 'Minimize window' },
	toggleMaximize: {
		id: 'app.window-controls.toggle-maximize',
		defaultMessage: 'Toggle maximize window',
	},
	close: { id: 'app.window-controls.close', defaultMessage: 'Close window' },
})

const appSettings = useAppSettings()

function detectInitialOs() {
	const ua = navigator.userAgent
	if (ua.includes('Windows')) return 'Windows'
	if (ua.includes('Mac')) return 'MacOS'
	if (ua.includes('Linux')) return 'Linux'
	return ''
}

const initialOs = detectInitialOs()
const nativeDecorations = ref(initialOs === 'MacOS')
const isMaximized = ref(false)
const os = ref(initialOs)
const controlsEl = ref(null)

const alwaysShowAppControls = computed(() => appSettings.getFeatureFlag('always_show_app_controls'))

const showControls = computed(
	() =>
		alwaysShowAppControls.value ||
		(!nativeDecorations.value && (os.value === 'Windows' || os.value === 'Linux')),
)

function setWindowControlsWidth(width) {
	document.documentElement.style.setProperty('--window-controls-width', `${width}px`)
}

let resizeObserver
watch(controlsEl, (el) => {
	resizeObserver?.disconnect()
	resizeObserver = undefined
	if (!el) {
		setWindowControlsWidth(0)
		return
	}

	resizeObserver = new ResizeObserver(() => {
		setWindowControlsWidth(el.getBoundingClientRect().width)
	})
	resizeObserver.observe(el)
	setWindowControlsWidth(el.getBoundingClientRect().width)
})

let unlistenResized
let unmounted = false

onMounted(async () => {
	const [detectedOs, settings, maximized] = await Promise.all([
		getOS(),
		getSettings(),
		getCurrentWindow().isMaximized(),
	])
	if (unmounted) return

	os.value = detectedOs
	nativeDecorations.value = settings.native_decorations
	isMaximized.value = maximized

	if (os.value !== 'MacOS') {
		void getCurrentWindow().setDecorations(nativeDecorations.value)
	}

	const unlisten = await getCurrentWindow().onResized(async () => {
		isMaximized.value = await getCurrentWindow().isMaximized()
	})
	if (unmounted) {
		unlisten()
	} else {
		unlistenResized = unlisten
	}
})

onUnmounted(() => {
	unmounted = true
	unlistenResized?.()
	resizeObserver?.disconnect()
	document.documentElement.style.removeProperty('--window-controls-width')
})

const handleClose = async () => {
	await saveWindowState(StateFlags.ALL)
	await getCurrentWindow().close()
}
</script>
<style scoped>
.close-button:hover {
	color: var(--color-accent-contrast) !important;
}
</style>
