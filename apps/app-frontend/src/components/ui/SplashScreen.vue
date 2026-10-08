<template>
	<div
		v-if="!doneLoading"
		ref="splash"
		class="splash-screen"
		:class="{ 'is-inverted': inverted, 'is-ready': themeReady }"
		data-tauri-drag-region
	>
		<div class="splash-cube" aria-hidden="true"></div>
		<div class="splash-glow" aria-hidden="true"></div>
		<div class="splash-content" data-tauri-drag-region>
			<NoctrinthAppLogo class="splash-logo" />
			<div
				class="splash-bar"
				role="progressbar"
				:aria-valuenow="realProgress ?? undefined"
				aria-valuemin="0"
				aria-valuemax="100"
			>
				<div
					class="splash-bar__fill"
					:class="{ 'is-real': realProgress !== null }"
					:style="
						realProgress !== null ? { transform: `scaleX(${realProgress / 100})` } : undefined
					"
				></div>
			</div>
			<span v-if="message" class="splash-message">{{ message }}</span>
		</div>
	</div>
</template>

<script setup>
/**
 * Drawn to stay smooth while the app it covers is starting up, which is when
 * the main thread has the least time to give it.
 *
 * So nothing on it moves through the main thread: the bar crawls on a CSS
 * animation of `transform`, and the exit is a Web Animation of `opacity` and
 * `transform`, both of which the compositor runs however busy the page
 * underneath is. The exit is not a Vue transition on purpose: that waits two
 * frames of the main thread before it starts, which at startup is exactly
 * where the page is busiest, and it froze there. Its colours come from
 * the document's own theme and accent rather than a theme class of its own,
 * so it is the same colour as what it uncovers, and the cube behind it is
 * grey so that the accent is the only colour on it.
 */
import { injectLoadingState } from '@modrinth/ui'
import { onMounted, ref, useTemplateRef, watch } from 'vue'
import { useRouter } from 'vue-router'

import NoctrinthAppLogo from '@/assets/modrinth_app.svg?component'
import { useAppEvent } from '@/composables/use-app-event'
import { rollSplashCubeInverted } from '@/helpers/noctrinth-splash-cube'
import { themeReady } from '@/helpers/noctrinth-window-reveal'
import { debugStartup } from '@/helpers/startup-debug'

const splash = useTemplateRef('splash')
/** Black on white or white on black: chosen in the flags tab, or now and then by chance. */
const inverted = rollSplashCubeInverted()
const doneLoading = ref(false)
/** Set only when there is a real fraction to show; the crawl stands in otherwise. */
const realProgress = ref(null)
const message = ref()

const MIN_DISPLAY_MS = 500
const FADE_MS = 420
const EASING = 'cubic-bezier(0.4, 0, 0.2, 1)'
const mountedAt = Date.now()

const loading = injectLoadingState()
const router = useRouter()
onMounted(() => debugStartup('Splash mounted'))

function onAfterLeave() {
	doneLoading.value = true
	debugStartup('Splash fade completed', { displayedMs: Date.now() - mountedAt })
	loading.setEnabled(true)
}

/** Fills the bar, fades out and grows the mark a little, all started this very task. */
function dismiss() {
	const root = splash.value
	if (!root?.animate) {
		onAfterLeave()
		return
	}

	const fill = root.querySelector('.splash-bar__fill')
	fill?.animate([{ transform: getComputedStyle(fill).transform }, { transform: 'scaleX(1)' }], {
		duration: 180,
		easing: 'ease-out',
		fill: 'forwards',
	})
	root
		.querySelector('.splash-content')
		?.animate([{ transform: 'scale(1)' }, { transform: 'scale(1.03)' }], {
			duration: FADE_MS,
			easing: EASING,
			fill: 'forwards',
		})
	root
		.animate([{ opacity: 1 }, { opacity: 0 }], {
			duration: FADE_MS,
			easing: EASING,
			fill: 'forwards',
		})
		.finished.then(onAfterLeave, onAfterLeave)
	debugStartup('Splash fade started', { displayedMs: Date.now() - mountedAt })
}

/**
 * When to go. Not the moment nothing is loading: at startup that is a gap of
 * half a second between the app state landing and the router mounting the
 * page, and leaving then faded the splash over an empty frame, came back for
 * the page's own loading, and faded again from fully opaque. So it waits for
 * the router to be ready, then for loading to stay quiet a little while, and
 * then goes once.
 */
const QUIET_MS = 180
let dismissing = false
let routerReady = false
let quietTimer = 0

function scheduleDismissal() {
	clearTimeout(quietTimer)
	if (dismissing || !routerReady || loading.barEnabled.value || loading.pending.value) return

	const elapsed = Date.now() - mountedAt
	const delay = Math.max(QUIET_MS, MIN_DISPLAY_MS - elapsed)
	debugStartup('Splash dismissal scheduled', { delayMs: delay, displayedMs: elapsed })
	quietTimer = window.setTimeout(() => {
		if (dismissing || loading.pending.value) return
		dismissing = true
		// One frame for the page that has just rendered to be painted under it.
		requestAnimationFrame(() => dismiss())
	}, delay)
}

watch([loading.barEnabled, loading.pending], ([barEnabled, pending]) => {
	debugStartup('Splash loading state changed', { barEnabled, pending })
	scheduleDismissal()
})

router.isReady().then(
	() => {
		routerReady = true
		scheduleDismissal()
	},
	() => {
		routerReady = true
		scheduleDismissal()
	},
)

useAppEvent('loading', (e) => {
	if (e.event.type === 'directory_move') {
		realProgress.value = 100 * (e.fraction ?? 1)
		message.value = 'Updating app directory...'
	}
})
</script>

<style scoped lang="scss">
.splash-screen {
	position: fixed;
	inset: 0;
	z-index: 10000;
	overflow: hidden;
	background: var(--color-bg);
	contain: strict;
	will-change: opacity;

	--splash-cube-image: url('@/assets/loading/noctrinth-cube-dark.webp');
	--splash-cube-strength: 0.5;
	--splash-glow-strength: 16%;
}

:global(html.light-mode .splash-screen) {
	--splash-cube-image: url('@/assets/loading/noctrinth-cube-light.webp');
	--splash-cube-strength: 0.7;
	--splash-glow-strength: 12%;
}

/* White on black, and below black on white. */
.splash-screen.is-inverted {
	--splash-cube-image: url('@/assets/loading/noctrinth-cube-light.webp');
	--splash-cube-strength: 0.2;
}

:global(html.light-mode .splash-screen.is-inverted) {
	--splash-cube-image: url('@/assets/loading/noctrinth-cube-dark.webp');
	--splash-cube-strength: 0.95;
}

/*
 * Nothing is drawn until the settings' theme is known: the window is hidden
 * until then anyway, and what was drawn in the cached theme only came back as
 * a stale frame, a cube that vanished and a bar that started over. Once it is
 * ready, everything starts once, in the right colours.
 */
.splash-screen:not(.is-ready) > * {
	visibility: hidden;
}

.splash-screen.is-ready .splash-cube,
.splash-screen.is-ready .splash-bar__fill {
	animation-play-state: running;
}

.splash-cube {
	position: absolute;
	inset: -40vh -40vw;
	background: var(--splash-cube-image) center / contain no-repeat;
	opacity: var(--splash-cube-strength);
	will-change: opacity;
	animation: splash-cube-in 0.9s ease-out both paused;
}

/* The accent, behind the mark: the one colour on the splash, so it matches. */
.splash-glow {
	position: absolute;
	inset: 0;
	background:
		radial-gradient(
			ellipse 55% 45% at 50% 50%,
			color-mix(in srgb, var(--color-brand) var(--splash-glow-strength), transparent),
			transparent 70%
		),
		linear-gradient(
			180deg,
			color-mix(in srgb, var(--color-bg) 10%, transparent) 0%,
			var(--color-bg) 100%
		);
}

.splash-content {
	position: absolute;
	inset: 0;
	display: flex;
	flex-direction: column;
	align-items: center;
	justify-content: center;
	gap: 1.25rem;
	color: var(--color-contrast);
	will-change: transform;
}

.splash-logo {
	height: 2.25rem;
	width: fit-content;
}

.splash-bar {
	width: 18rem;
	height: 0.375rem;
	overflow: hidden;
	border-radius: 999px;
	background: color-mix(in srgb, var(--color-brand) 16%, transparent);
}

/*
 * Scaled rather than resized, so it is the compositor that moves it. Until
 * there is a real fraction it crawls on its own: quickly at first, then ever
 * more slowly towards the end it never reaches, the way a start that takes as
 * long as it takes should look.
 */
.splash-bar__fill {
	width: 100%;
	height: 100%;
	border-radius: inherit;
	background: var(--color-brand);
	transform-origin: left center;
	transform: scaleX(0.04);
	will-change: transform;
	animation: splash-crawl 20s cubic-bezier(0.05, 0.75, 0.15, 1) forwards paused;

	&.is-real {
		animation: none;
		transition: transform 0.3s ease-out;
	}
}

.splash-message {
	font-size: 0.875rem;
	color: var(--color-secondary);
}

@keyframes splash-crawl {
	from {
		transform: scaleX(0.04);
	}
	to {
		transform: scaleX(0.92);
	}
}

@keyframes splash-cube-in {
	from {
		opacity: 0;
	}
}

@media (prefers-reduced-motion: reduce) {
	.splash-cube {
		animation: none;
	}
}
</style>
