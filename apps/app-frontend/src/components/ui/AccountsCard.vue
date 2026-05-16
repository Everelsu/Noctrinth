<template>
	<div
		v-if="allAccounts.length === 0"
		class="flex flex-col gap-3 bg-button-bg border border-solid border-surface-5 rounded-xl p-3 mt-2"
	>
		<span>{{ formatMessage(messages.notSignedIn) }}</span>
		<ButtonStyled color="brand">
			<button color="primary" :disabled="loginDisabled" @click="login()">
				<LogInIcon v-if="!loginDisabled" />
				<SpinnerIcon v-else class="animate-spin" />
				{{ formatMessage(messages.signInToMinecraft) }}
			</button>
		</ButtonStyled>
		<ButtonStyled>
			<button @click="elyLoginModal?.show()">
				<LogInIcon />
				{{ formatMessage(messages.addElyAccount) }}
			</button>
		</ButtonStyled>
	</div>
	<Accordion
		v-else
		class="w-full mt-2 bg-button-bg border border-solid border-surface-5 rounded-xl overflow-clip"
		button-class="button-base w-full bg-transparent px-3 py-2 border-0 cursor-pointer"
		:open-by-default="false"
	>
		<template #title>
			<div class="flex gap-2 w-full min-w-0">
				<Avatar
					size="36px"
					:src="
						selectedAccount
							? avatarUrl
							: 'https://launcher-files.modrinth.com/assets/steve_head.png'
					"
				/>
				<div class="flex flex-col items-start w-full min-w-0">
					<span class="truncate text-left w-full">{{
						selectedAccount ? selectedAccount.profile.name : formatMessage(messages.selectAccount)
					}}</span>
					<span class="text-secondary text-xs">{{ selectedAccountTypeLabel }}</span>
				</div>
			</div>
		</template>
		<div class="bg-button-bg pt-1 pb-2 border border-solid border-surface-5">
			<template v-if="allAccounts.length > 0">
				<div v-for="account in allAccounts" :key="account.profile.id" class="flex gap-1 items-center">
					<button
						class="flex items-center flex-shrink flex-grow overflow-clip gap-2 p-2 border-0 bg-transparent cursor-pointer button-base min-w-0"
						@click="setAccount(account)"
					>
						<RadioButtonCheckedIcon
							v-if="isAccountSelected(account)"
							class="w-5 h-5 text-brand shrink-0"
						/>
						<RadioButtonIcon v-else class="w-5 h-5 text-secondary shrink-0" />
						<Avatar :src="getAccountAvatarUrl(account)" size="24px" />
						<p
							class="m-0 truncate min-w-0"
							:class="
								isAccountSelected(account) ? 'text-contrast font-semibold' : 'text-primary'
							"
						>
							{{ account.profile.name }}
						</p>
						<span
							class="ml-1 shrink-0 rounded px-1.5 py-0.5 text-xs font-medium"
							:class="
								isElyAccount(account)
									? 'bg-brand-highlight text-brand'
									: 'bg-surface-3 text-secondary'
							"
						>
							{{ accountProviderLabel(account) }}
						</span>
					</button>
					<ButtonStyled circular color="red" color-fill="none" hover-color-fill="background">
						<button
							v-tooltip="formatMessage(messages.removeAccount)"
							class="mr-2"
							@click="logoutAccount(account)"
						>
							<TrashIcon />
						</button>
					</ButtonStyled>
				</div>
			</template>
			<div class="flex flex-col gap-2 px-2 pt-2">
				<ButtonStyled v-if="allAccounts.length > 0" class="w-full">
					<button :disabled="loginDisabled" @click="login()">
						<PlusIcon />
						{{ formatMessage(messages.addAccount) }}
					</button>
				</ButtonStyled>
				<ButtonStyled v-if="allAccounts.length > 0" class="w-full">
					<button @click="elyLoginModal?.show()">
						<PlusIcon />
						{{ formatMessage(messages.addElyAccount) }}
					</button>
				</ButtonStyled>
			</div>
		</div>
	</Accordion>
	<ElyLoginModal ref="elyLoginModal" @logged-in="refreshValues" />
</template>

<script setup lang="ts">
import {
	LogInIcon,
	PlusIcon,
	RadioButtonCheckedIcon,
	RadioButtonIcon,
	SpinnerIcon,
	TrashIcon,
} from '@modrinth/assets'
import {
	Accordion,
	Avatar,
	ButtonStyled,
	defineMessages,
	injectNotificationManager,
	useVIntl,
} from '@modrinth/ui'
import type { Ref } from 'vue'
import { computed, onUnmounted, ref } from 'vue'

import { trackEvent } from '@/helpers/analytics'
import {
	get_default_user,
	login as login_flow,
	remove_user,
	set_default_user,
	users,
} from '@/helpers/auth'
import {
	ely_get_default_user,
	ely_get_users,
	ely_logout,
	ely_set_default_user,
	type ElyCredentials,
} from '@/helpers/ely_auth'
import { getElyHeadUrl } from '@/helpers/ely_skins'
import { process_listener } from '@/helpers/events'
import { getPlayerHeadUrl } from '@/helpers/rendering/batch-skin-renderer.ts'
import type { Skin } from '@/helpers/skins'
import { get_available_skins } from '@/helpers/skins'
import { handleSevereError } from '@/store/error.js'
import ElyLoginModal from '@/components/ui/modal/ElyLoginModal.vue'

const { formatMessage } = useVIntl()
const { handleError } = injectNotificationManager()

const emit = defineEmits<{
	change: []
}>()

type MinecraftCredential = {
	profile: {
		id: string
		name: string
	}
}

type AnyCredential = MinecraftCredential | ElyCredentials

function isElyAccount(a: AnyCredential): a is ElyCredentials {
	return 'auth_provider' in a && a.auth_provider === 'ely_by'
}

const accounts: Ref<MinecraftCredential[]> = ref([])
const elyAccounts: Ref<ElyCredentials[]> = ref([])
const loginDisabled = ref(false)
const defaultUser = ref<string | undefined>()
const elyDefaultUser = ref<string | undefined>()
const selectedProvider = ref<'microsoft' | 'ely_by'>('microsoft')
const equippedSkin = ref<Skin | null>(null)
const headUrlCache = ref(new Map<string, string>())
const elyHeadCache = ref(new Map<string, string>())
const elyLoginModal = ref<InstanceType<typeof ElyLoginModal>>()

const allAccounts = computed<AnyCredential[]>(() => {
	const combined: AnyCredential[] = [...accounts.value, ...elyAccounts.value]
	combined.sort((a, b) => (a.profile?.name ?? '').localeCompare(b.profile?.name ?? ''))
	return combined
})

/**
 * Loads Ely.by accounts. This provider is fully optional and additive: any
 * failure here is swallowed (logged to console only) so it can never surface
 * error popups to, block, or otherwise disrupt users of Microsoft accounts.
 */
async function loadElyAccounts() {
	try {
		elyDefaultUser.value = (await ely_get_default_user()) ?? undefined
		const elyUserList = await ely_get_users()
		elyAccounts.value = Array.isArray(elyUserList) ? [...elyUserList] : []
	} catch (error) {
		console.warn('Failed to load Ely.by accounts:', error)
		elyAccounts.value = []
		elyDefaultUser.value = undefined
	}

	// Render Ely.by heads in the background — never await, so account display
	// is never delayed by skin-texture network requests.
	for (const account of elyAccounts.value) {
		getElyHeadUrl(account.profile.name)
			.then((headUrl) => elyHeadCache.value.set(account.profile.id, headUrl))
			.catch((error) =>
				console.warn('Failed to render Ely.by head for', account.profile.name, error),
			)
	}
}

async function refreshValues() {
	// Microsoft accounts — the primary provider. Behavior left unchanged.
	defaultUser.value = (await get_default_user().catch(handleError)) ?? undefined
	const userList = await users().catch(handleError)
	accounts.value = Array.isArray(userList) ? [...userList] : []

	// Ely.by accounts — isolated, optional, never blocks the above.
	await loadElyAccounts()

	// Pick a provider only when the current choice has no valid default account
	// (e.g. initial load or after the selected account was removed). An explicit
	// selection made via setAccount() is preserved as long as it still resolves.
	const currentProviderValid =
		(selectedProvider.value === 'microsoft' && defaultUser.value !== undefined) ||
		(selectedProvider.value === 'ely_by' && elyDefaultUser.value !== undefined)
	if (!currentProviderValid) {
		if (defaultUser.value) {
			selectedProvider.value = 'microsoft'
		} else if (elyDefaultUser.value) {
			selectedProvider.value = 'ely_by'
		}
	}

	// The Mojang skin system only works for the active Microsoft account.
	// Skip the call entirely when a Microsoft account is not the active one
	// (e.g. an Ely.by account is selected) to avoid spurious backend errors.
	if (!defaultUser.value) {
		equippedSkin.value = null
	} else {
		try {
			const skins = await get_available_skins()
			equippedSkin.value = skins.find((skin) => skin.is_equipped) ?? null

			if (equippedSkin.value) {
				try {
					const headUrl = await getPlayerHeadUrl(equippedSkin.value)
					headUrlCache.value.set(equippedSkin.value.texture_key, headUrl)
				} catch (error) {
					console.warn('Failed to get head render for equipped skin:', error)
				}
			}
		} catch {
			equippedSkin.value = null
		}
	}
}

function setLoginDisabled(value: boolean) {
	loginDisabled.value = value
}

const selectedAccount = computed<AnyCredential | undefined>(() => {
	if (selectedProvider.value === 'ely_by') {
		return elyAccounts.value.find((account) => account.profile.id === elyDefaultUser.value)
	}
	return accounts.value.find((account) => account.profile.id === defaultUser.value)
})

defineExpose({
	refreshValues,
	setLoginDisabled,
	loginDisabled,
	selectedAccount,
})

await refreshValues()

const STEVE_HEAD_URL = 'https://launcher-files.modrinth.com/assets/steve_head.png'

/**
 * Head-render URL for a Microsoft account. mc-heads.net expects an
 * undashed UUID (or a username) — passing a dashed UUID can silently fail,
 * so dashes are stripped here.
 */
function microsoftHeadUrl(id: string): string {
	return `https://mc-heads.net/avatar/${id.replace(/-/g, '')}/128`
}

const avatarUrl = computed(() => {
	if (selectedAccount.value && isElyAccount(selectedAccount.value)) {
		return elyHeadCache.value.get(selectedAccount.value.profile.id) ?? STEVE_HEAD_URL
	}
	if (equippedSkin.value?.texture_key) {
		const cachedUrl = headUrlCache.value.get(equippedSkin.value.texture_key)
		if (cachedUrl) {
			return cachedUrl
		}
		return `https://mc-heads.net/avatar/${equippedSkin.value.texture_key}/128`
	}
	if (selectedAccount.value?.profile?.id) {
		return microsoftHeadUrl(selectedAccount.value.profile.id)
	}
	return STEVE_HEAD_URL
})

function getAccountAvatarUrl(account: AnyCredential) {
	if (isElyAccount(account)) {
		return elyHeadCache.value.get(account.profile.id) ?? STEVE_HEAD_URL
	}
	if (
		account.profile.id === selectedAccount.value?.profile?.id &&
		equippedSkin.value?.texture_key
	) {
		const cachedUrl = headUrlCache.value.get(equippedSkin.value.texture_key)
		if (cachedUrl) {
			return cachedUrl
		}
	}
	return microsoftHeadUrl(account.profile.id)
}

/** Short provider label shown on the account badge. */
function accountProviderLabel(account: AnyCredential): string {
	return isElyAccount(account) ? 'Ely.by' : 'Microsoft'
}

/** Subtitle under the selected account name in the accordion header. */
const selectedAccountTypeLabel = computed(() => {
	if (!selectedAccount.value) {
		return formatMessage(messages.minecraftAccount)
	}
	return isElyAccount(selectedAccount.value)
		? formatMessage(messages.elyByAccount)
		: formatMessage(messages.microsoftAccount)
})

function isAccountSelected(account: AnyCredential): boolean {
	const selected = selectedAccount.value
	if (!selected) return false
	return (
		selected.profile.id === account.profile.id &&
		isElyAccount(selected) === isElyAccount(account)
	)
}

async function setAccount(account: AnyCredential) {
	if (isElyAccount(account)) {
		await ely_set_default_user(account.profile.id).catch(handleError)
		elyDefaultUser.value = account.profile.id
		selectedProvider.value = 'ely_by'
	} else {
		await set_default_user(account.profile.id).catch(handleError)
		defaultUser.value = account.profile.id
		selectedProvider.value = 'microsoft'
	}
	await refreshValues()
	emit('change')
}

async function login() {
	loginDisabled.value = true
	const loggedIn = await login_flow().catch(handleSevereError)

	if (loggedIn) {
		await setAccount(loggedIn)
	}

	trackEvent('AccountLogIn')
	loginDisabled.value = false
}

async function logoutAccount(account: AnyCredential) {
	const wasSelected =
		selectedAccount.value !== undefined &&
		selectedAccount.value.profile.id === account.profile.id &&
		isElyAccount(selectedAccount.value) === isElyAccount(account)

	if (isElyAccount(account)) {
		await ely_logout(account.profile.id).catch(handleError)
	} else {
		await remove_user(account.profile.id).catch(handleError)
	}
	await refreshValues()
	if (wasSelected && !selectedAccount.value && allAccounts.value.length > 0) {
		await setAccount(allAccounts.value[0])
	} else {
		emit('change')
	}
	trackEvent('AccountLogOut')
}

const unlisten = await process_listener(async (e) => {
	if (e.event === 'launched') {
		await refreshValues()
	}
})

onUnmounted(() => {
	unlisten()
})

const messages = defineMessages({
	notSignedIn: {
		id: 'minecraft-account.not-signed-in',
		defaultMessage: 'Not signed in',
	},
	addAccount: {
		id: 'minecraft-account.add-account',
		defaultMessage: 'Add account',
	},
	addElyAccount: {
		id: 'minecraft-account.add-ely-account',
		defaultMessage: 'Add Ely.by account',
	},
	removeAccount: {
		id: 'minecraft-account.remove-account',
		defaultMessage: 'Remove account',
	},
	selectAccount: {
		id: 'minecraft-account.select-account',
		defaultMessage: 'Select account',
	},
	minecraftAccount: {
		id: 'minecraft-account.label',
		defaultMessage: 'Minecraft account',
	},
	microsoftAccount: {
		id: 'minecraft-account.label-microsoft',
		defaultMessage: 'Microsoft account',
	},
	elyByAccount: {
		id: 'minecraft-account.label-ely-by',
		defaultMessage: 'Ely.by account',
	},
	signInToMinecraft: {
		id: 'minecraft-account.sign-in',
		defaultMessage: 'Sign in to Minecraft',
	},
})
</script>
