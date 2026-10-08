import { AxisPhysicsMSDModel } from './axis-physics'

export type SwipeDirection = 'back' | 'forward'

export interface SwipeConfig {
	enabled: boolean
	sensitivity: number
	invertDirection: boolean
	showIndicator: boolean
	pageSlide: boolean
}

export interface SwipeHandlers {
	getConfig(): SwipeConfig
	navigate(direction: SwipeDirection): void
	/** Live feedback while the swipe accumulates (0..1 progress, Firefox's `min(|amount|*4, 1)`). */
	onProgress?(doc: Document, direction: SwipeDirection | null, progress: number): void
	/** The gesture was released past the success threshold and navigation fired. */
	onCommit?(doc: Document, direction: SwipeDirection): void
	/** The gesture ended (commit, or spring-back finished). `committed` is true if it navigated. */
	onEnd?(doc: Document, committed: boolean): void
}

export interface SwipeHandle {
	onWheel(e: WheelEvent): void
	dispose(): void
}

/*
 * Gesture model ported from Firefox's widget/SwipeTracker.cpp (the layer Zen
 * uses unchanged), with the constants from StaticPrefList.yaml:
 *  - a whole swipe is `widget.swipe.pixel-size` = 1100px of pan on Windows;
 *  - releasing commits at kSwipeSuccessThreshold = 0.25 of a whole swipe,
 *    with the release velocity contributing 0.05 (a fast flick can commit a
 *    short swipe, and fingers twitching away from the target at release —
 *    velocity beyond the twitch tolerance — aborts);
 *  - the gesture direction is fixed by the first movement and the amount is
 *    clamped to that side (SwipeTracker::ClampToAllowedRange), so dragging
 *    back past the origin never navigates the other way mid-gesture;
 *  - if the current velocity says the release would NOT navigate, the visual
 *    amount is capped just below the threshold (0.999×) so the icon never
 *    shows "will navigate" falsely;
 *  - a swipe only starts when the pan is ≥8× more horizontal than vertical
 *    (SwipeTracker::CanTriggerSwipe's trial-and-error dominance factor);
 *  - an unsuccessful release springs the gesture back to zero through
 *    SwipeTracker::StartAnimating's physics: an RK4-integrated unit-mass
 *    spring (kSpringForce=250, critically damped) seeded with the release
 *    velocity, finished per AxisPhysicsMSDModel::IsFinished with the smallest
 *    visible increment of 1/wholeSwipePx.
 *
 * What CEF wheel events can't give us, unlike the PanGestureInput phases
 * Firefox gets from the OS: an explicit fingers-lifted event and momentum
 * flags. Release is detected as RELEASE_MS of silence, and the momentum tail
 * after it is swallowed unless a sharp delta spike starts a new gesture.
 */
const WHOLE_SWIPE_PX = 1100
const SUCCESS_THRESHOLD = 0.25
const VELOCITY_CONTRIBUTION = 0.05
const VELOCITY_TWITCH_TOLERANCE = 0.0000001
const MIN_EVENT_DT_S = 0.008
const SPRING_FORCE = 250
const SPRING_DAMPING_RATIO = 1
const AXIS_DOMINANCE = 8
const DECIDE_PX = 12
/** Silence after a moving lift = the fingers left the touchpad. */
const RELEASE_FAST_MS = 160
/** Fingers at rest are likely still down (holding): give them a long grace window. */
const RELEASE_HOLD_MS = 650
/** Below this speed (whole-swipes/s, ≈165px/s at default) the fingers count as resting. */
const HOLD_VELOCITY = 0.15
/** A decisive flick past the success formula commits immediately, like a browser fling. */
const FLICK_COMMIT_VELOCITY = 1.2
const GESTURE_GAP_MS = 1000
/** Post-commit momentum tail must jump 1.6× to count as a new gesture… */
const SPIKE_RATIO = 1.6
/** …but after a cancel, gently resuming the drag (growing deltas) re-grabs it. */
const SPIKE_RESUME_RATIO = 1.15
const SPIKE_MIN_PX = 30
/** sensitivity 1-5 scales the whole-swipe distance; 3 = Firefox's exact 1100px. */
const SENSITIVITY_SCALE = [1.6, 1.3, 1.0, 0.8, 0.65]

function wholeSwipePx(sensitivity: number): number {
	const index = Math.min(5, Math.max(1, Math.round(sensitivity))) - 1
	return WHOLE_SWIPE_PX * SENSITIVITY_SCALE[index]
}

type Phase = 'idle' | 'deciding' | 'active' | 'animating' | 'settled'
type SettleReason = 'commit' | 'cancel' | 'scroll'

interface GestureState {
	phase: Phase
	settleReason: SettleReason | null
	decideDx: number
	decideDy: number
	/** Signed gesture amount in whole-swipe units (SwipeTracker::mGestureAmount). */
	amount: number
	/** Sign of the gesture's fixed direction: +1 when deltaX was positive at decide time. */
	directionSign: 1 | -1
	/** Whole-swipe units per second (SwipeTracker::mCurrentVelocity). */
	velocity: number
	lastEventAt: number
	lastTailMag: number
	releaseTimer: number | undefined
	releaseView: Window | null
	animFrame: number | undefined
	animView: Window | null
}

export function createSwipeHandler(handlers: SwipeHandlers): SwipeHandle {
	const state: GestureState = {
		phase: 'idle',
		settleReason: null,
		decideDx: 0,
		decideDy: 0,
		amount: 0,
		directionSign: 1,
		velocity: 0,
		lastEventAt: 0,
		lastTailMag: Infinity,
		releaseTimer: undefined,
		releaseView: null,
		animFrame: undefined,
		animView: null,
	}
	let disposed = false

	const clearReleaseTimer = () => {
		if (state.releaseTimer !== undefined && state.releaseView) {
			state.releaseView.clearTimeout(state.releaseTimer)
		}
		state.releaseTimer = undefined
		state.releaseView = null
	}

	const cancelAnimationLoop = () => {
		if (state.animFrame !== undefined && state.animView) {
			state.animView.cancelAnimationFrame(state.animFrame)
		}
		state.animFrame = undefined
		state.animView = null
	}

	/** Stop an in-flight spring-back and let the visuals wind down. */
	const abortAnimation = (doc: Document) => {
		if (state.phase !== 'animating') return
		cancelAnimationLoop()
		handlers.onEnd?.(doc, false)
	}

	const toIdle = () => {
		clearReleaseTimer()
		cancelAnimationLoop()
		state.phase = 'idle'
		state.settleReason = null
		state.decideDx = 0
		state.decideDy = 0
		state.amount = 0
		state.velocity = 0
	}

	const settle = (reason: SettleReason) => {
		clearReleaseTimer()
		state.phase = 'settled'
		state.settleReason = reason
		state.lastTailMag = Infinity
	}

	/** SwipeTracker::ComputeSwipeSuccess, verbatim. */
	const computeSwipeSuccess = (): boolean => {
		const sign = state.directionSign
		if (state.velocity * sign < -VELOCITY_TWITCH_TOLERANCE) return false
		return state.amount * sign + state.velocity * sign * VELOCITY_CONTRIBUTION >= SUCCESS_THRESHOLD
	}

	/** Firefox caps the displayed amount below the threshold when velocity says the release would not navigate. */
	const displayAmount = (success: boolean): number => {
		let amount = state.amount
		if (!success && Math.abs(amount) >= SUCCESS_THRESHOLD) {
			amount = Math.sign(amount) * 0.999 * SUCCESS_THRESHOLD
		}
		return amount
	}

	const currentDirection = (config: SwipeConfig): SwipeDirection =>
		resolveDirection(state.directionSign, config.invertDirection)

	/** SwipeTracker::StartAnimating + WillRefresh: spring the gesture amount back to 0. */
	const startSpringBack = (doc: Document, config: SwipeConfig, startAmount: number) => {
		const view = doc.defaultView
		const direction = currentDirection(config)
		if (!view) {
			handlers.onEnd?.(doc, false)
			settle('cancel')
			return
		}
		state.phase = 'animating'
		state.lastTailMag = Infinity

		const axis = new AxisPhysicsMSDModel(
			startAmount,
			0,
			state.velocity,
			SPRING_FORCE,
			SPRING_DAMPING_RATIO,
		)
		const minIncrement = 1 / wholeSwipePx(config.sensitivity)
		let lastFrameAt = view.performance.now()

		const step = (frameNow: number) => {
			state.animFrame = undefined
			if (disposed || state.phase !== 'animating') return
			axis.simulate(Math.max(0, frameNow - lastFrameAt) / 1000)
			lastFrameAt = frameNow

			const finished = axis.isFinished(minIncrement)
			state.amount = finished ? axis.getDestination() : axis.getPosition()
			if (config.showIndicator || config.pageSlide) {
				handlers.onProgress?.(doc, direction, Math.min(Math.abs(state.amount) * 4, 1))
			}
			if (finished) {
				handlers.onEnd?.(doc, false)
				settle('cancel')
				return
			}
			state.animView = view
			state.animFrame = view.requestAnimationFrame(step)
		}
		state.animView = view
		state.animFrame = view.requestAnimationFrame(step)
	}

	const finishGesture = (doc: Document) => {
		if (state.phase !== 'active') return
		clearReleaseTimer()
		const config = handlers.getConfig()
		const success = !disposed && config.enabled && computeSwipeSuccess()
		if (success) {
			const direction = currentDirection(config)
			handlers.navigate(direction)
			handlers.onCommit?.(doc, direction)
			handlers.onEnd?.(doc, true)
			settle('commit')
		} else {
			startSpringBack(doc, config, displayAmount(false))
		}
	}

	const processActive = (
		e: WheelEvent,
		doc: Document,
		view: Window | null,
		effDx: number,
		dtMs: number,
		config: SwipeConfig,
	) => {
		e.preventDefault()
		e.stopPropagation()

		const deltaAmount = effDx / wholeSwipePx(config.sensitivity)
		// ClampToAllowedRange: the amount stays on the gesture's own side.
		const min = state.directionSign > 0 ? 0 : -1
		const max = state.directionSign > 0 ? 1 : 0
		state.amount = Math.min(max, Math.max(min, state.amount + deltaAmount))

		if (dtMs > 0) {
			state.velocity = deltaAmount / Math.max(MIN_EVENT_DT_S, dtMs / 1000)
		}

		const success = computeSwipeSuccess()
		const progress = Math.min(Math.abs(displayAmount(success)) * 4, 1)
		if (config.showIndicator || config.pageSlide) {
			handlers.onProgress?.(doc, currentDirection(config), progress)
		}

		// A decisive flick commits right away — with wheel events its release
		// is invisible until the momentum tail ends, so waiting would only add
		// latency to what the user already finished doing.
		if (success && Math.abs(state.velocity) >= FLICK_COMMIT_VELOCITY) {
			finishGesture(doc)
			return
		}

		clearReleaseTimer()
		if (view) {
			// Resting fingers are probably still down (holding): wait longer, so
			// the user can hold at the threshold or pull back for as long as
			// they like. A moving lift ends much sooner.
			const releaseDelay =
				Math.abs(state.velocity) >= HOLD_VELOCITY ? RELEASE_FAST_MS : RELEASE_HOLD_MS
			state.releaseView = view
			state.releaseTimer = view.setTimeout(() => finishGesture(doc), releaseDelay)
		}
	}

	const onWheel = (e: WheelEvent): void => {
		if (disposed) return
		const config = handlers.getConfig()
		if (!config.enabled) return

		const doc =
			(e.target as Node | null)?.ownerDocument ?? (e.view?.document as Document | undefined)
		if (!doc) return
		const view = doc.defaultView

		const now = e.timeStamp || Date.now()
		const gap = now - state.lastEventAt
		state.lastEventAt = now
		if (gap > GESTURE_GAP_MS) {
			if (state.phase === 'active') finishGesture(doc)
			abortAnimation(doc)
			toIdle()
		}

		// Pinch-zoom arrives as ctrl+wheel; never treat it as a swipe.
		if (e.ctrlKey) return

		const scale = e.deltaMode === 1 ? 16 : e.deltaMode === 2 ? 100 : 1
		// Some drivers report horizontal panning as shift+vertical-wheel.
		const rawDx = e.deltaX !== 0 ? e.deltaX : e.shiftKey ? e.deltaY : 0
		const effDx = rawDx * scale
		const effDy = (e.deltaX !== 0 || !e.shiftKey ? e.deltaY : 0) * scale

		switch (state.phase) {
			case 'idle': {
				if (effDx === 0 && effDy === 0) return
				state.phase = 'deciding'
				state.decideDx = 0
				state.decideDy = 0
			}
			// fall through
			case 'deciding': {
				state.decideDx += effDx
				state.decideDy += effDy
				if (Math.hypot(state.decideDx, state.decideDy) < DECIDE_PX) return
				// CanTriggerSwipe: horizontal must dominate vertical 8×.
				if (Math.abs(state.decideDx) <= Math.abs(state.decideDy) * AXIS_DOMINANCE) {
					settle('scroll')
					return
				}
				if (consumedByScrollable(e.target as Element | null, state.decideDx)) {
					settle('scroll')
					return
				}
				state.phase = 'active'
				state.directionSign = state.decideDx >= 0 ? 1 : -1
				state.amount = (state.decideDx - effDx) / wholeSwipePx(config.sensitivity)
				state.velocity = 0
				processActive(e, doc, view, effDx, 0, config)
				return
			}
			case 'active': {
				processActive(e, doc, view, effDx, gap, config)
				return
			}
			case 'animating':
			case 'settled': {
				if (state.settleReason === 'scroll' && state.phase === 'settled') return
				const magnitude = Math.hypot(effDx, effDy)
				// After a commit the decaying momentum tail must not re-navigate,
				// so a new gesture needs a sharp spike. After a cancel (including
				// the spring-back) the deltas only need to be growing — resuming
				// the drag re-grabs the gesture without a dead zone.
				const cancelled = state.phase === 'animating' || state.settleReason === 'cancel'
				const ratio = cancelled ? SPIKE_RESUME_RATIO : SPIKE_RATIO
				if (magnitude >= SPIKE_MIN_PX && magnitude > state.lastTailMag * ratio) {
					if (state.phase === 'animating') abortAnimation(doc)
					toIdle()
					state.phase = 'deciding'
					state.decideDx = effDx
					state.decideDy = effDy
					return
				}
				state.lastTailMag = magnitude
				return
			}
		}
	}

	return {
		onWheel,
		dispose() {
			disposed = true
			clearReleaseTimer()
			cancelAnimationLoop()
		},
	}
}

function resolveDirection(directionSign: number, invert: boolean): SwipeDirection {
	// Measured on a real precision touchpad in WebView2: fingers moving left
	// arrive as negative deltaX. The Steam plugin this came from assumed the
	// opposite, and the launcher shipped with left and right swapped.
	const fingersLeft = directionSign < 0
	const isBack = invert ? !fingersLeft : fingersLeft
	return isBack ? 'back' : 'forward'
}

/**
 * Mirrors the browser's overscroll handoff: if the swipe lands inside a
 * horizontally scrollable element that can still scroll in the swipe
 * direction, let it scroll instead of navigating.
 */
export function consumedByScrollable(target: Element | null, deltaX: number): boolean {
	const view = target?.ownerDocument?.defaultView
	let node: Element | null = target
	while (node && node !== node.ownerDocument?.documentElement) {
		if (node.scrollWidth > node.clientWidth + 1) {
			const overflowX = view?.getComputedStyle(node).overflowX
			if (overflowX === 'auto' || overflowX === 'scroll') {
				const atStart = node.scrollLeft <= 0
				const atEnd = node.scrollLeft + node.clientWidth >= node.scrollWidth - 1
				const wantsRight = deltaX > 0
				if ((wantsRight && !atEnd) || (!wantsRight && !atStart)) return true
			}
		}
		node = node.parentElement
	}
	return false
}
