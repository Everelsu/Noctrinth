/**
 * The store of downloaded content files, kept by hash.
 *
 * See `packages/app-lib/src/util/noctrinth_file_cache.rs` for what goes in it:
 * a mod is downloaded once and copied out of the store for every pack, version
 * and instance that asks for it afterwards.
 */
import { invoke } from '@tauri-apps/api/core'

/** How many bytes of downloaded files are being kept. */
export async function fileCacheSize(): Promise<number> {
	return await invoke('plugin:noctrinth-file-cache|file_cache_size')
}

/** Empties the store. Everything in it can be downloaded again. */
export async function purgeFileCache(): Promise<void> {
	return await invoke('plugin:noctrinth-file-cache|file_cache_purge')
}
