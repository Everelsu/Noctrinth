<script setup lang="ts">
/**
 * Two fingers across the touchpad go back and forward through the launcher's
 * pages exactly as Chrome's "Swipe between pages" does: Chromium's
 * OverscrollController and its affordance, by way of Two-Finger-Back — see
 * helpers/swipe. A swipe navigates when the fingers lift past the threshold;
 * pulled back short of it, it doesn't.
 */
import { onBeforeUnmount, onMounted } from 'vue'
import { useRouter } from 'vue-router'

import { startTouchpadNavigation } from '@/helpers/swipe/touchpad-navigation'

const router = useRouter()

let stop: (() => void) | undefined

onMounted(() => {
	stop = startTouchpadNavigation({
		navigate(back) {
			if (back) router.back()
			else router.forward()
		},
		// vue-router keeps both neighbours in history.state.
		canNavigate(back) {
			const state = window.history.state as { back?: unknown; forward?: unknown } | null
			return back ? !!state?.back : !!state?.forward
		},
		isBlocked: () => !!document.querySelector('[aria-modal="true"], [data-modal-root]'),
	})
})

onBeforeUnmount(() => stop?.())
</script>

<template>
	<span hidden />
</template>
