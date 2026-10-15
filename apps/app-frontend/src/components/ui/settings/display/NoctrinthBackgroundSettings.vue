<script setup lang="ts">
/** The background picture, under the accent in Appearance. */
import { ImageIcon, TrashIcon } from '@modrinth/assets'
import { Button, defineMessages, injectNotificationManager, Toggle, useVIntl } from '@modrinth/ui'
import { ref } from 'vue'

import {
	backgroundBlur,
	backgroundUrl,
	chooseBackground,
	clearBackground,
	saveBackgroundBlur,
} from '@/helpers/noctrinth-background'

const { formatMessage } = useVIntl()
const { handleError } = injectNotificationManager()

const messages = defineMessages({
	title: { id: 'noctrinth.settings.background.title', defaultMessage: 'Background picture' },
	description: {
		id: 'noctrinth.settings.background.description',
		defaultMessage:
			'A picture behind the launcher, showing through its frame. Cards on a page stay solid, so text on them is as easy to read as before.',
	},
	choose: { id: 'noctrinth.settings.background.choose', defaultMessage: 'Choose picture' },
	change: { id: 'noctrinth.settings.background.change', defaultMessage: 'Change picture' },
	remove: { id: 'noctrinth.settings.background.remove', defaultMessage: 'Remove' },
	blurTitle: { id: 'noctrinth.settings.background.blur.title', defaultMessage: 'Blur the picture' },
	blurDescription: {
		id: 'noctrinth.settings.background.blur.description',
		defaultMessage: 'Softens it, so it stays behind what is in front of it.',
	},
})

const busy = ref(false)

async function run(action: () => Promise<unknown>) {
	busy.value = true
	try {
		await action()
	} catch (error) {
		handleError(error as Error)
	} finally {
		busy.value = false
	}
}
</script>

<template>
	<div class="mt-6 flex flex-col gap-3">
		<div>
			<h2 class="m-0 text-lg font-semibold text-contrast">{{ formatMessage(messages.title) }}</h2>
			<p class="m-0 mt-1 text-secondary">{{ formatMessage(messages.description) }}</p>
		</div>
		<div class="flex flex-wrap items-center gap-4">
			<div
				class="aspect-video w-48 shrink-0 overflow-hidden rounded-xl border border-solid border-surface-5 bg-surface-2 bg-cover bg-center"
				:class="{ 'grid place-items-center text-secondary': !backgroundUrl }"
				:style="backgroundUrl ? { backgroundImage: `url(${backgroundUrl})` } : undefined"
			>
				<ImageIcon v-if="!backgroundUrl" class="size-8" />
			</div>
			<div class="flex flex-wrap gap-2">
				<Button :disabled="busy" @click="run(chooseBackground)">
					<ImageIcon />
					{{ formatMessage(backgroundUrl ? messages.change : messages.choose) }}
				</Button>
				<Button v-if="backgroundUrl" type="quiet" :disabled="busy" @click="run(clearBackground)">
					<TrashIcon />
					{{ formatMessage(messages.remove) }}
				</Button>
			</div>
		</div>
		<div v-if="backgroundUrl" class="flex items-center justify-between gap-4">
			<div class="min-w-0">
				<h3 id="background-blur-label" class="m-0 text-lg font-semibold text-contrast">
					{{ formatMessage(messages.blurTitle) }}
				</h3>
				<p class="m-0 mt-1 text-secondary">{{ formatMessage(messages.blurDescription) }}</p>
			</div>
			<Toggle
				id="background-blur"
				:model-value="backgroundBlur"
				aria-labelledby="background-blur-label"
				@update:model-value="(value: boolean) => run(() => saveBackgroundBlur(value))"
			/>
		</div>
	</div>
</template>
