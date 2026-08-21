<script setup lang="ts">
/**
 * The wordmark in the title bar, which says when the app is busy.
 *
 * The launcher already counts what it is waiting on — route changes and
 * suspended pages both hold a loading token — but the only thing that read that
 * count was a bar the app keeps switched off, so a page that took a moment
 * looked like a click that had not registered. The mark is on screen at all
 * times and costs no layout, so it says it instead: the knot turns, and a light
 * in the accent colour sweeps across the lettering.
 *
 * The sweep is a gradient masked by the wordmark itself, so it lights the
 * glyphs rather than the box around them, and needs no second copy of the
 * artwork to stay in step. The knot is the same SVG's own paths — every one of
 * them but the lettering — turned about the middle of the symbol.
 */
import { injectLoadingState } from '@modrinth/ui'
import { onBeforeUnmount, ref, watch } from 'vue'

import NoctrinthTextLogo from '@/assets/noctrinth-text.svg?component'

/**
 * How long the animation stays up once it has started.
 *
 * Most loads here finish in tens of milliseconds — fast enough that an
 * animation tied straight to the token would flicker, or never be seen at all,
 * which is what "is it even doing anything?" looks like from the outside. One
 * full turn is the smallest amount that reads as a turn.
 */
const MINIMUM_VISIBLE_MS = 1200

const loading = injectLoadingState()
const active = ref(false)

let startedAt = 0
let stopTimer: number | undefined

function clearStopTimer(): void {
	if (stopTimer !== undefined) {
		window.clearTimeout(stopTimer)
		stopTimer = undefined
	}
}

watch(
	() => loading.pending.value,
	(pending) => {
		clearStopTimer()

		if (pending) {
			if (!active.value) {
				startedAt = performance.now()
				active.value = true
			}
			return
		}

		const remaining = MINIMUM_VISIBLE_MS - (performance.now() - startedAt)
		if (remaining <= 0) {
			active.value = false
			return
		}

		stopTimer = window.setTimeout(() => {
			active.value = false
			stopTimer = undefined
		}, remaining)
	},
	{ immediate: true },
)

onBeforeUnmount(clearStopTimer)
</script>

<template>
	<span class="noctrinth-logo" :class="{ 'is-loading': active }">
		<NoctrinthTextLogo class="noctrinth-logo__mark" />
		<span class="noctrinth-logo__sweep" aria-hidden="true" />
	</span>
</template>

<style scoped lang="scss">
.noctrinth-logo {
	position: relative;
	display: inline-flex;
	align-items: center;
}

.noctrinth-logo__mark {
	height: 100%;
	width: auto;
}

/*
 * Everything but the first path is the knot; the first one is the lettering,
 * which keeps still while the symbol turns. The origin is the middle of the
 * symbol in the file's own coordinates, so the ribbons turn together as one
 * shape instead of each spinning about itself.
 */
.noctrinth-logo__mark :deep(path:not(:first-child)) {
	transform-box: view-box;
	transform-origin: 273px 273px;
}

.noctrinth-logo.is-loading .noctrinth-logo__mark :deep(path:not(:first-child)) {
	animation: noctrinth-logo-turn 1.2s cubic-bezier(0.65, 0.05, 0.36, 1) infinite;
}

.noctrinth-logo__sweep {
	position: absolute;
	inset: 0;
	opacity: 0;
	transition: opacity 200ms ease;

	background-image: linear-gradient(
		100deg,
		transparent 36%,
		var(--color-brand) 50%,
		transparent 64%
	);
	background-size: 300% 100%;
	background-repeat: no-repeat;
	background-position: 150% 0;

	// Masked by the wordmark, so the light lands on the letters themselves.
	mask-image: url('@/assets/noctrinth-text.svg');
	mask-size: contain;
	mask-repeat: no-repeat;
	mask-position: left center;
	-webkit-mask-image: url('@/assets/noctrinth-text.svg');
	-webkit-mask-size: contain;
	-webkit-mask-repeat: no-repeat;
	-webkit-mask-position: left center;
}

.noctrinth-logo.is-loading .noctrinth-logo__sweep {
	opacity: 1;
	animation: noctrinth-logo-sweep 1.2s ease-in-out infinite;
}

@keyframes noctrinth-logo-turn {
	from {
		transform: rotate(0deg);
	}
	to {
		transform: rotate(360deg);
	}
}

@keyframes noctrinth-logo-sweep {
	from {
		background-position: 150% 0;
	}
	to {
		background-position: -50% 0;
	}
}

/*
 * A turn that repeats for as long as a load takes is exactly the kind of motion
 * worth stopping for anyone who has asked for less of it. The mark still says
 * something is happening, once, without moving.
 */
@media (prefers-reduced-motion: reduce) {
	.noctrinth-logo.is-loading .noctrinth-logo__mark :deep(path:not(:first-child)) {
		animation: none;
	}

	.noctrinth-logo.is-loading .noctrinth-logo__sweep {
		animation: none;
		background-position: 50% 0;
		opacity: 0.6;
	}
}
</style>
