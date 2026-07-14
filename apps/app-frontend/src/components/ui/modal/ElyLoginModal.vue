<template>
	<NewModal ref="modal" :header="formatMessage(messages.header)">
		<div class="flex max-w-md flex-col gap-3">
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
				<StyledInput
					id="ely-password"
					v-model="password"
					:type="showPassword ? 'text' : 'password'"
					input-class="!pr-11"
					:placeholder="formatMessage(messages.passwordPlaceholder)"
					autocomplete="current-password"
					:disabled="loading"
					@keyup.enter="submit"
				>
					<template #right>
						<button
							type="button"
							class="absolute right-1.5 z-[1] flex h-8 w-8 cursor-pointer items-center justify-center rounded-lg border-0 bg-transparent text-secondary transition-colors hover:bg-button-bg hover:text-contrast"
							:aria-label="
								formatMessage(showPassword ? messages.hidePassword : messages.showPassword)
							"
							@click="showPassword = !showPassword"
						>
							<EyeOffIcon v-if="showPassword" class="h-5 w-5" />
							<EyeIcon v-else class="h-5 w-5" />
						</button>
					</template>
				</StyledInput>
			</div>

			<div v-if="needsTotp" class="flex flex-col gap-2">
				<label for="ely-totp" class="flex flex-col gap-1">
					<span class="text-lg font-semibold text-contrast">
						{{ formatMessage(messages.totpLabel) }}
					</span>
					<span>{{ formatMessage(messages.totpHint) }}</span>
				</label>
				<StyledInput
					id="ely-totp"
					v-model="totp"
					inputmode="numeric"
					autocomplete="one-time-code"
					:placeholder="formatMessage(messages.totpPlaceholder)"
					:disabled="loading"
					@keyup.enter="submit"
				/>
			</div>

			<Admonition v-if="errorMessage" type="critical">
				{{ errorMessage }}
			</Admonition>

			<p class="m-0 text-sm text-secondary">
				{{ formatMessage(messages.noAccount) }}
				<a
					href="https://ely.by"
					target="_blank"
					rel="noopener noreferrer"
					class="font-semibold text-brand hover:underline"
				>
					{{ formatMessage(messages.signUpLink) }}
				</a>
			</p>

			<div class="flex justify-end gap-2">
				<ButtonStyled type="outlined">
					<button :disabled="loading" @click="hide">
						<XIcon aria-hidden="true" />
						{{ formatMessage(messages.cancel) }}
					</button>
				</ButtonStyled>
				<ButtonStyled color="brand">
					<button
						:disabled="loading || !username.trim() || !password || (needsTotp && !totp.trim())"
						@click="submit"
					>
						<SpinnerIcon v-if="loading" class="animate-spin" aria-hidden="true" />
						<LogInIcon v-else aria-hidden="true" />
						{{ formatMessage(loading ? messages.signingIn : messages.signIn) }}
					</button>
				</ButtonStyled>
			</div>
		</div>
	</NewModal>
</template>

<script setup lang="ts">
import { EyeIcon, EyeOffIcon, LogInIcon, SpinnerIcon, XIcon } from '@modrinth/assets'
import {
	Admonition,
	ButtonStyled,
	defineMessages,
	NewModal,
	StyledInput,
	useVIntl,
} from '@modrinth/ui'
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
	totpLabel: { id: 'ely-login.totp-label', defaultMessage: 'Two-factor code' },
	totpPlaceholder: {
		id: 'ely-login.totp-placeholder',
		defaultMessage: 'Enter the 6-digit code...',
	},
	totpHint: {
		id: 'ely-login.totp-hint',
		defaultMessage: 'This account is protected with two-factor authentication.',
	},
})

const emit = defineEmits<{
	'logged-in': [ElyCredentials]
}>()

const modal = ref<InstanceType<typeof NewModal>>()
const username = ref('')
const password = ref('')
const totp = ref('')
const needsTotp = ref(false)
const showPassword = ref(false)
const loading = ref(false)
const errorMessage = ref('')

function show(event?: MouseEvent) {
	username.value = ''
	password.value = ''
	totp.value = ''
	needsTotp.value = false
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
	if (needsTotp.value && !totp.value.trim()) return
	loading.value = true
	errorMessage.value = ''
	try {
		// Ely.by delivers the TOTP code appended to the password ("password:code").
		const effectivePassword =
			needsTotp.value && totp.value.trim()
				? `${password.value}:${totp.value.trim()}`
				: password.value
		const creds = await ely_login(username.value.trim(), effectivePassword)
		emit('logged-in', creds)
		hide()
	} catch (e: unknown) {
		const raw = extractErrorMessage(e)
		// Ely.by rejects 2FA-protected accounts with this message until the
		// TOTP code is appended — reveal the code field instead of an error.
		if (/two.?factor/i.test(raw) && !needsTotp.value) {
			needsTotp.value = true
		} else {
			errorMessage.value = formatLoginError(raw)
		}
	} finally {
		loading.value = false
	}
}

function extractErrorMessage(e: unknown): string {
	return e && typeof e === 'object' && 'message' in e
		? String((e as { message: string }).message)
		: String(e)
}

function formatLoginError(msg: string): string {
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
