<script setup lang="ts">
/** The splash cube's choice, in the developer-only flags tab. */
import { defineMessages, RadioButtons, useVIntl } from '@modrinth/ui'
import { ref, watch } from 'vue'

import {
	readSplashCubeChoice,
	SPLASH_CUBE_CHOICES,
	type SplashCubeChoice,
	writeSplashCubeChoice,
} from '@/helpers/noctrinth-splash-cube'

const { formatMessage } = useVIntl()

const messages = defineMessages({
	title: { id: 'noctrinth.settings.splash-cube.title', defaultMessage: 'Splash cube' },
	description: {
		id: 'noctrinth.settings.splash-cube.description',
		defaultMessage: 'Which cube the splash screen shows. Takes effect on the next launch.',
	},
	theme: {
		id: 'noctrinth.settings.splash-cube.theme',
		defaultMessage: 'Matches the theme (rarely the other way round)',
	},
	inverted: {
		id: 'noctrinth.settings.splash-cube.inverted',
		defaultMessage: 'Inverted: black on white, white on black',
	},
	random: { id: 'noctrinth.settings.splash-cube.random', defaultMessage: 'Random every launch' },
})

const choice = ref<SplashCubeChoice>(readSplashCubeChoice())
watch(choice, writeSplashCubeChoice)
</script>

<template>
	<div class="flex flex-col gap-2 border-0 border-b border-solid border-surface-5 pb-4">
		<div>
			<h2 class="m-0 text-lg font-semibold text-contrast">{{ formatMessage(messages.title) }}</h2>
			<p class="m-0 text-secondary">{{ formatMessage(messages.description) }}</p>
		</div>
		<RadioButtons v-model="choice" :items="[...SPLASH_CUBE_CHOICES]">
			<template #default="{ item }">
				{{ formatMessage(messages[item as SplashCubeChoice]) }}
			</template>
		</RadioButtons>
	</div>
</template>
