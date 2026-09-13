/**
 * Where the interface's own start-up goes.
 *
 * The measuring was already here, and so was the notice for a step that has
 * hung. What was missing is the build it runs in: both halves were behind a
 * development-only gate — `import.meta.env.DEV` here and a `noop` logger in
 * `useDebugLogger` — and a development build is the one build whose start-up
 * nobody is complaining about. It also went to the webview's console, which is
 * not what a player sends when they report a slow start.
 *
 * So it goes to the launcher log as well, through the backend, in every build.
 * A start-up trace is a handful of lines a session and nothing is in it but
 * step names and milliseconds.
 */
import { invoke } from '@tauri-apps/api/core'
import { useDebugLogger } from '@modrinth/ui'

const debug = useDebugLogger('Startup')

/** When the interface started, so every step is stamped against the same zero. */
const startedAt = performance.now()

export function debugStartup(event: string, details: Record<string, unknown> = {}): void {
	const payload = JSON.stringify({
		at: new Date().toISOString(),
		sinceNavigationMs: Math.round(performance.now()),
		...details,
	})

	// In development this is the console, where it is read as it happens.
	debug(event, payload)

	// Everywhere, including there, it is also the launcher log — the one thing
	// a player can send. Nothing waits on it: a trace that delayed the start-up
	// it is measuring would be measuring itself.
	invoke('plugin:utils|log_startup_event', { event, details: payload }).catch(() => {})
}

export async function traceStartupStep<T>(label: string, run: () => Promise<T>): Promise<T> {
	const stepStartedAt = performance.now()
	debugStartup('Step started', { label })
	const timer = setTimeout(() => {
		debugStartup('Step still pending', {
			label,
			elapsedMs: Math.round(performance.now() - stepStartedAt),
		})
	}, 2000)
	try {
		const result = await run()
		debugStartup('Step completed', {
			label,
			elapsedMs: Math.round(performance.now() - stepStartedAt),
			sinceStartupMs: Math.round(performance.now() - startedAt),
		})
		return result
	} catch (error) {
		debugStartup('Step failed', {
			label,
			elapsedMs: Math.round(performance.now() - stepStartedAt),
			errorType: error instanceof Error ? error.name : typeof error,
		})
		throw error
	} finally {
		clearTimeout(timer)
	}
}
