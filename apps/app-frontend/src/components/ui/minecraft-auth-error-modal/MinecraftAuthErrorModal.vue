<script setup lang="ts">
import {
	CheckIcon,
	CopyIcon,
	DropdownIcon,
	LogInIcon,
	MessagesSquareIcon,
	WrenchIcon,
} from '@modrinth/assets'
import {
	Admonition,
	Button,
	ButtonLink,
	Collapsible,
	defineMessages,
	IconButton,
	IntlFormatted,
	NewModal,
	useVIntl,
} from '@modrinth/ui'
import { computed, ref } from 'vue'

import { handleSevereError } from '@/composables/use-error.js'
import { login as login_flow, set_default_user } from '@/helpers/auth.js'

import { findMinecraftAuthError, type MinecraftAuthError } from './minecraft-auth-errors'

const { formatMessage } = useVIntl()

const messages = defineMessages({
	header: { id: 'app.minecraft-auth-error.header', defaultMessage: 'Sign in Failed' },
	summary: {
		id: 'app.minecraft-auth-error.summary',
		defaultMessage:
			"We couldn't sign you into your Microsoft account. This may be due to account restrictions or regional limitations.",
	},
	whatHappened: {
		id: 'app.minecraft-auth-error.what-happened',
		defaultMessage: 'What we think happened',
	},
	howToFix: { id: 'app.minecraft-auth-error.how-to-fix', defaultMessage: 'How to fix it' },
	unknownTitle: { id: 'app.minecraft-auth-error.unknown', defaultMessage: 'Unknown error' },
	unknownBody: {
		id: 'app.minecraft-auth-error.unknown-body',
		defaultMessage:
			'We don’t recognize this error and can’t recommend specific steps to resolve it.',
	},
	unknownAdvice: {
		id: 'app.minecraft-auth-error.unknown-advice',
		defaultMessage:
			'Try visiting <login-link>Minecraft Login</login-link> and signing in, as it may prompt you with the necessary steps. You can also contact support and we can look into it further.',
	},
	contactSupport: {
		id: 'app.minecraft-auth-error.contact-support',
		defaultMessage: 'Contact support',
	},
	signInAgain: { id: 'app.minecraft-auth-error.sign-in-again', defaultMessage: 'Sign in again' },
	debugInformation: {
		id: 'app.minecraft-auth-error.debug-information',
		defaultMessage: 'Debug information',
	},
	copyDebugInfo: {
		id: 'app.minecraft-auth-error.copy-debug-info',
		defaultMessage: 'Copy debug info',
	},
	noErrorMessage: {
		id: 'app.error-modal.no-error-message',
		defaultMessage: 'No error message.',
	},
})

const modal = ref<InstanceType<typeof NewModal>>()
const rawError = ref<string>('')
const matchedError = ref<MinecraftAuthError | null>(null)
const debugCollapsed = ref(true)
const copied = ref(false)
const loadingSignIn = ref(false)

function show(errorVal: { message?: string }) {
	rawError.value = errorVal?.message ?? String(errorVal)

	matchedError.value = findMinecraftAuthError(rawError.value)

	debugCollapsed.value = true
	modal.value?.show()
}

function hide() {
	modal.value?.hide()
}

defineExpose({
	show,
	hide,
})

async function signInAgain() {
	try {
		loadingSignIn.value = true
		const loggedIn = await login_flow()
		if (loggedIn) {
			await set_default_user(loggedIn.profile.id)
		}
		loadingSignIn.value = false
		modal.value?.hide()
	} catch (err) {
		loadingSignIn.value = false
		handleSevereError(err)
	}
}

const debugInfo = computed(() => rawError.value || formatMessage(messages.noErrorMessage))

async function copyToClipboard(text: string) {
	await navigator.clipboard.writeText(text)
	copied.value = true
	setTimeout(() => {
		copied.value = false
	}, 3000)
}
</script>

<template>
	<NewModal ref="modal" :header="formatMessage(messages.header)" :max-width="'548px'">
		<div class="flex flex-col gap-6">
			<Admonition type="warning" :body="formatMessage(messages.summary)"> </Admonition>

			<!-- Matched error details -->
			<div class="bg-surface-2 rounded-2xl p-4 px-5 flex flex-col gap-3">
				<template v-if="matchedError">
					<div class="flex flex-col gap-1.5">
						<h3 class="text-base font-bold m-0">{{ formatMessage(messages.whatHappened) }}</h3>
						<p class="text-sm text-secondary m-0">
							{{ formatMessage(matchedError.whatHappened) }}
						</p>
					</div>

					<div class="flex flex-col gap-1.5">
						<h3 class="text-base font-bold m-0">{{ formatMessage(messages.howToFix) }}</h3>
						<ol class="list-none flex flex-col gap-2 m-0 pl-0">
							<li
								v-for="(step, index) in matchedError.stepsToFix"
								:key="index"
								class="flex items-baseline gap-2"
							>
								<span
									class="inline-flex items-center justify-center shrink-0 w-5 h-5 rounded-full bg-surface-4 border border-solid border-surface-5 text-xs font-medium"
								>
									{{ index + 1 }}
								</span>
								<span class="text-sm [&_a]:text-info [&_a]:font-medium [&_a]:underline">
									<IntlFormatted :message-id="step.message">
										<template #link="{ children }">
											<a :href="step.href" target="_blank" rel="noopener noreferrer">
												<component :is="() => children" />
											</a>
										</template>
									</IntlFormatted>
								</span>
							</li>
						</ol>
					</div>
				</template>
				<template v-else>
					<div class="flex flex-col gap-1.5">
						<h3 class="text-base font-bold m-0">{{ formatMessage(messages.unknownTitle) }}</h3>
						<p class="text-sm text-secondary m-0">
							{{ formatMessage(messages.unknownBody) }}
						</p>
						<p class="text-sm text-secondary m-0">
							<IntlFormatted :message-id="messages.unknownAdvice">
								<template #login-link="{ children }">
									<a
										class="text-info font-medium underline hover:underline"
										href="https://www.minecraft.net/en-us/login"
									>
										<component :is="() => children" />
									</a>
								</template>
							</IntlFormatted>
						</p>
					</div>
				</template>
			</div>

			<!-- Action buttons -->
			<div class="flex items-center gap-2">
				<ButtonLink href="https://support.modrinth.com" class="!w-full" @click="modal?.hide()">
					<MessagesSquareIcon /> {{ formatMessage(messages.contactSupport) }}
				</ButtonLink>
				<Button
					type="colored"
					color="brand"
					:disabled="loadingSignIn"
					class="!w-full"
					@click="signInAgain"
				>
					<LogInIcon /> {{ formatMessage(messages.signInAgain) }}
				</Button>
			</div>

			<div class="flex flex-col gap-2">
				<div class="w-full h-[1px] bg-surface-5"></div>

				<!-- Debug info -->
				<div class="overflow-clip">
					<button
						class="flex items-center justify-between w-full bg-transparent border-0 py-4 cursor-pointer"
						@click="debugCollapsed = !debugCollapsed"
					>
						<span class="flex items-center gap-2 text-contrast font-extrabold m-0">
							<WrenchIcon class="h-4 w-4" />
							{{ formatMessage(messages.debugInformation) }}
						</span>
						<DropdownIcon
							class="h-5 w-5 text-secondary transition-transform"
							:class="{ 'rotate-180': !debugCollapsed }"
						/>
					</button>
					<Collapsible :collapsed="debugCollapsed">
						<div
							class="p-3 bg-surface-2 rounded-2xl text-xs grid grid-cols-[1fr_auto] max-w-full items-start"
						>
							<div
								class="m-0 p-0 rounded-none bg-transparent text-sm font-mono break-words overflow-auto"
							>
								{{ debugInfo }}
							</div>
							<IconButton
								v-tooltip="formatMessage(messages.copyDebugInfo)"
								:label="formatMessage(messages.copyDebugInfo)"
								:disabled="copied"
								@click="copyToClipboard(debugInfo)"
							>
								<template v-if="copied"> <CheckIcon class="text-green" /> </template>
								<template v-else> <CopyIcon /> </template>
							</IconButton>
						</div>
					</Collapsible>
				</div>
			</div>
		</div>
	</NewModal>
</template>
