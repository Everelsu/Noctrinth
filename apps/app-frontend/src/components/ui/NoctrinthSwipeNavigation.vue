<template>
	<div
		v-if="direction"
		class="swipe-indicator pointer-events-none fixed top-1/2 z-[150] grid size-12 place-items-center rounded-full border border-solid"
		:class="[
			direction === 'back' ? 'left-0' : 'right-0',
			ready ? 'is-ready' : '',
			committed ? (direction === 'back' ? 'fly-back' : 'fly-forward') : '',
			available ? '' : 'is-blocked',
		]"
		:style="indicatorStyle"
		aria-hidden="true"
	>
		<ChevronLeftIcon v-if="direction === 'back'" class="size-6" />
		<ChevronRightIcon v-else class="size-6" />
	</div>
</template>

<script setup lang="ts">
/**
 * Two fingers across the touchpad go back and forward through the launcher's
 * pages, the way a browser does.
 *
 * The gesture itself is Firefox's SwipeTracker, ported in helpers/swipe: a
 * swipe commits at a quarter of its length or on a quick enough flick, a
 * release short of that springs back, and vertical scrolling and anything that
 * scrolls sideways itself are left alone. What is here is where it sends the
 * page and the arrow that grows out of the window's edge while it happens.
 */
import { ChevronLeftIcon, ChevronRightIcon } from '@modrinth/assets'
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'

import {
	createSwipeHandler,
	type SwipeConfig,
	type SwipeDirection,
} from '@/helpers/swipe/swipe-detector'

const router = useRouter()

const CONFIG: SwipeConfig = {
	enabled: true,
	sensitivity: 3,
	invertDirection: false,
	showIndicator: true,
	pageSlide: false,
}

const direction = ref<SwipeDirection | null>(null)
const progress = ref(0)
const committed = ref(false)
let hideTimer: ReturnType<typeof setTimeout> | undefined

/** Whether there is a page to go to that way; vue-router keeps both in history.state. */
function canGo(towards: SwipeDirection): boolean {
	const state = window.history.state as { back?: unknown; forward?: unknown } | null
	return towards === 'back' ? !!state?.back : !!state?.forward
}

const available = computed(() => !!direction.value && canGo(direction.value))
const ready = computed(() => available.value && progress.value >= 1)

const indicatorStyle = computed(() => {
	const p = progress.value
	// Slides in from beyond the edge to 16px inside it, growing as it comes.
	const offset = -56 + p * 72
	const signed = direction.value === 'back' ? offset : -offset
	return {
		transform: `translate(${signed}px, -50%) scale(${0.8 + p * 0.25})`,
		opacity: String(Math.min(1, 0.25 + p * 0.9)),
	}
})

const handler = createSwipeHandler({
	getConfig: () => CONFIG,
	navigate(towards) {
		if (!canGo(towards)) return
		if (towards === 'back') router.back()
		else router.forward()
	},
	onProgress(_doc, towards, amount) {
		clearTimeout(hideTimer)
		committed.value = false
		direction.value = towards
		progress.value = amount
	},
	onCommit() {
		committed.value = available.value
	},
	onEnd() {
		clearTimeout(hideTimer)
		hideTimer = setTimeout(
			() => {
				direction.value = null
				progress.value = 0
				committed.value = false
			},
			committed.value ? 260 : 120,
		)
	},
})

function onWheel(event: WheelEvent) {
	// Dialogs and the image viewer keep their own sideways gestures.
	if (document.querySelector('[aria-modal="true"], [data-modal-root]')) return
	handler.onWheel(event)
}

onMounted(() => {
	window.addEventListener('wheel', onWheel, { passive: false, capture: true })
})
onBeforeUnmount(() => {
	window.removeEventListener('wheel', onWheel, { capture: true })
	handler.dispose()
	clearTimeout(hideTimer)
})
</script>

<style scoped>
.swipe-indicator {
	margin-top: 0;
	background: var(--surface-3);
	border-color: var(--surface-5);
	color: var(--color-contrast);
	box-shadow: 0 10px 30px -10px rgb(0 0 0 / 0.6);
	transition:
		background-color 160ms ease-out,
		color 160ms ease-out,
		border-color 160ms ease-out,
		box-shadow 200ms ease-out;
	will-change: transform, opacity;
}
.swipe-indicator.is-ready {
	background: var(--color-brand);
	border-color: var(--color-brand);
	color: var(--color-accent-contrast);
	box-shadow:
		0 10px 30px -10px rgb(0 0 0 / 0.6),
		0 0 24px -2px var(--color-brand);
}
.swipe-indicator.is-blocked {
	color: var(--color-secondary);
}
.swipe-indicator.fly-back,
.swipe-indicator.fly-forward {
	transition:
		transform 240ms cubic-bezier(0.22, 1, 0.36, 1),
		opacity 240ms ease-out;
	opacity: 0 !important;
}
.swipe-indicator.fly-back {
	transform: translate(40px, -50%) scale(1.1) !important;
}
.swipe-indicator.fly-forward {
	transform: translate(-40px, -50%) scale(1.1) !important;
}
@media (prefers-reduced-motion: reduce) {
	.swipe-indicator,
	.swipe-indicator.fly-back,
	.swipe-indicator.fly-forward {
		transition: none;
	}
}
</style>
