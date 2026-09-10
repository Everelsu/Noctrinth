<template>
	<div class="flex flex-col gap-4 h-full">
		<!--
			What the launcher itself made of the files this instance last wrote.
			Upstream's banner above the console is mclo.gs reading the live log;
			this one reads the crash report and the JVM's own error file too, and
			says so without a connection. See helpers/noctrinth-crash.ts.
		-->
		<NoctrinthCrashDiagnosis
			v-if="localFindings.length"
			:instance-id="instanceId"
			:findings="localFindings"
			:on-open-settings="instancePage.openSettings"
			@dismiss="localFindings = []"
		/>
		<!--
			One console is two games talking over each other once an instance is
			running twice, so each copy gets its own. One copy, or none, is the page
			as it has always been: the live log alongside every log kept on disk.
		-->
		<div v-if="runningCopies.length > 1" class="flex min-h-0 flex-1 gap-4">
			<NoctrinthProcessConsole v-for="copy in runningCopies" :key="copy.uuid" :process="copy" />
		</div>
		<ConsolePageLayout v-else />
	</div>
</template>

<script setup>
import {
	ConsolePageLayout,
	defineMessages,
	injectModrinthClient,
	injectNotificationManager,
	provideConsoleManager,
	useVIntl,
} from '@modrinth/ui'
import { useQuery } from '@tanstack/vue-query'
import { computed, ref, shallowRef, triggerRef, watch, watchEffect } from 'vue'

import NoctrinthCrashDiagnosis from '@/components/ui/NoctrinthCrashDiagnosis.vue'
import NoctrinthProcessConsole from '@/components/ui/NoctrinthProcessConsole.vue'
import { useAppEvent } from '@/composables/use-app-event'
import { useInstanceConsole } from '@/composables/useInstanceConsole'
import { delete_logs_by_filename, get_output_by_filename } from '@/helpers/logs.js'
import { analyzeCrashText, analyzeInstanceCrash } from '@/helpers/noctrinth-crash'

import { injectInstancePage } from '../instance-context'
import { instanceKeys, instanceProcessesQueryOptions } from '../query-options'

const { formatMessage } = useVIntl()

const messages = defineMessages({
	liveLog: { id: 'app.instance.logs.live', defaultMessage: 'Live Log' },
	unknownLog: { id: 'app.instance.logs.unknown-file', defaultMessage: 'Unknown' },
})

const client = injectModrinthClient()
const { handleError } = injectNotificationManager()
const instancePage = injectInstancePage()
const instanceId = instancePage.instanceId
const {
	liveConsole,
	historicalConsole,
	hydrate,
	getHistoricalLogs,
	getHistoricalContent,
	invalidate,
	clearLive,
} = useInstanceConsole(instanceId.value)

const consoleHydrationQuery = useQuery({
	queryKey: computed(() => instanceKeys.console(instanceId.value)),
	queryFn: async () => {
		await hydrate()
		return true
	},
	staleTime: 0,
	refetchOnMount: 'always',
})

await consoleHydrationQuery.suspense()

function buildLogList(rawLogs) {
	return [
		{ name: formatMessage(messages.liveLog), live: true },
		...rawLogs
			.filter(
				(log) =>
					log.filename !== 'latest_stdout.log' &&
					log.filename !== 'latest_stdout' &&
					log.filename !== 'launcher_log.txt' &&
					(log.output == null || log.output !== '') &&
					(log.filename.includes('.log') || log.filename.endsWith('.txt')),
			)
			.map((log) => ({
				...log,
				name: log.filename || formatMessage(messages.unknownLog),
			})),
	]
}

const logs = ref(buildLogList([]))
const historicalLogsQuery = useQuery({
	queryKey: computed(() => instanceKeys.logs(instanceId.value)),
	queryFn: getHistoricalLogs,
	staleTime: 0,
})
watch(
	historicalLogsQuery.data,
	(allLogs) => {
		if (allLogs) logs.value = buildLogList(allLogs)
	},
	{ immediate: true },
)
watch(historicalLogsQuery.error, (error) => {
	if (error) handleError(error)
})

// The copies of this instance that are running, which is what decides whether
// this page shows one console or one per copy.
const processesQuery = useQuery(
	computed(() => ({
		...instanceProcessesQueryOptions(instanceId.value),
		enabled: !!instanceId.value,
	})),
)
// Oldest first, so the panes keep their places instead of swapping about
// whenever the list is fetched again.
const runningCopies = computed(() =>
	[...(processesQuery.data.value ?? [])].sort((a, b) =>
		String(a.start_time ?? '').localeCompare(String(b.start_time ?? '')),
	),
)

const selectedLogIndex = ref(0)
const isLive = computed(() => selectedLogIndex.value === 0)

const filteredLogs = computed(() =>
	instancePage.playing.value
		? logs.value.filter((l) => l.live || l.name !== 'latest.log')
		: logs.value,
)

const logSources = computed(() =>
	filteredLogs.value.map((l, i) => ({
		id: String(i),
		name: l?.name ?? `Log ${i}`,
		live: l?.live ?? false,
	})),
)

const activeConsole = computed(() => (isLive.value ? liveConsole : historicalConsole))

const logLines = shallowRef(activeConsole.value.output.value)
watchEffect(() => {
	logLines.value = activeConsole.value.output.value
	triggerRef(logLines)
})

const crashAnalysis = ref(null)

const localFindings = ref([])

/**
 * Reads what the instance left on disk: the crash report, the JVM error file
 * and the tail of the log. Runs when the page opens and when a run ends, which
 * is when those files are complete.
 */
async function diagnoseInstance() {
	const diagnosis = await analyzeInstanceCrash(instanceId.value)
	localFindings.value = diagnosis.findings
}

/** The same rules against a log the player has picked out of the list. */
async function diagnoseText(output, logType, filename) {
	const diagnosis = await analyzeCrashText(
		output,
		logType === 'CrashReport' ? 'crash_report' : 'log',
		filename,
	)
	localFindings.value = diagnosis.findings
}

async function analyseForCrash() {
	const lines = liveConsole.output.value
	if (lines.length === 0) return

	const content = lines.map((l) => l.text).join('\n')
	try {
		const data = await client.mclogs.insights_v1.analyse(content)
		if (data.analysis?.problems?.length > 0) {
			crashAnalysis.value = data
		}
	} catch {
		// Crash analysis is best-effort
	}
}

const selectedLog = computed(() => filteredLogs.value[selectedLogIndex.value])

const deleteDisabled = computed(() => {
	const log = selectedLog.value
	if (!log || log.live) return true
	return log.filename === 'latest.log' && instancePage.playing.value
})

async function deleteSelectedLog() {
	const log = selectedLog.value
	if (!log || log.live) return
	await delete_logs_by_filename(instanceId.value, log.log_type, log.filename)
	invalidate()
	const { data } = await historicalLogsQuery.refetch()
	if (data) logs.value = buildLogList(data)
	selectedLogIndex.value = 0
}

provideConsoleManager({
	logLines,
	logSources,
	activeLogSourceIndex: selectedLogIndex,
	showCommandInput: false,
	loading: ref(false),
	onClear: () => {
		if (!isLive.value) return
		void clearLive()
	},
	onDelete: deleteSelectedLog,
	deleteDisabled,
	deleteDisabledTooltip: 'Cannot delete latest.log while the instance is running',
	shareDisabled: instancePage.offline,
	emptyStateType: 'instance',
	crashAnalysis,
	onDismissCrash: () => {
		crashAnalysis.value = null
	},
})

watch(selectedLogIndex, async (newIndex) => {
	if (newIndex === 0) {
		// Back to the live console, where what is on disk is what there is to say.
		void diagnoseInstance()
		return
	}
	const log = filteredLogs.value[newIndex]
	if (!log) return

	const cached = getHistoricalContent(log.filename)
	if (cached) {
		historicalConsole.clear()
		historicalConsole.addLegacyLog(cached)
		void diagnoseText(cached, log.log_type, log.filename)
		return
	}

	const output = await get_output_by_filename(instanceId.value, log.log_type, log.filename).catch(
		handleError,
	)
	if (output) {
		historicalConsole.clear()
		historicalConsole.addLegacyLog(output)
		// A log picked out of the list is read by the same rules, so an old
		// crash can be looked at as well as the last one.
		void diagnoseText(output, log.log_type, log.filename)
	}
})

selectedLogIndex.value = 0

if (!instancePage.playing.value) {
	void analyseForCrash()
	void diagnoseInstance()
}

useAppEvent('log', (payload) => {
	if (payload.instance_id !== instanceId.value) return

	// While an instance is running twice, every copy has a console of its own and
	// this one is not shown. It follows the copy that started last, which is the
	// one the buffer behind it holds, so that it is coherent the moment the other
	// copies stop and the page comes back to a single console.
	const newest = runningCopies.value[runningCopies.value.length - 1]
	if (newest && payload.process_uuid && payload.process_uuid !== newest.uuid) return

	if (payload.type === 'log4j') {
		liveConsole.addLog4jEvent(payload)
	} else if (payload.type === 'legacy') {
		liveConsole.addLegacyLog(payload.message)
	}
})

useAppEvent('process', async (e) => {
	if (e.instance_id !== instanceId.value) return
	// A copy starting or stopping is what adds or takes away a console.
	void processesQuery.refetch()
	if (e.event === 'launched') {
		liveConsole.clear()
		invalidate()
		void historicalLogsQuery.refetch()
		selectedLogIndex.value = 0
	}
	if (e.event === 'finished') {
		invalidate()
		const { data } = await historicalLogsQuery.refetch()
		if (data) logs.value = buildLogList(data)
		void analyseForCrash()
		void diagnoseInstance()
	}
})
</script>
