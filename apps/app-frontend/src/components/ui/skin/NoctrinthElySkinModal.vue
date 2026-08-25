<!--
	Adding and editing an Ely.by skin.

	Deliberately a narrower version of `EditSkinModal`: Ely.by has no capes at
	all, and its edit form cannot replace a texture — a different texture is a
	different skin. What is left is the model, which is the one thing the site's
	own pages and this modal agree on.
-->
<template>
	<NewModal ref="modal" :on-hide="handleModalHide">
		<template #title>
			<span class="text-lg font-extrabold text-contrast">
				{{ formatMessage(mode === 'edit' ? messages.editSkinTitle : messages.addSkinTitle) }}
			</span>
		</template>

		<div class="flex flex-col md:flex-row gap-6">
			<div class="h-[25rem] w-[16rem] min-w-[16rem] flex-shrink-0 md:self-center">
				<SkinPreviewRenderer
					:variant="variant"
					:texture-src="previewTexture"
					framing="modal"
					:initial-rotation="Math.PI / 8"
					class="h-full w-full"
				/>
			</div>

			<div class="flex w-full min-w-52 flex-col gap-4 md:min-h-[20rem]">
				<section>
					<h2 class="text-base font-semibold mb-2">
						{{ formatMessage(messages.armStyleSection) }}
					</h2>
					<RadioButtons v-model="variant" :items="['CLASSIC', 'SLIM']" class="!flex-row flex-wrap">
						<template #default="{ item }">
							{{
								formatMessage(item === 'CLASSIC' ? messages.wideArmStyle : messages.slimArmStyle)
							}}
						</template>
					</RadioButtons>
				</section>

				<section v-if="mode === 'edit'" class="mt-auto">
					<h2 class="text-base font-semibold mb-2">{{ formatMessage(messages.textureSection) }}</h2>
					<p class="m-0 mb-2 leading-tight text-secondary">
						{{ formatMessage(messages.textureLocked) }}
					</p>
					<Button :disabled="saving" @click="uploadInstead">
						<UploadIcon /> {{ formatMessage(messages.uploadNewButton) }}
					</Button>
				</section>
			</div>
		</div>

		<template #actions>
			<div class="flex justify-end gap-2">
				<Button type="outlined" :disabled="saving" @click="hide">
					<XIcon />{{ formatMessage(commonMessages.cancelButton) }}
				</Button>
				<Button type="colored" color="brand" :disabled="saving || !hasChanges" @click="requestSave">
					<SpinnerIcon v-if="saving" class="animate-spin" />
					<CheckIcon v-else-if="mode === 'new'" />
					<SaveIcon v-else />
					{{ formatMessage(mode === 'new' ? messages.addSkinButton : messages.saveSkinButton) }}
				</Button>
			</div>
		</template>
	</NewModal>
</template>

<script setup lang="ts">
import { CheckIcon, SaveIcon, SpinnerIcon, UploadIcon, XIcon } from '@modrinth/assets'
import {
	Button,
	commonMessages,
	defineMessages,
	NewModal,
	RadioButtons,
	SkinPreviewRenderer,
	useVIntl,
} from '@modrinth/ui'
import { computed, ref, useTemplateRef } from 'vue'

import type { Skin, SkinModel } from '@/helpers/skins.ts'

const props = defineProps<{ saving?: boolean }>()

const emit = defineEmits<{
	save: [skin: Skin, isSlim: boolean]
	add: [isSlim: boolean]
	upload: []
}>()

const { formatMessage } = useVIntl()

const messages = defineMessages({
	editSkinTitle: {
		id: 'app.skins.modal.edit-title',
		defaultMessage: 'Editing skin',
	},
	addSkinTitle: {
		id: 'app.skins.modal.add-title',
		defaultMessage: 'Adding a skin',
	},
	armStyleSection: {
		id: 'app.skins.modal.arm-style-section',
		defaultMessage: 'Arm style',
	},
	wideArmStyle: {
		id: 'app.skins.modal.arm-style-wide',
		defaultMessage: 'Wide',
	},
	slimArmStyle: {
		id: 'app.skins.modal.arm-style-slim',
		defaultMessage: 'Slim',
	},
	textureSection: {
		id: 'app.skins.modal.texture-section',
		defaultMessage: 'Texture',
	},
	textureLocked: {
		id: 'app.skins.ely.edit.texture-locked',
		defaultMessage:
			'Ely.by keeps a texture and its skin together, so this one cannot be swapped out. Upload the new texture as another skin instead.',
	},
	uploadNewButton: {
		id: 'app.skins.ely.edit.upload-new',
		defaultMessage: 'Upload a new skin',
	},
	saveSkinButton: {
		id: 'app.skins.modal.save-skin-button',
		defaultMessage: 'Save skin',
	},
	addSkinButton: {
		id: 'app.skins.modal.add-skin-button',
		defaultMessage: 'Add skin',
	},
})

const modal = useTemplateRef<InstanceType<typeof NewModal>>('modal')
const mode = ref<'edit' | 'new'>('edit')
const skin = ref<Skin | null>(null)
const newTexture = ref('')
const variant = ref<SkinModel>('CLASSIC')
const originalVariant = ref<SkinModel>('CLASSIC')

const previewTexture = computed(() =>
	mode.value === 'new' ? newTexture.value : (skin.value?.texture ?? ''),
)

// Adding is always a change; editing only once the model actually differs.
const hasChanges = computed(() => mode.value === 'new' || variant.value !== originalVariant.value)

/** Only the two models exist on Ely.by, and UNKNOWN would select neither. */
function toModel(variant: SkinModel): SkinModel {
	return variant === 'SLIM' ? 'SLIM' : 'CLASSIC'
}

function show(event: MouseEvent, target: Skin) {
	mode.value = 'edit'
	skin.value = target
	newTexture.value = ''
	variant.value = toModel(target.variant)
	originalVariant.value = variant.value
	modal.value?.show(event)
}

/** The dialog for a texture that is about to be uploaded to Ely.by. */
function showNew(event: MouseEvent, texture: string, detected: SkinModel) {
	mode.value = 'new'
	skin.value = null
	newTexture.value = texture
	variant.value = toModel(detected)
	originalVariant.value = variant.value
	modal.value?.show(event)
}

function hide() {
	modal.value?.hide()
}

function handleModalHide() {
	if (props.saving) return
	skin.value = null
	newTexture.value = ''
}

function requestSave() {
	if (!hasChanges.value) return

	if (mode.value === 'new') {
		emit('add', variant.value === 'SLIM')
		return
	}

	if (!skin.value) return
	emit('save', skin.value, variant.value === 'SLIM')
}

function uploadInstead() {
	hide()
	emit('upload')
}

defineExpose({ show, showNew, hide })
</script>
