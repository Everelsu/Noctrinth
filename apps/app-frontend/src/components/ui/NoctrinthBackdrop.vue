<script setup lang="ts">
/**
 * The background picture, behind the whole window. While there is one, the
 * window's frame lets it through, and the cards on a page a little of it.
 */
import { watchEffect } from 'vue'

import { backgroundBlur, backgroundUrl } from '@/helpers/noctrinth-background'

watchEffect(() => {
	document.documentElement.classList.toggle('noctrinth-backdrop-active', !!backgroundUrl.value)
})
</script>

<template>
	<Transition name="noctrinth-backdrop">
		<div
			v-if="backgroundUrl"
			:key="backgroundUrl"
			class="noctrinth-backdrop"
			:class="{ 'noctrinth-backdrop--blur': backgroundBlur }"
			:style="{ backgroundImage: `url(${backgroundUrl})` }"
		/>
	</Transition>
</template>

<style>
.noctrinth-backdrop {
	position: fixed;
	inset: 0;
	z-index: 0;
	pointer-events: none;
	background-position: center;
	background-size: cover;
	background-repeat: no-repeat;
}

.noctrinth-backdrop::after {
	/* A glow of the accent from the top corner, so the picture sits in the
	   same colour as everything over it. */
	content: '';
	position: absolute;
	inset: 0;
	background:
		radial-gradient(
			ellipse at 15% 0%,
			color-mix(in srgb, var(--color-brand) 22%, transparent),
			transparent 60%
		),
		radial-gradient(
			ellipse at 100% 100%,
			color-mix(in srgb, var(--color-brand) 12%, transparent),
			transparent 55%
		);
}

.noctrinth-backdrop--blur {
	filter: blur(14px);
	/* The blur pulls the edges in; scaled up so they stay off-screen. */
	transform: scale(1.08);
}

.noctrinth-backdrop-enter-active,
.noctrinth-backdrop-leave-active {
	transition: opacity 0.4s ease;
}

.noctrinth-backdrop-enter-from,
.noctrinth-backdrop-leave-to {
	opacity: 0;
}

:root.noctrinth-backdrop-active .app-grid-layout {
	background-color: color-mix(in srgb, var(--color-raised-bg) 45%, transparent);
}

:root.noctrinth-backdrop-active .app-grid-navbar,
:root.noctrinth-backdrop-active .app-title-bar,
:root.noctrinth-backdrop-active .app-window-controls {
	background-color: transparent;
}

:root.noctrinth-backdrop-active body {
	/* The theme's solid surfaces, kept here so the page can mix from them. */
	--noctrinth-solid-surface-2: var(--surface-2);
	--noctrinth-solid-surface-3: var(--surface-3);
	--noctrinth-solid-surface-4: var(--surface-4);
}

:root.noctrinth-backdrop-active .app-contents {
	background-color: color-mix(in srgb, var(--color-bg) 50%, transparent);

	/* The cards on a page let the picture through too, a little. */
	--surface-2: color-mix(in srgb, var(--noctrinth-solid-surface-2) 72%, transparent);
	--surface-3: color-mix(in srgb, var(--noctrinth-solid-surface-3) 72%, transparent);
	--surface-4: color-mix(in srgb, var(--noctrinth-solid-surface-4) 72%, transparent);
	--color-raised-bg: var(--surface-3);
	--color-button-bg: var(--surface-4);
}

/* The frame's buttons — the sidebar's, the title bar's, the window's own — lit
   on hover with a veil rather than a solid grey disc. */
:root.noctrinth-backdrop-active .app-grid-layout,
:root.noctrinth-backdrop-active .app-window-controls {
	--color-button-bg: color-mix(in srgb, var(--noctrinth-solid-surface-4) 55%, transparent);
}

/* What opens over everything is frosted glass: mostly solid, so it reads as
   easily as before, with the picture blurred through it. */
:root.noctrinth-backdrop-active .modal-body,
:root.noctrinth-backdrop-active .download-manager-panel {
	--surface-3: color-mix(in srgb, var(--noctrinth-solid-surface-3) 80%, transparent);
	--color-raised-bg: var(--surface-3);
	backdrop-filter: blur(24px) saturate(1.3);
}

/* An open dropdown list is drawn inside the page; it stays solid to be read. */
:root.noctrinth-backdrop-active .app-contents :has(> .combobox-options-scrollbar) {
	--surface-4: var(--noctrinth-solid-surface-4);
}

:root.noctrinth-backdrop-active .app-sidebar {
	/* The accent's own wash, which is see-through already, over a light veil. */
	background:
		var(--brand-gradient-bg),
		linear-gradient(
			color-mix(in srgb, var(--noctrinth-solid-surface-3) 45%, transparent),
			color-mix(in srgb, var(--noctrinth-solid-surface-3) 45%, transparent)
		);
}
</style>
