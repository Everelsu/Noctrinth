<template>
	<NewModal ref="modal" header="Sign in with Ely.by">
		<div class="min-w-md flex max-w-md flex-col gap-3">
			<div class="flex flex-col gap-2">
				<label for="ely-username">
					<span class="text-lg font-semibold text-contrast">Username or email</span>
				</label>
				<StyledInput
					id="ely-username"
					v-model="username"
					placeholder="Enter your Ely.by username or email..."
					autocomplete="username"
					:disabled="loading"
					@keyup.enter="submit"
				/>
			</div>
			<div class="flex flex-col gap-2">
				<label for="ely-password">
					<span class="text-lg font-semibold text-contrast">Password</span>
				</label>
				<div class="relative flex items-center">
					<StyledInput
						id="ely-password"
						v-model="password"
						:type="showPassword ? 'text' : 'password'"
						placeholder="Enter your password..."
						autocomplete="current-password"
						:disabled="loading"
						class="w-full pr-10"
						@keyup.enter="submit"
					/>
					<button
						type="button"
						class="absolute right-2 border-0 bg-transparent cursor-pointer text-secondary hover:text-contrast p-1"
						:aria-label="showPassword ? 'Hide password' : 'Show password'"
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
						Cancel
					</button>
				</ButtonStyled>
				<ButtonStyled color="brand">
					<button :disabled="loading || !username.trim() || !password" @click="submit">
						<SpinnerIcon v-if="loading" class="animate-spin" aria-hidden="true" />
						<LogInIcon v-else aria-hidden="true" />
						{{ loading ? 'Signing in...' : 'Sign in' }}
					</button>
				</ButtonStyled>
			</div>
			<p class="m-0 text-secondary text-sm text-center">
				Don't have an account?
				<a href="https://ely.by" target="_blank" rel="noopener noreferrer" class="text-brand">
					Sign up at ely.by
				</a>
			</p>
		</div>
	</NewModal>
</template>

<script setup lang="ts">
import { EyeIcon, EyeOffIcon, LogInIcon, SpinnerIcon, XIcon } from '@modrinth/assets'
import { ButtonStyled, NewModal, StyledInput } from '@modrinth/ui'
import { ref } from 'vue'

import { ely_login, type ElyCredentials } from '@/helpers/ely_auth'

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
		return "Couldn't reach Ely.by servers. Check your internet connection."
	}

	// Strip the raw backend prefix for a cleaner display.
	msg = msg.replace(/^Ely\.by login failed:\s*/i, '').trim()

	return msg || 'Login failed. Please check your credentials.'
}

defineExpose({ show, hide })
</script>
