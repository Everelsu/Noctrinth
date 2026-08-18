<script setup>
import { TrashIcon } from '@modrinth/assets'
import { Button, IconButton, injectNotificationManager, useFormatBytes } from '@modrinth/ui'
import { computed, ref } from 'vue'

import {
	list_installed_runtimes,
	remove_installed_runtime,
	remove_unused_runtimes,
} from '@/helpers/jre'

const { handleError, addNotification } = injectNotificationManager()
const formatBytes = useFormatBytes()

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
			title: 'Runtimes removed',
			text: `Reclaimed ${formatBytes(freed)}.`,
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
				<h2 class="m-0 text-lg font-semibold text-contrast">Downloaded runtimes</h2>
				<p class="m-0 leading-tight text-secondary">
					Java the launcher downloaded for you, {{ formatBytes(totalSize) }} in total. Updating Java
					leaves the previous copy behind, so these build up over time.
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
				Remove {{ unused.length }} unused ({{ formatBytes(reclaimable) }})
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
						In use
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
				v-tooltip="
					runtime.in_use
						? 'A Java setting points at this one — change it first'
						: 'Delete this runtime'
				"
				type="quiet"
				color="red"
				label="Delete runtime"
				class="shrink-0"
				:disabled="busy || runtime.in_use"
				@click="removeOne(runtime)"
			>
				<TrashIcon />
			</IconButton>
		</div>
	</div>
</template>
