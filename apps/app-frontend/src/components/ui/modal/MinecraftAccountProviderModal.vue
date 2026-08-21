<script setup lang="ts">
/**
 * Asks which kind of Minecraft account is being added.
 *
 * Noctrinth signs in with two: Microsoft and Ely.by. The accounts card offers
 * both, but everything else that says "sign in to Minecraft" — the getting
 * started checklist, most of all, which is the first thing a new install shows
 * — went straight to Microsoft, leaving no way in for an Ely.by player.
 */
import { LogInIcon } from '@modrinth/assets'
import { Button, defineMessages, NewModal, useVIntl } from '@modrinth/ui'
import { ref } from 'vue'

const emit = defineEmits<{
	microsoft: []
	ely: []
}>()

const { formatMessage } = useVIntl()

const messages = defineMessages({
	header: {
		id: 'app.account-provider.header',
		defaultMessage: 'Add an account',
	},
	description: {
		id: 'app.account-provider.description',
		defaultMessage: 'Which account do you play with?',
	},
	microsoft: {
		id: 'app.account-provider.microsoft',
		defaultMessage: 'Microsoft account',
	},
	microsoftHint: {
		id: 'app.account-provider.microsoft-hint',
		defaultMessage: 'A Minecraft account bought from Mojang.',
	},
	ely: {
		id: 'app.account-provider.ely',
		defaultMessage: 'Ely.by account',
	},
	elyHint: {
		id: 'app.account-provider.ely-hint',
		defaultMessage: 'Plays on servers that accept Ely.by.',
	},
})

const modal = ref<InstanceType<typeof NewModal>>()

function choose(provider: 'microsoft' | 'ely'): void {
	modal.value?.hide()
	emit(provider)
}

defineExpose({
	show: () => modal.value?.show(),
	hide: () => modal.value?.hide(),
})
</script>

<template>
	<NewModal ref="modal" :header="formatMessage(messages.header)" max-width="480px" width="100%">
		<div class="flex w-full flex-col gap-4">
			<p class="m-0 text-base leading-6 text-primary">
				{{ formatMessage(messages.description) }}
			</p>
			<div class="flex flex-col gap-2">
				<Button
					type="colored"
					color="brand"
					size="lg"
					class="!h-auto !justify-start !py-3"
					native-type="button"
					@click="choose('microsoft')"
				>
					<LogInIcon aria-hidden="true" />
					<span class="flex min-w-0 flex-col items-start">
						<span class="font-semibold leading-5">{{ formatMessage(messages.microsoft) }}</span>
						<span class="text-sm font-normal leading-5 opacity-80">
							{{ formatMessage(messages.microsoftHint) }}
						</span>
					</span>
				</Button>
				<Button
					size="lg"
					class="!h-auto !justify-start !py-3"
					native-type="button"
					@click="choose('ely')"
				>
					<LogInIcon aria-hidden="true" />
					<span class="flex min-w-0 flex-col items-start">
						<span class="font-semibold leading-5">{{ formatMessage(messages.ely) }}</span>
						<span class="text-sm font-normal leading-5 text-secondary">
							{{ formatMessage(messages.elyHint) }}
						</span>
					</span>
				</Button>
			</div>
		</div>
	</NewModal>
</template>
