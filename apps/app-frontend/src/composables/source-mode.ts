/**
 * Catalog source toggle (Modrinth / CurseForge) for browse mode.
 *
 * Defined as a module-level singleton so the choice survives navigation —
 * e.g. opening a mod/modpack page and returning to Browse no longer resets
 * it. Also persisted to localStorage so it survives app restarts.
 */

import { ref, watch } from 'vue'

export type SourceMode = 'modrinth' | 'curseforge'

const STORAGE_KEY = 'noctrinth:catalog-source'

function loadInitial(): SourceMode {
	try {
		return localStorage.getItem(STORAGE_KEY) === 'curseforge' ? 'curseforge' : 'modrinth'
	} catch {
		return 'modrinth'
	}
}

const sourceMode = ref<SourceMode>(loadInitial())

watch(sourceMode, (value) => {
	try {
		localStorage.setItem(STORAGE_KEY, value)
	} catch {
		// localStorage unavailable — in-memory persistence still works.
	}
})

/** Shared catalog-source ref — the same instance for every caller. */
export function useSourceMode() {
	return sourceMode
}

// ─── Source colour accent ─────────────────────────────────────────────────────

const ACCENT_KEY = 'noctrinth:source-accent'

function loadAccentEnabled(): boolean {
	try {
		return localStorage.getItem(ACCENT_KEY) === 'true'
	} catch {
		return false
	}
}

const accentBySource = ref<boolean>(loadAccentEnabled())

watch(accentBySource, (value) => {
	try {
		localStorage.setItem(ACCENT_KEY, String(value))
	} catch {
		// localStorage unavailable — in-memory persistence still works.
	}
})

/**
 * Whether the Discover/Browse page recolours its accent to match the active
 * catalog (green for Modrinth, orange for CurseForge). Off by default.
 */
export function useSourceAccent() {
	return accentBySource
}
