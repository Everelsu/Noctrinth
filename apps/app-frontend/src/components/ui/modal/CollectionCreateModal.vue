<template>
	<NewModal ref="modal" header="Creating a collection">
		<div class="min-w-md flex max-w-md flex-col gap-3">
			<div class="flex flex-col gap-2">
				<label for="cc-name">
					<span class="text-lg font-semibold text-contrast">
						Name
						<span class="text-brand-red">*</span>
					</span>
				</label>
				<StyledInput
					id="cc-name"
					v-model="name"
					:maxlength="64"
					placeholder="Enter collection name..."
					autocomplete="off"
				/>
			</div>
			<div class="flex flex-col gap-2">
				<label for="cc-desc" class="flex flex-col gap-1">
					<span class="text-lg font-semibold text-contrast">Summary</span>
					<span>A sentence or two that describes your collection.</span>
				</label>
				<StyledInput
					id="cc-desc"
					v-model="description"
					multiline
					:maxlength="256"
					placeholder="This is a collection of..."
				/>
			</div>
			<p class="m-0">
				Your new collection will be created as a public collection with
				{{ initialProjects.length === 0 ? 'no projects' : initialProjects.length === 1 ? '1 project' : `${initialProjects.length} projects` }}.
			</p>
			<div class="flex justify-end gap-2">
				<ButtonStyled type="outlined">
					<button @click="hide">
						<XIcon aria-hidden="true" />
						Cancel
					</button>
				</ButtonStyled>
				<ButtonStyled color="brand">
					<button :disabled="submitting || !name.trim()" @click="submit">
						<SpinnerIcon v-if="submitting" class="animate-spin" aria-hidden="true" />
						<PlusIcon v-else aria-hidden="true" />
						{{ submitting ? 'Creating...' : 'Create collection' }}
					</button>
				</ButtonStyled>
			</div>
		</div>
	</NewModal>
</template>

<script setup lang="ts">
import { PlusIcon, SpinnerIcon, XIcon } from '@modrinth/assets'
import {
	ButtonStyled,
	injectNotificationManager,
	NewModal,
	StyledInput,
} from '@modrinth/ui'
import { ref } from 'vue'

import { type Collection, createCollection } from '@/helpers/modrinth-api'

const { handleError, addNotification } = injectNotificationManager()

const emit = defineEmits<{
	created: [Collection]
}>()

const modal = ref<InstanceType<typeof NewModal>>()
const name = ref('')
const description = ref('')
const submitting = ref(false)
const initialProjects = ref<string[]>([])

function show(presetProjects: string[] = [], event?: MouseEvent) {
	name.value = ''
	description.value = ''
	initialProjects.value = presetProjects
	submitting.value = false
	modal.value?.show(event)
}

function hide() {
	modal.value?.hide()
}

async function submit() {
	const trimmed = name.value.trim()
	if (!trimmed) return
	submitting.value = true
	try {
		const collection = await createCollection({
			name: trimmed,
			description: description.value.trim() || undefined,
			projects: initialProjects.value,
		})
		emit('created', collection)
		addNotification({
			title: 'Collection created',
			text: `Created "${collection.name}".`,
			type: 'success',
		})
		hide()
	} catch (e) {
		handleError(e)
	} finally {
		submitting.value = false
	}
}

defineExpose({ show, hide })
</script>
