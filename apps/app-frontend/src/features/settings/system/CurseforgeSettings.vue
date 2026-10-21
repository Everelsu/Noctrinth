<script setup lang="ts">
/** Noctrinth's own: what the launcher does with CurseForge, and whether at all. */
import { CheckIcon, SpinnerIcon, TrashIcon, UndoIcon, XIcon } from '@modrinth/assets'
import {
	Button,
	Combobox,
	type ComboboxOption,
	defineMessages,
	Input,
	Toggle,
	useVIntl,
} from '@modrinth/ui'
import { computed, ref } from 'vue'

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
	discoverSection: {
		id: 'noctrinth.settings.curseforge.section.discover',
		defaultMessage: 'Discover',
	},
	defaultSource: {
		id: 'noctrinth.settings.curseforge.default-source',
		defaultMessage: 'Browse opens on',
	},
	defaultSourceDescription: {
		id: 'noctrinth.settings.curseforge.default-source.description',
		defaultMessage:
			'Which catalogue Discover shows first. The switch above the results changes it any time.',
	},
	modrinth: { id: 'noctrinth.settings.curseforge.source.modrinth', defaultMessage: 'Modrinth' },
	curseforge: {
		id: 'noctrinth.settings.curseforge.source.curseforge',
		defaultMessage: 'CurseForge',
	},
	filesSection: {
		id: 'noctrinth.settings.curseforge.section.files',
		defaultMessage: 'Files and updates',
	},
	recognizeFilesTitle: {
		id: 'noctrinth.settings.curseforge.recognize-files.title',
		defaultMessage: 'Recognise files',
	},
	recognizeFiles: {
		id: 'noctrinth.settings.curseforge.recognize-files',
		defaultMessage: 'Recognise mods added by hand or from other launchers by their fingerprint',
	},
	checkUpdatesTitle: {
		id: 'noctrinth.settings.curseforge.check-updates.title',
		defaultMessage: 'Check for updates',
	},
	checkUpdates: {
		id: 'noctrinth.settings.curseforge.check-updates',
		defaultMessage: 'Check CurseForge for updates of its mods and packs',
	},
	releaseType: {
		id: 'noctrinth.settings.curseforge.release-type',
		defaultMessage: 'Update to',
	},
	releaseTypeDescription: {
		id: 'noctrinth.settings.curseforge.release-type.description',
		defaultMessage: 'Which files count as an update.',
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
	restrictedSection: {
		id: 'noctrinth.settings.curseforge.section.restricted',
		defaultMessage: 'Files kept off launchers',
	},
	searchOtherLaunchersTitle: {
		id: 'noctrinth.settings.curseforge.search-other-launchers.title',
		defaultMessage: 'Search other launchers',
	},
	searchOtherLaunchers: {
		id: 'noctrinth.settings.curseforge.search-other-launchers',
		defaultMessage:
			"Look for files kept off launchers in other launchers' instances, not only in Downloads",
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
	cacheTitle: { id: 'noctrinth.settings.curseforge.cache.title', defaultMessage: 'Cache' },
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

const sourceOptions = computed<ComboboxOption<'modrinth' | 'curseforge'>[]>(() => [
	{ value: 'modrinth', label: formatMessage(messages.modrinth) },
	{ value: 'curseforge', label: formatMessage(messages.curseforge) },
])
const releaseTypeOptions = computed<ComboboxOption<CurseforgeReleaseType>[]>(() =>
	(['release', 'beta', 'alpha'] as const).map((type) => ({
		value: type,
		label: formatMessage(messages[type]),
	})),
)

/** Everything but the key waits on CurseForge being on and usable. */
const active = computed(() => curseforgeKeyAvailable.value && curseforgeSettings.enabled)

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
	<section class="border-0 border-b border-solid border-divider pb-6">
		<div class="flex items-center justify-between gap-4">
			<div>
				<h2 id="curseforge-enabled-label" class="m-0 text-lg font-semibold text-contrast">
					{{ formatMessage(messages.title) }}
				</h2>
				<p class="m-0 mt-1 text-secondary">
					{{ formatMessage(curseforgeKeyAvailable ? messages.description : messages.noKey) }}
				</p>
			</div>
			<Toggle
				id="curseforge-enabled"
				v-model="curseforgeSettings.enabled"
				:disabled="!curseforgeKeyAvailable"
				aria-labelledby="curseforge-enabled-label"
			/>
		</div>
	</section>

	<div :class="['transition-opacity', active ? '' : 'pointer-events-none opacity-50']">
		<section class="mt-6">
			<h2 class="m-0 text-xl font-semibold text-contrast">
				{{ formatMessage(messages.discoverSection) }}
			</h2>
			<div class="mt-4 flex flex-wrap items-center justify-between gap-4">
				<div class="min-w-0 flex-1">
					<h3 id="curseforge-source-label" class="m-0 text-lg font-semibold text-contrast">
						{{ formatMessage(messages.defaultSource) }}
					</h3>
					<p class="m-0 mt-1">{{ formatMessage(messages.defaultSourceDescription) }}</p>
				</div>
				<Combobox
					v-model="curseforgeSettings.defaultSource"
					:options="sourceOptions"
					aria-labelledby="curseforge-source-label"
					class="w-56"
				/>
			</div>
		</section>

		<section class="mt-6">
			<h2 class="m-0 text-xl font-semibold text-contrast">
				{{ formatMessage(messages.filesSection) }}
			</h2>
			<div class="mt-4 flex flex-col gap-6">
				<div class="flex items-center justify-between gap-4">
					<div>
						<h3 class="m-0 text-lg font-semibold text-contrast">
							{{ formatMessage(messages.recognizeFilesTitle) }}
						</h3>
						<p class="m-0 mt-1">{{ formatMessage(messages.recognizeFiles) }}</p>
					</div>
					<Toggle id="curseforge-recognize" v-model="curseforgeSettings.recognizeFiles" />
				</div>
				<div class="flex items-center justify-between gap-4">
					<div>
						<h3 class="m-0 text-lg font-semibold text-contrast">
							{{ formatMessage(messages.checkUpdatesTitle) }}
						</h3>
						<p class="m-0 mt-1">{{ formatMessage(messages.checkUpdates) }}</p>
					</div>
					<Toggle id="curseforge-updates" v-model="curseforgeSettings.checkUpdates" />
				</div>
				<div
					:class="[
						'flex flex-wrap items-center justify-between gap-4',
						curseforgeSettings.checkUpdates ? '' : 'pointer-events-none opacity-50',
					]"
				>
					<div class="min-w-0 flex-1">
						<h3 id="curseforge-release-label" class="m-0 text-lg font-semibold text-contrast">
							{{ formatMessage(messages.releaseType) }}
						</h3>
						<p class="m-0 mt-1">{{ formatMessage(messages.releaseTypeDescription) }}</p>
					</div>
					<Combobox
						v-model="curseforgeSettings.updateReleaseType"
						:options="releaseTypeOptions"
						aria-labelledby="curseforge-release-label"
						class="w-56"
					/>
				</div>
			</div>
		</section>

		<section class="mt-6">
			<h2 class="m-0 text-xl font-semibold text-contrast">
				{{ formatMessage(messages.restrictedSection) }}
			</h2>
			<div class="mt-4 flex items-center justify-between gap-4">
				<div>
					<h3 class="m-0 text-lg font-semibold text-contrast">
						{{ formatMessage(messages.searchOtherLaunchersTitle) }}
					</h3>
					<p class="m-0 mt-1">{{ formatMessage(messages.searchOtherLaunchers) }}</p>
				</div>
				<Toggle id="curseforge-other-launchers" v-model="curseforgeSettings.searchOtherLaunchers" />
			</div>
		</section>
	</div>

	<section class="mt-6">
		<h2 class="m-0 text-xl font-semibold text-contrast">
			<label for="noctrinth-curseforge-key">{{ formatMessage(messages.ownKey) }}</label>
		</h2>
		<p class="m-0 mt-1">
			{{ formatMessage(messages.ownKeyHint) }}
			<a
				href="https://console.curseforge.com/"
				target="_blank"
				rel="noopener noreferrer"
				class="font-semibold text-brand hover:underline"
				>console.curseforge.com</a
			>
		</p>
		<div class="mt-4 flex flex-col gap-3">
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
				<Button
					:disabled="checking || (!keyInput.trim() && !hasUserCurseforgeKey)"
					@click="checkKey"
				>
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
		</div>
	</section>

	<section :class="['mt-6', active ? '' : 'pointer-events-none opacity-50']">
		<h2 class="m-0 text-xl font-semibold text-contrast">
			{{ formatMessage(messages.cacheTitle) }}
		</h2>
		<div class="mt-4 flex flex-wrap items-center justify-between gap-4">
			<p class="m-0 min-w-0 flex-1">
				<span v-if="cleared" class="inline-flex items-center gap-1.5 text-green">
					<CheckIcon class="size-4 shrink-0" aria-hidden="true" />
					{{ formatMessage(messages.cleared) }}
				</span>
				<template v-else>{{ formatMessage(messages.clearCacheHint) }}</template>
			</p>
			<Button @click="clearCache">
				<TrashIcon aria-hidden="true" />
				{{ formatMessage(messages.clearCache) }}
			</Button>
		</div>
	</section>
</template>
