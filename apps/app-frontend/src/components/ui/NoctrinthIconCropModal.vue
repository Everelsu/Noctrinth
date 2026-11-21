<script setup lang="ts">
import { CheckIcon, RotateCounterClockwiseIcon, XIcon } from '@modrinth/assets'
import {
	Button,
	defineMessages,
	IconButton,
	injectNotificationManager,
	NewModal,
	Slider,
	useVIntl,
} from '@modrinth/ui'
import { convertFileSrc } from '@tauri-apps/api/core'
import { computed, ref, useTemplateRef } from 'vue'

import { cache_icon, edit_icon } from '@/helpers/instance'

const props = defineProps<{
	instanceId: string
}>()

const emit = defineEmits<{
	saved: [iconPath: string]
}>()

const { formatMessage } = useVIntl()
const { handleError } = injectNotificationManager()

const messages = defineMessages({
	header: { id: 'app.icon-crop.header', defaultMessage: 'Crop instance icon' },
	hint: {
		id: 'app.icon-crop.hint',
		defaultMessage: 'Drag to reposition and scroll or slide to zoom.',
	},
	zoom: { id: 'app.icon-crop.zoom', defaultMessage: 'Zoom' },
	reset: { id: 'app.icon-crop.reset', defaultMessage: 'Reset' },
	useOriginal: { id: 'app.icon-crop.use-original', defaultMessage: 'Use as-is' },
	cancel: { id: 'app.icon-crop.cancel', defaultMessage: 'Cancel' },
	apply: { id: 'app.icon-crop.apply', defaultMessage: 'Apply' },
})

const STAGE_SIZE = 240
const OUTPUT_SIZE = 256

const modal = useTemplateRef<InstanceType<typeof NewModal>>('modal')
const sourcePath = ref<string | null>(null)
const imageSrc = ref<string>('')
const imageEl = ref<HTMLImageElement | null>(null)
const naturalWidth = ref(0)
const naturalHeight = ref(0)
const zoom = ref(1)
const offsetX = ref(0)
const offsetY = ref(0)
const saving = ref(false)

let dragging = false
let dragStartX = 0
let dragStartY = 0
let startOffsetX = 0
let startOffsetY = 0

const baseScale = computed(() => {
	if (!naturalWidth.value || !naturalHeight.value) return 1
	return Math.max(STAGE_SIZE / naturalWidth.value, STAGE_SIZE / naturalHeight.value)
})

const displayWidth = computed(() => naturalWidth.value * baseScale.value * zoom.value)
const displayHeight = computed(() => naturalHeight.value * baseScale.value * zoom.value)

function clampOffsets(x: number, y: number) {
	const maxX = Math.max(0, (displayWidth.value - STAGE_SIZE) / 2)
	const maxY = Math.max(0, (displayHeight.value - STAGE_SIZE) / 2)
	return {
		x: Math.min(maxX, Math.max(-maxX, x)),
		y: Math.min(maxY, Math.max(-maxY, y)),
	}
}

function updateZoom(next: number) {
	zoom.value = Math.min(4, Math.max(1, next))
	const clamped = clampOffsets(offsetX.value, offsetY.value)
	offsetX.value = clamped.x
	offsetY.value = clamped.y
}

function resetTransform() {
	zoom.value = 1
	offsetX.value = 0
	offsetY.value = 0
}

function onImageLoad(event: Event) {
	const img = event.target as HTMLImageElement
	imageEl.value = img
	naturalWidth.value = img.naturalWidth
	naturalHeight.value = img.naturalHeight
	resetTransform()
}

function onPointerDown(event: PointerEvent) {
	if (saving.value) return
	dragging = true
	dragStartX = event.clientX
	dragStartY = event.clientY
	startOffsetX = offsetX.value
	startOffsetY = offsetY.value
	;(event.currentTarget as HTMLElement)?.setPointerCapture?.(event.pointerId)
}

function onPointerMove(event: PointerEvent) {
	if (!dragging) return
	const clamped = clampOffsets(
		startOffsetX + (event.clientX - dragStartX),
		startOffsetY + (event.clientY - dragStartY),
	)
	offsetX.value = clamped.x
	offsetY.value = clamped.y
}

function onPointerUp(event: PointerEvent) {
	if (!dragging) return
	dragging = false
	;(event.currentTarget as HTMLElement)?.releasePointerCapture?.(event.pointerId)
}

function onWheel(event: WheelEvent) {
	event.preventDefault()
	const delta = event.deltaY > 0 ? -0.12 : 0.12
	updateZoom(Math.round((zoom.value + delta) * 100) / 100)
}

const imageStyle = computed(() => ({
	width: `${displayWidth.value}px`,
	height: `${displayHeight.value}px`,
	transform: `translate(-50%, -50%) translate(${offsetX.value}px, ${offsetY.value}px)`,
}))

function show(filePath: string) {
	sourcePath.value = filePath
	imageSrc.value = convertFileSrc(filePath)
	naturalWidth.value = 0
	naturalHeight.value = 0
	resetTransform()
	modal.value?.show()
}

async function applyOriginal() {
	if (!sourcePath.value || saving.value) return
	saving.value = true
	try {
		await edit_icon(props.instanceId, sourcePath.value)
		emit('saved', sourcePath.value)
		modal.value?.hide()
	} catch (error) {
		handleError(error as Error)
	} finally {
		saving.value = false
	}
}

async function applyCrop() {
	if (!imageEl.value || !naturalWidth.value || !naturalHeight.value || saving.value) return
	saving.value = true
	try {
		const canvas = document.createElement('canvas')
		canvas.width = OUTPUT_SIZE
		canvas.height = OUTPUT_SIZE
		const ctx = canvas.getContext('2d')
		if (!ctx) throw new Error('Canvas context unavailable')

		ctx.imageSmoothingEnabled = true
		ctx.imageSmoothingQuality = 'high'

		const ratio = OUTPUT_SIZE / STAGE_SIZE
		const drawW = displayWidth.value * ratio
		const drawH = displayHeight.value * ratio
		const drawX = OUTPUT_SIZE / 2 + offsetX.value * ratio - drawW / 2
		const drawY = OUTPUT_SIZE / 2 + offsetY.value * ratio - drawH / 2
		ctx.drawImage(imageEl.value, drawX, drawY, drawW, drawH)

		const blob = await new Promise<Blob | null>((resolve) => canvas.toBlob(resolve, 'image/png'))
		if (!blob) throw new Error('Failed to encode cropped icon')

		const bytes = Array.from(new Uint8Array(await blob.arrayBuffer()))
		const cachedPath = await cache_icon(bytes)
		await edit_icon(props.instanceId, cachedPath)
		emit('saved', cachedPath)
		modal.value?.hide()
	} catch (error) {
		handleError(error as Error)
	} finally {
		saving.value = false
	}
}

defineExpose({ show })
</script>

<template>
	<NewModal ref="modal" :header="formatMessage(messages.header)" max-width="460px">
		<div class="flex flex-col gap-4">
			<p class="m-0 text-sm text-secondary">
				{{ formatMessage(messages.hint) }}
			</p>

			<div class="flex flex-col items-center gap-3">
				<div
					class="relative overflow-hidden rounded-2xl border border-solid border-surface-5 bg-surface-2 select-none touch-none cursor-grab active:cursor-grabbing"
					:style="{ width: `${STAGE_SIZE}px`, height: `${STAGE_SIZE}px` }"
					@pointerdown="onPointerDown"
					@pointermove="onPointerMove"
					@pointerup="onPointerUp"
					@pointercancel="onPointerUp"
					@wheel="onWheel"
				>
					<img
						v-if="imageSrc"
						:src="imageSrc"
						alt=""
						draggable="false"
						class="pointer-events-none absolute left-1/2 top-1/2 max-w-none select-none"
						:style="imageStyle"
						@load="onImageLoad"
					/>
					<div
						class="pointer-events-none absolute inset-0 rounded-2xl ring-1 ring-inset ring-white/15"
					/>
				</div>

				<div class="flex w-full items-center gap-3 rounded-xl bg-surface-2.5 px-3 py-2">
					<span class="shrink-0 text-xs font-medium text-secondary">
						{{ formatMessage(messages.zoom) }}
					</span>
					<Slider
						:model-value="zoom"
						:min="1"
						:max="4"
						:step="0.02"
						class="flex-1"
						@update:model-value="(v) => updateZoom(Number(v))"
					/>
					<span class="w-10 shrink-0 text-right font-mono text-xs text-secondary tabular-nums">
						{{ zoom.toFixed(1) }}×
					</span>
					<IconButton
						v-tooltip="formatMessage(messages.reset)"
						:label="formatMessage(messages.reset)"
						type="quiet"
						size="sm"
						@click="resetTransform"
					>
						<RotateCounterClockwiseIcon class="size-4" />
					</IconButton>
				</div>
			</div>

			<div class="flex items-center justify-between gap-2 pt-1">
				<Button type="quiet" :disabled="saving" @click="applyOriginal">
					{{ formatMessage(messages.useOriginal) }}
				</Button>
				<div class="flex items-center gap-2">
					<Button type="outlined" :disabled="saving" @click="modal?.hide()">
						<XIcon aria-hidden="true" />
						{{ formatMessage(messages.cancel) }}
					</Button>
					<Button
						type="colored"
						color="brand"
						:disabled="saving || !naturalWidth"
						@click="applyCrop"
					>
						<CheckIcon aria-hidden="true" />
						{{ formatMessage(messages.apply) }}
					</Button>
				</div>
			</div>
		</div>
	</NewModal>
</template>
