/**
 * Clears tooltips that were left on screen with nothing to dismiss them.
 *
 * A tooltip hides when the pointer leaves the element it belongs to. That event
 * does not always arrive: the window loses focus mid-hover, or the pointer
 * leaves the window entirely rather than moving to something else inside it.
 * What is left is a tooltip floating over the interface with no way to get rid
 * of it but to hover the same element again.
 *
 * Upstream's own tooltips handle the third case this used to cover — an element
 * unmounted while its tooltip is up — in the directive itself, which is a
 * better place for it than here. What is left is the two moments the directive
 * cannot see, plus a navigation, since an element that survives the route
 * change (the sidebar's buttons, most of all) never gets the `mouseleave` that
 * would have closed it.
 */
import { dismissTooltip } from '@modrinth/ui/src/providers/tooltip'
import type { Router } from 'vue-router'

let installed = false

export function installTooltipCleanup(router: Router): void {
	if (installed) return
	installed = true

	window.addEventListener('blur', () => dismissTooltip())

	// `relatedTarget` is null only when the pointer left the window rather than
	// moving to another element inside it.
	document.addEventListener('mouseout', (event) => {
		if (!event.relatedTarget) dismissTooltip()
	})

	router.afterEach(() => dismissTooltip())
}
