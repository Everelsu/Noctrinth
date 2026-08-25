<!--
	The live console of one copy of a running instance.

	An instance can be running more than once, on a different account each time,
	and then one console is two games talking over each other. This is one copy's
	own: its own output, its own search and filters, its own clear. The page puts
	one of these beside another when there is more than one copy to show, which is
	why it provides a console manager of its own rather than sharing the page's.
-->
<template>
	<div class="flex min-h-0 min-w-0 flex-1 flex-col gap-2">
		<div class="flex items-center gap-2 text-sm">
			<OnlineIndicatorIcon />
			<span class="text-contrast">{{ accountName }}</span>
			<span class="text-secondary">{{
				formatMessage(messages.startedAt, { time: startedAt })
			}}</span>
		</div>
		<ConsolePageLayout />
	</div>
</template>

<script setup lang="ts">
import { OnlineIndicatorIcon } from '@modrinth/assets'
import { ConsolePageLayout, defineMessages, provideConsoleManager, useVIntl } from '@modrinth/ui'
import { computed, onMounted, ref, shallowRef, triggerRef, watch } from 'vue'

import { useAppEvent } from '@/composables/use-app-event'
import { useProcessConsole } from '@/composables/useInstanceConsole'

const props = defineProps<{
	process: { uuid: string; account_name?: string; start_time?: string }
}>()

const { formatMessage } = useVIntl()

const messages = defineMessages({
	startedAt: {
		id: 'instance.console.copy-started-at',
		defaultMessage: 'started {time}',
	},
	unknownAccount: {
		id: 'instance.console.copy-unknown-account',
		defaultMessage: 'Unknown account',
	},
})

const { console: liveConsole, hydrate, clear } = useProcessConsole(props.process.uuid)

const accountName = computed(
	() => props.process.account_name || formatMessage(messages.unknownAccount),
)

const startedAt = computed(() => {
	if (!props.process.start_time) return ''
	const started = new Date(props.process.start_time)
	return Number.isNaN(started.getTime()) ? '' : started.toLocaleTimeString()
})

const loading = ref(true)
const logLines = shallowRef(liveConsole.output.value)

watch(
	liveConsole.output,
	(lines) => {
		logLines.value = lines
		triggerRef(logLines)
	},
	{ deep: true },
)

onMounted(async () => {
	// Not silently: an empty console with the game plainly running is the kind
	// of thing that should say why.
	await hydrate().catch((error) => {
		console.warn('Failed to read back what this copy has already said:', error)
	})
	loading.value = false
})

// Kept in memory after the pane goes away, the way the single console has always
// been: leaving the page and coming back should show what was already there
// rather than depend on reading it back, which is a request that can fail.
// One console per copy of an instance, and only for copies started this run.

// Every copy hears every line, so each one keeps only what it said itself.
useAppEvent('log', (payload) => {
	if (payload.process_uuid !== props.process.uuid) return

	if (payload.type === 'log4j') {
		liveConsole.addLog4jEvent(payload)
	} else if (payload.type === 'legacy') {
		liveConsole.addLegacyLog(payload.message)
	}
})

provideConsoleManager({
	logLines,
	loading,
	emptyStateType: 'instance',
	onClear: () => {
		void clear()
	},
})
</script>
