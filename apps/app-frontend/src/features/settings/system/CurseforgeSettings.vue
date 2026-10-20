<script setup lang="ts">
/** Noctrinth's own: what the launcher does with CurseForge, and whether at all. */
import {
	CheckIcon,
	CurseForgeIcon,
	SpinnerIcon,
	TrashIcon,
	UndoIcon,
	XIcon,
} from '@modrinth/assets'
import {
	Button,
	defineMessages,
	Input,
	RadioButtons,
	SettingsToggleCard,
	Toggle,
	useVIntl,
} from '@modrinth/ui'
import { ref } from 'vue'

import {
	checkCurseforgeKey,
	clearCurseforgeCache,
	curseforgeKeyAvailable,
	type CurseforgeReleaseType,
	curseforgeSettings,
	hasUserCurseforgeKey,
	setUserCurseforgeKey,
} from '@/helpers/curseforge-settings'

const { formatMessage } = useVIntl()

const messages = defineMessages({
	title: { id: 'noctrinth.settings.curseforge.title', defaultMessage: 'CurseForge' },
	description: {
		id: 'noctrinth.settings.curseforge.description',
		defaultMessage:
			'Browse and install from CurseForge, recognise its files in your instances and keep them up to date. Off, and the launcher asks CurseForge nothing.',
	},
	noKey: {
		id: 'noctrinth.settings.curseforge.no-key',
		defaultMessage: 'This build has no CurseForge API key. Add your own below to use CurseForge.',
	},
	ownKey: { id: 'noctrinth.settings.curseforge.own-key', defaultMessage: 'Your own API key' },
	ownKeyHint: {
		id: 'noctrinth.settings.curseforge.own-key.hint',
		defaultMessage:
			'Used instead of the built-in one and kept on this computer. Sign in to the CurseForge console and copy a key from its API keys page:',
	},
	ownKeyPlaceholder: {
		id: 'noctrinth.settings.curseforge.own-key.placeholder',
		defaultMessage: 'Your key is saved',
	},
	check: { id: 'noctrinth.settings.curseforge.own-key.check', defaultMessage: 'Check' },
	checkAndSave: {
		id: 'noctrinth.settings.curseforge.own-key.check-and-save',
		defaultMessage: 'Check and save',
	},
	forget: { id: 'noctrinth.settings.curseforge.own-key.forget', defaultMessage: 'Forget my key' },
	keyOk: {
		id: 'noctrinth.settings.curseforge.own-key.ok',
		defaultMessage: 'CurseForge accepts the key',
	},
	keySaved: {
		id: 'noctrinth.settings.curseforge.own-key.saved',
		defaultMessage: 'CurseForge accepts the key; it is saved',
	},
	keyInvalid: {
		id: 'noctrinth.settings.curseforge.own-key.invalid',
		defaultMessage: 'CurseForge does not accept this key',
	},
	checkFailed: {
		id: 'noctrinth.settings.curseforge.own-key.check-failed',
		defaultMessage: 'Could not reach CurseForge to check the key',
	},
	defaultSource: {
		id: 'noctrinth.settings.curseforge.default-source',
		defaultMessage: 'Browse opens on',
	},
	modrinth: { id: 'noctrinth.settings.curseforge.source.modrinth', defaultMessage: 'Modrinth' },
	curseforge: {
		id: 'noctrinth.settings.curseforge.source.curseforge',
		defaultMessage: 'CurseForge',
	},
	recognizeFiles: {
		id: 'noctrinth.settings.curseforge.recognize-files',
		defaultMessage: 'Recognise mods added by hand or from other launchers by their fingerprint',
	},
	checkUpdates: {
		id: 'noctrinth.settings.curseforge.check-updates',
		defaultMessage: 'Check CurseForge for updates of its mods and packs',
	},
	releaseType: {
		id: 'noctrinth.settings.curseforge.release-type',
		defaultMessage: 'Update to',
	},
	release: {
		id: 'noctrinth.settings.curseforge.release-type.release',
		defaultMessage: 'Releases only',
	},
	beta: {
		id: 'noctrinth.settings.curseforge.release-type.beta',
		defaultMessage: 'Releases and betas',
	},
	alpha: {
		id: 'noctrinth.settings.curseforge.release-type.alpha',
		defaultMessage: 'Any file, alphas too',
	},
	searchOtherLaunchers: {
		id: 'noctrinth.settings.curseforge.search-other-launchers',
		defaultMessage:
			"Look for files kept off launchers in other launchers' instances, not only in Downloads",
	},
	clearCache: {
		id: 'noctrinth.settings.curseforge.clear-cache',
		defaultMessage: "Forget CurseForge's answers",
	},
	clearCacheHint: {
		id: 'noctrinth.settings.curseforge.clear-cache-hint',
		defaultMessage: 'They are kept for five minutes; this asks again right away.',
	},
	cleared: { id: 'noctrinth.settings.curseforge.cleared', defaultMessage: 'Forgotten' },
})

const sources = ['modrinth', 'curseforge'] as const
const releaseTypes: CurseforgeReleaseType[] = ['release', 'beta', 'alpha']

const keyInput = ref('')
const checking = ref(false)
const keyResult = ref<{ ok: boolean; text: string } | null>(null)

/** Checks the typed key and saves it if CurseForge takes it, or checks the saved one. */
async function checkKey() {
	const key = keyInput.value.trim() || null
	checking.value = true
	keyResult.value = null
	try {
		const ok = await checkCurseforgeKey(key)
		if (ok && key) {
			await setUserCurseforgeKey(key)
			keyInput.value = ''
		}
		keyResult.value = {
			ok,
			text: formatMessage(ok ? (key ? messages.keySaved : messages.keyOk) : messages.keyInvalid),
		}
	} catch {
		keyResult.value = { ok: false, text: formatMessage(messages.checkFailed) }
	} finally {
		checking.value = false
	}
}

async function forgetKey() {
	await setUserCurseforgeKey(null)
	keyResult.value = null
}

const cleared = ref(false)
async function clearCache() {
	await clearCurseforgeCache()
	cleared.value = true
}
</script>

<template>
	<p v-if="!curseforgeKeyAvailable" class="m-0 text-secondary">
		{{ formatMessage(messages.noKey) }}
	</p>
	<SettingsToggleCard
		v-else
		v-model="curseforgeSettings.enabled"
		:icon="CurseForgeIcon"
		:title="formatMessage(messages.title)"
		:description="formatMessage(messages.description)"
	>
		<template #expanded>
			<section class="flex flex-col gap-2">
				<h3 class="m-0 text-base font-semibold text-contrast">
					{{ formatMessage(messages.defaultSource) }}
				</h3>
				<RadioButtons v-model="curseforgeSettings.defaultSource" :items="[...sources]">
					<template #default="{ item }">
						{{ formatMessage(messages[item as (typeof sources)[number]]) }}
					</template>
				</RadioButtons>
			</section>

			<section class="flex flex-col gap-3">
				<label class="flex items-center justify-between gap-4">
					<span class="text-primary">{{ formatMessage(messages.recognizeFiles) }}</span>
					<Toggle v-model="curseforgeSettings.recognizeFiles" />
				</label>
				<label class="flex items-center justify-between gap-4">
					<span class="text-primary">{{ formatMessage(messages.checkUpdates) }}</span>
					<Toggle v-model="curseforgeSettings.checkUpdates" />
				</label>
			</section>

			<section v-if="curseforgeSettings.checkUpdates" class="flex flex-col gap-2">
				<h3 class="m-0 text-base font-semibold text-contrast">
					{{ formatMessage(messages.releaseType) }}
				</h3>
				<RadioButtons v-model="curseforgeSettings.updateReleaseType" :items="releaseTypes">
					<template #default="{ item }">
						{{ formatMessage(messages[item as CurseforgeReleaseType]) }}
					</template>
				</RadioButtons>
			</section>

			<section class="flex flex-col gap-3">
				<label class="flex items-center justify-between gap-4">
					<span class="text-primary">{{ formatMessage(messages.searchOtherLaunchers) }}</span>
					<Toggle v-model="curseforgeSettings.searchOtherLaunchers" />
				</label>
			</section>

			<section class="flex flex-wrap items-center gap-3">
				<Button @click="clearCache">
					<TrashIcon aria-hidden="true" />
					{{ formatMessage(messages.clearCache) }}
				</Button>
				<span v-if="cleared" class="flex items-center gap-1.5 text-sm text-green">
					<CheckIcon class="size-4 shrink-0" aria-hidden="true" />
					{{ formatMessage(messages.cleared) }}
				</span>
				<span v-else class="text-sm text-secondary">
					{{ formatMessage(messages.clearCacheHint) }}
				</span>
			</section>
		</template>
	</SettingsToggleCard>

	<section class="mt-4 flex flex-col gap-2">
		<label for="noctrinth-curseforge-key" class="text-base font-semibold text-contrast">
			{{ formatMessage(messages.ownKey) }}
		</label>
		<p class="m-0 text-sm text-secondary">
			{{ formatMessage(messages.ownKeyHint) }}
			<a
				href="https://console.curseforge.com/"
				target="_blank"
				rel="noopener noreferrer"
				class="font-semibold text-brand hover:underline"
				>console.curseforge.com</a
			>
		</p>
		<Input
			id="noctrinth-curseforge-key"
			v-model="keyInput"
			type="password"
			autocomplete="off"
			class="max-w-md"
			:placeholder="hasUserCurseforgeKey ? formatMessage(messages.ownKeyPlaceholder) : ''"
			@update:model-value="keyResult = null"
		/>
		<div class="flex flex-wrap items-center gap-3">
			<Button :disabled="checking || (!keyInput.trim() && !hasUserCurseforgeKey)" @click="checkKey">
				<SpinnerIcon v-if="checking" class="animate-spin" aria-hidden="true" />
				<CheckIcon v-else aria-hidden="true" />
				{{ formatMessage(keyInput.trim() ? messages.checkAndSave : messages.check) }}
			</Button>
			<Button v-if="hasUserCurseforgeKey" :disabled="checking" @click="forgetKey">
				<UndoIcon aria-hidden="true" />
				{{ formatMessage(messages.forget) }}
			</Button>
			<span
				v-if="keyResult"
				class="flex items-center gap-1.5 text-sm"
				:class="keyResult.ok ? 'text-green' : 'text-red'"
			>
				<CheckIcon v-if="keyResult.ok" class="size-4 shrink-0" aria-hidden="true" />
				<XIcon v-else class="size-4 shrink-0" aria-hidden="true" />
				{{ keyResult.text }}
			</span>
		</div>
	</section>
</template>
