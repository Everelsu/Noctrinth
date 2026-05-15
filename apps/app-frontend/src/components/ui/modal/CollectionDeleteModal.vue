<template>
	<ConfirmModal
		ref="modal"
		title="Are you sure you want to delete this collection?"
		description="This will permanently delete this collection. This action cannot be undone."
		:has-to-type="false"
		proceed-label="Delete"
		@proceed="confirm"
	/>
</template>

<script setup lang="ts">
import { ConfirmModal, injectNotificationManager } from '@modrinth/ui'
import { ref } from 'vue'

import { type Collection, deleteCollection } from '@/helpers/modrinth-api'

const { handleError, addNotification } = injectNotificationManager()

const emit = defineEmits<{
	deleted: [Collection]
}>()

const modal = ref<InstanceType<typeof ConfirmModal>>()
const collection = ref<Collection | null>(null)

function show(c: Collection, event?: MouseEvent) {
	collection.value = c
	modal.value?.show(event)
}

function hide() {
	modal.value?.hide()
}

async function confirm() {
	if (!collection.value) return
	const c = collection.value
	try {
		await deleteCollection(c.id)
		emit('deleted', c)
		addNotification({
			title: 'Collection deleted',
			text: `"${c.name}" was deleted.`,
			type: 'success',
		})
	} catch (e) {
		handleError(e)
	}
}

defineExpose({ show, hide })
</script>
