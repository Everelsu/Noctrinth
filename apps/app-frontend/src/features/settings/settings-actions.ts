import { createContext } from '@modrinth/ui'

import type { SettingsTabEntry } from '@/features/settings/settings-search'

export type InstanceDefaultsTab = 'java' | 'window' | 'hooks'

/**
 * Opens the app settings modal from anywhere in the app, optionally on a specific tab.
 */
export interface SettingsActions {
	openSettings: () => void
	openProfile: () => void
	openFeatureFlags: () => void
	openSyncedOptions: () => void
	openInstanceDefaults: (tab: InstanceDefaultsTab) => void
	/** Noctrinth's own: a tab by its name's message id, for the quick search. */
	openSettingsTab: (id: string) => void
	/** The tabs there are to open, for the quick search to find. */
	settingsTabs: () => SettingsTabEntry[]
}

export const [injectSettingsActions, provideSettingsActions] = createContext<SettingsActions>(
	'root',
	'settingsActions',
)
