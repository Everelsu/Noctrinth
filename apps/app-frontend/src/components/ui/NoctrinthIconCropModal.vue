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
import { computed, onBeforeUnmount, ref, useTemplateRef } from 'vue'

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

const STAGE_SIZE = 280
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
const isDragging = ref(false)

let activeObjectUrl: string | null = null
let dragStartX = 0
let dragStartY = 0
let startOffsetX = 0
let startOffsetY = 0

function revokeObjectUrl() {
	if (activeObjectUrl) {
		URL.revokeObjectURL(activeObjectUrl)
		activeObjectUrl = null
	}
}

onBeforeUnmount(revokeObjectUrl)

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
	isDragging.value = true
	dragStartX = event.clientX
	dragStartY = event.clientY
	startOffsetX = offsetX.value
	startOffsetY = offsetY.value
	;(event.currentTarget as HTMLElement)?.setPointerCapture?.(event.pointerId)
}

function onPointerMove(event: PointerEvent) {
	if (!isDragging.value) return
	const clamped = clampOffsets(
		startOffsetX + (event.clientX - dragStartX),
		startOffsetY + (event.clientY - dragStartY),
	)
	offsetX.value = clamped.x
	offsetY.value = clamped.y
}

function onPointerUp(event: PointerEvent) {
	if (!isDragging.value) return
	isDragging.value = false
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

async function show(filePath: string) {
	revokeObjectUrl()
	sourcePath.value = filePath
	imageSrc.value = ''
	naturalWidth.value = 0
	naturalHeight.value = 0
	resetTransform()
	modal.value?.show()

	const assetUrl = convertFileSrc(filePath)
	try {
		const response = await fetch(assetUrl)
		if (!response.ok) throw new Error(`HTTP ${response.status}`)
		const blob = await response.blob()
		if (sourcePath.value !== filePath) return
		activeObjectUrl = URL.createObjectURL(blob)
		imageSrc.value = activeObjectUrl
	} catch {
		if (sourcePath.value === filePath) {
			imageSrc.value = assetUrl
		}
	}
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
	<NewModal
		ref="modal"
		:header="formatMessage(messages.header)"
		max-width="540px"
		:on-hide="revokeObjectUrl"
	>
		<div class="flex flex-col gap-5">
			<p class="m-0 text-sm text-secondary">
				{{ formatMessage(messages.hint) }}
			</p>

			<div
				class="flex flex-col items-center gap-4 rounded-2xl border border-solid border-surface-5 bg-surface-2 p-5"
			>
				<div
					class="crop-stage relative overflow-hidden rounded-2xl border border-solid border-surface-5 select-none touch-none cursor-grab active:cursor-grabbing shadow-md"
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
						crossorigin="anonymous"
						alt=""
						draggable="false"
						class="pointer-events-none absolute left-1/2 top-1/2 max-w-none select-none"
						:style="imageStyle"
						@load="onImageLoad"
					/>
					<div
						class="pointer-events-none absolute inset-0 grid grid-cols-3 grid-rows-3 transition-opacity duration-150"
						:class="isDragging ? 'opacity-50' : 'opacity-0'"
					>
						<div class="border-0 border-b border-r border-solid border-white/40" />
						<div class="border-0 border-b border-r border-solid border-white/40" />
						<div class="border-0 border-b border-solid border-white/40" />
						<div class="border-0 border-b border-r border-solid border-white/40" />
						<div class="border-0 border-b border-r border-solid border-white/40" />
						<div class="border-0 border-b border-solid border-white/40" />
						<div class="border-0 border-r border-solid border-white/40" />
						<div class="border-0 border-r border-solid border-white/40" />
						<div />
					</div>
					<div
						class="pointer-events-none absolute inset-0 rounded-2xl ring-1 ring-inset ring-white/20"
					/>
				</div>

				<div class="flex w-full items-center gap-3 pt-1">
					<span class="shrink-0 text-sm font-medium text-secondary">
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
					<span class="w-11 shrink-0 text-right font-mono text-xs text-secondary tabular-nums">
						{{ zoom.toFixed(1) }}×
					</span>
					<IconButton
						v-tooltip="formatMessage(messages.reset)"
						:label="formatMessage(messages.reset)"
						type="quiet"
						size="sm"
						@click="resetTransform"
					>
						<RotateCounterClockwiseIcon />
					</IconButton>
				</div>
			</div>

			<div
				class="flex flex-wrap items-center justify-between gap-3 border-0 border-t border-solid border-surface-5 pt-4"
			>
				<Button type="quiet" :disabled="saving" @click="applyOriginal">
					{{ formatMessage(messages.useOriginal) }}
				</Button>
				<div class="flex items-center gap-2.5">
					<Button :disabled="saving" @click="modal?.hide()">
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

<style scoped>
.crop-stage {
	background-color: var(--surface-3);
	background-image:
		linear-gradient(45deg, var(--surface-4) 25%, transparent 25%),
		linear-gradient(-45deg, var(--surface-4) 25%, transparent 25%),
		linear-gradient(45deg, transparent 75%, var(--surface-4) 75%),
		linear-gradient(-45deg, transparent 75%, var(--surface-4) 75%);
	background-size: 16px 16px;
	background-position:
		0 0,
		0 8px,
		8px -8px,
		-8px 0;
}
</style>
