<template>
	<ConfirmModal
		ref="modal"
		:title="formatMessage(messages.title)"
		:description="formatMessage(messages.description)"
		:has-to-type="false"
		:proceed-label="formatMessage(commonMessages.deleteLabel)"
		@proceed="confirm"
	/>
</template>

<script setup lang="ts">
import {
	commonMessages,
	ConfirmModal,
	defineMessages,
	injectNotificationManager,
	useVIntl,
} from '@modrinth/ui'
import { ref } from 'vue'

import { type Collection, deleteCollection } from '@/helpers/modrinth-api'

const { handleError, addNotification } = injectNotificationManager()
const { formatMessage } = useVIntl()

const messages = defineMessages({
	title: {
		id: 'app.collection.delete-modal.title',
		defaultMessage: 'Are you sure you want to delete this collection?',
	},
	description: {
		id: 'app.collection.delete-modal.description',
		defaultMessage: 'This will permanently delete this collection. This action cannot be undone.',
	},
})

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
