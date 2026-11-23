<script setup lang="ts">
import { PlugIcon } from '@modrinth/assets'
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useRoute } from 'vue-router'

import {
	buildSandboxedPanelSrcdoc,
	pluginPages,
	registerPluginPanelSink,
	sendMessageFromPluginPanel,
	unregisterPluginPanelSink,
} from '@/helpers/noctrinth-plugins'
import { useBreadcrumbs } from '@/store/breadcrumbs'

const route = useRoute()
const breadcrumbs = useBreadcrumbs()
const iframeRef = ref<HTMLIFrameElement | null>(null)

const currentPage = computed(() => {
	const pluginId = String(route.params.pluginId ?? '')
	const pageId = String(route.params.pageId ?? '')
	return pluginPages.value.find((p) => p.pluginId === pluginId && p.pageId === pageId) ?? null
})

const srcdoc = computed(() => {
	const page = currentPage.value
	if (!page) return ''
	return buildSandboxedPanelSrcdoc(page.html, page.pluginId, page.pageId)
})

function handleWindowMessage(event: MessageEvent) {
	const page = currentPage.value
	if (!page || !iframeRef.value || event.source !== iframeRef.value.contentWindow) return
	const msg = event.data
	if (!msg || msg.__noctrinthPanel !== true) return
	if (msg.pluginId !== page.pluginId || msg.panelId !== page.pageId) return
	if (msg.type === 'message') {
		sendMessageFromPluginPanel(page.pluginId, page.pageId, msg.data)
	}
}

watch(
	currentPage,
	(next, prev) => {
		if (prev) {
			unregisterPluginPanelSink(prev.pluginId, prev.pageId)
		}
		if (next) {
			breadcrumbs.setRootContext({ name: next.title, link: route.path })
			registerPluginPanelSink(next.pluginId, next.pageId, (data) => {
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
	if (currentPage.value) {
		unregisterPluginPanelSink(currentPage.value.pluginId, currentPage.value.pageId)
	}
})
</script>

<template>
	<div class="flex h-full min-h-[calc(100vh-4rem)] w-full flex-col">
		<iframe
			v-if="currentPage"
			ref="iframeRef"
			:srcdoc="srcdoc"
			sandbox="allow-scripts"
			class="h-full w-full flex-1 border-0 bg-transparent"
		/>
		<div
			v-else
			class="flex flex-1 flex-col items-center justify-center gap-3 p-8 text-center text-secondary"
		>
			<PlugIcon class="size-8 opacity-60" aria-hidden="true" />
			<p class="m-0 text-sm">Plugin page is not available or the plugin is disabled.</p>
		</div>
	</div>
</template>
