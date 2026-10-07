<template>
	<div class="flex min-h-0 flex-1 flex-col gap-3">
		<div class="flex flex-wrap items-center justify-between gap-2">
			<Tabs v-if="!split" :value="activeUuid ?? ''" :tabs="tabs" @update:value="active = $event" />
			<span v-else class="text-sm text-secondary">
				{{ formatMessage(messages.running, { count: copies.length }) }}
			</span>
			<Button type="outlined" size="sm" @click="toggleSplit">
				<LayoutGridIcon v-if="!split" aria-hidden="true" />
				<MaximizeIcon v-else aria-hidden="true" />
				{{ formatMessage(split ? messages.oneAtATime : messages.sideBySide) }}
			</Button>
		</div>
		<div class="flex min-h-0 flex-1 gap-4">
			<template v-for="copy in copies" :key="copy.uuid">
				<NoctrinthProcessConsole
					v-if="split || copy.uuid === activeUuid"
					:process="copy"
					:show-header="split"
				/>
			</template>
		</div>
	</div>
</template>

<script setup lang="ts">
/**
 * The console when an instance is running more than once: one copy at a time
 * behind tabs named after the account each is signed in as, or all of them
 * side by side. Side by side gives every copy its own search and filters,
 * which is cramped past two, so tabs are where it starts.
 */
import { LayoutGridIcon, MaximizeIcon } from '@modrinth/assets'
import { Button, defineMessages, Tabs, useVIntl } from '@modrinth/ui'
import { computed, ref } from 'vue'

import NoctrinthProcessConsole from '@/components/ui/NoctrinthProcessConsole.vue'

type Copy = { uuid: string; account_name?: string; start_time?: string }

const props = defineProps<{ copies: Copy[] }>()

const { formatMessage } = useVIntl()

const messages = defineMessages({
	running: {
		id: 'instance.console.copies-running',
		defaultMessage: '{count, plural, one {# copy running} other {# copies running}}',
	},
	sideBySide: { id: 'instance.console.side-by-side', defaultMessage: 'Side by side' },
	oneAtATime: { id: 'instance.console.one-at-a-time', defaultMessage: 'One at a time' },
	unknownAccount: {
		id: 'instance.console.copy-unknown-account',
		defaultMessage: 'Unknown account',
	},
})

const SPLIT_KEY = 'noctrinth:console-copies-split'

function readSplit(): boolean {
	try {
		return localStorage.getItem(SPLIT_KEY) === 'true'
	} catch {
		return false
	}
}

const split = ref(readSplit())

function toggleSplit() {
	split.value = !split.value
	try {
		localStorage.setItem(SPLIT_KEY, String(split.value))
	} catch {
		// Remembering the choice is a convenience; the toggle works without it.
	}
}

function startedAt(copy: Copy): string {
	if (!copy.start_time) return ''
	const started = new Date(copy.start_time)
	return Number.isNaN(started.getTime())
		? ''
		: started.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
}

const tabs = computed(() =>
	props.copies.map((copy) => {
		const account = copy.account_name || formatMessage(messages.unknownAccount)
		const time = startedAt(copy)
		return { value: copy.uuid, label: time ? `${account} · ${time}` : account }
	}),
)

const active = ref<string | number>('')

/** The chosen copy while it runs; the newest one otherwise. */
const activeUuid = computed(() =>
	props.copies.some((copy) => copy.uuid === active.value)
		? active.value
		: props.copies[props.copies.length - 1]?.uuid,
)
</script>
