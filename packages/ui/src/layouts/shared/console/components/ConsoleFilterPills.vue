<template>
	<FilterPills v-model="selectedFilters" :options="visibleOptions">
		<template #all> {{ formatMessage(messages.all) }} </template>
	</FilterPills>
</template>

<script setup lang="ts">
import { computed } from 'vue'

import FilterPills from '#ui/components/base/FilterPills.vue'
import { defineMessages, useVIntl } from '#ui/composables/i18n'

import type { ConditionalLevel } from '../composables/console-filtering'
import type { LogLevel } from '../types'

type FilterValue = LogLevel | 'all'

const { formatMessage } = useVIntl()

const messages = defineMessages({
	all: { id: 'console.filter.all', defaultMessage: 'All' },
	error: { id: 'console.filter.error', defaultMessage: 'Error' },
	warn: { id: 'console.filter.warn', defaultMessage: 'Warn' },
	info: { id: 'console.filter.info', defaultMessage: 'Info' },
	debug: { id: 'console.filter.debug', defaultMessage: 'Debug' },
	trace: { id: 'console.filter.trace', defaultMessage: 'Trace' },
})

const ALWAYS_VISIBLE: Array<{ id: LogLevel; label: string }> = [
	{ id: 'error', label: formatMessage(messages.error) },
	{ id: 'warn', label: formatMessage(messages.warn) },
	{ id: 'info', label: formatMessage(messages.info) },
]

const CONDITIONAL_OPTIONS: Array<{ id: ConditionalLevel; label: string }> = [
	{ id: 'debug', label: formatMessage(messages.debug) },
	{ id: 'trace', label: formatMessage(messages.trace) },
]

const props = defineProps<{
	presentLevels: Set<ConditionalLevel>
}>()

const modelValue = defineModel<Set<FilterValue>>({ required: true })

const emit = defineEmits<{
	toggle: [value: FilterValue]
}>()

const visibleOptions = computed(() => [
	...ALWAYS_VISIBLE,
	...CONDITIONAL_OPTIONS.filter((opt) => props.presentLevels.has(opt.id)),
])

const selectedFilters = computed({
	get() {
		if (modelValue.value.has('all')) return []
		return [...modelValue.value] as string[]
	},
	set(ids: string[]) {
		if (ids.length === 0) {
			emit('toggle', 'all')
		} else {
			const current = selectedFilters.value
			const added = ids.find((id) => !current.includes(id))
			const removed = current.find((id) => !ids.includes(id))
			if (added) emit('toggle', added as FilterValue)
			if (removed) emit('toggle', removed as FilterValue)
		}
	},
})
</script>
