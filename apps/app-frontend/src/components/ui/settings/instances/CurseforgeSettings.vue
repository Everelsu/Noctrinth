<script setup lang="ts">
/** Noctrinth's own: what the launcher does with CurseForge, and whether at all. */
import { CheckIcon, CurseForgeIcon, TrashIcon } from '@modrinth/assets'
import {
	Button,
	defineMessages,
	RadioButtons,
	SettingsToggleCard,
	Toggle,
	useVIntl,
} from '@modrinth/ui'
import { ref } from 'vue'

import { CURSEFORGE_API_KEY } from '@/helpers/curseforge-key'
import {
	clearCurseforgeCache,
	type CurseforgeReleaseType,
	curseforgeSettings,
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
		defaultMessage: 'This build has no CurseForge API key, so CurseForge is unavailable.',
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

const cleared = ref(false)
async function clearCache() {
	await clearCurseforgeCache()
	cleared.value = true
}
</script>

<template>
	<p v-if="!CURSEFORGE_API_KEY" class="m-0 text-secondary">{{ formatMessage(messages.noKey) }}</p>
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
</template>
