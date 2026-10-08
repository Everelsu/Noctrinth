/**
 * Chrome's touchpad "Swipe between pages", as Two-Finger-Back ports it
 * (github.com/LucaSorvillo/Two-Finger-Back, GPL-3.0), brought into the launcher.
 * The numbers keep their Chromium names so a diff against upstream stays a grep:
 *   content/browser/renderer_host/overscroll_configuration.cc
 *   content/browser/renderer_host/overscroll_controller.cc
 *   content/browser/web_contents/aura/gesture_nav_simple.cc
 */
export const config = {
	// -- overscroll_configuration.cc
	/** Below this much accumulated delta there is no overscroll at all. */
	START_THRESHOLD_DIPS: 60,
	/** Fraction of the display's longest side that completes the navigation. */
	COMPLETE_THRESHOLD_PERCENT: 0.3,
	/** Inertial scrolling that runs this long without completing cancels the gesture. */
	MAX_INERTIAL_MS: 300,

	// -- overscroll_controller.cc
	/** Horizontal overscroll requires |dx| to dominate |dy| by this ratio. */
	MIN_DIRECTION_RATIO: 2.5,

	// -- gesture_nav_simple.cc
	ARROW_SIZE: 20,
	BACKGROUND_RADIUS: 20,
	MAX_RIPPLE_RADIUS: 40,
	MAX_RIPPLE_BURST_RADIUS: 48,
	RIPPLE_BURST_ANIMATION_MS: 200,
	ABORT_ANIMATION_MS: 300,
	/** Offset of the affordance once it sits at the activation threshold. */
	AFFORDANCE_ACTIVATION_OFFSET: 146,
	/** Extra travel allowed once dragged past the activation threshold. */
	AFFORDANCE_EXTRA_OFFSET: 72,
	/** gfx::Tween::FAST_OUT_SLOW_IN */
	TWEEN_FAST_OUT_SLOW_IN: [0.4, 0, 0.2, 1] as const,

	// -- Not from Chromium: the DOM shows neither gesture phases nor the momentum
	// flag, so both are read from the shape of the wheel stream.
	/**
	 * A wheel event this long after the previous one starts a new gesture, and
	 * only while nothing is on screen: resting fingers send nothing, so silence
	 * alone must never end a gesture.
	 */
	GESTURE_GAP_MS: 400,
	/** Strictly decaying events in a row after which the fingers have left the pad. */
	MOMENTUM_DECAY_EVENTS: 6,
	/** Shorter run: not a fling yet, but enough to stop a new overscroll starting. */
	MOMENTUM_SUSPECT_EVENTS: 3,
	/** Below this an event is jitter from resting fingers, not movement. */
	MICRO_DELTA_PX: 3,
	/** Once a fling runs, this much silence means it died out: a real end. */
	MOMENTUM_IDLE_MS: 200,
	/** Silence after which a gesture already past the threshold settles. */
	ARMED_IDLE_MS: 350,
	/** The same short of the threshold, where patience lets you stop and think. */
	GESTURE_SAFETY_MS: 1000,
}

/** Cubic bezier evaluator, so the curves match gfx::Tween exactly. */
export function cubicBezier(p1x: number, p1y: number, p2x: number, p2y: number) {
	const a = (v1: number, v2: number) => 1 - 3 * v2 + 3 * v1
	const b = (v1: number, v2: number) => 3 * v2 - 6 * v1
	const c = (v1: number) => 3 * v1
	const curve = (t: number, v1: number, v2: number) => ((a(v1, v2) * t + b(v1, v2)) * t + c(v1)) * t
	const slope = (t: number, v1: number, v2: number) =>
		3 * a(v1, v2) * t * t + 2 * b(v1, v2) * t + c(v1)

	return (x: number): number => {
		if (x <= 0) return 0
		if (x >= 1) return 1
		let t = x
		for (let i = 0; i < 8; i++) {
			const error = curve(t, p1x, p2x) - x
			if (Math.abs(error) < 1e-6) break
			const derivative = slope(t, p1x, p2x)
			if (derivative === 0) break
			t -= error / derivative
		}
		return curve(t, p1y, p2y)
	}
}

export const fastOutSlowIn = cubicBezier(...config.TWEEN_FAST_OUT_SLOW_IN)
