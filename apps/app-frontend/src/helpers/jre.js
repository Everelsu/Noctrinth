/**
 * All theseus API calls return serialized values (both return values and errors);
 * So, for example, addDefaultInstance creates a blank instance object, where the Rust struct is serialized,
 *  and deserialized into a usable JS object.
 */
import { invoke } from '@tauri-apps/api/core'

/*

JavaVersion {
    path: Path
    version: String
}

*/

export async function get_java_versions() {
	return await invoke('plugin:jre|get_java_versions')
}

export async function set_java_version(javaVersion) {
	return await invoke('plugin:jre|set_java_version', { javaVersion })
}

// Finds all the installation of Java 7, if it exists
// Returns [JavaVersion]
export async function find_filtered_jres(version) {
	return await invoke('plugin:jre|jre_find_filtered_jres', { version })
}

// Gets java version from a specific path by trying to run 'java -version' on it.
// This also validates it, as it returns null if no valid java version is found at the path
export async function get_jre(path) {
	return await invoke('plugin:jre|jre_get_jre', { path })
}

// Tests JRE version by running 'java -version' on it.
// Returns true if the version is valid, and matches given (after extraction)
export async function test_jre(path, majorVersion) {
	return await invoke('plugin:jre|jre_test_jre', { path, majorVersion })
}

// Automatically installs specified java version
export async function auto_install_java(javaVersion) {
	return await invoke('plugin:jre|jre_auto_install_java', { javaVersion })
}

// Get max memory in KiB
export async function get_max_memory() {
	return await invoke('plugin:jre|jre_get_max_memory')
}

/**
 * Which GPU Windows hands a given Java runtime.
 *
 * `preference` mirrors the values the OS stores; which physical adapter
 * "high_performance" resolves to is the driver's decision, not ours.
 *
 * @typedef {'auto' | 'power_saving' | 'high_performance'} GpuPreference
 * @typedef {{ name: string, likely_discrete: boolean }} GpuAdapter
 * @typedef {{
 *   supported: boolean,
 *   unsupported_reason: string | null,
 *   java_path: string,
 *   preference: GpuPreference,
 *   adapters: GpuAdapter[],
 * }} JavaGpuStatus
 */

/** @returns {Promise<JavaGpuStatus>} */
export async function get_gpu_status(path) {
	return await invoke('plugin:jre|jre_get_gpu_status', { path })
}

/** @returns {Promise<JavaGpuStatus>} */
export async function set_gpu_preference(path, preference) {
	return await invoke('plugin:jre|jre_set_gpu_preference', { path, preference })
}

/**
 * A Java runtime the launcher downloaded, as it sits on disk.
 *
 * @typedef {{
 *   path: string,
 *   java_path: string,
 *   name: string,
 *   major_version: number | null,
 *   size_bytes: number,
 *   in_use: boolean,
 * }} InstalledRuntime
 */

/** @returns {Promise<InstalledRuntime[]>} */
export async function list_installed_runtimes() {
	return await invoke('plugin:jre|jre_list_installed_runtimes')
}

export async function remove_installed_runtime(path) {
	return await invoke('plugin:jre|jre_remove_installed_runtime', { path })
}

/** @returns {Promise<number>} bytes reclaimed */
export async function remove_unused_runtimes() {
	return await invoke('plugin:jre|jre_remove_unused_runtimes')
}
