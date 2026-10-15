/**
 * The right sidebar's column opening or closing (App.vue's `.app-contents`
 * transition). The page narrows with it frame by frame, which suits text but
 * not what lays itself out from its own width in script: the library's grid
 * would change its column count mid-slide and shake. It reads this instead,
 * lays itself out once for the width it is heading to, and moves its cards
 * there on the same curve, in step with the sidebar.
 */
import { ref } from 'vue'

/** As long as the column's transition, with a frame to spare. */
export const SIDEBAR_MOTION_MS = 450

/** The column's width, which is how much the page gains or loses. */
export const SIDEBAR_WIDTH = 300

export const sidebarMoving = ref(false)
/** Which way it is going: true while it opens, so the page loses width. */
export const sidebarOpening = ref(false)

let settle: ReturnType<typeof setTimeout> | undefined

export function markSidebarMoving(opening: boolean): void {
	clearTimeout(settle)
	sidebarOpening.value = opening
	sidebarMoving.value = true
	settle = setTimeout(() => (sidebarMoving.value = false), SIDEBAR_MOTION_MS)
}
