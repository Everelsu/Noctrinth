<script setup>
import { GlobeIcon } from '@modrinth/assets'
import { defineMessages, injectNotificationManager, Input, Slider, useVIntl } from '@modrinth/ui'
import { useMutation, useQueryClient } from '@tanstack/vue-query'
import { ref, watch } from 'vue'

import { appSettingsKeys, get, set } from '@/helpers/settings.ts'

import AppDirectorySettings from './app-directory/index.vue'
import ContentStorageSettings from './content-storage.vue'

const { formatMessage } = useVIntl()
const { handleError } = injectNotificationManager()
const queryClient = useQueryClient()
const settings = ref(await get())

const messages = defineMessages({
	proxyTitle: {
		id: 'app.resource-settings.proxy.title',
		defaultMessage: 'Proxy',
	},
	proxyDescription: {
		id: 'app.resource-settings.proxy.description',
		defaultMessage:
			'Routes all launcher network traffic (Modrinth API and CDN downloads) through a proxy. Supports http://, https://, socks5:// and socks5h:// URLs. Useful when Modrinth is not reachable in your region. Leave empty to connect directly. (app restart required to take effect)',
	},
	maximumConcurrentDownloadsTitle: {
		id: 'app.settings.resource-management.maximum-concurrent-downloads.title',
		defaultMessage: 'Maximum concurrent downloads',
	},
	maximumConcurrentDownloadsDescription: {
		id: 'app.settings.resource-management.maximum-concurrent-downloads.description',
		defaultMessage:
			'Number of files the app can download at once. Lower this if downloads are unreliable on your connection. Requires an app restart.',
	},
	maximumConcurrentWritesTitle: {
		id: 'app.settings.resource-management.maximum-concurrent-writes.title',
		defaultMessage: 'Maximum concurrent writes',
	},
	maximumConcurrentWritesDescription: {
		id: 'app.settings.resource-management.maximum-concurrent-writes.description',
		defaultMessage:
			'Number of files the app can write to disk at once. Lower this if you frequently encounter I/O errors. Requires an app restart.',
	},
})

const limitsMutation = useMutation({
	networkMode: 'always',
	mutationKey: appSettingsKeys.update,
	scope: { id: 'app-settings' },
	mutationFn: async (limits) => set({ ...(await get()), ...limits }),
	onError: handleError,
	onSettled: () => queryClient.invalidateQueries({ queryKey: appSettingsKeys.all }),
})

watch(
	() => ({
		max_concurrent_downloads: settings.value.max_concurrent_downloads,
		proxy_url: settings.value.proxy_url || null,
		max_concurrent_writes: settings.value.max_concurrent_writes,
	}),
	(limits) => limitsMutation.mutate(limits),
)
</script>

<template>
	<div class="flex flex-col gap-6">
		<ContentStorageSettings />
		<AppDirectorySettings />

		<div class="flex flex-col gap-2.5">
			<h2 class="m-0 text-lg font-semibold text-contrast">
				{{ formatMessage(messages.proxyTitle) }}
			</h2>
			<Input
				id="proxy-url"
				v-model="settings.proxy_url"
				:icon="GlobeIcon"
				type="text"
				placeholder="socks5://127.0.0.1:1080"
				wrapper-class="w-full"
			/>
			<p class="m-0 leading-tight text-secondary">
				{{ formatMessage(messages.proxyDescription) }}
			</p>
		</div>

		<div class="flex flex-col gap-2.5">
			<h2 class="m-0 text-lg font-semibold text-contrast mt-4">
				{{ formatMessage(messages.maximumConcurrentDownloadsTitle) }}
			</h2>
			<Slider
				id="max-downloads"
				v-model="settings.max_concurrent_downloads"
				:min="1"
				:max="10"
				:step="1"
			/>
			<p class="m-0 leading-tight text-secondary">
				{{ formatMessage(messages.maximumConcurrentDownloadsDescription) }}
			</p>
		</div>

		<div class="flex flex-col gap-2.5">
			<h2 class="mt-0 m-0 text-lg font-semibold text-contrast">
				{{ formatMessage(messages.maximumConcurrentWritesTitle) }}
			</h2>
			<Slider
				id="max-writes"
				v-model="settings.max_concurrent_writes"
				:min="1"
				:max="50"
				:step="1"
			/>
			<p class="m-0 leading-tight text-secondary">
				{{ formatMessage(messages.maximumConcurrentWritesDescription) }}
			</p>
		</div>
	</div>
</template>
