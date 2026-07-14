<script setup lang="ts">
import {
	ChevronLeftIcon,
	ChevronRightIcon,
	ClipboardCopyIcon,
	FolderOpenIcon,
	ImageIcon,
	SortAscIcon,
	SortDescIcon,
	SpinnerIcon,
	TrashIcon,
	UpdatedIcon,
	XIcon,
} from '@modrinth/assets'
import {
	ButtonStyled,
	commonMessages,
	ConfirmModal,
	defineMessages,
	injectNotificationManager,
	useVIntl,
} from '@modrinth/ui'
import { convertFileSrc } from '@tauri-apps/api/core'
import dayjs from 'dayjs'
import { computed, onMounted, onUnmounted, ref } from 'vue'

import ContextMenu from '@/components/ui/ContextMenu.vue'
import { get_full_path } from '@/helpers/instance'
import {
	copyScreenshotToClipboard,
	deleteScreenshot,
	listInstanceScreenshots,
	type Screenshot,
} from '@/helpers/screenshots'
import type { GameInstance } from '@/helpers/types'
import { highlightInFolder, openPath } from '@/helpers/utils'

const props = defineProps<{
	instance: GameInstance
}>()

const { formatMessage } = useVIntl()
const { addNotification, handleError } = injectNotificationManager()

const messages = defineMessages({
	count: {
		id: 'app.instance.screenshots.count',
		defaultMessage: '{count, plural, one {# screenshot} other {# screenshots}}',
	},
	sortNewest: { id: 'app.instance.screenshots.sort-newest', defaultMessage: 'Newest first' },
	sortOldest: { id: 'app.instance.screenshots.sort-oldest', defaultMessage: 'Oldest first' },
	openFolderButton: {
		id: 'app.instance.screenshots.open-folder-button',
		defaultMessage: 'Open folder',
	},
	refresh: { id: 'app.instance.screenshots.refresh', defaultMessage: 'Refresh' },
	emptyTitle: { id: 'app.instance.screenshots.empty.title', defaultMessage: 'No screenshots yet' },
	emptyDescription: {
		id: 'app.instance.screenshots.empty.description',
		defaultMessage: 'Press F2 while playing this instance to capture a screenshot.',
	},
	copy: { id: 'app.instance.screenshots.copy', defaultMessage: 'Copy image' },
	copied: {
		id: 'app.instance.screenshots.copied',
		defaultMessage: 'Screenshot copied to clipboard',
	},
	copyFailed: {
		id: 'app.instance.screenshots.copy-failed',
		defaultMessage: 'Failed to copy screenshot',
	},
	openFolder: { id: 'app.instance.screenshots.reveal', defaultMessage: 'Show in folder' },
	previous: { id: 'app.instance.screenshots.previous', defaultMessage: 'Previous' },
	next: { id: 'app.instance.screenshots.next', defaultMessage: 'Next' },
	deleteTitle: {
		id: 'app.instance.screenshots.delete.title',
		defaultMessage: 'Delete this screenshot?',
	},
	deleteDescription: {
		id: 'app.instance.screenshots.delete.description',
		defaultMessage: 'This will permanently delete the screenshot from disk. This cannot be undone.',
	},
})

const screenshots = ref<Screenshot[]>([])
const loading = ref(true)
const refreshing = ref(false)
const sortAscending = ref(false)
const hoveredShot = ref<Screenshot | null>(null)
const lightboxIndex = ref<number | null>(null)
const zoom = ref(1)
const panX = ref(0)
const panY = ref(0)

const deleteModal = ref<InstanceType<typeof ConfirmModal>>()
const contextMenu = ref<InstanceType<typeof ContextMenu>>()
const pendingDelete = ref<Screenshot | null>(null)

const MIN_ZOOM = 1
const MAX_ZOOM = 6

const orderedScreenshots = computed(() =>
	sortAscending.value ? [...screenshots.value].reverse() : screenshots.value,
)

const lightboxScreenshot = computed(() =>
	lightboxIndex.value === null ? null : (orderedScreenshots.value[lightboxIndex.value] ?? null),
)

const isZoomed = computed(() => zoom.value > 1)

async function loadScreenshots() {
	// Only show the full-page spinner on the very first load. Manual refreshes
	// keep the existing grid on screen and just swap the data in, so the UI
	// never flickers/collapses.
	const initial = screenshots.value.length === 0
	if (initial) loading.value = true
	else refreshing.value = true
	try {
		screenshots.value = await listInstanceScreenshots(props.instance.path)
	} catch (error) {
		handleError(error as Error)
	} finally {
		loading.value = false
		refreshing.value = false
	}
}

function srcFor(shot: Screenshot) {
	return convertFileSrc(shot.path)
}

function formatTimestamp(modified: number) {
	return modified ? dayjs(modified).format('MMM D, YYYY HH:mm') : ''
}

function formatSize(bytes: number) {
	if (!bytes) return ''
	const mb = bytes / (1024 * 1024)
	if (mb >= 1) return `${mb.toFixed(1)} MB`
	return `${Math.max(1, Math.round(bytes / 1024))} KB`
}

function resetView() {
	zoom.value = 1
	panX.value = 0
	panY.value = 0
}

function openLightbox(shot: Screenshot) {
	lightboxIndex.value = orderedScreenshots.value.findIndex((s) => s.id === shot.id)
	resetView()
}

function closeLightbox() {
	lightboxIndex.value = null
}

function showPrevious() {
	if (lightboxIndex.value === null) return
	const total = orderedScreenshots.value.length
	lightboxIndex.value = (lightboxIndex.value - 1 + total) % total
	resetView()
}

function showNext() {
	if (lightboxIndex.value === null) return
	const total = orderedScreenshots.value.length
	lightboxIndex.value = (lightboxIndex.value + 1) % total
	resetView()
}

async function copyShot(shot: Screenshot | null) {
	if (!shot) return
	try {
		await copyScreenshotToClipboard(shot.path)
		addNotification({ title: formatMessage(messages.copied), type: 'success' })
	} catch (error) {
		addNotification({
			title: formatMessage(messages.copyFailed),
			text: error instanceof Error ? error.message : String(error),
			type: 'error',
		})
	}
}

async function revealShot(shot: Screenshot | null) {
	if (!shot) return
	await highlightInFolder(shot.path).catch((error) => handleError(error as Error))
}

async function openScreenshotsFolder() {
	try {
		const fullPath = await get_full_path(props.instance.path)
		await openPath(`${fullPath}/screenshots`)
	} catch (error) {
		handleError(error as Error)
	}
}

function requestDelete(shot: Screenshot | null) {
	if (!shot) return
	pendingDelete.value = shot
	deleteModal.value?.show()
}

async function confirmDelete() {
	const shot = pendingDelete.value
	if (!shot) return
	try {
		await deleteScreenshot(shot.path)
		const wasOpen = lightboxScreenshot.value?.id === shot.id
		screenshots.value = screenshots.value.filter((s) => s.id !== shot.id)

		if (wasOpen) {
			if (orderedScreenshots.value.length === 0) {
				closeLightbox()
			} else if (lightboxIndex.value !== null) {
				lightboxIndex.value = Math.min(lightboxIndex.value, orderedScreenshots.value.length - 1)
				resetView()
			}
		}
	} catch (error) {
		handleError(error as Error)
	} finally {
		pendingDelete.value = null
	}
}

function setZoom(next: number) {
	zoom.value = Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, Number(next.toFixed(2))))
	if (zoom.value === 1) {
		panX.value = 0
		panY.value = 0
	}
}

function zoomIn() {
	setZoom(zoom.value + 0.5)
}

function zoomOut() {
	setZoom(zoom.value - 0.5)
}

function onWheel(event: WheelEvent) {
	event.preventDefault()
	setZoom(zoom.value + (event.deltaY < 0 ? 0.25 : -0.25))
}

function toggleZoom() {
	if (isZoomed.value) {
		resetView()
	} else {
		setZoom(2.5)
	}
}

const dragging = ref(false)
let dragStartX = 0
let dragStartY = 0

function onPointerDown(event: PointerEvent) {
	if (!isZoomed.value) return
	dragging.value = true
	dragStartX = event.clientX - panX.value
	dragStartY = event.clientY - panY.value
	;(event.target as HTMLElement).setPointerCapture?.(event.pointerId)
}

function onPointerMove(event: PointerEvent) {
	if (!dragging.value) return
	panX.value = event.clientX - dragStartX
	panY.value = event.clientY - dragStartY
}

function onPointerUp() {
	dragging.value = false
}

function openContextMenu(event: MouseEvent, shot: Screenshot) {
	contextMenu.value?.showMenu(event, shot, [
		{ name: 'copy' },
		{ name: 'reveal' },
		{ type: 'divider' },
		{ name: 'delete', color: 'danger' },
	])
}

function onContextOption({ item, option }: { item: Screenshot; option: string }) {
	if (option === 'copy') void copyShot(item)
	else if (option === 'reveal') void revealShot(item)
	else if (option === 'delete') requestDelete(item)
}

function onKeydown(event: KeyboardEvent) {
	// Shortcuts are matched on `event.code` (physical key) rather than
	// `event.key`, so they work on non-Latin keyboard layouts too — e.g. on a
	// Russian layout Ctrl+C reports `event.key === 'с'` (Cyrillic), which would
	// otherwise never match a `'c'` check.

	// Ctrl/Cmd+C copies the open screenshot, or — when browsing the grid — the
	// one the cursor is hovering. Only intercept when there is actually a target
	// so normal text copying elsewhere is never blocked.
	if ((event.ctrlKey || event.metaKey) && event.code === 'KeyC') {
		const target = lightboxScreenshot.value ?? hoveredShot.value
		if (target) {
			event.preventDefault()
			void copyShot(target)
		}
		return
	}

	if (lightboxIndex.value === null) return

	switch (event.code) {
		case 'Escape':
			closeLightbox()
			break
		case 'ArrowLeft':
			showPrevious()
			break
		case 'ArrowRight':
			showNext()
			break
		case 'Equal':
		case 'NumpadAdd':
			zoomIn()
			break
		case 'Minus':
		case 'NumpadSubtract':
			zoomOut()
			break
		case 'Digit0':
		case 'Numpad0':
			resetView()
			break
		case 'Delete':
			requestDelete(lightboxScreenshot.value)
			break
	}
}

onMounted(() => {
	window.addEventListener('keydown', onKeydown)
	void loadScreenshots()
})

onUnmounted(() => {
	window.removeEventListener('keydown', onKeydown)
})
</script>

<template>
	<div class="flex flex-col gap-4">
		<ConfirmModal
			ref="deleteModal"
			:title="formatMessage(messages.deleteTitle)"
			:description="formatMessage(messages.deleteDescription)"
			:proceed-label="formatMessage(commonMessages.deleteLabel)"
			@proceed="confirmDelete"
		/>

		<div v-if="loading" class="flex h-64 items-center justify-center text-secondary">
			<SpinnerIcon class="size-8 animate-spin" />
		</div>

		<div
			v-else-if="screenshots.length === 0"
			class="flex flex-col items-center justify-center gap-3 rounded-2xl bg-bg-raised px-6 py-16 text-center"
		>
			<ImageIcon class="size-16 text-secondary opacity-60" />
			<h3 class="m-0 text-xl font-bold text-contrast">
				{{ formatMessage(messages.emptyTitle) }}
			</h3>
			<p class="m-0 max-w-md text-secondary">
				{{ formatMessage(messages.emptyDescription) }}
			</p>
			<ButtonStyled>
				<button @click="openScreenshotsFolder">
					<FolderOpenIcon />
					{{ formatMessage(messages.openFolderButton) }}
				</button>
			</ButtonStyled>
		</div>

		<template v-else>
			<div class="flex flex-wrap items-center justify-between gap-3">
				<span class="text-sm font-semibold text-secondary">
					{{ formatMessage(messages.count, { count: screenshots.length }) }}
				</span>
				<div class="flex items-center gap-2">
					<ButtonStyled>
						<button
							v-tooltip="formatMessage(sortAscending ? messages.sortOldest : messages.sortNewest)"
							@click="sortAscending = !sortAscending"
						>
							<SortDescIcon v-if="!sortAscending" />
							<SortAscIcon v-else />
							{{ formatMessage(sortAscending ? messages.sortOldest : messages.sortNewest) }}
						</button>
					</ButtonStyled>
					<ButtonStyled>
						<button @click="openScreenshotsFolder">
							<FolderOpenIcon />
							{{ formatMessage(messages.openFolderButton) }}
						</button>
					</ButtonStyled>
					<ButtonStyled circular>
						<button
							v-tooltip="formatMessage(messages.refresh)"
							:disabled="refreshing"
							@click="loadScreenshots"
						>
							<SpinnerIcon v-if="refreshing" class="animate-spin" />
							<UpdatedIcon v-else />
						</button>
					</ButtonStyled>
				</div>
			</div>

			<div
				class="grid grid-cols-2 gap-3 sm:grid-cols-3 min-[1100px]:grid-cols-4 min-[1500px]:grid-cols-5"
			>
				<button
					v-for="shot in orderedScreenshots"
					:key="shot.id"
					type="button"
					class="group relative aspect-video w-full cursor-pointer overflow-hidden rounded-xl border border-solid border-divider bg-bg-raised p-0 transition-[transform,box-shadow,border-color] duration-200 hover:-translate-y-0.5 hover:border-brand hover:shadow-lg focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-brand"
					@click="openLightbox(shot)"
					@contextmenu.prevent="openContextMenu($event, shot)"
					@mouseenter="hoveredShot = shot"
					@mouseleave="hoveredShot?.id === shot.id && (hoveredShot = null)"
					@focus="hoveredShot = shot"
					@blur="hoveredShot?.id === shot.id && (hoveredShot = null)"
				>
					<img
						:src="srcFor(shot)"
						alt=""
						loading="lazy"
						class="h-full w-full object-cover transition-transform duration-300 group-hover:scale-105"
					/>
					<div
						class="pointer-events-none absolute inset-x-0 bottom-0 flex flex-col gap-0.5 bg-gradient-to-t from-black/80 to-transparent px-3 pb-2 pt-6 text-left opacity-0 transition-opacity duration-200 group-hover:opacity-100"
					>
						<span class="truncate text-xs font-medium text-white/90">
							{{ formatTimestamp(shot.modified) }}
						</span>
					</div>
				</button>
			</div>
		</template>

		<Teleport to="body">
			<div
				v-if="lightboxScreenshot"
				class="fixed inset-0 z-[90] flex flex-col bg-black/90 backdrop-blur-sm"
				@click.self="closeLightbox"
			>
				<div
					class="relative z-10 flex items-center justify-between gap-3 bg-gradient-to-b from-black/70 to-transparent p-4 pb-8"
					@click.stop
				>
					<div class="flex min-w-0 flex-col">
						<span class="truncate text-base font-semibold text-white">
							{{ lightboxScreenshot.fileName }}
						</span>
						<span class="truncate text-sm text-white/60">
							{{ formatTimestamp(lightboxScreenshot.modified) }}
							<template v-if="lightboxScreenshot.size">
								· {{ formatSize(lightboxScreenshot.size) }}
							</template>
						</span>
					</div>
					<div
						v-if="orderedScreenshots.length > 1"
						class="absolute left-1/2 top-4 -translate-x-1/2 rounded-full bg-white/10 px-3 py-1 text-sm font-semibold tabular-nums text-white"
					>
						{{ (lightboxIndex ?? 0) + 1 }} / {{ orderedScreenshots.length }}
					</div>
					<div class="flex shrink-0 items-center gap-1.5">
						<ButtonStyled circular>
							<button
								v-tooltip="formatMessage(messages.copy)"
								@click="copyShot(lightboxScreenshot)"
							>
								<ClipboardCopyIcon />
							</button>
						</ButtonStyled>
						<ButtonStyled circular>
							<button
								v-tooltip="formatMessage(messages.openFolder)"
								@click="revealShot(lightboxScreenshot)"
							>
								<FolderOpenIcon />
							</button>
						</ButtonStyled>
						<ButtonStyled circular color="red">
							<button
								v-tooltip="formatMessage(commonMessages.deleteLabel)"
								@click="requestDelete(lightboxScreenshot)"
							>
								<TrashIcon />
							</button>
						</ButtonStyled>
						<ButtonStyled circular>
							<button v-tooltip="formatMessage(commonMessages.closeButton)" @click="closeLightbox">
								<XIcon />
							</button>
						</ButtonStyled>
					</div>
				</div>

				<div
					class="relative flex min-h-0 flex-1 items-center justify-center overflow-hidden px-4 pb-4"
					@click.self="closeLightbox"
					@wheel="onWheel"
				>
					<button
						v-if="orderedScreenshots.length > 1"
						type="button"
						class="absolute left-4 z-10 flex size-12 cursor-pointer items-center justify-center rounded-full border-0 bg-white/10 text-white transition-colors hover:bg-white/20"
						:aria-label="formatMessage(messages.previous)"
						@click.stop="showPrevious"
					>
						<ChevronLeftIcon class="size-7" />
					</button>

					<img
						:key="lightboxScreenshot.id"
						:src="srcFor(lightboxScreenshot)"
						alt=""
						draggable="false"
						class="max-h-full max-w-full select-none rounded-lg object-contain shadow-2xl transition-transform duration-100"
						:style="{
							transform: `translate(${panX}px, ${panY}px) scale(${zoom})`,
						}"
						@click.stop
						@dblclick="toggleZoom"
						@contextmenu.prevent="openContextMenu($event, lightboxScreenshot)"
						@pointerdown="onPointerDown"
						@pointermove="onPointerMove"
						@pointerup="onPointerUp"
						@pointercancel="onPointerUp"
					/>

					<button
						v-if="orderedScreenshots.length > 1"
						type="button"
						class="absolute right-4 z-10 flex size-12 cursor-pointer items-center justify-center rounded-full border-0 bg-white/10 text-white transition-colors hover:bg-white/20"
						:aria-label="formatMessage(messages.next)"
						@click.stop="showNext"
					>
						<ChevronRightIcon class="size-7" />
					</button>

					<div
						v-if="isZoomed"
						class="pointer-events-none absolute bottom-4 left-1/2 z-10 -translate-x-1/2 rounded-full bg-black/50 px-3 py-1 text-sm font-medium text-white backdrop-blur"
					>
						{{ Math.round(zoom * 100) }}%
					</div>
				</div>
			</div>
		</Teleport>

		<ContextMenu ref="contextMenu" @option-clicked="onContextOption">
			<template #copy> <ClipboardCopyIcon /> {{ formatMessage(messages.copy) }} </template>
			<template #reveal> <FolderOpenIcon /> {{ formatMessage(messages.openFolder) }} </template>
			<template #delete> <TrashIcon /> {{ formatMessage(commonMessages.deleteLabel) }} </template>
		</ContextMenu>
	</div>
</template>
