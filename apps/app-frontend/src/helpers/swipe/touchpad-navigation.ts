/**
 * Turns the wheel stream into the gesture events Chromium's OverscrollController
 * expects and plays its delegate, as Two-Finger-Back does (GPL-3.0) — with the
 * launcher's router where the extension has the browser's history.
 *
 * The navigation fires when the fingers leave the touchpad, never when the
 * threshold is crossed: drag past it, drag back, lift, and nothing happens. The
 * DOM shows neither gesture phases nor momentum, so lifting is read from the
 * stream — a run of strictly decaying deltas is a fling, silence after a fling
 * is its end, and a mouse move settles a gesture left hanging.
 */
import { Affordance } from './affordance'
import { config } from './overscroll-config'
import {
	MODE_EAST,
	MODE_NONE,
	OverscrollController,
	type OverscrollDelegate,
	type OverscrollMode,
} from './overscroll-controller'

export interface TouchpadNavigationHost {
	navigate(back: boolean): void
	canNavigate(back: boolean): boolean
	/** Somewhere a sideways gesture belongs to something else, like a dialog. */
	isBlocked(): boolean
}

const DOM_DELTA_PIXEL = 0

/** Starts listening; returns the function that stops. */
export function startTouchpadNavigation(host: TouchpadNavigationHost): () => void {
	let affordance: Affordance | null = null
	let completionThreshold = 0
	let maxDelta = 0

	const delegate: OverscrollDelegate = {
		// Chromium measures against the display, not the window.
		getDisplaySize: () => ({ width: screen.width, height: screen.height }),
		getMaxOverscrollDelta: () => (affordance ? maxDelta : null),
		onOverscrollModeChange(_oldMode, newMode) {
			// Nowhere to go that way: no affordance, and the one there was animates out.
			if (newMode === MODE_NONE || !host.canNavigate(newMode === MODE_EAST)) {
				if (affordance && !affordance.isFinishing()) affordance.abort()
				return
			}
			if (affordance) {
				affordance.destroy()
				affordance = null
			}
			const maxSize = Math.max(screen.width, screen.height)
			completionThreshold =
				maxSize * config.COMPLETE_THRESHOLD_PERCENT - config.START_THRESHOLD_DIPS
			maxDelta = maxSize - config.START_THRESHOLD_DIPS
			if (completionThreshold <= 0) return
			const created: Affordance = new Affordance(newMode, maxDelta / completionThreshold, () => {
				created.destroy()
				if (affordance === created) affordance = null
			})
			affordance = created
		},
		onOverscrollUpdate(deltaX) {
			if (!affordance || affordance.isFinishing()) return false
			affordance.setDragProgress(Math.abs(deltaX) / completionThreshold)
			return true
		},
		onOverscrollComplete(mode: OverscrollMode) {
			if (!affordance || affordance.isFinishing()) return
			affordance.complete()
			host.navigate(mode === MODE_EAST)
		},
	}

	const controller = new OverscrollController(delegate)

	// Stands in for the consumed-ACK signal: if anything under the pointer can
	// still scroll the gesture's way, the page takes it and there is no overscroll.
	function contentCanScroll(event: WheelEvent): boolean {
		const root = document.scrollingElement || document.documentElement
		for (const node of event.composedPath()) {
			if (!(node instanceof Element)) continue
			if (elementCanScroll(node, event.deltaX, event.deltaY, node === root)) return true
		}
		return root ? elementCanScroll(root, event.deltaX, event.deltaY, true) : false
	}

	let lastEventTime = 0
	let momentumTimer: ReturnType<typeof setTimeout> | undefined
	let safetyTimer: ReturnType<typeof setTimeout> | undefined
	let gestureIsPrecise = false
	let inMomentum = false
	let decayCount = 0
	let lastMagnitude = 0

	/**
	 * Only precise-pixel scrolls overscroll. A fractional delta can only come from
	 * a touchpad, and once one shows up the rest of the gesture is trusted; a
	 * classic wheel gives itself away with whole multiples of 100.
	 */
	function isPreciseScroll(event: WheelEvent): boolean {
		if (event.deltaMode !== DOM_DELTA_PIXEL) return false
		const fractional = (d: number) => d !== 0 && !Number.isInteger(d)
		if (fractional(event.deltaX) || fractional(event.deltaY)) {
			gestureIsPrecise = true
			return true
		}
		if (gestureIsPrecise) return true
		const chunky = (d: number) => d !== 0 && Number.isInteger(d) && d % 100 === 0
		if (chunky(event.deltaX)) return false
		return !(event.deltaX === 0 && chunky(event.deltaY))
	}

	/** A lifted hand leaves a fling of decaying deltas: the one hint of momentum. */
	function classifyEvent(magnitude: number) {
		if (inMomentum) return
		// Resting-finger jitter goes up and down one step; it counts as neither.
		if (magnitude < config.MICRO_DELTA_PX) return
		if (magnitude < lastMagnitude) decayCount++
		else decayCount = 0
		lastMagnitude = magnitude
		inMomentum = decayCount >= config.MOMENTUM_DECAY_EVENTS
	}

	/** Enough decay that this may finish an overscroll but must not start one. */
	function isDecaying() {
		return inMomentum || decayCount >= config.MOMENTUM_SUSPECT_EVENTS
	}

	function resetGestureTracking() {
		gestureIsPrecise = false
		inMomentum = false
		decayCount = 0
		lastMagnitude = 0
	}

	function clearGestureTimers() {
		clearTimeout(momentumTimer)
		clearTimeout(safetyTimer)
		momentumTimer = safetyTimer = undefined
	}

	/** The fling died out, or silence with none: settle on how far it got. */
	function settle() {
		clearGestureTimers()
		controller.resolve()
		resetGestureTracking()
	}

	function onWheel(event: WheelEvent) {
		if (event.ctrlKey || host.isBlocked()) return
		const now = event.timeStamp
		// A long enough gap is a new gesture — but only with nothing on screen:
		// resting fingers send nothing either, and that pause must leave the
		// gesture where it was.
		const liveGesture = affordance && !affordance.isFinishing()
		if (now - lastEventTime > config.GESTURE_GAP_MS && !liveGesture) {
			controller.scrollBegin()
			resetGestureTracking()
		}

		if (!isPreciseScroll(event)) return
		lastEventTime = now

		if (controller.needsContentCheck() && contentCanScroll(event)) {
			controller.markContentConsuming()
		}

		classifyEvent(Math.hypot(event.deltaX, event.deltaY))
		const isInertial = isDecaying()
		let consumed: boolean

		// Completion runs before this delta is folded in and needs the confirmed
		// fling: navigating off a hand merely slowing down would fire with the
		// fingers still on the pad.
		if (
			inMomentum &&
			!controller.shouldIgnoreInertialEvent() &&
			controller.shouldCompleteAction()
		) {
			controller.completeAction()
			consumed = false
		} else {
			// Chromium's gesture deltas are the negation of the wheel's.
			consumed = controller.scrollUpdate(-event.deltaX, -event.deltaY, isInertial, now)
		}

		if (consumed && event.cancelable) event.preventDefault()

		// Silence alone says nothing, except once a fling runs: its end is the
		// gesture's end, caught on a timer because the stream simply stops.
		clearGestureTimers()
		if (controller.isOverscrolling()) {
			if (inMomentum) momentumTimer = setTimeout(settle, config.MOMENTUM_IDLE_MS)
			const armed = controller.shouldCompleteAction()
			safetyTimer = setTimeout(settle, armed ? config.ARMED_IDLE_MS : config.GESTURE_SAFETY_MS)
		}
	}

	/** Nothing else tells fingers still down from fingers lifted gently. */
	function onMouseMove() {
		clearGestureTimers()
		controller.pointerMoved()
		resetGestureTracking()
	}

	/** A gesture cannot outlive the focus it started in. */
	function onInterrupted() {
		if (document.visibilityState === 'hidden' || !document.hasFocus()) {
			clearGestureTimers()
			controller.cancel()
			resetGestureTracking()
		}
	}

	window.addEventListener('wheel', onWheel, { passive: false, capture: true })
	window.addEventListener('mousemove', onMouseMove, { capture: true })
	window.addEventListener('blur', onInterrupted)
	document.addEventListener('visibilitychange', onInterrupted)

	return () => {
		window.removeEventListener('wheel', onWheel, { capture: true })
		window.removeEventListener('mousemove', onMouseMove, { capture: true })
		window.removeEventListener('blur', onInterrupted)
		document.removeEventListener('visibilitychange', onInterrupted)
		clearGestureTimers()
		resetGestureTracking()
		controller.cancel()
		affordance?.destroy()
		affordance = null
	}
}

function elementCanScroll(element: Element, deltaX: number, deltaY: number, isRoot: boolean) {
	const style = getComputedStyle(element)
	if (
		deltaX !== 0 &&
		axisCanScroll(
			element.scrollLeft,
			element.scrollWidth,
			element.clientWidth,
			style.overflowX,
			deltaX,
			isRoot,
		)
	) {
		return true
	}
	return (
		deltaY !== 0 &&
		axisCanScroll(
			element.scrollTop,
			element.scrollHeight,
			element.clientHeight,
			style.overflowY,
			deltaY,
			isRoot,
		)
	)
}

function axisCanScroll(
	position: number,
	scrollSize: number,
	clientSize: number,
	overflow: string,
	delta: number,
	isRoot: boolean,
) {
	// The viewport scrolls even when the root's computed overflow is visible.
	const scrollable = isRoot
		? overflow !== 'hidden' && overflow !== 'clip'
		: overflow === 'auto' || overflow === 'scroll' || overflow === 'overlay'
	if (!scrollable) return false
	const max = scrollSize - clientSize
	if (max <= 1) return false
	return delta < 0 ? position > 0.5 : position < max - 0.5
}
