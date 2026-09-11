import { type AbstractWebNotificationManager, provideInstanceImport } from '@modrinth/ui'
import { open } from '@tauri-apps/plugin-dialog'

import { get_importable_instances, import_instance } from '@/helpers/import.js'
import { findLauncherPaths } from '@/helpers/noctrinth-recovery'

/** Launcher type identifiers understood by the backend, with how they're shown. */
const LAUNCHERS = [
	{ name: 'ModrinthApp', displayName: 'Modrinth App', supportsDeletingSource: true },
	{ name: 'MultiMC' },
	{ name: 'GDLauncher' },
	{ name: 'ATLauncher' },
	{ name: 'Curseforge' },
	{ name: 'PrismLauncher' },
] as const

export function setupInstanceImportProvider(notificationManager: AbstractWebNotificationManager) {
	const { handleError } = notificationManager

	provideInstanceImport({
		async getDetectedLaunchers() {
			const launchers = []
			for (const launcher of LAUNCHERS) {
				try {
					// Every place it was found, not only where its installer would
					// have put it: a second drive and a portable build in Downloads
					// are both normal, and typing the path is what people give up on.
					for (const path of await findLauncherPaths(launcher.name)) {
						const instances = await get_importable_instances(launcher.name, path)
						if (instances?.length > 0) {
							launchers.push({ ...launcher, path, instances })
						}
					}
				} catch {
					// Skip launchers that fail detection
				}
			}
			return launchers
		},
		async getImportableInstances(launcherName: string, path: string) {
			return (await get_importable_instances(launcherName, path)) ?? []
		},
		async importInstances(selections) {
			for (const sel of selections) {
				for (const instanceName of sel.instanceNames) {
					await import_instance(sel.launcher, sel.path, instanceName).catch(handleError)
				}
			}
		},
		async selectDirectory() {
			const result = await open({ multiple: false, directory: true })
			return result?.toString() ?? null
		},
	})
}
