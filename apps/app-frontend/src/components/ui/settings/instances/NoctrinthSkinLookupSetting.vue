<script setup lang="ts">
/**
 * The fork's by-name skin lookup, as a row of the synced instance settings.
 *
 * It lives in a file of its own rather than in upstream's page: that page is
 * rewritten wholesale often enough that a row of the fork's inside it is a
 * conflict on every sync, and one line of it here is not.
 */
import { defineMessages, injectNotificationManager, Toggle, useVIntl } from '@modrinth/ui'
import { useMutation, useQuery, useQueryClient } from '@tanstack/vue-query'
import { computed } from 'vue'

import { appSettingsKeys, appSettingsQueryOptions, get, set } from '@/helpers/settings'

const { handleError } = injectNotificationManager()
const { formatMessage } = useVIntl()
const queryClient = useQueryClient()

const settingsQuery = useQuery(appSettingsQueryOptions())
const enabled = computed(() => settingsQuery.data.value?.universal_skins ?? false)

const mutation = useMutation({
	mutationKey: appSettingsKeys.update,
	scope: { id: 'app-settings' },
	mutationFn: async (universal_skins: boolean) => {
		await set({ ...(await get()), universal_skins })
	},
	onMutate: () => queryClient.cancelQueries({ queryKey: appSettingsKeys.all }),
	onError: handleError,
	onSettled: async () => {
		if (queryClient.isMutating({ mutationKey: appSettingsKeys.update }) === 1) {
			await queryClient.invalidateQueries({ queryKey: appSettingsKeys.all })
		}
	},
})

const messages = defineMessages({
	title: {
		id: 'app.settings.default-instance-options.universal-skins.title',
		defaultMessage: 'Skins for every player',
	},
	description: {
		id: 'app.settings.default-instance-options.universal-skins.description',
		defaultMessage:
			'Look a player’s skin up by name when the server sends none, so nobody is Steve on an offline-mode server.',
	},
})
</script>

<template>
	<section class="border-0 border-b border-solid border-surface-4 pb-6">
		<div class="flex items-center justify-between gap-6">
			<div class="flex min-w-0 flex-col gap-1">
				<h2 class="m-0 text-lg font-semibold text-contrast">
					{{ formatMessage(messages.title) }}
				</h2>
				<p class="m-0 text-secondary">
					{{ formatMessage(messages.description) }}
				</p>
			</div>
			<Toggle
				id="universal-skins"
				:model-value="enabled"
				:disabled="settingsQuery.isPending.value"
				:aria-label="formatMessage(messages.title)"
				@update:model-value="(value: boolean) => mutation.mutate(value)"
			/>
		</div>
	</section>
</template>
