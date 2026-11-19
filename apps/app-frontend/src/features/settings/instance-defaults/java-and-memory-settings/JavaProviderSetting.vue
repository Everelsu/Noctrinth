<script setup lang="ts">
/**
 * Noctrinth's own: where Java is downloaded from. Automatic tries every
 * source in turn, so one that is down or blocked costs a retry rather than
 * the install; any one of them can be pinned instead.
 */
import { Combobox, type ComboboxOption, defineMessages, useVIntl } from '@modrinth/ui'
import { computed } from 'vue'

import { useDefaultInstanceSettings } from '../use-default-instance-settings'

const { formatMessage } = useVIntl()

const PROVIDERS = ['auto', 'azul', 'temurin', 'temurin-github', 'mojang', 'mojang-mirror'] as const
type Provider = (typeof PROVIDERS)[number]

const { settings } = useDefaultInstanceSettings(
	(value) => ({
		provider: (PROVIDERS as readonly string[]).includes(value.java_provider)
			? (value.java_provider as Provider)
			: 'auto',
	}),
	(value) => ({ java_provider: value.provider }),
)

const messages = defineMessages({
	title: { id: 'noctrinth.settings.java-provider.title', defaultMessage: 'Download Java from' },
	auto: { id: 'noctrinth.settings.java-provider.auto', defaultMessage: 'Automatically' },
	azul: { id: 'noctrinth.settings.java-provider.azul', defaultMessage: 'Azul Zulu' },
	temurin: { id: 'noctrinth.settings.java-provider.temurin', defaultMessage: 'Eclipse Temurin' },
	'temurin-github': {
		id: 'noctrinth.settings.java-provider.temurin-github',
		defaultMessage: 'Eclipse Temurin from GitHub',
	},
	mojang: { id: 'noctrinth.settings.java-provider.mojang', defaultMessage: 'Mojang' },
	'mojang-mirror': {
		id: 'noctrinth.settings.java-provider.mojang-mirror',
		defaultMessage: 'Mojang through BMCLAPI (third-party mirror)',
	},
})

const options = computed<ComboboxOption<Provider>[]>(() =>
	PROVIDERS.map((provider) => ({ value: provider, label: formatMessage(messages[provider]) })),
)
</script>

<template>
	<div v-if="settings" class="flex flex-col gap-2.5">
		<div class="flex flex-wrap items-center justify-between gap-4">
			<div class="min-w-0 flex-1">
				<h2 id="java-provider-label" class="m-0 text-lg font-semibold text-contrast">
					{{ formatMessage(messages.title) }}
				</h2>
			</div>
			<Combobox
				v-model="settings.provider"
				:options="options"
				aria-labelledby="java-provider-label"
				class="w-72 max-w-full"
			/>
		</div>
	</div>
</template>
