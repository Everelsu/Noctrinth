/**
 * Starts loading a page before it is opened, so opening it waits on less.
 *
 * Two things stand between a click and the next page. Its code is a chunk of
 * its own, fetched and compiled the first time the page is visited; that is
 * done here while the launcher is idle, for every page at once. And a project
 * page waits on two rounds of requests before it draws anything, which is what
 * makes opening one from Browse feel slow; those are made here while the
 * pointer rests on the card, the way browsers start loading a link on hover,
 * so the click finds them in the launcher's cache.
 */
import type { Router } from 'vue-router'

import { get_project, get_project_v3, get_team, get_version_many } from '@/helpers/cache.js'

/** Long enough to mean "about to click", short enough to beat the click. */
const HOVER_INTENT_MS = 90

/** Loads every lazily loaded page's code once nothing else is happening. */
export function prefetchRouteChunks(router: Router) {
	const idle = (callback: () => void) =>
		'requestIdleCallback' in window
			? window.requestIdleCallback(callback, { timeout: 5000 })
			: setTimeout(callback, 1500)

	idle(() => {
		for (const record of router.getRoutes()) {
			for (const component of Object.values(record.components ?? {})) {
				// A lazy route's component is the loader itself; calling it starts the
				// import, and the module cache answers the navigation that follows.
				if (typeof component === 'function') {
					;(component as () => Promise<unknown>)().catch(() => {})
				}
			}
		}
	})
}

const warmed = new Set<string>()

async function warmProject(id: string) {
	if (warmed.has(id)) return
	warmed.add(id)
	try {
		const [project] = await Promise.all([
			get_project(id, 'must_revalidate'),
			get_project_v3(id, 'must_revalidate'),
		])
		if (!project) return
		await Promise.all([
			get_version_many(project.versions, 'must_revalidate'),
			get_team(project.team),
		])
	} catch {
		// A warm-up that fails changes nothing: the page asks again on its own.
		warmed.delete(id)
	}
}

/** Warms a project page's data while the pointer rests on a link to it. */
export function startProjectHoverPrefetch(): () => void {
	let timer: ReturnType<typeof setTimeout> | undefined
	let pendingId: string | null = null

	function onPointerOver(event: PointerEvent) {
		const link = (event.target as Element | null)?.closest?.('a[href^="/project/"]')
		const id = link?.getAttribute('href')?.match(/^\/project\/([^/?#]+)/)?.[1] ?? null
		if (id === pendingId) return
		clearTimeout(timer)
		pendingId = id
		if (id && !warmed.has(id)) {
			timer = setTimeout(() => void warmProject(decodeURIComponent(id)), HOVER_INTENT_MS)
		}
	}

	document.addEventListener('pointerover', onPointerOver, { passive: true })
	return () => {
		clearTimeout(timer)
		document.removeEventListener('pointerover', onPointerOver)
	}
}
