/**
 * Clears tooltips that were left on screen with nothing to dismiss them.
 *
 * floating-vue hides a tooltip when the pointer leaves the element it belongs
 * to. That event does not always arrive: a dialog opened over the button takes
 * the pointer with it, the button is unmounted while its tooltip is up, or the
 * window loses focus mid-hover. What is left is a tooltip floating over the
 * interface with no way to get rid of it but to hover the same button again.
 *
 * These are the moments where a tooltip is certainly stale, whether or not the
 * event that should have hidden it turned up:
 *
 * - the window lost focus, so nothing is being hovered any more;
 * - the pointer left the window entirely;
 * - the page changed under it.
 *
 * `hideAllPoppers` also closes open menus and dropdowns, which is why it is
 * only called on those three: each is a moment where an open menu should be
 * closing anyway.
 */
import { hideAllPoppers } from 'floating-vue'
import type { Router } from 'vue-router'

let installed = false

export function installPopperCleanup(router: Router): void {
	if (installed) return
	installed = true

	window.addEventListener('blur', () => hideAllPoppers())

	// `relatedTarget` is null only when the pointer left the window rather than
	// moving to another element inside it.
	document.addEventListener('mouseout', (event) => {
		if (!event.relatedTarget) hideAllPoppers()
	})

	router.afterEach(() => hideAllPoppers())
}
