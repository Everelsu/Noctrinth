<script setup lang="ts">
import { PlugIcon } from '@modrinth/assets'
import { Button, Input, NewModal, ProgressBar } from '@modrinth/ui'
import { ref, watch } from 'vue'

import { activePluginModal, resolvePluginModal } from '@/helpers/noctrinth-plugins'

const modalRef = ref<InstanceType<typeof NewModal> | null>(null)
const formValues = ref<Record<string, string>>({})

watch(
	activePluginModal,
	(next, prev) => {
		if (next) {
			if (!prev || prev.pluginId !== next.pluginId || prev.mode !== next.mode) {
				const initial: Record<string, string> = {}
				for (const block of next.request.blocks ?? []) {
					if (block.type === 'input') {
						initial[block.id] = block.value ?? ''
					}
				}
				formValues.value = initial
			} else {
				const merged = { ...formValues.value }
				for (const block of next.request.blocks ?? []) {
					if (block.type === 'input' && !(block.id in merged)) {
						merged[block.id] = block.value ?? ''
					}
				}
				formValues.value = merged
			}
			modalRef.value?.show()
		} else {
			modalRef.value?.hide()
		}
	},
	{ flush: 'post' },
)

function handleAction(actionId: string) {
	resolvePluginModal(actionId, { ...formValues.value })
}

function handleHide() {
	resolvePluginModal(null, { ...formValues.value })
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
				class="flex max-h-[55vh] flex-col gap-3 overflow-y-auto"
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
					<div v-else-if="block.type === 'input'" class="flex flex-col gap-1.5">
						<label v-if="block.label" class="text-xs font-medium text-contrast">
							{{ block.label }}
						</label>
						<textarea
							v-if="block.multiline"
							v-model="formValues[block.id]"
							rows="4"
							:placeholder="block.placeholder"
							class="w-full resize-y rounded-xl border border-solid border-surface-5 bg-surface-2 p-3 text-sm text-contrast outline-none focus:border-brand"
						/>
						<Input
							v-else
							v-model="formValues[block.id]"
							:type="block.secret ? 'password' : 'text'"
							:placeholder="block.placeholder"
						/>
					</div>
					<hr
						v-else-if="block.type === 'divider'"
						class="my-1 border-0 border-t border-solid border-surface-5"
					/>
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
