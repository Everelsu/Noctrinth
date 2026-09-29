<script setup lang="ts">
/**
 * Comes up by itself when a game the launcher started goes down.
 *
 * The same diagnosis the Logs tab shows, brought to the player at the moment
 * they are wondering what happened, instead of waiting for them to go looking.
 * A run the player stopped, or one that ended cleanly, raises nothing.
 */
import { FileTextIcon, XIcon } from '@modrinth/assets'
import { Button, defineMessages, NewModal, useVIntl } from '@modrinth/ui'
import { ref, useTemplateRef } from 'vue'
import { useRouter } from 'vue-router'

import NoctrinthCrashDiagnosis from '@/components/ui/NoctrinthCrashDiagnosis.vue'
import { useAppEvent } from '@/composables/use-app-event'
import { get } from '@/helpers/instance'
import {
	analyzeInstanceCrash,
	crashed,
	type CrashFinding,
	sortFindings,
} from '@/helpers/noctrinth-crash'

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
	close: { id: 'app.crash-modal.close', defaultMessage: 'Close' },
})

const modal = useTemplateRef<InstanceType<typeof NewModal>>('modal')

const instanceId = ref('')
const instanceName = ref('')
const status = ref('')
const findings = ref<CrashFinding[]>([])

useAppEvent('process', async (event) => {
	if (event.event !== 'finished') return

	const diagnosis = await analyzeInstanceCrash(event.instance_id)
	if (!crashed(diagnosis)) return

	const instance = await get(event.instance_id).catch(() => null)
	instanceId.value = event.instance_id
	instanceName.value = instance?.name ?? event.instance_id
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
		max-width="640px"
	>
		<div class="flex flex-col gap-4">
			<span class="text-sm text-secondary">
				{{ formatMessage(messages.status, { status }) }}
			</span>

			<NoctrinthCrashDiagnosis
				v-if="findings.length"
				:key="instanceId"
				:instance-id="instanceId"
				:findings="findings"
				permanent
			/>
			<span v-else>{{ formatMessage(messages.nothingFound) }}</span>

			<div class="flex justify-end gap-2">
				<Button type="transparent" @click="modal?.hide()">
					<XIcon aria-hidden="true" />
					{{ formatMessage(messages.close) }}
				</Button>
				<Button color="brand" @click="openLogs">
					<FileTextIcon aria-hidden="true" />
					{{ formatMessage(messages.openLogs) }}
				</Button>
			</div>
		</div>
	</NewModal>
</template>
