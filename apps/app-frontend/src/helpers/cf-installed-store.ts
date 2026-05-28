/**
 * Discover & persist which CurseForge mods are installed in a Minecraft
 * instance — using the same trick as xmcl-launcher.
 *
 * The Rust backend has no record of "which CF project did this .jar come
 * from?", so the canonical answer is CurseForge's own `/v1/fingerprints`
 * endpoint: hash each .jar locally (Murmur2, whitespace-stripped — see
 * `profile::compute_cf_fingerprints` in Rust), POST the hashes to CF, and
 * get back `{ file: { modId } }` for every recognised file.
 *
 * Why this trumps the previous localStorage-only approach:
 *   - Works for mods installed BEFORE this app version existed
 *   - Works for mods dragged into mods/ manually
 *   - Works for mods restored from a backup / synced between machines
 *   - Doesn't lie when the user uninstalls a mod (no stale entries)
 *
 * What localStorage is still good for:
 *   - Instant render: cache the last discovered set so the sidebar shows
 *     "Installed" badges immediately on next open, before the fingerprint
 *     round-trip resolves in the background.
 *   - Optimistic updates: in the same session, `markCfInstalled` adds an
 *     entry right after a successful install so the user sees it without
 *     waiting for the next scan.
 */

import { compute_cf_fingerprints } from './profile'
import { getCurseForgeFingerprintMatches } from './curseforge-api'

const KEY_PREFIX = 'cfins_'

function storeKey(instancePath: string): string {
  return KEY_PREFIX + btoa(unescape(encodeURIComponent(instancePath)))
}

/** Read the persisted list of "cf:<modId>" tags for an instance. */
function readRawIds(instancePath: string): number[] {
  try {
    const raw = localStorage.getItem(storeKey(instancePath))
    return raw ? (JSON.parse(raw) as number[]) : []
  } catch {
    return []
  }
}

function writeRawIds(instancePath: string, ids: number[]): void {
  try {
    // De-dup + sort for stable storage.
    const unique = [...new Set(ids)].sort((a, b) => a - b)
    localStorage.setItem(storeKey(instancePath), JSON.stringify(unique))
  } catch {
    // ignore quota / disabled storage
  }
}

/** Persist a single CF mod ID for the given instance (optimistic write). */
export function storeCfInstalled(instancePath: string, modId: number): void {
  const ids = readRawIds(instancePath)
  if (!ids.includes(modId)) {
    ids.push(modId)
    writeRawIds(instancePath, ids)
  }
}

/**
 * Return the cached set of installed CF project IDs in the "cf:<modId>"
 * format the BrowseSidebar / installed-check use.
 */
export function loadCfInstalledProjectIds(instancePath: string): string[] {
  return readRawIds(instancePath).map((id) => `cf:${id}`)
}

/**
 * Authoritative-ish discovery: hash every CurseForge-installable file in
 * the instance (mods, resource packs, shader packs, datapacks), ask CF
 * which ones it recognises, and persist the result.
 *
 * **Strictly additive** — the cache is never wiped, only grown:
 *   - Rust scan failed → return cache.
 *   - Empty result (no files at all) → return cache (do NOT wipe — the
 *     scan might have missed a folder, or files could be temporarily
 *     unreadable; better to keep stale entries than erase real installs).
 *   - CF API returned null → return cache.
 *   - CF returned zero matches → return cache (probably API issue).
 *   - CF returned matches → union with cache, persist, return.
 *
 * Trade-off: stale entries linger if the user uninstalls a mod outside
 * the app. Acceptable — user-driven flows naturally re-confirm, and
 * "false-installed" badge is much less broken than "lost-installed".
 *
 * To intentionally clear stale entries, callers can use `clearCfInstalled`.
 */
export async function refreshCfInstalledFromFingerprints(
  instancePath: string,
): Promise<string[]> {
  let pairs: Array<[string, number]>
  try {
    pairs = await compute_cf_fingerprints(instancePath)
  } catch (err) {
    console.warn('[CF-installed] fingerprint scan failed:', err)
    return loadCfInstalledProjectIds(instancePath)
  }

  if (pairs.length === 0) {
    // No files anywhere — likely a brand-new instance or a transient FS
    // hiccup. Preserve cache; user-driven uninstall is the right channel
    // for cleanup (see `clearCfInstalled`).
    return loadCfInstalledProjectIds(instancePath)
  }

  const fingerprints = pairs.map(([, fp]) => fp >>> 0) // ensure unsigned
  const matches = await getCurseForgeFingerprintMatches(fingerprints)
  if (!matches) {
    // CF API unavailable — keep whatever we had.
    return loadCfInstalledProjectIds(instancePath)
  }

  const discoveredIds = matches.exactMatches
    .map((m) => m.file.modId)
    .filter((x) => x > 0)

  if (discoveredIds.length === 0) {
    // CF recognised nothing despite us having files — almost certainly an
    // API blip or a brand-new file CF hasn't indexed yet. Preserve cache.
    return loadCfInstalledProjectIds(instancePath)
  }

  // Union with existing cache so a freshly-installed mod whose fingerprint
  // CF hasn't indexed yet is not lost.
  const existing = readRawIds(instancePath)
  const merged = Array.from(new Set([...existing, ...discoveredIds]))
  writeRawIds(instancePath, merged)
  return merged.map((id) => `cf:${id}`)
}

/** Explicitly wipe the cache for an instance — for manual "rescan" flows. */
export function clearCfInstalled(instancePath: string): void {
  writeRawIds(instancePath, [])
}
