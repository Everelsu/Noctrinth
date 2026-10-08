<script setup lang="ts">
/** Whether project descriptions arrive already translated. */
import { defineMessages, Toggle, useVIntl } from '@modrinth/ui'
import { ref, watch } from 'vue'

import { readAutoTranslate, writeAutoTranslate } from '@/helpers/noctrinth-translate'

const { formatMessage } = useVIntl()

const messages = defineMessages({
	title: {
		id: 'noctrinth.settings.auto-translate.title',
		defaultMessage: 'Translate descriptions automatically',
	},
	description: {
		id: 'noctrinth.settings.auto-translate.description',
		defaultMessage:
			"Project descriptions open translated into the launcher's language by Google Translate. Each one keeps a button back to the original.",
	},
})

const enabled = ref(readAutoTranslate())
watch(enabled, writeAutoTranslate)
</script>

<template>
	<div class="mt-6 flex items-center justify-between gap-4">
		<div>
			<h2 class="m-0 text-lg font-semibold text-contrast">{{ formatMessage(messages.title) }}</h2>
			<p class="m-0 mt-1 text-secondary">{{ formatMessage(messages.description) }}</p>
		</div>
		<Toggle id="noctrinth-auto-translate" v-model="enabled" />
	</div>
</template>
