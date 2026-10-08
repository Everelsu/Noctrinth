/**
 * Chromium's OverscrollController, narrowed to horizontal history navigation
 * from a precision touchpad, as Two-Finger-Back ports it (GPL-3.0). See
 * overscroll-config.ts for where it comes from.
 */
import { config } from './overscroll-config'

export type OverscrollMode = 'NONE' | 'EAST' | 'WEST'

/** EAST: content dragged right, which is back. WEST: dragged left, forward. */
export const MODE_NONE: OverscrollMode = 'NONE'
export const MODE_EAST: OverscrollMode = 'EAST'
export const MODE_WEST: OverscrollMode = 'WEST'

type ScrollState = 'NONE' | 'CONTENT_CONSUMING' | 'OVERSCROLLING'

export interface OverscrollDelegate {
	getDisplaySize(): { width: number; height: number }
	getMaxOverscrollDelta(): number | null
	onOverscrollModeChange(oldMode: OverscrollMode, newMode: OverscrollMode): void
	/** Returns true when the UI consumed it. */
	onOverscrollUpdate(deltaX: number): boolean
	onOverscrollComplete(mode: OverscrollMode): void
}

export class OverscrollController {
	private overscrollMode: OverscrollMode = MODE_NONE
	private lockedMode: OverscrollMode = MODE_NONE
	private scrollState: ScrollState = 'NONE'
	private overscrollDeltaX = 0
	private overscrollDeltaY = 0
	private ignoreFollowingInertialEvents = false
	private firstInertialEventTime: number | null = null

	constructor(private readonly delegate: OverscrollDelegate) {}

	/** GestureScrollBegin: a fresh gesture may overscroll again. */
	scrollBegin() {
		this.ignoreFollowingInertialEvents = false
		this.firstInertialEventTime = null
		if (this.overscrollMode !== MODE_NONE) this.setOverscrollMode(MODE_NONE)
		this.resetScrollState()
	}

	/** Something on the page scrolled, so this gesture can no longer overscroll. */
	markContentConsuming() {
		if (this.scrollState === 'NONE') this.scrollState = 'CONTENT_CONSUMING'
	}

	needsContentCheck() {
		return this.scrollState === 'NONE'
	}

	shouldIgnoreInertialEvent() {
		return this.ignoreFollowingInertialEvents
	}

	isOverscrolling() {
		return this.overscrollMode !== MODE_NONE
	}

	/**
	 * GestureScrollUpdate, in Chromium's sign convention (the negation of
	 * WheelEvent's): positive deltaX moves the content right. Returns true when
	 * the event was consumed and must not reach the page.
	 */
	scrollUpdate(deltaX: number, deltaY: number, isInertial: boolean, timeStampMs: number): boolean {
		if (this.ignoreFollowingInertialEvents && isInertial) return true

		// Only consumed once the overscroll is under way: the event that starts it
		// still reaches the page.
		const wasOverscrolling = this.overscrollMode !== MODE_NONE
		const stateWasOverscrolling = this.scrollState === 'OVERSCROLLING'

		this.processOverscroll(deltaX, deltaY, isInertial)

		if (isInertial) {
			// The first inertial event is ignored; a fling running longer than the
			// allowance without completing cancels.
			if (this.firstInertialEventTime === null) {
				this.firstInertialEventTime = timeStampMs
			} else if (timeStampMs - this.firstInertialEventTime >= config.MAX_INERTIAL_MS) {
				this.ignoreFollowingInertialEvents = true
				this.cancel()
			}
		} else {
			// Chromium never sees a finger event after a fling starts; here a hand
			// slowing for a few events reads as one, and the clock must restart.
			this.firstInertialEventTime = null
		}

		return wasOverscrolling || stateWasOverscrolling
	}

	/** Settles an open gesture: navigate if it got far enough, drop it otherwise. */
	resolve() {
		if (this.overscrollMode === MODE_NONE) return
		this.resetScrollState()
		if (this.shouldCompleteAction()) {
			this.completeAction()
			return
		}
		this.setOverscrollMode(MODE_NONE)
	}

	/** A mouse move is what settles a gesture whose fingers lifted with no momentum. */
	pointerMoved() {
		this.resolve()
	}

	private processOverscroll(deltaX: number, deltaY: number, isInertial: boolean): boolean {
		if (this.scrollState === 'CONTENT_CONSUMING') return false

		// Inertial events may finish an overscroll but never start one.
		if (this.overscrollMode === MODE_NONE && isInertial) return false

		this.overscrollDeltaX += deltaX
		this.overscrollDeltaY += deltaY

		const startThreshold = config.START_THRESHOLD_DIPS
		if (
			Math.abs(this.overscrollDeltaX) <= startThreshold &&
			Math.abs(this.overscrollDeltaY) <= startThreshold
		) {
			this.setOverscrollMode(MODE_NONE)
			return true
		}

		const cap = this.delegate.getMaxOverscrollDelta()
		if (cap !== null && this.overscrollMode !== MODE_NONE) {
			this.overscrollDeltaX = clampAbsoluteValue(this.overscrollDeltaX, cap + startThreshold)
		}

		let newMode: OverscrollMode = MODE_NONE
		if (
			Math.abs(this.overscrollDeltaX) > startThreshold &&
			Math.abs(this.overscrollDeltaX) > Math.abs(this.overscrollDeltaY) * config.MIN_DIRECTION_RATIO
		) {
			newMode = this.overscrollDeltaX > 0 ? MODE_EAST : MODE_WEST
		}

		if (this.overscrollMode === MODE_NONE) {
			this.setOverscrollMode(newMode)
		} else if (newMode !== this.overscrollMode) {
			this.setOverscrollMode(MODE_NONE)
		}

		if (this.overscrollMode === MODE_NONE) return false

		// The affordance starts moving from zero once the gesture clears the
		// threshold, so the threshold amount is not reported.
		let delegateDeltaX = this.overscrollDeltaX
		if (Math.abs(delegateDeltaX) > startThreshold) {
			delegateDeltaX += delegateDeltaX < 0 ? startThreshold : -startThreshold
		} else {
			delegateDeltaX = 0
		}
		return this.delegate.onOverscrollUpdate(delegateDeltaX)
	}

	/** DispatchEventCompletesAction: the gesture is far enough to navigate. */
	shouldCompleteAction(): boolean {
		if (this.overscrollMode === MODE_NONE) return false
		const size = this.delegate.getDisplaySize()
		const longestSide = Math.max(size.width, size.height)
		if (longestSide <= 0) return false
		return Math.abs(this.overscrollDeltaX) / longestSide >= config.COMPLETE_THRESHOLD_PERCENT
	}

	completeAction() {
		this.ignoreFollowingInertialEvents = true
		const mode = this.overscrollMode
		this.delegate.onOverscrollComplete(mode)
		this.reset()
	}

	private reset() {
		this.overscrollMode = MODE_NONE
		this.overscrollDeltaX = this.overscrollDeltaY = 0
		this.resetScrollState()
	}

	cancel() {
		this.setOverscrollMode(MODE_NONE)
		this.overscrollDeltaX = this.overscrollDeltaY = 0
		this.resetScrollState()
	}

	private setOverscrollMode(mode: OverscrollMode) {
		if (this.overscrollMode === mode) return
		// Once a direction is locked the gesture cannot flip to the other one.
		if (mode !== MODE_NONE && this.lockedMode !== MODE_NONE && mode !== this.lockedMode) return

		const oldMode = this.overscrollMode
		this.overscrollMode = mode
		if (this.overscrollMode === MODE_NONE) {
			this.overscrollDeltaX = this.overscrollDeltaY = 0
		} else {
			this.scrollState = 'OVERSCROLLING'
			this.lockedMode = this.overscrollMode
		}
		this.delegate.onOverscrollModeChange(oldMode, this.overscrollMode)
	}

	private resetScrollState() {
		this.scrollState = 'NONE'
		this.lockedMode = MODE_NONE
	}
}

function clampAbsoluteValue(value: number, maxAbs: number): number {
	return Math.min(Math.max(value, -maxAbs), maxAbs)
}
