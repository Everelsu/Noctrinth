<script setup>
import { defineMessages, DropdownSelect, injectNotificationManager, useVIntl } from '@modrinth/ui'
import { computed, ref, watch } from 'vue'

import { get_gpu_status, set_gpu_preference } from '@/helpers/jre'

const { handleError } = injectNotificationManager()
const { formatMessage } = useVIntl()

const messages = defineMessages({
	label: {
		id: 'app.settings.java.gpu.label',
		defaultMessage: 'Graphics adapter',
	},
	onNamedAdapter: {
		id: 'app.settings.java.gpu.on-named-adapter',
		defaultMessage: 'Windows will run this runtime on {adapter}.',
	},
	onHighPerformance: {
		id: 'app.settings.java.gpu.on-high-performance',
		defaultMessage: 'Windows will run this runtime on the high-performance adapter.',
	},
	onPowerSaving: {
		id: 'app.settings.java.gpu.on-power-saving',
		defaultMessage:
			'Windows will run this runtime on the power-saving adapter — usually the integrated one, which is the slow option for Minecraft.',
	},
	noPreference: {
		id: 'app.settings.java.gpu.no-preference',
		defaultMessage:
			'No preference is set, so Windows picks — on a laptop that is usually the integrated adapter. Available: {adapters}',
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

const choiceValues = CHOICES.map((choice) => choice.value)
const choiceLabel = (value) => {
	const choice = CHOICES.find((c) => c.value === value)
	return choice ? formatMessage(choice.label) : value
}

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

/** What this setting actually means for this machine, in one line. */
const explanation = computed(() => {
	if (!status.value) return ''

	const names = status.value.adapters.map((a) => a.name).join(' · ')

	switch (status.value.preference) {
		case 'high_performance':
			return discrete.value
				? formatMessage(messages.onNamedAdapter, { adapter: discrete.value.name })
				: formatMessage(messages.onHighPerformance)
		case 'power_saving':
			return formatMessage(messages.onPowerSaving)
		default:
			return formatMessage(messages.noPreference, { adapters: names })
	}
})

const isSuboptimal = computed(
	() =>
		hasTwoAdapters.value &&
		status.value &&
		(status.value.preference === 'auto' || status.value.preference === 'power_saving'),
)
</script>

<template>
	<div v-if="shown" class="flex flex-col gap-1.5">
		<div class="flex items-center justify-between gap-4">
			<span class="font-semibold text-contrast">{{ formatMessage(messages.label) }}</span>
			<DropdownSelect
				:model-value="status.preference"
				name="gpu-preference"
				class="w-[14rem]"
				:options="choiceValues"
				:display-name="choiceLabel"
				:disabled="saving || !hasTwoAdapters"
				@update:model-value="choose"
			/>
		</div>
		<p class="m-0 text-xs leading-tight" :class="isSuboptimal ? 'text-orange' : 'text-secondary'">
			{{ explanation }}
		</p>
	</div>
</template>
