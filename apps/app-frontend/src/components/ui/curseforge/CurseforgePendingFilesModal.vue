<script setup lang="ts">
/**
 * Noctrinth's own: the CurseForge files an instance is waiting on.
 *
 * Some authors do not let launchers download their files. The launcher has
 * already looked for each one on disk; what is listed here was not found, so
 * the user downloads it from CurseForge's site. While the modal is open the
 * Downloads folder is checked every couple of seconds, and each file that
 * arrives is installed and ticked off, with nothing to drag anywhere.
 */
import {
	CheckIcon,
	DownloadIcon,
	ExternalIcon,
	FolderOpenIcon,
	SpinnerIcon,
	XIcon,
} from '@modrinth/assets'
import {
	Avatar,
	Button,
	defineMessages,
	IconButton,
	injectNotificationManager,
	NewModal,
	useVIntl,
} from '@modrinth/ui'
import { downloadDir } from '@tauri-apps/api/path'
import { openUrl } from '@tauri-apps/plugin-opener'
import { computed, onBeforeUnmount, ref } from 'vue'

import {
	curseforgeCheckPendingDownloads,
	curseforgeDismissPendingFile,
	type CurseforgePendingFile,
	curseforgePendingFiles,
} from '@/helpers/curseforge'
import { openPath } from '@/helpers/utils'

const { handleError } = injectNotificationManager()
const { formatMessage } = useVIntl()

const emit = defineEmits<{
	/** Some of the files were installed; the instance's content changed. */
	installed: []
	hide: []
}>()

const messages = defineMessages({
	header: {
		id: 'app.curseforge.pending.header',
		defaultMessage: 'Download by hand',
	},
	description: {
		id: 'app.curseforge.pending.description',
		defaultMessage:
			"{count, plural, one {This file's author doesn't} other {These files' authors don't}} let launchers download {count, plural, one {it} other {them}}. Download {count, plural, one {it} other {each one}} from CurseForge and it will be picked up from your Downloads folder automatically.",
	},
	openAll: {
		id: 'app.curseforge.pending.open-all',
		defaultMessage: 'Open download pages',
	},
	openDownloads: {
		id: 'app.curseforge.pending.open-downloads',
		defaultMessage: 'Open Downloads folder',
	},
	openPage: {
		id: 'app.curseforge.pending.open-page',
		defaultMessage: 'Download',
	},
	skip: {
		id: 'app.curseforge.pending.skip',
		defaultMessage: 'Do without it',
	},
	waiting: {
		id: 'app.curseforge.pending.waiting',
		defaultMessage: 'Watching your Downloads folder…',
	},
	found: {
		id: 'app.curseforge.pending.found',
		defaultMessage: 'Installed',
	},
	allDone: {
		id: 'app.curseforge.pending.all-done',
		defaultMessage: 'Everything is installed',
	},
	done: {
		id: 'app.curseforge.pending.done',
		defaultMessage: 'Done',
	},
})

const POLL_INTERVAL_MS = 2000

const modal = ref<InstanceType<typeof NewModal> | null>(null)
const instanceId = ref<string | null>(null)
/** Every file the modal was opened with, so a found one stays listed, ticked. */
const files = ref<CurseforgePendingFile[]>([])
const pendingIds = ref<Set<string>>(new Set())
const checking = ref(false)
let pollTimer: ReturnType<typeof setInterval> | null = null

const remaining = computed(() => files.value.filter((file) => pendingIds.value.has(file.file_id)))
const allDone = computed(() => files.value.length > 0 && remaining.value.length === 0)

defineExpose({
	/**
	 * Opens the modal for an instance. Without `known`, what the instance is
	 * waiting on is asked for; with nothing to wait on, it stays closed.
	 */
	async show(id: string, known?: CurseforgePendingFile[]) {
		try {
			const pending = known ?? (await curseforgePendingFiles(id))
			if (pending.length === 0) return
			instanceId.value = id
			files.value = pending
			pendingIds.value = new Set(pending.map((file) => file.file_id))
			modal.value?.show()
			startPolling()
		} catch (err) {
			handleError(err)
		}
	},
})

function startPolling() {
	stopPolling()
	pollTimer = setInterval(() => void check(), POLL_INTERVAL_MS)
}

function stopPolling() {
	if (pollTimer) clearInterval(pollTimer)
	pollTimer = null
}

async function check() {
	if (!instanceId.value || checking.value || allDone.value) return
	checking.value = true
	try {
		const pending = await curseforgeCheckPendingDownloads(instanceId.value)
		const stillPending = new Set(pending.map((file) => file.file_id))
		const arrived = [...pendingIds.value].some((id) => !stillPending.has(id))
		pendingIds.value = stillPending
		if (arrived) emit('installed')
		if (allDone.value) stopPolling()
	} catch (err) {
		stopPolling()
		handleError(err)
	} finally {
		checking.value = false
	}
}

async function openPage(file: CurseforgePendingFile) {
	if (file.download_page_url) await openUrl(file.download_page_url)
}

async function openAllPages() {
	for (const file of remaining.value) {
		await openPage(file)
	}
}

async function openDownloads() {
	try {
		await openPath(await downloadDir())
	} catch (err) {
		handleError(err)
	}
}

async function skip(file: CurseforgePendingFile) {
	if (!instanceId.value) return
	try {
		await curseforgeDismissPendingFile(instanceId.value, file.file_id)
		files.value = files.value.filter((f) => f.file_id !== file.file_id)
		pendingIds.value.delete(file.file_id)
		if (files.value.length === 0) modal.value?.hide()
	} catch (err) {
		handleError(err)
	}
}

function onHide() {
	stopPolling()
	emit('hide')
}

onBeforeUnmount(stopPolling)
</script>

<template>
	<NewModal
		ref="modal"
		:header="formatMessage(messages.header)"
		width="40rem"
		max-width="calc(100vw - 2rem)"
		scrollable
		@hide="onHide"
	>
		<div class="flex flex-col gap-4">
			<p class="m-0 text-secondary">
				{{ formatMessage(messages.description, { count: files.length }) }}
			</p>

			<ul class="m-0 flex list-none flex-col gap-2 p-0">
				<li
					v-for="file in files"
					:key="file.file_id"
					class="flex items-center gap-3 rounded-2xl bg-surface-2 p-3"
				>
					<Avatar :src="file.icon_url" size="40px" :alt="file.project_name ?? file.file_name" />
					<div class="flex min-w-0 flex-1 flex-col">
						<span class="truncate font-semibold text-contrast">
							{{ file.project_name ?? file.file_name }}
						</span>
						<span class="truncate text-sm text-secondary">{{ file.file_name }}</span>
					</div>
					<template v-if="pendingIds.has(file.file_id)">
						<Button
							type="outlined"
							color="brand"
							:disabled="!file.download_page_url"
							@click="openPage(file)"
						>
							<DownloadIcon aria-hidden="true" />
							{{ formatMessage(messages.openPage) }}
						</Button>
						<IconButton
							v-tooltip="formatMessage(messages.skip)"
							type="quiet"
							:label="formatMessage(messages.skip)"
							@click="skip(file)"
						>
							<XIcon aria-hidden="true" />
						</IconButton>
					</template>
					<span v-else class="flex items-center gap-1 font-semibold text-brand">
						<CheckIcon aria-hidden="true" />
						{{ formatMessage(messages.found) }}
					</span>
				</li>
			</ul>

			<div class="flex flex-wrap items-center justify-between gap-2">
				<span v-if="allDone" class="flex items-center gap-2 font-semibold text-brand">
					<CheckIcon aria-hidden="true" />
					{{ formatMessage(messages.allDone) }}
				</span>
				<span v-else class="flex items-center gap-2 text-sm text-secondary">
					<SpinnerIcon class="animate-spin" aria-hidden="true" />
					{{ formatMessage(messages.waiting) }}
				</span>
				<div class="flex flex-wrap gap-2">
					<template v-if="!allDone">
						<Button @click="openDownloads">
							<FolderOpenIcon aria-hidden="true" />
							{{ formatMessage(messages.openDownloads) }}
						</Button>
						<Button type="colored" color="brand" @click="openAllPages">
							<ExternalIcon aria-hidden="true" />
							{{ formatMessage(messages.openAll) }}
						</Button>
					</template>
					<Button v-else type="colored" color="brand" @click="modal?.hide()">
						{{ formatMessage(messages.done) }}
					</Button>
				</div>
			</div>
		</div>
	</NewModal>
</template>
