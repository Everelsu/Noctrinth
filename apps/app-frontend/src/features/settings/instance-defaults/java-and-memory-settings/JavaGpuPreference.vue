<script setup>
import { Combobox, defineMessages, injectNotificationManager, useVIntl } from '@modrinth/ui'
import { computed, ref, watch } from 'vue'

import { get_gpu_status, set_gpu_preference } from '@/helpers/jre'

const { handleError } = injectNotificationManager()
const { formatMessage } = useVIntl()

const messages = defineMessages({
	label: {
		id: 'app.settings.java.gpu.label',
		defaultMessage: 'Graphics adapter',
	},
	choiceAuto: {
		id: 'app.settings.java.gpu.choice.auto',
		defaultMessage: 'Let Windows decide',
	},
	choiceHighPerformance: {
		id: 'app.settings.java.gpu.choice.high-performance',
		defaultMessage: 'High performance',
	},
	choicePowerSaving: {
		id: 'app.settings.java.gpu.choice.power-saving',
		defaultMessage: 'Power saving',
	},
})

const props = defineProps({
	/** Full path to the runtime's executable; the preference is keyed on it. */
	path: {
		type: String,
		default: '',
	},
})

const status = ref(null)
const saving = ref(false)

// The wording follows Windows' own graphics settings, because that is the other
// place this same value can be changed and the two should not disagree.
const CHOICES = [
	{ value: 'auto', label: messages.choiceAuto },
	{ value: 'high_performance', label: messages.choiceHighPerformance },
	{ value: 'power_saving', label: messages.choicePowerSaving },
]

async function refresh() {
	if (!props.path) {
		status.value = null
		return
	}
	try {
		status.value = await get_gpu_status(props.path)
	} catch (error) {
		handleError(error)
		status.value = null
	}
}

watch(() => props.path, refresh, { immediate: true })

async function choose(preference) {
	if (!status.value || saving.value || preference === status.value.preference) return
	saving.value = true
	try {
		status.value = await set_gpu_preference(props.path, preference)
	} catch (error) {
		handleError(error)
		await refresh()
	} finally {
		saving.value = false
	}
}

// With a single adapter there is no choice to make, so the row is not shown
// at all rather than shown disabled.
const shown = computed(() => status.value?.supported && !!props.path && hasTwoAdapters.value)

const discrete = computed(() => status.value?.adapters?.find((a) => a.likely_discrete))
const hasTwoAdapters = computed(() => (status.value?.adapters?.length ?? 0) > 1)

const integrated = computed(() =>
	status.value?.adapters.find((adapter) => !adapter.likely_discrete),
)
const options = computed(() =>
	CHOICES.map(({ value, label }) => ({
		value,
		label: formatMessage(label),
		subLabel:
			value === 'high_performance'
				? discrete.value?.name
				: value === 'power_saving'
					? integrated.value?.name
					: undefined,
	})),
)
const selectedSubLabel = computed(
	() => options.value.find((option) => option.value === status.value?.preference)?.subLabel,
)
</script>

<template>
	<div
		v-if="shown"
		class="flex flex-wrap items-start justify-between gap-3 border-0 border-t border-solid border-surface-5 pt-3"
	>
		<span class="py-2 font-semibold text-contrast">{{ formatMessage(messages.label) }}</span>
		<div class="flex w-72 max-w-full flex-col gap-1">
			<Combobox
				:model-value="status.preference"
				:aria-label="formatMessage(messages.label)"
				:options="options"
				:disabled="saving"
				class="w-full"
				@update:model-value="choose"
			/>
			<div v-if="selectedSubLabel" class="px-1 text-xs text-secondary">
				{{ selectedSubLabel }}
			</div>
		</div>
	</div>
</template>
