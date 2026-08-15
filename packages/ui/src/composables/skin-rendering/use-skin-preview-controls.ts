import { type ComputedRef, type Ref, ref } from 'vue'

type MaybeReadonlyRef<T> = Ref<T> | ComputedRef<T>

export function useSkinPreviewControls({
	initialRotation,
	onClickWithoutDrag,
}: {
	initialRotation: MaybeReadonlyRef<number | undefined>
	onClickWithoutDrag: () => void
}) {
	const modelRotation = ref((initialRotation.value ?? 15.75) + Math.PI)
	// Pitch (rotation around the model's X axis). Lets the user drag up/down
	// to peek at the crown / soles of the skin. Clamped to ±90° so the model
	// never rolls fully upside-down, which gets disorienting fast.
	const modelPitch = ref(0)
	const MAX_PITCH = Math.PI / 2

	const isDragging = ref(false)
	const previousX = ref(0)
	const previousY = ref(0)
	const hasDragged = ref(false)

	function onPointerDown(event: PointerEvent) {
		;(event.currentTarget as HTMLElement).setPointerCapture(event.pointerId)
		isDragging.value = true
		previousX.value = event.clientX
		previousY.value = event.clientY
		hasDragged.value = false
	}

	function onPointerMove(event: PointerEvent) {
		if (!isDragging.value) return
		const deltaX = event.clientX - previousX.value
		const deltaY = event.clientY - previousY.value
		modelRotation.value += deltaX * 0.01
		// The camera sits on -Z, so a positive rotation.x tips the head AWAY from
		// it — subtract the drag delta so the model follows the cursor: drag down
		// → head tips toward you → see crown, drag up → see soles. Clamp inclusive
		// of ±90° so user can look straight at top/bottom but not flip past.
		modelPitch.value = Math.min(MAX_PITCH, Math.max(-MAX_PITCH, modelPitch.value - deltaY * 0.01))
		previousX.value = event.clientX
		previousY.value = event.clientY
		hasDragged.value = true
	}

	function onPointerUp(event: PointerEvent) {
		isDragging.value = false

		const target = event.currentTarget as HTMLElement
		if (target.hasPointerCapture(event.pointerId)) {
			target.releasePointerCapture(event.pointerId)
		}
	}

	function onCanvasClick() {
		if (!hasDragged.value) {
			onClickWithoutDrag()
		}

		hasDragged.value = false
	}

	function ignoreControlClick(event: MouseEvent) {
		event.stopPropagation()
	}

	return {
		ignoreControlClick,
		modelPitch,
		modelRotation,
		onCanvasClick,
		onPointerDown,
		onPointerMove,
		onPointerUp,
	}
}
