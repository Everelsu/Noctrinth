/**
 * Whether the launcher is downloading or installing something right now.
 *
 * The two things that count are the progress bars the backend opens — a pack
 * download, a Minecraft download, an import — and the install jobs a mod goes
 * through. Both announce themselves as events, so this tracks what is open from
 * those rather than polling; the lists are asked for once at startup, and again
 * on a slow tick while anything is running, so a "finished" event that never
 * arrives cannot leave the app looking busy forever.
 *
 * Distinct from `injectLoadingState`, which counts route changes and suspended
 * components — the difference between "the app is fetching a page" and "your
 * mod is downloading".
 */
import { computed, onMounted, onScopeDispose, ref, watch } from 'vue'

import { useAppEvent } from '@/composables/use-app-event'
import { install_job_list } from '@/helpers/install'
import { progress_bars_list } from '@/helpers/state'

/** Job states that are still going somewhere. */
const ACTIVE_JOB_STATUSES = new Set(['queued', 'running'])

/** How often the event-built picture is checked against the real lists. */
const RECONCILE_MS = 4000

export function useAppBusy() {
	const bars = ref(new Set<string>())
	const jobs = ref(new Set<string>())
	const busy = computed(() => bars.value.size > 0 || jobs.value.size > 0)

	function track(set: typeof bars, id: string, active: boolean): void {
		const next = new Set(set.value)
		if (active) {
			next.add(id)
		} else {
			next.delete(id)
		}
		set.value = next
	}

	async function reconcile(): Promise<void> {
		try {
			const [openBars, openJobs] = await Promise.all([
				progress_bars_list(),
				install_job_list(false),
			])
			bars.value = new Set(Object.keys(openBars))
			jobs.value = new Set(
				openJobs.filter((job) => ACTIVE_JOB_STATUSES.has(job.status)).map((job) => job.job_id),
			)
		} catch (error) {
			console.warn('Failed to read what the launcher is working on:', error)
		}
	}

	// A bar reports a null fraction when it is done with; anything else is
	// progress, including the first tick, which is what opens it here.
	useAppEvent('loading', (payload) => {
		track(bars, payload.loader_uuid, payload.fraction !== null)
	})

	useAppEvent('install_job', (payload) => {
		track(jobs, payload.job_id, ACTIVE_JOB_STATUSES.has(payload.status))
	})

	let ticker: number | undefined

	function stopTicker(): void {
		if (ticker !== undefined) {
			window.clearInterval(ticker)
			ticker = undefined
		}
	}

	watch(busy, (isBusy) => {
		stopTicker()
		if (isBusy) {
			ticker = window.setInterval(() => void reconcile(), RECONCILE_MS)
		}
	})

	onMounted(() => void reconcile())
	onScopeDispose(stopTicker)

	return busy
}
