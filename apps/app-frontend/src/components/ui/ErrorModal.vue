<script setup>
import {
	CheckIcon,
	CopyIcon,
	DropdownIcon,
	FolderOpenIcon,
	HammerIcon,
	LogInIcon,
	UpdatedIcon,
	WrenchIcon,
	XIcon,
} from '@modrinth/assets'
import {
	Button,
	ButtonLink,
	Collapsible,
	commonMessages,
	defineMessages,
	IconButton,
	injectNotificationManager,
	IntlFormatted,
	useVIntl,
} from '@modrinth/ui'
import { computed, ref } from 'vue'

import { ChatIcon } from '@/assets/icons'
import ModalWrapper from '@/components/ui/modal/ModalWrapper.vue'
import { handleSevereError } from '@/composables/use-error.js'
import { trackEvent } from '@/helpers/analytics'
import { login as login_flow, set_default_user } from '@/helpers/auth.js'
import { install_existing_instance } from '@/helpers/install'
import { cancel_directory_change } from '@/helpers/settings.ts'
import { showAppDbBackupsFolder } from '@/helpers/utils.js'

const { handleError } = injectNotificationManager()
const { formatMessage } = useVIntl()

const messages = defineMessages({
	titleMinecraftAuth: {
		id: 'app.error-modal.title.minecraft-auth',
		defaultMessage: 'Unable to sign in to Minecraft',
	},
	titleDirectoryMove: {
		id: 'app.error-modal.title.directory-move',
		defaultMessage: 'Could not change app directory',
	},
	titleNoLoader: {
		id: 'app.error-modal.title.no-loader',
		defaultMessage: 'No loader selected',
	},
	titleStateInit: {
		id: 'app.error-modal.title.state-init',
		defaultMessage: 'Error initializing Noctrinth',
	},
	titleUnknown: { id: 'app.error-modal.title.unknown', defaultMessage: 'An error occurred' },
	networkHeading: { id: 'app.error-modal.network.heading', defaultMessage: 'Network issues' },
	networkBody: {
		id: 'app.error-modal.network.body',
		defaultMessage:
			"It looks like there were issues with Noctrinth connecting to Microsoft's servers. This is often the result of a poor connection, so we recommend trying again to see if it works. If issues continue to persist, follow the steps in <support-article>our support article</support-article> to troubleshoot.",
	},
	hostsFileBody: {
		id: 'app.error-modal.hosts-file.body',
		defaultMessage:
			'Noctrinth tried to connect to Microsoft / Xbox / Minecraft services, but the remote server rejected the connection. This may indicate that these services are blocked by the hosts file. Please visit <support-article>our support article</support-article> for steps on how to fix the issue.',
	},
	otherAccountHeading: {
		id: 'app.error-modal.other-account.heading',
		defaultMessage: 'Try another Microsoft account',
	},
	otherAccountBody: {
		id: 'app.error-modal.other-account.body',
		defaultMessage:
			'Double check you’ve signed in with the right account. You may own Minecraft on a different Microsoft account.',
	},
	tryAnotherAccount: {
		id: 'app.error-modal.other-account.button',
		defaultMessage: 'Try another account',
	},
	gamePassHeading: {
		id: 'app.error-modal.game-pass.heading',
		defaultMessage: 'Using PC Game Pass, coming from Bedrock, or just bought the game?',
	},
	gamePassBody: {
		id: 'app.error-modal.game-pass.body',
		defaultMessage:
			'Try signing in with the <launcher-link>official Minecraft Launcher</launcher-link> first. Once you’re done, come back here and sign in!',
	},
	trySigningInAgain: {
		id: 'app.error-modal.try-signing-in-again',
		defaultMessage: 'Try signing in again',
	},
	readOnlyHeading: {
		id: 'app.error-modal.directory.read-only.heading',
		defaultMessage: 'Change directory permissions',
	},
	readOnlyBody: {
		id: 'app.error-modal.directory.read-only.body',
		defaultMessage:
			'It looks like Noctrinth is unable to write to the directory you selected. Please adjust the permissions of the directory and try again or cancel the directory change.',
	},
	noSpaceHeading: {
		id: 'app.error-modal.directory.no-space.heading',
		defaultMessage: 'Not enough space',
	},
	noSpaceBody: {
		id: 'app.error-modal.directory.no-space.body',
		defaultMessage:
			'It looks like there is not enough space on the disk containing the directory you selected. Please free up some space and try again or cancel the directory change.',
	},
	directoryGenericBody: {
		id: 'app.error-modal.directory.generic.body',
		defaultMessage:
			'Noctrinth is unable to migrate to the new directory you selected. Please contact support for help or cancel the directory change.',
	},
	retryDirectoryChange: {
		id: 'app.error-modal.directory.retry',
		defaultMessage: 'Retry directory change',
	},
	cancelDirectoryChange: {
		id: 'app.error-modal.directory.cancel',
		defaultMessage: 'Cancel directory change',
	},
	stateInitBody: {
		id: 'app.error-modal.state-init.body',
		defaultMessage:
			'Noctrinth failed to load correctly. This may be because of a corrupted file, or because the app is missing crucial files.',
	},
	stateInitFixes: {
		id: 'app.error-modal.state-init.fixes',
		defaultMessage: 'You may be able to fix it through one of the following ways:',
	},
	stateInitFixInternet: {
		id: 'app.error-modal.state-init.fix-internet',
		defaultMessage: 'Ensuring you are connected to the internet, then try restarting the app.',
	},
	stateInitFixRedownload: {
		id: 'app.error-modal.state-init.fix-redownload',
		defaultMessage: 'Redownloading the app.',
	},
	noLoaderBody: {
		id: 'app.error-modal.no-loader.body',
		defaultMessage: 'Noctrinth failed to find the loader version for this instance.',
	},
	noLoaderFix: {
		id: 'app.error-modal.no-loader.fix',
		defaultMessage:
			'To resolve this, you need to repair the instance. Click the button below to do so.',
	},
	repairInstance: {
		id: 'app.error-modal.no-loader.repair',
		defaultMessage: 'Repair instance',
	},
	supportPrompt: {
		id: 'app.error-modal.support-prompt',
		defaultMessage:
			'If nothing is working and you need help, visit <support-page>our support page</support-page> and start a chat using the widget in the bottom right and we will be more than happy to assist! Make sure to provide the following debug information to the agent:',
	},
	getSupport: { id: 'app.error-modal.get-support', defaultMessage: 'Get support' },
	debugInformation: {
		id: 'app.error-modal.debug-information',
		defaultMessage: 'Debug information',
	},
	copyDebugInfo: { id: 'app.error-modal.copy-debug-info', defaultMessage: 'Copy debug info' },
	noErrorMessage: {
		id: 'app.error-modal.no-error-message',
		defaultMessage: 'No error message.',
	},
	openBackupsFolder: {
		id: 'app.error.state-init.open-backups-folder',
		defaultMessage: 'Open backups folder',
	},
})

const errorModal = ref()
const error = ref()
const closable = ref(true)
const errorCollapsed = ref(false)

const titleMessage = ref(messages.titleUnknown)
const title = computed(() => formatMessage(titleMessage.value))
const errorType = ref('unknown')
const supportLink = ref('')
const metadata = ref({})

defineExpose({
	async show(errorVal, context, canClose = true, source = null) {
		console.log(errorVal, context, canClose, source)
		closable.value = canClose

		if (errorVal.message && errorVal.message.includes('Minecraft authentication error:')) {
			titleMessage.value = messages.titleMinecraftAuth
			errorType.value = 'minecraft_auth'
			supportLink.value = ''

			if (
				errorVal.message.includes('existing connection was forcibly closed') ||
				errorVal.message.includes('error sending request for url')
			) {
				metadata.value.network = true
			}
			if (errorVal.message.includes('because the target machine actively refused it')) {
				metadata.value.hostsFile = true
			}
		} else if (errorVal.message && errorVal.message.includes('Move directory error:')) {
			titleMessage.value = messages.titleDirectoryMove
			errorType.value = 'directory_move'
			supportLink.value = ''

			if (errorVal.message.includes('directory is not writable')) {
				metadata.value.readOnly = true
			}

			if (errorVal.message.includes('Not enough space')) {
				metadata.value.notEnoughSpace = true
			}
		} else if (errorVal.message && errorVal.message.includes('No loader version selected for')) {
			titleMessage.value = messages.titleNoLoader
			errorType.value = 'no_loader_version'
			supportLink.value = ''
			metadata.value.instanceId = context.instanceId
		} else if (source === 'state_init') {
			titleMessage.value = messages.titleStateInit
			errorType.value = 'state_init'
			supportLink.value = ''
		} else {
			titleMessage.value = messages.titleUnknown
			errorType.value = 'unknown'
			supportLink.value = ''
			metadata.value = {}
		}

		error.value = errorVal
		errorModal.value.show()
	},
})

const loadingMinecraft = ref(false)
async function loginMinecraft() {
	try {
		loadingMinecraft.value = true
		const loggedIn = await login_flow()

		if (loggedIn) {
			await set_default_user(loggedIn.profile.id).catch(handleError)
		}

		await trackEvent('AccountLogIn', { source: 'ErrorModal' })
		loadingMinecraft.value = false
		errorModal.value.hide()
	} catch (err) {
		loadingMinecraft.value = false
		handleSevereError(err)
	}
}

async function cancelDirectoryChange() {
	try {
		await cancel_directory_change()
		window.location.reload()
	} catch (err) {
		handleError(err)
	}
}

function retryDirectoryChange() {
	window.location.reload()
}

async function openDbBackupsFolder() {
	await showAppDbBackupsFolder().catch(handleError)
}

const loadingRepair = ref(false)
async function repairInstance() {
	loadingRepair.value = true
	try {
		await install_existing_instance(metadata.value.instanceId, false)
		errorModal.value.hide()
	} catch (err) {
		handleSevereError(err)
	}
	loadingRepair.value = false
}

const hasDebugInfo = computed(
	() =>
		errorType.value === 'directory_move' ||
		errorType.value === 'minecraft_auth' ||
		errorType.value === 'state_init' ||
		errorType.value === 'no_loader_version',
)

const debugInfo = computed(
	() => error.value.message ?? error.value ?? formatMessage(messages.noErrorMessage),
)

const copied = ref(false)

async function copyToClipboard(text) {
	await navigator.clipboard.writeText(text)
	copied.value = true
	setTimeout(() => {
		copied.value = false
	}, 3000)
}
</script>

<template>
	<ModalWrapper ref="errorModal" :header="title" :closable="closable">
		<div class="modal-body max-w-[550px]">
			<div class="markdown-body">
				<template v-if="errorType === 'minecraft_auth'">
					<template v-if="metadata.network">
						<h3>{{ formatMessage(messages.networkHeading) }}</h3>
						<p>
							<IntlFormatted :message-id="messages.networkBody">
								<template #support-article="{ children }">
									<a
										href="https://support.modrinth.com/en/articles/9038231-minecraft-sign-in-issues#h_e71a5f805f"
									>
										<component :is="() => children" />
									</a>
								</template>
							</IntlFormatted>
						</p>
					</template>
					<template v-else-if="metadata.hostsFile">
						<h3>{{ formatMessage(messages.networkHeading) }}</h3>
						<p>
							<IntlFormatted :message-id="messages.hostsFileBody">
								<template #support-article="{ children }">
									<a
										href="https://support.modrinth.com/en/articles/9038231-minecraft-sign-in-issues#h_d694a29256"
									>
										<component :is="() => children" />
									</a>
								</template>
							</IntlFormatted>
						</p>
					</template>
					<template v-else>
						<h3>{{ formatMessage(messages.otherAccountHeading) }}</h3>
						<p>{{ formatMessage(messages.otherAccountBody) }}</p>
						<div class="cta-button">
							<button class="btn btn-primary" :disabled="loadingMinecraft" @click="loginMinecraft">
								<LogInIcon /> {{ formatMessage(messages.tryAnotherAccount) }}
							</button>
						</div>
						<h3>{{ formatMessage(messages.gamePassHeading) }}</h3>
						<p>
							<IntlFormatted :message-id="messages.gamePassBody">
								<template #launcher-link="{ children }">
									<a href="https://www.minecraft.net/en-us/download">
										<component :is="() => children" />
									</a>
								</template>
							</IntlFormatted>
						</p>
					</template>
					<div class="cta-button">
						<button class="btn btn-primary" :disabled="loadingMinecraft" @click="loginMinecraft">
							<LogInIcon /> {{ formatMessage(messages.trySigningInAgain) }}
						</button>
					</div>
				</template>
				<template v-if="errorType === 'directory_move'">
					<template v-if="metadata.readOnly">
						<h3>{{ formatMessage(messages.readOnlyHeading) }}</h3>
						<p>{{ formatMessage(messages.readOnlyBody) }}</p>
					</template>
					<template v-else-if="metadata.notEnoughSpace">
						<h3>{{ formatMessage(messages.noSpaceHeading) }}</h3>
						<p>{{ formatMessage(messages.noSpaceBody) }}</p>
					</template>
					<template v-else>
						<p>{{ formatMessage(messages.directoryGenericBody) }}</p>
					</template>

					<div class="cta-button">
						<button class="btn" @click="retryDirectoryChange">
							<UpdatedIcon /> {{ formatMessage(messages.retryDirectoryChange) }}
						</button>
						<button class="btn btn-danger" @click="cancelDirectoryChange">
							<XIcon /> {{ formatMessage(messages.cancelDirectoryChange) }}
						</button>
					</div>
				</template>
				<template v-else-if="errorType === 'state_init'">
					<p>{{ formatMessage(messages.stateInitBody) }}</p>
					<p>{{ formatMessage(messages.stateInitFixes) }}</p>
					<ul>
						<li>{{ formatMessage(messages.stateInitFixInternet) }}</li>
						<li>{{ formatMessage(messages.stateInitFixRedownload) }}</li>
					</ul>
				</template>
				<template v-else-if="errorType === 'no_loader_version'">
					<p>{{ formatMessage(messages.noLoaderBody) }}</p>
					<p>{{ formatMessage(messages.noLoaderFix) }}</p>
					<div class="cta-button">
						<button class="btn btn-primary" :disabled="loadingRepair" @click="repairInstance">
							<HammerIcon /> {{ formatMessage(messages.repairInstance) }}
						</button>
					</div>
				</template>
				<template v-else>
					{{ debugInfo }}
				</template>
				<template v-if="hasDebugInfo">
					<div class="w-full h-[1px] bg-surface-5 mb-3"></div>
					<p>
						<IntlFormatted :message-id="messages.supportPrompt">
							<template #support-page="{ children }">
								<a :href="supportLink">
									<component :is="() => children" />
								</a>
							</template>
						</IntlFormatted>
					</p>
				</template>
			</div>
			<div class="flex items-center gap-2">
				<ButtonLink :href="supportLink" @click="errorModal.hide()"
					><ChatIcon /> {{ formatMessage(messages.getSupport) }}</ButtonLink
				>
				<Button v-if="closable" @click="errorModal.hide()"
					><XIcon /> {{ formatMessage(commonMessages.closeButton) }}</Button
				>
			</div>
			<template v-if="hasDebugInfo">
				<div class="flex flex-col gap-2">
					<div class="w-full h-[1px] bg-surface-5"></div>

					<div class="overflow-clip">
						<button
							class="flex items-center justify-between w-full bg-transparent border-0 py-4 cursor-pointer"
							@click="errorCollapsed = !errorCollapsed"
						>
							<span class="flex items-center gap-2 text-contrast font-extrabold m-0">
								<WrenchIcon class="h-4 w-4" />
								{{ formatMessage(messages.debugInformation) }}
							</span>
							<DropdownIcon
								class="h-5 w-5 text-secondary transition-transform"
								:class="{ 'rotate-180': !errorCollapsed }"
							/>
						</button>
						<Collapsible :collapsed="errorCollapsed">
							<div
								class="p-3 bg-surface-2 rounded-2xl text-xs grid grid-cols-[1fr_auto] max-w-full items-start"
							>
								<div
									class="m-0 p-0 rounded-none bg-transparent text-sm font-mono break-words overflow-auto"
								>
									{{ debugInfo }}
									<button class="btn" @click="openDbBackupsFolder">
										<FolderOpenIcon aria-hidden="true" />
										{{ formatMessage(messages.openBackupsFolder) }}
									</button>
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
			</template>
		</div>
	</ModalWrapper>
</template>

<style>
.light-mode {
	--color-orange-bg: rgba(255, 163, 71, 0.2);
}

.dark-mode,
.oled-mode {
	--color-orange-bg: rgba(224, 131, 37, 0.2);
}
</style>

<style scoped lang="scss">
.cta-button {
	display: flex;
	align-items: center;
	justify-content: center;
	padding: 0.5rem;
	gap: 0.5rem;
}

.warning-banner {
	display: flex;
	flex-direction: column;
	gap: 0.5rem;
	padding: var(--gap-lg);
	background-color: var(--color-orange-bg);
	border: 2px solid var(--color-orange);
	border-radius: var(--radius-md);
	margin-bottom: 1rem;
}

.warning-banner__title {
	display: flex;
	align-items: center;
	gap: 0.5rem;
	font-weight: 700;

	svg {
		color: var(--color-orange);
		height: 1.5rem;
		width: 1.5rem;
	}
}

.modal-body {
	display: flex;
	flex-direction: column;
	gap: var(--gap-md);
}

.markdown-body {
	overflow: auto;
}
</style>
