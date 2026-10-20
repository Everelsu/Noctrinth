<script setup lang="ts">
/**
 * Noctrinth's own: which instance a CurseForge project goes into, when the
 * page was not opened from one. The backend then picks the file that fits
 * that instance's game version and loader, or says none does.
 */
import { PlusIcon, SearchIcon } from '@modrinth/assets'
import {
	Avatar,
	Button,
	defineMessages,
	injectNotificationManager,
	Input,
	NewModal,
	useVIntl,
} from '@modrinth/ui'
import { computed, ref } from 'vue'

import { getInstanceIconUrl, list as listInstances } from '@/helpers/instance'
import type { GameInstance } from '@/helpers/types'

const { handleError } = injectNotificationManager()
const { formatMessage } = useVIntl()

const emit = defineEmits<{ pick: [instanceId: string] }>()

const messages = defineMessages({
	header: { id: 'app.curseforge.instance-picker.header', defaultMessage: 'Add to an instance' },
	search: { id: 'app.curseforge.instance-picker.search', defaultMessage: 'Search instances' },
	add: { id: 'app.curseforge.instance-picker.add', defaultMessage: 'Add' },
	empty: {
		id: 'app.curseforge.instance-picker.empty',
		defaultMessage: 'No instances to add it to yet.',
	},
})

const modal = ref<InstanceType<typeof NewModal> | null>(null)
const instances = ref<GameInstance[]>([])
const query = ref('')

const shown = computed(() => {
	const text = query.value.trim().toLowerCase()
	return instances.value.filter((instance) => !text || instance.name.toLowerCase().includes(text))
})

defineExpose({
	async show() {
		try {
			instances.value = await listInstances()
			query.value = ''
			modal.value?.show()
		} catch (err) {
			handleError(err)
		}
	},
})

function pick(instance: GameInstance) {
	modal.value?.hide()
	emit('pick', instance.id)
}
</script>

<template>
	<NewModal
		ref="modal"
		:header="formatMessage(messages.header)"
		width="34rem"
		max-width="calc(100vw - 2rem)"
		scrollable
	>
		<div class="flex flex-col gap-3">
			<Input
				v-model="query"
				:icon="SearchIcon"
				type="text"
				:placeholder="formatMessage(messages.search)"
				clearable
				wrapper-class="w-full"
			/>
			<p v-if="shown.length === 0" class="m-0 text-secondary">
				{{ formatMessage(messages.empty) }}
			</p>
			<ul class="m-0 flex list-none flex-col gap-2 p-0">
				<li
					v-for="instance in shown"
					:key="instance.id"
					class="flex items-center gap-3 rounded-2xl bg-surface-2 p-3"
				>
					<Avatar
						:src="getInstanceIconUrl(instance.icon_path)"
						:tint-by="instance.id"
						size="40px"
						:alt="instance.name"
					/>
					<div class="flex min-w-0 flex-1 flex-col">
						<span class="truncate font-semibold text-contrast">{{ instance.name }}</span>
						<span class="truncate text-sm capitalize text-secondary">
							{{ instance.loader }} {{ instance.game_version }}
						</span>
					</div>
					<Button type="outlined" color="brand" @click="pick(instance)">
						<PlusIcon aria-hidden="true" />
						{{ formatMessage(messages.add) }}
					</Button>
				</li>
			</ul>
		</div>
	</NewModal>
</template>
