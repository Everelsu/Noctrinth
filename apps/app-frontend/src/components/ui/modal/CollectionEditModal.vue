<template>
	<NewModal ref="modal" header="Editing collection">
		<div class="flex w-[30rem] flex-col gap-3">
			<div class="flow-root">
				<div class="float-end ml-4">
					<Avatar
						:src="collection?.icon_url ?? undefined"
						:alt="collection?.name"
						:tint-by="collection?.id"
						size="108px"
						class="!border-4"
						no-shadow
					/>
				</div>
				<div class="overflow-hidden">
					<label class="mb-2 block text-lg font-semibold text-contrast" for="ce-title">
						Title
					</label>
					<StyledInput
						id="ce-title"
						v-model="title"
						:maxlength="64"
						autocomplete="off"
						wrapper-class="w-full"
					/>
				</div>
				<label
					class="mb-2 mt-4 block text-lg font-semibold text-contrast"
					for="ce-desc"
				>
					Description
				</label>
				<StyledInput
					id="ce-desc"
					v-model="description"
					multiline
					:maxlength="255"
					wrapper-class="h-24"
				/>
				<label
					for="ce-visibility"
					class="mb-2 mt-4 block text-lg font-semibold text-contrast"
				>
					Visibility
				</label>
				<RadioButtons
					id="ce-visibility"
					v-model="visibility"
					:items="['listed', 'unlisted', 'private']"
				>
					<template #default="{ item }">
						<span class="flex items-center gap-1">
							{{ item === 'listed' ? 'Public' : item === 'unlisted' ? 'Unlisted' : 'Private' }}
						</span>
					</template>
				</RadioButtons>
			</div>
			<div class="flex justify-end gap-2">
				<ButtonStyled>
					<button class="w-24" @click="hide">
						<XIcon aria-hidden="true" />
						Cancel
					</button>
				</ButtonStyled>
				<ButtonStyled color="brand">
					<button class="w-36" :disabled="saving || !title.trim()" @click="save">
						<SpinnerIcon v-if="saving" class="animate-spin" aria-hidden="true" />
						<SaveIcon v-else aria-hidden="true" />
						{{ saving ? 'Saving...' : 'Save' }}
					</button>
				</ButtonStyled>
			</div>
		</div>
	</NewModal>
</template>

<script setup lang="ts">
import { SaveIcon, SpinnerIcon, XIcon } from '@modrinth/assets'
import {
	Avatar,
	ButtonStyled,
	injectNotificationManager,
	NewModal,
	RadioButtons,
	StyledInput,
} from '@modrinth/ui'
import { ref } from 'vue'

import { type Collection, editCollection } from '@/helpers/modrinth-api'

const { handleError, addNotification } = injectNotificationManager()

const emit = defineEmits<{
	saved: [Collection]
}>()

const modal = ref<InstanceType<typeof NewModal>>()
const collection = ref<Collection | null>(null)
const title = ref('')
const description = ref('')
const visibility = ref<'listed' | 'unlisted' | 'private'>('private')
const saving = ref(false)

function show(c: Collection, event?: MouseEvent) {
	collection.value = c
	title.value = c.name
	description.value = c.description ?? ''
	visibility.value = (c.status as 'listed' | 'unlisted' | 'private') || 'private'
	modal.value?.show(event)
}

function hide() {
	modal.value?.hide()
}

async function save() {
	if (!collection.value) return
	const c = collection.value
	const payload: Record<string, unknown> = {}
	const newName = title.value.trim()
	if (newName && newName !== c.name) payload.name = newName
	const newDesc = description.value.trim()
	const currentDesc = c.description ?? ''
	if (newDesc !== currentDesc) payload.description = newDesc || null
	if (visibility.value !== c.status) payload.status = visibility.value
	if (Object.keys(payload).length === 0) {
		hide()
		return
	}
	saving.value = true
	try {
		await editCollection(c.id, payload)
		const updated: Collection = {
			...c,
			...(payload.name !== undefined ? { name: payload.name as string } : {}),
			...(payload.description !== undefined
				? { description: payload.description as string | null }
				: {}),
			...(payload.status !== undefined ? { status: payload.status as string } : {}),
			updated: new Date().toISOString(),
		}
		emit('saved', updated)
		addNotification({
			title: 'Collection saved',
			text: 'Your changes are saved.',
			type: 'success',
		})
		hide()
	} catch (e) {
		handleError(e)
	} finally {
		saving.value = false
	}
}

defineExpose({ show, hide })
</script>
