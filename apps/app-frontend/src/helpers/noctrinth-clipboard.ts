import { invoke } from '@tauri-apps/api/core'

/**
 * Puts files on the clipboard the way Explorer's Ctrl+C does, so a paste into
 * a folder or a chat gets all of them. Rejects where the platform has no such
 * clipboard format; the caller copies one image instead.
 */
export async function copyFilesToClipboard(paths: string[]): Promise<void> {
	await invoke('plugin:noctrinth-clipboard|clipboard_copy_files', { paths })
}
