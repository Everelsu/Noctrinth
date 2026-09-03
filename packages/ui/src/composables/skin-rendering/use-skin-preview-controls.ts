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

	// How much closer the wheel has brought the model. The frame it is drawn in
	// is left alone: what grows is the figure inside it, so the nametag, the
	// subtitle and the controls under it all stay where they were.
	const modelZoom = ref(1)
	const MIN_ZOOM = 0.5
	const MAX_ZOOM = 4

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

	function onWheel(event: WheelEvent) {
		// Nothing else should move while the wheel is over the model: the page
		// scrolling out from under a zoom is what makes it feel broken.
		event.preventDefault()

		// A wheel reports lines on Firefox and pages when it is turned against a
		// modifier, so both are brought back to something like pixels first.
		const pixels =
			event.deltaMode === 1
				? event.deltaY * 16
				: event.deltaMode === 2
					? event.deltaY * 100
					: event.deltaY

		// Multiplicative, so a notch is the same proportion of the size wherever
		// it is turned, rather than a leap at one end and nothing at the other.
		const zoomed = modelZoom.value * Math.exp(-pixels * 0.001)
		modelZoom.value = Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, zoomed))
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
		modelZoom,
		onCanvasClick,
		onPointerDown,
		onPointerMove,
		onPointerUp,
		onWheel,
	}
}
