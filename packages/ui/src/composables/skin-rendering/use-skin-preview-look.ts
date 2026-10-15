import { useRenderLoop } from '@tresjs/core'
import * as THREE from 'three'
import { onMounted, onUnmounted, type Ref, type ShallowRef } from 'vue'

/** How far the head turns, in radians: about 50° sideways, 30° up and down. */
const MAX_YAW = 0.85
const MAX_PITCH = 0.5
/** Pointer this far from the model's centre, in model widths, turns the head all the way. */
const FULL_TURN_DISTANCE = 1.6
/**
 * How far past the preview's edges, in preview widths, the pointer is still
 * watched. Further out it is somewhere else on the page, not near the player.
 */
const NEAR_MARGIN = 0.35
/** Seconds without the pointer moving before the head looks ahead again. */
const IDLE_SECONDS = 4
/** How quickly the head catches up with where it wants to look; higher is snappier. */
const FOLLOW_RATE = 6

/**
 * Noctrinth's own: the player's head follows the pointer while it is near
 * them, the way a player in the game turns to look at you, and settles back to
 * looking ahead once the pointer stops or goes elsewhere.
 *
 * Called after `useSkinPreviewAnimation`, so its frame runs after the mixer has
 * posed the head and the turn goes on top of the animation instead of being
 * overwritten by it.
 */
export function useSkinPreviewLook(options: {
	scene: ShallowRef<THREE.Object3D | null>
	container: Readonly<Ref<HTMLElement | null>>
	modelRotation: Ref<number>
}) {
	let pointer: { x: number; y: number } | null = null
	let idleSeconds = 0
	let yaw = 0
	let pitch = 0

	let head: THREE.Object3D | null = null
	let headOwner: THREE.Object3D | null = null
	const baseQuaternion = new THREE.Quaternion()
	const lookedQuaternion = new THREE.Quaternion()
	let looked = false
	const turn = new THREE.Quaternion()
	const euler = new THREE.Euler(0, 0, 0, 'YXZ')

	function onPointerMove(event: PointerEvent) {
		pointer = { x: event.clientX, y: event.clientY }
		idleSeconds = 0
	}

	function onPointerLeave() {
		pointer = null
	}

	onMounted(() => {
		window.addEventListener('pointermove', onPointerMove, { passive: true })
		document.documentElement.addEventListener('pointerleave', onPointerLeave)
	})

	onUnmounted(() => {
		window.removeEventListener('pointermove', onPointerMove)
		document.documentElement.removeEventListener('pointerleave', onPointerLeave)
	})

	function target(): [number, number] {
		const box = options.container.value?.getBoundingClientRect()
		if (!pointer || !box || idleSeconds > IDLE_SECONDS) return [0, 0]

		const margin = box.width * NEAR_MARGIN
		const near =
			pointer.x > box.left - margin &&
			pointer.x < box.right + margin &&
			pointer.y > box.top - margin &&
			pointer.y < box.bottom + margin
		if (!near) return [0, 0]

		const reach = Math.max(box.width, 1) * FULL_TURN_DISTANCE * 0.5
		const dx = (pointer.x - (box.left + box.width / 2)) / reach
		// The head sits in the upper part of the preview, not at its middle.
		const dy = (pointer.y - (box.top + box.height * 0.3)) / reach

		// The pointer is where the camera is looking from; turned into the
		// model's own frame by taking away how far the model itself is turned.
		// The camera sits on -Z, so the model faces it at a rotation of π.
		const turned = options.modelRotation.value - Math.PI
		const wrapped = Math.atan2(Math.sin(turned), Math.cos(turned))
		const facingAway = Math.abs(wrapped) > Math.PI / 2
		if (facingAway) return [0, 0]

		return [
			THREE.MathUtils.clamp(dx * MAX_YAW - wrapped, -MAX_YAW, MAX_YAW),
			// The head hangs off a bone that faces the other way in the model, so
			// its X axis is flipped and looking up is a negative turn.
			THREE.MathUtils.clamp(-dy * MAX_PITCH, -MAX_PITCH, MAX_PITCH),
		]
	}

	const { onLoop } = useRenderLoop()
	onLoop(({ delta }) => {
		const root = options.scene.value
		if (root !== headOwner) {
			headOwner = root
			head = root?.getObjectByName('Head') ?? null
			looked = false
		}
		if (!head) return

		idleSeconds += delta
		const [targetYaw, targetPitch] = target()
		const follow = 1 - Math.exp(-FOLLOW_RATE * delta)
		yaw += (targetYaw - yaw) * follow
		pitch += (targetPitch - pitch) * follow

		// An animation that does not move the head leaves last frame's turn in
		// place; taken back off before this frame's goes on, so it never adds up.
		if (looked && head.quaternion.equals(lookedQuaternion)) {
			head.quaternion.copy(baseQuaternion)
		}
		baseQuaternion.copy(head.quaternion)

		euler.set(pitch, yaw, 0)
		head.quaternion.premultiply(turn.setFromEuler(euler))
		lookedQuaternion.copy(head.quaternion)
		looked = true
	})
}
