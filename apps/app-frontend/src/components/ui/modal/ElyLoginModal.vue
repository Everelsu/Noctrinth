<template>
	<NewModal ref="modal" :header="formatMessage(messages.header)">
		<div class="min-w-md flex max-w-md flex-col gap-3">
			<div class="flex flex-col gap-2">
				<label for="ely-username">
					<span class="text-lg font-semibold text-contrast">
						{{ formatMessage(messages.usernameLabel) }}
					</span>
				</label>
				<StyledInput
					id="ely-username"
					v-model="username"
					:placeholder="formatMessage(messages.usernamePlaceholder)"
					autocomplete="username"
					:disabled="loading"
					@keyup.enter="submit"
				/>
			</div>
			<div class="flex flex-col gap-2">
				<label for="ely-password">
					<span class="text-lg font-semibold text-contrast">
						{{ formatMessage(messages.passwordLabel) }}
					</span>
				</label>
				<div class="relative flex items-center">
					<StyledInput
						id="ely-password"
						v-model="password"
						:type="showPassword ? 'text' : 'password'"
						:placeholder="formatMessage(messages.passwordPlaceholder)"
						autocomplete="current-password"
						:disabled="loading"
						class="w-full pr-10"
						@keyup.enter="submit"
					/>
					<button
						type="button"
						class="absolute right-2 border-0 bg-transparent cursor-pointer text-secondary hover:text-contrast p-1"
						:aria-label="
							formatMessage(showPassword ? messages.hidePassword : messages.showPassword)
						"
						@click="showPassword = !showPassword"
					>
						<EyeOffIcon v-if="showPassword" class="w-4 h-4" />
						<EyeIcon v-else class="w-4 h-4" />
					</button>
				</div>
			</div>
			<p v-if="errorMessage" class="m-0 text-red text-sm">{{ errorMessage }}</p>
			<div class="flex justify-end gap-2">
				<ButtonStyled type="outlined">
					<button :disabled="loading" @click="hide">
						<XIcon aria-hidden="true" />
						{{ formatMessage(messages.cancel) }}
					</button>
				</ButtonStyled>
				<ButtonStyled color="brand">
					<button :disabled="loading || !username.trim() || !password" @click="submit">
						<SpinnerIcon v-if="loading" class="animate-spin" aria-hidden="true" />
						<LogInIcon v-else aria-hidden="true" />
						{{ formatMessage(loading ? messages.signingIn : messages.signIn) }}
					</button>
				</ButtonStyled>
			</div>
			<p class="m-0 text-secondary text-sm text-center">
				{{ formatMessage(messages.noAccount) }}
				<a href="https://ely.by" target="_blank" rel="noopener noreferrer" class="text-brand">
					{{ formatMessage(messages.signUpLink) }}
				</a>
			</p>
		</div>
	</NewModal>
</template>

<script setup lang="ts">
import { EyeIcon, EyeOffIcon, LogInIcon, SpinnerIcon, XIcon } from '@modrinth/assets'
import { ButtonStyled, defineMessages, NewModal, StyledInput, useVIntl } from '@modrinth/ui'
import { ref } from 'vue'

import { ely_login, type ElyCredentials } from '@/helpers/ely_auth'

const { formatMessage } = useVIntl()

const messages = defineMessages({
	header: { id: 'ely-login.header', defaultMessage: 'Sign in with Ely.by' },
	usernameLabel: { id: 'ely-login.username-label', defaultMessage: 'Username or email' },
	usernamePlaceholder: {
		id: 'ely-login.username-placeholder',
		defaultMessage: 'Enter your Ely.by username or email...',
	},
	passwordLabel: { id: 'ely-login.password-label', defaultMessage: 'Password' },
	passwordPlaceholder: {
		id: 'ely-login.password-placeholder',
		defaultMessage: 'Enter your password...',
	},
	showPassword: { id: 'ely-login.show-password', defaultMessage: 'Show password' },
	hidePassword: { id: 'ely-login.hide-password', defaultMessage: 'Hide password' },
	cancel: { id: 'ely-login.cancel', defaultMessage: 'Cancel' },
	signIn: { id: 'ely-login.sign-in', defaultMessage: 'Sign in' },
	signingIn: { id: 'ely-login.signing-in', defaultMessage: 'Signing in...' },
	noAccount: { id: 'ely-login.no-account', defaultMessage: "Don't have an account?" },
	signUpLink: { id: 'ely-login.sign-up-link', defaultMessage: 'Sign up at ely.by' },
	loginFailedNetwork: {
		id: 'ely-login.error.network',
		defaultMessage: "Couldn't reach Ely.by servers. Check your internet connection.",
	},
	loginFailedGeneric: {
		id: 'ely-login.error.generic',
		defaultMessage: 'Login failed. Please check your credentials.',
	},
})

const emit = defineEmits<{
	'logged-in': [ElyCredentials]
}>()

const modal = ref<InstanceType<typeof NewModal>>()
const username = ref('')
const password = ref('')
const showPassword = ref(false)
const loading = ref(false)
const errorMessage = ref('')

function show(event?: MouseEvent) {
	username.value = ''
	password.value = ''
	showPassword.value = false
	loading.value = false
	errorMessage.value = ''
	modal.value?.show(event)
}

function hide() {
	modal.value?.hide()
}

async function submit() {
	if (!username.value.trim() || !password.value) return
	loading.value = true
	errorMessage.value = ''
	try {
		const creds = await ely_login(username.value.trim(), password.value)
		emit('logged-in', creds)
		hide()
	} catch (e: unknown) {
		errorMessage.value = formatLoginError(e)
	} finally {
		loading.value = false
	}
}

function formatLoginError(e: unknown): string {
	let msg =
		e && typeof e === 'object' && 'message' in e
			? String((e as { message: string }).message)
			: String(e)

	if (/request failed|error sending request/i.test(msg)) {
		return formatMessage(messages.loginFailedNetwork)
	}

	// Strip the raw backend prefix for a cleaner display.
	msg = msg.replace(/^Ely\.by login failed:\s*/i, '').trim()

	return msg || formatMessage(messages.loginFailedGeneric)
}

defineExpose({ show, hide })
</script>

<style scoped>
/* WebView2 renders its own native password reveal/clear controls for
   <input type="password">. Hide them so only the custom show/hide button
   from this modal is shown. */
:deep(input::-ms-reveal),
:deep(input::-ms-clear) {
	display: none;
}
</style>
