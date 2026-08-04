<script setup lang="ts">
import {
	CalendarIcon,
	ClipboardCopyIcon,
	ContractIcon,
	ExpandIcon,
	FolderOpenIcon,
	ImageIcon,
	LeftArrowIcon,
	RightArrowIcon,
	SortAscIcon,
	SortDescIcon,
	SpinnerIcon,
	TrashIcon,
	UpdatedIcon,
	XIcon,
} from '@modrinth/assets'
import {
	ButtonStyled,
	Card,
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
	zoom: { id: 'app.instance.screenshots.zoom', defaultMessage: 'Zoom' },
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
		screenshots.value = await listInstanceScreenshots(props.instance.id)
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

// Resolved once on mount: looking the instance path up on every click added a
// round trip to a button that already feels slow because it waits on the OS
// file manager.
const screenshotsFolder = ref<string | null>(null)

async function resolveScreenshotsFolder() {
	try {
		const fullPath = await get_full_path(props.instance.id)
		screenshotsFolder.value = `${fullPath}/screenshots`
	} catch (error) {
		handleError(error as Error)
	}
}

async function openScreenshotsFolder() {
	if (!screenshotsFolder.value) await resolveScreenshotsFolder()
	if (!screenshotsFolder.value) return
	await openPath(screenshotsFolder.value).catch((error) => handleError(error as Error))
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
	void resolveScreenshotsFolder()
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

			<div class="gallery">
				<Card v-for="shot in orderedScreenshots" :key="shot.id" class="gallery-item">
					<a
						role="button"
						:tabindex="0"
						:aria-label="shot.fileName"
						@click="openLightbox(shot)"
						@keydown.enter.prevent="openLightbox(shot)"
						@keydown.space.prevent="openLightbox(shot)"
						@contextmenu.prevent="openContextMenu($event, shot)"
						@mouseenter="hoveredShot = shot"
						@mouseleave="hoveredShot?.id === shot.id && (hoveredShot = null)"
						@focus="hoveredShot = shot"
						@blur="hoveredShot?.id === shot.id && (hoveredShot = null)"
					>
						<img :src="srcFor(shot)" :alt="shot.fileName" loading="lazy" class="gallery-image" />
					</a>
					<div class="gallery-body">
						<h3 class="truncate">{{ shot.fileName }}</h3>
					</div>
					<span class="gallery-time">
						<CalendarIcon />
						{{ formatTimestamp(shot.modified) }}
						<template v-if="shot.size"> · {{ formatSize(shot.size) }} </template>
					</span>
				</Card>
			</div>
		</template>

		<Teleport to="body">
			<div v-if="lightboxScreenshot" class="expanded-image-modal" @click="closeLightbox">
				<div class="content" @wheel="onWheel">
					<img
						:key="lightboxScreenshot.id"
						class="image"
						:src="srcFor(lightboxScreenshot)"
						:alt="lightboxScreenshot.fileName"
						draggable="false"
						:style="{
							transform: `translate(-50%, -50%) translate(${panX}px, ${panY}px) scale(${zoom})`,
						}"
						@click.stop
						@dblclick="toggleZoom"
						@contextmenu.prevent="openContextMenu($event, lightboxScreenshot)"
						@pointerdown="onPointerDown"
						@pointermove="onPointerMove"
						@pointerup="onPointerUp"
						@pointercancel="onPointerUp"
					/>

					<div class="floating" @click.stop>
						<div class="text">
							<h2>{{ lightboxScreenshot.fileName }}</h2>
							<p>
								{{ formatTimestamp(lightboxScreenshot.modified) }}
								<template v-if="lightboxScreenshot.size">
									· {{ formatSize(lightboxScreenshot.size) }}
								</template>
								<template v-if="orderedScreenshots.length > 1">
									· {{ (lightboxIndex ?? 0) + 1 }} / {{ orderedScreenshots.length }}
								</template>
								<template v-if="isZoomed"> · {{ Math.round(zoom * 100) }}% </template>
							</p>
						</div>
						<div class="controls">
							<div class="buttons">
								<ButtonStyled circular>
									<button
										v-tooltip="formatMessage(commonMessages.closeButton)"
										@click="closeLightbox"
									>
										<XIcon aria-hidden="true" />
									</button>
								</ButtonStyled>
								<ButtonStyled circular>
									<button
										v-tooltip="formatMessage(messages.copy)"
										@click="copyShot(lightboxScreenshot)"
									>
										<ClipboardCopyIcon aria-hidden="true" />
									</button>
								</ButtonStyled>
								<ButtonStyled circular>
									<button
										v-tooltip="formatMessage(messages.openFolder)"
										@click="revealShot(lightboxScreenshot)"
									>
										<FolderOpenIcon aria-hidden="true" />
									</button>
								</ButtonStyled>
								<ButtonStyled circular>
									<button v-tooltip="formatMessage(messages.zoom)" @click="toggleZoom">
										<ExpandIcon v-if="!isZoomed" aria-hidden="true" />
										<ContractIcon v-else aria-hidden="true" />
									</button>
								</ButtonStyled>
								<ButtonStyled v-if="orderedScreenshots.length > 1" circular>
									<button v-tooltip="formatMessage(messages.previous)" @click="showPrevious">
										<LeftArrowIcon aria-hidden="true" />
									</button>
								</ButtonStyled>
								<ButtonStyled v-if="orderedScreenshots.length > 1" circular>
									<button v-tooltip="formatMessage(messages.next)" @click="showNext">
										<RightArrowIcon aria-hidden="true" />
									</button>
								</ButtonStyled>
								<ButtonStyled circular color="red">
									<button
										v-tooltip="formatMessage(commonMessages.deleteLabel)"
										@click="requestDelete(lightboxScreenshot)"
									>
										<TrashIcon aria-hidden="true" />
									</button>
								</ButtonStyled>
							</div>
						</div>
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

<!--
	Grid and viewer deliberately mirror the project gallery
	(pages/project/Gallery.vue) so screenshots feel like the rest of the app,
	with the screenshot-only extras — zoom/pan, copy, reveal, delete — folded
	into the same floating control bar.
-->
<style scoped lang="scss">
.gallery {
	display: grid;
	grid-template-columns: repeat(auto-fill, minmax(20rem, 1fr));
	width: 100%;
	gap: 1rem;
}

.gallery-item {
	padding: 0;
	overflow: hidden;
	margin: 0;
	display: flex;
	flex-direction: column;

	a {
		cursor: pointer;
	}

	.gallery-image {
		width: 100%;
		aspect-ratio: 2/1;
		object-fit: cover;
		object-position: center;
		display: block;
	}

	.gallery-body {
		flex-grow: 1;
		padding: 1rem 1rem 0.5rem;

		h3 {
			margin: 0;
			font-size: 1rem;
		}
	}

	.gallery-time {
		display: flex;
		align-items: center;
		gap: 0.25rem;
		padding: 0 1rem 1rem;
		color: var(--color-secondary);
		font-size: 0.875rem;

		svg {
			width: 1rem;
			height: 1rem;
			flex-shrink: 0;
		}
	}
}

.expanded-image-modal {
	position: fixed;
	z-index: 90;
	overflow: auto;
	top: 0;
	left: 0;
	width: 100%;
	height: 100%;
	background-color: rgba(0, 0, 0, 0.7);
	display: flex;
	justify-content: center;
	align-items: center;

	.content {
		position: relative;
		width: calc(100vw - 2 * var(--gap-lg));
		height: calc(100vh - 2 * var(--gap-lg));
		overflow: hidden;

		.image {
			position: absolute;
			left: 50%;
			top: 50%;
			max-width: calc(100vw - 2 * var(--gap-lg));
			max-height: calc(100vh - 2 * var(--gap-lg));
			border-radius: var(--radius-lg);
			user-select: none;
			transition: transform 0.1s ease-out;
		}

		.floating {
			position: absolute;
			left: 50%;
			transform: translateX(-50%);
			bottom: var(--gap-md);
			display: flex;
			flex-direction: column;
			align-items: center;
			gap: var(--gap-md);
			transition: opacity 0.25s ease-in-out;
			opacity: 1;
			padding: 2rem 2rem 0 2rem;

			&:not(&:hover) {
				opacity: 0.4;

				.text {
					transform: translateY(2.5rem) scale(0.8);
					opacity: 0;
				}

				.controls {
					transform: translateY(0.25rem) scale(0.9);
				}
			}

			.text {
				display: flex;
				flex-direction: column;
				max-width: 40rem;
				transition:
					opacity 0.25s ease-in-out,
					transform 0.25s ease-in-out;
				text-shadow: 1px 1px 10px #000000d4;
				margin-bottom: 0.25rem;
				gap: 0.5rem;

				h2 {
					color: var(--dark-color-base);
					font-size: 1.25rem;
					text-align: center;
					margin: 0;
					word-break: break-all;
				}

				p {
					color: var(--dark-color-base);
					text-align: center;
					margin: 0;
				}
			}

			.controls {
				background-color: var(--color-raised-bg);
				padding: var(--gap-md);
				border-radius: var(--radius-md);
				transition:
					opacity 0.25s ease-in-out,
					transform 0.25s ease-in-out;
			}
		}
	}
}

.buttons {
	display: flex;
	gap: 0.5rem;
}
</style>
