import { onBeforeUnmount, onMounted } from 'vue'

/**
 * The keys a gallery answers to: Ctrl+C copies, Ctrl+A selects everything,
 * Delete deletes, Escape lets go of the selection.
 *
 * Nothing is taken while a dialog or the image viewer is open — the viewer has
 * keys of its own — or while the focus is somewhere text is typed, or while
 * text is selected, which is what Ctrl+C means there.
 */
export function useGalleryShortcuts(handlers: {
	/** Returns whether there was anything to copy. */
	copy: () => boolean
	selectAll: () => boolean
	remove: () => boolean
	clear: () => boolean
}) {
	function hasTextSelection() {
		const selection = document.getSelection()
		return !!selection && !selection.isCollapsed && selection.toString().length > 0
	}

	function onKeydown(event: KeyboardEvent) {
		if (event.defaultPrevented || event.repeat) return
		if (document.querySelector('[aria-modal="true"], [data-modal-root]')) return
		const target = event.target as HTMLElement | null
		if (
			target?.closest('input, textarea, select, [contenteditable]:not([contenteditable="false"])')
		)
			return

		const modifier = event.ctrlKey || event.metaKey
		const key = event.key.toLowerCase()
		let handled = false

		if (modifier && key === 'c' && !hasTextSelection()) handled = handlers.copy()
		else if (modifier && key === 'a') handled = handlers.selectAll()
		else if (!modifier && event.key === 'Delete') handled = handlers.remove()
		else if (!modifier && event.key === 'Escape') handled = handlers.clear()

		if (handled) event.preventDefault()
	}

	onMounted(() => document.addEventListener('keydown', onKeydown))
	onBeforeUnmount(() => document.removeEventListener('keydown', onKeydown))
}
