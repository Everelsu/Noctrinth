/**
 * Port of Firefox's spring physics used for the swipe spring-back:
 * gfx/layers/AxisPhysicsModel.cpp (RK4 integration at a fixed 120Hz timestep,
 * with interpolation between samples for variable framerates) and
 * gfx/layers/AxisPhysicsMSDModel.cpp (unit-mass mass-spring-damper).
 * SwipeTracker constructs it as (0, 0, 0, springConstant=250, dampingRatio=1).
 */

const FIXED_TIMESTEP_S = 1 / 120

interface PhysicsState {
	p: number
	v: number
}

interface Derivative {
	dp: number
	dv: number
}

export class AxisPhysicsMSDModel {
	private progress = 1
	private prev: PhysicsState
	private next: PhysicsState
	private destination: number
	private readonly springConstant: number
	private readonly springConstantSqrtXTwo: number
	private readonly dampingRatio: number

	constructor(
		initialPosition: number,
		initialDestination: number,
		initialVelocity: number,
		springConstant: number,
		dampingRatio: number,
	) {
		this.prev = { p: initialPosition, v: initialVelocity }
		this.next = { p: initialPosition, v: initialVelocity }
		this.destination = initialDestination
		this.springConstant = springConstant
		this.springConstantSqrtXTwo = Math.sqrt(springConstant) * 2
		this.dampingRatio = dampingRatio
	}

	private acceleration(state: PhysicsState): number {
		const springForce = (this.destination - state.p) * this.springConstant
		const dampForce = -state.v * this.dampingRatio * this.springConstantSqrtXTwo
		return springForce + dampForce
	}

	simulate(deltaSeconds: number): void {
		for (this.progress += deltaSeconds / FIXED_TIMESTEP_S; this.progress > 1; this.progress -= 1) {
			this.integrate(FIXED_TIMESTEP_S)
		}
	}

	private integrate(dt: number): void {
		this.prev = { ...this.next }

		const a = this.evaluate(this.next, 0, { dp: 0, dv: 0 })
		const b = this.evaluate(this.next, dt * 0.5, a)
		const c = this.evaluate(this.next, dt * 0.5, b)
		const d = this.evaluate(this.next, dt, c)

		const dpdt = (1 / 6) * (a.dp + 2 * (b.dp + c.dp) + d.dp)
		const dvdt = (1 / 6) * (a.dv + 2 * (b.dv + c.dv) + d.dv)

		this.next = { p: this.next.p + dpdt * dt, v: this.next.v + dvdt * dt }
	}

	private evaluate(initState: PhysicsState, dt: number, derivative: Derivative): Derivative {
		const state: PhysicsState = {
			p: initState.p + derivative.dp * dt,
			v: initState.v + derivative.dv * dt,
		}
		return { dp: state.v, dv: this.acceleration(state) }
	}

	getPosition(): number {
		return this.lerp(this.prev.p, this.next.p, this.progress)
	}

	getVelocity(): number {
		return this.lerp(this.prev.v, this.next.v, this.progress)
	}

	getDestination(): number {
		return this.destination
	}

	setDestination(destination: number): void {
		this.destination = destination
	}

	isFinished(smallestVisibleIncrement: number): boolean {
		const finishVelocity = smallestVisibleIncrement * 2
		return (
			Math.abs(this.destination - this.getPosition()) < smallestVisibleIncrement &&
			Math.abs(this.getVelocity()) <= finishVelocity
		)
	}

	private lerp(v1: number, v2: number, blend: number): number {
		return v1 * (1 - Math.min(1, blend)) + v2 * Math.min(1, blend)
	}
}
