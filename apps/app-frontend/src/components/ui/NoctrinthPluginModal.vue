<script setup lang="ts">
import { PlugIcon } from '@modrinth/assets'
import { Button, NewModal, ProgressBar } from '@modrinth/ui'
import { ref, watch } from 'vue'

import { activePluginModal, resolvePluginModal } from '@/helpers/noctrinth-plugins'

const modalRef = ref<InstanceType<typeof NewModal> | null>(null)

watch(
	activePluginModal,
	(next) => {
		if (next) modalRef.value?.show()
		else modalRef.value?.hide()
	},
	{ flush: 'post' },
)

function handleAction(actionId: string) {
	resolvePluginModal(actionId)
}

function handleHide() {
	resolvePluginModal(null)
}
</script>

<template>
	<NewModal
		ref="modalRef"
		max-width="34rem"
		:header="activePluginModal?.request.title ?? ''"
		:on-hide="handleHide"
	>
		<div v-if="activePluginModal" class="flex flex-col gap-4">
			<div class="flex items-center gap-1.5 text-xs text-secondary">
				<PlugIcon class="size-3.5 shrink-0" aria-hidden="true" />
				<span>{{ activePluginModal.pluginName }}</span>
			</div>

			<p v-if="activePluginModal.request.description" class="m-0 text-sm text-primary">
				{{ activePluginModal.request.description }}
			</p>

			<div
				v-if="activePluginModal.request.blocks?.length"
				class="flex max-h-[50vh] flex-col gap-3 overflow-y-auto"
			>
				<template v-for="(block, idx) in activePluginModal.request.blocks" :key="idx">
					<p v-if="block.type === 'text'" class="m-0 text-sm text-primary whitespace-pre-wrap">
						{{ block.text }}
					</p>
					<pre
						v-else-if="block.type === 'code'"
						class="m-0 overflow-x-auto rounded-xl border border-solid border-surface-5 bg-surface-2 p-3 font-mono text-xs text-primary"
					><code>{{ block.text }}</code></pre>
					<div v-else-if="block.type === 'progress'" class="flex flex-col gap-1.5">
						<span v-if="block.label" class="text-xs font-medium text-secondary">
							{{ block.label }}
						</span>
						<ProgressBar :progress="block.value" :max="block.max ?? 100" color="brand" />
					</div>
					<div
						v-else-if="block.type === 'list'"
						class="flex flex-col divide-y divide-surface-5 rounded-xl border border-solid border-surface-5 bg-surface-2"
					>
						<div
							v-for="(item, itemIdx) in block.items"
							:key="itemIdx"
							class="flex flex-col gap-0.5 px-3 py-2"
						>
							<span class="text-sm font-medium text-contrast">{{ item.title }}</span>
							<span v-if="item.subtitle" class="text-xs text-secondary">{{ item.subtitle }}</span>
						</div>
					</div>
				</template>
			</div>

			<div
				v-if="activePluginModal.request.actions?.length"
				class="flex flex-wrap justify-end gap-2 pt-2"
			>
				<Button
					v-for="action in activePluginModal.request.actions"
					:key="action.id"
					:type="action.color && action.color !== 'standard' ? 'colored' : 'base'"
					:color="action.color && action.color !== 'standard' ? action.color : undefined"
					@click="handleAction(action.id)"
				>
					{{ action.label }}
				</Button>
			</div>
		</div>
	</NewModal>
</template>
