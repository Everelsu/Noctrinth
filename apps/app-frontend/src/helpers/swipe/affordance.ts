/**
 * The Affordance of gesture_nav_simple.cc: the circle with an arrow that slides
 * in from the edge while a swipe is dragged, ripples out as it nears the
 * threshold, bursts on completion and slides back on abort. Ported by way of
 * Two-Finger-Back (GPL-3.0); the geometry and timing are upstream's, the
 * colours are the launcher's — custom properties reach into the shadow root.
 *
 * The painted layer is 96x96 (twice the burst radius) and starts one
 * background radius outside the window, so the circle's centre sits at -20px
 * and reaches 126px at the activation threshold.
 */
import { config, fastOutSlowIn } from './overscroll-config'
import { MODE_EAST, type OverscrollMode } from './overscroll-controller'

const SIZE = 2 * config.MAX_RIPPLE_BURST_RADIUS
const CENTRE = config.MAX_RIPPLE_BURST_RADIUS
const EDGE_OFFSET = config.MAX_RIPPLE_BURST_RADIUS + config.BACKGROUND_RADIUS
const EXTRA_RATIO = config.AFFORDANCE_EXTRA_OFFSET / config.AFFORDANCE_ACTIVATION_OFFSET
const ARROW_SCALE = config.ARROW_SIZE / 24
const ARROW_ORIGIN = CENTRE - config.ARROW_SIZE / 2

/** Material "arrow_back" / "arrow_forward", the icons Chromium draws here. */
const ARROW_BACK = 'M20 11H7.83l5.59-5.59L12 4l-8 8 8 8 1.41-1.41L7.83 13H20v-2z'
const ARROW_FORWARD = 'M12 4l-1.41 1.41L16.17 11H4v2h12.17l-5.58 5.59L12 20l8-8z'

type State = 'DRAGGING' | 'ABORTING' | 'COMPLETING'

export class Affordance {
	private state: State = 'DRAGGING'
	private dragProgress = 0
	private abortProgress = 0
	private completeProgress = 0
	private rafId = 0
	private readonly host: HTMLDivElement
	private readonly layer: HTMLDivElement
	private readonly ripple: SVGCircleElement

	constructor(
		private readonly mode: OverscrollMode,
		private readonly maxDragProgress: number,
		private readonly onAnimationEnded: () => void,
	) {
		this.host = createHost()
		const shadow = this.host.attachShadow({ mode: 'closed' })
		shadow.innerHTML = markup(mode)
		this.layer = shadow.querySelector('.layer') as HTMLDivElement
		this.ripple = shadow.querySelector('.ripple') as SVGCircleElement

		document.body.appendChild(this.host)
		this.updatePaintedLayer()
		this.paint()
	}

	isFinishing() {
		return this.state !== 'DRAGGING'
	}

	/** 0 is no progress, 1 the activation threshold; more maps to travel past it. */
	setDragProgress(progress: number) {
		if (this.state !== 'DRAGGING' || this.dragProgress === progress) return
		this.dragProgress = progress
		this.updatePaintedLayer()
		this.paint()
	}

	/** Slides back out, for as long as it took to come in, then disposes of itself. */
	abort() {
		if (this.state !== 'DRAGGING') return
		this.state = 'ABORTING'
		const duration = this.getAffordanceProgress() * config.ABORT_ANIMATION_MS
		this.animate(duration, (t) => {
			this.abortProgress = t
			this.updatePaintedLayer()
			this.paint()
		})
	}

	/** Bursts the ripple, then disposes of itself. */
	complete() {
		if (this.state !== 'DRAGGING') return
		this.state = 'COMPLETING'
		this.animate(config.RIPPLE_BURST_ANIMATION_MS, (t) => {
			this.completeProgress = t
			this.layer.style.opacity = String(fastOutSlowIn(1 - t))
			this.paint()
		})
	}

	destroy() {
		if (this.rafId) cancelAnimationFrame(this.rafId)
		this.rafId = 0
		this.host.remove()
	}

	private getAffordanceProgress(): number {
		let progress = this.dragProgress
		if (progress >= 1) {
			const extra =
				this.maxDragProgress === 1 ? 1 : Math.min(1, (progress - 1) / (this.maxDragProgress - 1))
			progress = 1 + fastOutSlowIn(extra) * EXTRA_RATIO
		}
		return progress * (1 - fastOutSlowIn(this.abortProgress))
	}

	private updatePaintedLayer() {
		const offset = this.getAffordanceProgress() * config.AFFORDANCE_ACTIVATION_OFFSET
		const x = this.mode === MODE_EAST ? offset : -offset
		this.layer.style.transform = `translateX(${x}px)`
	}

	private paint() {
		const progress = Math.min(1, this.getAffordanceProgress())
		let rippleRadius: number
		if (this.state === 'COMPLETING') {
			const burst = fastOutSlowIn(this.completeProgress)
			rippleRadius =
				config.MAX_RIPPLE_RADIUS +
				burst * (config.MAX_RIPPLE_BURST_RADIUS - config.MAX_RIPPLE_RADIUS)
		} else {
			rippleRadius =
				config.BACKGROUND_RADIUS + progress * (config.MAX_RIPPLE_RADIUS - config.BACKGROUND_RADIUS)
		}
		this.ripple.setAttribute('r', String(rippleRadius))
		this.layer.classList.toggle('activated', progress >= 1)
	}

	/** gfx::LinearAnimation: a linear 0..1, tweened by the callback. */
	private animate(durationMs: number, onFrame: (t: number) => void) {
		const start = performance.now()
		const step = (now: number) => {
			const t = durationMs <= 0 ? 1 : Math.min(1, (now - start) / durationMs)
			onFrame(t)
			if (t < 1) {
				this.rafId = requestAnimationFrame(step)
			} else {
				this.rafId = 0
				this.onAnimationEnded()
			}
		}
		this.rafId = requestAnimationFrame(step)
	}
}

function createHost(): HTMLDivElement {
	const host = document.createElement('div')
	const declarations: Record<string, string> = {
		position: 'fixed',
		inset: '0',
		width: '100%',
		height: '100%',
		margin: '0',
		padding: '0',
		border: 'none',
		background: 'transparent',
		overflow: 'hidden',
		'pointer-events': 'none',
		'z-index': '2147483647',
	}
	for (const [property, value] of Object.entries(declarations)) {
		host.style.setProperty(property, value, 'important')
	}
	return host
}

function markup(mode: OverscrollMode): string {
	const east = mode === MODE_EAST
	const arrow = east ? ARROW_BACK : ARROW_FORWARD
	const edge = `${east ? 'left' : 'right'}: ${-EDGE_OFFSET}px`
	return `
<style>
	.layer {
		position: absolute;
		top: calc(50% - ${CENTRE}px);
		${edge};
		width: ${SIZE}px;
		height: ${SIZE}px;
		will-change: transform, opacity;
	}
	.ripple { fill: color-mix(in srgb, var(--color-brand) 30%, transparent); }
	.bg {
		fill: var(--surface-4, #2a2635);
		stroke: var(--surface-5, #3a3546);
		stroke-width: 1;
		filter: drop-shadow(0 2px 3.6px rgba(0, 0, 0, 0.45));
	}
	.arrow { fill: var(--color-brand); }
	.layer.activated .bg { fill: var(--color-brand); stroke: var(--color-brand); }
	.layer.activated .arrow { fill: var(--color-accent-contrast, #000); }
</style>
<div class="layer">
	<svg width="${SIZE}" height="${SIZE}" viewBox="0 0 ${SIZE} ${SIZE}">
		<circle class="ripple" cx="${CENTRE}" cy="${CENTRE}" r="${config.BACKGROUND_RADIUS}"></circle>
		<circle class="bg" cx="${CENTRE}" cy="${CENTRE}" r="${config.BACKGROUND_RADIUS}"></circle>
		<path class="arrow" transform="translate(${ARROW_ORIGIN} ${ARROW_ORIGIN}) scale(${ARROW_SCALE})" d="${arrow}"></path>
	</svg>
</div>`
}
