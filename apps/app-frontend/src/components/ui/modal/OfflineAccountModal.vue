<template>
	<NewModal
		ref="modal"
		:header="formatMessage(messages.header)"
		max-width="480px"
		width="100%"
		actions-divider
	>
		<div class="flex w-full flex-col gap-4">
			<p class="m-0 text-base leading-6 text-secondary">
				{{ formatMessage(messages.description) }}
			</p>

			<div class="flex flex-col gap-2">
				<label for="offline-username">
					<span class="text-lg font-semibold text-contrast">
						{{ formatMessage(messages.usernameLabel) }}
					</span>
				</label>
				<Input
					id="offline-username"
					v-model="username"
					:placeholder="formatMessage(messages.usernamePlaceholder)"
					autocomplete="off"
					:disabled="loading"
					@keyup.enter="submit"
				/>
				<span v-if="username && !uuid" class="text-sm text-brand-red">
					{{ formatMessage(messages.invalid) }}
				</span>
				<span v-else-if="uuid" class="text-sm text-secondary">
					{{ formatMessage(messages.uuid, { uuid }) }}
				</span>
			</div>

			<Admonition type="warning" :header="formatMessage(messages.limitationHeader)">
				{{ formatMessage(messages.limitationBody) }}
			</Admonition>
		</div>

		<template #actions>
			<div class="flex justify-end gap-2">
				<Button :disabled="loading" type="outlined" @click="hide">
					<XIcon aria-hidden="true" />
					{{ formatMessage(commonMessages.cancelButton) }}
				</Button>
				<Button :disabled="loading || !uuid" type="colored" color="brand" @click="submit">
					<SpinnerIcon v-if="loading" class="animate-spin" aria-hidden="true" />
					<PlusIcon v-else aria-hidden="true" />
					{{ formatMessage(messages.add) }}
				</Button>
			</div>
		</template>
	</NewModal>
</template>

<script setup lang="ts">
/**
 * Making an account that is only a name.
 *
 * The UUID is shown while it is typed because it is the one thing about an
 * offline account that is not obvious, and the one thing that decides whether
 * a world already on this machine recognises the player: change the name and
 * everything that world knew about them stays behind under the old one.
 */
import { PlusIcon, SpinnerIcon, XIcon } from '@modrinth/assets'
import {
	Admonition,
	Button,
	commonMessages,
	defineMessages,
	Input,
	NewModal,
	useVIntl,
} from '@modrinth/ui'
import { ref, watch } from 'vue'

import { offline_add, offline_preview_uuid, type OfflineCredentials } from '@/helpers/offline_auth'

const { formatMessage } = useVIntl()

const emit = defineEmits<{ added: [OfflineCredentials] }>()

const modal = ref<InstanceType<typeof NewModal>>()
const username = ref('')
const uuid = ref<string | null>(null)
const loading = ref(false)

watch(username, async (value) => {
	uuid.value = value.trim() ? await offline_preview_uuid(value.trim()).catch(() => null) : null
})

function show(event?: MouseEvent) {
	username.value = ''
	uuid.value = null
	loading.value = false
	modal.value?.show(event)
}

function hide() {
	modal.value?.hide()
}

async function submit() {
	if (!uuid.value || loading.value) return

	loading.value = true
	try {
		emit('added', await offline_add(username.value.trim()))
		hide()
	} finally {
		loading.value = false
	}
}

const messages = defineMessages({
	header: { id: 'offline-account.header', defaultMessage: 'Add an offline account' },
	description: {
		id: 'offline-account.description',
		defaultMessage:
			'A name and nothing else. It is what the launcher plays as when there is no connection to sign in with.',
	},
	usernameLabel: { id: 'offline-account.username-label', defaultMessage: 'Name' },
	usernamePlaceholder: {
		id: 'offline-account.username-placeholder',
		defaultMessage: 'The name to play as...',
	},
	invalid: {
		id: 'offline-account.invalid',
		defaultMessage: '3 to 16 letters, digits or underscores — what a server will accept.',
	},
	uuid: { id: 'offline-account.uuid', defaultMessage: 'Will play as {uuid}' },
	limitationHeader: {
		id: 'offline-account.limitation-header',
		defaultMessage: 'Singleplayer and offline servers only',
	},
	limitationBody: {
		id: 'offline-account.limitation-body',
		defaultMessage:
			'Servers in online mode ask Mojang whether the session is real, and there is none here. Everything else — worlds on this machine, LAN, and servers running in offline mode — works as it always did.',
	},
	add: { id: 'offline-account.add', defaultMessage: 'Add account' },
})

defineExpose({ show, hide })
</script>
