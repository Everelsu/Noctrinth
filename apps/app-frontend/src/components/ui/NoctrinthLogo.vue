<script setup lang="ts">
/**
 * The wordmark in the title bar, which says when the app is busy.
 *
 * The launcher already counts what it is waiting on — route changes and
 * suspended pages both hold a loading token — but the only thing that showed it
 * was a bar the app keeps switched off. The mark is on screen at all times and
 * costs no layout, so it is the honest place for it: a light sweeps across the
 * lettering while something is in flight, in the accent colour, and stops when
 * nothing is.
 *
 * The sweep is a gradient masked by the wordmark itself, laid over the real
 * one, so it lights the glyphs rather than the box around them and needs no
 * second copy of the artwork to stay in step.
 */
import { injectLoadingState } from '@modrinth/ui'

import NoctrinthTextLogo from '@/assets/noctrinth-text.svg?component'

const loading = injectLoadingState()
</script>

<template>
	<span class="noctrinth-logo" :class="{ 'is-loading': loading.pending.value }">
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

.noctrinth-logo__sweep {
	position: absolute;
	inset: 0;
	opacity: 0;
	transition: opacity 200ms ease;

	background-image: linear-gradient(
		100deg,
		transparent 38%,
		var(--color-brand) 50%,
		transparent 62%
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
	animation: noctrinth-logo-sweep 1.4s ease-in-out infinite;
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
 * A sweep that repeats for as long as a load takes is exactly the kind of
 * motion that is worth stopping for anyone who has asked for less of it. The
 * mark still says something is happening, once, without moving.
 */
@media (prefers-reduced-motion: reduce) {
	.noctrinth-logo.is-loading .noctrinth-logo__sweep {
		animation: none;
		background-position: 50% 0;
		opacity: 0.6;
	}
}
</style>
