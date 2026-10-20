<script setup lang="ts">
/**
 * Putting back instances that are on disk but not in the launcher.
 *
 * The folder holds the mods, the worlds and the settings; the database holds
 * everything else, and a database that was lost takes every instance with it
 * while every world is still sitting there. This finds those folders and offers
 * them back — see `packages/app-lib/src/api/instance_recovery.rs`.
 */
import { FolderSearchIcon, PlusIcon, SpinnerIcon } from '@modrinth/assets'
import {
	Admonition,
	Button,
	defineMessages,
	injectNotificationManager,
	useVIntl,
} from '@modrinth/ui'
import { useQueryClient } from '@tanstack/vue-query'
import { ref } from 'vue'

import { adoptOrphan, type OrphanedInstance, scanForOrphans } from '@/helpers/noctrinth-recovery'
import { instanceKeys } from '@/pages/instance/query-options'

const { formatMessage } = useVIntl()
const { handleError, addNotification } = injectNotificationManager()
const queryClient = useQueryClient()

const scanning = ref(false)
const scanned = ref(false)
const orphans = ref<OrphanedInstance[]>([])
const importing = ref<string | null>(null)

async function scan() {
	scanning.value = true
	try {
		orphans.value = await scanForOrphans()
		scanned.value = true
	} catch (error) {
		handleError(error)
	} finally {
		scanning.value = false
	}
}

async function adopt(orphan: OrphanedInstance) {
	if (!orphan.game_version) return

	importing.value = orphan.folder
	try {
		await adoptOrphan(
			orphan.folder,
			orphan.name,
			orphan.game_version,
			orphan.loader ?? 'vanilla',
			orphan.loader_version,
		)
		orphans.value = orphans.value.filter((other) => other.folder !== orphan.folder)
		await queryClient.invalidateQueries({ queryKey: instanceKeys.all })
		addNotification({
			title: formatMessage(messages.imported, { name: orphan.name }),
			text: formatMessage(messages.importedBody),
			type: 'success',
		})
	} catch (error) {
		handleError(error)
	} finally {
		importing.value = null
	}
}

function describe(orphan: OrphanedInstance): string {
	const parts: string[] = []
	if (orphan.game_version) {
		parts.push(
			orphan.loader && orphan.loader !== 'vanilla'
				? `${orphan.game_version} · ${loaderName(orphan.loader)}`
				: orphan.game_version,
		)
	}
	if (orphan.mods > 0) parts.push(formatMessage(messages.mods, { count: orphan.mods }))
	if (orphan.worlds > 0) parts.push(formatMessage(messages.worlds, { count: orphan.worlds }))
	return parts.join(' · ')
}

function loaderName(loader: string): string {
	return loader.charAt(0).toUpperCase() + loader.slice(1)
}

const messages = defineMessages({
	title: { id: 'app.recover-instances.title', defaultMessage: 'Instances in the launcher folder' },
	description: {
		id: 'app.recover-instances.description',
		defaultMessage:
			'Looks for instance folders the launcher has no record of — what is left when the database is lost but the folders are not.',
	},
	scan: { id: 'app.recover-instances.scan', defaultMessage: 'Scan the folder' },
	scanning: { id: 'app.recover-instances.scanning', defaultMessage: 'Scanning...' },
	none: {
		id: 'app.recover-instances.none',
		defaultMessage: 'Every folder in there is already an instance.',
	},
	found: {
		id: 'app.recover-instances.found',
		defaultMessage:
			'{count, plural, one {# folder is not in the launcher} other {# folders are not in the launcher}}',
	},
	import: { id: 'app.recover-instances.import', defaultMessage: 'Import' },
	unknownVersion: {
		id: 'app.recover-instances.unknown-version',
		defaultMessage: 'No log or crash report to read the version from — import it by hand.',
	},
	mods: {
		id: 'app.recover-instances.mods',
		defaultMessage: '{count, plural, one {# mod} other {# mods}}',
	},
	worlds: {
		id: 'app.recover-instances.worlds',
		defaultMessage: '{count, plural, one {# world} other {# worlds}}',
	},
	imported: { id: 'app.recover-instances.imported', defaultMessage: '{name} is back' },
	importedBody: {
		id: 'app.recover-instances.imported-body',
		defaultMessage: 'Its folder is untouched. The version files are fetched on the next launch.',
	},
})
</script>

<template>
	<section class="border-0 border-b border-solid border-surface-4 pb-6">
		<div class="flex flex-col gap-4">
			<div class="flex items-center justify-between gap-6">
				<div class="flex min-w-0 flex-col gap-1">
					<h2 class="m-0 text-lg font-semibold text-contrast">
						{{ formatMessage(messages.title) }}
					</h2>
					<p class="m-0 text-secondary">{{ formatMessage(messages.description) }}</p>
				</div>
				<Button class="shrink-0" :disabled="scanning" @click="scan">
					<SpinnerIcon v-if="scanning" class="animate-spin" aria-hidden="true" />
					<FolderSearchIcon v-else aria-hidden="true" />
					{{ formatMessage(scanning ? messages.scanning : messages.scan) }}
				</Button>
			</div>

			<p v-if="scanned && orphans.length === 0" class="m-0 text-secondary">
				{{ formatMessage(messages.none) }}
			</p>

			<template v-if="orphans.length > 0">
				<span class="font-semibold text-contrast">
					{{ formatMessage(messages.found, { count: orphans.length }) }}
				</span>

				<div class="flex flex-col gap-2">
					<div
						v-for="orphan in orphans"
						:key="orphan.folder"
						class="flex items-center justify-between gap-4 rounded-xl bg-surface-2 p-3"
					>
						<div class="flex min-w-0 flex-col gap-0.5">
							<span class="truncate font-semibold text-contrast">{{ orphan.name }}</span>
							<span class="truncate text-sm text-secondary">{{ describe(orphan) }}</span>
						</div>
						<Button
							class="shrink-0"
							:disabled="!orphan.game_version || importing === orphan.folder"
							@click="adopt(orphan)"
						>
							<SpinnerIcon
								v-if="importing === orphan.folder"
								class="animate-spin"
								aria-hidden="true"
							/>
							<PlusIcon v-else aria-hidden="true" />
							{{ formatMessage(messages.import) }}
						</Button>
					</div>
				</div>

				<Admonition v-if="orphans.some((orphan) => !orphan.game_version)" type="info">
					{{ formatMessage(messages.unknownVersion) }}
				</Admonition>
			</template>
		</div>
	</section>
</template>
