<template>
	<NewModal ref="modal" :header="formatMessage(messages.header)">
		<div class="min-w-md flex max-w-md flex-col gap-3">
			<div class="flex flex-col gap-2">
				<label for="cc-name">
					<span class="text-lg font-semibold text-contrast">
						{{ formatMessage(messages.nameLabel) }}
						<span class="text-brand-red">*</span>
					</span>
				</label>
				<Input
					id="cc-name"
					v-model="name"
					:maxlength="64"
					:placeholder="formatMessage(messages.namePlaceholder)"
					autocomplete="off"
				/>
			</div>
			<div class="flex flex-col gap-2">
				<label for="cc-desc" class="flex flex-col gap-1">
					<span class="text-lg font-semibold text-contrast">
						{{ formatMessage(messages.summaryLabel) }}
					</span>
					<span>{{ formatMessage(messages.summaryHint) }}</span>
				</label>
				<Textarea
					id="cc-desc"
					v-model="description"
					:maxlength="256"
					:placeholder="formatMessage(messages.summaryPlaceholder)"
				/>
			</div>
			<p class="m-0">
				{{ formatMessage(messages.visibilityNote, { count: initialProjects.length }) }}
			</p>
			<div class="flex justify-end gap-2">
				<Button type="outlined" @click="hide">
					<XIcon aria-hidden="true" />
					{{ formatMessage(commonMessages.cancelButton) }}
				</Button>
				<Button :disabled="submitting || !name.trim()" type="colored" color="brand" @click="submit">
					<SpinnerIcon v-if="submitting" class="animate-spin" aria-hidden="true" />
					<PlusIcon v-else aria-hidden="true" />
					{{ formatMessage(submitting ? messages.creating : messages.create) }}
				</Button>
			</div>
		</div>
	</NewModal>
</template>

<script setup lang="ts">
import { PlusIcon, SpinnerIcon, XIcon } from '@modrinth/assets'
import {
	Button,
	commonMessages,
	defineMessages,
	injectNotificationManager,
	Input,
	NewModal,
	Textarea,
	useVIntl,
} from '@modrinth/ui'
import { ref } from 'vue'

import { type Collection, createCollection } from '@/helpers/modrinth-api'

const { formatMessage } = useVIntl()
const { handleError, addNotification } = injectNotificationManager()

const messages = defineMessages({
	header: { id: 'app.collection.create.header', defaultMessage: 'Creating a collection' },
	nameLabel: { id: 'app.collection.create.name', defaultMessage: 'Name' },
	namePlaceholder: {
		id: 'app.collection.create.name.placeholder',
		defaultMessage: 'Enter collection name...',
	},
	summaryLabel: { id: 'app.collection.create.summary', defaultMessage: 'Summary' },
	summaryHint: {
		id: 'app.collection.create.summary.hint',
		defaultMessage: 'A sentence or two that describes your collection.',
	},
	summaryPlaceholder: {
		id: 'app.collection.create.summary.placeholder',
		defaultMessage: 'This is a collection of...',
	},
	visibilityNote: {
		id: 'app.collection.create.visibility-note',
		defaultMessage:
			'Your new collection will be created as a public collection with {count, plural, =0 {no projects} one {# project} other {# projects}}.',
	},
	create: { id: 'app.collection.create.submit', defaultMessage: 'Create collection' },
	creating: { id: 'app.collection.create.submitting', defaultMessage: 'Creating...' },
	created: { id: 'app.collection.create.created', defaultMessage: 'Collection created' },
	createdDetail: {
		id: 'app.collection.create.created.detail',
		defaultMessage: 'Created “{name}”.',
	},
})

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
			title: formatMessage(messages.created),
			text: formatMessage(messages.createdDetail, { name: collection.name }),
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
