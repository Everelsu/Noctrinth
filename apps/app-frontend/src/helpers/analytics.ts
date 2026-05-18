/**
 * Analytics is disabled in Noctrinth.
 *
 * Upstream Modrinth ships PostHog analytics pointed at `posthog.modrinth.com`.
 * For this fork those requests would only hammer Modrinth's servers with no
 * purpose, so every entry point below is a no-op. The event types are kept so
 * existing `trackEvent(...)` call sites stay type-checked.
 */

interface InstanceProperties {
	loader: string
	game_version: string
}

interface ProjectProperties extends InstanceProperties {
	id: string
	project_type: string
}

type AnalyticsEventMap = {
	Launched: { version: string; dev: boolean; onboarded: boolean }
	PageView: { path: string; fromPath: string; failed: unknown }
	InstanceCreate: { source: string }
	InstanceCreateStart: { source: string }
	InstanceStart: InstanceProperties & { source: string }
	InstanceStop: Partial<InstanceProperties> & { source?: string }
	InstanceDuplicate: InstanceProperties
	InstanceRepair: InstanceProperties
	InstanceSetIcon: Record<string, never>
	InstanceRemoveIcon: Record<string, never>
	InstanceUpdateAll: InstanceProperties & { count: number; selected: boolean }
	InstanceProjectUpdate: InstanceProperties & { id: string; name: string; project_type: string }
	InstanceProjectDisable: InstanceProperties & {
		id: string
		name: string
		project_type: string
		disabled: boolean
	}
	InstanceProjectRemove: InstanceProperties & { id: string; name: string; project_type: string }
	ProjectInstall: ProjectProperties & { version_id: string; title: string; source: string }
	ProjectInstallStart: { source: string }
	PackInstall: { id: string; version_id: string; title: string; source: string }
	PackInstallStart: Record<string, never>
	AccountLogIn: { source?: string }
	AccountLogOut: Record<string, never>
	JavaTest: { path: string; success: boolean }
	JavaManualSelect: { version: string }
	JavaAutoDetect: { path: string; version: string }
}

export type AnalyticsEvent = keyof AnalyticsEventMap

type OptionalArgs<T> = Record<string, never> extends T ? [properties?: T] : [properties: T]

// All analytics entry points are intentionally no-ops — see the file header.

export const initAnalytics = (): void => {}

export const debugAnalytics = (): void => {}

export const optOutAnalytics = (): void => {}

export const optInAnalytics = (): void => {}

export const trackEvent = <E extends AnalyticsEvent>(
	_eventName: E,
	..._args: OptionalArgs<AnalyticsEventMap[E]>
): void => {}
