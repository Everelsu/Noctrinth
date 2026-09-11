/**
 * Instances that are on disk but not in the launcher.
 *
 * See `packages/app-lib/src/api/instance_recovery.rs` for what is read out of a
 * folder, and why a version that cannot be read is left blank rather than
 * guessed at.
 */
import { invoke } from '@tauri-apps/api/core'

export interface OrphanedInstance {
	folder: string
	name: string
	game_version: string | null
	loader: string | null
	loader_version: string | null
	mods: number
	worlds: number
	modified: number
}

/** The folders under the instances directory the launcher has no row for. */
export async function scanForOrphans(): Promise<OrphanedInstance[]> {
	return await invoke('plugin:noctrinth-recovery|recovery_scan_for_orphans')
}

/** Puts one back, with its folder where it already is. Returns its id. */
export async function adoptOrphan(
	folder: string,
	name: string,
	gameVersion: string,
	loader: string,
	loaderVersion: string | null,
): Promise<string> {
	return await invoke('plugin:noctrinth-recovery|recovery_adopt_orphan', {
		folder,
		name,
		gameVersion,
		loader,
		loaderVersion,
	})
}

/**
 * Everywhere a launcher was found, rather than only where its installer would
 * have put it. See `packages/app-lib/src/api/launcher_search.rs`.
 */
export async function findLauncherPaths(launcherType: string): Promise<string[]> {
	return await invoke('plugin:noctrinth-recovery|recovery_find_launchers', { launcherType })
}
