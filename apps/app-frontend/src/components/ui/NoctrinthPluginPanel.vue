<script setup lang="ts">
import { PlugIcon, XIcon } from '@modrinth/assets'
import { IconButton } from '@modrinth/ui'
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'

import {
	activePluginPanel,
	buildSandboxedPanelSrcdoc,
	closePluginPanel,
	registerPluginPanelSink,
	sendMessageFromPluginPanel,
	unregisterPluginPanelSink,
} from '@/helpers/noctrinth-plugins'

const iframeRef = ref<HTMLIFrameElement | null>(null)

const srcdoc = computed(() => {
	const panel = activePluginPanel.value
	if (!panel) return ''
	return buildSandboxedPanelSrcdoc(panel.html, panel.pluginId, panel.panelId)
})

function handleWindowMessage(event: MessageEvent) {
	const panel = activePluginPanel.value
	if (!panel || !iframeRef.value || event.source !== iframeRef.value.contentWindow) return
	const msg = event.data
	if (!msg || msg.__noctrinthPanel !== true) return
	if (msg.pluginId !== panel.pluginId || msg.panelId !== panel.panelId) return
	if (msg.type === 'close') {
		closePluginPanel()
	} else if (msg.type === 'message') {
		sendMessageFromPluginPanel(panel.pluginId, panel.panelId, msg.data)
	}
}

watch(
	activePluginPanel,
	(next, prev) => {
		if (prev) {
			unregisterPluginPanelSink(prev.pluginId, prev.panelId)
		}
		if (next) {
			registerPluginPanelSink(next.pluginId, next.panelId, (data) => {
				iframeRef.value?.contentWindow?.postMessage({ __noctrinthHost: true, data }, '*')
			})
		}
	},
	{ immediate: true },
)

onMounted(() => {
	window.addEventListener('message', handleWindowMessage)
})

onBeforeUnmount(() => {
	window.removeEventListener('message', handleWindowMessage)
	if (activePluginPanel.value) {
		unregisterPluginPanelSink(activePluginPanel.value.pluginId, activePluginPanel.value.panelId)
	}
})
</script>

<template>
	<Transition name="plugin-panel">
		<div
			v-if="activePluginPanel"
			class="fixed inset-0 z-40 flex justify-end bg-black/45 backdrop-blur-[2px]"
			@click.self="closePluginPanel"
		>
			<aside
				class="flex h-full max-w-[92vw] flex-col border-0 border-l border-solid border-surface-5 bg-surface-2 shadow-2xl"
				:style="{ width: `${activePluginPanel.width}px` }"
			>
				<header
					class="flex shrink-0 items-center justify-between gap-3 border-0 border-b border-solid border-surface-5 px-4 py-3"
				>
					<div class="flex min-w-0 items-center gap-2">
						<PlugIcon class="size-4 shrink-0 text-brand" aria-hidden="true" />
						<div class="min-w-0">
							<div class="truncate text-sm font-semibold text-contrast">
								{{ activePluginPanel.title }}
							</div>
							<div class="truncate text-xs text-secondary">
								{{ activePluginPanel.pluginName }}
							</div>
						</div>
					</div>
					<IconButton type="quiet" size="sm" label="Close" @click="closePluginPanel">
						<XIcon />
					</IconButton>
				</header>
				<iframe
					ref="iframeRef"
					:srcdoc="srcdoc"
					sandbox="allow-scripts"
					class="h-full w-full flex-1 border-0 bg-transparent"
				/>
			</aside>
		</div>
	</Transition>
</template>

<style scoped>
.plugin-panel-enter-active,
.plugin-panel-leave-active {
	transition: opacity 0.18s ease;
}
.plugin-panel-enter-from,
.plugin-panel-leave-to {
	opacity: 0;
}
</style>
