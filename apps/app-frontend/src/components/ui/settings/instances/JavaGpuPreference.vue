<script setup>
import { DropdownSelect, injectNotificationManager } from '@modrinth/ui'
import { computed, ref, watch } from 'vue'

import { get_gpu_status, set_gpu_preference } from '@/helpers/jre'

const { handleError } = injectNotificationManager()

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
	{ value: 'auto', label: 'Let Windows decide' },
	{ value: 'high_performance', label: 'High performance' },
	{ value: 'power_saving', label: 'Power saving' },
]

const choiceValues = CHOICES.map((choice) => choice.value)
const choiceLabel = (value) => CHOICES.find((c) => c.value === value)?.label ?? value

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

const shown = computed(() => status.value?.supported && props.path)

const discrete = computed(() => status.value?.adapters?.find((a) => a.likely_discrete))
const hasTwoAdapters = computed(() => (status.value?.adapters?.length ?? 0) > 1)

/** What this setting actually means for this machine, in one line. */
const explanation = computed(() => {
	if (!status.value) return ''

	if (!hasTwoAdapters.value) {
		return 'This machine only reports one graphics adapter, so there is nothing to switch between.'
	}

	const names = status.value.adapters.map((a) => a.name).join(' · ')

	switch (status.value.preference) {
		case 'high_performance':
			return discrete.value
				? `Windows will run this runtime on ${discrete.value.name}.`
				: 'Windows will run this runtime on the high-performance adapter.'
		case 'power_saving':
			return 'Windows will run this runtime on the power-saving adapter — usually the integrated one, which is the slow option for Minecraft.'
		default:
			return `No preference is set, so Windows picks — on a laptop that is usually the integrated adapter. Available: ${names}`
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
			<span class="font-semibold text-contrast">Graphics adapter</span>
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
