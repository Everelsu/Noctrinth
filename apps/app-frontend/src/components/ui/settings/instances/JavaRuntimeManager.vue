<script setup>
import { TrashIcon } from '@modrinth/assets'
import {
	Button,
	defineMessages,
	IconButton,
	injectNotificationManager,
	useFormatBytes,
	useVIntl,
} from '@modrinth/ui'
import { computed, ref } from 'vue'

import {
	list_installed_runtimes,
	remove_installed_runtime,
	remove_unused_runtimes,
} from '@/helpers/jre'

const { handleError, addNotification } = injectNotificationManager()
const formatBytes = useFormatBytes()
const { formatMessage } = useVIntl()

const messages = defineMessages({
	title: {
		id: 'app.settings.java.runtimes.title',
		defaultMessage: 'Downloaded runtimes',
	},
	description: {
		id: 'app.settings.java.runtimes.description',
		defaultMessage:
			'Java the launcher downloaded for you, {size} in total. Updating Java leaves the previous copy behind, so these build up over time.',
	},
	removeUnused: {
		id: 'app.settings.java.runtimes.remove-unused',
		defaultMessage: 'Remove {count} unused ({size})',
	},
	inUse: {
		id: 'app.settings.java.runtimes.in-use',
		defaultMessage: 'In use',
	},
	inUseTooltip: {
		id: 'app.settings.java.runtimes.in-use-tooltip',
		defaultMessage: 'A Java setting points at this one — change it first',
	},
	deleteTooltip: {
		id: 'app.settings.java.runtimes.delete-tooltip',
		defaultMessage: 'Delete this runtime',
	},
	deleteLabel: {
		id: 'app.settings.java.runtimes.delete-label',
		defaultMessage: 'Delete runtime',
	},
	removedTitle: {
		id: 'app.settings.java.runtimes.removed-title',
		defaultMessage: 'Runtimes removed',
	},
	removedText: {
		id: 'app.settings.java.runtimes.removed-text',
		defaultMessage: 'Reclaimed {size}.',
	},
})

const emit = defineEmits(['changed'])

const runtimes = ref([])
const busy = ref(false)

async function refresh() {
	try {
		runtimes.value = await list_installed_runtimes()
	} catch (error) {
		handleError(error)
		runtimes.value = []
	}
}

await refresh()

const unused = computed(() => runtimes.value.filter((runtime) => !runtime.in_use))
const reclaimable = computed(() =>
	unused.value.reduce((total, runtime) => total + runtime.size_bytes, 0),
)
const totalSize = computed(() =>
	runtimes.value.reduce((total, runtime) => total + runtime.size_bytes, 0),
)

async function removeOne(runtime) {
	if (busy.value) return
	busy.value = true
	try {
		await remove_installed_runtime(runtime.path)
		await refresh()
		emit('changed')
	} catch (error) {
		handleError(error)
	} finally {
		busy.value = false
	}
}

async function removeUnused() {
	if (busy.value) return
	busy.value = true
	try {
		const freed = await remove_unused_runtimes()
		await refresh()
		emit('changed')
		addNotification({
			title: formatMessage(messages.removedTitle),
			text: formatMessage(messages.removedText, { size: formatBytes(freed) }),
			type: 'success',
		})
	} catch (error) {
		handleError(error)
	} finally {
		busy.value = false
	}
}
</script>

<template>
	<div v-if="runtimes.length" class="flex flex-col gap-3">
		<div class="flex items-start justify-between gap-4">
			<div class="flex flex-col gap-1 min-w-0">
				<h2 class="m-0 text-lg font-semibold text-contrast">
					{{ formatMessage(messages.title) }}
				</h2>
				<p class="m-0 leading-tight text-secondary">
					{{ formatMessage(messages.description, { size: formatBytes(totalSize) }) }}
				</p>
			</div>
			<Button
				v-if="unused.length"
				:disabled="busy"
				type="colored"
				color="red"
				class="shrink-0"
				@click="removeUnused"
			>
				<TrashIcon />
				{{
					formatMessage(messages.removeUnused, {
						count: unused.length,
						size: formatBytes(reclaimable),
					})
				}}
			</Button>
		</div>

		<div
			v-for="runtime in runtimes"
			:key="runtime.path"
			class="flex items-center gap-3 rounded-lg bg-bg-raised px-3 py-2"
		>
			<div class="flex flex-col min-w-0 flex-1">
				<span class="flex items-center gap-2 min-w-0">
					<span class="font-semibold text-contrast truncate">
						{{ runtime.major_version ? `Java ${runtime.major_version}` : runtime.name }}
					</span>
					<span
						v-if="runtime.in_use"
						class="shrink-0 rounded px-1.5 py-0.5 text-[0.65rem] font-bold uppercase tracking-wide bg-brand-highlight text-brand"
					>
						{{ formatMessage(messages.inUse) }}
					</span>
				</span>
				<span class="text-xs text-secondary truncate" :title="runtime.path">
					{{ runtime.name }}
				</span>
			</div>

			<span class="shrink-0 text-sm text-secondary tabular-nums">
				{{ formatBytes(runtime.size_bytes) }}
			</span>

			<IconButton
				v-tooltip="formatMessage(runtime.in_use ? messages.inUseTooltip : messages.deleteTooltip)"
				type="quiet"
				color="red"
				:label="formatMessage(messages.deleteLabel)"
				class="shrink-0"
				:disabled="busy || runtime.in_use"
				@click="removeOne(runtime)"
			>
				<TrashIcon />
			</IconButton>
		</div>
	</div>
</template>
