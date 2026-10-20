<script setup lang="ts">
/**
 * Noctrinth's own: where Java is downloaded from. Automatic tries every
 * provider in turn, so one that is down or blocked costs a retry rather than
 * the install; any one of them can be pinned instead.
 */
import { defineMessages, RadioButtons, useVIntl } from '@modrinth/ui'

import { useDefaultInstanceSettings } from '../use-default-instance-settings'

const { formatMessage } = useVIntl()

const PROVIDERS = ['auto', 'azul', 'temurin', 'mojang', 'mojang-mirror'] as const
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
	description: {
		id: 'noctrinth.settings.java-provider.description',
		defaultMessage:
			'Where the launcher gets Java when an instance needs one it does not have. Every download is checked against the checksum its source publishes.',
	},
	auto: {
		id: 'noctrinth.settings.java-provider.auto',
		defaultMessage: 'Automatically: Azul, then Temurin, then Mojang, then the BMCLAPI mirror',
	},
	azul: { id: 'noctrinth.settings.java-provider.azul', defaultMessage: 'Azul Zulu' },
	temurin: { id: 'noctrinth.settings.java-provider.temurin', defaultMessage: 'Eclipse Temurin' },
	mojang: {
		id: 'noctrinth.settings.java-provider.mojang',
		defaultMessage: 'Mojang, the Java the official launcher uses',
	},
	'mojang-mirror': {
		id: 'noctrinth.settings.java-provider.mojang-mirror',
		defaultMessage: "Mojang's Java through the BMCLAPI mirror, a third-party service",
	},
})
</script>

<template>
	<div v-if="settings" class="flex flex-col gap-2.5">
		<h3 class="m-0 text-lg font-semibold text-contrast">{{ formatMessage(messages.title) }}</h3>
		<p class="m-0 leading-tight text-secondary">{{ formatMessage(messages.description) }}</p>
		<RadioButtons v-model="settings.provider" :items="[...PROVIDERS]">
			<template #default="{ item }">
				{{ formatMessage(messages[item as Provider]) }}
			</template>
		</RadioButtons>
	</div>
</template>
