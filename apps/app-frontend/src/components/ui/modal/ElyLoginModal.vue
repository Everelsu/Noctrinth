<template>
	<NewModal ref="modal" hide-header no-padding width="440px">
		<div class="flex w-full flex-col">
			<div
				class="relative flex flex-col items-center gap-3 overflow-hidden px-6 pb-6 pt-10 text-center"
			>
				<div
					class="pointer-events-none absolute inset-x-0 top-0 h-32 bg-gradient-to-b from-brand-highlight to-transparent opacity-70"
				/>
				<button
					type="button"
					class="absolute right-4 top-4 z-[1] flex h-9 w-9 cursor-pointer items-center justify-center rounded-full border-0 bg-button-bg text-secondary transition-[filter,color] hover:text-contrast hover:brightness-125 disabled:cursor-not-allowed disabled:opacity-50"
					:aria-label="formatMessage(commonMessages.closeButton)"
					:disabled="loading"
					@click="hide"
				>
					<XIcon class="h-5 w-5" aria-hidden="true" />
				</button>
				<div
					class="relative flex h-16 w-16 items-center justify-center rounded-2xl bg-brand-highlight text-brand shadow-sm"
				>
					<UserIcon class="h-8 w-8" aria-hidden="true" />
				</div>
				<div class="relative flex flex-col gap-1">
					<h2 class="m-0 text-2xl font-extrabold text-contrast">
						{{ formatMessage(messages.header) }}
					</h2>
					<p class="m-0 text-sm leading-relaxed text-secondary">
						{{ formatMessage(messages.subtitle) }}
					</p>
				</div>
			</div>

			<div class="flex flex-col gap-4 px-6 pb-6">
				<div class="flex flex-col gap-1.5">
					<label for="ely-username" class="text-sm font-semibold text-contrast">
						{{ formatMessage(messages.usernameLabel) }}
					</label>
					<StyledInput
						id="ely-username"
						v-model="username"
						class="w-full"
						:icon="UserIcon"
						:placeholder="formatMessage(messages.usernamePlaceholder)"
						autocomplete="username"
						:disabled="loading"
						:error="!!errorMessage"
						@keyup.enter="submit"
					/>
				</div>

				<div class="flex flex-col gap-1.5">
					<label for="ely-password" class="text-sm font-semibold text-contrast">
						{{ formatMessage(messages.passwordLabel) }}
					</label>
					<StyledInput
						id="ely-password"
						v-model="password"
						class="w-full"
						:icon="LockIcon"
						:type="showPassword ? 'text' : 'password'"
						input-class="!pr-11"
						:placeholder="formatMessage(messages.passwordPlaceholder)"
						autocomplete="current-password"
						:disabled="loading"
						:error="!!errorMessage"
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

				<div v-if="needsTotp" class="flex flex-col gap-1.5">
					<label for="ely-totp" class="text-sm font-semibold text-contrast">
						{{ formatMessage(messages.totpLabel) }}
					</label>
					<StyledInput
						id="ely-totp"
						v-model="totp"
						class="w-full"
						:icon="ShieldIcon"
						inputmode="numeric"
						autocomplete="one-time-code"
						:placeholder="formatMessage(messages.totpPlaceholder)"
						:disabled="loading"
						:error="!!errorMessage"
						@keyup.enter="submit"
					/>
					<p class="m-0 text-xs text-secondary">
						{{ formatMessage(messages.totpHint) }}
					</p>
				</div>

				<Transition
					enter-active-class="transition-all duration-200 ease-out"
					enter-from-class="opacity-0 -translate-y-1"
					enter-to-class="opacity-100 translate-y-0"
				>
					<p
						v-if="errorMessage"
						class="m-0 flex items-start gap-2 rounded-xl bg-red-highlight p-3 text-sm text-red"
					>
						<TriangleAlertIcon class="mt-0.5 h-4 w-4 shrink-0" aria-hidden="true" />
						<span>{{ errorMessage }}</span>
					</p>
				</Transition>

				<div class="mt-1 flex flex-col gap-2">
					<ButtonStyled color="brand" size="large">
						<button
							class="!w-full"
							:disabled="loading || !username.trim() || !password || (needsTotp && !totp.trim())"
							@click="submit"
						>
							<SpinnerIcon v-if="loading" class="animate-spin" aria-hidden="true" />
							<LogInIcon v-else aria-hidden="true" />
							{{ formatMessage(loading ? messages.signingIn : messages.signIn) }}
						</button>
					</ButtonStyled>
					<ButtonStyled size="large">
						<button class="!w-full" :disabled="loading" @click="hide">
							<XIcon aria-hidden="true" />
							{{ formatMessage(messages.cancel) }}
						</button>
					</ButtonStyled>
				</div>

				<p
					class="m-0 border-0 border-t border-solid border-button-border pt-4 text-center text-sm text-secondary"
				>
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
			</div>
		</div>
	</NewModal>
</template>

<script setup lang="ts">
import {
	EyeIcon,
	EyeOffIcon,
	LockIcon,
	LogInIcon,
	ShieldIcon,
	SpinnerIcon,
	TriangleAlertIcon,
	UserIcon,
	XIcon,
} from '@modrinth/assets'
import {
	ButtonStyled,
	commonMessages,
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
	subtitle: {
		id: 'ely-login.subtitle',
		defaultMessage: 'Use your Ely.by credentials to add the account to Noctrinth.',
	},
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
