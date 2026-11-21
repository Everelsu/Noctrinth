<script setup lang="ts">
/**
 * Comes up by itself when a game the launcher started goes down.
 *
 * The same diagnosis the Logs tab shows, brought to the player at the moment
 * they are wondering what happened, instead of waiting for them to go looking.
 * A run the player stopped, or one that ended cleanly, raises nothing.
 */
import { FileTextIcon, FolderOpenIcon, IssuesIcon, XIcon } from '@modrinth/assets'
import { Avatar, Button, defineMessages, NewModal, useVIntl } from '@modrinth/ui'
import { ref, useTemplateRef } from 'vue'
import { useRouter } from 'vue-router'

import NoctrinthCrashDiagnosis from '@/components/ui/NoctrinthCrashDiagnosis.vue'
import { useAppEvent } from '@/composables/use-app-event'
import { get, getInstanceIconUrl } from '@/helpers/instance'
import {
	analyzeInstanceCrash,
	crashed,
	type CrashFinding,
	sortFindings,
} from '@/helpers/noctrinth-crash'
import { showInstanceInFolder } from '@/helpers/utils.js'

const { formatMessage } = useVIntl()
const router = useRouter()

const messages = defineMessages({
	header: { id: 'app.crash-modal.header', defaultMessage: '{instance} crashed' },
	status: { id: 'app.crash-modal.status', defaultMessage: 'Exit status: {status}' },
	nothingFound: {
		id: 'app.crash-modal.nothing-found',
		defaultMessage:
			'The launcher did not recognise the cause. The log has the details, and can be shared from there.',
	},
	openLogs: { id: 'app.crash-modal.open-logs', defaultMessage: 'Open logs' },
	openFolder: { id: 'app.crash.open-folder', defaultMessage: 'Open instance folder' },
	close: { id: 'app.crash-modal.close', defaultMessage: 'Close' },
})

const modal = useTemplateRef<InstanceType<typeof NewModal>>('modal')

const instanceId = ref('')
const instanceName = ref('')
const instanceIcon = ref<string | undefined>(undefined)
const instanceSubtitle = ref('')
const status = ref('')
const findings = ref<CrashFinding[]>([])

useAppEvent('process', async (event) => {
	if (event.event !== 'finished') return

	const diagnosis = await analyzeInstanceCrash(event.instance_id)
	if (!crashed(diagnosis)) return

	const instance = await get(event.instance_id).catch(() => null)
	instanceId.value = event.instance_id
	instanceName.value = instance?.name ?? event.instance_id
	instanceIcon.value = getInstanceIconUrl(instance?.icon_path)
	instanceSubtitle.value = instance
		? [instance.loader, instance.game_version].filter(Boolean).join(' · ')
		: ''
	status.value = diagnosis.exit?.status ?? ''
	findings.value = sortFindings(diagnosis.findings)
	modal.value?.show()
})

async function openLogs() {
	modal.value?.hide()
	await router.push(`/instance/${encodeURIComponent(instanceId.value)}/logs`)
}
</script>

<template>
	<NewModal
		ref="modal"
		:header="formatMessage(messages.header, { instance: instanceName })"
		scrollable
		max-width="620px"
	>
		<div class="flex flex-col gap-4">
			<div
				class="flex items-center justify-between gap-3 rounded-2xl border border-solid border-surface-5 bg-surface-3 px-3.5 py-2.5"
			>
				<div class="flex min-w-0 items-center gap-3">
					<Avatar :src="instanceIcon" :alt="instanceName" size="36px" class="shrink-0" />
					<div class="flex min-w-0 flex-col">
						<span class="truncate text-sm font-semibold text-contrast">{{ instanceName }}</span>
						<span v-if="instanceSubtitle" class="truncate text-xs capitalize text-secondary">
							{{ instanceSubtitle }}
						</span>
					</div>
				</div>
				<span
					v-if="status"
					class="shrink-0 rounded-full bg-highlight-red px-2.5 py-1 font-mono text-xs font-medium text-red"
				>
					{{ formatMessage(messages.status, { status }) }}
				</span>
			</div>

			<NoctrinthCrashDiagnosis
				v-if="findings.length"
				:key="instanceId"
				:instance-id="instanceId"
				:findings="findings"
				permanent
				embedded
			/>
			<div
				v-else
				class="flex items-start gap-3 rounded-2xl border border-solid border-surface-5 bg-surface-3 p-4"
			>
				<div class="grid size-9 shrink-0 place-items-center rounded-xl bg-highlight-orange">
					<IssuesIcon class="size-5 text-orange" />
				</div>
				<p class="m-0 text-sm leading-relaxed text-secondary">
					{{ formatMessage(messages.nothingFound) }}
				</p>
			</div>

			<div class="flex items-center justify-between gap-2 pt-1">
				<Button type="quiet" @click="showInstanceInFolder(instanceId)">
					<FolderOpenIcon aria-hidden="true" />
					{{ formatMessage(messages.openFolder) }}
				</Button>
				<div class="flex items-center gap-2">
					<Button type="outlined" @click="modal?.hide()">
						<XIcon aria-hidden="true" />
						{{ formatMessage(messages.close) }}
					</Button>
					<Button type="colored" color="brand" @click="openLogs">
						<FileTextIcon aria-hidden="true" />
						{{ formatMessage(messages.openLogs) }}
					</Button>
				</div>
			</div>
		</div>
	</NewModal>
</template>
